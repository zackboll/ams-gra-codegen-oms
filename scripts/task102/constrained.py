"""Separate extracted authentic SHA-256 control, never a full Log substitute."""
import json
import os
from pathlib import Path
import shlex
import shutil
import socket
import subprocess
import sys
import time

from websockets.sync.client import connect

ROOT = Path(__file__).resolve().parents[2]


def execute(campaign, release, schema, generator, sleet, compiler):
    # Run the unchanged Task093 qualification in the already isolated setup.
    # No global toolchain/cache/trust settings are modified.
    wrapper = campaign.scratch / f'{release}-alire-wrapper'
    wrapper.mkdir()
    alr = wrapper / 'alr'
    alr.write_text('#!/bin/sh\nexec ' + shlex.quote(shutil.which('alr')) + ' -s '
                   + shlex.quote(str(campaign.setup / 'settings')) + ' "$@"\n')
    alr.chmod(0o755)
    previous = campaign.env['PATH']
    campaign.env['PATH'] = str(wrapper) + ':' + previous
    extracted = campaign.scratch / f'{release}-constrained-ada'
    try:
        campaign.run(f'{release}-constrained-ada', [sys.executable, ROOT / 'scripts/check-task093-real.py',
            '--compiler', compiler, '--version', '2.' + release[-1], '--schema', schema,
            '--scratch', extracted, '--generator', generator])
    finally:
        campaign.env['PATH'] = previous
    assert json.loads((extracted / 'qualification.json').read_text())['status'] == 'PASS'
    build = campaign.scratch / f'{release}-hash-ada'
    build.mkdir()
    for path in (extracted / 'consumer').glob('*.ad?'):
        if path.name != 'hexbinary_real_consumer.adb':
            shutil.copyfile(path, build / path.name)
    shutil.copyfile(ROOT / 'tests/ada-real-uci-log/hash_probe.adb', build / 'hash_probe.adb')
    (build / 'probe.gpr').write_text(f'with "{campaign.scratch / "support/ams_gra_oms_json"}";\nproject Probe is\nfor Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ("hash_probe.adb");\npackage Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato"); end Compiler; end Probe;\n')
    for policy in ('Normal', 'Check', 'Ignore'):
        adc = build / 'policy.adc'
        adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
        campaign.run(f'{release}-hash-{policy}-build', campaign.shell(['gprbuild', '-p', '-P', 'probe.gpr', '-cargs:Ada', '-gnato', f'-gnatec={adc}']), build)
        assert campaign.run(f'{release}-hash-{policy}-execute', [build / 'bin/hash_probe']) == 'TASK102_HASH_OCTETS_COMPLETE accepted=1 rejected=3\n'
    directory = campaign.scratch / f'{release}-constrained-broker'
    directory.mkdir()
    # Task093's synthetic envelope has no schema-version attribute. Sleet
    # requires one: add it ONLY to that synthetic wrapper, never to the
    # authentic root or any extracted declaration/member/facet.
    source = (extracted / 'real-binary.xsd').read_text()
    broker_schema = directory / 'extracted-binary.xsd'
    broker_schema.write_text(source.replace('<xs:schema ', f'<xs:schema version="002.{release[-1]}.0" ', 1))
    services = directory / 'services.d'
    services.mkdir()
    (services / 'binary.toml').write_text('service_id = "task102-constrained"\nservice_uuid = "550e8400-e29b-41d4-a716-446655440103"\nallowed_topics = ["task102-constrained"]\n')
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    config = directory / 'server.toml'
    config.write_text(f'bind_addr = "127.0.0.1:{port}"\nserver_id = "task102-constrained"\nsystem_label = "Task102 constrained probe"\nschema_path = "{broker_schema}"\nschema_version = "002.{release[-1]}.0"\nservices_dir = "{services}"\nsystem_uuid = "550e8400-e29b-41d4-a716-446655440000"\n')
    wire = []
    def exchange(client, label, command, prefix, detail=''):
        client.send(command)
        response = client.recv(timeout=5)
        wire.append(dict(label=label, command=command, response=response))
        assert response.startswith(prefix) and detail in response, response
    with (directory / 'server.stdout').open('wb') as out, (directory / 'server.stderr').open('wb') as err:
        server = subprocess.Popen([str(sleet / 'target/release/sleet'), str(config)], stdout=out, stderr=err,
                                  env=dict(os.environ, RUST_LOG='debug', NO_COLOR='1'))
        try:
            deadline = time.monotonic() + 10
            while 'listening' not in (directory / 'server.stdout').read_text():
                assert server.poll() is None and time.monotonic() < deadline
                time.sleep(0.02)
            with connect(f'ws://127.0.0.1:{port}/', subprotocols=['owp']) as client:
                exchange(client, 'init', 'INIT ' + json.dumps(dict(versions=['1.0'], schema=f'002.{release[-1]}.0', service_id='task102-constrained', verbose=True)), '+OK')
                info = client.recv(timeout=5)
                assert info.startswith('INFO ')
                wire.append(dict(label='INFO', response=info))
                exchange(client, 'sub', 'SUB 1 BinaryNotice task102-constrained', '+OK')
                for octets in (32, 16, 31, 33):
                    doc = {'BinaryNotice': {'Data': '0080FF', 'SHA_2_Hash': '0080FF' + '00' * (octets - 3)}}
                    exchange(client, f'hash-{octets}-octets', 'PUB task102-constrained ' + json.dumps(doc),
                             '+OK' if octets == 16 else '-ERR Invalid-Message', '' if octets == 16 else 'FacetViolation')
                    if octets == 16:
                        delivered = client.recv(timeout=5)
                        assert delivered.startswith('MSG 1 ') and json.loads(delivered.split(' ', 2)[2]) == doc
                        wire.append(dict(label='hash-16-delivered', response=delivered))
                exchange(client, 'unsub', 'UNSUB 1', '+OK')
        finally:
            (directory / 'wire.json').write_text(json.dumps(wire, indent=2) + '\n')
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)
            (directory / 'server.exit').write_text(str(server.returncode) + '\n')
    assert server.returncode == 0 and 'shutdown complete' in (directory / 'server.stdout').read_text()
    with socket.socket() as probe:
        probe.settimeout(1)
        assert probe.connect_ex(('127.0.0.1', port)) != 0
    (directory / 'result.json').write_text(json.dumps(dict(
        scope='unchanged extracted authentic Binary declarations; synthetic envelope, NOT Log',
        ada_accepts_octets=32, broker_accepts_octets=16, broker_rejects_octets=[32, 31, 33],
        server_exit=0, cleanup=True), indent=2) + '\n')