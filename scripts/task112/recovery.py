#!/usr/bin/env python3
"""Test-only real Sleet/TLS fault campaign. Never synthesizes OWP/WS success."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import select
import socket
import ssl
import struct
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task108.wss_peer import Peer
from task109.relay import frames
from task110 import cases
from websockets.sync.client import connect

MODES = ("normal", "eof", "normal", "reset", "normal", "pending", "normal", "explicit")
NAMES = list(cases.MESSAGES.values())


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def plan(release):
    numbers = (102, 103, 105 if release == "26" else 104)
    base = [cases.family(n).golden(release) for n in numbers]
    result = []
    for epoch, mode in enumerate(MODES, 1):
        item = dict(epoch=epoch, mode=mode)
        for phase, key in enumerate(("publish", "external", "ambiguous"), 1):
            documents = copy.deepcopy(base)
            for document in documents:
                # Existing schema field, not an added epoch wire extension.
                document["MessageHeader"]["Timestamp"] = f"2026-01-{epoch:02}T03:04:{phase:02}Z"
            item[key] = documents[0] if key == "ambiguous" else documents
        result.append(item)
    return result


class Relay:
    def __init__(self, upstream, cert, key, out):
        self.upstream, self.out = upstream, out
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.context.load_cert_chain(cert, key)
        self.listener = socket.socket()
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen()
        self.listener.settimeout(0.2)
        self.port = self.listener.getsockname()[1]
        self.armed = [threading.Event() for _ in MODES]
        self.stop = threading.Event()
        self.errors, self.results = [], []
        self.thread = threading.Thread(target=self.serve)

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
                try:
                    with raw, socket.create_connection(("127.0.0.1", self.upstream), timeout=5) as broker:
                        raw.settimeout(5)
                        peer = Peer(raw, self.context)
                        withheld = False
                        while not self.stop.is_set():
                            require(time.monotonic() < deadline, "relay session deadline")
                            armed = self.armed[epoch - 1].is_set()
                            if armed and mode in ("eof", "reset", "pending"):
                                if mode != "pending" or withheld:
                                    if mode == "eof":
                                        try:
                                            peer.tls.unwrap()
                                        except ssl.SSLWantReadError:
                                            pass
                                        peer.flush()
                                        stats["shutdown"] = "authenticated_close_notify_without_ws_close"
                                    else:
                                        raw.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
                                        stats["shutdown"] = "socket_reset_without_close_notify"
                                    break
                            ready, _, _ = select.select([raw, broker], [], [], 0.01)
                            if raw in ready or peer.tls.pending():
                                data = peer.recv(65536)
                                if not data:
                                    stats["shutdown"] = "client_tls_eof"
                                    break
                                streams["ada"].extend(data)
                                broker.sendall(data)
                            if broker in ready:
                                data = broker.recv(65536)
                                if not data:
                                    # Pinned Sleet exits on WS Close without a
                                    # reciprocal frame. Supply only real TLS EOF.
                                    try:
                                        peer.tls.unwrap()
                                    except ssl.SSLWantReadError:
                                        pass
                                    peer.flush()
                                    stats["shutdown"] = "broker_eof_then_authenticated_tls_eof"
                                    break
                                streams["sleet"].extend(data)
                                hold = False
                                if mode == "pending":
                                    try:
                                        sent = frames(streams["ada"])
                                    except ValueError:
                                        sent = None  # incomplete capture, not a protocol decision
                                    if sent and sum(f["opcode"] == 1 and f["text"].startswith("PUB ")
                                                    for f in sent["frames"]) == 4:
                                        # This is the next real broker response
                                        # after three fully acknowledged PUBs.
                                        # Keep it off Ada's socket, including MSG.
                                        hold = True
                                        try:
                                            responses = frames(streams["sleet"])
                                        except ValueError:
                                            responses = None
                                        withheld = bool(responses and sum(
                                            f["opcode"] == 1 and f["text"].startswith("+OK")
                                            for f in responses["frames"]) == 8)
                                if not hold:
                                    peer.sendall(data)
                                    streams["delivered"].extend(data)
                        stats["completed"] = not self.stop.is_set()
                finally:
                    for name, data in streams.items():
                        (self.out / f"epoch{epoch}-{name}.bin").write_bytes(data)
                        stats[name + "_sha256"] = hashlib.sha256(data).hexdigest()
                    wire = {name: frames(data) for name, data in streams.items()}
                    (self.out / f"epoch{epoch}-wire.json").write_text(json.dumps(wire, indent=2) + "\n")
                    stats["wire"] = wire
                    self.results.append(stats)
        except BaseException as error:
            self.errors.append(repr(error))
        finally:
            self.listener.close()


def validate_wire(results, release):
    require(len(results) == 8, "eight actual sessions")
    for epoch, result in enumerate(results, 1):
        require(result["completed"], "incomplete relay epoch")
        wire = result["wire"]
        client = [f["text"] for f in wire["ada"]["frames"] if f["opcode"] == 1]
        server = [f["text"] for f in wire["sleet"]["frames"] if f["opcode"] == 1]
        delivered = [f["text"] for f in wire["delivered"]["frames"] if f["opcode"] == 1]
        require("sec-websocket-protocol: owp" in wire["sleet"]["upgrade"].lower(), "genuine owp upgrade")
        require(sum(t.startswith("INIT ") for t in client) == 1, "one INIT per epoch")
        init = json.loads(next(t[5:] for t in client if t.startswith("INIT ")))
        require(init["schema"] == f"002.{release[-1]}.0", "release-specific INIT")
        require(init.get("service_id") == ("task110-multimessage" if release == "26" else "task106-multimessage"), "authentic service INIT")
        require(sum(t.startswith("INFO ") for t in server) == 1, "one real INFO")
        require([t.split()[1] for t in client if t.startswith("SUB ")] == ["1", "2", "3"], "fresh subscriptions")
        require(sum(t.startswith("PUB ") for t in client) == (4 if result["mode"] == "pending" else 3), "no replay/duplicate PUB")
        require(sum(t.startswith("+OK") for t in delivered) == 7, "only current epoch ACKs")
        require(sum(t.startswith("MSG ") for t in delivered) == 6, "six authentic typed deliveries")
        if result["mode"] == "pending":
            require(sum(t.startswith("+OK") for t in server) == 8, "real PUB ACK withheld before reset")
        if result["mode"] in ("eof", "reset", "pending"):
            require(not any(f["opcode"] == 8 for side in wire.values() for f in side["frames"]), "no fabricated WS Close")


def validate_output(output):
    require("TASK112_COMPLETE epochs=8" in output and "TASK112_FAILED" not in output, "Ada completion")
    require(output.count("CALL ") == 48 and output.count("RETIRED ") == 8, "exact callback counts")
    resources = re.findall(r"^CLEANUP (\d+) fd=\s*(\d+) tasks=\s*(\d+)$", output, re.M)
    require([int(row[0]) for row in resources] == list(range(1, 9)), "all epoch cleanup receipts")
    require(len({tuple(row[1:]) for row in resources}) == 1, "no FD/task growth")
    for epoch, mode in enumerate(MODES, 1):
        for family in range(1, 4):
            for count in (1, 2):
                require(output.splitlines().count(f"CALL {epoch} {family} {count}") == 1, "epoch handler isolation")
        require(output.count(f"EVENT {epoch} CONNECTION_CLOSED ") == 1, "one Connection_Closed notification")
        require(output.count(f"EVENT {epoch} COMMAND_ACCEPTED ") == 7, "no phantom application ACK")
        require(output.count(f"EVENT {epoch} INITIALIZED ") == 1, "fresh INFO event")
        if mode not in ("reset", "pending"):
            require(f"TRANSPORT_EXCEPTION {epoch} " not in output, "unexpected transport exception")
    return resources


def execute(args, campaign=None):
    # Optional test-only campaign hooks; the historical Task112 defaults remain.
    campaign_plan = getattr(campaign, "plan", plan)
    campaign_relay = getattr(campaign, "Relay", Relay)
    campaign_wire = getattr(campaign, "validate_wire", validate_wire)
    campaign_output = getattr(campaign, "validate_output", validate_output)
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    release = args.release
    service = "task110-multimessage" if release == "26" else "task106-multimessage"
    prefix = "task110" if release == "26" else "task106"
    topics = [prefix + "-" + key for key in cases.MESSAGES]
    corpus = campaign_plan(release)
    (out / "plan.jsonl").write_text("".join(json.dumps(item) + "\n" for item in corpus))
    services = out / "services.d"
    services.mkdir()
    (services / "consumer.toml").write_text(f'service_id = "{service}"\n'
        'service_uuid = "550e8400-e29b-41d4-a716-446655440112"\nallowed_topics = ' + json.dumps(topics) + '\n')
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    config = out / "server.toml"
    config.write_text(f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task112"\nsystem_label = "Task112"\n'
        f'schema_path = "{args.schema.resolve()}"\nschema_version = "002.{release[-1]}.0"\nservices_dir = "{services}"\n'
        'system_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    server_cmd = [str(args.sleet.resolve() / "target/release/sleet"), str(config)]
    (out / "server.command.json").write_text(json.dumps(server_cmd) + "\n")
    relay = campaign_relay(port, args.cert, args.key, out)
    child = reader = None
    result = dict(completed=False, release=release, cleanup=False)
    observations = []
    with (out / "server.stdout").open("wb") as log, (out / "server.stderr").open("wb") as err:
        server = subprocess.Popen(server_cmd, stdout=log, stderr=err, env=dict(os.environ, RUST_LOG="debug", NO_COLOR="1"))
        try:
            deadline = time.monotonic() + 20
            while "listening" not in (out / "server.stdout").read_text():
                require(server.poll() is None and time.monotonic() < deadline, "broker startup deadline")
                time.sleep(0.01)
            relay.thread.start()
            with connect(f"ws://127.0.0.1:{port}/", subprotocols=["owp"], open_timeout=5, close_timeout=1) as witness:
                def recv():
                    message = witness.recv(timeout=5)
                    observations.append(dict(direction="receive", text=message))
                    return message

                def send(message):
                    observations.append(dict(direction="send", text=message))
                    witness.send(message)

                send("INIT " + json.dumps(dict(versions=["1.0"], schema=f"002.{release[-1]}.0", service_id=service, verbose=True)))
                initial = [recv(), recv()]
                require(any(t.startswith("INFO ") for t in initial) and any(t.startswith("+OK") for t in initial), "witness INIT")
                for sid, (name, topic) in enumerate(zip(NAMES, topics), 1):
                    send(f"SUB w{sid} {name} {topic}")
                    require(recv().startswith("+OK"), "witness subscription")

                def witness_deliveries(documents):
                    wanted = {"MSG w" + str(i) + " " + json.dumps({name: doc}, sort_keys=True)
                              for i, (name, doc) in enumerate(zip(NAMES, documents), 1)}
                    got = set()
                    for _ in documents:
                        text = recv()
                        operation, sid, document = text.split(" ", 2)
                        canonical = operation + " " + sid + " " + json.dumps(json.loads(document), sort_keys=True)
                        require(canonical in wanted and canonical not in got, "independent witness value/duplicate")
                        got.add(canonical)

                command = [str(args.consumer.resolve()), f"wss://localhost:{relay.port}/", str(out / "plan.jsonl"), f"002.{release[-1]}.0"]
                (out / "consumer.command.json").write_text(json.dumps(command) + "\n")
                with (out / "consumer.stderr").open("w") as stderr:
                    child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=stderr, text=True,
                                             env=dict(os.environ, OMS_WSS_TEST_CA=str(args.cert.resolve())))
                    lines = queue.Queue()

                    def collect():
                        try:
                            with (out / "consumer.stdout").open("w") as stream:
                                for line in child.stdout:
                                    stream.write(line); stream.flush(); lines.put(line.strip())
                        finally:
                            lines.put(None)

                    reader = threading.Thread(target=collect)
                    reader.start()
                    while True:
                        try:
                            line = lines.get(timeout=12)
                        except queue.Empty as error:
                            raise TimeoutError("HARNESS_TIMEOUT waiting for Ada observation") from error
                        if line is None:
                            break
                        parts = line.split()
                        if not parts:
                            continue
                        if parts[0] == "READY" and parts[2] == "2":
                            item = corpus[int(parts[1]) - 1]
                            witness_deliveries(item["publish"])
                            for name, topic, doc in zip(NAMES, topics, item["external"]):
                                send(f"PUB {topic} " + json.dumps({name: doc}))
                                responses = [recv(), recv()]
                                require(sum(t.startswith("+OK") for t in responses) == 1, "external publisher ACK")
                                msg = next(t for t in responses if t.startswith("MSG "))
                                require(json.loads(msg.split(" ", 2)[2]) == {name: doc}, "external publication witnessed")
                        elif parts[0] == "ARMED":
                            epoch = int(parts[1])
                            if parts[2] == "pending":
                                text = recv()
                                require(text.startswith("MSG w1 ") and json.loads(text.split(" ", 2)[2]) ==
                                        {"Log": corpus[epoch - 1]["ambiguous"]}, "ambiguous PUB independently observed once")
                            relay.armed[epoch - 1].set()
                    require(child.wait(timeout=5) == 0, "Ada application failure (see consumer.stdout)")
                relay.thread.join(5)
                require(not relay.thread.is_alive() and not relay.errors, "relay failure: " + repr(relay.errors))
                # A protocol barrier proves no latent duplicate publication is
                # queued on the same ordered observer stream after recovery.
                send("UNSUB w1")
                require(recv().startswith("+OK"), "unexpected queued/replayed witness delivery")
            campaign_wire(relay.results, release)
            output = (out / "consumer.stdout").read_text()
            resources = campaign_output(output)
            require(server.poll() is None, "same broker survived campaign")
            broker_log = (out / "server.stdout").read_text()
            registrations = re.findall(r'connection\{id=(\d+)[^\n]*subscription registered sid=([123]) topic=(\S+) message_name=(\S+)', broker_log)
            owners = {}
            for connection, sid, topic, name in registrations:
                owners.setdefault(connection, []).append((sid, topic, name))
            expected = [(str(i), topic, name) for i, (topic, name) in enumerate(zip(topics, NAMES), 1)]
            require(len(owners) == len(corpus) and all(rows == expected for rows in owners.values()),
                    "broker must independently register three subscriptions on eight distinct Ada connections")
            result.update(completed=True, epochs=len(corpus), callbacks=output.count("CALL "), ambiguous_publications=1 if campaign is None else 0,
                          broker_survived=True, consumer_exit=child.returncode, resources=resources,
                          broker_connections=owners)
            if campaign is not None:
                result.update(campaign.validate_broker(broker_log, owners, relay.results))
        except Exception as error:
            result["error"] = f"{type(error).__name__}: {error}"
        finally:
            if child is not None:
                if child.poll() is None:
                    child.kill(); child.wait(timeout=5)
                if reader is not None:
                    reader.join(5)
                child.stdout.close()
                (out / "consumer.exit").write_text(str(child.returncode) + "\n")
            relay.stop.set()
            if relay.thread.ident is not None:
                relay.thread.join(6)
            else:
                relay.listener.close()
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill(); server.wait(timeout=5)
                result["forced_broker_kill"] = True
            (out / "server.exit").write_text(str(server.returncode) + "\n")
    result.update(server_exit=server.returncode, relay_errors=relay.errors,
                  epochs_observed=[{k: v for k, v in item.items() if k != "wire"} for item in relay.results])
    result["cleanup"] = (not relay.thread.is_alive() and (reader is None or not reader.is_alive())
                         and server.returncode == 0 and not result.get("forced_broker_kill"))
    with socket.socket() as probe:
        probe.settimeout(1)
        result["broker_socket_closed"] = probe.connect_ex(("127.0.0.1", port)) != 0
    result["cleanup"] = result["cleanup"] and result["broker_socket_closed"]
    (out / "observer.json").write_text(json.dumps(observations, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result), flush=True)
    return result["completed"] and result["cleanup"] and not result.get("error")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("evidence", "consumer", "sleet", "schema", "cert", "key"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--release", choices=("25", "26"), required=True)
    raise SystemExit(0 if execute(parser.parse_args()) else 1)