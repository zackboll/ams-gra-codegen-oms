#!/usr/bin/env python3
"""Keep the original qualification argv; constrain its Alire subprocesses."""
import json
import os
from pathlib import Path
import sys


def arguments(args):
    if "build" in args:
        # GPRbuild's last jobs switch wins over Alire's automatic -j setting.
        if "--" not in args:
            args = [*args, "--"]
        index = args.index("--") + 1
        args = [*args[:index], "-j1", *args[index:]]
    return [os.environ["PACK_REAL_ALR"], "--settings=" + os.environ["PACK_SETTINGS"], *args]


if __name__ == "__main__":
    argv = arguments(sys.argv[1:])
    with Path(os.environ["PACK_ALR_TRACE"]).open("a") as log:
        log.write(json.dumps(dict(pid=os.getpid(), cwd=os.getcwd(), argv=argv)) + "\n")
    os.execv(argv[0], argv)