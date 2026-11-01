"""Actual combined Ada execution against unchanged pinned Sleet, not a mock."""
import argparse
import json
import os
from pathlib import Path
import queue
import re
import socket
import subprocess
import threading
import time

from websockets.sync.client import connect

import admission
import cases


def execute(directory, checkout, schema, consumer):
    directory.mkdir(parents=True, exist_ok=False)
    identity = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True)
    if identity.splitlines() != [admission.REV, admission.TREE] or subprocess.check_output(
            ['git', '-C', str(checkout), 'status', '--porcelain']):
        raise ValueError('wrong/dirty Sleet source')
    for name, pin in admission.PINS.items():
        if admission.digest(schema.parent / name) != pin:
            raise ValueError('changed authentic schema: ' + name)
    (directory / 'sleet-source.txt').write_text(identity)
    binary = checkout / 'target/release/sleet'
    (directory / 'sleet-binary.sha256').write_text(admission.digest(binary) + '\n')
    services = directory / 'services.d'
    services.mkdir()
    (services / 'multimessage.toml').write_text(
        f'service_id = "{admission.SERVICE}"\nservice_uuid = "550e8400-e29b-41d4-a716-446655440106"\n'
        'allowed_topics = ["task106-log", "task106-rdma", "task106-survivability"]\n')
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    config = directory / 'server.toml'
    config.write_text(
        f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task106"\nsystem_label = "Task106"\n'
        f'schema_path = "{schema}"\nschema_version = "002.5.0"\nservices_dir = "{services}"\n'
        'system_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    plan = cases.corpus()
    documents = directory / 'plan.jsonl'
    documents.write_text(''.join(json.dumps(item) + '\n' for item in plan))
    transcript = []
    observations = []
    child = reader = None
    thread_errors = []
    result = dict(completed=False, cleanup=False)
    server_command = [str(binary), str(config)]
    (directory / 'server.command.json').write_text(json.dumps(server_command) + '\n')
    with (directory / 'server.stdout').open('wb') as out, (directory / 'server.stderr').open('wb') as err:
        server = subprocess.Popen(server_command, stdout=out, stderr=err,
                                  env=dict(os.environ, RUST_LOG='debug', NO_COLOR='1'))
        try:
            deadline = time.monotonic() + 30
            while 'listening' not in (directory / 'server.stdout').read_text():
                if server.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError('Sleet startup failure')
                time.sleep(0.02)
            command = [str(consumer), f'ws://127.0.0.1:{port}/', str(documents)]
            (directory / 'consumer.command.json').write_text(json.dumps(command) + '\n')
            child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            lines = queue.Queue()

            def collect():
                try:
                    with (directory / 'consumer.stdout').open('w') as stream:
                        for line in child.stdout:
                            stream.write(line)
                            stream.flush()
                            lines.put(line.rstrip('\n'))
                except BaseException as error:
                    thread_errors.append(str(error))
                finally:
                    lines.put(None)

            reader = threading.Thread(target=collect)
            reader.start()

            def await_line(prefix):
                deadline = time.monotonic() + 15
                while True:
                    line = lines.get(timeout=max(0.01, deadline - time.monotonic()))
                    if line is None or line.startswith('TASK106_FAILED'):
                        raise ValueError('consumer failed before ' + prefix + ': ' + str(line))
                    if line.startswith(prefix):
                        return line
                    if time.monotonic() > deadline:
                        raise TimeoutError('consumer marker timeout: ' + prefix)

            await_line('TASK106_SUBSCRIBED')
            with connect(f'ws://127.0.0.1:{port}/', subprotocols=['owp']) as publisher:
                if publisher.subprotocol != 'owp':
                    raise ValueError('publisher OWP negotiation')
                transcript.append(dict(connection='publisher-witness', subprotocol='owp'))

                def send(command):
                    transcript.append(dict(send=command))
                    publisher.send(command)

                def receive(timeout=5):
                    frame = publisher.recv(timeout=timeout)
                    transcript.append(dict(receive=frame))
                    return frame

                send('INIT ' + json.dumps(dict(versions=['1.0'], schema='002.5.0', service_id=admission.SERVICE, verbose=True)))
                if receive() != '+OK':
                    raise ValueError('publisher INIT')
                info = receive()
                if not info.startswith('INFO ') or json.loads(info[5:])['version'] != '1.0':
                    raise ValueError('publisher INFO')
                for index, key in enumerate(cases.MESSAGES, 101):
                    send(f'SUB {index} {cases.MESSAGES[key]} {cases.TOPICS[key]}')
                    if receive() != '+OK':
                        raise ValueError('witness SUB')
                for label, command, prefix, detail in cases.broker_cases():
                    send(command)
                    response = receive()
                    if not response.startswith(prefix) or detail not in response:
                        raise ValueError(f'wrong rejection {label}: {response}')
                    observations.append(dict(label=label, stage='OWP authorization/state rejection' if
                                             prefix == '-ERR Illegal-State' else 'Broker/XSD rejection', response=response))
                for index, item in enumerate(plan, 1):
                    await_line(f'TASK106_READY {index}')
                    key = item['function']
                    if item['action'] == 'external':
                        send(f'PUB {cases.TOPICS[key]} ' + json.dumps({cases.MESSAGES[key]: item['document']}))
                        if receive() != '+OK':
                            raise ValueError('independent publication rejected')
                    if item['action'] != 'remove':
                        witness = receive()
                        sid = 101 + list(cases.MESSAGES).index(key)
                        # The broker forwards original external lexical spellings;
                        # generated publications must match the independent canonical oracle.
                        document = item['document'] if item['action'] != 'publish' else item['expected']
                        prefix = f'MSG {sid} '
                        if not witness.startswith(prefix) or json.loads(witness[len(prefix):]) != {cases.MESSAGES[key]: document}:
                            raise ValueError('witness global/SID/value/order mismatch: ' + witness)
                    done = await_line(f'TASK106_DONE {index} ')
                    observations.append(dict(step=index, action=item['action'], function=key, result=done,
                                             stage='Generated Ada codec rejection' if 'error' in item else 'positive'))
                for sid in (101, 102, 103):
                    send(f'UNSUB {sid}')
                    if receive() != '+OK':
                        raise ValueError('witness cleanup')
            child.wait(timeout=15)
            reader.join(timeout=5)
            (directory / 'consumer.stderr').write_text(child.stderr.read())
            (directory / 'consumer.exit').write_text(str(child.returncode) + '\n')
            output = (directory / 'consumer.stdout').read_text()
            if child.returncode or reader.is_alive() or thread_errors:
                raise ValueError('consumer/thread failure')
            counts = cases.expected_counts()
            marker = 'TASK106_COMPLETE counts' + ''.join(' ' + str(counts[k]) for k in cases.MESSAGES) + ' errors ' + str(cases.expected_errors())
            if marker not in output or 'TASK106_CLEANUP_COMPLETE lost_events=0' not in output:
                raise ValueError('missing exact authored counts/cleanup')
            deliveries = [line for line in output.splitlines() if line.startswith('TASK106_DELIVERY')]
            expected_deliveries = [item for item in plan if item['action'] in ('publish', 'external')]
            if len(deliveries) != len(expected_deliveries):
                raise ValueError('missing/duplicate Ada delivery')
            for line, item in zip(deliveries, expected_deliveries, strict=True):
                _, sid, name, document = line.split(' ', 3)
                key = item['function']
                expected = item['expected'] if item['action'] == 'publish' else item['document']
                if int(sid) != 1 + list(cases.MESSAGES).index(key) or name != cases.MESSAGES[key] or json.loads(document) != {name: expected}:
                    raise ValueError('actual Ada routing/document mismatch')
            if output.count('TASK106_OWNERSHIP_REJECTED') != 9 or output.count('TASK106_DECODE_ERROR') != cases.expected_errors():
                raise ValueError('missing isolation controls')
            result.update(completed=True, counts=counts, decode_errors=cases.expected_errors(),
                          broker_rejections=len(cases.broker_cases()), steps=len(plan),
                          consumer_exit=child.returncode, witness_deliveries=cases.expected_witnesses())
        except Exception as error:
            result['error'] = f'{type(error).__name__}: {error}'
        finally:
            if child is not None:
                if child.poll() is None:
                    child.kill(); child.wait(timeout=5)
                if reader is not None:
                    reader.join(timeout=5)
                if not (directory / 'consumer.stderr').exists():
                    (directory / 'consumer.stderr').write_text(child.stderr.read())
                (directory / 'consumer.exit').write_text(str(child.returncode) + '\n')
                child.stdout.close(); child.stderr.close()
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                result['forced_kill'] = True
                server.kill(); server.wait(timeout=5)
            (directory / 'server.exit').write_text(str(server.returncode) + '\n')
    with socket.socket() as probe:
        probe.settimeout(1)
        socket_closed = probe.connect_ex(('127.0.0.1', port)) != 0
    log = (directory / 'server.stdout').read_text()
    registrations = re.findall(r'connection\{id=(\d+)[^\n]*subscription registered sid=([123]) topic=(\S+) message_name=(\S+)', log)
    if len(registrations) != 3 or len({r[0] for r in registrations}) != 1:
        result['error'] = 'three Ada subscriptions did not share one broker connection'
    ada_id = registrations[0][0] if registrations else '?'
    ada_unsubs = re.findall(r'connection\{id=' + re.escape(ada_id) + r'[^\n]*recv operation="UNSUB"', log)
    if len(ada_unsubs) != 3:
        result['error'] = 'unexpected ownership/cleanup wire unsubscribe count'
    result['cleanup'] = (server.returncode == 0 and socket_closed and 'shutdown complete' in log
                         and reader is not None and not reader.is_alive() and not thread_errors)
    result.update(server_exit=server.returncode, ada_connection_id=ada_id, registrations=registrations)
    (directory / 'wire.json').write_text(json.dumps(transcript, indent=2) + '\n')
    (directory / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
    (directory / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
    return result['completed'] and result['cleanup'] and 'error' not in result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ('evidence', 'checkout', 'schema', 'consumer'):
        parser.add_argument('--' + argument, required=True, type=Path)
    args = parser.parse_args()
    raise SystemExit(0 if execute(args.evidence.resolve(), args.checkout.resolve(), args.schema.resolve(), args.consumer.resolve()) else 1)