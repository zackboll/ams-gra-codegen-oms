#!/usr/bin/env python3
"""Replay every retained provider Ada invocation under both unfiltered maps.

Run in the pinned PERF-005 container. Outputs must be new. No retained input is
modified. This is correctness qualification, not a performance campaign.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from perf005_evidence import canonical, drivers, input_text, load_profile, sha
from perf005_mapping import reconcile_policy, source_reads


def replay(data, record, mapping, output):
    output.mkdir(parents=True, exist_ok=False)
    argv = []
    for arg in record["argv"]:
        if arg.startswith("-gnatec="):
            name = arg.split("=", 1)[1]
            path = output / f"config-{len(argv)}.adc"
            path.write_text(input_text(data, name))
            # Identical retained ADC content gets identical mtime; ALI D records
            # otherwise differ just because the replay copies were made later.
            os.utime(path, (1700000000, 1700000000))
            arg = f"-gnatec={path}"
        elif arg.startswith("-gnatem="):
            path = output / "mapping.input"
            path.write_text(mapping)
            arg = f"-gnatem={path}"
        argv.append(arg)
    trace = output / "compiler.strace"
    env = dict(os.environ, LC_ALL="C")
    for key in list(env):
        if key.startswith(("ADA_", "GPR_", "GCC_")):
            del env[key]
    # All project-owned units are mapped. The only unmapped sources in these
    # retained maps are conventional units from the qualified default runtime.
    result = subprocess.run(["strace", "-f", "-ttt", "-yy", "-s", "1048576",
                             "-e", "trace=execve,read,openat", "-o", str(trace), "--", *argv],
                            cwd=output, env=env, capture_output=True)
    (output / "stdout").write_bytes(result.stdout)
    (output / "stderr").write_bytes(result.stderr)
    from perf005_profile import executions
    backends = [r for r in executions(trace) if Path(r["executable"]).name == "gnat1"]
    if len(backends) != 1:
        raise RuntimeError("replay missing backend")
    reads = source_reads(trace, frozenset([backends[0]["pid"]])).get(backends[0]["pid"], {})
    alis = list(output.glob("*.ali"))
    objects = list(output.glob("*.o"))
    def normalize(text):
        return text.replace(str(output), "<OUTPUT>")
    report = dict(command=argv, exit=result.returncode,
                  stdout=normalize(result.stdout.decode()), stderr=normalize(result.stderr.decode()),
                  reads=reads, ali=normalize(alis[0].read_text()) if len(alis) == 1 else None,
                  object_sha256=sha(objects[0]) if len(objects) == 1 else None,
                  trace_sha256=sha(trace), input_mapping_sha256=hashlib.sha256(mapping.encode()).hexdigest())
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    summary = []
    for compiler in (14, 16):
        original, pilot = [args.evidence / "r1" / n for n in (f"original-v2{compiler}", f"pilot{compiler}")]
        for policy in ("normal", "Check", "Ignore"):
            reports = reconcile_policy(original, pilot, policy)
            data = load_profile(original / f"oam-ArraySample-{policy}-build.profile.json")
            prepared = load_profile(pilot / f"provider-{policy}-build.profile.json")
            records = {canonical(r["argv"][-1], original): r for r in drivers(data)
                       if r["argv"][-1].endswith((".ads", ".adb"))
                       and "/alire/cache/dependencies/gnatcoll_" in r["argv"][-1]}
            pilot_maps = {canonical(k.split("\n")[0], pilot): v
                          for k, v in prepared["pre_execution_mapping_inputs"].items()}
            for i, report in enumerate(reports):
                source = report["source"]
                record = records[source]
                mapping_name = next(a.split("=", 1)[1] for a in record["argv"] if a.startswith("-gnatem="))
                left_map = data["pre_execution_mapping_inputs"][record["argv"][-1] + "\n" + mapping_name]
                # Only documented campaign/state-root relocation. Preserve all
                # entries, kind, order, filename and content; never filter.
                right_map = pilot_maps[source].replace(str(pilot) + "-state", str(original) + "-state").replace(str(pilot), str(original))
                stem = args.output / f"{compiler}-{policy}" / f"{i:03d}"
                left = replay(data, record, left_map, stem / "original")
                right = replay(data, record, right_map, stem / "pilot")
                equal = (left["exit"] == right["exit"] == 0
                         and left["stdout"] == right["stdout"] and left["stderr"] == right["stderr"]
                         and bool(left["reads"]) and left["reads"] == right["reads"]
                         and {canonical(p, original): h for p, h in left["reads"].items()} == report["source_reads"]
                         and left["ali"] is not None and left["ali"] == right["ali"]
                         and left["object_sha256"] is not None and left["object_sha256"] == right["object_sha256"])
                item = dict(compiler=compiler, policy=policy, source=source, equivalent=equal,
                            evidence=str(stem), original_mapping_sha256=report["original_sha256"],
                            pilot_mapping_sha256=report["pilot_sha256"])
                summary.append(item)
                (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
                if not equal:
                    raise RuntimeError(f"mapping replay differs: {stem}")
            (args.output / f"mapping-{compiler}-{policy}.json").write_text(json.dumps(reports, indent=2) + "\n")
            print(f"GNAT{compiler} {policy}: {len(reports)} paired invocations PASS", flush=True)


if __name__ == "__main__":
    main()