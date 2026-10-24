#!/usr/bin/env python3
"""Summarize PERF-005 measured CPU, residuals and actual compiler executions."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re

from perf005_profile import executions, phase


def report(root):
    result = dict(campaign=str(root), commands=[], phases_sampled_self_cpu_seconds={},
                  compiler_invocations={}, archive_invocations=0)
    phases = Counter()
    compiles = Counter()
    outer_cpu = 0
    build_wall = 0
    preparation_cpu = 0
    consumer_cpu = 0
    for path in sorted(root.glob("*build.profile.json")):
        data = json.loads(path.read_text())
        outer_cpu += data["cpu_seconds"]
        if path.name.startswith("provider-"):
            preparation_cpu += data["cpu_seconds"]
        else:
            consumer_cpu += data["cpu_seconds"]
        build_wall += data["elapsed_seconds"]
        phases.update(data["sampled_self_cpu_seconds"])
        trace = Path(str(path).removesuffix(".profile.json") + ".strace")
        items = executions(trace) if trace.exists() else []
        for item in items:
            name = Path(item["executable"]).name
            if re.search(r"(?:^|-)gcc(?:-\d+)?$", name) and "-c" in item["argv"]:
                argv = item["argv"]
                if any(arg.endswith((".adb", ".ads", ".c", ".s", ".S")) for arg in argv):
                    compiles[phase(argv)] += 1
            if name == "ar" or name.endswith("-ar"):
                result["archive_invocations"] += 1
        result["commands"].append(dict(file=path.name, elapsed_seconds=data["elapsed_seconds"],
                                       cpu_seconds=data["cpu_seconds"], exit=data["exit"],
                                       sampled_peak_tree_rss_kib=data["sampled_peak_tree_rss_kib"]))
    result.update(phases_sampled_self_cpu_seconds=dict(phases), compiler_invocations=dict(compiles),
                  outer_build_cpu_seconds=outer_cpu, sequential_build_elapsed_seconds=build_wall,
                  provider_preparation_cpu_seconds=preparation_cpu,
                  consumer_build_cpu_seconds=consumer_cpu,
                  residual_build_cpu_seconds=outer_cpu - sum(phases.values()) if phases else None,
                  provider_sampled_fraction_of_outer_build_cpu=(phases["gnatcoll-minimal"] + phases["gnatcoll-core"]) / outer_cpu if outer_cpu and phases else None)
    total = root / "perf005-total.json"
    if total.exists():
        result["campaign_total"] = json.loads(total.read_text())
        result["non_build_child_cpu_seconds"] = result["campaign_total"]["child_cpu_seconds"] - outer_cpu
        result["non_build_elapsed_seconds"] = result["campaign_total"]["elapsed_seconds"] - build_wall
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("campaign", type=Path)
    args = parser.parse_args()
    print(json.dumps(report(args.campaign.resolve()), indent=2))


if __name__ == "__main__":
    main()