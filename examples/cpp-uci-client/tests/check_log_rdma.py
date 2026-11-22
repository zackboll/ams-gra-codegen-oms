#!/usr/bin/env python3
"""Focused both-release qualification using the CPP-OMS-008 raw peers."""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess as sp
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))  # reuse Task103's authentic RDMA cases
from task103 import cases

SLEET = "e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b"
TREE = "7e705f4eb5f658957bd414b4abc8207311f3c1e4"
RDMA = "RDMA_InitializeSetup"
FIXTURES = ROOT / "tests/fixtures/service-generate"


@contextmanager
def launch(command, log, **kwargs):
    with log.open("w") as out, sp.Popen(command, stdout=out, stderr=sp.STDOUT, **kwargs) as process:
        try:
            yield process
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


def wait_line(log, regex, process, seconds=10):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        match = re.search(regex, log.read_text(), re.M)
        if match:
            return match
        if process.poll() is not None:
            break
        time.sleep(0.02)
    raise AssertionError(f"missing {regex!r} in {log}")


def checked_exit(process, log, code):
    actual = process.wait(timeout=20)
    assert actual == code, (log, actual, code)
    return log.read_text()


def assert_closed(text, *sids):
    assert re.findall(r"^UNSUBSCRIBED sid=(\S+) acknowledged", text, re.M) == list(sids)
    assert "callbacks-after-unsubscribe=0" in text
    assert re.search(r"runtime-threads=0 sockets=(\d+) sockets-at-start=\1 resources-released=yes$", text, re.M)


def plans(cell, v):
    def compact(doc):
        return json.dumps(doc, separators=(",", ":"))

    def log(n):
        doc = json.loads((FIXTURES / f"task093-real-log{v}.json").read_text())
        doc["MessageData"]["Timestamp"] = f"2026-01-02T03:05:{n:02d}Z"
        if n == 0:
            doc["MessageData"]["Data"] = "000"
        return doc

    first = cases.golden(v)
    second = next(doc for kind, doc, _ in cases.positives(v) if kind == "reversed")
    second["MessageData"]["RDMA_InitializeSetupID"]["UUID"] = "12345678-1234-1234-8234-123456789abd"
    second["MessageData"]["Endpoint"][0]["IPv6_Endpoint"]["IPv6_Address"] = "fE80::aBcD"
    second["MessageData"]["Endpoint"][0]["IPv6_Endpoint"]["Description"] = "do not echo"
    rejected = dict(cases.invalid_documents(v))["ipv4-invalid"]
    decoder_bad = dict(cases.decoder_invalid(v))["invalid-timestamp"]
    (cell / "rdma.json").write_text(compact({RDMA: first}) + "\n")
    many = cases.golden(v)
    many["MessageData"]["Endpoint"] = second["MessageData"]["Endpoint"] * 2 + [
        {"IPv4_Endpoint": {"IPv4_Address": "203.0.113.77"}}, first["MessageData"]["Endpoint"][1]]
    (cell / "many.json").write_text(compact(many) + "\n")
    rows = [
        ("log-1", "uci.log", "+OK", "Log", log(1)),
        ("rdma-1", "uci.rdma", "+OK", RDMA, first),
        ("wrong-global", "uci.rdma", "-ERR Invalid-Message", "RDMAInitializeSetupMT", first),
        ("wrong-topic", "uci.log", "-ERR Illegal-State", RDMA, first),
        ("broker-bad-rdma", "uci.rdma", "-ERR Invalid-Message", RDMA, rejected),
        ("decode-bad-log", "uci.log", "+OK", "Log", log(0)),
        ("decode-bad-rdma", "uci.rdma", "+OK", RDMA, decoder_bad),
        ("log-2", "uci.log", "+OK", "Log", log(2)),
        ("rdma-2", "uci.rdma", "+OK", RDMA, second),
    ]
    plan = cell / "plan.tsv"
    plan.write_text("".join(f"{label}\t{topic}\t{status}\t{compact({name: doc})}\n"
                            for label, topic, status, name, doc in rows))
    return plan


def broker_tests(args, cell, v, root):
    plan = plans(cell, v)
    services = cell / "services"
    services.mkdir()
    for i, (service, topics) in enumerate({"cpp-uci-client": ["uci.log", "uci.rdma"],
                                            "cpp-uci-wire": ["uci.log", "uci.rdma"],
                                            "cpp-log-only": ["uci.log"],
                                            "cpp-no-topics": ["other"]}.items(), 1):
        text = (f'service_id = "{service}"\nservice_uuid = "550e8400-e29b-41d4-a716-44665544090{i}"\n'
                f'allowed_topics = {json.dumps(topics)}\n')
        for topic, message in [("uci.log", "Log"), ("uci.rdma", RDMA)]:
            if topic in topics:
                text += f'\n[[topic_bindings]]\ntopic = "{topic}"\nallowed_messages = ["{message}"]\n'
        (services / f"{service}.toml").write_text(text)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    schema = f"002.{v[1]}.0"
    config = cell / "server.toml"
    config.write_text(f'bind_addr = "127.0.0.1:{port}"\nserver_id = "cpp-oms-009"\n'
                      f'system_label = "CPP-OMS-009"\nschema_version = "{schema}"\n'
                      f'schema_path = {json.dumps(str(root))}\nservices_dir = {json.dumps(str(services))}\n')
    slog = cell / "sleet.log"
    with launch([args.sleet / "target/release/sleet", config], slog,
                env=dict(os.environ, RUST_LOG="sleet=debug", NO_COLOR="1")) as sleet:
        wait_line(slog, rf"listening.*addr=127\.0\.0\.1:{port}", sleet, 60)
        url = f"ws://127.0.0.1:{port}"
        binary = args.build / "bin" / f"cpp-uci-client-uci{v}-log-rdma"

        @contextmanager
        def app(label, *options, service="cpp-uci-client", combined=True, executable=binary):
            path = cell / f"{label}.out"
            command = [executable, "--url", url, "--service-id", service, *options]
            if combined:
                command.append("--with-rdma")
            with launch(command, path) as process:
                yield process, path

        with app("typed", "--max-messages", "4") as (process, path):
            wait_line(path, r"^READY sid=sub-1 message=Log topic=uci.log sid=sub-2 message=RDMA_InitializeSetup topic=uci.rdma", process)
            with launch([args.wire, "round-trip", url, "cpp-uci-wire", schema, plan], cell / "wire.out") as wire:
                evidence = checked_exit(wire, cell / "wire.out", 0)
                assert evidence.count("byte-exact") == 6
            text = checked_exit(process, path, 0)
        assert_closed(text, "sub-2", "sub-1")
        assert text.count("CONNECTED owp=1.0") == 1 and "STOP max-messages=4 reached" in text
        assert re.findall(r"^RECEIVED \S+ #(\d+)", text, re.M) == ["1", "2", "3", "4"]
        assert re.findall(rf"^RECEIVED Log #\d+ severity=INFO classification=U schema={re.escape(schema)} "
                          r"header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:(\d\d)Z "
                          r"system=12345678-1234-1234-8234-123456789abc$", text, re.M) == ["01", "02"]
        rdma = re.findall(r"^RECEIVED RDMA_InitializeSetup .*?$", text, re.M)
        for row, last, endpoints in zip(rdma, "cd", ["ipv4=192.0.2.10 ipv6=2001:db8::1", "ipv6=fE80::aBcD ipv4=198.51.100.42"], strict=True):
            assert (f"setup=12345678-1234-1234-8234-123456789ab{last} provider=TCP classification=U schema={schema} "
                    f"header-timestamp=2026-01-02T03:04:05Z endpoints=2 {endpoints}") in row
        for name in ("Log", RDMA):
            path = "LogMT.MessageData.Data" if name == "Log" else "RDMA_InitializeSetupMT.MessageHeader.Timestamp"
            assert f"DECODE-ERROR {name} rejected by the client: codec path={path} " in text
            assert f"FAMILY message={name} callbacks=2 callbacks-after-unsubscribe=0 decode-errors=1 handler-exceptions=0" in text
        assert all(secret not in text for secret in ('not-a-dateZ', '999.0.0.1', 'do not echo', 'NATO:', 'Task093', '"000"'))
        wait_line(slog, r'service_id="cpp-uci-client"\}.*disconnected', sleet)
        app_lines = [line for line in slog.read_text().splitlines() if 'service_id="cpp-uci-client"}' in line]
        for fragment in (f"initialized schema={schema}", "subscription registered sid=sub-1 topic=uci.log message_name=Log",
                         f"subscription registered sid=sub-2 topic=uci.rdma message_name={RDMA}",
                         "subscription removed sid=sub-2", "subscription removed sid=sub-1", "disconnected"):
            assert sum(fragment in line for line in app_lines) == 1, (slog, fragment)
        with app("log-default", "--duration-seconds", "1", service="cpp-log-only", combined=False) as (process, path):
            text = checked_exit(process, path, 0)
            assert_closed(text, "sub-1")
            assert "0 Log message(s)" in text and RDMA not in text
        with app("duration", "--duration-seconds", "1") as (process, path):
            text = checked_exit(process, path, 0)
            assert_closed(text, "sub-2", "sub-1")
            assert "0 typed message(s)" in text
        for service, sids in [("cpp-no-topics", ()), ("cpp-log-only", ("sub-1",))]:
            with app(service, service=service) as (process, path):
                text = checked_exit(process, path, 4)
                assert_closed(text, *sids)
                assert "READY" not in text and "SUBSCRIBE-REJECTED by the broker:" in text
                assert f"not authorized to subscribe to topic uci.{'rdma' if sids else 'log'}" in text
        with app("wrong-release", executable=args.build / "bin" /
                 f"cpp-uci-client-uci{'26' if v == '25' else '25'}-log-rdma") as (process, path):
            text = checked_exit(process, path, 3)
            assert "owp=Unsupported-Schema" in text and "READY" not in text
            assert_closed(text)
        if args.log_build:
            with app("log-build-rejects-flag", executable=args.log_build / "bin" / f"cpp-uci-client-uci{v}") as (process, path):
                text = checked_exit(process, path, 2)
                assert "--with-rdma requires a Log + RDMA build" in text and "CONNECTED" not in text
        for sig in (signal.SIGINT, signal.SIGTERM):
            with app(sig.name) as (process, path):
                wait_line(path, r"^READY .*sid=sub-2", process)
                process.send_signal(sig)
                assert_closed(checked_exit(process, path, 128 + sig), "sub-2", "sub-1")
        with launch([args.build / f"receiver-{v}", str(port), cell / "rdma.json", cell / "many.json"],
                    cell / "receiver.out") as receiver:
            assert "CPP_OMS_009_RECEIVER_COMPLETE" in checked_exit(receiver, cell / "receiver.out", 0)


def setup_tests(args, cell, v):
    for mode, code, outcome in [("drop-second", 5, "SUBSCRIBE-FAILED"), ("setup-signal", 143, "during setup"),
                                ("drop-final", 5, "CONNECTION-LOST"), ("drop-active", 5, "CONNECTION-LOST")]:
        peer_log, app_log = cell / f"peer-{mode}.out", cell / f"app-{mode}.out"
        with launch([args.peer, mode, "15"], peer_log) as peer:
            port = wait_line(peer_log, r"^PORT (\d+)$", peer)[1]
            binary = args.build / "bin" / f"cpp-uci-client-uci{v}-log-rdma"
            with launch([binary, "--url", f"ws://127.0.0.1:{port}", "--service-id", "cpp-uci-client",
                         "--with-rdma"], app_log) as app:
                if mode == "setup-signal":
                    wait_line(peer_log, r"^SECOND SUB PENDING$", peer)
                    app.send_signal(signal.SIGTERM)
                text = checked_exit(app, app_log, code)
                assert outcome in text
                assert_closed(text, *(["sub-2", "sub-1"] if mode == "setup-signal" else []))
                if mode in ("setup-signal", "drop-second"):
                    assert "READY" not in text
                if mode == "drop-active":
                    assert "READY" in text
                if mode != "setup-signal":
                    assert "UNSUBSCRIBE-FAILED sid=sub-1" in text
                if mode in ("drop-active", "drop-final"):
                    assert "UNSUBSCRIBE-FAILED sid=sub-2" in text
            frames = checked_exit(peer, peer_log, 0)
            assert frames.count("FRAME INIT ") == 1 and "CONNECTIONS 1" in frames and "REQUESTED-SUBPROTOCOL owp" in frames
            commands = re.findall(r"^FRAME ((?:SUB|UNSUB) .*)$", frames, re.M)
            assert commands == ["SUB sub-1 Log uci.log", f"SUB sub-2 {RDMA} uci.rdma"] + (
                ["UNSUB sub-2", "UNSUB sub-1"] if mode == "setup-signal" else [])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("sleet", "build", "wire", "peer", "out", "schema25", "schema26"):
        parser.add_argument("--" + name, required=True, type=lambda path: Path(path).resolve())
    parser.add_argument("--log-build", type=lambda path: Path(path).resolve())
    args = parser.parse_args()
    assert not args.out.is_relative_to(ROOT), "use external scratch storage"
    assert sp.check_output(["git", "-C", args.sleet, "rev-parse", "HEAD", "HEAD^{tree}"], text=True).split() == [SLEET, TREE]
    assert not sp.check_output(["git", "-C", args.sleet, "status", "--porcelain"])
    for v, root in [("25", args.schema25), ("26", args.schema26)]:
        cell = args.out / v
        cell.mkdir(parents=True)
        broker_tests(args, cell, v, root)
        setup_tests(args, cell, v)
        print(f"CPP_OMS_009 UCI 2.{v[1]}: real Sleet, receiver and setup cases PASS", flush=True)


if __name__ == "__main__":
    main()