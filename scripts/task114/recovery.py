#!/usr/bin/env python3
"""Opt-in complete-frame pre-broker interception; real TLS, OWP and Sleet."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import select
import socket
import ssl
import struct
import sys
import threading
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task112 import recovery as base
from task113 import recovery as previous
from task114.framing import Gate, Parser

require, frames, texts = base.require, base.frames, previous.texts
MODES, plan = previous.MODES, previous.plan


class Relay(base.Relay):
    def __init__(self, *args):
        super().__init__(*args)
        self.captured = [threading.Event() for _ in MODES]
        self.probed = [threading.Event() for _ in MODES]

    def serve(self):
        try:
            for epoch, mode in enumerate(MODES, 1):
                deadline = time.monotonic() + 20
                while not self.stop.is_set():
                    try:
                        raw, _ = self.listener.accept()
                        break
                    except socket.timeout:
                        require(time.monotonic() < deadline, "relay accept deadline")
                else:
                    return
                streams = {side: bytearray() for side in ("ada", "forwarded", "sleet", "delivered")}
                stats = dict(epoch=epoch, mode=mode, completed=False)
                gate = Gate(mode)
                try:
                    with raw, socket.create_connection(("127.0.0.1", self.upstream), timeout=5) as broker:
                        raw.settimeout(5)
                        peer = base.Peer(raw, self.context)
                        while not self.stop.is_set():
                            require(time.monotonic() < deadline, "relay session deadline")
                            armed = self.armed[epoch - 1].is_set()
                            if armed and gate.target and (mode != "unsub" or self.probed[epoch - 1].is_set()):
                                gate.parser.finish()
                                raw.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
                                stats["shutdown"] = "reset_after_complete_interception_and_pending_barrier"
                                break
                            ready, _, _ = select.select([raw, broker], [], [], 0.01)
                            if raw in ready or peer.tls.pending():
                                data = peer.recv(65536)
                                require(data, "unexpected client EOF")
                                streams["ada"].extend(data)
                                forwarded = gate.feed(data)
                                if forwarded:
                                    broker.sendall(forwarded)
                                    streams["forwarded"].extend(forwarded)
                                if gate.target:
                                    offset, target, text = gate.target
                                    stats.update(target_offset=offset, target_bytes=len(target), target_text=text,
                                                 target_sha256=hashlib.sha256(target).hexdigest())
                                    self.captured[epoch - 1].set()
                            if broker in ready:
                                data = broker.recv(65536)
                                if not data:
                                    require(mode == "normal", "unexpected broker EOF")
                                    gate.parser.finish()
                                    try:
                                        peer.tls.unwrap()
                                    except ssl.SSLWantReadError:
                                        pass
                                    peer.flush()
                                    stats["shutdown"] = "broker_eof_then_authenticated_tls_eof"
                                    break
                                streams["sleet"].extend(data)
                                peer.sendall(data)
                                streams["delivered"].extend(data)
                        stats["completed"] = not self.stop.is_set()
                finally:
                    for side, data in streams.items():
                        (self.out / f"epoch{epoch}-{side}.bin").write_bytes(data)
                        stats[side + "_sha256"] = hashlib.sha256(data).hexdigest()
                    stats["wire"] = {side: frames(data) for side, data in streams.items()}
                    validate_boundary(bytes(streams["ada"]), bytes(streams["forwarded"]), stats)
                    (self.out / f"epoch{epoch}-wire.json").write_text(json.dumps(stats["wire"], indent=2) + "\n")
                    self.results.append(stats)
        except BaseException as error:
            self.errors.append(repr(error))
        finally:
            self.listener.close()


def on_armed(epoch, item, send, recv, relay):
    require(relay.captured[epoch - 1].wait(5), "complete intercepted command deadline")
    if item["mode"] == "unsub":
        # Real independent publisher, after interception and Ada's pending barrier.
        name = base.NAMES[2]
        topic = ("task110" if item["external"][2]["MessageHeader"]["SchemaVersion"] == "002.6.0" else "task106") + "-survivability"
        send(f"PUB {topic} " + json.dumps({name: item["external"][2]}))
        responses = [recv(), recv()]
        require(responses.count("+OK") == 1 and any(t.startswith("MSG w3 ") for t in responses),
                "independent broker ownership probe publication")


def on_probe(epoch, relay):
    require(epoch == 4 and relay.captured[epoch - 1].is_set(), "probe follows intercepted UNSUB")
    relay.probed[epoch - 1].set()


def validate_owners(owners, expected):
    require(len(owners) == 5, "five distinct broker connections")
    for epoch, connection in enumerate(sorted(owners, key=int), 1):
        require([tuple(r) for r in owners[connection]] == (expected[:2] if epoch == 2 else expected),
                "independent exact broker registration ownership")


def validate_boundary(ada, forwarded, result):
    parser = Parser()
    parsed = []
    for pos in range(0, len(ada), 65536):
        parsed.extend(parser.feed(ada[pos:pos + 65536]))
    parser.finish()
    if result["mode"] == "normal":
        require(ada == forwarded, "normal client stream unchanged")
        return
    targets = [(offset, raw, text) for offset, raw, text in parsed if
               (text == "UNSUB 3" if result["mode"] == "unsub" else bool(text and text.startswith("SUB 3 ")))]
    require(len(targets) == 1, "exactly one complete intercepted command")
    offset, raw, text = targets[0]
    require(offset + len(raw) == len(ada) and forwarded == ada[:offset], "entire target withheld at exact boundary")
    require((result["target_offset"], result["target_bytes"], result["target_text"], result["target_sha256"]) ==
            (offset, len(raw), text, hashlib.sha256(raw).hexdigest()), "interception metadata independently reparsed")


def validate_wire(results, release):
    """Independent expected commands, release fixtures, responses and ownership probe."""
    require(len(results) == len(MODES), "five actual sessions")
    prefix = "task110" if release == "26" else "task106"
    topics = [prefix + "-" + key for key in base.cases.MESSAGES]
    for epoch, (result, item) in enumerate(zip(results, plan(release)), 1):
        mode = item["mode"]
        require(result["completed"] and result["epoch"] == epoch and result["mode"] == mode, "epoch identity")
        wire = result["wire"]
        client, forwarded, server, delivered = (texts(wire[side]) for side in ("ada", "forwarded", "sleet", "delivered"))
        require(wire["sleet"] == wire["delivered"], "broker responses unchanged; no fabrication")
        require(forwarded == (client if mode == "normal" else client[:-1]), "exact forwarded commands")
        for side in ("sleet", "delivered"):
            require("sec-websocket-protocol: owp" in wire[side]["upgrade"].lower(), "genuine owp upgrade")
        operations = [t.split()[0] for t in client]
        wanted = ["INIT"] + ["SUB"] * 3
        if mode != "sub":
            wanted += ["PUB"] * 3
        if mode in ("unsub", "normal"):
            wanted += ["UNSUB"]
        require(operations == wanted, "no replay/duplicate/unintended commands")
        init = json.loads(client[0][5:])
        require(init == dict(versions=["1.0"], schema=f"002.{release[-1]}.0",
                            service_id=prefix + "-multimessage", verbose=True), "release-specific INIT")
        require([t for t in client if t.startswith("SUB ")] ==
                [f"SUB {sid} {name} {topic}" for sid, (name, topic) in enumerate(zip(base.NAMES, topics), 1)],
                "fresh explicit subscription identities")
        require([t for t in client if t.startswith("UNSUB ")] == (["UNSUB 3"] if mode in ("unsub", "normal") else []), "exact UNSUB")
        ack_count = 3 if mode == "sub" else 8 if mode == "normal" else 7
        require(delivered.count("+OK") == ack_count, "no phantom ACK")
        require(server.count("+OK") == ack_count, "no intercepted-command broker ACK")
        require(sum(t.startswith("INFO ") for t in server) == 1, "one actual INFO")
        require(all(t == "+OK" or t.startswith(("INFO ", "MSG ")) for t in server), "no broker errors")
        require(delivered == server, "all real broker responses delivered")
        if mode != "normal":
            require(result["shutdown"] == "reset_after_complete_interception_and_pending_barrier", "observed reset boundary")
            require(not any(f["opcode"] == 8 for capture in wire.values() for f in capture["frames"]), "no WS Close")
        else:
            require(any(f["opcode"] == 8 for f in wire["ada"]["frames"]), "clean client Close")
        publications = [t.split(" ", 2) for t in client if t.startswith("PUB ")]
        require([p[1] for p in publications] == ([] if mode == "sub" else topics), "publication routes")
        require([json.loads(p[2]) for p in publications] == ([] if mode == "sub" else
                [{name: doc} for name, doc in zip(base.NAMES, item["publish"])]), "authentic generated publications")
        messages = [t.split(" ", 2) for t in delivered if t.startswith("MSG ")]
        expected = [] if mode == "sub" else [(str(sid), {name: doc}) for phase in ("publish", "external")
                    for sid, (name, doc) in enumerate(zip(base.NAMES, item[phase]), 1)]
        if mode == "unsub":
            expected.append(("3", {base.NAMES[2]: item["external"][2]}))
            require(messages[-1][1] == "3" and json.loads(messages[-1][2]) == expected[-1][1],
                    "authentic broker ownership probe after UNSUB interception")
        actual = [(p[1], json.loads(p[2])) for p in messages]
        # Sleet may schedule the three families in either order within a phase.
        require(len(actual) == len(expected) and
                sorted(json.dumps(row, sort_keys=True) for row in actual) ==
                sorted(json.dumps(row, sort_keys=True) for row in expected),
                "exact authentic current-epoch deliveries including ownership probe")


def validate_output(output):
    require(output.count("PROBE_REJECTED 4 sid=3 busy=true acks=7") == 1, "retired SID rejects actual broker probe")
    mapped = output.replace("TASK114", "TASK113")
    for epoch in (1, 3, 5):
        marker = f"EVENT {epoch} COMMAND_ACCEPTED 3 +OK\n"
        require(mapped.count(marker) == 2, "normal SUB and UNSUB acknowledgments")
        before, after = mapped.rsplit(marker, 1)
        mapped = before + after
        mapped = mapped.replace(f"RETIRED {epoch} callbacks= 2 2 2 acks= 8", f"RETIRED {epoch} callbacks= 2 2 2 acks= 7")
        require(output.count(f"CONTROL_UNSUB {epoch} sid=3 acks=8 active=false") == 1, "Ada positive-control UNSUB")
    return previous.validate_output(mapped)


def validate_broker(log, owners, results):
    observed = {}
    for connection, sid, topic, name in re.findall(
            r'connection\{id=(\d+)[^\n]*subscription registered sid=([123]) topic=(\S+) message_name=(\S+)', log):
        observed.setdefault(connection, []).append((sid, topic, name))
    require(observed == {key: [tuple(row) for row in rows] for key, rows in owners.items()},
            "ownership independently reconstructed from broker log")
    require(len(owners) == 5, "five broker connections")
    prefix = "task110" if owners[sorted(owners, key=int)[0]][0][1].startswith("task110-") else "task106"
    validate_owners(owners, [(str(sid), prefix + "-" + family, name)
                            for sid, (family, name) in enumerate(base.cases.MESSAGES.items(), 1)])
    observations = []
    require([row["mode"] for row in results] == list(MODES), "broker epoch association")
    for epoch, connection in enumerate(sorted(owners, key=int), 1):
        mode = MODES[epoch - 1]
        rows = [line for line in log.splitlines() if re.search(rf"connection\{{id={connection}(?: |\}})", line)]
        registered = [line for line in rows if "subscription registered sid=3 " in line]
        removed = [line for line in rows if "subscription removed sid=" in line]
        require(sum("disconnected" in line for line in rows) == 1, "broker disconnect cleanup observed")
        received = [re.search(r'recv operation="(\w+)"', line).group(1)
                    for line in rows if re.search(r'recv operation="(\w+)"', line)]
        expected_ops = ["INIT"] + ["SUB"] * (2 if mode == "sub" else 3)
        if mode != "sub":
            expected_ops += ["PUB"] * 3
        if mode == "normal":
            expected_ops += ["UNSUB"]
        require(received == expected_ops, "broker received exactly forwarded operations")
        acks = sum('send operation="+OK"' in line for line in rows)
        require(acks == len(expected_ops), "source-backed broker ACK count")
        require(len(registered) == (mode != "sub"), "broker third SUB positive/negative control")
        require(len(removed) == (mode == "normal"), "application UNSUB processing, not disconnect cleanup")
        if removed:
            require("subscription removed sid=3" in removed[0], "positive-control removes selected SID")
        observations.append(dict(epoch=epoch, operation=mode, broker_connection=connection,
                                 processed=mode == "normal", registration=registered, application_removal=removed,
                                 received_operations=received, broker_acks=acks))
    return dict(pending_operations=observations)


def validate_observer(observations, release):
    prefix = "task110" if release == "26" else "task106"
    topics = [prefix + "-" + family for family in base.cases.MESSAGES]
    expected = ["INIT"] + [f"SUB w{sid} {name} {topic}" for sid, (name, topic)
                           in enumerate(zip(base.NAMES, topics), 1)]
    documents = []
    for item in plan(release):
        if item["mode"] == "sub":
            continue
        for name, topic, doc in zip(base.NAMES, topics, item["external"]):
            expected.append("PUB " + topic + " " + json.dumps({name: doc}, sort_keys=True))
        for phase in ("publish", "external"):
            documents.extend((f"w{sid}", {name: doc}) for sid, (name, doc)
                             in enumerate(zip(base.NAMES, item[phase]), 1))
        if item["mode"] == "unsub":
            name, doc = base.NAMES[2], item["external"][2]
            expected.append("PUB " + topics[2] + " " + json.dumps({name: doc}, sort_keys=True))
            documents.append(("w3", {name: doc}))
    expected.append("UNSUB w1")
    sent, delivered = [], []
    for row in observations:
        text = row["text"]
        if row["direction"] == "send":
            if text.startswith("INIT "):
                require(json.loads(text[5:]) == dict(versions=["1.0"], schema=f"002.{release[-1]}.0",
                    service_id=prefix + "-multimessage", verbose=True), "observer release INIT")
                sent.append("INIT")
            elif text.startswith("PUB "):
                op, topic, doc = text.split(" ", 2)
                sent.append(f"{op} {topic} " + json.dumps(json.loads(doc), sort_keys=True))
            else:
                sent.append(text)
        elif row["direction"] == "receive" and text.startswith("MSG "):
            _, sid, doc = text.split(" ", 2)
            delivered.append((sid, json.loads(doc)))
        else:
            require(row["direction"] == "receive" and (text == "+OK" or text.startswith("INFO ")),
                    "no observer errors")
    require(sent == expected, "exact independent publisher commands including ownership probe")
    canonical = lambda rows: sorted(json.dumps(row, sort_keys=True) for row in rows)
    require(canonical(delivered) == canonical(documents), "independent authentic observer deliveries; no replay")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("evidence", "consumer", "sleet", "schema", "cert", "key"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--release", choices=("25", "26"), required=True)
    raise SystemExit(0 if base.execute(parser.parse_args(), sys.modules[__name__]) else 1)