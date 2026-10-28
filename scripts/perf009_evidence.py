"""Independent portable evidence integrity and read-time input checks.

Seals detect missing, additional and altered files, including a manifest cut at
a valid line boundary. They are integrity records, not external signatures.
"""
import ast
from functools import lru_cache
import json
from pathlib import Path
import re

from perf005_evidence import drivers, successful
from perf005_profile import executions, mapping_reads
from perf005_provider import digest


def file_inventory(root):
    paths = list(root.rglob("*"))
    if any(p.is_symlink() for p in paths):
        raise RuntimeError("symlink in portable evidence")
    return {str(p.relative_to(root)): digest(p) for p in sorted(paths)
            if p.is_file() and p != root / "evidence-seal.json"}


def seal(root):
    path = root / "evidence-seal.json"
    with path.open("x") as output:
        json.dump(dict(completed=True, files=file_inventory(root)), output, indent=2)
        output.write("\n")
    return digest(path)


def verify_seal(root, expected_hash):
    path = root / "evidence-seal.json"
    record = json.loads(path.read_text())
    if (digest(path) != expected_hash or record.get("completed") is not True
            or not record.get("files") or record["files"] != file_inventory(root)):
        raise RuntimeError("portable evidence seal incomplete/changed")


@lru_cache(maxsize=4)
def read_inputs(trace, trace_hash):
    """Actual bytes read by each process, never post-build temporary files."""
    if digest(trace) != trace_hash:
        raise RuntimeError("trace identity changed")
    pending, result = {}, {}
    with trace.open() as stream:
        for line in stream:
            match = re.match(r"(\d+)\s+\d+\.\d+\s+(.*)", line)
            if not match:
                continue
            pid, event = int(match[1]), match[2]
            if event.startswith("read(") and event.endswith("<unfinished ...>"):
                pending[pid] = event.removesuffix("<unfinished ...>")
                continue
            if event.startswith("<... read resumed>") and pid in pending:
                event = pending.pop(pid) + event.removeprefix("<... read resumed>")
            read = re.fullmatch(r'read\(\d+<([^>]+)>,\s*("(?:[^"\\]|\\.)*"), \d+\)\s+= (\d+)', event)
            if not read or not read[1].startswith("/"):
                continue
            name = read[1]
            if not (name.startswith("/tmp/GPR.") or name.endswith((".adc", ".rsp", ".input"))):
                continue
            content = ast.literal_eval("b" + read[2])
            if len(content) != int(read[3]):
                raise RuntimeError("truncated indirect-input read")
            key = (pid, name)
            result[key] = result.get(key, b"") + content
    return result


def verify_trace(profile):
    trace = profile.with_name(profile.name.replace(".profile.json", ".strace"))
    if not trace.is_file():
        raise RuntimeError("missing raw execution evidence")
    return _verify_trace(profile, digest(profile), digest(trace))


@lru_cache(maxsize=4)
def _verify_trace(profile, profile_hash, trace_hash):
    data = json.loads(profile.read_text())
    trace = profile.with_name(profile.name.replace(".profile.json", ".strace"))
    if not trace.is_file() or executions(trace) != data.get("executions"):
        raise RuntimeError("missing/altered raw execution evidence")
    if data.get("pre_execution_mapping_inputs") != mapping_reads(trace):
        raise RuntimeError("altered read-time mapping evidence")
    inputs = read_inputs(trace, trace_hash)
    # GNAT consumes ADCs in the backend; GCC consumes specs/response files.
    # Require a read witness from the same compiler family invocation. No
    # mutable post-build GPR configuration is accepted as that witness.
    for row in data["executions"]:
        if Path(row["executable"]).name != "gnat1":
            continue
        for arg in row["argv"]:
            if not arg.startswith("-gnatec="):
                continue
            name = arg.split("=", 1)[1]
            retained = data.get("gpr_temporary_inputs", {}).get(name)
            if retained is None or inputs.get((row["pid"], name)) != retained.encode():
                raise RuntimeError("configuration differs from compiler read-time input")
    for row in drivers(data):
        for arg in row["argv"]:
            prefix = next((p for p in ("@", "-specs=") if arg.startswith(p)), None)
            if prefix:
                name = arg[len(prefix):]
                retained = data.get("gpr_temporary_inputs", {}).get(name)
                if retained is None or inputs.get((row["pid"], name)) != retained.encode():
                    raise RuntimeError("response/specs differs from compiler read-time input")
    return data


def verify_execution(profile, executable, marker):
    data = verify_trace(profile)
    rows = [r for r in data["executions"] if r["executable"] == str(executable)]
    if data.get("exit") != 0 or len(rows) != 1 or data["command"][0] != str(executable):
        raise RuntimeError("missing/false consumer execution")
    successful(rows[0])
    if profile.with_name(profile.name.replace(".profile.json", ".log")).read_bytes() != marker:
        raise RuntimeError("consumer execution marker differs")


def verify_preparation(profile):
    data = verify_trace(profile)
    rows = drivers(data)
    if data.get("exit") != 0 or len(rows) != 106 or "-f" not in data["command"]:
        raise RuntimeError("incomplete/false provider preparation")
    for row in rows:
        successful(row)
    if len({tuple(row["argv"]) for row in rows}) != 106:
        raise RuntimeError("duplicate provider compilation")
    return data


def verify_artifacts(record, started_ns):
    paths = record.get("artifacts", {})
    if len(paths) != 11 or len({str(Path(p).resolve()) for p in paths}) != 11:
        raise RuntimeError("incomplete/duplicated portable artifact inventory")
    for name, expected in paths.items():
        path = Path(name)
        if (not path.is_file() or path.is_symlink() or digest(path) != expected
                or path.stat().st_mtime_ns < started_ns):
            raise RuntimeError("missing/stale/changed portable artifact")