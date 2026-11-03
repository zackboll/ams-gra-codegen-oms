#!/usr/bin/env python3
"""Run the unchanged complete production portable campaign in private state."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time

from perf005_provider import digest
from perf009_evidence import seal

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--family", choices=["portable"], required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--original", action="store_true", help="complete traced original for independent correctness comparison")
    parser.add_argument("--jobs", type=int, default=1)
    args = parser.parse_args()
    if args.jobs < 1 or args.jobs > 4:
        parser.error("compiler budget must be 1..4")
    identity = subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD", "HEAD^{tree}"], cwd=ROOT, text=True).splitlines()
    if subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "status", "--porcelain"], cwd=ROOT):
        parser.error("exact committed source required")
    generator = args.generator.resolve(strict=True)
    generator_hash = digest(generator)
    evidence = args.scratch.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    # /tmp is an allowed GPR temporary-output root. Provider artifacts must be
    # outside it or the existing PERF-005 sandbox correctly rejects overlap.
    runtime = Path(tempfile.mkdtemp(prefix="perf009r-", dir="/var/tmp"))
    campaign = runtime / "campaign"
    state = Path(str(campaign) + "-state")
    locations = subprocess.check_output(["alr", "version"], text=True)
    def location(key):
        rows = [s.split(":", 1)[1].strip() for s in locations.splitlines() if s.startswith(key + ":")]
        if len(rows) != 1:
            raise RuntimeError("missing private Alire location: " + key)
        return Path(rows[0])
    alr = Path(os.environ.get("PACK_REAL_ALR", shutil.which("alr"))).resolve(strict=True)
    spec = importlib.util.spec_from_file_location("perf009_state", ROOT / "scripts/perf005-state.py")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    helper.create_state(location("settings folder") / "indexes/community", location("toolchain folder"), state, alr)
    env = dict(os.environ, HOME=str(state / "home"), PATH=str(state / "bin") + ":" + os.environ["PATH"],
               XDG_CONFIG_HOME=str(state / "home/.config"), XDG_CACHE_HOME=str(state / "home/.cache"),
               XDG_DATA_HOME=str(state / "home/.local/share"), TMPDIR="/tmp", PYTHONDONTWRITEBYTECODE="1")
    started = time.monotonic()
    command = [sys.executable, str(ROOT / "scripts/check-task093-helper-demand.py"),
               "--family", "portable", "--compiler", args.compiler, "--scratch", str(campaign),
               "--generator", str(generator), "--portable-provider-original" if args.original else "--portable-provider-reuse",
               "--provider-state", str(state), "--provider-jobs", str(args.jobs)]
    try:
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        report = json.loads((campaign / "portable-ci-verification.json").read_text())
        if digest(generator) != generator_hash:
            raise RuntimeError("generator changed during portable qualification")
        current = subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD", "HEAD^{tree}"], cwd=ROOT, text=True).splitlines()
        if current != identity or subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "status", "--porcelain"], cwd=ROOT):
            raise RuntimeError("qualification source changed during portable campaign")
        report.update(source_commit=identity[0], source_tree=identity[1], generator_sha256=generator_hash,
                      elapsed_seconds=time.monotonic() - started, runtime_root=str(runtime),
                      acceptance="correctness and actual compilation reduction; no performance certification")
        (campaign / "portable-ci-summary.json").write_text(json.dumps(report, indent=2) + "\n")
        seal_hash = seal(campaign)
        (evidence / "campaign-seal.json").write_text(json.dumps(dict(sha256=seal_hash, campaign=str(campaign))) + "\n")
    finally:
        # Original execute-log paths/markers and source manifest stay available
        # to existing packed-worker verifiers. tar preserves case-sensitive names.
        if campaign.exists():
            for file in campaign.iterdir():
                if file.is_file() and file.suffix in (".log", ".json", ".sha256", ".lock"):
                    shutil.copyfile(file, evidence / file.name)
            with tarfile.open(evidence / "portable-campaign.tar.gz", "w:gz", compresslevel=1) as archive:
                archive.add(campaign, arcname="campaign")
    print("PERF009R_CI_COMPLETE", json.dumps(report), flush=True)


if __name__ == "__main__":
    main()