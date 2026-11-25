#!/usr/bin/env python3
"""Exercise both releases of the standalone client over verified WSS and WS."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import select
import signal
import socket
import ssl
import subprocess as sp
import sys
import warnings

sys.dont_write_bytecode = True
from check_survivability import (ALL, FAMILIES, ROOT, SLEET, SUBS, TREE, assert_closed,
                                 broker, checked_exit, executable, launch, plans, wait_line)

sys.path.insert(0, str(ROOT / "tests/cpp-lacal"))
from verified_wss import (HANDSHAKE, VERIFY, Forwarder, RecordRequest, certificates, context,
                          frame, frames, handshake, listener, receive)

PROFILE = ("", "-log-rdma", "-log-rdma-survivability")
FLAGS = ((), ALL[:1], ALL)
SERVICE = "cpp-uci-client"
TRUST_ENV = {k: v for k, v in os.environ.items() if k not in ("SSL_CERT_DIR", "SSL_CERT_FILE")}


def application(args, v, suffix):
    if args.bundles:
        return executable(args.bundles / f"uci{v}{suffix}", v, suffix)
    return executable(args.build if suffix.endswith("-tls") else args.plain, v, suffix)


def init(schema, service=SERVICE):
    return dict(service_id=service, schema=schema, verbose=True, versions=["1.0"])


def text_frame(sock):
    opcode, payload = receive(sock)
    assert opcode == 1
    return payload.decode()


def send(sock, text):
    sock.sendall(frame(1, text.encode()))


def uns(n):
    return [f"sub-{i}" for i in range(n, 0, -1)]


def command(binary, url, ca=None, *, service=SERVICE, options=()):
    return [binary, "--url", url, "--service-id", service, *options,
            *([] if ca is None else ["--ca-file", str(ca)])]


def reject(text, kind):
    assert not re.search(r"^(CONNECTED|READY|RECEIVED|UNSUBSCRIBED)\b", text, re.M), text
    if kind:
        assert f"CONNECT-FAILED kind={kind} " in text
        assert "CLOSED close=not-connected callbacks=0 " in text
        assert_closed(text)
    else:
        assert text.startswith("ERROR ") and "CPP-UCI-CLIENT" not in text
    assert "-----BEGIN CERTIFICATE-----" not in text


def cli_controls(args, cell, v, pki):
    tls = application(args, v, PROFILE[2] + "-tls")
    with listener() as server:
        host = f"localhost:{server.getsockname()[1]}"
        ws, wss = "ws://" + host, "wss://" + host
        ca = pki / "ca.pem"
        cases = [
            ("plain-ca", tls, ws, ca, (), None),
            ("duplicate-ca", tls, wss, ca, ("--ca-file", "other.pem"), None),
            ("missing-ca", tls, wss, None, ("--ca-file",), None),
            ("empty-ca", tls, wss, "", (), None),
            ("no-bypass", tls, wss, None, ("--insecure",), None),
            ("bad-mode", tls, wss, None, ALL[1:], None),
            ("bad-rdma", application(args, v, "-tls"), wss, ca, ALL[:1], None),
            ("bad-survivability", application(args, v, PROFILE[1] + "-tls"), wss, ca, ALL, None),
        ]
        cases += [(name, tls, url, None, (), kind) for name, url, kind in (
            ("userinfo", "wss://user@" + host, None), ("fragment", wss + "/#frag", None),
            ("scheme", "https://" + host, None), ("no-host", "wss:///owp", None),
            ("bad-port", "wss://localhost:0", None), ("bad-path", wss + "/%xyz", None),
            ("bad-ipv6", "wss://[localhost]:" + host.split(":")[1], "invalid_argument"),
            ("bad-dns", "wss://.localhost:" + host.split(":")[1], "invalid_argument"),
        )]
        cases += [(f"invalid-ca-{name}", tls, wss, pki / name, (), "invalid_argument")
                  for name in ("missing.pem", "invalid-pem.pem", "malformed.pem", "good.pem", "empty.pem")]
        if args.plain or args.bundles:
            for i, suffix in enumerate(PROFILE):
                binary = application(args, v, suffix)
                cases.extend([(f"no-wss-{i}", binary, wss, None, (), None),
                              (f"no-ca-{i}", binary, ws, ca, (), None)])
        for name, binary, url, bundle, flags, kind in cases:
            path = cell / f"cli-{name}.out"
            with launch(command(binary, url, bundle, options=flags), path, env=TRUST_ENV, cwd=cell) as child:
                reject(checked_exit(child, path, 2), kind)
            assert not select.select([server], [], [], 0)[0], f"unexpected connection: {name}"


def peer_case(binary, cell, pki, schema, name, *, scenario=None, n=3, docs=(), cert="good",
              host="localhost", ca="ca.pem", trust=TRUST_ENV, kind=None, deny=0, sig=None,
              expected=0, version=None, client_command=command):
    """An independent TLS/WS peer that checks frames and never encodes UCI payloads."""
    scenario = scenario or name
    path = cell / f"peer-{name}.out"
    with listener() as server, ThreadPoolExecutor(max_workers=1) as worker:
        port = server.getsockname()[1]
        sni, commands, evidence = [], [], {}
        with warnings.catch_warnings():
            # Python deprecates configuring the deliberately obsolete peer.
            warnings.simplefilter("ignore", DeprecationWarning)
            ctx = context(pki, cert, sni, version)
        if scenario == "obsolete":
            ctx.set_ciphers("DEFAULT:@SECLEVEL=0")

        def serve(child):
            with server.accept()[0] as raw:
                raw.settimeout(8)
                if scenario in ("interrupted", "tls-timeout"):
                    # The peer never replies to ClientHello, so no application
                    # keys can exist. Require a TLS handshake record, not HTTP.
                    hello = raw.recv(65536)
                    assert hello[:1] == b"\x16" and b"GET " not in hello and b"INIT " not in hello
                    if scenario == "tls-timeout":
                        assert raw.recv(65536) == b""
                    evidence["application_bytes"] = 0
                    return
                try:
                    tls = ctx.wrap_socket(raw, server_side=True, suppress_ragged_eofs=False)
                except ssl.SSLError as exc:
                    assert (kind == VERIFY and "ALERT" in exc.reason or
                            scenario == "obsolete" and exc.reason == "UNSUPPORTED_PROTOCOL"), exc.reason
                    evidence.update(alert=exc.reason, application_bytes=0)
                    return
                with tls:
                    assert kind not in (VERIFY, HANDSHAKE, "timeout"), "unexpected successful TLS handshake"
                    evidence.update(tls=tls.version(), cipher=tls.cipher()[0])
                    request = RecordRequest(tls)
                    handshake(request, scenario)
                    assert request.data.startswith(b"GET /owp?control=%2B HTTP/1.1\r\n")
                    assert f"Host: {host}:{port}\r\n".encode() in request.data
                    if kind == "handshake_rejected":
                        try:
                            assert tls.recv(1) == b""
                        except ssl.SSLEOFError:
                            pass
                        evidence["websocket_application_bytes"] = 0
                        return
                    first = text_frame(tls)
                    assert first.startswith("INIT ") and json.loads(first[5:]) == init(schema)
                    commands.append("INIT")
                    send(tls, "+OK")
                    send(tls, 'INFO {"version":"1.0","server_id":"tls-control"}')
                    retained = n
                    for i in range(n):
                        sub = text_frame(tls)
                        assert sub == SUBS[i]
                        commands.append(sub)
                        if deny == i + 1:
                            send(tls, "-ERR Illegal-State qualification denial")
                            retained = i
                            break
                        if i == 2 and scenario == "drop-third":
                            return
                        send(tls, "+OK")
                        if i == 2 and scenario == "drop-final":
                            return
                    if not deny:
                        wait_line(path, r"^READY ", child)
                        if scenario == "drop-ready":
                            return
                        if sig:
                            child.send_signal(sig)
                        for _, topic, _, payload in docs:
                            sid = f"sub-{[t for t, _ in FAMILIES].index(topic) + 1}"
                            send(tls, f"MSG {sid} {payload}")
                    for sid in uns(retained):
                        msg = text_frame(tls)
                        assert msg == "UNSUB " + sid
                        commands.append(msg)
                        send(tls, "+OK")
                    assert receive(tls) == (8, b"\x03\xe8")
                    evidence["client_ws_close"] = True
                    if scenario == "no-ws-close":
                        return
                    tls.sendall(frame(8, b"\x03\xe8"))
                    if scenario == "no-close-notify":
                        return
                    with tls.unwrap():
                        evidence["close_notify"] = True

        url = f"wss://{host}:{port}/owp?control=%2B"
        options = (*FLAGS[n - 1], "--max-messages", str(len(docs) or 10), "--duration-seconds", "10")
        with launch(client_command(binary, url, None if ca is None else pki / ca, options=options),
                    path, env=trust, cwd=cell) as child:
            future = worker.submit(serve, child)
            text = checked_exit(child, path, expected)
            future.result(timeout=10)
        assert not select.select([server], [], [], 0)[0], "unexpected second connection"
        if "tls" in evidence or kind == VERIFY:
            assert sni == (["localhost"] if host == "localhost" else [None])
    if kind:
        reject(text, kind)
        assert not commands
    elif scenario.startswith("drop-"):
        assert_closed(text)
        assert commands == ["INIT", *SUBS]
        retained = 2 if scenario == "drop-third" else 3
        assert re.findall(r"^UNSUBSCRIBE-FAILED sid=(\S+)", text, re.M) == uns(retained)
        if scenario == "drop-third":
            assert "READY" not in text and "SUBSCRIBE-FAILED" in text
        else:
            assert "CONNECTION-LOST" in text
    else:
        assert_closed(text, *uns(deny - 1 if deny else n))
        if scenario in ("no-ws-close", "no-close-notify"):
            assert "CLOSED close=failed kind=" in text and "close_notify" not in evidence
        else:
            assert "CLOSED close=ok" in text and evidence["close_notify"]
        if deny:
            assert "READY" not in text and "SUBSCRIBE-REJECTED" in text
    (cell / f"peer-{name}.json").write_text(json.dumps(dict(exit=expected, sni=sni, commands=commands, **evidence)) + "\n")
    return text


def operator_smoke(args, cell, v, pki, plan):
    def smoke_command(binary, url, ca, *, options):
        bounds = options[len(ALL):]
        return [sys.executable, ROOT / "examples/cpp-uci-client/deploy/smoke.py",
                "--release", f"2.{v[1]}", "--profile", "C", "--variant", "tls", binary.parents[1],
                "--test-url", url, "--service-id", SERVICE, "--ca-file", ca, *bounds]

    schema = f"002.{v[1]}.0"
    binary = application(args, v, PROFILE[2] + "-tls")
    docs = [row for row in documents(plan, 3) if not row[0].startswith("decode-bad-")]
    for scenario, expected, host in (("clean", 0, "127.0.0.1"), ("no-ws-close", 6, "localhost")):
        text = peer_case(binary, cell, pki, schema, f"operator-smoke-{scenario}", scenario=scenario,
                         docs=docs, host=host, expected=expected, client_command=smoke_command)
        check_summaries(text, schema, 3, decoded_bad=False)
        assert ("SMOKE OK:" if expected == 0 else "SMOKE FAILED: client status 6") in text


def documents(plan, n, *, broker_checks=False):
    return [row for line in plan.read_text().splitlines() if
            (row := line.split("\t", 3))[1] in {topic for topic, _ in FAMILIES[:n]}
            and (row[2] == "+OK" or broker_checks)
            and (n == 3 or not row[0].startswith("decode-bad-"))]


def summaries(text):
    return [(family, fields) for family, fields in re.findall(r"^RECEIVED (\S+) #\d+ (.*)$", text, re.M)]


def check_summaries(text, schema, n, *, decoded_bad):
    assert re.findall(r"^RECEIVED \S+ #(\d+)", text, re.M) == [str(i) for i in range(1, n * 2 + 1)]
    assert text.count("CONNECTED owp=1.0") == 1 and text.count("READY sid=") == 1
    assert f"STOP max-messages={n * 2} reached" in text
    assert f"delivered={n * 2} decode-errors={3 if decoded_bad else 0} handler-exceptions=0" in text
    assert "queue-rejected-full=0 queue-rejected-closed=0" in text
    rows = summaries(text)
    log = f"severity=INFO classification=U schema={schema} header-timestamp=2026-01-02T03:04:05Z "
    assert [s for f, s in rows if f == "Log"] == [
        log + f"log-timestamp=2026-01-02T03:05:0{i}Z system=12345678-1234-1234-8234-123456789abc"
        for i in (1, 2)]
    if n >= 2:
        rdma = f" provider=TCP classification=U schema={schema} header-timestamp=2026-01-02T03:04:05Z endpoints=2 "
        assert [s for f, s in rows if f == FAMILIES[1][1]] == [
            f"setup=12345678-1234-1234-8234-123456789ab{end}" + rdma + addresses
            for end, addresses in (("c", "ipv4=192.0.2.10 ipv6=2001:db8::1"),
                                   ("d", "ipv6=2001:db8::42 ipv4=198.51.100.42"))]
    if n == 3:
        header = f" classification=U schema={schema} header-timestamp=2026-01-02T03:04:05Z"
        levels = ":ACQUISITION,TRACK,LAUNCH,INTERCEPT,+1"
        assert [s for f, s in rows if f == FAMILIES[2][1]] == [
            "risk-level=12345678-1234-1234-8234-123456789abc mode=LIVE" + header + " risk-settings=1 HIGH:NONE",
            "risk-level=12345678-1234-1234-8234-123456789abd mode=EXERCISE" + header + " risk-settings=4 " +
            " ".join(priority + levels for priority in ("HIGH", "MEDIUM", "LOW")) + " settings-omitted=1"]
    if n >= 2:
        for _, family in FAMILIES[:n]:
            assert (f"FAMILY message={family} callbacks=2 callbacks-after-unsubscribe=0 "
                    f"decode-errors={int(decoded_bad)} handler-exceptions=0") in text
    if decoded_bad:
        assert "DECODE-ERROR SurvivabilityRiskLevel category=codec path=SurvivabilityRiskLevelMT.MessageHeader.Timestamp" in text
    assert not any(secret in text for secret in ("not-a-dateZ", "PRIVATE-RISK-LABEL", "ffffffffffab", "NATO:", '"000"'))


def offline(args, cell, v, pki, plan):
    schema = f"002.{v[1]}.0"
    cli_controls(args, cell, v, pki)
    full = application(args, v, PROFILE[2] + "-tls")
    for n, profile in enumerate(PROFILE, 1):
        cases = [application(args, v, profile + "-tls")]
        if n < 3:
            cases.append(full)
        for i, binary in enumerate(cases):
            docs = [row for row in documents(plan, n) if not row[0].startswith("decode-bad-")]
            text = peer_case(binary, cell, pki, schema, f"verified-{n}-{i}", n=n, docs=docs,
                             version=ssl.TLSVersion.TLSv1_2 if n == 1 else ssl.TLSVersion.TLSv1_3)
            check_summaries(text, schema, n, decoded_bad=False)
    for case, cert, host in [("untrusted", "other-leaf", "localhost"), ("wrong-dns", "wrong-dns", "localhost"),
                             ("wrong-ip", "wrong-ip", "127.0.0.1"), ("expired", "expired", "localhost"),
                             ("future", "future", "localhost"), ("cn-only", "cn-only", "localhost"),
                             ("wrong-purpose", "client-only", "localhost")]:
        peer_case(full, cell, pki, schema, case, cert=cert, host=host, kind=VERIFY, expected=3)
    peer_case(full, cell, pki, schema, "obsolete", kind=HANDSHAKE, expected=3, version=ssl.TLSVersion.TLSv1_1)
    for case, kind in [("interrupted", HANDSHAKE), ("tls-timeout", "timeout"),
                       ("invalid-upgrade", "handshake_rejected"), ("missing-protocol", "handshake_rejected"),
                       ("wrong-protocol", "handshake_rejected")]:
        peer_case(full, cell, pki, schema, case, kind=kind, expected=3)
    trust = dict(TRUST_ENV, SSL_CERT_FILE=str(pki / "ca.pem"))
    for case, ca in (("default-trust", None), ("additive-trust", "other.pem")):
        peer_case(full, cell, pki, schema, case, ca=ca, trust=trust, sig=signal.SIGTERM, expected=143)
    for position in (1, 2, 3):
        peer_case(full, cell, pki, schema, f"deny-{position}", deny=position, expected=4)
    peer_case(full, cell, pki, schema, "deny-close-failed", scenario="no-ws-close", deny=3, expected=4)
    for sig in (signal.SIGINT, signal.SIGTERM):
        peer_case(full, cell, pki, schema, sig.name, sig=sig, expected=128 + sig)
    for case in ("drop-third", "drop-final", "drop-ready"):
        peer_case(full, cell, pki, schema, case, expected=5)
    for case in ("no-ws-close", "no-close-notify"):
        peer_case(full, cell, pki, schema, case, n=1, docs=documents(plan, 1)[:1], expected=6)


@contextmanager
def relay(pki, broker_port):
    """Task109's existing TLS forwarder; only copy and passively record bytes."""
    with listener() as server, ThreadPoolExecutor(max_workers=1) as worker:
        port = server.getsockname()[1]
        sni, info = [], {}
        ctx = context(pki, "good", sni)
        streams = (bytearray(), bytearray())

        def forward():
            with server.accept()[0] as raw:
                raw.settimeout(8)
                peer = Forwarder.__new__(Forwarder)
                peer.batches = 0
                Forwarder.__init__(peer, raw, ctx)
                info.update(tls=peer.tls.version(), sni=sni)
                with socket.create_connection(("127.0.0.1", broker_port), timeout=8) as plain:
                    while True:
                        # As in CPP-OMS-011, drain the BIO before poll: pending()
                        # alone doesn't account for buffered ciphertext.
                        while True:
                            try:
                                data = peer.tls.read(65536)
                            except ssl.SSLWantReadError:
                                peer.flush()
                                break
                            assert data, "TLS ended before broker EOF"
                            plain.sendall(data)
                            streams[0].extend(data)
                        ready, _, _ = select.select([raw, plain], [], [], 12)
                        assert ready, "relay idle timeout"
                        if raw in ready:
                            peer.feed()
                        if plain in ready:
                            data = plain.recv(65536)
                            if not data:
                                # Authenticated EOF only; no synthetic WS reply.
                                try:
                                    peer.tls.unwrap()
                                except ssl.SSLWantReadError:
                                    pass
                                peer.flush()
                                info["broker_eof"] = True
                                return
                            peer.sendall(data)
                            streams[1].extend(data)

        future = worker.submit(forward)
        yield port, info, streams
        future.result(timeout=15)
        assert not select.select([server], [], [], 0)[0], "unexpected second TLS connection"
        assert info["tls"] in ("TLSv1.2", "TLSv1.3") and sni == ["localhost"] and info["broker_eof"]


def wire_evidence(info, streams, port, schema, n, docs, *, wrong=False):
    client, server = map(frames, streams)
    assert f"Host: localhost:{port}" in client["upgrade"]
    for side in (client, server):
        assert [line.split(":", 1)[1].strip() for line in side["upgrade"].splitlines()
                if line.lower().startswith("sec-websocket-protocol:")] == ["owp"]
    outgoing, incoming = ([f["text"] for f in side["frames"] if f["opcode"] == 1] for side in (client, server))
    assert outgoing[0].startswith("INIT ") and json.loads(outgoing[0][5:]) == init(schema)
    if wrong:
        assert len(outgoing) == len(incoming) == 1 and incoming[0].startswith("-ERR Unsupported-Schema ")
    else:
        assert outgoing[1:] == SUBS[:n] + ["UNSUB " + sid for sid in uns(n)]
        assert incoming[0] == "+OK" and incoming[1].startswith("INFO ")
        assert json.loads(incoming[1][5:])["version"] == "1.0"
        assert incoming[2:2+n] == incoming[-n:] == ["+OK"] * n
        assert incoming[2+n:-n] == [f"MSG sub-{[t for t, _ in FAMILIES].index(topic) + 1} {payload}"
                                    for _, topic, status, payload in docs if status == "+OK"]
        assert sum(f["opcode"] == 8 for f in client["frames"]) == 1
    assert all(f["opcode"] != 8 for f in server["frames"])
    # No raw rejected payloads or certificate material in the test report.
    return dict(info, init=init(schema), sub_commands=outgoing[1:], msg_count=sum(x.startswith("MSG ") for x in incoming),
                close_from_client=not wrong, close_from_broker=False)


def broker_cases(args, cell, v, root, pki, plan, core):
    schema = f"002.{v[1]}.0"
    with broker(args, cell, v, root) as (_process, _log, broker_port):
        ws = f"ws://127.0.0.1:{broker_port}"

        def round_trip(binary, url, ca, label, n, rows, expected):
            app_log = cell / f"{label}.out"
            wire_log = cell / f"{label}-observer.out"
            publication = cell / f"{label}.tsv"
            publication.write_text("".join("\t".join(row) + "\n" for row in rows))
            options = (*FLAGS[n - 1], "--max-messages", str(n * 2))
            with launch(command(binary, url, ca, options=options), app_log, env=TRUST_ENV, cwd=cell) as child:
                wait_line(app_log, r"^READY ", child)
                with launch([args.wire, "round-trip-three", ws, "cpp-uci-wire", schema, publication], wire_log) as wire:
                    observed = checked_exit(wire, wire_log, 0)
                text = checked_exit(child, app_log, expected)
            assert observed.count("byte-exact") == sum(r[2] == "+OK" for r in rows)
            assert observed.count("NO MSG ") == sum(r[2] != "+OK" for r in rows)
            assert_closed(text, *uns(n))
            assert ("CLOSED close=ok" if expected == 0 else "CLOSED close=failed kind=") in text
            check_summaries(text, schema, n, decoded_bad=n == 3)
            return sorted(summaries(text))  # stable within-family order checked separately

        full = application(args, v, PROFILE[2] + "-tls")
        for n, suffix in enumerate(PROFILE, 1):
            binary = application(args, v, suffix + "-tls")
            rows = documents(plan, n, broker_checks=n == 3)
            plain = round_trip(binary, ws, None, f"tls-bin-ws-{n}", n, rows, 0)
            if args.plain or args.bundles:
                assert plain == round_trip(application(args, v, suffix), ws, None, f"plain-ws-{n}", n, rows, 0)
            if n < 3:
                assert plain == round_trip(full, ws, None, f"profile-c-ws-{n}", n, rows, 0)
            with relay(pki, broker_port) as (port, info, streams):
                assert plain == round_trip(binary, f"wss://localhost:{port}/owp", pki / "ca.pem", f"sleet-wss-{n}", n, rows, 6)
            evidence = wire_evidence(info, streams, port, schema, n, rows)
            (cell / f"sleet-wss-{n}.json").write_text(json.dumps(dict(evidence, exit=6), indent=2) + "\n")

        other = "26" if v == "25" else "25"
        with relay(pki, broker_port) as (port, info, streams):
            path = cell / "wrong-release.out"
            binary = application(args, other, PROFILE[2] + "-tls")
            with launch(command(binary, f"wss://localhost:{port}/owp", pki / "ca.pem", options=ALL), path, env=TRUST_ENV, cwd=cell) as child:
                text = checked_exit(child, path, 3)
            reject(text, "broker_rejected")
            assert "owp=Unsupported-Schema" in text
        wire_evidence(info, streams, port, f"002.{other[1]}.0", 0, [], wrong=True)

        for case in ("three_families_share_queue_and_pending_discard", "callback_exception_and_per_sid_quiescence"):
            with relay(pki, broker_port) as (port, _info, _streams):
                path = cell / f"core-{case}.out"
                with launch([args.build / f"survivability-{v}-tls", str(broker_port), core,
                             f"wss://localhost:{port}/owp", pki / "ca.pem", case], path, env=TRUST_ENV) as child:
                    assert "CPP_OMS_012_WSS_RECEIVER_COMPLETE" in checked_exit(child, path, 0)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("build", "out", "plain", "sleet", "wire", "schema25", "schema26", "bundles"):
        parser.add_argument("--" + name, type=lambda p: Path(p).resolve(), required=name == "out")
    args = parser.parse_args()
    if not __debug__:
        parser.error("qualification assertions must be enabled")
    if args.out.is_relative_to(ROOT):
        parser.error("use external scratch storage")
    if (not args.build and not args.bundles) or (args.bundles and args.plain):
        parser.error("select --build [--plain], or --bundles (plus --build for broker receiver-core tests)")
    if args.bundles and args.bundles.is_relative_to(ROOT):
        parser.error("stage bundles outside the checkout")
    if any((args.sleet, args.wire, args.schema25, args.schema26)):
        if not all((args.build, args.sleet, args.wire, args.schema25, args.schema26)):
            parser.error("real broker qualification requires --build --sleet --wire --schema25 --schema26 together")
        assert sp.check_output(["git", "-C", args.sleet, "rev-parse", "HEAD", "HEAD^{tree}"], text=True).split() == [SLEET, TREE]
        assert not sp.check_output(["git", "-C", args.sleet, "status", "--porcelain"])
    else:
        print("OFFLINE: real Sleet broker not exercised", flush=True)
    args.out.mkdir(parents=True, mode=0o700)
    pki = args.out / "pki"
    certificates(pki, full=True)
    if args.bundles:
        # Isolate both default trust paths; positive cases use explicit --ca-file.
        # The default/additive controls replace only the file.
        empty = pki / "empty-trust-dir"
        empty.mkdir()
        TRUST_ENV.clear()
        TRUST_ENV.update(LC_ALL="C", SSL_CERT_FILE=str(pki / "empty.pem"), SSL_CERT_DIR=str(empty))
    for v, root in (("25", args.schema25), ("26", args.schema26)):
        cell = args.out / v
        cell.mkdir()
        plan, core, _, _ = plans(cell, v)
        offline(args, cell, v, pki, plan)
        if args.bundles:
            operator_smoke(args, cell, v, pki, plan)
        print(f"CPP_OMS_012 UCI 2.{v[1]}: verified peer, TLS negatives, rollback and shutdown PASS", flush=True)
        if args.sleet:
            broker_cases(args, cell, v, root, pki, plan, core)
            print(f"CPP_OMS_012 UCI 2.{v[1]}: pinned Sleet WS exit 0; verified WSS A/B/C exit 6; receiver core PASS", flush=True)


if __name__ == "__main__":
    main()