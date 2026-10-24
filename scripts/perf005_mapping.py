"""Concrete-invocation GNAT map reconciliation; never a general map filter.

See PERF-005-R2 in the pilot report for the versioned Fmap/Fname/Osint sources.
Only additional conventional runtime cache entries may differ. Every actual
source read must be complete, unchanged, and identical across the two traces.
"""
import ast
from functools import lru_cache
import hashlib
from pathlib import Path
import re
import subprocess

from perf005_evidence import canonical, drivers, load_profile, provider_inputs, sha


def parse_mapping(text):
    lines = text.splitlines()
    if not lines or len(lines) % 3 or any(not line for line in lines):
        raise RuntimeError("invalid or missing GNAT mapping evidence")
    rows = [tuple(lines[i:i + 3]) for i in range(0, len(lines), 3)]
    units, files = set(), set()
    for unit, filename, path in rows:
        if (not re.fullmatch(r"[a-z][a-z0-9_.]*%[sb]", unit)
                or Path(filename).name != filename or not Path(path).is_absolute()
                or path == "/"):
            raise RuntimeError("invalid GNAT mapping entry")
        # Fmap actually permits replacement. Reject even identical duplicates:
        # sorting is safe only after BOTH hash-table keys have been checked.
        if unit in units or filename in files:
            raise RuntimeError("duplicate/ambiguous GNAT unit or filename mapping")
        units.add(unit)
        files.add(filename)
    return rows


@lru_cache(maxsize=12)
def source_reads(trace, pids):
    """Read-time source contents by gnat1 PID, including resumed read syscalls.

    strace C octal escapes describe bytes, not UTF-8 Unicode code points.
    Evidence is accepted only when the complete bytes match the retained file.
    """
    pending, reads = {}, {}
    with trace.open() as stream:
        for line in stream:
            head = re.match(r"(\d+)\s+\d+\.\d+\s+(.*)", line)
            if not head:
                continue
            pid, event = int(head[1]), head[2]
            if pid not in pids:
                continue
            if event.startswith("read(") and event.endswith("<unfinished ...>"):
                pending[pid] = event.removesuffix("<unfinished ...>")
                continue
            if event.startswith("<... read resumed>") and pid in pending:
                event = pending.pop(pid) + event.removeprefix("<... read resumed>")
            match = re.fullmatch(r'read\(\d+<([^>]+\.(?:ads|adb))>,\s*("(?:[^"\\]|\\.)*"), \d+\)\s+= (\d+)', event)
            if not match:
                continue
            payload = ast.literal_eval("b" + match[2])
            if len(payload) != int(match[3]):
                raise RuntimeError("truncated source-read evidence")
            if payload:
                key = (pid, match[1])
                reads[key] = reads.get(key, b"") + payload
    result = {}
    for (pid, name), content in reads.items():
        path = Path(name)
        if not path.is_file() or path.is_symlink() or path.read_bytes() != content:
            raise RuntimeError(f"incomplete/changed source-read evidence: {name}")
        result.setdefault(pid, {})[name] = hashlib.sha256(content).hexdigest()
    return result


def invocation_reads(data, trace, source, root):
    backends = [r for r in data["executions"] if Path(r["executable"]).name == "gnat1"
                and source in r["argv"]]
    if len(backends) != 1 or backends[0].get("exit") != 0:
        raise RuntimeError("missing/ambiguous successful mapping backend")
    pids = frozenset(r["pid"] for r in data["executions"]
                     if Path(r["executable"]).name == "gnat1"
                     and any("/alire/cache/dependencies/gnatcoll_" in a for a in r["argv"]))
    reads = source_reads(trace, pids).get(backends[0]["pid"], {})
    if source not in reads or not any(Path(p).name == "system.ads" for p in reads):
        raise RuntimeError("incomplete actual source-resolution evidence")
    if any(sha(Path(p)) != digest for p, digest in reads.items()):
        raise RuntimeError("changed source hash after reading trace")
    return {canonical(p, root): h for p, h in sorted(reads.items())}


def runtime_root(root):
    lines = (root / "oam-ArraySample-provider-identity.log").read_text().splitlines()
    paths = [Path(line) for line in lines if line.endswith("/adainclude")]
    if len(paths) != 1 or not paths[0].is_dir():
        raise RuntimeError("missing qualified runtime source directory")
    return paths[0].resolve()


@lru_cache(maxsize=None)
def runtime_filename(unit, compiler):
    kind = ".ads" if unit.endswith("%s") else ".adb"
    krunch = Path(compiler).with_name("gnatkr")
    return subprocess.check_output([str(krunch), unit[:-2].replace(".", "-") + kind, "8"], text=True).strip()


def conventional_runtime(unit, filename, path, runtime, compiler):
    if path.parent != runtime or path.name != filename or path.is_symlink() or not path.is_file():
        raise RuntimeError("mapping difference is not a direct qualified runtime source")
    kind = ".ads" if unit.endswith("%s") else ".adb"
    if not filename.endswith(kind):
        raise RuntimeError("spec/body mapping substitution")
    wanted = runtime_filename(unit, compiler)
    if wanted != filename:
        raise RuntimeError("nonconventional runtime unit mapping")
    text = re.sub(r"--[^\n]*", "", path.read_text())
    declarations = re.findall(r"\b(?:package|procedure|function)\s+(?:body\s+)?([\w.]+)", text, re.I)
    if unit[:-2] not in [name.lower() for name in declarations]:
        raise RuntimeError("runtime mapping unit/source declaration mismatch")
    return sha(path)


def reconcile_maps(left_text, right_text, left_root, right_root, left_reads, right_reads, compilers):
    left, right = parse_mapping(left_text), parse_mapping(right_text)
    normalized = [{tuple(canonical(s, root) for s in row): row for row in rows}
                  for rows, root in ((left, left_root), (right, right_root))]
    a, b = normalized
    common, only_left, only_right = sorted(a.keys() & b.keys()), sorted(a.keys() - b.keys()), sorted(b.keys() - a.keys())
    conflicts = sorted({r[0] for r in only_left} & {r[0] for r in only_right})
    file_conflicts = sorted({r[1] for r in only_left} & {r[1] for r in only_right})
    if conflicts or file_conflicts:
        raise RuntimeError(f"conflicting source mapping: {conflicts}, {file_conflicts}")
    if not left_reads or left_reads != right_reads:
        raise RuntimeError("actual dependency/source resolution differs or is missing")
    resolved = []
    runtimes = runtime_root(left_root), runtime_root(right_root)
    for side, keys in (("both", common), ("original", only_left), ("pilot", only_right)):
        for row in keys:
            index = 1 if side == "pilot" else 0
            raw = normalized[index][row]
            path = Path(raw[2])
            if not path.is_file() or path.is_symlink():
                raise RuntimeError(f"missing/invalid mapped source: {path}")
            digest = sha(path)
            if side == "both":
                other = Path(b[row][2])
                if not other.is_file() or other.is_symlink() or digest != sha(other):
                    raise RuntimeError("mapped source hash differs")
                classification = "Proven equivalent source relocation"
                counterpart = str(other)
            else:
                digest = conventional_runtime(raw[0], raw[1], path, runtimes[index], compilers[index])
                other = runtimes[1 - index] / raw[1]
                if digest != conventional_runtime(raw[0], raw[1], other, runtimes[1 - index], compilers[1 - index]):
                    raise RuntimeError("runtime mapped source hash differs")
                counterpart = str(other)
                classification = ("Proven equivalent source relocation" if row[2] in left_reads
                                  else "Proven irrelevant mapping difference")
            resolved.append(dict(side=side, unit=row[0], filename=row[1], normalized_path=row[2],
                                 path=str(path), resolved_path=str(path.resolve()), counterpart=counterpart,
                                 sha256=digest, read_by_compiler=row[2] in left_reads,
                                 classification=classification,
                                 basis="complete paired compiler source reads; Fname.UF.Get_File_Name and Osint.Find_File"))
    return dict(original_sha256=hashlib.sha256(left_text.encode()).hexdigest(),
                pilot_sha256=hashlib.sha256(right_text.encode()).hexdigest(),
                common=common, original_only=only_left, pilot_only=only_right,
                conflicts=conflicts, filename_conflicts=file_conflicts, duplicates=[],
                resolved=resolved, source_reads=left_reads)


def reconcile_policy(original, pilot, policy):
    paths = [original / f"oam-ArraySample-{policy}-build.profile.json",
             pilot / f"provider-{policy}-build.profile.json"]
    roots = original, pilot
    data = [load_profile(path) for path in paths]
    import json
    versions = [json.loads((root / "result.json").read_text())["records"][0]["compiler"] for root in roots]
    if versions[0] != versions[1] or versions[0] not in ("14.2.0", "16.1.0"):
        raise RuntimeError("mapping compiler versions differ/unqualified")
    from perf005_profile import mapping_reads
    for d, path in zip(data, paths):
        trace = path.with_name(path.name.replace(".profile.json", ".strace"))
        if mapping_reads(trace) != d.get("pre_execution_mapping_inputs"):
            raise RuntimeError("read-time mapping evidence differs from raw trace")
    records = []
    for d, root in zip(data, roots):
        inventory = {}
        for record in drivers(d):
            argv = record["argv"]
            sources = [s for s in argv if "/alire/cache/dependencies/gnatcoll_" in s
                       and s.endswith((".ads", ".adb"))]
            if sources:
                if len(sources) != 1 or record.get("exit") != 0:
                    raise RuntimeError("invalid provider invocation")
                key = canonical(sources[0], root)
                if key in inventory:
                    raise RuntimeError("duplicate provider invocation")
                inventory[key] = record
        records.append(inventory)
    if records[0].keys() != records[1].keys() or len(records[0]) != 89:
        raise RuntimeError("incomplete mapping invocation inventory")
    result = []
    for source in sorted(records[0]):
        pair = [r[source] for r in records]
        texts, reads, names = [], [], []
        for record, d, root, path in zip(pair, data, roots, paths):
            argv = record["argv"]
            actual_source = source.replace("<CAMPAIGN>", str(root))
            maps = [a.split("=", 1)[1] for a in argv if a.startswith("-gnatem=")]
            if len(maps) != 1:
                raise RuntimeError("missing/ambiguous effective mapping")
            key = actual_source + "\n" + maps[0]
            if key not in d.get("pre_execution_mapping_inputs", {}):
                raise RuntimeError("missing read-time mapping evidence")
            texts.append(d["pre_execution_mapping_inputs"][key])
            names.append(maps[0])
            reads.append(invocation_reads(d, path.with_name(path.name.replace(".profile.json", ".strace")), actual_source, root))
        report = reconcile_maps(*texts, *roots, *reads, [p["executable"] for p in pair])
        actual_units = [row[0] for row in parse_mapping(texts[0])
                        if canonical(row[2], original) == source]
        if len(actual_units) != 1:
            raise RuntimeError("missing/ambiguous actual compilation unit mapping")
        report.update(source=source, actual_unit=actual_units[0], compiler=versions[0], policy=policy,
                      original_mapping=names[0], pilot_mapping=names[1])
        result.append(report)
    # An extra cache entry must also have a historical, complete read witness;
    # merely hashing today's two copies is not evidence of R1 source identity.
    witnessed = {p: h for report in result for p, h in report["source_reads"].items()}
    for report in result:
        for row in report["resolved"]:
            if row["side"] != "both" and witnessed.get(row["normalized_path"]) != row["sha256"]:
                raise RuntimeError(f"unwitnessed runtime cache mapping: {row['unit']}")
    return result


def provider_equivalence(original, pilot, policy):
    """Preserve all non-map options and prove each map for its actual source.

    This is not reusable approval of a filtered mapping file. Changed source
    reads, missing traces or any new non-runtime difference fail every time.
    """
    left = provider_inputs(original / f"oam-ArraySample-{policy}-build.profile.json", original)
    right = provider_inputs(pilot / f"provider-{policy}-build.profile.json", pilot)
    reports = reconcile_policy(original, pilot, policy)
    if len(left) != 106 or len(right) != 106:
        raise RuntimeError("incomplete provider effective-input inventory")
    def checked_options(inputs):
        result = []
        for argv in inputs:
            maps = [a for a in argv if isinstance(a, list) and a[0] == "-gnatem="]
            if maps:
                sources = [a for a in argv if isinstance(a, str) and a.endswith((".adb", ".ads"))]
                if len(sources) != 1 or len(maps) != 1 or sources[0] not in {r["source"] for r in reports}:
                    raise RuntimeError("unreconciled provider mapping")
                # Replace only this independently reconciled map with its
                # concrete source identity. Every other option stays exact.
                argv = [a if a is not maps[0] else ["-gnatem=", sources[0]] for a in argv]
            result.append(argv)
        import json
        return sorted(result, key=lambda v: json.dumps(v, sort_keys=True))
    if checked_options(left) != checked_options(right):
        raise RuntimeError("actual provider non-mapping effective inputs differ")
    return reports