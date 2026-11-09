#!/usr/bin/env python3
"""Verify frozen real-TLS deadline cases and source/artifact identities."""
import argparse
import json
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task116.qualify import digest, PINS
from task116.peer import CASES, validate, parse_prefix, require


def verify(out):
    result = json.loads((out / "result.json").read_text())
    require(result["completed"] and not result["development"], "not an acceptance campaign")
    require({k: v[0] for k, v in result["dependencies"].items()} == PINS, "dependency pins")
    require(result["compiler"] in ("GNAT 14.2.0", "GNAT 16.1.0"), "compiler identity")
    require(all(step["exit"] == 0 and not step["timed_out"] for step in result["steps"]), "failed/timed-out step")
    artifacts = json.loads((out / "artifacts.json").read_text())
    for name, pin in artifacts.items():
        path = (out / name).resolve()
        require(path.is_relative_to(out.resolve()), "unsafe artifact path")
        require(digest(path) == pin, "changed artifact: " + name)
    require(digest(out / "build/bin/deadline_probe") == result["executable_sha256"], "probe binary changed")
    corpus = json.loads((out / "cases/result.json").read_text())
    require(corpus["completed"] and [(c["mode"], c["deadline_ms"]) for c in corpus["cases"]] == CASES, "case coverage")
    summaries = []
    for case in corpus["cases"]:
        validate(case)
        directory = out / "cases" / f"{case['mode']}-{case['deadline_ms']}"
        require(case == json.loads((directory / "result.json").read_text()), "case receipt disagreement")
        require(case["stdout"] == (directory / "stdout").read_text(), "stdout disagreement")
        for session in case["observations"]["sessions"]:
            require("FORBIDDEN" not in session.get("request", ""), "replay")
        if case["mode"] in ("write", "queued-write"):
            captured = (directory / "client.frames").read_bytes()
            frames, partial = parse_prefix(captured)
            session = case["observations"]["sessions"][0]
            publications = [payload for op, payload in frames if op == 1]
            require(len(captured) == session["captured_bytes"] and partial == session["partial_tail"], "write capture boundary")
            require(len(publications) == session["complete_publications"], "write frame count")
            require(all(p.startswith(f" {i}:".encode()) for i, p in enumerate(publications, 1)), "publication order/replay")
            require(all(b"FORBIDDEN" not in p for _, p in frames), "post-failure output")
        counts = re.search(r"RESOURCES fd=\s*(\d+) tasks=\s*(\d+)", case["stdout"])
        require(counts is not None and int(counts[2]) == 1, "tasks remain after retirement")
        summaries.append(dict(mode=case["mode"], deadline_ms=case["deadline_ms"], elapsed=case["elapsed"],
                              connections=case["observations"]["connections"],
                              fd=int(counts[1]), tasks=int(counts[2])))
    return dict(verified=True, source=result["source"], compiler=result["compiler"], backend=result["backend"],
                dependencies=result["dependencies"], executable_sha256=result["executable_sha256"],
                artifacts_sha256=digest(out / "artifacts.json"), cases=summaries,
                allocation="min(2 seconds, (remaining caller time - 2 ms) / 3) per AHT setup duration",
                tolerance_seconds=0.150,
                limitations=["Conservative setup allocation may fail early; shares are not redistributed",
                             "No preemption of local CPU/filesystem work or scheduler delays",
                             "Pinned AHT DNS may retain its own detached resolver worker until OS resolver completion"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    receipt = verify(args.evidence.resolve())
    if args.receipt:
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))