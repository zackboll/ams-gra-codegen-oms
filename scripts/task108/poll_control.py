#!/usr/bin/env python3
"""Prove TLS-generated output progresses through actual OMS Poll alone."""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["probe", "peer", "cert", "key", "evidence"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    peer_command = [str(args.peer.resolve()), str(args.cert.resolve()), str(args.key.resolve())]
    peer = subprocess.Popen(peer_command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        port = peer.stdout.readline()
        assert port.startswith("PORT "), port
        command = [str(args.probe.resolve()), f"wss://localhost:{int(port.split()[1])}/", str(args.cert.resolve()), "tls-control"]
        run = subprocess.run(command, capture_output=True, text=True, timeout=15)
        stdout, stderr = peer.communicate(timeout=10)
        (out / "peer.command.json").write_text(json.dumps(peer_command) + "\n")
        (out / "client.command.json").write_text(json.dumps(command) + "\n")
        (out / "peer.stdout").write_text(port + stdout)
        (out / "peer.stderr").write_text(stderr)
        (out / "peer.exit").write_text(str(peer.returncode) + "\n")
        (out / "client.stdout").write_text(run.stdout)
        (out / "client.stderr").write_text(run.stderr)
        (out / "client.exit").write_text(str(run.returncode) + "\n")
        assert peer.returncode == 0 and "TLS_CONTROL_REPLY_RECEIVED plaintext=0" in stdout, (stdout, stderr, run.stdout)
        # Peer ends the rejected renegotiation fatally; this must not become a
        # successful WebSocket session or a TLS-truncation acceptance.
        assert run.returncode == 1 and "OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR" in run.stdout, run.stdout
        assert "VERIFIED_OWP_UPGRADE" in run.stdout and "MESSAGE " not in run.stdout
        (out / "result.json").write_text(json.dumps({"completed": True, "poll_tls_control": True, "application_plaintext": 0}) + "\n")
        print("POLL_TLS_GENERATED_OUTPUT_COMPLETE plaintext=0")
    finally:
        if peer.poll() is None:
            peer.kill()
            peer.communicate()


if __name__ == "__main__":
    main()