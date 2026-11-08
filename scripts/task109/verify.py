#!/usr/bin/env python3
"""Independently recheck Task106's complete authority plus actual WSS wire."""
import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task106_evidence import verify_cell
from task106 import cases


def verify(directory):
    def raw(path):
        return (directory / path).read_bytes()

    expected = verify_cell(raw, "wss")
    tls = json.loads(raw("tls/result.json"))
    if (not tls.get("completed") or not tls.get("cleanup") or tls.get("errors")
            or tls.get("consumer_exit") != 0 or tls.get("tls") not in ("TLSv1.2", "TLSv1.3")
            or tls.get("partial_writes", 0) < 2 or tls.get("record_batches", 0) < 2):
        raise ValueError("missing verified WSS relay completion")
    wire = json.loads(raw("tls/wire.json"))
    messages = [frame["text"] for frame in wire["sleet"]["frames"]
                if frame["opcode"] == 1 and frame["text"].startswith("MSG ")]
    plan = [item for item in cases.corpus() if item["action"] in ("external", "publish")]
    if len(messages) != len(plan):
        raise ValueError("missing actual Ada TLS MSG frames")
    for message, item in zip(messages, plan, strict=True):
        _, sid, document = message.split(" ", 2)
        key = item["function"]
        payload = item["expected"] if item["action"] == "publish" else item["document"]
        if (int(sid) != 1 + list(cases.MESSAGES).index(key)
                or json.loads(document) != {cases.MESSAGES[key]: payload}):
            raise ValueError("actual WSS delivery SID/global/value/order mismatch")
    if (directory / "plaintext").exists():
        verify_cell(raw, "plaintext")
    return dict(completed=True, task106=expected, actual_wss_deliveries=len(messages))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.directory.resolve()), indent=2))