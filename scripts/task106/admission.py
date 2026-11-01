"""Independent Task106 three-message admission probe; no Ada decoder is used."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import xml.etree.ElementTree as ET

from websockets.sync.client import connect

ROOT = Path(__file__).resolve().parents[2]
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
SERVICE = 'task106-multimessage'
PINS = {
    'UCI_MessageDefinitions_v2_5_0.xsd': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
    'UCI_SecurityMarkings_v2_5_0.xsd': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
}
MESSAGES = (
    ('Log', 'task106-log', 'task093-real-log25.json'),
    ('RDMA_InitializeSetup', 'task106-rdma', 'task092-real-rdma25.json'),
    ('SurvivabilityRiskLevel', 'task106-survivability', 'task092-real-survivability25.json'),
)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(directory, sleet, schema_source):
    directory.mkdir(parents=True, exist_ok=False)
    identity = subprocess.check_output(['git', '-C', str(sleet), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True)
    if identity.splitlines() != [REV, TREE] or subprocess.check_output(
            ['git', '-C', str(sleet), 'status', '--porcelain']):
        raise ValueError('unqualified or dirty Sleet source')
    (directory / 'sleet-source.txt').write_text(identity)
    binary = sleet / 'target/release/sleet'
    (directory / 'sleet-binary.sha256').write_text(digest(binary) + '\n')
    schemas = directory / 'schema'
    schemas.mkdir()
    for name, pin in PINS.items():
        if digest(schema_source.parent / name) != pin:
            raise ValueError('authentic schema pin mismatch: ' + name)
        shutil.copyfile(schema_source.parent / name, schemas / name)
    (directory / 'schema-sources.json').write_text(json.dumps(PINS, indent=2) + '\n')
    schema = schemas / 'UCI_MessageDefinitions_v2_5_0.xsd'
    ns = {'xs': 'http://www.w3.org/2001/XMLSchema'}
    root = ET.parse(schema).getroot()
    if [x.attrib['schemaLocation'] for x in root.findall('xs:include', ns)] != ['UCI_SecurityMarkings_v2_5_0.xsd'] or root.findall('xs:import', ns):
        raise ValueError('unexpected schema closure')
    for name, _, _ in MESSAGES:
        element = root.find(f"xs:element[@name='{name}']", ns)
        if element is None or element.attrib['type'].split(':')[-1] != name + 'MT':
            raise ValueError('incorrect authentic global binding: ' + name)
    goldens = {}
    for name, _, fixture in MESSAGES:
        source = ROOT / 'tests/fixtures/service-generate' / fixture
        shutil.copyfile(source, directory / fixture)
        goldens[name] = dict(body=json.loads(source.read_bytes()), text=source.read_text(), sha256=digest(source))
    (directory / 'goldens.json').write_text(json.dumps(
        {name: item['sha256'] for name, item in goldens.items()}, indent=2) + '\n')
    services = directory / 'services.d'
    services.mkdir()
    (services / 'multimessage.toml').write_text(
        f'service_id = "{SERVICE}"\nservice_uuid = "550e8400-e29b-41d4-a716-446655440106"\n'
        'allowed_topics = ["task106-log", "task106-rdma", "task106-survivability"]\n')
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    config = directory / 'server.toml'
    config.write_text(
        f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task106"\nsystem_label = "Task106"\n'
        f'schema_path = "{schema}"\nschema_version = "002.5.0"\nservices_dir = "{services}"\n'
        'system_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    wire = []
    result = dict(status='MULTI_MESSAGE_SLEET_COMPATIBILITY_BLOCKED',
                  generated_ada_decoder_executed=False, subscriptions=[], deliveries=[])

    def send(client, command):
        wire.append(dict(connection='subscriber', send=command))
        client.send(command)

    def receive(client, timeout=5):
        frame = client.recv(timeout=timeout)
        wire.append(dict(connection='subscriber', receive=frame))
        return frame

    def publish(client, index, delivered=True):
        name, topic, _ = MESSAGES[index - 1]
        # Wrap the independent body verbatim, rather than repairing it with a codec.
        send(client, f'PUB {topic} {{"{name}":' + goldens[name]['text'] + '}')
        response = receive(client)
        if response != '+OK':
            raise ValueError(f'publication rejected {name}: {response}')
        if delivered:
            frame = receive(client)
            prefix = f'MSG {index} '
            if not frame.startswith(prefix) or json.loads(frame[len(prefix):]) != {name: goldens[name]['body']}:
                raise ValueError('delivery SID/global/body mismatch: ' + frame)
            result['deliveries'].append(dict(sid=index, name=name, topic=topic))

    command = [str(binary), str(config)]
    (directory / 'server.command.json').write_text(json.dumps(dict(argv=command, cwd=str(directory)), indent=2) + '\n')
    with (directory / 'server.stdout').open('wb') as out, (directory / 'server.stderr').open('wb') as err:
        server = subprocess.Popen(command, cwd=directory, stdout=out, stderr=err,
                                  env=dict(os.environ, RUST_LOG='debug', NO_COLOR='1'))
        try:
            deadline = time.monotonic() + 30
            while 'listening' not in (directory / 'server.stdout').read_text():
                if server.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError('broker startup failed')
                time.sleep(0.02)
            if 'schema loaded' not in (directory / 'server.stdout').read_text():
                raise ValueError('schema admission not observed')
            with connect(f'ws://127.0.0.1:{port}/', subprotocols=['owp']) as client:
                if client.subprotocol != 'owp':
                    raise ValueError('OWP negotiation failed')
                wire.append(dict(connection='subscriber', subprotocol=client.subprotocol))
                send(client, 'INIT ' + json.dumps(dict(versions=['1.0'], schema='002.5.0', service_id=SERVICE, verbose=True)))
                if receive(client) != '+OK':
                    raise ValueError('INIT rejected')
                info = receive(client)
                if not info.startswith('INFO '):
                    raise ValueError('invalid INFO')
                details = json.loads(info[5:])
                if (details.get('version') != '1.0' or details.get('server_id') != 'task106'
                        or details.get('system_label') != 'Task106'
                        or details.get('uuids', {}).get('system') != '550e8400-e29b-41d4-a716-446655440000'
                        or details.get('uuids', {}).get('service') != '550e8400-e29b-41d4-a716-446655440106'):
                    raise ValueError('incorrect INFO session/service identity')
                (directory / 'info.json').write_text(json.dumps(details, indent=2) + '\n')
                for index, (name, topic, _) in enumerate(MESSAGES, 1):
                    send(client, f'SUB {index} {name} {topic}')
                    if receive(client) != '+OK':
                        raise ValueError('subscription rejected: ' + name)
                    result['subscriptions'].append(dict(sid=index, name=name, topic=topic))
                for index in (1, 2, 3, 2, 1, 3):
                    publish(client, index)
                send(client, 'UNSUB 1')
                if receive(client) != '+OK':
                    raise ValueError('UNSUB rejected')
                publish(client, 1, delivered=False)
                publish(client, 2)
                publish(client, 3)
                try:
                    unexpected = receive(client, timeout=0.2)
                except TimeoutError:
                    pass
                else:
                    raise ValueError('unexpected post-unsubscribe frame: ' + unexpected)
                for index in (2, 3):
                    send(client, f'UNSUB {index}')
                    if receive(client) != '+OK':
                        raise ValueError('survivor UNSUB rejected')
                result.update(owp_negotiated=True, init_count=1, info_validated=True,
                              survivor_deliveries=True, unsubscribed=True)
            result['websocket_closed'] = True
        except Exception as error:
            result['error'] = f'{type(error).__name__}: {error}'
        finally:
            (directory / 'wire.json').write_text(json.dumps(wire, indent=2) + '\n')
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                result['forced_kill'] = True
                server.kill()
                server.wait(timeout=5)
            (directory / 'server.exit').write_text(str(server.returncode) + '\n')
    with socket.socket() as probe:
        probe.settimeout(1)
        closed = probe.connect_ex(('127.0.0.1', port)) != 0
    result['cleanup'] = (server.returncode == 0 and closed and result.get('websocket_closed', False)
                         and 'shutdown complete' in (directory / 'server.stdout').read_text())
    if 'error' not in result and result['cleanup'] and len(result['deliveries']) == 8:
        result['status'] = 'PASS'
    (directory / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ('evidence', 'sleet', 'uci25'):
        parser.add_argument('--' + argument, required=True, type=Path)
    args = parser.parse_args()
    outcome = execute(args.evidence.resolve(), args.sleet.resolve(), args.uci25.resolve())
    raise SystemExit(0 if outcome['status'] == 'PASS' else 1)