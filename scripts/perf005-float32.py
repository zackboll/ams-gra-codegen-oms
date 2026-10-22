#!/usr/bin/env python3
"""Explicit PERF-005 baseline/profiling entry point; ordinary runners unchanged."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import resource
import shutil
import sys
import time

from perf005_profile import run
from perf005_provider import Pilot
from perf005_sandbox import consumer_layout, validate_layout

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", action="store_true", help="detailed, sampled attribution; not clean timing")
    parser.add_argument("--representative", action="store_true", help="first naming case only; NOT a complete campaign")
    parser.add_argument("--reuse-provider", action="store_true", help="experimental three-variant campaign-local pilot")
    parser.add_argument("runner_args", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    runner_args = args.runner_args
    if runner_args[:1] == ["--"]:
        runner_args = runner_args[1:]
    if "--family" not in runner_args or runner_args[runner_args.index("--family") + 1] != "float32":
        parser.error("this investigation is bounded to --family float32")
    for required in ("--compiler", "--scratch", "--generator"):
        if required not in runner_args or runner_args.index(required) + 1 >= len(runner_args):
            parser.error(f"runner requires {required}")
    if args.reuse_provider and "--red" in runner_args:
        parser.error("intentional-red is baseline-only; pilot must not weaken existing diagnostics")
    if len(os.sched_getaffinity(0)) != 4:
        parser.error("PERF-005 requires an explicit four-CPU affinity allocation")
    spec = importlib.util.spec_from_file_location("perf005_runner", ROOT / "scripts/check-task093-helper-demand.py")
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    if args.representative:
        runner.NAMES = runner.NAMES[:1]
    scratch = Path(runner_args[runner_args.index("--scratch") + 1]).resolve()
    alr_path = shutil.which("alr")
    if alr_path is None:
        parser.error("Alire 2.1.1 is required")
    state = Path(alr_path).parent.parent.resolve()
    if not (state / "settings/settings.toml").is_file() or not (state / "cache/toolchains").is_dir() or Path(os.environ["HOME"]).resolve() != state / "home":
        parser.error("private PERF-005 Alire state/HOME required; use perf005-state.py")
    if scratch == state or state in scratch.parents or scratch in state.parents:
        parser.error("consumer output and Alire state roots must be separate")
    if args.reuse_provider:
        validate_layout([state, Path(os.environ["HOME"]), Path("/tmp"), Path("/dev")],
                        [scratch / "perf005-providers"])

    def checked(command, cwd, stem, expected=0):
        writable = None
        protected = ()
        if "-XEXTERNALLY_BUILT=true" in command:
            writable, protected = consumer_layout(command, cwd, state, Path(os.environ["HOME"]))
        code, text = run(command, cwd, stem,
                         trace=args.trace and ("build" in command or "gprbuild" in command),
                         writable=writable, protected=protected)
        if expected is not None and code != expected:
            print(text.decode(errors="replace"), file=sys.stderr)
            raise RuntimeError(f"exit {code}, expected {expected}: {stem}")
        return code, text

    pilot = Pilot(scratch, runner_args[runner_args.index("--compiler") + 1], checked)

    def adapted(command, cwd, stem, expected=0):
        if args.reuse_provider and command[:5] == ["alr", "-n", "build", "--", "-f"]:
            return pilot.build(command, cwd, stem, expected)
        return checked(command, cwd, stem, expected)

    runner.checked = adapted
    sys.argv = [str(runner.__file__), *runner_args]
    started = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    runner.main()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    (scratch / "perf005-total.json").write_text(json.dumps(dict(
        completed=True, trace=args.trace, representative=args.representative, reuse_provider=args.reuse_provider,
        elapsed_seconds=time.monotonic() - started,
        child_cpu_seconds=after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
        self_cpu_seconds=time.process_time(),
        child_maxrss_kib=after.ru_maxrss,
        memory_method="kernel largest-child high-water RSS; not simultaneous tree RSS"), indent=2) + "\n")


if __name__ == "__main__":
    main()