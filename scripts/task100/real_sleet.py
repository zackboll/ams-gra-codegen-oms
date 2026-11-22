#!/usr/bin/env python3
"""Execute generated Ada against an unmodified exact pinned Sleet checkout."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import ssl
import threading

REV = "94796daf55ebd1134dce0925e6a204a05ad3423b"
TREE = "47da595b9078cac1a9351e8290d1ae7f3976367f"
ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--consumer", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--tls-cert", type=Path)
    parser.add_argument("--tls-key", type=Path)
    args = parser.parse_args()
    checkout = args.checkout.resolve()
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    identity = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD", "HEAD^{tree}"], text=True)
    assert identity.splitlines() == [REV, TREE]
    assert not subprocess.check_output(["git", "-C", str(checkout), "status", "--porcelain"], text=True)
    (evidence / "sleet-source.txt").write_text(identity)
    binary = checkout / "target/release/sleet"
    assert binary.is_file()
    (evidence / "sleet-binary.sha256").write_text(hashlib.sha256(binary.read_bytes()).hexdigest() + "\n")
    services = evidence / "services.d"
    services.mkdir()
    (services / "task100.toml").write_text(
        'service_id = "task100"\nservice_uuid = "550e8400-e29b-41d4-a716-446655440100"\n'
        'allowed_topics = ["common-topic"]\n')
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    config = evidence / "server.toml"
    schema = ROOT / "tests/fixtures/service-generate/lacal-websocket.xsd"
    config.write_text(
        f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task100-pinned-sleet"\n'
        f'system_label = "Task 100"\nschema_path = "{schema}"\n'
        f'schema_version = "000.1.0"\nservices_dir = "{services}"\n'
        'system_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    env = dict(os.environ, RUST_LOG="debug")
    with (evidence / "server.stdout").open("wb") as out, (evidence / "server.stderr").open("wb") as err:
        server = subprocess.Popen([str(binary), str(config)], stdout=out, stderr=err, env=env)
        try:
            deadline = time.monotonic() + 8
            while True:
                assert server.poll() is None, "Sleet exited before ready"
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                        break
                except OSError:
                    assert time.monotonic() < deadline, "Sleet ready timeout"
                    time.sleep(0.02)
            stop = threading.Event()
            proxy_errors = []
            proxy = None
            if args.tls_cert:
                assert args.tls_key
                context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                context.load_cert_chain(args.tls_cert, args.tls_key)
                proxy = socket.socket()
                proxy.bind(("127.0.0.1", 0))
                proxy.listen()
                proxy.settimeout(10)

                def terminate_tls():
                    try:
                        with proxy.accept()[0] as raw:
                            raw.settimeout(10)
                            with context.wrap_socket(raw, server_side=True) as secure, socket.create_connection(("127.0.0.1", port), timeout=10) as plain:
                                # Single-owner SSL calls; no concurrent SSL_read/
                                # SSL_write. Select is test-only terminator work.
                                import select
                                while not stop.is_set():
                                    ready, _, _ = select.select([secure, plain], [], [], 0.05)
                                    if secure.pending() or secure in ready:
                                        data = secure.recv(65536)
                                        if not data:
                                            break
                                        plain.sendall(data)
                                    if plain in ready:
                                        data = plain.recv(65536)
                                        if not data:
                                            # Authenticate TLS EOF; closing an
                                            # SSLSocket alone truncates it.
                                            try:
                                                secure.unwrap()
                                            except (ssl.SSLError, OSError):
                                                pass
                                            break
                                        secure.sendall(data)
                    except ssl.SSLEOFError:
                        pass  # OMS explicitly aborts after the WS Close exchange.
                    except BaseException as error:
                        proxy_errors.append(repr(error))

                terminator = threading.Thread(target=terminate_tls, daemon=True)
                terminator.start()
                command = [str(args.consumer.resolve()), f"wss://localhost:{proxy.getsockname()[1]}/"]
            else:
                command = [str(args.consumer.resolve()), f"ws://127.0.0.1:{port}/"]
            (evidence / "consumer.command.json").write_text(json.dumps(command) + "\n")
            consumer_env = dict(os.environ)
            if args.tls_cert:
                consumer_env['OMS_WSS_TEST_CA'] = str(args.tls_cert.resolve())
            result = subprocess.run(command, capture_output=True, text=True, timeout=30, env=consumer_env)
            if proxy:
                stop.set()
                terminator.join(12)
                proxy.close()
                assert not terminator.is_alive() and not proxy_errors, proxy_errors
            (evidence / "consumer.stdout").write_text(result.stdout)
            (evidence / "consumer.stderr").write_text(result.stderr)
            (evidence / "consumer.exit").write_text(str(result.returncode) + "\n")
            print(result.stdout, end="")
            assert result.returncode == 0, "generated Ada consumer failed"
            assert "TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1" in result.stdout
            assert "OWNERSHIP_WEBSOCKET_COMPLETE" in result.stdout
        finally:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)
            (evidence / "server.exit").write_text(str(server.returncode) + "\n")
    assert not subprocess.check_output(["git", "-C", str(checkout), "status", "--porcelain"], text=True)
    (evidence / "result.json").write_text(json.dumps({"completed": True, "revision": REV,
        "generated_typed_handlers": True, "real_uci": False,
        "tls_terminator": bool(args.tls_cert)}, indent=2) + "\n")
    print("TASK100_REAL_PINNED_SLEET_COMPLETE")


if __name__ == "__main__":
    main()