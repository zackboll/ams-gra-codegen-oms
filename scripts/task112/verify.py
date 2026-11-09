#!/usr/bin/env python3
"""Verify external full execution records; optionally emit a compact receipt."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import qualify
import recovery


def verify(directory, profile=None):
    campaign_recovery = getattr(profile, "recovery", recovery)
    def load(name):
        return json.loads((directory / name).read_text())
    campaign = load("result.json")
    recovery.require(campaign["completed"] and not campaign["development"], "clean-source acceptance only")
    recovery.require(all(step["exit"] == 0 and not step["timed_out"] for step in campaign["steps"]), "all commands passed")
    for name, pin in load("input-sources.json").items():
        recovery.require(qualify.digest(qualify.ROOT / name) == pin, "changed qualified source: " + name)
    artifacts = load("artifacts.json")
    for name, pin in artifacts.items():
        recovery.require(qualify.digest(directory / name) == pin, "changed external artifact: " + name)
    for name, pin in qualify.PINS.items():
        recovery.require((directory / (name + "-identity.stdout")).read_text().splitlines()[0] == pin, "dependency identity")
        for suffix in ("-clean.stdout", "-final-clean.stdout"):
            recovery.require(not (directory / (name + suffix)).read_text().strip(), "dependency clean")
    receipt = dict(format=getattr(profile, "FORMAT", "task112-v1"), source=campaign["source"], compiler=campaign["compiler"],
                   backend=campaign["backend"], dependencies=qualify.PINS,
                   dependency_trees={n: (directory / (n + "-identity.stdout")).read_text().splitlines()[1]
                                     for n in qualify.PINS},
                   input_sources=load("input-sources.json"),
                   full_artifact_manifest_sha256=qualify.digest(directory / "artifacts.json"),
                   generator_sha256=(directory / "generator.sha256").read_text().strip(),
                   sleet_binary_sha256=(directory / "sleet-binary.sha256").read_text().strip(),
                   steps=campaign["steps"], releases={})
    for release in campaign["releases"]:
        cell = directory / ("recovery" + release)
        result = json.loads((cell / "result.json").read_text())
        recovery.require(result["completed"] and result["cleanup"] and not result.get("error") and
                         result["consumer_exit"] == result["server_exit"] == 0 and not result["relay_errors"], "real integration passed")
        results = result["epochs_observed"]
        compact = []
        for epoch, observation in enumerate(results, 1):
            wire = json.loads((cell / f"epoch{epoch}-wire.json").read_text())
            for side in wire:
                raw = (cell / f"epoch{epoch}-{side}.bin").read_bytes()
                recovery.require(hashlib.sha256(raw).hexdigest() == observation[side + "_sha256"], "raw wire digest")
                recovery.require(recovery.frames(raw) == wire[side], "independent wire parse")
            observation["wire"] = wire
            row = {k: v for k, v in observation.items() if k != "wire"}
            row["protocol"] = {}
            for side, capture in wire.items():
                texts = [f["text"] for f in capture["frames"] if f["opcode"] == 1]
                row["protocol"][side] = dict(upgrade=capture["upgrade"],
                    commands=[t for t in texts if not t.startswith(("PUB ", "MSG "))],
                    pub_count=sum(t.startswith("PUB ") for t in texts),
                    msg_count=sum(t.startswith("MSG ") for t in texts),
                    document_frames=[dict(operation=t.split()[0], sha256=hashlib.sha256(t.encode()).hexdigest())
                                     for t in texts if t.startswith(("PUB ", "MSG "))])
            compact.append(row)
        campaign_recovery.validate_wire(results, release)
        output = (cell / "consumer.stdout").read_text()
        campaign_recovery.validate_output(output)
        if profile is not None:
            recovery.require(campaign_recovery.validate_broker((cell / "server.stdout").read_text(),
                result["broker_connections"], results) == {"pending_operations": result["pending_operations"]},
                "independent broker operation evidence")
        receipt["releases"][release] = dict(result={k: v for k, v in result.items() if k != "epochs_observed"},
            generated=load("generated" + release + "-sources.json"), schemas=qualify.SCHEMAS[release],
            consumer_observations=[line for line in output.splitlines() if re.match(
                r"^(CALL|SUBSCRIBED|PENDING|ARMED|TRANSPORT_EXCEPTION|RETIRED|CLEANUP|TASK11[23]_COMPLETE) ", line)],
            epochs=compact,
            artifacts={str(p.relative_to(directory)): qualify.digest(p) for p in
                       (cell / "consumer.stdout", cell / "consumer.stderr", cell / "server.stdout",
                        cell / "server.stderr", cell / "observer.json", cell / "result.json", cell / "plan.jsonl")})
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    receipt = verify(args.directory.resolve())
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print("TASK112_VERIFIED", receipt["compiler"], receipt["backend"])