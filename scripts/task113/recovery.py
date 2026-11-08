#!/usr/bin/env python3
"""Pending SUB/UNSUB fault controls over real verified TLS and pinned Sleet.

Only withhold actual broker bytes. Never manufacture protocol responses.
The Task112 controller owns broker/witness/process lifetimes and fixtures.
"""
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
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task112 import recovery as base

require = base.require
frames = base.frames
MODES = ("normal", "sub", "normal", "unsub", "normal")


def plan(release):
    corpus = base.plan(release)[:len(MODES)]
    for item, mode in zip(corpus, MODES):
        item["mode"] = mode
        del item["ambiguous"]
    return corpus


def texts(capture):
    return [f["text"] for f in capture["frames"] if f["opcode"] == 1]


def boundary(mode, capture):
    commands = texts(capture)
    return ((mode == "sub" and sum(t.startswith("SUB ") for t in commands) == 3) or
            (mode == "unsub" and "UNSUB 3" in commands))


class Relay(base.Relay):
    """Same TLS byte-forwarding architecture; two observed command boundaries."""
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
                streams = dict(ada=bytearray(), sleet=bytearray(), delivered=bytearray())
                stats = dict(epoch=epoch, mode=mode, completed=False)
                hold = withheld = False
                try:
                    with raw, socket.create_connection(("127.0.0.1", self.upstream), timeout=5) as broker:
                        raw.settimeout(5)
                        peer = base.Peer(raw, self.context)
                        while not self.stop.is_set():
                            require(time.monotonic() < deadline, "relay session deadline")
                            if self.armed[epoch - 1].is_set() and withheld:
                                raw.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
                                stats["shutdown"] = "socket_reset_after_actual_withheld_ack"
                                break
                            ready, _, _ = select.select([raw, broker], [], [], 0.01)
                            if raw in ready or peer.tls.pending():
                                data = peer.recv(65536)
                                require(data, "unexpected client EOF")
                                streams["ada"].extend(data)
                                broker.sendall(data)
                                try:
                                    hold = boundary(mode, frames(streams["ada"]))
                                except ValueError:
                                    pass  # Partial frame is not an observed command boundary.
                            if broker in ready:
                                data = broker.recv(65536)
                                if not data:
                                    require(mode == "normal", "broker EOF before withheld ACK")
                                    try:
                                        peer.tls.unwrap()
                                    except ssl.SSLWantReadError:
                                        pass
                                    peer.flush()
                                    stats["shutdown"] = "broker_eof_then_authenticated_tls_eof"
                                    break
                                streams["sleet"].extend(data)
                                if hold:
                                    try:
                                        responses = texts(frames(streams["sleet"]))
                                        withheld = responses.count("+OK") == (4 if mode == "sub" else 8)
                                    except ValueError:
                                        pass
                                    if withheld:
                                        stats["withheld_actual_ack"] = True
                                else:
                                    peer.sendall(data)
                                    streams["delivered"].extend(data)
                        stats["completed"] = not self.stop.is_set()
                finally:
                    for side, data in streams.items():
                        (self.out / f"epoch{epoch}-{side}.bin").write_bytes(data)
                        stats[side + "_sha256"] = hashlib.sha256(data).hexdigest()
                    stats["wire"] = {side: frames(data) for side, data in streams.items()}
                    (self.out / f"epoch{epoch}-wire.json").write_text(json.dumps(stats["wire"], indent=2) + "\n")
                    self.results.append(stats)
        except BaseException as error:
            self.errors.append(repr(error))
        finally:
            self.listener.close()


def validate_wire(results, release):
    require(len(results) == len(MODES), "five actual sessions")
    prefix = "task110" if release == "26" else "task106"
    topics = [prefix + "-" + key for key in base.cases.MESSAGES]
    for epoch, (result, item) in enumerate(zip(results, plan(release)), 1):
        mode = item["mode"]
        require(result["completed"] and result["epoch"] == epoch and result["mode"] == mode, "epoch identity")
        wire = result["wire"]
        client, server, delivered = (texts(wire[side]) for side in ("ada", "sleet", "delivered"))
        for side in ("sleet", "delivered"):
            require("sec-websocket-protocol: owp" in wire[side]["upgrade"].lower(), "genuine owp upgrade")
        operations = [t.split()[0] for t in client]
        wanted = ["INIT"] + ["SUB"] * 3
        if mode != "sub":
            wanted += ["PUB"] * 3
        if mode == "unsub":
            wanted += ["UNSUB"]
        require(operations == wanted, "no replay/duplicate/unintended commands")
        init = json.loads(client[0][5:])
        require(init == dict(versions=["1.0"], schema=f"002.{release[-1]}.0",
                            service_id=prefix + "-multimessage", verbose=True), "release-specific INIT")
        require([t for t in client if t.startswith("SUB ")] ==
                [f"SUB {sid} {name} {topic}" for sid, (name, topic) in enumerate(zip(base.NAMES, topics), 1)],
                "fresh explicit subscription identities")
        require([t for t in client if t.startswith("UNSUB ")] == (["UNSUB 3"] if mode == "unsub" else []), "exact UNSUB")
        ack_count = 3 if mode == "sub" else 7
        require(delivered.count("+OK") == ack_count, "no phantom ACK")
        require(server.count("+OK") == ack_count + (mode != "normal"), "actual withheld broker ACK")
        require(sum(t.startswith("INFO ") for t in server) == 1, "one actual INFO")
        require(all(t == "+OK" or t.startswith(("INFO ", "MSG ")) for t in server), "no broker errors")
        require(delivered == (server if mode == "normal" else server[:-1]), "withheld ACK never delivered")
        if mode != "normal":
            require(server[-1] == "+OK" and result["withheld_actual_ack"] and
                    result["shutdown"] == "socket_reset_after_actual_withheld_ack", "observed reset boundary")
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
        actual = [(p[1], json.loads(p[2])) for p in messages]
        # Sleet may schedule the three families in either order within a phase.
        require(len(actual) == len(expected) and
                sorted(json.dumps(row, sort_keys=True) for row in actual) ==
                sorted(json.dumps(row, sort_keys=True) for row in expected),
                "six authentic current-epoch deliveries")


def validate_output(output):
    require("TASK113_COMPLETE epochs=5" in output and "TASK113_FAILED" not in output, "Ada completion")
    lines = output.splitlines()
    resources = re.findall(r"^CLEANUP (\d+) fd=\s*(\d+) tasks=\s*(\d+)$", output, re.M)
    require([int(r[0]) for r in resources] == list(range(1, 6)), "all cleanup receipts")
    require(len({tuple(r[1:]) for r in resources}) == 1, "no FD/task growth")
    require(output.count("CALL ") == 24 and output.count("RETIRED ") == 5, "exact callbacks/retirements")
    require(output.count("PENDING ") == 2 and output.count("ARMED ") == 2, "only two interrupted commands")
    for epoch, mode in enumerate(MODES, 1):
        acks = 3 if mode == "sub" else 7
        calls = 0 if mode == "sub" else 2
        for family in range(1, 4):
            for count in (1, 2):
                require(lines.count(f"CALL {epoch} {family} {count}") == (mode != "sub"), "callback epoch isolation")
        require(output.count(f"EVENT {epoch} CONNECTION_CLOSED ") == 1, "one terminal event")
        require(output.count(f"EVENT {epoch} COMMAND_ACCEPTED ") == acks, "no phantom application ACK")
        require(output.count(f"EVENT {epoch} INITIALIZED ") == 1, "fresh INFO")
        require(lines.count(f"RETIRED {epoch} callbacks= {calls} {calls} {calls} acks= {acks} close=1 active=0 lost=0 busy=false") == 1,
                "terminal cleanup state")
        exceptions = [line for line in lines if line.startswith(f"TRANSPORT_EXCEPTION {epoch} ")]
        require(len(exceptions) == (mode != "normal"), "only expected reset exceptions")
        if mode != "normal":
            require("Connection reset by peer" in exceptions[0], "expected socket reset")
            require(f"PENDING {epoch} {mode} sid=3 busy=true active=true,true,false acks={acks}" in lines, "production pending state")
            require(f"ARMED {epoch} {mode}" in lines, "armed observed pending command")
    return resources


def validate_broker(log, owners, results):
    require(len(owners) == 5, "five distinct Ada broker sessions")
    require([row["mode"] for row in results] == list(MODES), "broker epoch association")
    connections = sorted(owners, key=int)
    observations = []
    for epoch, mode in enumerate(MODES, 1):
        connection = connections[epoch - 1]
        rows = [line for line in log.splitlines() if re.search(rf"connection\{{id={connection}(?: |\}})", line)]
        prefix = "task110" if owners[connection][0][1].startswith("task110-") else "task106"
        expected = [(str(sid), prefix + "-" + family, name)
                    for sid, (family, name) in enumerate(base.cases.MESSAGES.items(), 1)]
        require([tuple(row) for row in owners[connection]] == expected, "broker subscription ownership")
        removed = [line for line in rows if "subscription removed sid=" in line]
        require(len(removed) == (mode == "unsub"), "exact broker UNSUB processing")
        if mode != "normal":
            marker = "subscription registered sid=3 " if mode == "sub" else "subscription removed sid=3"
            evidence = [line for line in rows if marker in line]
            require(len(evidence) == 1, "independent broker command processing")
            observations.append(dict(epoch=epoch, operation=mode, broker_connection=connection,
                                     processed=True, acknowledged_to_ada=False, broker_log=evidence[0]))
    return dict(pending_operations=observations)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("evidence", "consumer", "sleet", "schema", "cert", "key"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--release", choices=("25", "26"), required=True)
    raise SystemExit(0 if base.execute(parser.parse_args(), sys.modules[__name__]) else 1)