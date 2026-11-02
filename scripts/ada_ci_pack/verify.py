#!/usr/bin/env python3
"""Independently re-read a pack's original logs/artifacts and every worker exit."""
import argparse
import importlib.util
import json
from pathlib import Path

try:
    from . import run
except ImportError:
    import run


def verify(root, expected_sha, expected_tree, pack):
    summary = json.loads((root / "summary.json").read_text())
    source = summary["source"]
    if source["sha"] != expected_sha or source["tree"] != expected_tree:
        raise RuntimeError("pack source identity mismatch")
    if summary["exit"] != 0 or summary["pack"] != pack or summary["runner_slots"] != 1:
        raise RuntimeError("pack did not complete successfully")
    if summary["generator_sha256"] != run.sha(run.ROOT / "target/debug/ams-gra-codegen-oms"):
        raise RuntimeError("generator identity mismatch")
    names = run.INVENTORY["packs"][pack]
    if [worker["suite"] for worker in summary["workers"]] != names:
        raise RuntimeError("missing/duplicate constituent worker")
    records = [("ada-json-guard", root / "seed", summary["guard"])]
    records.extend((name, root / name, summary["workers"][i]) for i, name in enumerate(names))
    for name, directory, recorded in records:
        completion = json.loads((directory / "completion.json").read_text())
        if completion != recorded or completion["suite"] != name or completion["exit"] != 0 or completion["source"] != source:
            raise RuntimeError("failed/mismatched worker: " + name)
        steps = run.INVENTORY["suites"][name]["steps"]
        if len(completion["steps"]) != len(steps):
            raise RuntimeError("missing constituent step: " + name)
        for i, step in enumerate(steps):
            record = json.loads((directory / f"step-{i}.json").read_text())
            if (record["exit"] != 0 or record["reason"] is not None
                    or (directory / f"step-{i}.exit").read_text() != "0\n"
                    or record["command"][-1] != step["run"]
                    or completion["steps"][i] != dict(name=step["name"], **record)):
                raise RuntimeError("failed/mismatched step: " + name)
            # A successful shell-only assertion step may legitimately print nothing.
            if not (directory / f"step-{i}.log").is_file():
                raise RuntimeError("missing step log: " + name)
        run.verify_artifacts(name, directory)
        if run.verify_markers(name, directory) != completion["evidence"]:
            raise RuntimeError("independent evidence identity mismatch: " + name)
        if name == "ada-json-guard":
            spec = importlib.util.spec_from_file_location("guard", run.ROOT / "scripts/check-task087a.py")
            guard = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(guard)
            for policy in ("normal", "check", "ignore"):
                guard.validate_output((directory / "task087a-qualification" / (policy + "-run.log")).read_bytes())
        trace = [json.loads(line) for line in (directory / "alr.jsonl").read_text().splitlines()]
        if not trace or not any("build" in row["argv"] for row in trace):
            raise RuntimeError("missing toolchain execution trace: " + name)
        for row in trace:
            if row["argv"][1] != "--settings=" + str(directory / "settings"):
                raise RuntimeError("shared settings access: " + name)
            if "build" in row["argv"] and row["argv"][row["argv"].index("--") + 1] != "-j1":
                raise RuntimeError("unbounded inner compilation: " + name)
    samples = [json.loads(line) for line in (root / "resources.jsonl").read_text().splitlines()]
    if len(samples) < 2 or min(row["disk_free_bytes"] for row in samples) <= 0:
        raise RuntimeError("missing/invalid resource evidence")
    return dict(source=source, pack=pack, verified_suites=[name for name, _, _ in records],
                concurrency=summary["concurrency"], generator_sha256=summary["generator_sha256"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--expected-sha", required=True)
    parser.add_argument("--expected-tree", required=True)
    parser.add_argument("--pack", choices=["a", "b"], required=True)
    args = parser.parse_args()
    result = verify(args.scratch.resolve(), args.expected_sha, args.expected_tree, args.pack)
    run.write(args.scratch / "independently-verified.json", result)
    print(json.dumps(result))