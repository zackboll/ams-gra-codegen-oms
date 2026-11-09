#!/usr/bin/env python3
"""Controlled real TLS peer; no fake TLS, transport replacement or insecure mode."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import select
import socket
import ssl
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task108.wss_peer import Peer
from task100.local_peer import frame, receive

CASES = [("handshake", 1000), ("handshake", 50), ("upgrade", 1000),
         ("trickle-upgrade", 1000), ("cumulative-upgrade", 1000),
         ("write", 1000), ("queued-write", 1000), ("close", 1000),
         ("expired-open", -1), ("short-open", 1), ("expired-send", 1000),
         ("expired-close", 1000), ("normal", 1000), ("reuse", 1000),
         ("normal", 9000), ("long-handshake", 9000),
         ("legacy-handshake", 0), ("legacy-normal", 0), ("legacy-close", 0)]


def require(value, detail):
    if not value:
        raise ValueError(detail)


def upgrade(peer, observations, stall, stop):
    request = bytearray()
    while not request.endswith(b"\r\n\r\n"):
        data = peer.recv(1)
        require(data, "EOF before HTTP request")
        request.extend(data)
        require(len(request) <= 16384, "request limit")
    require(b"Sec-WebSocket-Protocol: owp\r\n" in request, "owp request")
    observations["request"] = request.decode("ascii")
    if stall:
        stop.wait(5)
        return
    key = next(line.split(b": ", 1)[1] for line in request.split(b"\r\n") if line.startswith(b"Sec-WebSocket-Key:"))
    require(len(base64.b64decode(key)) == 16, "nonce")
    accept = base64.b64encode(hashlib.sha1(key + b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11").digest())
    peer.sendall(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                 b"Sec-WebSocket-Accept: " + accept + b"\r\nSec-WebSocket-Protocol: owp\r\n\r\n")
    observations["upgrade_sent"] = True


def parse_prefix(data):
    """Only complete client frames; a failed write may leave one partial tail."""
    pos, result = 0, []
    while pos + 2 <= len(data):
        first, second = data[pos:pos + 2]
        require(first & 0x70 == 0 and first & 0x80 and second & 0x80, "client frame flags")
        size, start = second & 127, pos + 2
        if size in (126, 127):
            count = 2 if size == 126 else 8
            if start + count > len(data):
                break
            size = int.from_bytes(data[start:start + count], "big"); start += count
        if start + 4 + size > len(data):
            break
        key = data[start:start + 4]; start += 4
        payload = bytes(b ^ key[i % 4] for i, b in enumerate(data[start:start + size]))
        result.append((first & 15, payload))
        pos = start + size
    return result, len(data) - pos


def run_case(probe, cert, key, out, mode, millis):
    directory = out / f"{mode}-{millis}"
    directory.mkdir()
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert, key)
    listener = socket.socket()
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 4096)
    listener.bind(("127.0.0.1", 0)); listener.listen(); listener.settimeout(0.1)
    port = listener.getsockname()[1]
    stop = threading.Event()
    errors, observations = [], dict(connections=0, sessions=[])

    def serve():
        try:
            while not stop.is_set():
                try:
                    raw, _ = listener.accept()
                except socket.timeout:
                    continue
                observations["connections"] += 1
                item = dict(accepted_at=time.monotonic())
                observations["sessions"].append(item)
                with raw:
                    raw.settimeout(5)
                    if mode in ("handshake", "long-handshake", "legacy-handshake"):
                        item["client_hello"] = raw.recv(65536).hex()
                        require(item["client_hello"].startswith("16"), "actual TLS ClientHello")
                        stop.wait(5)
                        continue
                    if mode == "cumulative-upgrade":
                        time.sleep(0.15)  # consumes the same caller budget before TLS
                    peer = Peer(raw, context)
                    item["tls_verified_peer_handshake"] = True
                    if mode == "cumulative-upgrade":
                        time.sleep(0.35)  # does not grant a fresh upgrade budget
                    if mode == "trickle-upgrade":
                        request = bytearray()
                        while not request.endswith(b"\r\n\r\n"):
                            data = peer.recv(1)
                            require(data, "trickle HTTP request EOF")
                            request.extend(data)
                            require(len(request) <= 16384, "request limit")
                        item["request"] = request.decode("ascii")
                        count = 0
                        for byte in b"HTTP/1.1 101 Switching Protocols\r\n":
                            if stop.wait(0.10):
                                break
                            peer.sendall(bytes([byte]))
                            count += 1
                        item["trickle_bytes"] = count
                        continue
                    upgrade(peer, item, mode in ("upgrade", "cumulative-upgrade"), stop)
                    if mode in ("upgrade", "cumulative-upgrade"):
                        continue
                    if mode in ("write", "queued-write"):
                        if mode == "queued-write":
                            peer.sendall(frame(9, b"deadline-ping"))
                        item["read_stall_started"] = time.monotonic()
                        stop.wait(5)  # no TLS/socket reads while the Ada Send is running
                        captured = bytearray()
                        raw.settimeout(1)
                        while True:
                            try:
                                data = peer.recv(65536)
                                if not data:
                                    break
                                captured.extend(data)
                            except (ssl.SSLError, ConnectionError, TimeoutError):
                                break
                        (directory / "client.frames").write_bytes(captured)
                        decoded, partial = parse_prefix(captured)
                        publications = [payload for op, payload in decoded if op == 1]
                        require(all(payload.startswith(f" {i}:".encode()) for i, payload in enumerate(publications, 1)),
                                "publication replay or order")
                        require(all(b"FORBIDDEN" not in p for _, p in decoded), "post-failure replay")
                        item.update(complete_publications=len(publications), partial_tail=partial,
                                    captured_bytes=len(captured), pong_count=sum(op == 10 for op, _ in decoded))
                        if mode == "queued-write":
                            require(decoded and decoded[0] == (10, b"deadline-ping"), "queued Pong before publication")
                        continue
                    if mode in ("normal", "legacy-normal", "reuse") and observations["connections"] == 1:
                        require(receive(peer) == (1, b"Hello"), "normal publication")
                        peer.sendall(frame(1, b"Hello"))
                        item["echo"] = True
                    try:
                        op, payload = receive(peer)
                        require(op == 8 and payload == b"\x03\xe8", "only graceful Close expected")
                        require(mode not in ("expired-send", "expired-close"), "expired operation emitted frame")
                        item["close"] = True
                        if mode in ("close", "legacy-close"):
                            stop.wait(5)  # deliberately no reciprocal WS Close / TLS EOF
                        else:
                            # Genuine TLS EOF only, as with non-reciprocal pinned Sleet.
                            try:
                                peer.tls.unwrap()
                            except ssl.SSLWantReadError:
                                pass
                            peer.flush()
                    except (ssl.SSLError, RuntimeError, ConnectionError) as error:
                        if mode not in ("expired-send", "expired-close"):
                            raise
                        item["abortive_eof"] = type(error).__name__
        except Exception as error:
            errors.append(f"{type(error).__name__}: {error}")
        finally:
            listener.close()

    thread = threading.Thread(target=serve)
    thread.start()
    command = [str(probe.resolve()), f"wss://localhost:{port}/", str(cert.resolve()), mode, str(millis)]
    (directory / "command.json").write_text(json.dumps(command) + "\n")
    start = time.monotonic()
    try:
        child = subprocess.run(command, capture_output=True, text=True, timeout=12)
        elapsed = time.monotonic() - start
    finally:
        stop.set(); thread.join(6)
    require(not thread.is_alive(), "peer thread did not finish")
    (directory / "stdout").write_text(child.stdout)
    (directory / "stderr").write_text(child.stderr)
    result = dict(mode=mode, deadline_ms=millis, elapsed=elapsed, exit=child.returncode,
                  observations=observations, peer_errors=errors, cleanup=True, stdout=child.stdout)
    (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    validate(result)
    print("CASE_PASS", mode, millis, f"{elapsed:.4f}s", flush=True)
    return result


def validate(result):
    mode, millis, elapsed = result["mode"], result["deadline_ms"], result["elapsed"]
    require(result["exit"] == 0 and "DEADLINE_PROBE_PASS " + mode in result["stdout"], "Ada probe failed")
    require(not result["peer_errors"] and result["cleanup"], "peer/cleanup failure")
    expected = 0 if mode in ("expired-open", "short-open") else 2 if mode == "reuse" else 1
    require(result["observations"]["connections"] == expected, "connection attempted/replayed")
    if millis > 0 and millis <= 1000:
        # 150 ms external scheduling/process-exit tolerance; 2 s cannot pass.
        require(elapsed <= millis / 1000 + 0.150, "caller deadline exceeded tolerance")
    if mode == "expired-open":
        require(elapsed < 0.150, "expired Open blocked")
    if mode in ("legacy-handshake", "long-handshake"):
        require(1.8 <= elapsed <= 2.25, "historical two-second setup cap changed")
    if mode in ("write", "queued-write", "upgrade", "trickle-upgrade", "cumulative-upgrade", "close", "expired-send", "expired-close"):
        require(0.80 <= elapsed <= 1.15, "operation was not exercised through caller deadline")
    if mode == "legacy-close":
        require(0.20 <= elapsed <= 0.60, "historical close wait changed")
    for session in result["observations"]["sessions"]:
        if mode not in ("handshake", "legacy-handshake", "long-handshake"):
            require(session.get("tls_verified_peer_handshake"), "missing real TLS handshake")
        if mode not in ("handshake", "legacy-handshake", "long-handshake", "upgrade", "trickle-upgrade", "cumulative-upgrade"):
            require(session.get("upgrade_sent"), "missing real upgrade")
        if mode == "queued-write":
            require(session["pong_count"] == 1, "queued control output")
    require("RESOURCES fd=" in result["stdout"], "resource sample missing")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("probe", "cert", "key", "evidence"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=False)
    results = [run_case(args.probe, args.cert, args.key, args.evidence, mode, millis) for mode, millis in CASES]
    (args.evidence / "result.json").write_text(json.dumps(dict(completed=True, cases=results), indent=2) + "\n")