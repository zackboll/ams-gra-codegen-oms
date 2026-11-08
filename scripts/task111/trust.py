#!/usr/bin/env python3
"""Real TLS peer driving one long-lived production Ada client; no mocks."""
import argparse
import json
import os
from pathlib import Path
import queue
import socket
import ssl
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/task100"))
from local_peer import frame, handshake, receive

PREFLIGHT = {"both", "plaintext"}
NEGATIVE = ("untrusted", "malformed", "hostname", "both", "plaintext", "handshake-close", "upgrade")


def validate(mode, output, observation):
    """Backend-independent categories plus independent peer observations."""
    if "HARNESS_FAILURE" in output or "DONE " + mode not in output:
        raise ValueError("probe/harness did not complete")
    positive = mode in ("pem", "path")
    if ("REJECTED category=Transport_Error" in output) == positive:
        raise ValueError("wrong Ada outcome/category")
    if observation.get("error"):
        raise ValueError("peer/harness failure: " + observation["error"])
    if mode in PREFLIGHT:
        if observation["accepted"]:
            raise ValueError("configuration rejection reached network")
    elif mode == "malformed":
        # The pin validates PEM envelope before dialling, but decodes its DER
        # after TCP connect. Either no dial or TCP EOF without any TLS/app data
        # is a configuration rejection, not a failed TLS verification.
        if observation["accepted"] and not observation.get("pre_tls_eof"):
            raise ValueError("malformed PEM progressed beyond configuration")
    elif not observation["accepted"] or not observation.get("clienthello"):
        raise ValueError("missing TLS attempt / possible plaintext downgrade")
    if positive:
        if (not observation.get("tls") or observation.get("application") != ["INIT {}"]
                or not observation.get("normal_close")):
            raise ValueError("incomplete positive exchange/close")
    elif observation.get("application"):
        raise ValueError("application data following rejected Open")
    elif mode == "upgrade":
        if not observation.get("tls") or not observation.get("invalid_upgrade") or not observation.get("eof"):
            raise ValueError("invalid upgrade did not fail closed after verified TLS")
    elif mode not in PREFLIGHT | {"malformed"} and not observation.get("handshake_failed"):
        raise ValueError("expected failed TLS handshake")


class Driver:
    def __init__(self, args):
        self.args = args
        self.out = args.evidence.resolve()
        self.out.mkdir(parents=True, exist_ok=False)
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.context.load_cert_chain(args.cert, args.key)
        self.lines = queue.Queue()
        self.transcript = []
        self.results = []

    def read_until(self, marker, timeout=12):
        deadline = time.monotonic() + timeout
        output = []
        while time.monotonic() < deadline:
            try:
                line = self.lines.get(timeout=max(0.001, deadline - time.monotonic()))
            except queue.Empty:
                break
            output.append(line)
            if "HARNESS_FAILURE" in line:
                raise RuntimeError("Ada harness failure: " + line)
            if line == marker:
                return "\n".join(output)
        raise RuntimeError("probe deadline waiting for " + marker)

    def command(self, command, marker):
        self.transcript.append("> " + command)
        self.child.stdin.write(command + "\n")
        self.child.stdin.flush()
        return self.read_until(marker)

    def attempt(self, mode):
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        listener.settimeout(0.05)
        port = listener.getsockname()[1]
        stop = threading.Event()
        observation = dict(accepted=False, application=[])

        def eof(sock):
            try:
                data = sock.recv(4096)
            except (ssl.SSLEOFError, ConnectionResetError):
                data = b""
            if data:
                observation["application"].append(data.decode(errors="replace"))
                raise ValueError("unexpected post-rejection bytes")
            observation["eof"] = True

        def serve():
            try:
                while not stop.is_set():
                    try:
                        raw, _ = listener.accept()
                        break
                    except socket.timeout:
                        continue
                else:
                    return
                observation["accepted"] = True
                with raw:
                    raw.settimeout(5)
                    prefix = raw.recv(5, socket.MSG_PEEK)
                    observation["prefix_hex"] = prefix.hex()
                    if mode == "malformed" and not prefix:
                        observation["pre_tls_eof"] = True
                        return
                    observation["clienthello"] = len(prefix) >= 3 and prefix[0:2] == b"\x16\x03"
                    if not observation["clienthello"]:
                        raise ValueError("not a TLS ClientHello")
                    if mode == "handshake-close":
                        observation["handshake_failed"] = "server closed after ClientHello"
                        return
                    try:
                        secure = self.context.wrap_socket(raw, server_side=True)
                    except ssl.SSLError as error:
                        # Only a TLS alert/EOF is an expected trust failure. A peer
                        # timeout, bad fixture or other exception is harness failure.
                        if mode not in ("untrusted", "untrusted-close", "hostname") or error.reason not in (
                                "TLSV1_ALERT_UNKNOWN_CA", "SSLV3_ALERT_BAD_CERTIFICATE", "SSLV3_ALERT_CERTIFICATE_UNKNOWN",
                                "TLSV1_ALERT_INTERNAL_ERROR", "UNEXPECTED_EOF_WHILE_READING"):
                            raise
                        observation["handshake_failed"] = error.reason
                        return
                    with secure:
                        observation["tls"] = secure.version()
                        observation["cipher"] = secure.cipher()[0]
                        handshake(secure, "wrong-protocol" if mode == "upgrade" else "valid")
                        observation["owp_offer"] = True  # handshake requires exactly owp offer
                        if mode == "upgrade":
                            observation["invalid_upgrade"] = "not-owp"
                            eof(secure)
                            return
                        opcode, payload = receive(secure)
                        if (opcode, payload) != (1, b"INIT {}"):
                            raise ValueError("unexpected client application frame")
                        observation["application"].append(payload.decode())
                        secure.sendall(frame(1, b"INFO {}"))
                        if receive(secure) != (8, b"\x03\xe8"):
                            raise ValueError("missing normal Close")
                        secure.sendall(frame(8, b"\x03\xe8"))
                        eof(secure)
                        observation["normal_close"] = True
            except BaseException as error:
                observation["error"] = repr(error)

        thread = threading.Thread(target=serve)
        thread.start()
        scheme = "ws" if mode == "plaintext" else "wss"
        host = "127.0.0.1" if mode in ("hostname", "plaintext") else "localhost"
        url = f"{scheme}://{host}:{port}/"
        start = time.monotonic()
        try:
            output = self.command(mode + " " + url, "DONE " + mode)
        finally:
            stop.set()
            thread.join(6)
            listener.close()
        if thread.is_alive():
            raise RuntimeError("peer thread leaked")
        elapsed = time.monotonic() - start
        if elapsed >= 10:
            raise ValueError("case exceeded bounded deadline")
        self.results.append(dict(mode=mode, url=url, trust_source="CA_Path" if mode == "path" else "CA_PEM contents",
                                 seconds=round(elapsed, 4), output=output, peer=observation))
        (self.out / "attempts.json").write_text(json.dumps(self.results, indent=2) + "\n")
        validate(mode, output, observation)

    def execute(self):
        command = [str(self.args.probe.resolve()), str(self.args.cert.resolve()), str(self.args.unrelated.resolve())]
        env = dict(os.environ)
        if self.args.memcheck:
            env["VALGRIND_LIB"] = str(self.args.memcheck_lib.resolve())
            command = [str(self.args.memcheck.resolve()), "--leak-check=full", "--show-leak-kinds=definite,indirect",
                       "--errors-for-leak-kinds=definite,indirect", "--error-exitcode=97",
                       "--log-file=" + str(self.out / "memcheck.log"), *command]
        (self.out / "probe.command.json").write_text(json.dumps(command) + "\n")
        self.child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                      stderr=subprocess.PIPE, text=True, bufsize=1, env=env)

        def collect():
            for line in self.child.stdout:
                self.transcript.append(line.rstrip())
                self.lines.put(line.rstrip())

        reader = threading.Thread(target=collect)
        reader.start()
        try:
            self.read_until("READY")
            # Warm up provider, DNS worker and both successful and failed paths.
            for _ in range(3):
                for mode in ("untrusted", "pem", "pem", "path"):
                    self.attempt(mode)
            baseline = self.command("baseline", "DONE baseline")
            for mode in NEGATIVE:
                self.attempt(mode)
                self.attempt("pem")  # every independent rejection is followed by reuse
            # Principal sequence, including the explicit Close-after-failure arm.
            for _ in range(20):
                for mode in ("untrusted", "pem", "pem", "untrusted-close", "pem", "pem"):
                    self.attempt(mode)
            self.attempt("path")
            final = self.command("quit", "COMPLETE")
            self.child.wait(timeout=5)
            if self.child.returncode != 0:
                raise RuntimeError("probe nonzero exit")
            result = dict(completed=True, same_client=True, lifecycle_cycles=20, baseline=baseline, final=final,
                          cases=self.results, no_plaintext_downgrade=True, needs_read_runtime_exercised=False)
            (self.out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
            print("TASK111_TRUST_COMPLETE", len(self.results), "attempts", flush=True)
        finally:
            if self.child.poll() is None:
                self.child.kill()
                self.child.wait(timeout=5)
            reader.join(5)
            (self.out / "probe.stdout").write_text("\n".join(self.transcript) + "\n")
            (self.out / "probe.stderr").write_text(self.child.stderr.read())
            self.child.stdin.close(); self.child.stdout.close(); self.child.stderr.close()
            if reader.is_alive():
                raise RuntimeError("reader thread leaked")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("probe", "cert", "key", "unrelated", "evidence"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--memcheck", type=Path)
    parser.add_argument("--memcheck-lib", type=Path)
    Driver(parser.parse_args()).execute()