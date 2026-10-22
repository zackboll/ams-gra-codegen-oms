#!/usr/bin/env python3
"""Opt-in Linux command accounting; includes reaped subprocess CPU, not estimates.

Usage: python3 scripts/measure-task061.py /absolute/metrics.json COMMAND [ARGS...]
getrusage peak RSS is the largest individual process, not summed resident memory.
Also samples the process tree's summed VmRSS every 50 ms (shared pages may be
counted more than once and short-lived peaks can be missed).
Run comparable commands with taskset on an otherwise idle machine for comparisons.
"""
import json
from pathlib import Path
import resource
import subprocess
import sys
import time

start = time.monotonic()
process = subprocess.Popen(sys.argv[2:])
peak_tree_rss = 0
samples = 0
while process.poll() is None:
    pending = [process.pid]
    rss = 0
    while pending:
        pid = pending.pop()
        try:
            # Enumerate all threads: the Python driver's pool threads create
            # children not listed under its main thread's children file.
            for task in Path(f"/proc/{pid}/task").iterdir():
                pending.extend(map(int, (task / "children").read_text().split()))
            for line in Path(f"/proc/{pid}/status").read_text().splitlines():
                if line.startswith("VmRSS:"):
                    rss += int(line.split()[1])
                    break
        except (FileNotFoundError, ProcessLookupError):
            pass
    peak_tree_rss = max(peak_tree_rss, rss)
    samples += 1
    time.sleep(0.05)
returncode = process.wait()
wall = time.monotonic() - start
usage = resource.getrusage(resource.RUSAGE_CHILDREN)
Path(sys.argv[1]).write_text(json.dumps({
    "command": sys.argv[2:], "wall_seconds": wall,
    "user_seconds": usage.ru_utime, "system_seconds": usage.ru_stime,
    "cpu_percent": 100 * (usage.ru_utime + usage.ru_stime) / wall,
    "max_process_rss_kib": usage.ru_maxrss,
    "sampled_peak_tree_rss_kib": peak_tree_rss, "rss_samples": samples,
    "exit_code": returncode,
}, indent=2) + "\n")
sys.exit(returncode)