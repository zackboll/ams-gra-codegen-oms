"""Full unchanged pinned UCI Log transport, both releases and worlds."""
import asyncio
import importlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from oms_lacal.wire import ProtocolError
from support import ROOT, OAM


class FullUciWhitespaceVisibleSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_full_log_delivery_recovery_and_teardown(self):
        for release in ['2.5', '2.6']:
            for world in ['closed-schema', 'open-extensions']:
                await self.exercise_world(world, release)

    async def exercise_world(self, world, release):
        choice = os.environ['AMS_GRA_SLEET_BIN']
        checkout = Path(choice).resolve().parents[2]
        self.assertEqual(subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
            ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b', '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        with TemporaryDirectory(prefix='py016-sleet-') as temporary:
            path = Path(temporary)
            schema = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
            import hashlib
            import importlib.util
            spec = importlib.util.spec_from_file_location('py016_measure', ROOT / 'scripts/measure-py016-uci.py')
            measure = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(measure)
            self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), measure.driver.PINS[release][1])
            self.assertEqual(hashlib.sha256(schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd').read_bytes()).hexdigest(), measure.INCLUDES[release])
            contract = path / 'service.yaml'
            contract.write_text(measure.driver.contract('Log', release))
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            package = 'full_log_' + release.replace('.', '') + '_' + world.replace('-', '_')
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                '--contract', str(contract), '--language', 'python',
                '--world', world, '--with-codec', '--output', str(path / package)],
                check=True, capture_output=True)
            sys.path.insert(0, temporary)
            self.addCleanup(sys.path.remove, temporary)
            m = importlib.import_module(package + '.model')
            c = importlib.import_module(package + '.service_codec')
            api = importlib.import_module(package + '.service_api')
            for name in [package, package + '.model', package + '.service_codec', package + '.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            decoder_calls = []

            def observed_decoder(text):
                decoder_calls.append(text)
                return c.decode_LogMT(text)

            registry = CodecRegistry([CodecBinding(OAM, 'Log', m.LogMT, c.encode_LogMT,
                                                   observed_decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-whitespace-visible"\nservice_uuid="550e8400-e29b-41d4-a716-446655440016"\n'
                'allowed_topics=["py006.selected"]\n[[topic_bindings]]\ntopic="py006.selected"\n'
                'allowed_messages=["Log"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py016-test"\n'
                f'system_label="Python Whitespace Visible"\nschema_path={json.dumps(str(schema))}\n'
                f'schema_version="002.{release[-1]}.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                f'services_dir={json.dumps(str(services))}\n')
            observations = []
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([choice, str(config)], stdout=log, stderr=log,
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
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-whitespace-visible', '002.' + release[-1] + '.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def handler(payload):
                            self.assertIs(type(payload), m.LogMT)
                            self.assertIs(type(payload.securityinformation.classifiedby), m.WhitespaceVisibleString1024Type)
                            self.assertEqual(len(payload.securityinformation.ownerproducer), 2)
                            received.put_nowait(payload)

                        handle = await subscriber.subscribe(message_namespace=OAM, message_name='Log', topic='py006.selected', subscription_group=None, handler=handler)
                        golden = json.loads((ROOT / 'tests/fixtures/python-whitespace-visible/goldens' / (release + '-Log.json')).read_text())
                        value = c.decode_LogMT(json.dumps(golden))
                        before = len(decoder_calls)
                        await api.function_selected.exchange_output.publish(publisher, value)
                        actual = await asyncio.wait_for(received.get(), 10)
                        self.assertEqual(actual, value)
                        self.assertEqual(len(decoder_calls), before + 1)
                        self.assertEqual(json.loads(c.encode_LogMT(actual)), golden)
                        malformed = []
                        for invalid in ['\x00', '\x0b', '\u00a0'] + (['', '\t'] if release == '2.6' else []):
                            document = json.loads(json.dumps(golden))
                            document['SecurityInformation']['ClassifiedBy'] = invalid
                            malformed.append(json.dumps(document))
                        for body in malformed:
                            before = len(decoder_calls)
                            try:
                                await publisher._operation('PUB py006.selected {"Log":' + body + '}')
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
                                    observations.append(dict(payload=body, outcome='generated decoder rejection',
                                                             decoder_input=decoder_calls[-1]))
                            self.assertTrue(received.empty())
                            recovery = value
                            await api.function_selected.exchange_output.publish(publisher, recovery)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), recovery)
                        # The unchanged broker can reject lexical violations
                        # before delivery. Do not invent decoder failures when
                        # instrumentation shows that the decoder was not called.
                        self.assertGreater(len(decoder_calls), 0)
                        self.assertTrue(any(row['outcome'] == 'broker rejection' for row in observations))
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    print('PY016 FULL LOG SLEET OBSERVATIONS ' + release + ' ' + world + ': ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY016_FULL_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence) / release / world
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / package, destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])


if __name__ == '__main__':
    unittest.main()