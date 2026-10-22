#!/usr/bin/env python3
"""Fail-closed full Float32 campaign equivalence and forced-compile inventory."""
import argparse
import hashlib
import json
from pathlib import Path
import re

from perf005_provider import source_identity, sources, verify_variant
from perf005_profile import executions

NAMES = ["ArraySample", "Array_Encode_Hex_Binary", "Encode_Hex_Binary",
         "Array_encode_hex_binary", "Array_Decode_Hex_Binary"]
POLICIES = ["normal", "Check", "Ignore"]
REQUIRED = {"helper_demand_consumer.adb", "service_codec.adb", "programs.ads",
            "ams_gra_oms_json_float32.adb", "ams_gra_oms_json_guard.adb",
            "ams_gra_oms_json_occurrences.adb", "ams_gra_float32.c"}


def normalized(text, root):
    return text.replace(str(root) + "-state", "<STATE>").replace(str(root), "<CAMPAIGN>")


def provider_commands(trace):
    result = []
    for record in executions(trace):
        argv = record["argv"]
        if "-c" not in argv or not re.search(r"(?:^|-)gcc(?:-\d+)?$", Path(record["executable"]).name):
            continue
        if not any("gnatcoll_" in arg and arg.endswith((".adb", ".ads", ".c", ".s", ".S")) for arg in argv):
            continue
        options = []
        for arg in argv[1:]:
            if arg.startswith(("-gnatem=", "-specs=/tmp/", "@/tmp/", "-gnatec=/tmp/")):
                continue
            if arg.startswith("-gnatec="):
                arg = "-gnatec=<POLICY>"
            if "/alire/cache/dependencies/" in arg:
                arg = "<DEPENDENCIES>/" + arg.split("/alire/cache/dependencies/", 1)[1]
            options.append(arg)
        result.append(options)
    return sorted(result)


def compare(baseline, candidate, traced=None, attribution=None):
    left = json.loads((baseline / "result.json").read_text())
    right = json.loads((candidate / "result.json").read_text())
    if left != right or left.get("completed") is not True or left.get("red") is not False:
        raise RuntimeError("ordered outcomes differ/incomplete")
    expected = [(profile, name, policy) for profile in ("oam", "qualified") for name in NAMES for policy in POLICIES]
    actual = [(r["profile"], r["name"], r["policy"]) for r in left["records"]]
    if actual != expected:
        raise RuntimeError("incomplete or unordered campaign")
    if normalized((baseline / "runtime.log").read_text(), baseline) != normalized((candidate / "runtime.log").read_text(), candidate):
        raise RuntimeError("runtime identity mismatch")
    provenance = json.loads((candidate / "perf005-provider-consumers.json").read_text())
    if [p["cell"] for p in provenance] != [f"{p}-{n}-{a}-build" for p, n, a in expected]:
        raise RuntimeError("missing/duplicate provider provenance")
    compile_counts = {}
    for profile, name, policy in expected:
        label = f"{profile}-{name}"
        cell = f"{label}-{policy}"
        ld, rd = baseline / label, candidate / label
        if policy == "normal":
            for location in (label, label + "-plain", label + "-support"):
                def files(root):
                    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
                            for p in root.rglob("*") if p.is_file() and p.suffix in (".adb", ".ads", ".c", ".gpr", ".toml")
                            and not any(part in ("alire", "obj", "obj-float32", "bin") for part in p.relative_to(root).parts)}
                if files(baseline / location) != files(candidate / location):
                    raise RuntimeError(f"generated/support/configuration bytes differ: {location}")
            if source_identity(sources(ld)) != source_identity(sources(rd)):
                raise RuntimeError("resolved provider sources differ")
            for suffix in ("portable-reject", "portable-generate-reject"):
                filename = f"{label}-{suffix}"
                if normalized((baseline / (filename + ".log")).read_text(), baseline) != normalized((candidate / (filename + ".log")).read_text(), candidate):
                    raise RuntimeError("negative CLI diagnostic differs")
                for root in (baseline, candidate):
                    if (root / (label + "-rejected")).exists():
                        raise RuntimeError("non-atomic negative CLI rejection")
                    if json.loads((root / (filename + ".profile.json")).read_text())["exit"] != 1:
                        raise RuntimeError("negative CLI outcome differs")
        for filename in (f"{cell}-alire.lock", f"{cell}-gnat.log", f"{cell}-execute.log"):
            if normalized((baseline / filename).read_text(), baseline) != normalized((candidate / filename).read_text(), candidate):
                raise RuntimeError(f"lock/compiler/runtime outcome differs: {filename}")
        if (candidate / f"{cell}-execute.log").read_bytes() != b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n":
            raise RuntimeError("completion marker differs")
        if policy != "normal" and (baseline / f"{cell}.adc").read_bytes() != (candidate / f"{cell}.adc").read_bytes():
            raise RuntimeError("assertion policy differs")
        record = json.loads((candidate / f"{cell}-build.profile.json").read_text())
        if record["exit"] != 0 or "-f" not in record["command"] or "-XEXTERNALLY_BUILT=true" not in record["command"]:
            raise RuntimeError("forced external-provider consumer build missing")
        text = (candidate / f"{cell}-build.log").read_text()
        units = re.findall(r"\[(?:Ada|C)\]\s+(\S+\.(?:adb|ads|c))", text)
        baseline_units = re.findall(r"\[(?:Ada|C)\]\s+(\S+\.(?:adb|ads|c))", (baseline / f"{cell}-build.log").read_text())
        owned_names = {p.name for location in (ld, baseline / (label + "-support"))
                       for p in location.rglob("*") if p.suffix in (".adb", ".ads", ".c")
                       and "alire" not in p.relative_to(location).parts}
        baseline_owned = set(baseline_units) & owned_names
        if not REQUIRED <= set(units) or any("gnatcoll" in unit for unit in units):
            raise RuntimeError(f"required compilation missing or provider rebuilt: {cell}")
        if not set(units) <= baseline_owned or not baseline_owned <= set(units):
            raise RuntimeError(f"baseline/candidate project-owned compilation differs: {cell}")
        compile_counts[cell] = len(units)
        item = provenance[len(compile_counts) - 1]
        variant = Path(item["provider"])
        provider = json.loads((variant / "complete.json").read_text())
        if item["policy"] != policy or provider["identity"]["policy"] != policy:
            raise RuntimeError("wrong provider variant")
        verify_variant(variant, provider["identity"])
        if hashlib.sha256((variant / "complete.json").read_bytes()).hexdigest() != item["manifest_sha256"]:
            raise RuntimeError("provider provenance corrupt")
        if traced:
            trace_record = json.loads((traced / f"{cell}-build.profile.json").read_text())
            invocations = [v["argv"] for v in trace_record["executions"]
                           if "-c" in v["argv"] and re.search(r"(?:^|-)gcc(?:-\d+)?$", Path(v["executable"]).name)]
            compiled = {Path(arg).name for cmd in invocations for arg in cmd if arg.endswith((".adb", ".ads", ".c"))}
            if not REQUIRED <= compiled or any("gnatcoll" in unit for unit in compiled):
                raise RuntimeError(f"executed compiler coverage differs: {cell}")
            for cmd in invocations:
                if any(Path(arg).name in REQUIRED for arg in cmd):
                    policies = [arg for arg in cmd if arg.startswith("-gnatec=") and arg.endswith(".adc")]
                    if (policy == "normal" and policies) or (policy != "normal" and not policies):
                        # C is intentionally unaffected by Ada policy switches.
                        if not any(arg.endswith(".c") for arg in cmd):
                            raise RuntimeError("compiler policy propagation missing")
    if traced:
        if json.loads((traced / "result.json").read_text()) != right:
            raise RuntimeError("traced positive outcomes differ")
        compare(baseline, traced)
    if attribution:
        if not traced:
            raise RuntimeError("provider command equivalence requires traced preparation")
        for policy in POLICIES:
            original = provider_commands(attribution / f"oam-ArraySample-{policy}-build.strace")
            prepared = provider_commands(traced / f"provider-{policy}-build.strace")
            if len(original) != 106 or original != prepared:
                raise RuntimeError(f"actual provider compiler options/source inventory differ: {policy}")
    for policy in POLICIES:
        text = (candidate / f"provider-{policy}-build.log").read_text()
        if "libgnatcoll_minimal.a" not in text or "libgnatcoll_core.a" not in text:
            raise RuntimeError("provider preparation archive evidence missing")
    return dict(completed=True, positive_cells=30, negative_cli_cases=20,
                provider_constructions=3, generated_support_compile_counts=compile_counts,
                actual_compiler_coverage_verified=traced is not None,
                provider_command_equivalence_verified=attribution is not None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--traced", type=Path)
    parser.add_argument("--attribution", type=Path, help="original representative traces for provider command equivalence")
    args = parser.parse_args()
    print(json.dumps(compare(args.baseline.resolve(), args.candidate.resolve(),
                             args.traced.resolve() if args.traced else None,
                             args.attribution.resolve() if args.attribution else None), indent=2))


if __name__ == "__main__":
    main()