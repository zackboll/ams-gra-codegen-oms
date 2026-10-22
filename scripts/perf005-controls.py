#!/usr/bin/env python3
"""Disposable generated/support compile-failure controls against a prepared pilot.

Does not mutate positive evidence. Run within the same campaign toolchain state.
"""
import argparse
import json
import os
from pathlib import Path
import shutil

from perf005_profile import run
from perf005_provider import verify_variant


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("campaign", type=Path)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    campaign = args.campaign.resolve()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    label = "oam-ArraySample"
    controls = [("generated", "service_codec.adb"),
                ("support-ada", "float32/ams_gra_oms_json_float32.adb"),
                ("support-c", "float32/ams_gra_float32.c")]
    results = []
    for policy in ("normal", "Check", "Ignore"):
        provider = campaign / "perf005-providers" / policy
        record = json.loads((provider / "complete.json").read_text())
        verify_variant(provider, record["identity"])
        for kind, filename in controls:
            root = scratch / f"{policy}-{kind}"
            root.mkdir()
            consumer = root / label
            support = root / (label + "-support")
            shutil.copytree(campaign / label, consumer)
            shutil.copytree(campaign / (label + "-support"), support)
            # Pin is already relative to this matching disposable support copy.
            source = consumer / filename if kind == "generated" else support / filename
            source.write_text(("#error PERF005_REQUIRED_COMPILE\n" if kind == "support-c"
                               else 'pragma Compile_Time_Error (True, "PERF005_REQUIRED_COMPILE");\n') + source.read_text())
            command = ["alr", "-n", "build", "--", "-f", "-XEXTERNALLY_BUILT=true",
                       f"-XGNATCOLL_OBJECT_ROOT={provider}/artifacts/"]
            if policy != "normal":
                command += ["-cargs:Ada", f"-gnatec={provider}/policy.adc"]
            code, output = run(command, consumer, root / "failure", trace=True,
                               writable=[consumer, support, Path(os.environ["HOME"]),
                                         Path(shutil.which("alr")).parent.parent, Path("/tmp"), Path("/dev")])
            if code == 0 or b"PERF005_REQUIRED_COMPILE" not in output:
                raise RuntimeError(f"required compiler failure did not propagate: {policy}/{kind}")
            trace = json.loads((root / "failure.profile.json").read_text())
            if not any(str(source) in item["argv"] and item["exit"] not in (None, 0)
                       for item in trace["executions"]):
                raise RuntimeError("missing actual source/compiler failure evidence")
            verify_variant(provider, record["identity"])
            results.append(dict(policy=policy, kind=kind, exit=code, compiler_failure_verified=True))
    (scratch / "result.json").write_text(json.dumps(dict(completed=True, controls=results), indent=2) + "\n")
    print(f"PERF005_FAILURE_CONTROLS_COMPLETE controls={len(results)}")


if __name__ == "__main__":
    main()