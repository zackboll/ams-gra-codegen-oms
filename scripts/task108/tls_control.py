#!/usr/bin/env python3
"""Generate a real TLS 1.2 HelloRequest, never application plaintext."""
import argparse
import json
from pathlib import Path
import socket
import subprocess
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["probe", "cert", "key", "evidence"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    command = ["openssl", "s_server", "-accept", str(port), "-tls1_2",
               "-cert", str(args.cert.resolve()), "-key", str(args.key.resolve()), "-naccept", "1"]
    with (out / "server.stdout").open("w+") as stdout, (out / "server.stderr").open("w+") as stderr:
        server = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr, text=True)
        # The consumer's failed Connect is retryable; readiness is observed in
        # s_server's ACCEPT line instead of consuming its one allowed session.
        import time
        deadline = time.monotonic() + 5
        while "ACCEPT" not in (out / "server.stdout").read_text():
            assert server.poll() is None and time.monotonic() < deadline
            time.sleep(0.01)
        client_command = [str(args.probe.resolve()), "localhost", str(port), str(args.cert.resolve()), "generated"]
        client = subprocess.Popen(client_command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        watchdog = threading.Timer(12, client.kill)
        watchdog.start()
        lines = []
        try:
            for line in client.stdout:
                lines.append(line)
                if line.strip() == "AWAIT_TLS_CONTROL":
                    server.stdin.write("r\n")
                    server.stdin.flush()
            err = client.stderr.read()
            code = client.wait(timeout=2)
        finally:
            watchdog.cancel()
            server.terminate()
            server.communicate(timeout=5)
        text = "".join(lines)
        (out / "server.command.json").write_text(json.dumps(command) + "\n")
        (out / "client.command.json").write_text(json.dumps(client_command) + "\n")
        (out / "client.stdout").write_text(text)
        (out / "client.stderr").write_text(err)
        (out / "client.exit").write_text(str(code) + "\n")
        assert code == 0 and "TLS_CONTROL_FLUSH_COMPLETE plaintext=0" in text, text
        (out / "result.json").write_text(json.dumps({"completed": True, "tls_generated_output": True}) + "\n")
        print(text, end="")


if __name__ == "__main__":
    main()