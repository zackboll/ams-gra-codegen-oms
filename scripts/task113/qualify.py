#!/usr/bin/env python3
"""Task113 profile of the immutable Task112 qualification builder."""
import argparse
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task112 import qualify as base

TEST_DIRECTORY = "tests/ada-wss-subscription-recovery"
RECOVERY_SCRIPT = "scripts/task113/recovery.py"
TASK_NAME = "TASK113"

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "deps", "schema25", "schema26", "generator"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    parser.add_argument("--release", choices=("25", "26"), nargs="+", default=["26"])
    parser.add_argument("--development", action="store_true", help="diagnostic only, not acceptance")
    base.execute(parser.parse_args(), sys.modules[__name__])