#!/usr/bin/env python3
"""Disposable real-GNAT and separately labeled retained-evidence controls.

Never modifies the supplied positive campaign. Requires its private Alire
environment for the pinned compiler. Every output directory must be new.
"""
import argparse
import copy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys

from perf005_ci import validate_toolchain
from perf005_evidence import drivers, effective_args
from perf005_profile import run
from perf005_provider import digest, manifest, source_identity, sources
from perf009_evidence import seal, verify_artifacts, verify_execution, verify_preparation, verify_seal, verify_trace
from perf009_portable import MARKER, validate_build
from perf009_provider import validate_lock, verify_provider

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True


def rejected(callback):
    try:
        callback()
    except (RuntimeError, ValueError, KeyError, FileNotFoundError) as error:
        return str(error)
    raise RuntimeError("intentional negative was accepted")


def live_replays(campaign, output, policy):
    """Use genuine provider compiler inputs; mutate disposable inputs only."""
    profile = campaign / f"provider-{policy}-build.profile.json"
    data = verify_trace(profile)
    row = next(r for r in drivers(data) if any(a.endswith("/core/src/gnatcoll-json.adb") for a in r["argv"]))
    source = Path(row["argv"][-1])
    mapping_name = next(a.split("=", 1)[1] for a in row["argv"] if a.startswith("-gnatem="))
    mapping = data["pre_execution_mapping_inputs"][str(source) + "\n" + mapping_name]
    spec = importlib.util.spec_from_file_location("portable_replay", ROOT / "scripts/perf005-mapping-experiments.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    results = []
    for kind in ("provider-source", "gpr-configuration", "compiler-response", "read-time-mapping"):
        root = output / f"{policy}-{kind}"
        root.mkdir()
        changed, record, map_text = copy.deepcopy(data), copy.deepcopy(row), mapping
        if kind in ("provider-source", "read-time-mapping"):
            target = source
            if kind == "read-time-mapping":
                from perf005_mapping import parse_mapping
                target = Path(next(r[2] for r in parse_mapping(mapping) if r[0] == "gnatcoll%s"))
            bad = root / target.name
            bad.write_text('pragma Compile_Time_Error (True, "PERF009_CHANGED_PROVIDER");\n' + target.read_text())
            if kind == "provider-source":
                record["argv"][-1] = str(bad)
            map_text = map_text.replace(str(target), str(bad))
        elif kind == "gpr-configuration":
            name = next(a.split("=", 1)[1] for a in record["argv"] if a.startswith("-gnatec="))
            changed["gpr_temporary_inputs"][name] += '\npragma Compile_Time_Error (True, "PERF009_CHANGED_CONFIGURATION");\n'
        else:
            response = root / "compiler.rsp"
            response.write_text("-fperf009-invalid-compiler-option\n")
            record["argv"].insert(1, "@" + str(response))
        # The shared replay parser requires one GNAT backend. An invalid GCC
        # response must fail before that backend, so record it directly.
        if kind == "compiler-response":
            code, text = run(record["argv"], root, root / "failure", trace=True)
            report = dict(exit=code, stderr=text.decode(errors="replace"))
        else:
            report = module.replay(changed, record, map_text, root / "replay")
        if report["exit"] == 0 or not report["stderr"]:
            raise RuntimeError("actual changed provider/configuration GNAT failure missing")
        results.append(dict(policy=policy, control=kind, category="actual-compiler-failure",
                            exit=report["exit"], diagnostic=report["stderr"]))
    # Policy behavior is checked by an executable assertion, not text from the
    # harness. Check must raise; Ignore must not evaluate the assertion.
    root = output / f"{policy}-assertion-behavior"
    root.mkdir()
    (root / "probe.adb").write_text('procedure Probe is begin pragma Assert (False); end Probe;\n')
    adc = root / "policy.adc"
    adc.write_text(f"pragma Assertion_Policy ({'Ignore' if policy == 'Ignore' else 'Check'});\n")
    code, text = run([row["executable"], "-c", "-gnat2022", f"-gnatec={adc}", str(root / "probe.adb")], root, root / "compile", trace=True)
    if code:
        raise RuntimeError(text.decode(errors="replace"))
    gnatmake = str(Path(row["executable"]).with_name("gnatmake"))
    code, text = run([gnatmake, "-q", "probe.adb"], root, root / "link", trace=True)
    if code:
        raise RuntimeError(text.decode(errors="replace"))
    code, text = run([str(root / "probe")], root, root / "execute", trace=True)
    if (policy == "Ignore" and code != 0) or (policy != "Ignore" and (code == 0 or b"ASSERTION_ERROR" not in text)):
        raise RuntimeError("actual assertion policy behavior differs")
    results.append(dict(policy=policy, control="assertion-behavior", category="actual-compiler-execution",
                        exit=code, assertion_raised=policy != "Ignore"))
    return results


def verifier_controls(campaign, output, compiler, policy):
    results = []
    variant = campaign / "perf005-providers" / policy
    record = json.loads((variant / "complete.json").read_text())
    expected_hash = digest(variant / "complete.json")
    for kind in ("missing-object", "corrupt-object", "changed-object", "post-preparation-mutation",
                 "cross-policy", "false-preparation", "altered-manifest"):
        root = output / f"{policy}-{kind}"
        shutil.copytree(variant, root)
        for path in root.rglob("*"):
            path.chmod(0o755 if path.is_dir() else 0o644)
        artifact = next(p for p in (root / "artifacts").rglob("*.o"))
        if kind == "missing-object":
            artifact.unlink()
        elif kind in ("corrupt-object", "changed-object", "post-preparation-mutation"):
            artifact.write_bytes(b"corrupted provider object")
            if kind == "post-preparation-mutation":
                changed = copy.deepcopy(record)
                changed["artifacts"] = manifest(root / "artifacts")
                (root / "complete.json").write_text(json.dumps(changed))
        elif kind == "cross-policy":
            record_expected = dict(record["identity"], policy="Ignore" if policy != "Ignore" else "Check")
        else:
            changed = copy.deepcopy(record)
            changed["completed"] = False if kind == "false-preparation" else True
            if kind == "altered-manifest":
                changed["artifacts"].pop(next(iter(changed["artifacts"])))
            (root / "complete.json").write_text(json.dumps(changed))
        # Restore immutability so the intended content/identity control, not a
        # writable-mode rejection, is exercised.
        for p in (root / "artifacts").rglob("*"):
            p.chmod(0o555 if p.is_dir() else 0o444)
        error = rejected(lambda: verify_provider(root, record_expected if kind == "cross-policy" else record["identity"], expected_hash))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    identity = (campaign / "oam-ArraySample-provider-identity.log").read_bytes()
    for name in ("gcc", "system.ads", "libgnat.a"):
        import re
        changed = re.sub(rb"[0-9a-f]{64}(  [^\n]*/" + name.encode() + rb"\n)", rb"0" * 64 + rb"\1", identity)
        if changed == identity:
            raise RuntimeError("identity mutation was a no-op")
        error = rejected(lambda: validate_toolchain(changed, compiler))
        results.append(dict(policy=policy, control="identity-" + name, category="verifier-only", rejection=error))
    profile = campaign / f"oam-ArraySample-{policy}-build.profile.json"
    data = verify_trace(profile)
    command = data["command"]
    consumer = campaign / "oam-ArraySample"
    for kind in ("missing-compile", "duplicate-compile", "missing-binder", "missing-linker", "wrong-policy"):
        changed = copy.deepcopy(data)
        if kind == "missing-compile":
            changed["executions"].remove(drivers(changed)[0])
        elif kind == "duplicate-compile":
            changed["executions"].append(copy.deepcopy(drivers(changed)[0]))
        elif kind in ("missing-binder", "missing-linker"):
            name = "gnatbind" if kind == "missing-binder" else "ld"
            changed["executions"] = [r for r in changed["executions"] if Path(r["executable"]).name != name]
        else:
            for name, value in changed["gpr_temporary_inputs"].items():
                if "Assertion_Policy" in value:
                    changed["gpr_temporary_inputs"][name] = "pragma Assertion_Policy (Check);\n" if policy == "Ignore" else "pragma Assertion_Policy (Ignore);\n"
            if policy == "normal":
                changed["gpr_temporary_inputs"]["/tmp/forced.adc"] = "pragma Assertion_Policy (Check);\n"
                drivers(changed)[0]["argv"].append("-gnatec=/tmp/forced.adc")
        error = rejected(lambda: validate_build(command, consumer, changed, compiler, policy, 0))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    # Copy real trace bytes once; mutate only the disposable profile. Every
    # case is checked against genuine raw execution/read evidence.
    root = output / f"{policy}-retained-inputs"
    root.mkdir()
    disposable = root / "build.profile.json"
    shutil.copyfile(profile.with_name(profile.name.replace(".profile.json", ".strace")), root / "build.strace")
    for kind in ("read-time-map-record", "gpr-config-record", "missing-execution-record"):
        changed = copy.deepcopy(data)
        if kind == "read-time-map-record":
            key = next(iter(changed["pre_execution_mapping_inputs"]))
            changed["pre_execution_mapping_inputs"][key] += "altered%s\naltered.ads\n/wrong/altered.ads\n"
        elif kind == "gpr-config-record":
            backend = next(r for r in changed["executions"] if Path(r["executable"]).name == "gnat1")
            key = next(a.split("=", 1)[1] for a in backend["argv"] if a.startswith("-gnatec="))
            changed["gpr_temporary_inputs"][key] += "pragma Assertion_Policy (Ignore);\n"
        else:
            changed["executions"].pop()
        disposable.write_text(json.dumps(changed))
        error = rejected(lambda: verify_trace(disposable))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    # Reject a pretend successful process or truncated preparation inventory
    # against the preserved genuine provider trace.
    prep = campaign / f"provider-{policy}-build.profile.json"
    root = output / f"{policy}-preparation-evidence"
    root.mkdir()
    shutil.copyfile(prep.with_name(prep.name.replace(".profile.json", ".strace")), root / "build.strace")
    disposable = root / "build.profile.json"
    for kind in ("false-success-preparation", "incomplete-preparation"):
        changed = json.loads(prep.read_text())
        changed["executions"] = [] if kind == "false-success-preparation" else changed["executions"][:-1]
        disposable.write_text(json.dumps(changed))
        error = rejected(lambda: verify_preparation(disposable))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    # Actual child execution witnesses distinguish a printed marker from a
    # skipped consumer. A shell printing it is not the qualified executable.
    root = output / f"{policy}-false-consumer"
    root.mkdir()
    run([sys.executable, "-c", f"print({MARKER.decode().strip()!r})"], root, root / "execute", trace=True)
    error = rejected(lambda: verify_execution(root / "execute.profile.json",
                     consumer / "bin/helper_demand_consumer", MARKER))
    results.append(dict(policy=policy, control="false-success-consumer", category="actual-execution-verifier-rejection", rejection=error))
    root = output / f"{policy}-skipped-consumer"
    root.mkdir()
    run(["/bin/true"], root, root / "execute", trace=True)
    error = rejected(lambda: verify_execution(root / "execute.profile.json",
                     consumer / "bin/helper_demand_consumer", MARKER))
    results.append(dict(policy=policy, control="skipped-consumer", category="actual-execution-verifier-rejection", rejection=error))
    # Mutate actual retained locks, not a newly invented lock schema.
    original_lock = consumer / "alire/alire.lock"
    for kind in ("wrong-dependency-version", "wrong-provider-origin"):
        path = output / f"{policy}-{kind}.lock"
        text = original_lock.read_text()
        text = text.replace('version = "26.0.0"', 'version = "25.0.0"') if kind == "wrong-dependency-version" else text.replace("sha512:cc5dd0", "sha512:000000")
        if text == original_lock.read_text():
            raise RuntimeError("lock mutation was a no-op")
        path.write_text(text)
        error = rejected(lambda: validate_lock(path, compiler))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    roots = sources(consumer)
    changed_root = output / f"{policy}-provider-source-identity"
    changed_root.mkdir()
    (changed_root / "gnatcoll.ads").write_text("changed provider source\n")
    actual = source_identity(dict(roots, gnatcoll_minimal=changed_root))
    if actual == record["identity"]["sources"]:
        raise RuntimeError("provider source mutation accepted")
    results.append(dict(policy=policy, control="provider-source-identity", category="verifier-only", rejected=True))
    for kind in ("missing-generated-object", "stale-generated-object", "duplicate-generated-object"):
        root = output / f"{policy}-{kind}"
        root.mkdir()
        verified = dict(artifacts={})
        for i in range(11):
            p = root / f"{i}.o"
            p.write_bytes(b"disposable artifact")
            verified["artifacts"][str(p)] = digest(p)
        if kind == "missing-generated-object":
            (root / "0.o").unlink()
        elif kind == "stale-generated-object":
            os.utime(root / "0.o", ns=(0, 0))
        else:
            verified["artifacts"][str(root / "../" / root.name / "0.o")] = digest(root / "0.o")
        error = rejected(lambda: verify_artifacts(verified, 1))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    for kind in ("truncated-manifest", "altered-evidence", "added-evidence"):
        root = output / f"{policy}-{kind}"
        root.mkdir()
        (root / "one").write_text("one")
        (root / "two").write_text("two")
        h = seal(root)
        if kind == "truncated-manifest":
            changed = json.loads((root / "evidence-seal.json").read_text())
            changed["files"].pop("two")
            (root / "evidence-seal.json").write_text(json.dumps(changed))
        else:
            (root / ("one" if kind == "altered-evidence" else "three")).write_text("changed")
        error = rejected(lambda: verify_seal(root, h))
        results.append(dict(policy=policy, control=kind, category="verifier-only", rejection=error))
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--campaign", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    args = parser.parse_args()
    args.scratch.mkdir(parents=True, exist_ok=False)
    records = []
    for policy in ("normal", "Check", "Ignore"):
        records += live_replays(args.campaign, args.scratch, policy)
        records += verifier_controls(args.campaign, args.scratch, args.compiler, policy)
        print("CONTROLS", args.compiler, policy, len(records), flush=True)
    (args.scratch / "result.json").write_text(json.dumps(dict(completed=True, compiler=args.compiler,
        records=records, exhaustive=False,
        limitation="Full portable consumer mapping replay and ABI negative controls still required before production"), indent=2) + "\n")


if __name__ == "__main__":
    main()