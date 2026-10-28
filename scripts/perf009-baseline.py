#!/usr/bin/env python3
"""Instrument the unchanged full portable runner before reuse experiments.

This retains compiler read-time inputs using PERF-005's tracer. Traced elapsed
time is attribution data, NOT a clean benchmark. No provider objects are reused.
Prepare a new private state with perf005-state.py and select its HOME/PATH.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import resource
import shutil
import subprocess
import sys
import time

from perf005_ci import validate_toolchain
from perf005_profile import run
from perf005_provider import Pilot, digest, sources
from perf009_portable import validate_result
from perf009_evidence import seal

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("positive worker budget required")
    if subprocess.check_output(["git", "-C", str(ROOT), "status", "--porcelain"]).strip():
        parser.error("commit implementation before exact-source qualification")
    scratch = args.scratch.resolve()
    alr = shutil.which("alr")
    if not alr:
        parser.error("private verified Alire state required")
    state = Path(alr).parent.parent.resolve()
    if not (state / "seed.json").is_file() or Path(os.environ["HOME"]).resolve() != state / "home":
        parser.error("private state/HOME required; prepare using perf005-state.py")
    if scratch.exists() or scratch == state or state in scratch.parents or scratch in state.parents:
        parser.error("fresh scratch independent of private state required")
    spec = importlib.util.spec_from_file_location("perf009_original", ROOT / "scripts/check-task093-helper-demand.py")
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)

    def checked(command, cwd, stem, expected=0):
        building = command[:4] == ["alr", "-n", "build", "--"]
        if building:
            position = command.index("-cargs:Ada") if "-cargs:Ada" in command else len(command)
            command = command[:position] + [f"-j{args.jobs}"] + command[position:]
        code, text = run(command, cwd, stem, trace=building or stem.name.endswith("-execute"))
        if expected is not None and code != expected:
            print(text.decode(errors="replace"), file=sys.stderr, flush=True)
            raise RuntimeError(f"exit {code}, expected {expected}: {stem}")
        return code, text

    runner.checked = checked
    sys.argv = [str(runner.__file__), "--family", "portable", "--compiler", args.compiler,
                "--scratch", str(scratch), "--generator", str(args.generator.resolve())]
    started = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    self_before = time.process_time()
    runner.main()
    campaign_elapsed = time.monotonic() - started
    self_cpu = time.process_time() - self_before
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    validate_result(scratch, args.compiler)
    # Record the actual independently resolved original toolchain for the
    # read-only mapping comparator. This never prepares/reuses a provider.
    pilot = Pilot(scratch, args.compiler, checked)
    for profile in ("oam", "qualified"):
        for name in runner.NAMES:
            consumer = scratch / f"{profile}-{name}"
            pilot.configuration(consumer, sources(consumer))
            validate_toolchain((scratch / f"{consumer.name}-provider-identity.log").read_bytes(), args.compiler)
    profiles = [json.loads(p.read_text()) for p in scratch.glob("*build.profile.json")]
    identity = subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD", "HEAD^{tree}"], text=True).splitlines()
    (scratch / "baseline-total.json").write_text(json.dumps(dict(
        completed=True, compiler=args.compiler, instrumented=True,
        source_commit=identity[0], source_tree=identity[1],
        elapsed_seconds=campaign_elapsed,
        child_cpu_seconds=after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
        self_cpu_seconds=self_cpu,
        worktree_dirty=bool(subprocess.check_output(["git", "-C", str(ROOT), "status", "--porcelain"], text=True)),
        tooling_sha256={p.name: digest(p) for p in (Path(__file__), Path(runner.__file__))},
        sampled_peak_tree_rss_kib=max(p["sampled_peak_tree_rss_kib"] for p in profiles),
        memory_method="per-command sampled simultaneous process-tree RSS; see profiles",
    ), indent=2) + "\n")
    evidence_hash = seal(scratch)
    scratch.with_name(scratch.name + "-seal.json").write_text(json.dumps(
        dict(source_commit=identity[0], source_tree=identity[1], evidence_sha256=evidence_hash), indent=2) + "\n")


if __name__ == "__main__":
    main()