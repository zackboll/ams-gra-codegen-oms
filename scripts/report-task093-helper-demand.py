#!/usr/bin/env python3
"""Summarize completed benchmark measurements and per-operation costs."""
import argparse
from collections import defaultdict
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("campaigns", nargs="+", type=Path)
    args = parser.parse_args()
    report = []
    for root in args.campaigns:
        measurements = []
        for path in sorted(root.glob("*.measurement.json")):
            measurement = json.loads(path.read_text())
            label = measurement["label"]
            campaign = root / label
            result = json.loads((campaign / "result.json").read_text())
            if measurement["exit"] != 0 or result["completed"] is not True:
                raise RuntimeError(f"incomplete/failed measurement: {path}")
            groups = defaultdict(lambda: dict(commands=0, wall_seconds=0, cpu_seconds=0))
            negatives = 0
            for command_path in campaign.rglob("*.command.json"):
                command = json.loads(command_path.read_text())
                if "elapsed_seconds" not in command:
                    continue
                if "-worker." in command_path.name:
                    continue  # Would double count child measurements.
                label_name = command_path.name
                if "build" in label_name:
                    group = "forced_compilation_linking"
                elif "execute" in label_name:
                    group = "generated_execution"
                elif any(key in label_name for key in ["toolchain", "preflight", "isolation", "gnat", "alire-paths"]):
                    group = "alire_selection_identity_resolution"
                elif "runtime" in label_name:
                    group = "runtime_identity"
                else:
                    group = "generator_cli"
                row = groups[group]
                row["commands"] += 1
                row["wall_seconds"] += command["elapsed_seconds"]
                row["cpu_seconds"] += command.get("cpu_seconds", 0)
                if command.get("expected") == 1:
                    if command["exit"] != 1:
                        raise RuntimeError(f"negative control failed: {command_path}")
                    negatives += 1
            measurement.update(positive_cells=len(result["records"]),
                               negative_cli_cases=negatives, operations=dict(groups))
            measurements.append(measurement)
        baselines = {row["label"].rsplit("-w", 1)[0]: row["wall_seconds"]
                     for row in measurements if row["label"].endswith("-woriginal")}
        for row in measurements:
            key = row["label"].rsplit("-w", 1)[0]
            if key in baselines:
                row["speedup_vs_original"] = baselines[key] / row["wall_seconds"]
        report.append(dict(root=str(root), measurements=measurements))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()