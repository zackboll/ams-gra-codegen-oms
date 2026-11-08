#!/usr/bin/env python3
"""Compact receipts; recheck external artifacts and clean campaign source blobs."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task111.trust import validate


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_resources(focused):
    baseline = re.search(r"RESOURCES fd=\s*(\d+) tasks=\s*(\d+)", focused["baseline"])
    if not baseline or baseline.group(2) != "1":
        raise ValueError("missing quiescent baseline")
    final = re.search(r"RESOURCES fd=\s*(\d+) tasks=\s*(\d+)", focused["final"])
    if not final or final.groups() != baseline.groups():
        raise ValueError("final resources differ")
    for index, case in enumerate(focused["cases"]):
        validate(case["mode"], case["output"], case["peer"])
        counts = re.search(r"RESOURCES fd=\s*(\d+) tasks=\s*(\d+)", case["output"])
        # The first twelve attempts intentionally precede runtime warm-up.
        if not counts or (index >= 12 and counts.groups() != baseline.groups()) or case["seconds"] >= 10:
            raise ValueError("resource/deadline check missing")


def create(directory):
    result = json.loads((directory / "result.json").read_text())
    if not result.get("completed") or result.get("development"):
        raise ValueError("not an acceptance run")
    source = (directory / "source.stdout").read_text().splitlines()
    if (directory / "source-status.stdout").read_text().strip():
        raise ValueError("dirty campaign source")
    inputs = json.loads((directory / "input-sources.json").read_text())
    # Every recorded input must actually be from the stated Git commit, not
    # merely agree with a subsequently edited working directory.
    for path, pin in inputs.items():
        blob = subprocess.check_output(["git", "show", source[0] + ":" + path], cwd=ROOT)
        if hashlib.sha256(blob).hexdigest() != pin:
            raise ValueError("input not from campaign commit: " + path)
    focused = json.loads((directory / "focused/result.json").read_text())
    if not focused["completed"] or len(focused["cases"]) != 147 or not focused["same_client"]:
        raise ValueError("missing focused completion")
    cases = focused["cases"]
    check_resources(focused)
    summary = dict(attempts=len(cases), modes=dict(Counter(c["mode"] for c in cases)),
                   tls=sorted({c["peer"]["tls"] for c in cases if "tls" in c["peer"]}),
                   max_case_seconds=max(c["seconds"] for c in cases), baseline=focused["baseline"],
                   final=focused["final"], same_client=True, lifecycle_cycles=focused["lifecycle_cycles"],
                   negatives={c["mode"]: c for c in cases if c["mode"] not in ("pem", "path", "untrusted-close")})
    integrations = {}
    for release in result["integration"]:
        integrations[release] = dict(
            verified=json.loads((directory / ("verify" + release + ".stdout")).read_text()),
            broker=json.loads((directory / ("uci" + release) / "wss/result.json").read_text()),
            tls=json.loads((directory / ("uci" + release) / "tls/result.json").read_text()))
    identities = {name: (directory / (name + "-identity.stdout")).read_text().splitlines()
                  for name in ("aht", "codec", "sleet", "uci25", "uci26")}
    memcheck = None
    if (directory / "memcheck/memcheck.log").exists():
        log = (directory / "memcheck/memcheck.log").read_text()
        for marker in ("definitely lost: 0 bytes", "indirectly lost: 0 bytes", "possibly lost: 0 bytes", "ERROR SUMMARY: 0 errors"):
            if marker not in log:
                raise ValueError("memory validation failed: " + marker)
        memcheck = log
    artifacts = {str(p.relative_to(directory)): digest(p) for p in directory.rglob("*")
                 if p.is_file() and (p.name in ("result.json", "attempts.json", "probe.stdout", "input-sources.json",
                     "schema-sources.json", "generated-sources.json", "alire.lock", "wire.json", "observations.json",
                     "plan.jsonl", "consumer.stdout", "server.stdout", "consumer.exit", "server.exit", "memcheck.log")
                     or p.name.endswith((".command.json", "-identity.stdout")))}
    return dict(external_directory=str(directory), source=source, source_status="", result=result,
                identities=identities, trust=summary, integrations=integrations, memcheck=memcheck, artifacts=artifacts)


def verify(path):
    receipt = json.loads(path.read_text())
    directory = Path(receipt["external_directory"])
    if create(directory) != receipt:
        raise ValueError("receipt/artifact mismatch")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, nargs="?")
    parser.add_argument("--verify", type=Path)
    args = parser.parse_args()
    if args.verify:
        verify(args.verify.resolve())
        print("TASK111_RECEIPT_VERIFIED")
    else:
        print(json.dumps(create(args.directory.resolve()), indent=2))