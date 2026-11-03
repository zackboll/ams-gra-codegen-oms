#!/usr/bin/env python3
"""Real verified-TLS downstream API probes; only Python's standard library."""
import argparse
import hashlib
import json
from pathlib import Path
import socket
import ssl
import subprocess
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    args = parser.parse_args()
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    cert, key = out / "cert.pem", out / "key.pem"
    command = ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
               "-keyout", str(key), "-out", str(cert), "-days", "2", "-subj",
               "/CN=localhost", "-addext", "subjectAltName=DNS:localhost"]
    generated = subprocess.run(command, capture_output=True, text=True, check=True)
    (out / "certificate.command.json").write_text(json.dumps(command) + "\n")
    (out / "certificate.stderr").write_text(generated.stderr)
    results = []
    for case in ["idle", "deaf", "sink", "untrusted", "hostname", "truncation"]:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert, key)
        listener = socket.socket()
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 4096)
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        resume, done = threading.Event(), threading.Event()
        expected = []
        errors, received = [], bytearray()

        def peer():
            try:
                raw, _ = listener.accept()
                raw.settimeout(15)
                try:
                    conn = context.wrap_socket(raw, server_side=True)
                except ssl.SSLError:
                    if case not in {"untrusted", "hostname"}:
                        raise
                    return
                with conn:
                    if case == "truncation":
                        return  # SSLSocket.close does not send close_notify.
                    if case == "sink":
                        assert resume.wait(12), "consumer did not observe backpressure"
                        while True:
                            data = conn.recv(65536)
                            if not data:
                                break
                            received.extend(data)
                            if len(received) == expected[0]:
                                conn.sendall(b"accounted")
                                assert done.wait(12)
                                break
                    else:
                        assert done.wait(12), "consumer did not finish"
            except BaseException as exc:
                errors.append(repr(exc))
            finally:
                listener.close()

        thread = threading.Thread(target=peer, daemon=True)
        thread.start()
        command = [str(args.probe.resolve()),
                   "127.0.0.1" if case == "hostname" else "localhost",
                   str(listener.getsockname()[1]), "" if case == "untrusted" else str(cert),
                   case if case in {"idle", "deaf", "sink"} else "read"]
        proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        lines = []
        # A watchdog terminates even a falsely blocking nonblocking operation.
        watchdog = threading.Timer(20, proc.kill)
        watchdog.start()
        try:
            for line in proc.stdout:
                lines.append(line)
                if line.strip() == "RESUME_SINK":
                    resume.set()
                if line.startswith("BACKPRESSURE accepted="):
                    expected.append(int(line.split("accepted=")[1].split()[0]))
            stderr = proc.stderr.read()
            code = proc.wait(timeout=2)
        finally:
            watchdog.cancel()
            done.set()
            thread.join(16)
            if proc.poll() is None:
                proc.kill()
                proc.wait()
        stdout = "".join(lines)
        (out / f"{case}.command.json").write_text(json.dumps(command) + "\n")
        (out / f"{case}.stdout").write_text(stdout)
        (out / f"{case}.stderr").write_text(stderr)
        (out / f"{case}.exit").write_text(str(code) + "\n")
        assert not thread.is_alive() and not errors, errors
        if case in {"untrusted", "hostname", "truncation"}:
            assert code == 1 and "AHT.TRANSPORT.TLS.TLS_ERROR" in stdout, stdout
            assert "AHT_CONSUMER_COMPLETE" not in stdout
        else:
            assert code == 0 and "AHT_CONSUMER_COMPLETE" in stdout, stdout
        result = {"case": case, "exit": code}
        if case == "sink":
            total = int(next(s.split("=")[1] for s in lines if s.startswith("FLUSH_COMPLETE")))
            pattern = bytes(range(251))
            assert len(received) == total and received == (pattern * (total // 251 + 1))[:total]
            result.update(bytes=total, sha256=hashlib.sha256(received).hexdigest())
        results.append(result)
        print(case, "PASS", flush=True)
    (out / "result.json").write_text(json.dumps({"completed": True, "cases": results}, indent=2) + "\n")


if __name__ == "__main__":
    main()