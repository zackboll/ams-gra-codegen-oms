"""PY-017 generated endpoints and independent OWP observer, unchanged pinned broker."""
import asyncio
import copy
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from websockets.asyncio.client import connect
from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from oms_lacal.wire import ProtocolError
from support import ROOT, OAM


class NetworkAddressesSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_synthetic_delivery_recovery_observer_and_teardown(self):
        for world in ['closed-schema', 'open-extensions']:
            await self.exercise(world)

    async def test_authentic_rdma_delivery_recovery_observer_and_teardown(self):
        for release in ['2.5', '2.6']:
            for world in ['closed-schema', 'open-extensions']:
                await self.exercise(world, release)

    async def exercise(self, world, release=None):
        broker_binary = Path(os.environ['AMS_GRA_SLEET_BIN']).resolve()
        checkout = broker_binary.parents[2]
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'rev-parse',
                         'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
                         ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b',
                          '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        version = release or '2.5'
        message = 'RDMA_InitializeSetup' if release else 'NetworkNotice'
        payload_name = 'RDMA_InitializeSetupMT' if release else 'Payload'
        fixture = ROOT / 'tests/fixtures/python-network-addresses'
        with TemporaryDirectory(prefix='py017-sleet-') as temporary:
            path = Path(temporary)
            contract = fixture / 'service.yaml'
            schema = fixture / 'oam.xsd'
            if release:
                schema = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
                spec = importlib.util.spec_from_file_location('py017_measure', ROOT / 'scripts/measure-py017-uci.py')
                measure = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(measure)
                self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), measure.driver.PINS[release][1])
                self.assertEqual(hashlib.sha256(schema.with_name(
                    f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd').read_bytes()).hexdigest(), measure.INCLUDES[release])
                contract = ROOT / 'tests/fixtures/task103' / ('uci' + release.replace('.', '') + '-service.yaml')
            package = 'network_' + version.replace('.', '') + '_' + world.replace('-', '_')
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema), '--contract', str(contract),
                '--language', 'python', '--world', world, '--with-codec', '--output', str(path / package)],
                check=True, capture_output=True)
            sys.path.insert(0, temporary)
            self.addCleanup(sys.path.remove, temporary)
            m, c, api = [importlib.import_module(package + '.' + name) for name in ['model', 'service_codec', 'service_api']]
            for name in [package, package + '.model', package + '.service_codec', package + '.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            encode, decode = getattr(c, 'encode_' + payload_name), getattr(c, 'decode_' + payload_name)
            if release:
                # This helper explicitly constructs generated types independently
                # of decoding and does not execute its unittest main on import.
                saved = sys.argv
                try:
                    sys.argv = ['rdma_probe', str(path / package), 'package', release.replace('.', ''),
                        str(ROOT / 'tests/fixtures/service-generate' / ('task092-real-rdma' + release.replace('.', '') + '.json'))]
                    spec = importlib.util.spec_from_file_location('py017_typed', fixture / 'rdma_probe.py')
                    probe = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(probe)
                finally:
                    sys.argv = saved
                sample = probe.typed
                v4, v6 = sample().messagedata.endpoint
                values = [sample(sequence) for sequence in [(v4,), (v6,), (v4, v6), (v6, v4), (v4, v6, v4), ()]]
                endpoint_in, endpoint_out = api.function_rdma.exchange_receive, api.function_rdma.exchange_publish
                topic = 'task103-rdma'
            else:
                v4 = m.NetworkEndpointType.IPv4(m.IPv4_EndpointType(ipv4_address=m.Leaf('198.51.100.42')))
                v6 = m.NetworkEndpointType.IPv6(m.IPv6_EndpointType(ipv6_address=m.IPv6_AddressType('fE80::aBcD')))
                def sample(sequence=(v4, v6)):
                    return m.Payload(required=m.IPv4_AddressType('1.2.3.4'), selected=v4,
                        positive=(m.IPv6_AddressType(':001.009.099.199'),), endpoint=sequence)
                values = [sample(sequence) for sequence in [(v4,), (v6,), (v4, v6), (v6, v4), (v4, v6, v4), ()]]
                endpoint_in, endpoint_out = api.function_loop.exchange_input, api.function_loop.exchange_output
                topic = 'network.loop'
            decoder_calls = []
            def observed_decoder(text):
                decoder_calls.append(text)
                return decode(text)
            registry = CodecRegistry([CodecBinding(OAM, message, getattr(m, payload_name), encode, observed_decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-network"\nservice_uuid="550e8400-e29b-41d4-a716-446655440017"\n'
                f'allowed_topics=["{topic}"]\n[[topic_bindings]]\ntopic="{topic}"\nallowed_messages=["{message}"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py017-test"\n'
                f'system_label="Python Network Addresses"\nschema_path={json.dumps(str(schema))}\n'
                f'schema_version="002.{version[-1]}.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                f'services_dir={json.dumps(str(services))}\n')
            observations = []
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([str(broker_binary), str(config)], stdout=log, stderr=log,
                                          env={**os.environ, 'RUST_LOG': 'sleet=debug'})
                try:
                    async with asyncio.timeout(120):
                        while True:
                            self.assertIsNone(broker.poll(), 'broker exited before connection')
                            try:
                                _, writer = await asyncio.open_connection('127.0.0.1', port)
                                writer.close()
                                await writer.wait_closed()
                                break
                            except ConnectionRefusedError:
                                await asyncio.sleep(.05)
                    url = f'ws://127.0.0.1:{port}'
                    runtime_config = RuntimeConfig(url, 'python-network', '002.' + version[-1] + '.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher, \
                            connect(url, subprotocols=['owp']) as observer:
                        # Independent wire subscriber uses no generated/runtime
                        # protocol helpers and verifies original message identity.
                        await observer.send('INIT ' + json.dumps(dict(versions=['1.0'], schema='002.' + version[-1] + '.0',
                            service_id='python-network', verbose=True)))
                        self.assertEqual(await observer.recv(), '+OK')
                        self.assertTrue((await observer.recv()).startswith('INFO '))
                        await observer.send(f'SUB independent {message} {topic}')
                        self.assertEqual(await observer.recv(), '+OK')
                        received = asyncio.Queue()
                        def handler(payload):
                            self.assertIs(type(payload), getattr(m, payload_name))
                            received.put_nowait(payload)
                        handle = await endpoint_in.subscribe(subscriber, handler)
                        for value in values:
                            before = len(decoder_calls)
                            await endpoint_out.publish(publisher, value)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                            self.assertEqual(len(decoder_calls), before + 1)
                            frame = await asyncio.wait_for(observer.recv(), 5)
                            self.assertTrue(frame.startswith('MSG independent '), frame)
                            self.assertEqual(json.loads(frame.split(' ', 2)[2]), {message: json.loads(encode(value))})
                        # Remove observer before malformed traffic so decoder
                        # rejection attribution cannot be confused with its queue.
                        await observer.send('UNSUB independent')
                        self.assertEqual(await observer.recv(), '+OK')
                        valid = json.loads(encode(values[2]))
                        malformed = []
                        for text, member, index in [('256.0.0.1', 'IPv4', 0), ('01.2.3.4', 'IPv4', 0),
                                                    ('[::1]', 'IPv6', 1), ('::ffff:256.0.2.128', 'IPv6', 1)]:
                            doc = copy.deepcopy(valid)
                            if release:
                                doc['MessageData']['Endpoint'][index][member + '_Endpoint'][member + '_Address'] = text
                            else:
                                doc['Endpoint'][index][member][member + '_Address'] = text
                            malformed.append(json.dumps(doc))
                        doc = copy.deepcopy(valid)
                        (doc['MessageData'] if release else doc)['Endpoint'] = [{}]
                        malformed.append(json.dumps(doc))
                        for body in malformed:
                            before = len(decoder_calls)
                            try:
                                await publisher._operation('PUB ' + topic + ' {' + json.dumps(message) + ':' + body + '}')
                            except ServerRejection as error:
                                self.assertEqual(len(decoder_calls), before)
                                observations.append(dict(payload=body, outcome='broker rejection', code=error.code))
                            else:
                                event = await asyncio.wait_for(subscriber.next_event(), 5)
                                if type(event.error) is ProtocolError:
                                    self.assertEqual(len(decoder_calls), before)
                                    observations.append(dict(payload=body, outcome='runtime envelope rejection'))
                                else:
                                    self.assertIs(type(event.error), CodecFailure)
                                    self.assertEqual(len(decoder_calls), before + 1)
                                    observations.append(dict(payload=body, outcome='generated decoder rejection', decoder_input=decoder_calls[-1]))
                            self.assertTrue(received.empty())
                            await endpoint_out.publish(publisher, values[2])
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), values[2])
                        self.assertTrue(any(row['outcome'] == 'broker rejection' for row in observations))
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        await endpoint_out.publish(publisher, values[2])
                        await asyncio.sleep(.1)
                        self.assertTrue(received.empty())
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    print('PY017 SLEET ' + message + ' ' + version + ' ' + world + ': ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY017_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence) / message / version / world
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / package, destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])


if __name__ == '__main__':
    unittest.main()