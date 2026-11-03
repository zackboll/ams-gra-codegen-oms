#!/usr/bin/env python3
"""Memory-BIO TLS peer controls ciphertext fragmentation and record batching."""
import argparse
import json
import os
from pathlib import Path
import re
import socket
import ssl
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "task100"))
from local_peer import exact, frame, handshake, receive


class Peer:
    def __init__(self, raw, context, fragment=False):
        self.raw = raw
        self.incoming, self.outgoing = ssl.MemoryBIO(), ssl.MemoryBIO()
        self.tls = context.wrap_bio(self.incoming, self.outgoing, server_side=True)
        self.fragment = fragment
        self.records = 0
        self.fragments = 0
        while True:
            try:
                self.tls.do_handshake()
                self.flush()
                break
            except ssl.SSLWantReadError:
                self.flush()
                self.feed()

    def feed(self):
        data = self.raw.recv(65536)
        if not data:
            self.incoming.write_eof()
        else:
            self.incoming.write(data)

    def flush(self):
        data = self.outgoing.read()
        if not data:
            return
        # Count actual records, not application write calls.
        pos = 0
        while pos < len(data):
            assert pos + 5 <= len(data)
            pos += 5 + int.from_bytes(data[pos + 3:pos + 5], "big")
            self.records += 1
        assert pos == len(data)
        if self.fragment:
            for i in range(0, len(data), 7):
                self.raw.sendall(data[i:i + 7])
                self.fragments += 1
                time.sleep(0.001)
        else:
            self.raw.sendall(data)

    def recv(self, count):
        while True:
            try:
                return self.tls.read(count)
            except ssl.SSLWantReadError:
                self.flush()
                self.feed()

    def sendall(self, data):
        assert self.tls.write(data) == len(data)
        self.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--cert", type=Path, required=True)
    parser.add_argument("--key", type=Path, required=True)
    parser.add_argument("--shim", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    results = []
    for case in ["idle", "fragments", "records", "ping", "close", "backpressure", "burst-backpressure", "deaf",
                 "wrong-protocol", "untrusted", "hostname", "truncation", "failed-handshake"]:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(args.cert, args.key)
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        listener.settimeout(10)
        port = listener.getsockname()[1]
        errors, observations = [], {}

        def serve():
            try:
                if case == "failed-handshake":
                    for _ in range(20):
                        with listener.accept()[0] as raw:
                            raw.sendall(b"not TLS\r\n")
                    return
                with listener.accept()[0] as raw:
                    raw.settimeout(10)
                    try:
                        peer = Peer(raw, context)
                    except ssl.SSLError:
                        if case not in {"untrusted", "hostname"}:
                            raise
                        return
                    handshake(peer, case)
                    if case == "wrong-protocol":
                        return
                    if case == "truncation":
                        return
                    if case == "deaf":
                        time.sleep(4)  # No reads: bounded Send must fail closed.
                        return
                    # The client is continuously polling while the fixture is
                    # idle. It must not wait for this future record to arrive.
                    time.sleep(0.15)
                    if case == "records":
                        before = peer.records
                        peer.tls.write(frame(1, b"Hello 1") + frame(1, b"Hello 2"))
                        peer.tls.write(frame(1, b"Hello 3"))
                        peer.tls.write(frame(1, b"Hello 4"))
                        peer.flush()  # three records in one socket write
                        observations["batched_records"] = peer.records - before
                        assert observations["batched_records"] == 3
                    if case in {"ping", "backpressure", "fragments"}:
                        peer.fragment = case == "fragments"
                        payload = bytes(range(125))
                        peer.sendall(frame(9, payload))
                        # No application Send is made: Poll alone must send Pong.
                        assert receive(peer) == (10, payload)
                        observations["exact_pong"] = payload.hex()
                        peer.fragment = False
                    close = b"\x03\xe8downstream"
                    if case == "burst-backpressure":
                        first, second = bytes(range(125)), b"second-ping\x00\xff"
                        peer.sendall(frame(9, first) + frame(9, second) + frame(8, close))
                        assert receive(peer) == (10, first)
                        assert receive(peer) == (10, second)
                        assert receive(peer) == (8, close)
                        observations["ordered_control_responses"] = 3
                        return
                    peer.sendall(frame(8, close))
                    assert receive(peer) == (8, close)
                    observations["exact_close"] = close.hex()
                    observations["fragmented_ciphertext_sends"] = peer.fragments
            except BaseException as exc:
                errors.append(repr(exc))
            finally:
                listener.close()

        thread = threading.Thread(target=serve, daemon=True)
        thread.start()
        mode = "records" if case == "records" else case
        command = [str(args.probe.resolve()), f"wss://{'127.0.0.1' if case == 'hostname' else 'localhost'}:{port}/owp",
                   "" if case == "untrusted" else str(args.cert.resolve()), mode]
        env = dict(os.environ)
        if case in {"backpressure", "burst-backpressure"}:
            env["LD_PRELOAD"] = str(args.shim.resolve())
        run = subprocess.run(command, capture_output=True, text=True, timeout=18, env=env)
        thread.join(12)
        (out / f"{case}.command.json").write_text(json.dumps(command) + "\n")
        (out / f"{case}.stdout").write_text(run.stdout)
        (out / f"{case}.stderr").write_text(run.stderr)
        (out / f"{case}.exit").write_text(str(run.returncode) + "\n")
        (out / f"{case}.peer.json").write_text(json.dumps({"errors": errors, **observations}, indent=2) + "\n")
        assert not errors and not thread.is_alive(), (case, errors, run.stdout)
        negative = case in {"wrong-protocol", "untrusted", "hostname", "truncation"}
        if negative:
            assert run.returncode == 1 and "OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR" in run.stdout, run.stdout
            assert "WSS_PROBE_COMPLETE" not in run.stdout
        elif case == "failed-handshake":
            assert run.returncode == 0 and "FAILED_HANDSHAKE_CLEANUP_COMPLETE cycles=20 fd_growth=0" in run.stdout, run.stdout
        elif case == "deaf":
            assert run.returncode == 0 and "WSS_DEAF_PUBLICATION_FAILED_CLOSED" in run.stdout, run.stdout
        else:
            assert run.returncode == 0 and "WSS_PROBE_COMPLETE" in run.stdout, run.stdout
            polls = int(run.stdout.split("polls=")[1])
            assert polls > 10, "idle period not observed"
        if case in {"backpressure", "burst-backpressure"}:
            blocked, partial = map(int, re.search(r"WSS_BLOCKED_SENDS=(\d+) WSS_PARTIAL_SENDS=(\d+)", run.stderr).groups())
            assert blocked > 10 and partial > 1, run.stderr
            observations.update(blocked=blocked, partial=partial)
        results.append({"case": case, "exit": run.returncode, **observations})
        print(case, "PASS", flush=True)
    (out / "result.json").write_text(json.dumps({"completed": True, "cases": results}, indent=2) + "\n")


if __name__ == "__main__":
    main()