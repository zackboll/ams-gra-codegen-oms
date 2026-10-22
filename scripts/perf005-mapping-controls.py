#!/usr/bin/env python3
"""Genuine compiler negative controls for the R2 map/source-resolution rule."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess

from perf005_mapping import parse_mapping, reconcile_maps

SPEC = importlib.util.spec_from_file_location("mapping_replay", Path(__file__).with_name("perf005-mapping-experiments.py"))
replay_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(replay_module)


def rejected(call):
    try:
        call()
    except RuntimeError as error:
        return str(error)
    raise RuntimeError("negative mapping validation unexpectedly passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=False, parents=True)
    results = []
    for compiler in (14, 16):
        campaign = args.evidence / "r1" / f"original-v2{compiler}"
        gcc = (campaign / "oam-ArraySample-provider-identity.log").read_text().splitlines()[0]
        for policy in ("normal", "Check", "Ignore"):
            root = args.output / f"{compiler}-{policy}"
            root.mkdir()
            for directory in ("good", "redirected"):
                (root / directory).mkdir()
            good, bad = root / "good/dep.ads", root / "redirected/dep.ads"
            good.write_text("package Dep is Value : constant Integer := 1; end Dep;\n")
            bad.write_text("package Dep is Value : constant Integer := 2; end Dep;\n")
            source = root / "probe.adb"
            source.write_text("with Dep; function Probe return Integer is begin return Dep.Value; end Probe;\n")
            adc = root / "policy.adc"
            adc.write_text("" if policy == "normal" else f"pragma Assertion_Policy ({policy});\n")
            record = dict(argv=[gcc, "-c", "-x", "ada", "-gnatA", "-O2", "-gnatn", "-gnatwaCJ",
                                f"-gnatec={adc}", "-gnatem=unused", str(source)])
            data = dict(gpr_temporary_inputs={str(adc): adc.read_text()})
            original = f"dep%s\ndep.ads\n{good}\n"
            redirected = f"dep%s\ndep.ads\n{bad}\n"
            variants = {"original": original, "redirected-same-basename": redirected,
                        "spec-body": original.replace("dep%s", "dep%b"),
                        "conflict-good-last": redirected + original,
                        "conflict-bad-last": original + redirected,
                        "malformed": "dep%s\ndep.ads\n", "missing": ""}
            runs = {name: replay_module.replay(data, record, text, root / name)
                    for name, text in variants.items()}
            a, b = runs["original"], runs["redirected-same-basename"]
            if (a["exit"] != 0 or b["exit"] != 0 or a["reads"] == b["reads"]
                    or a["object_sha256"] == b["object_sha256"]):
                raise RuntimeError("redirection did not change genuine dependency resolution")
            # A conflicting unit really is order-dependent in both compilers.
            if (runs["conflict-good-last"]["object_sha256"] != a["object_sha256"]
                    or runs["conflict-bad-last"]["object_sha256"] != b["object_sha256"]):
                raise RuntimeError("unexpected compiler conflict precedence")
            failures = {}
            for name in ("spec-body", "conflict-good-last", "conflict-bad-last", "malformed", "missing"):
                if name == "spec-body":
                    # Syntactically valid, but not equivalent to the required spec.
                    failures[name] = rejected(lambda: reconcile_maps(original, variants[name], root, root,
                                                                      a["reads"], runs[name]["reads"], [gcc, gcc]))
                else:
                    failures[name] = rejected(lambda name=name: parse_mapping(variants[name]))
            failures["redirected-same-basename"] = rejected(lambda: reconcile_maps(
                original, redirected, root, root, a["reads"], b["reads"], [gcc, gcc]))
            failures["changed-dependency-resolution"] = rejected(lambda: reconcile_maps(
                original, original, root, root, a["reads"], b["reads"], [gcc, gcc]))
            # Unchanged path/map but mutated bytes must fail against read-time evidence.
            good.write_text("package Dep is Value : constant Integer := 3; end Dep;\n")
            from perf005_mapping import source_reads
            source_reads.cache_clear()
            from perf005_profile import executions
            trace = root / "original/compiler.strace"
            pid = next(r["pid"] for r in executions(trace) if Path(r["executable"]).name == "gnat1")
            failures["changed-source-hash"] = rejected(lambda: source_reads(trace, frozenset([pid])))
            assertion = root / "assertion_probe.adb"
            assertion.write_text("procedure Assertion_Probe is begin pragma Assert (False); end Assertion_Probe;\n")
            executable = root / "assertion_probe"
            naming = root / "naming.adc"
            naming.write_text('pragma Source_File_Name_Project (Spec_File_Name => "*.ads", Casing => lowercase, Dot_Replacement => "-");\n'
                              'pragma Source_File_Name_Project (Body_File_Name => "*.adb", Casing => lowercase, Dot_Replacement => "-");\n')
            build = subprocess.run([str(Path(gcc).with_name("gnatmake")), "-q", "-o", str(executable),
                                    str(assertion), "-cargs", "-gnatA", "-O2", "-gnatn", "-gnatwaCJ",
                                    f"-gnatec={adc}", f"-gnatec={naming}"], cwd=root, capture_output=True)
            if build.returncode:
                raise RuntimeError(build.stderr.decode())
            execution = subprocess.run([str(executable)], cwd=root, capture_output=True)
            if ((policy == "Check") != (execution.returncode != 0)
                    or (policy == "Check" and b"ADA.ASSERTIONS.ASSERTION_ERROR" not in execution.stderr)):
                raise RuntimeError("genuine assertion-policy execution differs")
            (root / "assertion-build.stderr").write_bytes(build.stderr)
            (root / "assertion-run.stderr").write_bytes(execution.stderr)
            results.append(dict(compiler=compiler, policy=policy, negative_controls=failures,
                                compiler_exits={name: r["exit"] for name, r in runs.items()},
                                conflicting_entries_are_order_dependent=True,
                                assertion_execution_exit=execution.returncode))
            (args.output / "result.json").write_text(json.dumps(results, indent=2) + "\n")
            print(f"GNAT{compiler} {policy}: {len(failures)} negative controls PASS", flush=True)


if __name__ == "__main__":
    main()