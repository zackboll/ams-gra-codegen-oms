#!/usr/bin/env python3
"""Real WSS acceptance for the standalone executable, using the pinned TLS peer."""
import argparse
import copy
import json
import os
from pathlib import Path
import queue
import re
import select
import socket
import ssl
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task115 import build
from task112.recovery import Relay as ExistingRelay, Peer, require
from task109.relay import frames
from task110 import cases
from websockets.sync.client import connect

NAMES = list(cases.MESSAGES.values())
TOPICS = ["uci.log", "uci.rdma", "uci.survivability"]


class Relay(ExistingRelay):
    """Forward real broker bytes unchanged; interrupt only on callback barrier."""
    def serve(self):
        epoch = 0
        try:
            while not self.stop.is_set():
                try:
                    raw, _ = self.listener.accept()
                except socket.timeout:
                    continue
                epoch += 1
                streams = {side: bytearray() for side in ("ada", "sleet", "delivered")}
                stats = dict(epoch=epoch, completed=False)
                try:
                    with raw, socket.create_connection(("127.0.0.1", self.upstream), timeout=5) as broker:
                        raw.settimeout(5)
                        peer = Peer(raw, self.context)
                        deadline = time.monotonic() + 40
                        while not self.stop.is_set():
                            require(time.monotonic() < deadline, "relay session timeout")
                            if self.armed[epoch - 1].is_set():
                                try:
                                    peer.tls.unwrap()
                                except ssl.SSLWantReadError:
                                    pass
                                peer.flush()
                                stats["shutdown"] = "authenticated_close_notify_without_ws_close"
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
                    stats["completed"] = True
                except ssl.SSLError as error:
                    # An invalid-trust test must abort TLS before any WS bytes.
                    if not self.allow_tls_rejection or streams["ada"]:
                        raise
                    stats.update(completed=True, shutdown="tls_rejected", tls_error=type(error).__name__)
                finally:
                    for side, data in streams.items():
                        path = self.out / f"epoch{epoch}-{side}.wire"
                        path.write_bytes(data)
                    self.results.append(stats)
        except Exception as error:
            self.errors.append(f"{type(error).__name__}: {error}")
        finally:
            self.listener.close()


def run_case(args, name, budget, epochs, interrupts, duration=4, permanent=None, pem=False):
    out = args.evidence.resolve() / name
    out.mkdir(parents=True, exist_ok=False)
    services = out / "services.d"
    services.mkdir()
    service_id = "denied-service" if permanent == "service" else "ada-uci-client"
    (services / "client.toml").write_text(f'service_id = "{service_id}"\n'
        'service_uuid = "550e8400-e29b-41d4-a716-446655440115"\nallowed_topics = ' + json.dumps(TOPICS) + '\n')
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    schema = args.build.resolve() / "schema" / next(iter(build.SCHEMAS[args.release]))
    config = out / "server.toml"
    config.write_text(f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task115"\nsystem_label = "Task115"\n'
        f'schema_path = "{schema}"\nschema_version = "002.{args.release[-1]}.0"\nservices_dir = "{services}"\n'
        'system_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    relay = Relay(port, args.cert, args.key, out)
    relay.allow_tls_rejection = permanent == "trust"
    server = child = reader = None
    resources, output, observations = [], [], []
    result = dict(completed=False, name=name, budget=budget, expected_epochs=epochs)
    start = time.monotonic()
    try:
        with (out / "server.stdout").open("w") as log, (out / "server.stderr").open("w") as err:
            server = subprocess.Popen([args.deps / "sleet/target/release/sleet", config], stdout=log, stderr=err,
                                      env=dict(os.environ, RUST_LOG="debug", NO_COLOR="1"))
        limit = time.monotonic() + 20
        while "listening" not in (out / "server.stdout").read_text():
            require(server.poll() is None and time.monotonic() < limit, "Sleet startup timeout")
            time.sleep(0.01)
        relay.thread.start()
        with connect(f"ws://127.0.0.1:{port}/", subprotocols=["owp"], open_timeout=5, close_timeout=1) as witness:
            def send(text):
                observations.append(dict(direction="send", text=text))
                witness.send(text)

            def recv():
                text = witness.recv(timeout=5)
                observations.append(dict(direction="receive", text=text))
                return text

            send("INIT " + json.dumps(dict(versions=["1.0"], schema=f"002.{args.release[-1]}.0",
                                         service_id=service_id, verbose=True)))
            initial = [recv(), recv()]
            require("+OK" in initial and any(t.startswith("INFO ") for t in initial), "witness INIT")
            for sid, (message, topic) in enumerate(zip(NAMES, TOPICS), 1):
                send(f"SUB w{sid} {message} {topic}")
                require(recv() == "+OK", "witness SUB")
            trust = args.wrong_cert if permanent == "trust" else args.cert
            command = [str(args.build.resolve() / "build/bin/ada-uci-client"), "--url",
                       f"wss://localhost:{relay.port}/", "--ca-pem-file" if pem else "--ca-path", str(trust.resolve()),
                       "--duration-seconds", str(duration), "--reconnect-attempts", str(budget),
                       "--reconnect-delay-ms", "100"]
            (out / "consumer.command.json").write_text(json.dumps(command) + "\n")
            with (out / "consumer.stderr").open("w") as stderr:
                child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=stderr, text=True)
            lines = queue.Queue()

            def collect():
                with (out / "consumer.stdout").open("w") as stream:
                    for line in child.stdout:
                        stream.write(line); stream.flush(); lines.put(line.strip())
                lines.put(None)

            reader = threading.Thread(target=collect)
            reader.start()
            corpus = [cases.family(n).golden(args.release) for n in
                      (102, 103, 105 if args.release == "26" else 104)]
            callbacks = {}
            while True:
                require(time.monotonic() - start < 35, "HARNESS_TIMEOUT")
                line = lines.get(timeout=12)
                if line is None:
                    break
                output.append(line)
                match = re.fullmatch(r"SUBSCRIBED epoch=(\d+) SurvivabilityRiskLevel sid=3", line)
                if match:
                    epoch = int(match[1])
                    for message, topic, original in zip(NAMES, TOPICS, corpus):
                        doc = copy.deepcopy(original)
                        doc["MessageHeader"]["Timestamp"] = f"2026-01-{epoch:02}T03:04:05Z"
                        send(f"PUB {topic} " + json.dumps({message: doc}))
                        responses = [recv(), recv()]
                        require(responses.count("+OK") == 1, "independent publisher acknowledgment")
                        observed = next(t for t in responses if t.startswith("MSG "))
                        require(json.loads(observed.split(" ", 2)[2]) == {message: doc}, "independent authentic delivery")
                match = re.match(r"RECEIVED epoch=(\d+) (\S+) timestamp=(\S+)", line)
                if match:
                    epoch, family, timestamp = int(match[1]), match[2], match[3]
                    require(timestamp == f"2026-01-{epoch:02}T03:04:05Z", "old epoch callback")
                    callbacks.setdefault(epoch, []).append(family)
                    if len(callbacks[epoch]) == 3 and epoch <= interrupts:
                        relay.armed[epoch - 1].set()
                match = re.fullmatch(r"RESOURCES epoch=(\d+) fd=(\d+) tasks=(\d+)", line)
                if match:
                    resources.append(dict(zip(("epoch", "fd", "tasks"), map(int, match.groups()))))
            expected_exit = 1 if interrupts == epochs or permanent else 0
            require(child.wait(timeout=3) == expected_exit, "unexpected application exit")
            reader.join(3)
            # Listener remains active until after process exit, detecting extra attempts.
            time.sleep(0.15)
            require(len(relay.results) == epochs, "application connection count")
            if not permanent:
                require(callbacks == {e: NAMES for e in range(1, epochs + 1)}, "three typed callbacks each epoch")
                broker_log = (out / "server.stdout").read_text()
                require(broker_log.count("disconnected") >= epochs, "broker retirement cleanup")
            else:
                require(not callbacks and not any(x.startswith("RECONNECT") for x in output), "permanent failure retried")
            require(not relay.errors, "relay errors: " + repr(relay.errors))
            result.update(completed=True, exit=child.returncode, callbacks=callbacks,
                          resources=resources, elapsed=time.monotonic() - start)
    except Exception as error:
        result["error"] = f"{type(error).__name__}: {error}"
    finally:
        if child is not None:
            if child.poll() is None:
                child.kill(); child.wait(timeout=5)
            if reader is not None:
                reader.join(5)
            child.stdout.close()
        relay.stop.set()
        if relay.thread.ident is not None:
            relay.thread.join(6)
        else:
            relay.listener.close()
        if server is not None:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill(); server.wait(timeout=5)
                result["forced_kill"] = True
        with socket.socket() as probe:
            probe.settimeout(1)
            result["broker_socket_closed"] = probe.connect_ex(("127.0.0.1", port)) != 0
        result.update(relay=relay.results, relay_errors=relay.errors,
                      cleanup=not relay.thread.is_alive() and (reader is None or not reader.is_alive()) and
                      server is not None and server.returncode == 0 and not result.get("forced_kill") and
                      result["broker_socket_closed"])
        (out / "observer.json").write_text(json.dumps(observations, indent=2) + "\n")
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    require(result["completed"] and result["cleanup"], str(result))
    print("CASE_PASS", name, flush=True)
    return result


def execute(args):
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    metadata = json.loads((args.build / "build.json").read_text())
    require(metadata["inputs"] == build.inputs(), "build inputs changed")
    require(metadata["release"] == args.release, "wrong compiled release")
    for name, pin in build.PINS.items():
        dep = args.deps / name
        require(subprocess.check_output(["git", "-C", dep, "rev-parse", "HEAD"], text=True).strip() == pin, "dependency identity")
        require(not subprocess.check_output(["git", "-C", dep, "status", "--porcelain"]), "dirty dependency")
    for name in ("cert", "wrong-cert"):
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
                        "-keyout", out / (name + "-key.pem"), "-out", out / (name + ".pem"),
                        "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost"],
                       check=True, capture_output=True, timeout=10)
        (out / (name + "-key.pem")).chmod(0o600)
    args.cert, args.key, args.wrong_cert = out / "cert.pem", out / "cert-key.pem", out / "wrong-cert.pem"
    results = []
    for name, budget, epochs, interrupts, extra in (
            ("default", 0, 1, 1, {}), ("recovery", 2, 3, 2, {"duration": 5}),
            ("exhausted", 1, 2, 2, {}), ("invalid-trust", 2, 1, 0, {"permanent": "trust"}),
            ("denied-service", 2, 1, 0, {"permanent": "service"}),
            ("normal", 2, 1, 0, {"duration": 2, "pem": True})):
        results.append(run_case(args, name, budget, epochs, interrupts, **extra))
    deadline_probe = probe_deadline(args)
    (out / "result.json").write_text(json.dumps(dict(completed=True, build=metadata, cases=results,
        deadline_probe=deadline_probe,
        sleet_binary_sha256=build.digest(args.deps / "sleet/target/release/sleet")), indent=2) + "\n")
    from task115.verify import verify
    verify(out)
    (out / "artifacts.json").write_text(json.dumps({str(p.relative_to(out)): build.digest(p)
        for p in out.rglob("*") if p.is_file() and p.name != "artifacts.json" and "key.pem" not in p.name}, indent=2) + "\n")
    print("TASK115_CASES_PASS", flush=True)


def probe_deadline(args):
    """Real stalled TLS setup: record API deadline overrun, NOT a passing case."""
    out = args.evidence / "deadline-probe"
    out.mkdir()
    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    listener.listen()
    listener.settimeout(5)
    port = listener.getsockname()[1]
    command = [str(args.build.resolve() / "build/bin/ada-uci-client"), "--url",
               f"wss://localhost:{port}/", "--ca-path", str(args.cert),
               "--duration-seconds", "1", "--reconnect-attempts", "2"]
    (out / "consumer.command.json").write_text(json.dumps(command) + "\n")
    start = time.monotonic()
    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as child:
        try:
            raw, _ = listener.accept()
            with raw:
                # The TCP endpoint accepts but never answers the actual ClientHello.
                stdout, stderr = child.communicate(timeout=10)
        finally:
            listener.close()
        elapsed = time.monotonic() - start
        (out / "consumer.stdout").write_text(stdout)
        (out / "consumer.stderr").write_text(stderr)
        result = dict(requested_seconds=1, elapsed=elapsed, exit=child.returncode,
                      deadline_overrun_observed=elapsed > 1.5, retry_count=stdout.count("RECONNECT"))
        require(child.returncode == 1 and result["retry_count"] == 0, "stalled setup fail-closed behavior")
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("build", "evidence", "deps"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--release", choices=("25", "26"), required=True)
    execute(parser.parse_args())