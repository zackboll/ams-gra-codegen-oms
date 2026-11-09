#!/usr/bin/env python3
"""Reparse standalone WSS wire captures and independent broker observations."""
import argparse
import copy
import json
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task115 import build
from task109.relay import frames
from task110 import cases

NAMES = ["Log", "RDMA_InitializeSetup", "SurvivabilityRiskLevel"]
TOPICS = ["uci.log", "uci.rdma", "uci.survivability"]


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(out):
    result = json.loads((out / "result.json").read_text())
    require(result["completed"], "incomplete campaign")
    metadata = result["build"]
    release = metadata["release"]
    corpus = [cases.family(n).golden(release) for n in
              (102, 103, 105 if release == "26" else 104)]
    require(metadata["schemas"] == build.SCHEMAS[release], "schema authorities")
    require({k: v[0] for k, v in metadata["dependencies"].items()} == build.PINS, "dependency authorities")
    require([c["name"] for c in result["cases"]] ==
            ["default", "recovery", "exhausted", "invalid-trust", "denied-service", "normal"], "required cases")
    summaries = []
    for case in result["cases"]:
        directory = out / case["name"]
        require(case == json.loads((directory / "result.json").read_text()), "case receipt disagreement")
        require(case["completed"] and case["cleanup"] and not case["relay_errors"], "incomplete case")
        output = (directory / "consumer.stdout").read_text()
        permanent = case["name"] in ("invalid-trust", "denied-service")
        require(len(re.findall(r"^CONNECTING epoch=", output, re.M)) == case["expected_epochs"], "extra connection attempt")
        require(len(case["relay"]) == case["expected_epochs"], "TLS connection count")
        normal_exit = case["name"] in ("recovery", "normal")
        require(case["exit"] == (0 if normal_exit else 1), "exit classification")
        require(("COMPLETE" in output) == normal_exit, "normal completion classification")
        if permanent:
            require("RECONNECT" not in output and "RECEIVED" not in output, "permanent failure must be terminal")
        for epoch, relay in enumerate(case["relay"], 1):
            require(relay["completed"] and relay["epoch"] == epoch, "relay boundary")
            ada = (directory / f"epoch{epoch}-ada.wire").read_bytes()
            sleet = (directory / f"epoch{epoch}-sleet.wire").read_bytes()
            delivered = (directory / f"epoch{epoch}-delivered.wire").read_bytes()
            require(sleet == delivered, "broker bytes altered")
            if case["name"] == "invalid-trust":
                require(not ada and not sleet and relay["shutdown"] == "tls_rejected", "trust failed before upgrade")
                continue
            commands = [f["text"] for f in frames(ada)["frames"] if f["opcode"] == 1]
            require(commands and commands[0].startswith("INIT "), "one INIT first")
            init = json.loads(commands[0][5:])
            require(init == dict(versions=["1.0"], schema=f"002.{release[-1]}.0",
                                 service_id="ada-uci-client", verbose=True), "compiled schema and stable identity")
            expected = [] if permanent else [f"SUB {sid} {name} {topic}" for sid, (name, topic)
                                            in enumerate(zip(NAMES, TOPICS), 1)]
            require(commands[1:] == expected, "exact receive-only operations; no replay")
            received = [f["text"] for f in frames(sleet)["frames"] if f["opcode"] == 1]
            if permanent:
                require(any(t.startswith("-ERR") for t in received), "real terminal broker rejection")
                continue
            require(received.count("+OK") == 4 and sum(t.startswith("INFO ") for t in received) == 1,
                    "fresh INIT and subscription acknowledgments")
            messages = [t.split(" ", 2) for t in received if t.startswith("MSG ")]
            require(len(messages) == 3, "three actual deliveries")
            for sid, name, (_, actual_sid, text) in zip(range(1, 4), NAMES, messages):
                require(actual_sid == str(sid), "local SID")
                document = json.loads(text)
                require(list(document) == [name], "authentic message family")
                timestamp = document[name]["MessageHeader"]["Timestamp"]
                require(timestamp == f"2026-01-{epoch:02}T03:04:05Z", "epoch payload")
                expected_doc = copy.deepcopy(corpus[sid - 1])
                expected_doc["MessageHeader"]["Timestamp"] = timestamp
                require(document == {name: expected_doc}, "independent authentic payload values")
                require(f"RECEIVED epoch={epoch} {name} timestamp={timestamp}" in output, "generated typed callback")
            retired = output.index(f"FINALIZED epoch={epoch}")
            require(not re.search(rf"RECEIVED epoch={epoch} ", output[retired:]), "callback after retirement")
        broker = (directory / "server.stdout").read_text()
        registrations = re.findall(r'connection\{id=(\d+)[^\n]*subscription registered sid=([123]) topic=(\S+) message_name=(\S+)', broker)
        owners = {}
        for owner, sid, topic, name in registrations:
            owners.setdefault(owner, []).append((sid, topic, name))
        expected = [(str(i), topic, name) for i, (topic, name) in enumerate(zip(TOPICS, NAMES), 1)]
        require(len(owners) == (0 if permanent else case["expected_epochs"]) and
                all(rows == expected for rows in owners.values()), "independent Sleet subscription ownership")
        # The independent observer uses w1..w3; only the application's numeric
        # subscriptions are counted above. Never infer registration from callbacks.
        if not permanent:
            observer = json.loads((directory / "observer.json").read_text())
            published = [row["text"] for row in observer if row["direction"] == "send" and
                         row["text"].startswith("PUB ")]
            expected_pubs = []
            for epoch in range(1, case["expected_epochs"] + 1):
                for name, topic, original in zip(NAMES, TOPICS, corpus):
                    doc = copy.deepcopy(original)
                    doc["MessageHeader"]["Timestamp"] = f"2026-01-{epoch:02}T03:04:05Z"
                    expected_pubs.append((topic, {name: doc}))
            actual_pubs = [(text.split(" ", 2)[1], json.loads(text.split(" ", 2)[2])) for text in published]
            require(actual_pubs == expected_pubs, "independent publisher sequence; no duplicates")
            resources = case["resources"]
            require(len(resources) == case["expected_epochs"], "retirement resource samples")
            require(all(r["fd"] == resources[0]["fd"] and r["tasks"] == resources[0]["tasks"]
                        for r in resources[1:]), "resource growth after warmup")
        summaries.append(dict(name=case["name"], epochs=case["expected_epochs"],
                              exit=case["exit"], broker_owners=owners, resources=case["resources"]))
    receipt = dict(verified=True, source=metadata["source"], dirty=metadata["dirty"],
                   compiler=metadata["compiler"], backend=metadata["backend"], release=release,
                   dependencies=metadata["dependencies"], schemas=metadata["schemas"], inputs=metadata["inputs"],
                   generated=metadata["generated"], executable_sha256=metadata["executable_sha256"], cases=summaries,
                   limitations=["Only unambiguous peer closure is retryable; broad Transport_Error is terminal",
                                "Fixed transport setup/write deadlines can overrun global deadline"])
    if "deadline_probe" in result:
        require(result["deadline_probe"] == json.loads((out / "deadline-probe/result.json").read_text()),
                "deadline probe disagreement")
        receipt["deadline_probe"] = result["deadline_probe"]
    receipt["task115_acceptance"] = False
    if (out / "artifacts.json").exists():
        artifacts = json.loads((out / "artifacts.json").read_text())
        for name, pin in artifacts.items():
            require(build.digest(out / name) == pin, "artifact changed: " + name)
        receipt["artifacts_sha256"] = build.digest(out / "artifacts.json")
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    receipt = verify(args.evidence.resolve())
    if args.receipt:
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))