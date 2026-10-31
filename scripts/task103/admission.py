"""Task103 mandatory authentic admission gate, adapted from Task102's probe.

This is an independent broker probe, not a generated Ada consumer or a codec test.
The Task092 golden is wrapped verbatim, never canonicalized or repaired.
"""
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import uuid
import xml.etree.ElementTree as ET

from websockets.sync.client import connect

ROOT = Path(__file__).resolve().parents[2]
MESSAGE = 'RDMA_InitializeSetup'
TOPIC = 'task103-rdma'
PINS = {
    '25': ('ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
           '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b'),
    '26': ('af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
           'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'),
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(release, base, sleet, schema_source):
    directory = base / f'phase-a-{release}'
    directory.mkdir()
    identity = subprocess.check_output(
        ['git', '-C', str(sleet), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True)
    assert identity.splitlines() == [
        'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b',
        '7e705f4eb5f658957bd414b4abc8207311f3c1e4']
    assert not subprocess.check_output(['git', '-C', str(sleet), 'status', '--porcelain'])
    (directory / 'sleet-source.txt').write_text(identity)
    binary = sleet / 'target/release/sleet'
    (directory / 'sleet-binary.sha256').write_text(digest(binary) + '\n')
    schemas = directory / 'schema'
    schemas.mkdir()
    source = schema_source.parent
    names = [f'UCI_{kind}_v2_{release[-1]}_0.xsd'
             for kind in ('MessageDefinitions', 'SecurityMarkings')]
    for name, pin in zip(names, PINS[release], strict=True):
        assert digest(source / name) == pin
        shutil.copyfile(source / name, schemas / name)
    schema = schemas / names[0]
    (directory / 'schema-sources.json').write_text(json.dumps(
        {name: digest(schemas / name) for name in names}, indent=2) + '\n')
    tree = ET.parse(schema).getroot()
    ns = {'xs': 'http://www.w3.org/2001/XMLSchema'}
    includes = tree.findall('xs:include', ns)
    assert [node.attrib['schemaLocation'] for node in includes] == [names[1]]
    assert not tree.findall('xs:import', ns)
    assert not ET.parse(schemas / names[1]).getroot().findall('xs:include', ns)
    selection = [tree.find(f"xs:element[@name='{MESSAGE}']", ns)]
    for name in ['RDMA_InitializeSetupMT', 'RDMA_InitializeSetupMDT',
                 'NetworkEndpointType', 'IPv4_EndpointType', 'IPv6_EndpointType']:
        selection.append(tree.find(f"xs:complexType[@name='{name}']", ns))
    for name in ['IPv4_AddressType', 'IPv6_AddressType', 'UniversallyUniqueIdentifierType']:
        selection.append(tree.find(f"xs:simpleType[@name='{name}']", ns))
    assert all(node is not None for node in selection)
    assert selection[0].attrib['type'].endswith(':RDMA_InitializeSetupMT')
    endpoint = selection[2].find("xs:sequence/xs:element[@name='Endpoint']", ns)
    assert endpoint is not None
    assert endpoint.attrib['minOccurs'] == '0' and endpoint.attrib['maxOccurs'] == 'unbounded'
    (directory / 'schema-rdma-selection.xml').write_text('\n'.join(
        ET.tostring(node, encoding='unicode') for node in selection) + '\n')
    golden = ROOT / f'tests/fixtures/service-generate/task092-real-rdma{release}.json'
    shutil.copyfile(golden, directory / golden.name)
    assert (directory / golden.name).read_bytes() == golden.read_bytes()
    body = json.loads(golden.read_bytes())
    assert body['MessageHeader']['SchemaVersion'] == f'002.{release[-1]}.0'
    assert body['MessageData']['RDMA_Provider'] == 'TCP'
    for value in [body['MessageHeader']['SystemID']['UUID'],
                  body['MessageData']['RDMA_InitializeSetupID']['UUID']]:
        parsed = uuid.UUID(value)
        assert parsed.int and parsed.variant == uuid.RFC_4122
    assert body['MessageData']['Endpoint'] == [
        {'IPv4_Endpoint': {'IPv4_Address': '192.0.2.10'}},
        {'IPv6_Endpoint': {'IPv6_Address': '2001:db8::1'}}]
    assert ipaddress.ip_address('192.0.2.10').version == 4
    assert ipaddress.ip_address('2001:db8::1').version == 6
    assert body['SecurityInformation']['OwnerProducer'] == [
        {'GovernmentIdentifier': 'USA'}, {'NATO_SpecialWord': 'NATO:EXAMPLE'}]
    (directory / 'golden.sha256').write_text(digest(golden) + '\n')
    services = directory / 'services.d'
    services.mkdir()
    (services / 'rdma.toml').write_text(
        'service_id = "task103-rdma"\n'
        'service_uuid = "550e8400-e29b-41d4-a716-446655440103"\n'
        'allowed_topics = ["task103-rdma"]\n')
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    config = directory / 'server.toml'
    config.write_text(
        f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task103"\n'
        f'system_label = "Task103"\nschema_path = "{schema}"\n'
        f'schema_version = "002.{release[-1]}.0"\nservices_dir = "{services}"\n'
        'system_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    wire = []
    result = dict(release=release, schema_admitted=False, owp_negotiated=False,
                  initialized=False, subscribed=False, admitted=False, delivered=False,
                  unsubscribed=False, cleanup=False, generated_ada_decoder_executed=False)

    def send(client, command):
        wire.append(dict(send=command))
        client.send(command)

    def receive(client, timeout=5):
        response = client.recv(timeout=timeout)
        wire.append(dict(receive=response))
        return response

    command = [str(binary), str(config)]
    (directory / 'server.command.json').write_text(json.dumps(command) + '\n')
    with (directory / 'server.stdout').open('wb') as out, (directory / 'server.stderr').open('wb') as err:
        server = subprocess.Popen(command, stdout=out, stderr=err,
                                  env=dict(os.environ, RUST_LOG='debug', NO_COLOR='1'))
        try:
            deadline = time.monotonic() + 30
            while 'listening' not in (directory / 'server.stdout').read_text():
                assert server.poll() is None, 'broker exited before schema admission'
                assert time.monotonic() < deadline, 'broker readiness timeout'
                time.sleep(0.02)
            assert 'schema loaded' in (directory / 'server.stdout').read_text()
            result['schema_admitted'] = True
            with connect(f'ws://127.0.0.1:{port}/', subprotocols=['owp']) as client:
                assert client.subprotocol == 'owp'
                result['owp_negotiated'] = True
                send(client, 'INIT ' + json.dumps(dict(
                    versions=['1.0'], schema=f'002.{release[-1]}.0',
                    service_id=TOPIC, verbose=True)))
                assert receive(client) == '+OK'
                info = receive(client)
                assert info.startswith('INFO ') and json.loads(info[5:])['version'] == '1.0'
                result['initialized'] = True
                send(client, f'SUB 1 {MESSAGE} {TOPIC}')
                assert receive(client) == '+OK'
                result['subscribed'] = True
                envelope = '{"' + MESSAGE + '":' + golden.read_text() + '}'
                assert json.loads(envelope) == {MESSAGE: body}
                publication = f'PUB {TOPIC} ' + envelope
                (directory / 'publication.txt').write_text(publication)
                send(client, publication)
                response = receive(client)
                result['publication_response'] = response
                result['admitted'] = response == '+OK'
                if result['admitted']:
                    delivery = receive(client)
                    assert delivery.startswith('MSG 1 '), delivery
                    received = json.loads(delivery.split(' ', 2)[2])
                    assert list(received) == [MESSAGE]
                    assert received[MESSAGE] == json.loads(golden.read_bytes())
                    (directory / 'delivered.json').write_text(json.dumps(received, indent=2) + '\n')
                    result['delivered'] = True
                else:
                    assert response.startswith('-ERR '), response
                    result['rejection_stage'] = 'Broker/XSD rejection'
                    try:
                        unexpected = receive(client, timeout=0.5)
                    except TimeoutError:
                        result['no_delivery_observation_seconds'] = 0.5
                    else:
                        raise ValueError(f'unexpected post-rejection frame: {unexpected!r}')
                send(client, 'UNSUB 1')
                assert receive(client) == '+OK'
                result['unsubscribed'] = True
            result['websocket_closed'] = True
        except Exception as exc:
            result['error'] = f'{type(exc).__name__}: {exc}'
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
    result.update(server_exit=server.returncode, socket_closed=closed, server_pid=server.pid,
                  cleanup=server.returncode == 0 and closed and result.get('websocket_closed', False)
                  and 'shutdown complete' in (directory / 'server.stdout').read_text())
    required = ['schema_admitted', 'owp_negotiated', 'initialized', 'subscribed',
                'admitted', 'delivered', 'unsubscribed', 'cleanup']
    result['status'] = ('PASS' if all(result[key] for key in required) and 'error' not in result
                        else 'REAL_RDMA_SLEET_COMPATIBILITY_BLOCKED')
    (directory / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
    return result


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['evidence', 'sleet', 'uci25', 'uci26']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    base = args.evidence.resolve()
    base.mkdir(parents=True, exist_ok=False)
    results = [execute(release, base, args.sleet.resolve(), schema.resolve())
               for release, schema in [('25', args.uci25), ('26', args.uci26)]]
    (base / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    raise SystemExit(0 if all(result['status'] == 'PASS' for result in results) else 1)
