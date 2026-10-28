#!/usr/bin/env python3
"""Explicit full portable reuse experiment; CI/default runner remain unchanged.

Requires a completed independently instrumented original campaign. Retains
read-time mappings and actual compiler/binder/linker executions, not just logs.
This is attribution/qualification, not a clean performance benchmark.
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
from perf005_mapping import provider_equivalence
from perf005_profile import run
from perf005_provider import Pilot, digest
from perf005_sandbox import consumer_layout, validate_layout
from perf009_portable import compare, validate_build, validate_result

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    args = parser.parse_args()
    baseline, scratch = args.baseline.resolve(), args.scratch.resolve()
    validate_result(baseline, args.compiler)
    baseline_sources = dict(line.split("  ", 1)[::-1] for line in (baseline / "sources.sha256").read_text().splitlines())
    generator = args.generator.resolve(strict=True)
    if baseline_sources.get(str(generator)) != digest(generator):
        parser.error("generator identity differs from original campaign")
    if scratch.exists() or scratch == baseline or baseline in scratch.parents or scratch in baseline.parents:
        parser.error("fresh independent scratch required")
    alr = shutil.which("alr")
    if not alr:
        parser.error("private verified Alire state required")
    state = Path(alr).parent.parent.resolve()
    home = Path(os.environ["HOME"]).resolve()
    if (not (state / "seed.json").is_file() or home != state / "home"
            or not (state / "settings/settings.toml").is_file()):
        parser.error("private state/HOME required; prepare using perf005-state.py")
    original_state = Path(str(baseline) + "-state")
    if state == original_state or json.loads((state / "seed.json").read_text()) != json.loads(
            (original_state / "seed.json").read_text()):
        parser.error("independent private state with identical seed required")
    validate_layout([state, home, Path("/tmp"), Path("/dev")], [scratch / "perf005-providers"])
    spec = importlib.util.spec_from_file_location("perf009_runner", ROOT / "scripts/check-task093-helper-demand.py")
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)

    def checked(command, cwd, stem, expected=0):
        forced = "-XEXTERNALLY_BUILT=true" in command
        writable, protected = consumer_layout(command, cwd, state, home) if forced else (None, ())
        started_ns = time.time_ns()
        code, text = run(command, cwd, stem, trace=forced or "gprbuild" in command,
                         writable=writable, protected=protected)
        if expected is not None and code != expected:
            print(text.decode(errors="replace"), file=sys.stderr, flush=True)
            raise RuntimeError(f"exit {code}, expected {expected}: {stem}")
        if stem.name.endswith("-provider-identity"):
            validate_toolchain(text, args.compiler)
        if forced:
            policy = stem.name.removesuffix("-build").rsplit("-", 1)[1]
            data = json.loads(Path(str(stem) + ".profile.json").read_text())
            verified = validate_build(command, cwd, data, args.compiler, policy, started_ns)
            Path(str(stem) + ".verification.json").write_text(json.dumps(verified, indent=2) + "\n")
        return code, text

    pilot = Pilot(scratch, args.compiler, checked, jobs=4)

    def adapted(command, cwd, stem, expected=0):
        if command[:5] == ["alr", "-n", "build", "--", "-f"]:
            return pilot.build(command, cwd, stem, expected)
        return checked(command, cwd, stem, expected)

    runner.checked = adapted
    sys.argv = [str(runner.__file__), "--family", "portable", "--compiler", args.compiler,
                "--scratch", str(scratch), "--generator", str(args.generator.resolve())]
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    self_before = time.process_time()
    started = time.monotonic()
    runner.main()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    campaign = dict(elapsed_seconds=time.monotonic() - started,
                    child_cpu_seconds=after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
                    self_cpu_seconds=time.process_time() - self_before, instrumented=True,
                    includes_post_campaign_equivalence=False)
    report = compare(baseline, scratch, args.compiler)
    proof = {}
    for policy in ("normal", "Check", "Ignore"):
        proof[policy] = provider_equivalence(baseline, scratch, policy)
    (scratch / "portable-provider-equivalence.json").write_text(json.dumps(proof, indent=2) + "\n")
    profiles = [json.loads(p.read_text()) for p in scratch.glob("*build.profile.json")]
    identity = subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD", "HEAD^{tree}"], text=True).splitlines()
    dirty = bool(subprocess.check_output(["git", "-C", str(ROOT), "status", "--porcelain"], text=True))
    report.update(campaign=campaign, source_commit=identity[0], source_tree=identity[1],
                  worktree_dirty=dirty,
                  tooling_sha256={p.name: digest(p) for p in (Path(__file__), ROOT / "scripts/perf009_portable.py",
                                  ROOT / "scripts/check-task093-helper-demand.py")},
                  sampled_peak_tree_rss_kib=max(p["sampled_peak_tree_rss_kib"] for p in profiles),
                  performance_qualified=False, negative_controls_qualified=False,
                  disposition="QUALIFICATION_FAILED",
                  limitation="Experimental matrix only; mandatory negative controls and clean/hosted benchmarks outstanding")
    (scratch / "perf009-experiment.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"PERF009_EXPERIMENT_COMPLETE compiler={args.compiler} positive=30 compiled=150 providers=3 NOT_CI_QUALIFIED")


if __name__ == "__main__":
    main()