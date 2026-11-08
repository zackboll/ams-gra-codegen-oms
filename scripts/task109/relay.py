#!/usr/bin/env python3
"""Test-only TLS byte forwarder for the unchanged Task106 Sleet driver.

The executable adapter accepts Task106's ws URL and plan, but starts the real
Ada consumer with verified wss://localhost and an explicit CA. No OWP or
WebSocket bytes are manufactured or rewritten. MemoryBIO permits deterministic
partial ciphertext and multiple buffered records, using Task108's TLS peer.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import socket
import ssl
import subprocess
import sys
import threading
import time
from urllib.parse import urlsplit

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/task108"))
from wss_peer import Peer


class Forwarder(Peer):
    def flush(self):
        data = self.outgoing.read()
        if not data:
            return
        pos = 0
        records = 0
        while pos < len(data):
            if pos + 5 > len(data):
                raise ValueError("incomplete TLS record header")
            pos += 5 + int.from_bytes(data[pos + 3:pos + 5], "big")
            records += 1
        if pos != len(data):
            raise ValueError("incomplete TLS record")
        self.records += records
        # One small prefix and a pause expose partial encrypted input, without
        # spending milliseconds per seven bytes of large authentic UCI values.
        self.raw.sendall(data[:7])
        self.fragments += 1
        time.sleep(0.003)
        self.raw.sendall(data[7:])
        if records > 1:
            self.batches += 1

    def sendall(self, data):
        # Two actual TLS application records flushed together. Poll must also
        # progress when plaintext is buffered inside the backend, not in TCP.
        middle = max(1, len(data) // 2)
        for part in (data[:middle], data[middle:]):
            if part and self.tls.write(part) != len(part):
                raise ValueError("short TLS application write")
        self.flush()


def frames(data):
    """Passive wire evidence only; never used to supply expected payload values."""
    header, separator, data = data.partition(b"\r\n\r\n")
    if not separator:
        raise ValueError("missing HTTP upgrade")
    result = []
    while data:
        if len(data) < 2:
            raise ValueError("partial WebSocket frame")
        opcode, masked, size = data[0] & 15, bool(data[1] & 128), data[1] & 127
        offset = 2
        if size in (126, 127):
            width = 2 if size == 126 else 8
            size = int.from_bytes(data[offset:offset + width], "big")
            offset += width
        mask = data[offset:offset + 4] if masked else None
        offset += 4 if masked else 0
        payload = data[offset:offset + size]
        if len(payload) != size:
            raise ValueError("partial WebSocket payload")
        if masked:
            payload = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
        result.append(dict(opcode=opcode, bytes=size, text=payload.decode() if opcode == 1 else payload.hex()))
        data = data[offset + size:]
    return dict(upgrade=header.decode(), frames=result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("consumer", "cert", "key", "evidence"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("url")
    parser.add_argument("plan", type=Path)
    args = parser.parse_args()
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    target = urlsplit(args.url)
    if target.scheme != "ws" or target.hostname != "127.0.0.1" or target.path != "/":
        raise ValueError("expected Task106 loopback URL")
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(args.cert, args.key)
    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    port = listener.getsockname()[1]
    listener.listen(1)
    listener.settimeout(10)
    stop = threading.Event()
    errors, stats = [], {}
    streams = {"ada": bytearray(), "sleet": bytearray()}

    def forward():
        peer = None
        try:
            with listener.accept()[0] as raw:
                raw.settimeout(5)
                # Set before Peer.__init__, which calls our flush at handshake.
                peer = Forwarder.__new__(Forwarder)
                peer.batches = 0
                Forwarder.__init__(peer, raw, context)
                stats.update(tls=peer.tls.version(), cipher=peer.tls.cipher())
                with socket.create_connection((target.hostname, target.port), timeout=5) as plain:
                    while not stop.is_set():
                        ready, _, _ = select.select([raw, plain], [], [], 0.05)
                        if peer.tls.pending() or raw in ready:
                            data = peer.recv(65536)
                            if not data:
                                break
                            plain.sendall(data)
                            streams["ada"].extend(data)
                        if plain in ready:
                            data = plain.recv(65536)
                            if not data:
                                # Sleet finished its WS Close exchange. Deliver
                                # authenticated TLS EOF rather than truncating
                                # the TLS session (same rule as Task108).
                                try:
                                    peer.tls.unwrap()
                                except ssl.SSLWantReadError:
                                    pass
                                peer.flush()
                                break
                            peer.sendall(data)
                            streams["sleet"].extend(data)
                stats.update(records=peer.records, partial_writes=peer.fragments, record_batches=peer.batches)
        except (ssl.SSLEOFError, ConnectionResetError):
            # Production Close aborts TLS after its bounded WS Close exchange.
            pass
        except BaseException as error:
            errors.append(repr(error))
        finally:
            if peer is not None:
                stats.update(records=peer.records, partial_writes=peer.fragments, record_batches=peer.batches)
            listener.close()

    thread = threading.Thread(target=forward)
    thread.start()
    command = [str(args.consumer.resolve()), f"wss://localhost:{port}/", str(args.plan.resolve())]
    (out / "consumer.command.json").write_text(json.dumps(command) + "\n")
    child = None
    code = 1
    try:
        child = subprocess.Popen(command, env=dict(os.environ, OMS_WSS_TEST_CA=str(args.cert.resolve())))
        code = child.wait(timeout=120)
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.wait(timeout=5)
        stop.set()
        listener.close()
        thread.join(12)
        stats.update(consumer_exit=child.returncode if child else None,
                     cleanup=not thread.is_alive(), errors=errors)
        for name, data in streams.items():
            (out / (name + ".bin")).write_bytes(data)
            stats[name + "_sha256"] = hashlib.sha256(data).hexdigest()
        (out / "result.json").write_text(json.dumps(stats, indent=2) + "\n")
    if code or thread.is_alive() or errors:
        raise RuntimeError(f"TLS relay/consumer failed: {stats}")
    wire = {name: frames(data) for name, data in streams.items()}
    (out / "wire.json").write_text(json.dumps(wire, indent=2) + "\n")
    client = [frame["text"] for frame in wire["ada"]["frames"] if frame["opcode"] == 1]
    server = [frame["text"] for frame in wire["sleet"]["frames"] if frame["opcode"] == 1]
    if (sum(text.startswith("INIT ") for text in client) != 1
            or sum(text.startswith("SUB ") for text in client) != 3
            or sum(text.startswith("UNSUB ") for text in client) != 3
            or sum(text.startswith("INFO ") for text in server) != 1
            or "Sec-WebSocket-Protocol: owp".lower() not in wire["sleet"]["upgrade"].lower()
            or stats.get("partial_writes", 0) < 2 or stats.get("record_batches", 0) < 2):
        raise ValueError("missing actual Ada WSS wire/progression evidence")
    init = json.loads(next(text[5:] for text in client if text.startswith("INIT ")))
    if init.get("schema") != "002.5.0" or init.get("service_id") != "task106-multimessage":
        raise ValueError("incorrect Ada INIT schema/service")
    # Pinned Sleet breaks immediately on received Close, without flushing a
    # reciprocal WS frame. Do not invent a broker reply: record the Ada Close
    # and supply only authenticated TLS EOF. Task108 owns ordered Close tests.
    if not any(frame["opcode"] == 8 for frame in wire["ada"]["frames"]):
        raise ValueError("missing Ada WebSocket Close")
    stats["completed"] = True
    (out / "result.json").write_text(json.dumps(stats, indent=2) + "\n")


if __name__ == "__main__":
    main()