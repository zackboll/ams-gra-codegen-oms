#!/usr/bin/env python3
"""Local bounded-CPU serial/parallel matrix; never dispatches hosted CI.

Each invocation must finish, verify provenance, and compare equivalent before
the final summary is written. Interrupted runs retain individual measurements
but are not a completed benchmark. Run without competing qualification jobs.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
from task093_resources import Measurement
import importlib.util

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("equivalence", ROOT / "scripts/compare-task093-helper-demand.py")
equivalence = importlib.util.module_from_spec(spec)
spec.loader.exec_module(equivalence)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True, type=Path)
    parser.add_argument("--generator", required=True, type=Path)
    parser.add_argument("--cpus", required=True, help="exact four-CPU Linux affinity, e.g. 0,1,2,3")
    parser.add_argument("--original", type=Path, help="optional unchanged original helper-demand script")
    parser.add_argument("--selection", choices=["portable-14", "float32-14", "float32-16"],
                        help="benchmark one complete campaign on its own four-CPU budget")
    args = parser.parse_args()
    cpus = {int(value) for value in args.cpus.split(",")}
    if len(cpus) != 4 or not cpus <= os.sched_getaffinity(0):
        parser.error("four distinct available logical CPUs required")
    os.sched_setaffinity(0, cpus)
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    rows = []
    modes = [(str(count), ROOT / "scripts/check-task093-helper-demand.py") for count in [1, 2, 4]]
    if args.original:
        modes.insert(0, ("original", args.original.resolve()))
    metadata = dict(affinity=sorted(cpus), platform=subprocess.check_output(["uname", "-a"], text=True).strip(),
                    libc=subprocess.check_output(["getconf", "GNU_LIBC_VERSION"], text=True).strip(),
                    revision=subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"], text=True).strip(),
                    tracked_diff_sha256=equivalence.hashlib.sha256(subprocess.check_output(["git", "-C", str(ROOT), "diff", "HEAD"])).hexdigest())
    (scratch / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    for compiler, family in [("14", "portable"), ("14", "float32"), ("16", "float32")]:
        if args.selection and args.selection != f"{family}-{compiler}":
            continue
        reference = None
        for mode, script in modes:
            label = f"{family}-{compiler}-w{mode}"
            destination = scratch / label
            command = [sys.executable, str(script), "--compiler", compiler, "--family", family,
                       "--generator", str(args.generator.resolve()), "--scratch", str(destination)]
            env = dict(os.environ, AMS_GRA_TASK093_WORKERS="1" if mode == "original" else mode)
            measurement = Measurement()
            with (scratch / f"{label}.log").open("xb") as log:
                result = subprocess.run(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
            row = dict(label=label, exit=result.returncode, command=command, **measurement.finish())
            (scratch / f"{label}.measurement.json").write_text(json.dumps(row, indent=2) + "\n")
            if result.returncode:
                raise RuntimeError(f"benchmark failed: {label}; see retained log")
            equivalence.verify(destination)
            if reference is None:
                reference = destination
            row["equivalent_items"] = equivalence.compare(reference, destination)
            row["records"] = len(json.loads((destination / "result.json").read_text())["records"])
            rows.append(row)
            print(json.dumps(row), flush=True)
        # Per-campaign complete evidence is useful even if a later, independent
        # compiler campaign fails. It is not a substitute for the final matrix.
        (scratch / f"{family}-{compiler}-summary.json").write_text(json.dumps(
            dict(completed=True, environment=metadata,
                 measurements=[row for row in rows if row["label"].startswith(f"{family}-{compiler}-")]), indent=2) + "\n")
    (scratch / "summary.json").write_text(json.dumps(dict(completed=True, environment=metadata, measurements=rows), indent=2) + "\n")


if __name__ == "__main__":
    main()