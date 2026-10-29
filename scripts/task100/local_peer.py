#!/usr/bin/env python3
"""Deterministic real TCP peer; no mock transport and no external packages."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import socket
import struct
import subprocess
import threading
import time


def exact(sock, count):
    result = b""
    while len(result) < count:
        chunk = sock.recv(count - len(result))
        if not chunk:
            raise RuntimeError("unexpected EOF")
        result += chunk
    return result


def receive(sock):
    first, second = exact(sock, 2)
    if not second & 128:
        raise RuntimeError("unmasked client frame")
    size = second & 127
    if size == 126:
        size = int.from_bytes(exact(sock, 2), "big")
    elif size == 127:
        size = int.from_bytes(exact(sock, 8), "big")
    if size > 1_048_576:
        raise RuntimeError("excessive client payload")
    key = exact(sock, 4)
    data = exact(sock, size)
    return first & 15, bytes(b ^ key[i % 4] for i, b in enumerate(data))


def frame(opcode, payload=b"", final=True):
    first = opcode | (128 if final else 0)
    if len(payload) < 126:
        header = bytes([first, len(payload)])
    elif len(payload) < 65536:
        header = bytes([first, 126]) + struct.pack("!H", len(payload))
    else:
        header = bytes([first, 127]) + struct.pack("!Q", len(payload))
    return header + payload


def handshake(sock, case):
    request = b""
    while not request.endswith(b"\r\n\r\n"):
        request += exact(sock, 1)
        if len(request) > 16_384:
            raise RuntimeError("excessive request")
    assert b"Sec-WebSocket-Protocol: owp\r\n" in request
    key = next(line.split(b": ", 1)[1] for line in request.split(b"\r\n")
               if line.startswith(b"Sec-WebSocket-Key:"))
    assert len(base64.b64decode(key)) == 16
    accept = base64.b64encode(hashlib.sha1(key + b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11").digest())
    protocol = b"Sec-WebSocket-Protocol: owp\r\n"
    if case == "missing-protocol":
        protocol = b""
    elif case == "wrong-protocol":
        protocol = b"Sec-WebSocket-Protocol: not-owp\r\n"
    elif case == "duplicate-protocol":
        protocol *= 2
    elif case == "extensions":
        protocol += b"Sec-WebSocket-Extensions: permessage-deflate\r\n"
    elif case == "malformed-header":
        protocol += b"this is not a header\r\n"
    elif case == "excessive-headers":
        protocol += b"X-Large: " + b"a" * 17_000 + b"\r\n"
    if case == "wrong-accept":
        accept = b"wrong"
    upgrade = b"not-websocket" if case == "invalid-upgrade" else b"websocket"
    status = b"302 Found" if case == "redirect" else b"101 Switching Protocols"
    response = (b"HTTP/1.1 " + status + b"\r\nUpgrade: " + upgrade +
                b"\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " + accept +
                b"\r\n" + protocol + b"\r\n")
    if case == "tcp-split":
        for byte in response:
            sock.sendall(bytes([byte]))
    else:
        sock.sendall(response)


BAD_HANDSHAKES = {"missing-protocol", "wrong-protocol", "duplicate-protocol",
                  "extensions", "malformed-header", "excessive-headers",
                  "wrong-accept", "invalid-upgrade", "redirect"}
BAD_FRAMES = {
    "masked": b"\x81\x80\x00\x00\x00\x00",
    "reserved": b"\x83\x00",
    "rsv": b"\xc1\x00",
    "utf8": frame(1, b"\xc0\xaf"),
    "binary": frame(2, b"binary"),
    "oversized": b"\x81\x7f" + struct.pack("!Q", 1_048_577),
    "fragment-control": frame(9, b"bad", False),
    "large-control": b"\x89\x7e\x00\x7e",
    "nonminimal": b"\x81\x7e\x00\x01x",
    "length-msb": b"\x81\x7f\x80" + b"\x00" * 7,
    "orphan-continuation": frame(0, b"bad"),
    "new-text-during-fragment": frame(1, b"a", False) + frame(1, b"b"),
    "bad-close-code": frame(8, b"\x03\xed"),
    "bad-close-utf8": frame(8, b"\x03\xe8\xff"),
    "fragment-oversized": frame(1, b"a", False) + b"\x80\x7f" + struct.pack("!Q", 1_048_576),
}


def close_exchange(sock):
    assert receive(sock) == (8, b"\x03\xe8")
    sock.sendall(frame(8, b"\x03\xe8"))
    assert sock.recv(1) == b""


def transport_session(sock, case):
    handshake(sock, case)
    if case in BAD_HANDSHAKES:
        try:
            assert sock.recv(1) == b""
        except ConnectionResetError:
            pass
        return
    if case == "send-failure":
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
        return
    if case == "large":
        time.sleep(0.05)
    opcode, payload = receive(sock)
    assert opcode == 1
    if case == "large":
        assert payload == b"s" * 1_048_576
    elif case == "validation":
        assert payload == b"valid-after-rejection"
    else:
        assert payload == "INIT {} {-0, UTF8: £}".encode()
    if case in BAD_FRAMES:
        sock.sendall(BAD_FRAMES[case])
        try:
            assert sock.recv(1) == b""
        except ConnectionResetError:
            pass
        return
    if case == "abrupt":
        return
    if case == "peer-close":
        sock.sendall(frame(8, b"\x03\xe8"))
        assert receive(sock) == (8, b"\x03\xe8")
        assert sock.recv(1) == b""
        return
    if case == "fragment-ping":
        sock.sendall(frame(1, b"He", False) + frame(9, b"x\x00y") + frame(0, b"llo"))
        assert receive(sock) == (10, b"x\x00y")
    elif case == "utf8-fragments":
        sock.sendall(frame(1, b"\xc2", False) + frame(0, b"\xa3"))
    elif case == "receive-large":
        sock.sendall(frame(1, b"x" * 1_048_576))
    elif case in {"tcp-split", "stalled-partial"}:
        data = frame(1, b"Hello")
        for i, byte in enumerate(data):
            sock.sendall(bytes([byte]))
            if i + 1 < len(data):
                time.sleep(0.25 if case == "stalled-partial" else 0.002)
    else:
        sock.sendall(frame(10, b"ignore") + frame(1, b"Hello"))
    if case == "finalize":
        assert sock.recv(1) == b""
    else:
        close_exchange(sock)


def generated_session(sock):
    handshake(sock, "valid")
    subscriptions = {}
    while True:
        opcode, payload = receive(sock)
        if opcode == 8:
            sock.sendall(frame(8, payload))
            assert sock.recv(1) == b""
            return
        assert opcode == 1
        text = payload.decode()
        if text.startswith("INIT "):
            info = {"version": "1.0", "server_id": "task100-peer", "uuids": {
                "system": "550e8400-e29b-41d4-a716-446655440000",
                "service": "550e8400-e29b-41d4-a716-446655440100"}, "system_label": "task100"}
            sock.sendall(frame(1, b"+OK") + frame(1, ("INFO " + json.dumps(info)).encode()))
        elif text.startswith("SUB "):
            _, sid, name, topic, *group = text.split()
            assert topic == "common-topic"
            subscriptions[sid] = name
            sock.sendall(frame(1, b"+OK"))
        elif text.startswith("UNSUB "):
            del subscriptions[text.split()[1]]
            sock.sendall(frame(1, b"+OK"))
        elif text.startswith("PUB "):
            _, topic, document = text.split(" ", 2)
            if topic == "rejected-topic":
                sock.sendall(frame(1, b'-ERR Illegal-State "topic rejected"'))
                continue
            name, = json.loads(document)
            sock.sendall(frame(1, b"+OK"))
            for sid, subscribed in subscriptions.items():
                if subscribed == name:
                    if name.endswith("MessageA"):
                        original = json.loads(document)
                        body = original[name]
                        at = next(k for k in body if k.endswith("At"))
                        invalid = json.loads(document)
                        invalid[name][at] = "25:00:00Z"
                        bad_payload = json.dumps(invalid)
                        duplicate = '{"' + name + '":' + json.dumps(body) + ',"' + name + '":' + json.dumps(body) + '}'
                        bad_type = json.loads(document)
                        count = next(k for k in body if k.endswith("Count"))
                        bad_type[name][count] = "not-an-integer"
                        for bad in ['{"broken":', duplicate, bad_payload, json.dumps(bad_type)]:
                            sock.sendall(frame(1, f"MSG {sid} {bad}".encode()))
                    sock.sendall(frame(1, f"MSG {sid} {document}".encode()))
        else:
            raise RuntimeError("unexpected OWP command " + text)


def float_session(sock, namespace):
    handshake(sock, "valid")
    assert receive(sock) == (1, b"FLOAT64_TRANSPORT")
    prefix = "" if namespace == "oam" else "{urn:task100}"
    for token in ["-0", "4.9406564584124654e-324", "1.7976931348623157e308",
                  "1.00000000000000011102230246251565404236316680908203126"]:
        payload = ('{"' + prefix + 'Sample":' + token + ',"' + prefix + 'Count":42,"' +
                   prefix + 'At":"12:34:56.5000Z","' + prefix + 'Label":"task100"}').encode()
        sock.sendall(frame(1, payload[:7], False) + frame(9, b"float") + frame(0, payload[7:]))
        assert receive(sock) == (10, b"float")
        assert receive(sock) == (1, payload), "transport changed exact numeric lexeme"
    close_exchange(sock)


def run(binary, case, evidence, namespace="oam", preload=None):
    errors = []
    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    listener.settimeout(8)
    port = listener.getsockname()[1]
    mode = ("closed" if case in {"abrupt", "peer-close"} else case)

    def serve():
        try:
            for _ in range(30 if case == "lifecycle" else 1):
                with listener.accept()[0] as sock:
                    sock.settimeout(6)
                    if case == "generated":
                        generated_session(sock)
                    elif case == "float64":
                        float_session(sock, namespace)
                    else:
                        transport_session(sock, case)
        except Exception as error:
            errors.append(repr(error))
        finally:
            listener.close()

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    command = [str(binary), f"ws://127.0.0.1:{port}/owp?task=100", mode]
    env = None
    if preload:
        import os
        env = dict(os.environ, LD_PRELOAD=str(preload))
    result = subprocess.run(command, capture_output=True, text=True, timeout=30, env=env)
    thread.join(8)
    (evidence / f"{case}.stdout").write_text(result.stdout)
    (evidence / f"{case}.stderr").write_text(result.stderr)
    (evidence / f"{case}.exit").write_text(str(result.returncode) + "\n")
    expected_failure = case in BAD_HANDSHAKES or case in BAD_FRAMES
    assert not errors and not thread.is_alive(), (case, errors, result.stdout)
    assert (result.returncode != 0) == expected_failure, (case, result.stdout, result.stderr)
    if expected_failure:
        # A timeout, crash or unrelated Program_Error is not rejection evidence.
        assert "raised OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR :" in result.stdout
        assert "TRANSPORT_PROBE_COMPLETE" not in result.stdout
    if preload:
        line = next(line for line in result.stderr.splitlines() if line.startswith("TASK100_FORCED_SHORT_SENDS="))
        assert int(line.split("=")[1]) > 0, "partial-send injector not exercised"
    if case == "lifecycle":
        assert "RESOURCE_LIFECYCLES_COMPLETE cycles=30 fd_growth=0 task_growth=0" in result.stdout
    if not expected_failure and case not in {"generated", "float64", "send-failure", "abrupt", "peer-close", "receive-large"}:
        assert ("RECEIVED £" if case == "utf8-fragments" else "RECEIVED Hello") in result.stdout
    if case == "receive-large":
        assert "RECEIVED_MAX_MESSAGE_COMPLETE" in result.stdout
    if not expected_failure:
        marker = ("TASK100_GENERATED_CONSUMER_COMPLETE" if case == "generated" else
                  "TASK100_FLOAT64_TRANSPORT_COMPLETE" if case == "float64" else "TRANSPORT_PROBE_COMPLETE")
        assert marker in result.stdout, (case, result.stdout)
    print("TASK100_LOCAL_CASE_COMPLETE", case, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--consumer", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--float-probe", type=Path)
    parser.add_argument("--namespace", choices=["oam", "qualified"], default="oam")
    parser.add_argument("--partial-send-shim", type=Path)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=False)
    cases = ["valid", "tcp-split", "stalled-partial", "fragment-ping", "utf8-fragments", "receive-large", "peer-close", "abrupt", "large",
             "send-failure", "validation", "lifecycle", "finalize", *sorted(BAD_HANDSHAKES), *BAD_FRAMES]
    for case in cases:
        run(args.probe.resolve(), case, args.evidence,
            preload=args.partial_send_shim.resolve() if args.partial_send_shim and case == "large" else None)
    invalid_urls = ["wss://127.0.0.1:80/", "http://127.0.0.1:80/",
                    "ws://192.0.2.1:80/", "ws://127.0.0.1:0/",
                    "ws://127.0.0.1:80/\r\nInjected: yes"]
    for index, url in enumerate(invalid_urls):
        result = subprocess.run([str(args.probe.resolve()), url, "valid"],
                                capture_output=True, text=True, timeout=3)
        assert result.returncode != 0 and "UPGRADE_OWP_COMPLETE" not in result.stdout
        assert "raised OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR :" in result.stdout
        label = f"invalid-url-{index}"
        (args.evidence / f"{label}.stdout").write_text(result.stdout)
        (args.evidence / f"{label}.stderr").write_text(result.stderr)
        (args.evidence / f"{label}.exit").write_text(str(result.returncode) + "\n")
        cases.append(label)
        print("TASK100_LOCAL_CASE_COMPLETE", label, flush=True)
    run(args.consumer.resolve(), "generated", args.evidence)
    if args.float_probe:
        run(args.float_probe.resolve(), "float64", args.evidence, args.namespace)
    completed = cases + ["generated"] + (["float64"] if args.float_probe else [])
    (args.evidence / "result.json").write_text(json.dumps({"completed": True, "cases": completed}, indent=2) + "\n")
    print("TASK100_LOCAL_SUITE_COMPLETE", len(completed))


if __name__ == "__main__":
    main()