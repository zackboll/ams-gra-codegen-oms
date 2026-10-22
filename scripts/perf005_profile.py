"""Linux-only, opt-in process tracing and non-overlapping CPU sampling.

CPU attribution is sampling, not exact process accounting. Only /proc stat
utime+stime are used (never cutime/cstime). The outer child rusage is the
independent reconciliation total. strace observes execs including absolute
Alire compiler paths; inclusive process lifetimes must not be summed as wall.
"""
import ast
import json
import os
from pathlib import Path
import re
import resource
import subprocess
import threading
import time


def phase(argv, cwd=""):
    text = " ".join(argv)
    name = Path(argv[0]).name if argv else ""
    if name in ("ar", "ranlib") or name.endswith(("-ar", "-ranlib")):
        return "archive"
    if any(x in name for x in ("gnatbind", "gnatlink")) or name in ("ld", "collect2"):
        return "bind-link"
    compiling = bool(re.search(r"(?:^|-)(?:gcc(?:-\d+)?|g\+\+|gnat1|cc1(?:plus)?|as)$", name))
    if compiling:
        if "b__" in text or "b~" in text:
            return "bind-link"
        if "-c" not in argv and name.endswith("gcc"):
            return "bind-link"
        ownership = text + " " + cwd
        if "gnatcoll_minimal" in ownership or "/minimal/" in ownership:
            return "gnatcoll-minimal"
        if "gnatcoll_" in ownership or "/core/" in ownership:
            return "gnatcoll-core"
        if "-support/" in ownership or "/support/" in ownership:
            return "repository-support"
        return "generated-consumer"
    if name == "alr":
        return "alire-setup"
    return "unclassified"


def snapshot():
    result = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            raw = (entry / "stat").read_text()
            fields = raw[raw.rindex(")") + 2:].split()
            argv = (entry / "cmdline").read_bytes().decode(errors="replace").rstrip("\0").split("\0")
            cwd = os.readlink(entry / "cwd")
            result[int(entry.name)] = (int(fields[1]), int(fields[11]) + int(fields[12]),
                                       int(fields[21]) * os.sysconf("SC_PAGE_SIZE") // 1024,
                                       argv, cwd, fields[19])
        except (OSError, ValueError, IndexError):
            continue
    return result


def executions(path):
    """Extract successful exec identities and final status from strace -f output."""
    commands = []
    active = {}
    pending = {}
    for line in path.read_text().splitlines():
        match = re.match(r"(\d+)\s+(\d+\.\d+)\s+(.*)", line)
        if not match:
            continue
        pid, stamp, event = int(match[1]), float(match[2]), match[3]
        if event.startswith("execve(") and event.endswith("<unfinished ...>"):
            pending[pid] = (stamp, event.removesuffix("<unfinished ...>"))
            continue
        if event.startswith("<... execve resumed>") and pid in pending:
            stamp, first = pending.pop(pid)
            event = first + event.removeprefix("<... execve resumed>")
        executed = re.match(r'execve\(("(?:[^"\\]|\\.)*"), (\[.*?\]), .*\)\s+= 0', event)
        if executed:
            try:
                argv = ast.literal_eval(executed[2])
                record = dict(pid=pid, executable=ast.literal_eval(executed[1]),
                              argv=argv, started=stamp, exit=None)
            except (ValueError, SyntaxError):
                continue
            commands.append(record)
            active[pid] = record
        ended = re.search(r"\+\+\+ (?:exited with (\d+)|killed by (\w+)).*\+\+\+", event)
        if ended and pid in active:
            record = active.pop(pid)
            record["exit"] = int(ended[1]) if ended[1] else ended[2]
            record["inclusive_elapsed_seconds"] = stamp - record["started"]
    return commands


def run(command, cwd, stem, trace=False, interval=0.02, writable=None):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.monotonic()
    actual = command
    trace_path = Path(str(stem) + ".strace")
    if trace:
        # Detailed runs retain GPR-generated response/config files so compiler
        # switches do not depend on vanished /tmp inputs. Clean timing is exact.
        if "gprbuild" in command or command[:3] == ["alr", "-n", "build"] and "--" in command:
            actual = list(command)
            position = actual.index("-cargs:Ada") if "-cargs:Ada" in actual else len(actual)
            actual.insert(position, "--keep-temp-files")
        if writable:
            trace_path.touch(exist_ok=False)
            writable = [*writable, trace_path]
        actual = ["strace", "-f", "-ttt", "-s", "65535", "-e", "trace=execve",
                  "-o", str(trace_path), "--", *actual]
    totals = {}
    observed = {}
    peak = 0
    stop = threading.Event()
    with stem.with_suffix(".log").open("xb") as output:
        from perf005_sandbox import restrict_writes
        process = subprocess.Popen(actual, cwd=cwd, stdin=subprocess.DEVNULL,
                                   stdout=output, stderr=subprocess.STDOUT,
                                   preexec_fn=(lambda: restrict_writes(writable)) if writable else None)

        def sample():
            nonlocal peak
            previous = {}
            while not stop.is_set():
                data = snapshot()
                descendants = {process.pid}
                while True:
                    expanded = descendants | {pid for pid, row in data.items() if row[0] in descendants}
                    if expanded == descendants:
                        break
                    descendants = expanded
                peak = max(peak, sum(data[pid][2] for pid in descendants if pid in data))
                for pid in descendants:
                    if pid not in data:
                        continue
                    parent, ticks, rss, argv, directory, birth = data[pid]
                    key = (pid, birth)
                    delta = max(0, ticks - previous.get(key, 0))
                    previous[key] = ticks
                    category = phase(argv, directory)
                    totals[category] = totals.get(category, 0) + delta / os.sysconf("SC_CLK_TCK")
                    observed[str(pid)] = dict(argv=argv, cwd=directory, phase=category)
                    observed[str(pid)]["sampled_self_cpu_seconds"] = previous[key] / os.sysconf("SC_CLK_TCK")
                stop.wait(interval)

        thread = threading.Thread(target=sample)
        if trace:
            thread.start()
        code = process.wait()
        stop.set()
        if trace:
            thread.join()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    record = dict(command=command, cwd=str(cwd), exit=code,
                  elapsed_seconds=time.monotonic() - started, cpu_seconds=cpu,
                  sampled_self_cpu_seconds=totals,
                  residual_cpu_seconds=cpu - sum(totals.values()),
                  sampled_peak_tree_rss_kib=peak if trace else None,
                  sampling_interval_seconds=interval if trace else None)
    if trace:
        record["executions"] = executions(trace_path)
        record["observed_processes"] = observed
        temporary_inputs = {}
        for execution in record["executions"]:
            for arg in execution["argv"]:
                value = arg[1:] if arg.startswith("@") else arg.split("=", 1)[-1]
                path = Path(value)
                if value.startswith("/tmp/GPR.") and path.is_file():
                    temporary_inputs[value] = path.read_text(errors="replace")
        record["gpr_temporary_inputs"] = temporary_inputs
    Path(str(stem) + ".profile.json").write_text(json.dumps(record, indent=2) + "\n")
    return code, stem.with_suffix(".log").read_bytes()