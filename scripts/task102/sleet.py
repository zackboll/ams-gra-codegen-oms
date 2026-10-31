#!/usr/bin/env python3
"""One real-schema, generated-Ada pinned-Sleet cell (no simulated broker)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import socket
import subprocess
import sys
import threading
import time

from websockets.exceptions import ConnectionClosed
from websockets.sync.client import connect

sys.dont_write_bytecode = True
import cases

REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'


def execute(checkout, consumer, schema, release, directory):
    directory.mkdir()
    identity = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True)
    assert identity.splitlines() == [REV, TREE]
    assert not subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain'])
    binary = checkout / 'target/release/sleet'
    (directory / 'sleet-source.txt').write_text(identity)
    (directory / 'sleet-binary.sha256').write_text(hashlib.sha256(binary.read_bytes()).hexdigest() + '\n')
    services = directory / 'services.d'
    services.mkdir()
    (services / 'log.toml').write_text('service_id = "task102-log"\nservice_uuid = "550e8400-e29b-41d4-a716-446655440102"\nallowed_topics = ["task102-log"]\n')
    # Sleet logs its configured address, not local_addr, so port zero cannot
    # discover the actual endpoint. A failed bind must fail this campaign.
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    config = directory / 'server.toml'
    config.write_text(f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task102"\nsystem_label = "Task102"\nschema_path = "{schema}"\nschema_version = "002.{release[-1]}.0"\nservices_dir = "{services}"\nsystem_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    positives = cases.positives(release)
    documents = directory / 'documents.jsonl'
    documents.write_text(''.join(json.dumps(value) + '\n' for _, value, _ in positives))
    expectations = directory / 'expectations.txt'
    expectations.write_text('\n' * (len(positives) - 1))
    transcript = []
    def exchange(client, label, command, prefix, detail=''):
        client.send(command)
        response = client.recv(timeout=5)
        transcript.append(dict(label=label, command=command, response=response))
        if not response.startswith(prefix) or detail not in response:
            raise ValueError(f'{label}: expected {prefix!r}/{detail!r}, got {response!r}')
        return response
    def initialize(client, service='task102-log', accepted=True):
        response = exchange(client, 'init-' + service, 'INIT ' + json.dumps(dict(versions=['1.0'],
            schema=f'002.{release[-1]}.0', service_id=service, verbose=True)), '+OK' if accepted else '-ERR Unsupported-Service',
            '' if accepted else 'unknown service_id unauthorized')
        if accepted:
            info = client.recv(timeout=5)
            assert info.startswith('INFO ') and json.loads(info[5:])['version'] == '1.0'
            transcript.append(dict(label='INFO', response=info))
        return response
    server_command = [str(binary), str(config)]
    (directory / 'server.command.json').write_text(json.dumps(server_command) + '\n')
    child = None
    reader = None
    errors = []
    with (directory / 'server.stdout').open('wb') as out, (directory / 'server.stderr').open('wb') as err:
        server = subprocess.Popen(server_command, stdout=out, stderr=err, env=dict(os.environ, RUST_LOG='debug', NO_COLOR='1'))
        try:
            import re
            deadline = time.monotonic() + 10
            while True:
                log = (directory / 'server.stdout').read_text()
                match = re.search(r'listening.*addr=127\.0\.0\.1:(\d+)', log)
                if match:
                    break
                assert server.poll() is None, 'Sleet exited before schema admission: ' + log
                assert time.monotonic() < deadline, 'Sleet readiness not observed'
                time.sleep(0.02)
            assert f'schema_version="002.{release[-1]}.0"' in log and 'schema loaded' in log
            url = f'ws://127.0.0.1:{match[1]}/'
            command = [str(consumer), url, f'002.{release[-1]}.0', str(documents), str(expectations)]
            (directory / 'consumer.command.json').write_text(json.dumps(command) + '\n')
            child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            lines = queue.Queue()
            def collect():
                try:
                    with (directory / 'consumer.stdout').open('w') as stream:
                        for line in child.stdout:
                            stream.write(line)
                            stream.flush()
                            lines.put(line.strip())
                except BaseException as exc:
                    errors.append(exc)
                finally:
                    lines.put(None)
            reader = threading.Thread(target=collect)
            reader.start()
            deadline = time.monotonic() + 15
            while True:
                line = lines.get(timeout=max(0.01, deadline - time.monotonic()))
                assert line is not None, 'consumer exited before subscription'
                if line == 'TASK102_SUBSCRIBED':
                    break
                assert time.monotonic() < deadline, 'Ada SUB acceptance not observed'
            with connect(url, subprotocols=['owp']) as client:
                assert client.subprotocol == 'owp'
                initialize(client)
                for label, command, prefix, detail in cases.broker_cases(release):
                    exchange(client, label, command, prefix, detail)
                for label, invalid in cases.decoder_invalid(release):
                    exchange(client, label, 'PUB task102-log ' + json.dumps({'Log': invalid}), '+OK')
                exchange(client, 'same-subscription-recovery', 'PUB task102-log ' + json.dumps({'Log': positives[0][1]}), '+OK')
            child.wait(timeout=30)
            reader.join(timeout=5)
            assert not reader.is_alive() and not errors
            (directory / 'consumer.stderr').write_text(child.stderr.read())
            (directory / 'consumer.exit').write_text(str(child.returncode) + '\n')
            output = (directory / 'consumer.stdout').read_text()
            assert child.returncode == 0, output
            assert 'TASK102_ADA_REAL_LOG_COMPLETE handlers= 7 rejected= 3' in output
            assert 'TASK102_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1' in output
            assert 'TASK102_CLEANUP_COMPLETE' in output
            published = [line.removeprefix('TASK102_PUBLISHED ') for line in output.splitlines() if line.startswith('TASK102_PUBLISHED ')]
            assert len(published) == len(positives)
            for text, (_, _, expected) in zip(published, positives, strict=True):
                assert json.loads(text) == expected, 'independent Log value oracle mismatch'
                data = json.loads(text)['MessageData'].get('Data')
                if data is not None:
                    assert data == data.upper() and bytes.fromhex(data) == bytes.fromhex(expected['MessageData']['Data'])
            received = [json.loads(line.removeprefix('TASK102_WIRE_RECEIVED '))
                        for line in output.splitlines() if line.startswith('TASK102_WIRE_RECEIVED ')]
            assert len(received) == 11 and all(set(doc) == {'Log'} for doc in received)
            assert [doc['Log'] for doc in received[:3]] == [doc for _, doc in cases.decoder_invalid(release)]
            assert received[3]['Log'] == positives[0][1]
            assert [doc['Log'] for doc in received[4:10]] == [expected for _, _, expected in positives]
            # Publish lexical variants independently, without canonicalizing via
            # the Ada encoder first. Observe broker admission and wire spelling.
            with connect(url, subprotocols=['owp']) as lexical:
                initialize(lexical)
                exchange(lexical, 'lexical-sub', 'SUB 99 Log task102-log', '+OK')
                for label, doc, expected in positives[-2:]:
                    exchange(lexical, 'broker-' + label, 'PUB task102-log ' + json.dumps({'Log': doc}), '+OK')
                    delivery = lexical.recv(timeout=5)
                    assert delivery.startswith('MSG 99 ') and json.loads(delivery.split(' ', 2)[2]) == {'Log': doc}
                    transcript.append(dict(label='broker-' + label + '-delivery', response=delivery))
                exchange(lexical, 'lexical-unsub', 'UNSUB 99', '+OK')
            with connect(url, subprotocols=['owp']) as client:
                initialize(client, 'unauthorized', accepted=False)
                try:
                    client.recv(timeout=5)
                except ConnectionClosed as exc:
                    transcript.append(dict(label='wrong-service-closed', detail=str(exc)))
                else:
                    raise ValueError('wrong service did not close')
        finally:
            (directory / 'wire.json').write_text(json.dumps(transcript, indent=2) + '\n')
            if child is not None and child.poll() is None:
                child.kill()
                child.wait(timeout=5)
            if reader is not None:
                reader.join(timeout=5)
                assert not reader.is_alive(), 'consumer output thread leaked'
            if child is not None:
                child.stdout.close()
                child.stderr.close()
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)
            (directory / 'server.exit').write_text(str(server.returncode) + '\n')
    assert server.returncode == 0, 'unclean server close'
    log = (directory / 'server.stdout').read_text()
    assert 'route PUB message' in log and 'message_name=Log' in log
    assert 'shutdown complete' in log
    with socket.socket() as probe:
        probe.settimeout(1)
        assert probe.connect_ex(('127.0.0.1', port)) != 0, 'broker socket retained'
    result = dict(completed=True, release=release, revision=REV, tree=TREE, handlers=7,
                  witness_handlers=1, decode_rejected=3, published=6, broker_rejected=10,
                  consumer_exit=0, server_exit=0, cleanup=True)
    (directory / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print('TASK102_REAL_SLEET_CELL_COMPLETE', release, 'handlers=7 decode_rejected=3 witness=1', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['checkout', 'consumer', 'schema', 'evidence']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--release', choices=['25', '26'], required=True)
    args = parser.parse_args()
    execute(args.checkout.resolve(), args.consumer.resolve(), args.schema.resolve(), args.release, args.evidence.resolve())