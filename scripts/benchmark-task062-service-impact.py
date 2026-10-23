#!/usr/bin/env python3
"""Measure Task 062 without replacing production commands.

Usage: benchmark-task062-service-impact.py DRIVER BINARY EVIDENCE_DIRECTORY
Set the pinned-root variables and TMPDIR, and apply taskset to this process.
Linux /proc is required. Peak RSS is a 50ms sampled sum (not GNU time's
largest single-child RSS); shared pages can be counted more than once.
The wrapper's own CPU overhead is reported separately from child CPU.
"""
import base64
import json
import os
from pathlib import Path
import re
import resource
import runpy
import subprocess
import sys
import threading
import time

script, binary, destination = sys.argv[1:]
dest = Path(destination)
dest.mkdir(parents=True, exist_ok=True)
original = subprocess.run
records = []
peak = 0
samples = 0
io = {}
stop = threading.Event()


def sample():
    global peak, samples
    while not stop.is_set():
        pending = [os.getpid()]
        rss = 0
        while pending:
            pid = pending.pop()
            try:
                proc = Path(f'/proc/{pid}')
                # A subprocess launched by a pool thread belongs to that task,
                # not necessarily to the main thread's children list.
                for task in (proc / 'task').iterdir():
                    pending.extend(map(int, (task / 'children').read_text().split()))
                rss += int((proc / 'statm').read_text().split()[1]) * os.sysconf('SC_PAGE_SIZE')
                values = dict(line.split(': ') for line in (proc / 'io').read_text().splitlines())
                io[pid] = {k: int(v) for k, v in values.items()}
            except (FileNotFoundError, ProcessLookupError, PermissionError):
                pass
        peak = max(peak, rss)
        samples += 1
        stop.wait(.05)


def instrument(args, *a, **kw):
    start = time.monotonic()
    result = original(args, *a, **kw)
    elapsed = time.monotonic() - start
    if len(args) > 1 and args[1] == 'service-check':
        record = {'command': list(map(str, args)), 'exit_status': result.returncode,
                  'duration_seconds': elapsed}
        for name in ['stdout', 'stderr']:
            data = getattr(result, name)
            if data is None:
                handle = kw[name]
                handle.flush()
                data = Path(handle.name).read_bytes()
            if isinstance(data, str):
                data = data.encode()
            record[name + '_base64'] = base64.b64encode(data).decode()
        contract = Path(args[args.index('--contract') + 1])
        record['contract'] = contract.read_text()
        release = re.search(r'uci_schema_version: "([^"]+)"', record['contract'])[1]
        message = re.search(r'^        message: (.+)$', record['contract'], re.MULTILINE)[1]
        world = args[args.index('--world') + 1]
        world = {'closed-schema': 'ClosedSchemaSet', 'open-extensions': 'OpenExtensions'}[world]
        backend = args[args.index('--language') + 1].capitalize()
        record['identity'] = '\t'.join((release, world, backend, message))
        records.append(record)
    return result


subprocess.run = instrument
thread = threading.Thread(target=sample)
thread.start()
start = time.monotonic()
status = 0
try:
    sys.argv = [script, binary]
    runpy.run_path(script, run_name='__main__')
except SystemExit as error:
    status = error.code or 0
except BaseException:
    status = 1
    raise
finally:
    wall = time.monotonic() - start
    stop.set()
    thread.join()
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    own = resource.getrusage(resource.RUSAGE_SELF)
    metrics = {'wall_seconds': wall, 'child_user_seconds': usage.ru_utime,
               'child_system_seconds': usage.ru_stime,
               'driver_user_seconds': own.ru_utime, 'driver_system_seconds': own.ru_stime,
               'cpu_percent': 100 * (usage.ru_utime + usage.ru_stime) / wall,
               'peak_tree_rss_bytes_sampled': peak, 'rss_sample_interval_seconds': .05,
               'samples': samples, 'exit_status': status, 'checks': len(records),
               'successes': sum(r['exit_status'] == 0 for r in records),
               'child_major_faults': usage.ru_majflt, 'child_minor_faults': usage.ru_minflt,
               'child_block_inputs': usage.ru_inblock, 'child_block_outputs': usage.ru_oublock,
               'affinity': sorted(os.sched_getaffinity(0)),
               'sampled_process_io': io}
    (dest / 'metrics.json').write_text(json.dumps(metrics, indent=2) + '\n')
    (dest / 'commands.json').write_text(json.dumps(records, indent=2) + '\n')
sys.exit(status)