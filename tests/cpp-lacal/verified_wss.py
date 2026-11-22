#!/usr/bin/env python3
"""Independent loopback TLS controls and transparent pinned-Sleet relay."""
import argparse
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

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/task109"))
from relay import Forwarder, frames
from local_peer import exact, frame, handshake, receive

VERIFY = "tls_verification_failed"
HANDSHAKE = "tls_handshake_failed"


def certificates(out, full):
    out.mkdir(mode=0o700)

    def openssl(*args):
        subprocess.run(["openssl", *args], cwd=out, check=True, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=15)

    def root(name, ca=True):
        openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes",
                "-keyout", name + ".key", "-out", name + ".pem", "-days", "3", "-subj", "/CN=" + name,
                "-addext", "basicConstraints=critical,CA:" + ("TRUE" if ca else "FALSE"),
                "-addext", "keyUsage=critical," + ("keyCertSign,cRLSign" if ca else "digitalSignature"),
                *([] if ca else ["-addext", "subjectAltName=DNS:localhost"]))

    root("ca")
    root("other")
    (out / "index").touch()
    (out / "serial").write_text("1000\n")
    openssl("req", "-new", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes",
            "-keyout", "leaf.key", "-out", "leaf.csr", "-subj", "/CN=localhost")

    def leaf(name, san, dates=(), issuer="ca", usage="serverAuth"):
        (out / "leaf.cnf").write_text(
            "[ca]\ndefault_ca=issuer\n[issuer]\ndatabase=index\nserial=serial\nnew_certs_dir=.\n"
            f"certificate={issuer}.pem\nprivate_key={issuer}.key\ndefault_md=sha256\ndefault_days=2\n"
            "policy=policy\nunique_subject=no\nx509_extensions=leaf\n[policy]\ncommonName=supplied\n"
            "[leaf]\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\n"
            f"extendedKeyUsage={usage}\n" + (f"subjectAltName={san}\n" if san else ""))
        openssl("ca", "-batch", "-notext", "-config", "leaf.cnf", "-in", "leaf.csr", "-out", name + ".pem", *dates)

    good = "DNS:localhost,IP:127.0.0.1,IP:::1"
    leaf("good", good)
    leaf("wrong-dns", "DNS:wrong.invalid,IP:127.0.0.1")
    if full:
        root("self-signed", ca=False)
        leaf("other-leaf", good, issuer="other")
        leaf("wrong-ip", "DNS:127.0.0.1,IP:127.0.0.2")
        leaf("cn-only", "")
        leaf("expired", good, ("-startdate", "20000101000000Z", "-enddate", "20010101000000Z"))
        leaf("future", good, ("-startdate", "20990101000000Z", "-enddate", "21000101000000Z"))
        leaf("client-only", good, usage="clientAuth")
        (out / "bundle.pem").write_bytes((out / "other.pem").read_bytes() + (out / "ca.pem").read_bytes())
        (out / "malformed.pem").write_bytes((out / "ca.pem").read_bytes() + b"-----BEGIN CERTIFICATE-----\nbad\n")
        (out / "invalid-pem.pem").write_text("-----BEGIN CERTIFICATE-----\nbad\n-----END CERTIFICATE-----\n")
        (out / "empty.pem").touch()


def context(pki, cert, sni, version=None):
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(pki / (cert + ".pem"), pki / ("self-signed.key" if cert == "self-signed" else "leaf.key"))
    if version is not None:
        ctx.minimum_version = ctx.maximum_version = version
    ctx.sni_callback = lambda _sock, name, _ctx: sni.append(name)
    return ctx


def listener(ip="127.0.0.1", backlog=1):
    sock = socket.socket(socket.AF_INET6 if ":" in ip else socket.AF_INET)
    sock.settimeout(5)
    sock.bind((ip, 0))
    sock.listen(backlog)
    return sock


def client_command(probe, url, ca, timeout, expected, scenario):
    return [probe, url, str(ca), str(timeout), expected, scenario]


class RecordRequest:
    def __init__(self, sock):
        self.sock = sock
        self.data = bytearray()

    def recv(self, count):
        data = self.sock.recv(count)
        self.data.extend(data)
        return data

    def sendall(self, data):
        self.sock.sendall(data)


def drain_large_pub(sock):
    """Require byte-for-byte payload, then return to the Close assertion."""
    prefix = b'PUB telemetry {"Log":"'
    expected = prefix + b"x" * (32 * 1024 * 1024) + b'"}'
    offset, continuation = 0, False
    while True:
        first, second = exact(sock, 2)
        assert first & 0x7F == (0 if continuation else 1) and second & 0x80
        size = second & 0x7F
        if size in (126, 127):
            size = int.from_bytes(exact(sock, 2 if size == 126 else 8), "big")
        assert offset + size <= len(expected)
        key, data = exact(sock, 4), exact(sock, size)
        # Apply each byte of the mask with bytes.translate, not a Python loop
        # over the whole payload (also keeps the sanitizer run bounded).
        chunk = expected[offset:offset + size]
        for index, mask in enumerate(key):
            table = bytes(value ^ mask for value in range(256))
            assert data[index::4].translate(table) == chunk[index::4]
        offset += size
        if first & 0x80:
            assert offset == len(expected)
            return
        continuation = True


def probe_case(probe, pki, name, expected="ok", *, host="localhost", cert="good", ca="ca.pem", mode=None,
               version=None, budget=2000, env=None):
    ip = "::1" if host == "[::1]" else "127.0.0.1"
    with listener(ip) as server:
        port = server.getsockname()[1]
        sni, errors, evidence, observations = [], [], {}, []
        ctx = context(pki, cert, sni, version)
        mode = mode or name
        if mode == "obsolete-tls":
            ctx.set_ciphers("DEFAULT:@SECLEVEL=0")  # only the isolated TLS-1.1 server
        scenario = {"ws-close-timeout": "close-timeout", "tls-close-timeout": "close-timeout",
                    "tls-close-truncated": "close-truncated"}.get(mode, mode)
        target = "/log?qualified=%2B"
        url = f"wss://{host}:{port}{target}"
        if mode == "query-only":
            target = "/?qualified=%2B"
            url = f"wss://{host}:{port}{target[1:]}"
        large_write = mode in {"write-drop", "write-close", "write-drain"}

        def observe(prefix):
            line = child.stdout.readline().rstrip()
            assert line.startswith(prefix), (prefix, line)
            observations.append(line)

        def serve():
            try:
                with server.accept()[0] as raw:
                    raw.settimeout(4)
                    if mode in {"interrupted", "tls-timeout"}:
                        hello = raw.recv(16384)
                        assert hello[:1] == b"\x16" and b"GET " not in hello and b"INIT " not in hello
                        if mode == "tls-timeout":
                            while raw.recv(16384):
                                pass
                        return
                    if mode == "shared-budget":
                        time.sleep(0.25)
                    try:
                        ses = ctx.wrap_socket(raw, server_side=True, suppress_ragged_eofs=False)
                    except ssl.SSLError as exc:
                        if mode == "obsolete-tls":
                            assert exc.reason == "UNSUPPORTED_PROTOCOL", str(exc)
                            evidence.update(alert=exc.reason, application_bytes=0)
                            return
                        # A certificate alert during the TLS handshake (not an
                        # EOF after application traffic) is independent evidence.
                        assert expected == VERIFY and "ALERT" in str(exc), str(exc)
                        evidence.update(alert=exc.reason, application_bytes=0)
                        return
                    with ses:
                        evidence["tls"] = ses.version()
                        assert expected != VERIFY, "client completed TLS with a bad certificate"
                        if mode == "before-upgrade":
                            ses.shutdown(socket.SHUT_WR)
                            return
                        if mode == "upgrade-timeout":
                            assert ses.recv(1) == b"G"
                            time.sleep(0.4)
                            return
                        request = RecordRequest(ses)
                        if mode == "http-rejection":
                            # Read the identical request, but use a complete
                            # non-101 response that has no close-delimited body.
                            while not request.data.endswith(b"\r\n\r\n"):
                                assert request.recv(1) and len(request.data) < 16384
                            request.sendall(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                        else:
                            handshake(request, mode)
                        assert request.data.startswith(f"GET {target} HTTP/1.1\r\n".encode())
                        assert f"Host: {host}:{port}\r\n".encode() in request.data
                        if expected == "handshake_rejected":
                            # No INIT (or other WS bytes) on a rejected upgrade.
                            try:
                                assert ses.recv(1) == b""
                            except ssl.SSLEOFError:
                                pass
                            return
                        opcode, init = receive(ses)
                        assert opcode == 1 and init.startswith(b"INIT ")
                        assert json.loads(init[5:]) == dict(service_id="cpp-oms-011", schema="002.6.0",
                                                           verbose=True, versions=["1.0"])
                        if mode == "init-rejected":
                            ses.sendall(frame(1, b"-ERR Illegal-State rejected"))
                            return
                        ses.sendall(frame(1, b"+OK"))
                        if mode in {"info-timeout", "shared-budget"}:
                            time.sleep(0.55)
                            return
                        info = b'INFO {"version":"1.0","server_id":"tls-control"}'
                        ses.sendall(frame(1, b"INFO {}" if mode == "invalid-info" else info))
                        if mode == "invalid-info":
                            return
                        if mode in {"sub-ack", "active-pub"}:
                            assert receive(ses)[1].startswith(b"SUB " if mode == "sub-ack" else b"PUB ")
                            return
                        if mode == "active-read":
                            return
                        if large_write:
                            observe("connect=ok ")
                            observe("in-flight-write")
                            if mode == "write-close":
                                time.sleep(0.7)
                            if mode != "write-drain":
                                return
                            observe("close-deferred")
                            drain_large_pub(ses)
                        if mode in {"peer-close-timeout", "peer-close-clean"}:
                            ses.sendall(frame(8, b"\x03\xe8"))
                            assert receive(ses)[0] == 8
                            if mode == "peer-close-clean":
                                with ses.unwrap():
                                    evidence["close_notify"] = True
                            else:
                                time.sleep(0.7)
                            return
                        assert receive(ses) == (8, b"\x03\xe8")
                        if mode == "ws-close-timeout":
                            time.sleep(0.7)
                            return
                        ses.sendall(frame(8, b"\x03\xe8"))
                        if mode == "tls-close-timeout":
                            time.sleep(0.7)
                            return
                        if mode == "tls-close-truncated":
                            ses.shutdown(socket.SHUT_WR)
                            with socket.socket(fileno=ses.detach()) as tcp:
                                tcp.settimeout(4)
                                while tcp.recv(16384):
                                    pass
                            return
                        with ses.unwrap():  # independent reciprocal TLS close_notify
                            evidence["close_notify"] = True
            except Exception as exc:
                errors.append(repr(exc))

        ca_path = ca if ca == "-" else pki / ca
        with subprocess.Popen(client_command(probe, url, ca_path, budget, expected, scenario),
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1,
                              env=env) as child:
            thread = threading.Thread(target=serve)
            thread.start()
            try:
                # No concurrent reads of child.stdout (the large-write peer
                # needs the client's exact in-flight observation first).
                if large_write:
                    thread.join(8)
                    assert not thread.is_alive(), "missing in-flight write observation"
                stdout, stderr = child.communicate(timeout=10)
            finally:
                if child.poll() is None:
                    child.kill()
                thread.join(6)
        assert not thread.is_alive() and not errors and child.returncode == 0 and "cleanup=ok" in stdout, (
            name, child.returncode, errors, stdout, stderr)
        if "tls" in evidence or expected == VERIFY:
            assert sni == (["localhost"] if host == "localhost" else [None]), (name, sni)
        if mode == "shared-budget":
            elapsed = int(stdout.split("elapsed_ms=")[1].split()[0])
            assert 415 <= elapsed < 650, stdout
        print(json.dumps(dict(case=name, sni=sni, **evidence, client=observations + stdout.splitlines())), flush=True)


def local_controls(probe, pki):
    def without_dial(name, sock, url, ca, expected, budget=150):
        result = subprocess.run(client_command(probe, url, ca, budget, expected, name),
                                capture_output=True, text=True, timeout=8)
        assert result.returncode == 0 and "cleanup=ok" in result.stdout, (name, result.stdout, result.stderr)
        if sock is not None:
            assert not select.select([sock], [], [], 0)[0], f"unexpected dial: {name}"
        print(json.dumps(dict(case=name, client=result.stdout.splitlines())), flush=True)

    with listener() as sock:
        port = sock.getsockname()[1]
        url = f"wss://127.0.0.1:{port}"
        for name in ("missing.pem", "malformed.pem", "invalid-pem.pem", "empty.pem", "good.pem", "."):
            without_dial(f"invalid-trust:{name}", sock, url, pki / name, "invalid_argument")
        without_dial("plaintext-ca", sock, f"ws://127.0.0.1:{port}", pki / "ca.pem", "invalid_argument")
        for authority in (f"localhost:{port}#fragment", f"user@localhost:{port}", "", ":443", "localhost:",
                          "localhost:65536", "localhost:0", "::1", "[::1", "[localhost]:443", "[::1]no",
                          ".localhost", "localhost..invalid"):
            without_dial(f"url:{authority}", sock, "wss://" + authority, "-", "invalid_argument")
        without_dial("unsupported-url", sock, f"https://localhost:{port}", "-", "unsupported_scheme")
        without_dial("dns-timeout", sock, f"wss://slow-resolution.invalid:{port}", pki / "ca.pem", "timeout")
        without_dial("dns-failure", sock, f"wss://failed-resolution.invalid:{port}", pki / "ca.pem", "connect_failed")
    without_dial("tcp-refused", None, url, pki / "ca.pem", "connect_failed")
    with listener(backlog=0) as sock:
        port = sock.getsockname()[1]
        # Linux's zero backlog admits one completed connection. Leave it
        # unaccepted; the next SYN cannot complete before this test's deadline.
        with socket.create_connection(("127.0.0.1", port), timeout=2):
            without_dial("tcp-timeout", None, f"wss://127.0.0.1:{port}", pki / "ca.pem", "timeout")
        with sock.accept()[0]:
            assert not select.select([sock], [], [], 0)[0], "second connection unexpectedly completed"


def matrix(probe, pki, smoke):
    probe_case(probe, pki, "dns-tls12", version=ssl.TLSVersion.TLSv1_2)
    probe_case(probe, pki, "wrong-dns", VERIFY, cert="wrong-dns")
    if smoke:
        print("CPP_OMS_011_WSS_SMOKE_COMPLETE")
        return
    local_controls(probe, pki)
    probe_case(probe, pki, "ipv4-tls13", host="127.0.0.1", ca="bundle.pem", version=ssl.TLSVersion.TLSv1_3)
    probe_case(probe, pki, "ipv6", host="[::1]")
    probe_case(probe, pki, "obsolete-tls", HANDSHAKE, version=ssl.TLSVersion.TLSv1_1)
    probe_case(probe, pki, "query-only")
    # Redirect only this child process's OpenSSL default paths, never the
    # host's trust store. The server uses this default root, not tls_ca_file.
    default = dict(os.environ, SSL_CERT_FILE=str(pki / "ca.pem"))
    probe_case(probe, pki, "default-paths", ca="-", env=default)
    probe_case(probe, pki, "additive-default-paths", ca="other.pem", env=default)
    for name, cert, host, ca in [
        ("untrusted-ca", "other-leaf", "localhost", "ca.pem"),
        ("self-signed", "self-signed", "localhost", "ca.pem"),
        ("wrong-ip", "wrong-ip", "127.0.0.1", "ca.pem"),
        ("expired", "expired", "localhost", "ca.pem"),
        ("not-yet-valid", "future", "localhost", "ca.pem"),
        ("missing-san", "cn-only", "localhost", "ca.pem"),
        ("client-only-eku", "client-only", "localhost", "ca.pem"),
        ("default-untrusted", "good", "localhost", "-"),
    ]:
        probe_case(probe, pki, name, VERIFY, cert=cert, host=host, ca=ca)
    for name, expected, budget in [
        ("interrupted", HANDSHAKE, 2000), ("tls-timeout", "timeout", 160),
        ("before-upgrade", "connect_failed", 2000), ("upgrade-timeout", "timeout", 200),
        ("invalid-upgrade", "handshake_rejected", 2000), ("http-rejection", "handshake_rejected", 2000),
        ("wrong-protocol", "handshake_rejected", 2000), ("missing-protocol", "handshake_rejected", 2000),
        ("duplicate-protocol", "handshake_rejected", 2000), ("init-rejected", "broker_rejected", 2000),
        ("invalid-info", "protocol_violation", 2000), ("info-timeout", "timeout", 200),
        ("shared-budget", "timeout", 450),
    ]:
        probe_case(probe, pki, name, expected, budget=budget)
    for name in ("sub-ack", "active-read", "active-pub", "write-drop", "write-close", "write-drain", "destructor",
                 "ws-close-timeout", "tls-close-timeout", "tls-close-truncated", "peer-close-timeout", "peer-close-clean"):
        probe_case(probe, pki, name)
    print("CPP_OMS_011_WSS_MATRIX_COMPLETE")


def relay_log(pki, ws_port, binary, golden, result_file, schema):
    """Forward bytes unchanged, including Close; passively validate the wire."""
    with listener() as server:
        port = server.getsockname()[1]
        info, errors, sni = {}, [], []
        ctx = context(pki, "good", sni)
        streams = {"client": bytearray(), "broker": bytearray()}

        def forward():
            try:
                with server.accept()[0] as raw:
                    raw.settimeout(6)
                    peer = Forwarder.__new__(Forwarder)
                    peer.batches = 0
                    Forwarder.__init__(peer, raw, ctx)
                    info.update(tls=peer.tls.version(), cipher=peer.tls.cipher(), sni=sni)
                    with socket.create_connection(("127.0.0.1", ws_port), timeout=5) as plain:
                        tls_open = True
                        while tls_open:
                            ready, _, _ = select.select([raw, plain], [], [], 0 if peer.tls.pending() else 10)
                            assert ready or peer.tls.pending(), "relay idle timeout"
                            if raw in ready:
                                peer.feed()  # one TCP read, not a blocking SSL application read
                            # A readable socket can contain only part of a TLS
                            # record, or only TLS control records. Never wait
                            # for more bytes here: the broker may be ready too.
                            while True:
                                try:
                                    data = peer.tls.read(65536)
                                except ssl.SSLWantReadError:
                                    peer.flush()
                                    break
                                if not data:
                                    tls_open = False
                                    break
                                plain.sendall(data)
                                streams["client"].extend(data)
                            if plain in ready:
                                data = plain.recv(65536)
                                if not data:
                                    # Broker EOF, not a fabricated WS Close.
                                    try:
                                        peer.tls.unwrap()
                                    except ssl.SSLWantReadError:
                                        pass
                                    peer.flush()
                                    info["broker_eof"] = True
                                    break
                                peer.sendall(data)
                                streams["broker"].extend(data)
                    info.update(records=peer.records, fragments=peer.fragments, batches=peer.batches)
            except Exception as exc:
                errors.append(repr(exc))

        thread = threading.Thread(target=forward)
        thread.start()
        try:
            result = subprocess.run([binary], capture_output=True, text=True, timeout=90,
                                    env=dict(os.environ, CPP_OMS_003_SLEET_URL=f"ws://127.0.0.1:{ws_port}",
                                             CPP_OMS_003_GOLDEN=str(golden), CPP_OMS_011_CA=str(pki / "ca.pem"),
                                             CPP_OMS_011_WSS_URL=f"wss://localhost:{port}"))
        finally:
            thread.join(12)
        print(result.stdout, end="", flush=True)
        result_file.with_suffix(".status.json").write_text(json.dumps(dict(info=info, errors=errors,
            client_exit=result.returncode, bytes_forwarded={key: len(data) for key, data in streams.items()}), indent=2) + "\n")
        assert result.returncode == 0 and not errors and not thread.is_alive(), (result.stderr, errors)
        wire = {name: frames(data) for name, data in streams.items()}
        result_file.write_text(json.dumps(dict(info=info, wire=wire), indent=2) + "\n")
        assert info["sni"] == ["localhost"] and info["tls"] in {"TLSv1.2", "TLSv1.3"} and info["broker_eof"]
        assert info["fragments"] > 1 and info["batches"] > 1
        assert f"Host: localhost:{port}".lower() in wire["client"]["upgrade"].lower()
        assert "sec-websocket-protocol: owp" in wire["broker"]["upgrade"].lower()
        client, broker = ([f["text"] for f in wire[name]["frames"] if f["opcode"] == 1]
                          for name in ("client", "broker"))
        assert json.loads(client[0][5:]) == dict(versions=["1.0"], schema=schema, service_id="task102-log", verbose=True)
        assert broker[0] == "+OK" and json.loads(broker[1][5:])["version"] == "1.0"
        assert [line for line in client if line.startswith(("SUB ", "UNSUB "))] == [
            "SUB sub-1 Log task102-log", "SUB sub-2 Log unauthorized-topic", "UNSUB sub-1",
            "SUB sub-3 Log task102-log", "UNSUB sub-3"]
        assert any(line.startswith('MSG sub-3 {"Log":') for line in broker)
        assert sum(f["opcode"] == 8 for f in wire["client"]["frames"]) == 1
        assert not any(f["opcode"] == 8 for f in wire["broker"]["frames"])
        print("CPP_OMS_011_SLEET_WSS_COMPLETE", schema, json.dumps(info), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    offline = commands.add_parser("offline")
    offline.add_argument("probe", type=Path)
    offline.add_argument("scratch", type=Path)
    offline.add_argument("--smoke", action="store_true")
    log = commands.add_parser("relay-log")
    log.add_argument("pki", type=Path)
    log.add_argument("ws_port", type=int)
    log.add_argument("binary", type=Path)
    log.add_argument("golden", type=Path)
    log.add_argument("result", type=Path)
    log.add_argument("schema")
    args = parser.parse_args()
    if args.command == "offline":
        pki = args.scratch.resolve() / "pki"
        args.scratch.mkdir(parents=True, exist_ok=True)
        certificates(pki, not args.smoke)
        matrix(args.probe.resolve(), pki, args.smoke)
    else:
        relay_log(args.pki.resolve(), args.ws_port, args.binary.resolve(), args.golden, args.result, args.schema)


if __name__ == "__main__":
    main()