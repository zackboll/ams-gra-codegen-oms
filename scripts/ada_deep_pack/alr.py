#!/usr/bin/env python3
"""Preserve Alire settings/pins; bound nested builds in packed workers only."""
import json
import os
import subprocess
import sys


def arguments(argv):
    result = list(argv)
    if "build" in result:
        if "--" not in result:
            result.append("--")
        index = result.index("--") + 1
        result.insert(index, "-j1")
    return [os.environ["ADA_DEEP_REAL_ALR"], *result]


if __name__ == "__main__":
    command = arguments(sys.argv[1:])
    with open(os.environ["ADA_DEEP_ALR_TRACE"], "a") as stream:
        stream.write(json.dumps(command) + "\n")
    raise SystemExit(subprocess.call(command))