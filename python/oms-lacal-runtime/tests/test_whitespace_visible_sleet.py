"""Synthetic whitespace-visible transport; instrumented decoder distinguishes rejection sites."""
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


class WhitespaceVisibleSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_whitespace_visible_delivery_recovery_and_teardown(self):
        for world in ['closed-schema', 'open-extensions']:
            await self.exercise_world(world)

    async def exercise_world(self, world):
        choice = os.environ['AMS_GRA_SLEET_BIN']
        checkout = Path(choice).resolve().parents[2]
        self.assertEqual(subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
            ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b', '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        with TemporaryDirectory(prefix='py016-sleet-') as temporary:
            path = Path(temporary)
            schema = ROOT / 'tests/fixtures/python-whitespace-visible/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            package = 'whitespace_models_' + world.replace('-', '_')
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
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
                return c.decode_Payload(text)

            registry = CodecRegistry([CodecBinding(OAM, 'WhitespaceNotice', m.Payload, c.encode_Payload,
                                                   observed_decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-whitespace-visible"\nservice_uuid="550e8400-e29b-41d4-a716-446655440016"\n'
                'allowed_topics=["whitespace.loop"]\n[[topic_bindings]]\ntopic="whitespace.loop"\n'
                'allowed_messages=["WhitespaceNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py016-test"\n'
                f'system_label="Python Whitespace Visible"\nschema_path={json.dumps(str(schema))}\n'
                'schema_version="002.5.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                f'services_dir={json.dumps(str(services))}\n')
            observations = []
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([choice, str(config)], stdout=log, stderr=log,
                                          env={**os.environ, 'RUST_LOG': 'sleet=debug'})
                try:
                    async with asyncio.timeout(15):
                        while True:
                            self.assertIsNone(broker.poll(), 'broker exited before connection')
                            try:
                                _, writer = await asyncio.open_connection('127.0.0.1', port)
                                writer.close()
                                await writer.wait_closed()
                                break
                            except ConnectionRefusedError:
                                await asyncio.sleep(.05)
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-whitespace-visible', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def handler(payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.required), m.FoldShort)
                            self.assertIs(type(payload.positive), tuple)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, handler)
                        def sample(text=' a\t b ', optional=None):
                            return m.Payload(required=m.FoldShort(text), preserved=m.KeepShort(' a\n\r  b '),
                                optional=optional, positive=(m.Leaf(''), m.Leaf(' x ')),
                                inherited=(m.FoldLong(''),), bounded=(m.KeepLong('a\nb'),),
                                detail=m.Detail(text=m.KeepEmpty('')),
                                repeated=(m.Outer.Nested(m.Selection.Fold(m.Leaf(' a\t b '))),
                                          m.Outer.Nested(m.Selection.Record(m.Detail(text=m.KeepEmpty(' y '))))))

                        for text in ['', '\r\n\t', ' a\t b ', ' \t'*4000+'a'+('\n'*4000)+'b']:
                            for optional in [None, m.KeepEmpty(''), m.KeepEmpty(' a\n\r  b ')]:
                                value = sample(text, optional)
                                body = json.loads(c.encode_Payload(value))
                                self.assertEqual(body['Required'], value.required.value)
                                self.assertEqual(body['Preserved'], ' a\n\r  b ')
                                before = len(decoder_calls)
                                await api.function_loop.exchange_output.publish(publisher, value)
                                self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                                self.assertEqual(len(decoder_calls), before + 1)
                        for invalid in ['', '\t', '\x00', '\x0b', '\x0c', '\x7f', '\u00a0']:
                            with self.assertRaises(ValueError):
                                m.KeepShort(invalid)
                        valid_body = json.loads(c.encode_Payload(sample()))
                        malformed = [json.dumps({**valid_body, 'Preserved': invalid}) for invalid in
                            ['', '\t', '\x00', '\x0b', '\x0c', '\x7f', '\u00a0', None, 1, []]]
                        malformed += [json.dumps({**valid_body, 'Required': invalid}) for invalid in
                            ['\x00', '\x0b', '\x0c', '\x7f', '\u00a0']]
                        malformed += [json.dumps({**valid_body, 'Repeated':
                            [{'Nested': {'Fold': '\x00'}}]})]
                        malformed.append(json.dumps({key: value for key, value in valid_body.items()
                                                     if key != 'Required'}))
                        malformed.append(json.dumps({**valid_body, 'Unknown': 'x'}))
                        malformed.append(json.dumps(valid_body).replace(
                            '"Required": "a b"', '"Required":"a b","Required":"a b"'))
                        for body in malformed:
                            before = len(decoder_calls)
                            try:
                                await publisher._operation('PUB whitespace.loop {"WhitespaceNotice":' + body + '}')
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
                            recovery = sample()
                            await api.function_loop.exchange_output.publish(publisher, recovery)
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
                    print('PY016 SLEET OBSERVATIONS ' + world + ': ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY016_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence) / world
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / package, destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])


if __name__ == '__main__':
    unittest.main()