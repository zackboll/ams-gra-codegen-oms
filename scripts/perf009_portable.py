"""Fail-closed portable experiment checks; not a Float32 inventory/profile."""
import json
from pathlib import Path
import re

from perf005_ci import COMPILERS, NAMES, POLICIES
from perf005_evidence import drivers, effective_args, successful
from perf005_provider import digest, source_identity, sources, verify_variant

MARKER = b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n"


def cells():
    return [(profile, name, policy) for profile in ("oam", "qualified")
            for name in NAMES for policy in POLICIES]


def inventory(consumer):
    """Portable project closure: generated bodies/bodyless model specs + guard.

    Service_API is not imported by this consumer; Service_Codec's spec is
    compiled with its body. The support project imports src, NOT float32.
    Reject new/removed sources until the project closure is reviewed again.
    """
    namespace = "task093" if consumer.name.startswith("qualified-") else "oam"
    generated = {"helper_demand_consumer.adb", "service_codec.adb", "service_codec.ads",
                 "service_api.ads", "programs.ads", f"programs-{namespace}.ads"}
    actual = {p.name for p in consumer.iterdir() if p.suffix.lower() in (".ads", ".adb", ".c", ".s")}
    support = consumer.parent / (consumer.name + "-support") / "src"
    if actual != generated or {p.name for p in support.iterdir()} != {
            "ams_gra_oms_json_guard.adb", "ams_gra_oms_json_guard.ads"}:
        raise RuntimeError("portable source inventory changed")
    paths = [consumer / name for name in sorted(generated)
             if name.endswith(".adb") or name.startswith("programs")]
    paths.append(support / "ams_gra_oms_json_guard.adb")
    if any(p.is_symlink() or not p.is_file() for p in paths):
        raise RuntimeError("portable source is missing or symlinked")
    return paths


def validate_build(command, consumer, data, compiler, policy, started_ns):
    if "-f" not in command or "-XEXTERNALLY_BUILT=true" not in command or data["exit"] != 0:
        raise RuntimeError("forced portable provider build required")
    expected = {str(p): digest(p) for p in inventory(consumer)}
    version, gcc_hash, _, _, _ = COMPILERS[compiler]
    identity = (consumer.parent / f"{consumer.name}-provider-identity.log").read_text()
    executable = identity.splitlines()[0]
    if f"{gcc_hash}  {executable}\n" not in identity or digest(Path(executable)) != gcc_hash:
        raise RuntimeError("portable compiler identity changed")
    wanted = [] if policy == "normal" else [f"pragma Assertion_Policy ({policy});\n"]
    compiled = []
    binder_compilations = 0
    artifacts = {}
    for record in drivers(data):
        successful(record)
        if record["executable"] != executable:
            raise RuntimeError("unexpected portable compiler")
        inputs = [a for a in record["argv"] if a.endswith((".adb", ".ads", ".c", ".s", ".S"))]
        if len(inputs) != 1:
            raise RuntimeError("ambiguous portable compiler input")
        source = inputs[0]
        if not Path(source).is_absolute() and source == "b__helper_demand_consumer.adb":
            binder_compilations += 1
            continue
        if source not in expected or source in compiled:
            raise RuntimeError("unexpected/duplicate portable compiler input")
        compiled.append(source)
        backends = [r for r in data["executions"] if Path(r["executable"]).name == "gnat1"
                    and source in r["argv"]]
        if len(backends) != 1:
            raise RuntimeError("portable compiler backend missing/duplicate")
        successful(backends[0])
        for invocation in (record, backends[0]):
            args = effective_args(data, invocation["argv"][1:], consumer.parent, source=source)
            scalars = [a for a in args if isinstance(a, str)]
            if (any(a.startswith("-O") and a != "-O0" for a in scalars)
                    or any(a.startswith(("-m32", "-mx32", "--RTS=", "--target=", "-gnatep=", "-fplugin"))
                           for a in scalars)):
                raise RuntimeError("portable ABI/runtime/optimization override")
            policies = [a[1] for a in args if isinstance(a, list) and a[0] == "-gnatec="
                        and re.search(r"\bAssertion_Policy\b", a[1], re.I)]
            if policies != wanted or not {"-gnat2022", "-gnato", "-O0"} <= set(
                    scalars):
                raise RuntimeError("portable effective compiler policy/options differ")
        path = Path(source)
        objdir = consumer / "obj" if path.parent == consumer else path.parent.parent / "obj"
        for artifact in (objdir / (path.stem + ".o"), objdir / (path.stem + ".ali")):
            if (not artifact.is_file() or artifact.is_symlink()
                    or artifact.stat().st_mtime_ns < started_ns):
                raise RuntimeError("missing/stale portable object or ALI")
            artifacts[str(artifact)] = digest(artifact)
    if set(compiled) != set(expected):
        raise RuntimeError("missing portable generated/support compilation")
    if binder_compilations != 1:
        raise RuntimeError("missing/duplicate binder-generated compilation")
    for name in ("gnatbind", "ld"):
        invocations = [r for r in data["executions"] if Path(r["executable"]).name == name]
        if len(invocations) != 1:
            raise RuntimeError("missing/duplicate portable bind/link execution")
        successful(invocations[0])
    executable_path = consumer / "bin/helper_demand_consumer"
    if (not executable_path.is_file() or executable_path.is_symlink()
            or executable_path.stat().st_mtime_ns < started_ns):
        raise RuntimeError("missing/stale portable executable")
    artifacts[str(executable_path)] = digest(executable_path)
    return dict(compiled_sources=expected, artifacts=artifacts, compiler=version,
                policy=policy, bind=True, link=True)


def validate_result(root, compiler):
    result = json.loads((root / "result.json").read_text())
    records = result.get("records", [])
    if (result.get("completed") is not True or result.get("red") is not False
            or [(r["profile"], r["name"], r["policy"]) for r in records] != cells()
            or any(r["compiler"] != COMPILERS[compiler][0] or r["family"] != "portable"
                   or r["compiler_exit"] != 0 or r["intentional_red"] is not False for r in records)):
        raise RuntimeError("incomplete/wrong portable matrix")
    for profile, name, policy in cells():
        if (root / f"{profile}-{name}-{policy}-execute.log").read_bytes() != MARKER:
            raise RuntimeError("portable completion marker missing/changed")
    return result


def source_bytes(directory):
    return {p.name: p.read_bytes() for p in directory.iterdir() if p.suffix in (".ads", ".adb")}


def compare(baseline, candidate, compiler):
    if baseline == candidate or baseline in candidate.parents or candidate in baseline.parents:
        raise RuntimeError("baseline and candidate must be independent")
    if validate_result(baseline, compiler) != validate_result(candidate, compiler):
        raise RuntimeError("ordered portable outcomes differ")
    for root in (baseline, candidate):
        lines = (root / "sources.sha256").read_text().splitlines()
        if not lines or len(lines) != len(set(lines)):
            raise RuntimeError("portable source manifest empty/duplicate")
        seen = set()
        for line in lines:
            expected, name = line.split("  ", 1)
            if name in seen:
                raise RuntimeError("portable manifest duplicate path")
            seen.add(name)
            path = Path(name)
            if not path.is_file() or path.is_symlink() or digest(path) != expected:
                raise RuntimeError("portable source evidence changed/missing")
        required = {str(p) for p in root.rglob("*") if p.is_file()
                    and p.suffix in (".ads", ".adb", ".c", ".gpr", ".lock", ".xsd")}
        if not required <= seen:
            raise RuntimeError("portable source manifest truncated")
    provenance = json.loads((candidate / "perf005-provider-consumers.json").read_text())
    if [r["cell"] for r in provenance] != [f"{p}-{n}-{a}-build" for p, n, a in cells()]:
        raise RuntimeError("portable provider provenance incomplete")
    for profile, name, policy in cells():
        label = f"{profile}-{name}"
        cell = f"{label}-{policy}"
        for suffix in ("", "-plain", "-opted"):
            if source_bytes(baseline / (label + suffix)) != source_bytes(candidate / (label + suffix)):
                raise RuntimeError("portable generated source bytes differ")
        for root in (baseline, candidate):
            inventory(root / label)
            if b"codec status: READY" not in (root / f"{label}-codec-check.log").read_bytes():
                raise RuntimeError("portable readiness evidence missing")
        if source_identity(sources(baseline / label)) != source_identity(sources(candidate / label)):
            raise RuntimeError("portable provider sources differ")
        def normalize(text, root):
            return text.replace(str(root) + "-state", "<STATE>").replace(str(root), "<CAMPAIGN>")
        for suffix in ("-alire.lock", ".adc"):
            if suffix == ".adc" and policy == "normal":
                continue
            if normalize((baseline / (cell + suffix)).read_text(), baseline) != normalize(
                    (candidate / (cell + suffix)).read_text(), candidate):
                raise RuntimeError("portable lock/policy differs")
        verification = json.loads((candidate / f"{cell}-build.verification.json").read_text())
        if len(verification["compiled_sources"]) != 5 or verification["policy"] != policy:
            raise RuntimeError("portable fresh compilation evidence incomplete")
        from perf009_provider import validate_lock
        validate_lock(candidate / f"{cell}-alire.lock", compiler)
    for policy in POLICIES:
        variant = candidate / "perf005-providers" / policy
        record = json.loads((variant / "complete.json").read_text())
        if record["identity"]["compiler"] != compiler or record["identity"]["policy"] != policy:
            raise RuntimeError("portable provider compiler/policy mismatch")
        verify_variant(variant, record["identity"])
    return dict(completed=True, positive_cells=30, execution_cells=30,
                generated_support_compilations=150, provider_constructions=3)