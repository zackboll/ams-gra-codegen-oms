#!/usr/bin/env python3
"""Reparse full external Task114 artifacts and complete raw interception boundaries."""
import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task112 import verify as base
from task114 import recovery

FORMAT = "task114-v1"


def verify(directory):
    receipt = base.verify(directory, sys.modules[__name__])
    for release, cell in receipt["releases"].items():
        folder = directory / ("recovery" + release)
        recovery.validate_observer(json.loads((folder / "observer.json").read_text()), release)
        for row in cell["epochs"]:
            epoch = row["epoch"]
            recovery.validate_boundary((folder / f"epoch{epoch}-ada.bin").read_bytes(),
                                       (folder / f"epoch{epoch}-forwarded.bin").read_bytes(), row)
        output = (folder / "consumer.stdout").read_text()
        cell["consumer_observations"] = [line for line in output.splitlines() if line.startswith(
            ("CALL ", "SUBSCRIBED ", "PENDING ", "ARMED ", "TRANSPORT_EXCEPTION ", "RETIRED ", "CLEANUP ",
             "CONTROL_UNSUB ", "PROBE_REJECTED ", "TASK114_COMPLETE "))]
    return receipt

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    receipt = verify(args.directory.resolve())
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print("TASK114_VERIFIED", receipt["compiler"], receipt["backend"])