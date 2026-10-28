#!/usr/bin/env python3
"""Untraced full portable timing with a fail-closed contention precondition.

Not a production CI entry point. Run separate fresh states in A/B/B/A order.
Both paths receive identical explicit worker budgets. Reuse requires a sealed
traced qualification; timed results never count as read-time mapping proof.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import shutil
import subprocess
import sys
import tarfile
import threading
import time

from perf005_profile import run, snapshot
from perf005_sandbox import consumer_layout
from perf009_evidence import verify_seal
from perf009_portable import inventory, validate_result
from perf009_provider import PortablePilot

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True


def cpu_counters(cpus):
    return {int(line.split()[0][3:]): [int(v) for v in line.split()[1:]]
            for line in Path("/proc/stat").read_text().splitlines()
            if re.match(r"cpu\d+ ", line) and int(line.split()[0][3:]) in cpus}


def contention(interval=2):
    affinity = set(os.sched_getaffinity(0))
    # Include SMT siblings: an idle logical CPU is not an uncontended core.
    siblings = set(affinity)
    for cpu in affinity:
        text = Path(f"/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list").read_text().strip()
        for part in text.split(","):
            bounds = [int(v) for v in part.split("-")]
            siblings.update(range(bounds[0], bounds[-1] + 1))
    before = cpu_counters(siblings)
    time.sleep(interval)
    after = cpu_counters(siblings)
    busy = {}
    for cpu in before:
        delta = [b - a for a, b in zip(before[cpu], after[cpu])]
        # guest ticks are already included in user/nice.
        total = sum(delta[:8])
        busy[str(cpu)] = 1 - (delta[3] + delta[4]) / total if total else 1
    return dict(affinity=sorted(affinity), physical_siblings=sorted(siblings),
                busy_fraction=busy, uncontended=bool(busy) and max(busy.values()) < 0.05,
                method="two-second idle preflight across selected CPUs and SMT siblings; no process stopping")


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--mode", choices=("original", "reuse"), required=True)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--jobs", type=int, required=True)
    parser.add_argument("--qualification", type=Path)
    parser.add_argument("--qualification-seal")
    parser.add_argument("--preflight-only", action="store_true")
    args = parser.parse_args()
    args.scratch.mkdir(parents=True, exist_ok=False)
    preflight = contention()
    (args.scratch / "contention.json").write_text(json.dumps(preflight, indent=2) + "\n")
    if not preflight["uncontended"]:
        raise SystemExit("PERFORMANCE_GATE_INCOMPLETE: selected physical cores are contended; no benchmark accepted")
    if args.preflight_only:
        return
    if args.jobs < 1 or args.jobs > len(os.sched_getaffinity(0)):
        parser.error("worker budget must fit explicit CPU affinity")
    if subprocess.check_output(["git", "-C", str(ROOT), "status", "--porcelain"]).strip():
        parser.error("clean committed implementation required")
    if args.mode == "reuse":
        if not args.qualification or not args.qualification_seal:
            parser.error("sealed traced qualification required")
        verify_seal(args.qualification, args.qualification_seal)
        qualified = json.loads((args.qualification / "perf009-experiment.json").read_text())
        if not qualified.get("negative_controls_qualified"):
            parser.error("mandatory live negative qualification incomplete")
    alr = shutil.which("alr")
    state = Path(alr).parent.parent.resolve() if alr else Path("/missing")
    if not (state / "seed.json").is_file() or Path(os.environ["HOME"]).resolve() != state / "home":
        parser.error("fresh private seeded Alire state required")
    campaign = args.scratch / "campaign"
    spec = importlib.util.spec_from_file_location("portable_benchmark_runner", ROOT / "scripts/check-task093-helper-demand.py")
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    counts = dict(provider=0, owned=0, bind=0, link=0, execution=0)
    peak = [0]
    stop = threading.Event()

    def sample():
        while not stop.is_set():
            data = snapshot()
            descendants = {os.getpid()}
            while True:
                expanded = descendants | {pid for pid, row in data.items() if row[0] in descendants}
                if expanded == descendants:
                    break
                descendants = expanded
            peak[0] = max(peak[0], sum(data[p][2] for p in descendants if p in data))
            stop.wait(0.1)

    def checked(command, cwd, stem, expected=0):
        building = command[:4] == ["alr", "-n", "build", "--"] or "gprbuild" in command
        forced = "-XEXTERNALLY_BUILT=true" in command
        if command[:4] == ["alr", "-n", "build", "--"]:
            pos = command.index("-cargs:Ada") if "-cargs:Ada" in command else len(command)
            command = command[:pos] + [f"-j{args.jobs}"] + command[pos:]
        paths = inventory(cwd) if forced else []
        started_ns = time.time_ns()
        writable, protected = consumer_layout(command, cwd, state, state / "home") if forced else (None, ())
        code, text = run(command, cwd, stem, trace=False, writable=writable, protected=protected)
        if expected is not None and code != expected:
            raise RuntimeError(f"timed command failed: {stem}")
        if building:
            units = [s.decode() for s in re.findall(rb"\[(?:Ada|C|Asm)\]\s+(\S+\.(?:adb|ads|c|s|S))", text)]
            if forced:
                if sorted(units) != sorted(p.name for p in paths):
                    raise RuntimeError("timed generated compilation inventory differs")
                for source in paths:
                    objdir = cwd / "obj" if source.parent == cwd else source.parent.parent / "obj"
                    for suffix in (".o", ".ali"):
                        p = objdir / (source.stem + suffix)
                        if not p.is_file() or p.is_symlink() or p.stat().st_mtime_ns < started_ns:
                            raise RuntimeError("timed generated artifact missing/stale")
                if not all(m in text for m in (b"[gprbind]", b"[link]")):
                    raise RuntimeError("timed bind/link missing")
                counts["owned"] += 5
                counts["bind"] += 1
                counts["link"] += 1
            elif "gprbuild" in command:
                if len(units) != 106:
                    raise RuntimeError("timed provider preparation incomplete")
                counts["provider"] += 106
            elif len(units) == 111:
                counts["provider"] += 106
                counts["owned"] += 5
                counts["bind"] += int(b"[gprbind]" in text)
                counts["link"] += int(b"[link]" in text)
            else:
                raise RuntimeError("timed original compilation incomplete")
        if stem.name.endswith("-execute"):
            counts["execution"] += 1
        return code, text

    pilot = PortablePilot(campaign, args.compiler, checked, args.jobs, traced=False)
    def adapted(command, cwd, stem, expected=0):
        if args.mode == "reuse" and command[:5] == ["alr", "-n", "build", "--", "-f"]:
            return pilot.build(command, cwd, stem, expected)
        return checked(command, cwd, stem, expected)
    runner.checked = adapted
    sys.argv = [str(runner.__file__), "--family", "portable", "--compiler", args.compiler,
                "--scratch", str(campaign), "--generator", str(args.generator.resolve())]
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    self_cpu = time.process_time()
    started = time.monotonic()
    thread = threading.Thread(target=sample)
    thread.start()
    try:
        runner.main()
        validate_result(campaign, args.compiler)
    finally:
        stop.set()
        thread.join()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    elapsed = time.monotonic() - started
    expected = dict(provider=318 if args.mode == "reuse" else 3180, owned=150, bind=30, link=30, execution=30)
    if counts != expected:
        raise RuntimeError("timed complete campaign counts differ")
    pack_started = time.monotonic()
    with tarfile.open(args.scratch / "textual-evidence.tar.gz", "w:gz", compresslevel=1) as archive:
        for p in sorted(campaign.rglob("*")):
            if p.is_file() and p.suffix in (".log", ".json", ".lock", ".sha256", ".ads", ".adb", ".gpr", ".toml", ".adc"):
                archive.add(p, arcname=str(p.relative_to(campaign)), recursive=False)
    report = dict(completed=True, traced=False, mode=args.mode, compiler=args.compiler,
                  elapsed_seconds=elapsed, child_cpu_seconds=after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
                  harness_cpu_seconds=time.process_time() - self_cpu, peak_tree_rss_kib=peak[0], counts=counts,
                  packaging_seconds=time.monotonic() - pack_started, setup_included=False,
                  acceptance_qualified=False, limitation="preflight alone cannot guarantee sustained uncontended execution; independent A/B/B/A review required")
    (args.scratch / "benchmark.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()