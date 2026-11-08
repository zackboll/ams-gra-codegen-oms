#!/usr/bin/env python3
"""Exercise TASK100 framing, bounds and lifecycle cases through verified TLS."""
import argparse
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
import sys
import threading
import time
import struct

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "task100"))
from local_peer import BAD_FRAMES, BAD_HANDSHAKES, transport_session, generated_session


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["probe", "consumer", "cert", "key", "evidence"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    out = args.evidence.resolve()
    out.mkdir(parents=True, exist_ok=False)
    cases = ["valid", "tcp-split", "stalled-partial", "fragment-ping", "utf8-fragments",
             "receive-large", "peer-close", "large", "send-failure", "validation",
             "lifecycle", "finalize", *sorted(BAD_HANDSHAKES), *BAD_FRAMES, "generated"]
    results = []
    for case in cases:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(args.cert, args.key)
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        listener.settimeout(10)
        port = listener.getsockname()[1]
        errors = []

        def serve():
            try:
                for _ in range(30 if case == "lifecycle" else 1):
                    with listener.accept()[0] as raw:
                        raw.settimeout(6)
                        with context.wrap_socket(raw, server_side=True) as sock:
                            if case == "send-failure":
                                from local_peer import handshake
                                handshake(sock, case)
                                # A TLS record can remain buffered when the
                                # underlying reset arrives. Do not reset before
                                # the client has consumed its HTTP upgrade.
                                time.sleep(0.2)
                                sock.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER,
                                                struct.pack("ii", 1, 0))
                            else:
                                (generated_session if case == "generated" else
                                 lambda s: transport_session(s, case))(sock)
            except BrokenPipeError:
                if case != "excessive-headers":
                    errors.append("unexpected BrokenPipeError")
                # The client may reject at 16 KiB before the TLS peer has
                # finished writing this intentionally oversized response.
                # Its exact transport exception is still required below.
            except BaseException as error:
                errors.append(repr(error))
            finally:
                listener.close()

        thread = threading.Thread(target=serve, daemon=True)
        thread.start()
        binary = args.consumer if case == "generated" else args.probe
        command = [str(binary.resolve()), f"wss://localhost:{port}/owp?task=108r1",
                   "closed" if case == "peer-close" else case]
        run = subprocess.run(command, capture_output=True, text=True, timeout=35,
                             env=dict(os.environ, OMS_WSS_TEST_CA=str(args.cert.resolve())))
        thread.join(12)
        (out / f"{case}.command.json").write_text(json.dumps(command) + "\n")
        (out / f"{case}.stdout").write_text(run.stdout)
        (out / f"{case}.stderr").write_text(run.stderr)
        (out / f"{case}.exit").write_text(str(run.returncode) + "\n")
        assert not errors and not thread.is_alive(), (case, errors, run.stdout)
        negative = case in BAD_FRAMES or case in BAD_HANDSHAKES
        if negative:
            assert run.returncode == 1 and "OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR" in run.stdout, (case, run.stdout)
        else:
            marker = "TASK100_GENERATED_CONSUMER_COMPLETE" if case == "generated" else "TRANSPORT_PROBE_COMPLETE"
            assert run.returncode == 0 and marker in run.stdout, (case, run.stdout)
        if case == "lifecycle":
            assert "RESOURCE_LIFECYCLES_COMPLETE cycles=30 fd_growth=0 task_growth=0" in run.stdout
        results.append({"case": case, "exit": run.returncode})
        print(case, "PASS", flush=True)
    (out / "result.json").write_text(json.dumps({"completed": True, "cases": results}, indent=2) + "\n")


if __name__ == "__main__":
    main()