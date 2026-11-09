#!/usr/bin/env python3
"""Reparse full external Task113 artifacts and emit a compact receipt."""
import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task112 import verify as base
from task113 import recovery

FORMAT = "task113-v1"

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    receipt = base.verify(args.directory.resolve(), sys.modules[__name__])
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print("TASK113_VERIFIED", receipt["compiler"], receipt["backend"])