#!/usr/bin/env python3
"""Qualify the three-family client on pinned Sleet and the scripted peer."""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess as sp

from check_log_rdma import (ROOT, SLEET, TREE, FIXTURES, RDMA, assert_closed,
                           cases, checked_exit, launch, wait_line)
from task104 import cases as cases25
from task105 import cases as cases26

RISK = "SurvivabilityRiskLevel"
FAMILIES = (("uci.log", "Log"), ("uci.rdma", RDMA), ("uci.survivability", RISK))
SUBS = [f"SUB sub-{i} {name} {topic}" for i, (topic, name) in enumerate(FAMILIES, 1)]
ROSTER = ("sid=sub-1 message=Log topic=uci.log sid=sub-2 message=RDMA_InitializeSetup topic=uci.rdma "
          "sid=sub-3 message=SurvivabilityRiskLevel topic=uci.survivability")
ALL = ("--with-rdma", "--with-survivability")


def plans(cell, v):
    risk_cases = cases25 if v == "25" else cases26
    risk = risk_cases.golden(v)
    maximum = next(doc for tag, doc, _ in risk_cases.positives(v) if tag == "maximum")
    risk_uuid = "12345678-1234-1234-8234-123456789abc"
    maximum["MessageData"]["SurvivabilityRiskLevelID"]["DescriptiveLabel"] = "PRIVATE-RISK-LABEL"
    maximum["MessageData"]["AppliesToSystemID"] = [{"UUID": "12345678-1234-1234-8234-ffffffffffab"}]
    maximum["MessageData"]["SurvivabilityRiskLevelID"]["UUID"] = risk_uuid[:-1] + "d"
    maximum["MessageHeader"]["Mode"] = "EXERCISE"
    rdma = cases.golden(v)
    rdma2 = next(doc for tag, doc, _ in cases.positives(v) if tag == "reversed")
    rdma2["MessageData"]["RDMA_InitializeSetupID"]["UUID"] = risk_uuid[:-1] + "d"

    def log(n):
        doc = json.loads((FIXTURES / f"task093-real-log{v}.json").read_text())
        doc["MessageData"]["Timestamp"] = f"2026-01-02T03:05:{n:02d}Z"
        if not n:
            doc["MessageData"]["Data"] = "000"
        return doc

    def envelope(name, body):
        return json.dumps({name: body}, separators=(",", ":"))

    first = [("log-1", "uci.log", "+OK", "Log", log(1)),
             ("rdma-1", "uci.rdma", "+OK", RDMA, rdma),
             ("risk-1", "uci.survivability", "+OK", RISK, risk)]
    rejected = [("wrong-global", "uci.survivability", "-ERR Invalid-Message", RISK + "MT", risk),
                ("wrong-topic", "uci.log", "-ERR Illegal-State", RISK, risk)]
    rejected += [(label, "uci.survivability", "-ERR Invalid-Message", RISK, doc)
                 for label, doc in risk_cases.invalid_documents(v)
                 if label in {"priority-invalid", "allowed-invalid", "missing-priority", "missing-allowed",
                              "empty-risk-setting", "too-many-settings", "empty-risk-level", "too-many-levels",
                              "owner-two-arms"}]
    bad = [("decode-bad-log", "uci.log", "+OK", "Log", log(0)),
           ("decode-bad-rdma", "uci.rdma", "+OK", RDMA, cases.decoder_invalid(v)[0][1]),
           ("decode-bad-risk", "uci.survivability", "+OK", RISK, risk_cases.decoder_invalid(v)[0][1])]
    last = [("rdma-2", "uci.rdma", "+OK", RDMA, rdma2),
            ("risk-2", "uci.survivability", "+OK", RISK, maximum),
            ("log-2", "uci.log", "+OK", "Log", log(2))]
    # The wire peer requires both +OK and a byte-exact observed MSG for `bad`.
    plan = cell / "plan.tsv"
    plan.write_text("".join(f"{label}\t{topic}\t{status}\t{envelope(name, doc)}\n"
                            for label, topic, status, name, doc in first + rejected + bad + last))
    core = cell / "publications.txt"
    core.write_text("".join(f"PUB {topic} {envelope(name, doc)}\n" for _, topic, _, name, doc in first))
    for n in (1, 2):
        (cell / f"compat-{n}.tsv").write_text("".join(
            f"{label}\t{topic}\t{status}\t{envelope(name, doc)}\n" for label, topic, status, name, doc in first[:n]))
    return plan, core, len(rejected), len(envelope(RISK, bad[2][4]))


def executable(build, v, suffix="-log-rdma-survivability"):
    return build / "bin" / f"cpp-uci-client-uci{v}{suffix}"


@contextmanager
def broker(args, cell, v, root):
    schema = f"002.{v[1]}.0"
    services = cell / "services"
    services.mkdir()
    for i, (service, count) in enumerate({"cpp-uci-client": 3, "cpp-uci-wire": 3, "cpp-uci-three": 3,
                                         "cpp-deny-first": 0, "cpp-deny-second": 1, "cpp-deny-third": 2}.items(), 1):
        allowed = [topic for topic, _ in FAMILIES[:count]] or ["other"]
        text = (f'service_id = "{service}"\nservice_uuid = "550e8400-e29b-41d4-a716-44665544100{i}"\n'
                f'allowed_topics = {json.dumps(allowed)}\n')
        for topic, name in FAMILIES[:count]:
            text += f'\n[[topic_bindings]]\ntopic = "{topic}"\nallowed_messages = ["{name}"]\n'
        (services / f"{service}.toml").write_text(text)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    config = cell / "server.toml"
    config.write_text(f'bind_addr = "127.0.0.1:{port}"\nserver_id = "cpp-oms-010"\nsystem_label = "CPP-OMS-010"\n'
                      f'schema_version = "{schema}"\nschema_path = {json.dumps(str(root))}\n'
                      f'services_dir = {json.dumps(str(services))}\n')
    slog = cell / "sleet.log"
    with launch([args.sleet / "target/release/sleet", config], slog,
                env=dict(os.environ, RUST_LOG="sleet=debug", NO_COLOR="1")) as sleet:
        wait_line(slog, rf"listening.*addr=127\.0\.0\.1:{port}", sleet, 60)
        yield sleet, slog, port


def broker_tests(args, cell, v, root):
    plan, core, rejected_count, invalid_bytes = plans(cell, v)
    schema = f"002.{v[1]}.0"
    with broker(args, cell, v, root) as (sleet, slog, port):
        url = f"ws://127.0.0.1:{port}"

        @contextmanager
        def app(label, *options, service="cpp-uci-client", binary=executable(args.build, v)):
            path = cell / f"{label}.out"
            with launch([binary, "--url", url, "--service-id", service, *options], path) as process:
                yield process, path

        with app("typed", *ALL, "--max-messages", "6") as (process, path):
            wait_line(path, r"^READY " + ROSTER + r" \(all three SUBs acknowledged by the broker\)$", process)
            wire_log = cell / "wire.out"
            with launch([args.wire, "round-trip-three", url, "cpp-uci-wire", schema, plan], wire_log) as wire:
                observed = checked_exit(wire, wire_log, 0)
            text = checked_exit(process, path, 0)
        assert observed.count("byte-exact") == 9 and observed.count("NO MSG ") == rejected_count
        assert "PUB decode-bad-risk -> +OK\nOBSERVED decode-bad-risk sid=wire-risk byte-exact" in observed
        assert_closed(text, "sub-3", "sub-2", "sub-1")
        assert text.count("CONNECTED owp=1.0") == 1 and "STOP max-messages=6 reached" in text
        assert re.findall(r"^RECEIVED \S+ #(\d+)", text, re.M) == list("123456")
        assert re.findall(rf"^RECEIVED Log #\d+ severity=INFO classification=U schema={re.escape(schema)} "
                          r"header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:(\d\d)Z ",
                          text, re.M) == ["01", "02"]
        rdma = re.findall(rf"^RECEIVED {RDMA} #\d+ (.*)$", text, re.M)
        for row, ending, addresses in zip(rdma, "cd", ["ipv4=192.0.2.10 ipv6=2001:db8::1",
                                                        "ipv6=2001:db8::42 ipv4=198.51.100.42"], strict=True):
            assert row == (f"setup=12345678-1234-1234-8234-123456789ab{ending} provider=TCP classification=U schema={schema} "
                           f"header-timestamp=2026-01-02T03:04:05Z endpoints=2 {addresses}"), row
        summaries = re.findall(rf"^RECEIVED {RISK} #\d+ (.*)$", text, re.M)
        prefix = "risk-level=12345678-1234-1234-8234-123456789ab"
        header = f" classification=U schema={schema} header-timestamp=2026-01-02T03:04:05Z"
        levels = ":ACQUISITION,TRACK,LAUNCH,INTERCEPT,+1"
        assert summaries == [prefix + "c mode=LIVE" + header + " risk-settings=1 HIGH:NONE",
                             prefix + "d mode=EXERCISE" + header + " risk-settings=4 " +
                             " ".join(priority + levels for priority in ("HIGH", "MEDIUM", "LOW")) + " settings-omitted=1"]
        assert f"DECODE-ERROR {RISK} category=codec path={RISK}MT.MessageHeader.Timestamp payload-bytes={invalid_bytes}" in text
        assert "DECODE-ERROR Log rejected by the client: codec path=LogMT.MessageData.Data" in text
        assert f"DECODE-ERROR {RDMA} rejected by the client: codec path={RDMA}MT.MessageHeader.Timestamp" in text
        for _, name in FAMILIES:
            assert f"FAMILY message={name} callbacks=2 callbacks-after-unsubscribe=0 decode-errors=1 handler-exceptions=0" in text
        assert "delivered=6 decode-errors=3 handler-exceptions=0" in text
        for secret in ("not-a-dateZ", "PRIVATE-RISK-LABEL", "ffffffffffab", "NATO:", "PT1S", '"000"'):
            assert secret not in text, secret
        wait_line(slog, r'service_id="cpp-uci-client"\}.*disconnected', sleet)
        trace = [line for line in slog.read_text().splitlines() if 'service_id="cpp-uci-client"}' in line]
        expected = [f"initialized schema={schema}", 'send operation="INFO"'] + [
            f"subscription registered sid=sub-{i} topic={topic} message_name={name}"
            for i, (topic, name) in enumerate(FAMILIES, 1)] + [f"subscription removed sid=sub-{i}" for i in (3, 2, 1)] + ["disconnected"]
        indices = []
        for fragment in expected:
            found = [i for i, line in enumerate(trace) if fragment in line]
            assert len(found) == 1, (slog, fragment, found)
            indices += found
        assert indices == sorted(indices)
        assert len({re.search(r"connection\{id=(\d+)", row)[1] for row in trace}) == 1

        for i, position in enumerate(("first", "second", "third")):
            with app(f"deny-{position}", *ALL, service=f"cpp-deny-{position}") as (process, path):
                text = checked_exit(process, path, 4)
                assert "READY" not in text and "SUBSCRIBE-REJECTED by the broker" in text
                assert f"not authorized to subscribe to topic {FAMILIES[i][0]}" in text
                assert_closed(text, *(f"sub-{n}" for n in range(i, 0, -1)))
        for sig in (signal.SIGINT, signal.SIGTERM):
            with app(sig.name, *ALL) as (process, path):
                wait_line(path, r"^READY " + ROSTER, process)
                process.send_signal(sig)
                assert_closed(checked_exit(process, path, 128 + sig), "sub-3", "sub-2", "sub-1")
        with app("wrong-release", *ALL, binary=executable(args.build, "26" if v == "25" else "25")) as (process, path):
            text = checked_exit(process, path, 3)
            assert "owp=Unsupported-Schema" in text and "READY" not in text
            assert_closed(text)

        for n, suffix, flags in [(1, "", ()), (2, "-log-rdma", ("--with-rdma",))]:
            # Identical server, mode, duration and service for the old and new profiles.
            baseline = executable(args.baseline or args.build, v, suffix)

            def duration(label, binary):
                with app(label, *flags, "--duration-seconds", "1", binary=binary) as (process, path):
                    out = checked_exit(process, path, 0)
                    assert_closed(out, *(f"sub-{i}" for i in range(n, 0, -1)))
                    return out
            assert duration(f"c-mode-{n}", executable(args.build, v)) == duration(f"ab-mode-{n}", baseline)

            def typed(label, binary):
                wire_log = cell / f"wire-{label}.out"
                with app(label, *flags, "--max-messages", str(n), binary=binary) as (process, path):
                    wait_line(path, r"^READY ", process)
                    with launch([args.wire, "round-trip", url, "cpp-uci-wire", schema, cell / f"compat-{n}.tsv"], wire_log) as wire:
                        assert checked_exit(wire, wire_log, 0).count("byte-exact") == n
                    out = checked_exit(process, path, 0)
                    assert_closed(out, *(f"sub-{i}" for i in range(n, 0, -1)))
                    # Exclude display sequence numbers: cross-SID order is not a guarantee.
                    summaries = re.findall(r"^RECEIVED (\S+) #\d+ (.*)$", out, re.M)
                    assert len(summaries) == n
                    return sorted(summaries)
            assert typed(f"c-typed-{n}", executable(args.build, v)) == typed(f"ab-typed-{n}", baseline)
        for label, suffix, flags in [("a", "", ALL), ("b", "-log-rdma", ALL),
                                      ("c", "-log-rdma-survivability", ("--with-survivability",))]:
            with app(f"bad-cli-{label}", *flags, binary=executable(args.build, v, suffix)) as (process, path):
                text = checked_exit(process, path, 2)
                assert text.startswith("ERROR ") and "CONNECTED" not in text
        receiver_log = cell / "core.out"
        with launch([args.build / f"survivability-{v}", str(port), core], receiver_log) as receiver:
            assert "CPP_OMS_010_RECEIVER_COMPLETE" in checked_exit(receiver, receiver_log, 0)


def setup_tests(args, cell, v):
    modes = [(f"three-drop-{i}", i, 5) for i in (1, 2, 3)] + [
        ("three-between", 2, 143), ("three-pending", 3, 143), ("three-final", 3, 5), ("three-active", 3, 5)]
    for mode, count, code in modes:
        peer_log, app_log = cell / f"peer-{mode}.out", cell / f"app-{mode}.out"
        with launch([args.peer, mode, "12"], peer_log) as peer:
            port = wait_line(peer_log, r"^PORT (\d+)$", peer)[1]
            with launch([executable(args.build, v), "--url", f"ws://127.0.0.1:{port}", "--service-id", "cpp-uci-client",
                         *ALL], app_log) as app:
                if code == 143:
                    wait_line(peer_log, r"^SUB PENDING$", peer)
                    app.send_signal(signal.SIGTERM)
                text = checked_exit(app, app_log, code)
            frames = checked_exit(peer, peer_log, 0)
        assert frames.count("FRAME INIT ") == 1 and "CONNECTIONS 1" in frames and "REQUESTED-SUBPROTOCOL owp" in frames
        commands = re.findall(r"^FRAME ((?:SUB|UNSUB) .*)$", frames, re.M)
        if code == 143:
            sids = [f"sub-{i}" for i in range(count, 0, -1)]
            assert commands == SUBS[:count] + [f"UNSUB {sid}" for sid in sids]
            assert_closed(text, *sids)
            assert "READY" not in text and ("before Survivability subscribe" if count == 2 else "during setup") in text
        else:
            assert_closed(text)
            assert commands[:count] == SUBS[:count] and len(commands) == count
            if "drop-" in mode:
                assert "READY" not in text and "SUBSCRIBE-FAILED" in text
                established = count - 1
            else:
                assert "CONNECTION-LOST" in text
                if mode == "three-active":
                    assert "READY " + ROSTER in text
                established = count
            assert re.findall(r"^UNSUBSCRIBE-FAILED sid=(\S+)", text, re.M) == [
                f"sub-{i}" for i in range(established, 0, -1)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("sleet", "build", "wire", "peer", "out", "schema25", "schema26", "baseline"):
        parser.add_argument("--" + name, required=name != "baseline", type=lambda path: Path(path).resolve())
    args = parser.parse_args()
    assert not args.out.is_relative_to(ROOT), "use external scratch storage"
    assert sp.check_output(["git", "-C", args.sleet, "rev-parse", "HEAD", "HEAD^{tree}"], text=True).split() == [SLEET, TREE]
    assert not sp.check_output(["git", "-C", args.sleet, "status", "--porcelain"])
    for v, root in [("25", args.schema25), ("26", args.schema26)]:
        cell = args.out / v
        cell.mkdir(parents=True)
        broker_tests(args, cell, v, root)
        setup_tests(args, cell, v)
        print(f"CPP_OMS_010 UCI 2.{v[1]}: pinned Sleet, receiver and setup cases PASS", flush=True)


if __name__ == "__main__":
    main()