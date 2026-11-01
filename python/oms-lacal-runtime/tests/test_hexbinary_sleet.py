"""Synthetic Binary interoperability; broker facets are not Python validation."""
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
from support import ROOT, OAM


class HexBinarySleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_binary_delivery_recovery_and_teardown(self):
        binary = os.environ['AMS_GRA_SLEET_BIN']
        checkout = Path(binary).resolve().parents[2]
        self.assertEqual(subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
            ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b', '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        with TemporaryDirectory(prefix='py013-sleet-') as temporary:
            path = Path(temporary)
            schema = ROOT / 'tests/fixtures/python-hexbinary/sleet.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                '--world', 'closed-schema', '--with-codec', '--output', str(path / 'binary_models')],
                check=True, capture_output=True)
            sys.path.insert(0, temporary)
            self.addCleanup(sys.path.remove, temporary)
            m = importlib.import_module('binary_models.model')
            c = importlib.import_module('binary_models.service_codec')
            api = importlib.import_module('binary_models.service_api')
            for name in ['binary_models', 'binary_models.model', 'binary_models.service_codec', 'binary_models.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            decoder_calls = []

            def observed_decoder(text):
                decoder_calls.append(text)
                return c.decode_Payload(text)

            registry = CodecRegistry([CodecBinding(OAM, 'BinaryNotice', m.Payload, c.encode_Payload,
                                                   observed_decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-binary"\nservice_uuid="550e8400-e29b-41d4-a716-446655440013"\n'
                'allowed_topics=["binary.loop"]\n[[topic_bindings]]\ntopic="binary.loop"\n'
                'allowed_messages=["BinaryNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py013-test"\n'
                f'system_label="Python Binary"\nschema_path={json.dumps(str(schema))}\n'
                'schema_version="002.5.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                f'services_dir={json.dumps(str(services))}\n')
            observations = []
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([binary, str(config)], stdout=log, stderr=log,
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
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-binary', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def handler(payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.required), bytes)
                            self.assertIs(type(payload.repeated), tuple)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, handler)
                        for value in [m.Payload(required=b''), m.Payload(required=b'\x00\x80\xff', optional=b'',
                                      repeated=(b'', b'\xff', b'\x00\x80'))]:
                            expected = ('{"Required":""}' if not value.required else
                                '{"Required":"0080FF","Optional":"","Repeated":["","FF","0080"]}')
                            self.assertEqual(c.encode_Payload(value), expected)
                            await api.function_loop.exchange_output.publish(publisher, value)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                            self.assertEqual(json.loads(decoder_calls[-1]), json.loads(expected))
                        await publisher._operation('PUB binary.loop {"BinaryNotice":{"Required":"80ff","Repeated":["aa","00"]}}')
                        self.assertEqual(await asyncio.wait_for(received.get(), 5),
                                         m.Payload(required=b'\x80\xff', repeated=(b'\xaa', b'\x00')))
                        # The broker's lexical-character length bug is observed,
                        # not copied into the Python octet-count validator.
                        constrained = m.Payload(required=b'', constrained=m.Exact(b'\x80\xff'))
                        self.assertEqual(c.decode_Payload('{"Required":"","Constrained":"80ff"}'), constrained)
                        try:
                            await api.function_loop.exchange_output.publish(publisher, constrained)
                        except ServerRejection as error:
                            observations.append(dict(case='valid two-octet carrier', outcome='broker rejection', code=error.code))
                        else:
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), constrained)
                            observations.append(dict(case='valid two-octet carrier', outcome='typed delivery'))
                        malformed = ['{"Required":"0"}', '{"Required":"GG"}', '{"Required":[0]}',
                                     '{"Required":"00 00"}', '{"Required":"","Repeated":["00","Z0"]}',
                                     '{"Required":"","Constrained":"80"}']
                        for body in malformed:
                            before = len(decoder_calls)
                            try:
                                await publisher._operation('PUB binary.loop {"BinaryNotice":' + body + '}')
                            except ServerRejection as error:
                                self.assertEqual(len(decoder_calls), before)
                                observations.append(dict(payload=body, outcome='broker rejection', code=error.code))
                            else:
                                event = await asyncio.wait_for(subscriber.next_event(), 5)
                                self.assertIs(type(event.error), CodecFailure)
                                self.assertEqual(len(decoder_calls), before + 1)
                                observations.append(dict(payload=body, outcome='generated decoder rejection',
                                                         decoder_input=decoder_calls[-1]))
                            self.assertTrue(received.empty())
                            recovery = m.Payload(required=b'\xff\x80')
                            await api.function_loop.exchange_output.publish(publisher, recovery)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), recovery)
                        self.assertTrue(any(row['outcome'] == 'generated decoder rejection' for row in observations),
                                        'no actual transport-to-decoder rejection')
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    print('PY013 SLEET OBSERVATIONS: ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY013_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence)
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / 'binary_models', destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])


if __name__ == '__main__':
    unittest.main()