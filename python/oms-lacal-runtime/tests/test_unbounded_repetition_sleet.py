"""PY-012 synthetic interoperability, explicitly separate from real UCI readiness."""
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


class UnboundedSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_typed_delivery_decoder_recovery_and_teardown(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'pinned Sleet required, no skip')
        checkout = Path(binary).resolve().parents[2]
        self.assertEqual(subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
            ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b', '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        with TemporaryDirectory(prefix='py012-sleet-') as temp:
            path = Path(temp)
            schema = ROOT / 'tests/fixtures/python-unbounded-repetition/sleet.xsd'
            generator = os.environ['AMS_GRA_GENERATOR']
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                            '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(path / 'sequence_models')],
                           check=True, capture_output=True)
            sys.path.insert(0, temp)
            self.addCleanup(sys.path.remove, temp)
            m = importlib.import_module('sequence_models.model')
            c = importlib.import_module('sequence_models.service_codec')
            api = importlib.import_module('sequence_models.service_api')
            for name in ['sequence_models', 'sequence_models.model', 'sequence_models.service_codec', 'sequence_models.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            decoder_calls = []

            def observed_decoder(text):
                decoder_calls.append(text)
                return c.decode_Payload(text)

            registry = CodecRegistry([CodecBinding(OAM, 'SequenceNotice', m.Payload, c.encode_Payload,
                                                  observed_decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-unbounded"\nservice_uuid="550e8400-e29b-41d4-a716-446655440012"\n'
                'allowed_topics=["sequence.loop"]\n[[topic_bindings]]\ntopic="sequence.loop"\n'
                'allowed_messages=["SequenceNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py012-test"\n'
                              f'system_label="Python Sequences"\nschema_path={json.dumps(str(schema))}\n'
                              'schema_version="002.5.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                              f'services_dir={json.dumps(str(services))}\n')
            observations = []
            evidence_path = Path(os.environ['AMS_GRA_PY012_EVIDENCE']) if os.environ.get('AMS_GRA_PY012_EVIDENCE') else None
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
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-unbounded', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def handler(payload: m.Payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.required), tuple)
                            self.assertIs(type(payload.optional), tuple)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, handler)
                        for optional in [(), ('single',), ('last', ' first ', 'last', 'λ')]:
                            value = m.Payload(required=('a', 'b', 'a'), optional=optional,
                                              times=('2000-02-29T00:00:00.1200Z',))
                            await api.function_loop.exchange_output.publish(publisher, value)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        # Explicit empty array is an authored raw JSON transport input.
                        await publisher._operation('PUB sequence.loop {"SequenceNotice":{"Required":["single"],"Optional":[]}}')
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), m.Payload(required=('single',)))
                        for malformed in ['{}', '{"Required":[]}', '{"Required":null}',
                                          '{"Required":"scalar"}', '{"Required":["a",3,"b"]}',
                                          '{"Required":["a"],"Optional":null}',
                                          '{"Required":["a"],"Optional":["a",false]}',
                                          '{"Required":["a"],"extra":[]}',
                                          '{"Required":["a"],"Times":["2023-02-29T00:00:00Z"]}']:
                            before = len(decoder_calls)
                            try:
                                await publisher._operation('PUB sequence.loop {"SequenceNotice":' + malformed + '}')
                            except ServerRejection as error:
                                self.assertEqual(len(decoder_calls), before)
                                observations.append(dict(payload=malformed, outcome='broker rejection', code=error.code))
                            else:
                                event = await asyncio.wait_for(subscriber.next_event(), 5)
                                self.assertIs(type(event.error), CodecFailure)
                                self.assertEqual(len(decoder_calls), before + 1)
                                observations.append(dict(payload=malformed, outcome='generated decoder rejection',
                                                         decoder_input=decoder_calls[-1]))
                            self.assertTrue(received.empty())
                            await api.function_loop.exchange_output.publish(publisher, value)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        self.assertTrue(any(row['outcome'] == 'generated decoder rejection' for row in observations),
                                        'required malformed transport-to-decoder coverage did not execute')
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    print('PY012 REJECTION ATTRIBUTION: ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    if evidence_path:
                        evidence_path.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', evidence_path / 'broker.log')
                        shutil.copytree(path / 'sequence_models', evidence_path / 'generated')
                        (evidence_path / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])
            print('PY012 PINNED SLEET UNBOUNDED REPETITION: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()