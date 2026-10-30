"""PY-010: generated VisibleAscii models/codecs through the shared pinned Sleet."""
import asyncio
import importlib
import json
import os
import shutil
from pathlib import Path
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from support import ROOT, OAM


class VisibleAsciiSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_visible_ascii_delivery_rejection_and_recovery(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'pinned Sleet mandatory; no skip')
        with TemporaryDirectory(prefix='py010-broker-') as temp:
            path = Path(temp)
            schema = ROOT / 'tests/fixtures/python-visible-ascii/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                            '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(path / 'visible_ascii_models')],
                           check=True, capture_output=True)
            sys.path.insert(0, temp)
            self.addCleanup(sys.path.remove, temp)
            m = importlib.import_module('visible_ascii_models.model')
            c = importlib.import_module('visible_ascii_models.service_codec')
            api = importlib.import_module('visible_ascii_models.service_api')
            for name in ['visible_ascii_models', 'visible_ascii_models.model', 'visible_ascii_models.service_codec', 'visible_ascii_models.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            registry = CodecRegistry([CodecBinding(OAM, 'TextNotice', m.Payload, c.encode_Payload,
                                                  c.decode_Payload, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-visible-ascii"\nservice_uuid="550e8400-e29b-41d4-a716-446655440009"\n'
                'allowed_topics=["visible.loop"]\n[[topic_bindings]]\ntopic="visible.loop"\n'
                'allowed_messages=["TextNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="python-visible-ascii-test"\n'
                              f'system_label="Python VisibleAscii"\nschema_path={json.dumps(str(schema))}\n'
                              'schema_version="002.5.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                              f'services_dir={json.dumps(str(services))}\n')
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([binary, str(config)], stdout=log, stderr=log,
                                          env={**os.environ, 'RUST_LOG': 'sleet=debug'})
                try:
                    async with asyncio.timeout(15):
                        while True:
                            if broker.poll() is not None:
                                log.seek(0)
                                self.fail('broker exited before OWP session: ' + log.read())
                            try:
                                _, writer = await asyncio.open_connection('127.0.0.1', port)
                                writer.close()
                                await writer.wait_closed()
                                break
                            except ConnectionRefusedError:
                                await asyncio.sleep(.05)
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-visible-ascii', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def typed_handler(payload: m.Payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.named), m.Text256)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, typed_handler)
                        for text in [' ', '~' * 256, ' lead  and trail ']:
                            value = m.Payload(inherited=m.Text256(text), direct='unconstrained',
                                              named=m.Text256(text), values=(m.Text256(text),),
                                              nested=m.Nested(value=m.Text256(text)))
                            await api.function_loop.exchange_output.publish(publisher, value)
                            actual = await asyncio.wait_for(received.get(), 5)
                            self.assertEqual(actual, value)
                            self.assertEqual(actual.named.value, text)
                        observations = []
                        for bad in ['\t', '\r', '\n', '\x00', '\x1f', '\x7f', 'é', '', 'x' * 257]:
                            body = json.loads(c.encode_Payload(value))
                            body['Nested']['Value'] = bad
                            malformed = json.dumps(body)
                            with self.assertRaises(c.CodecError):
                                c.decode_Payload(malformed)
                            try:
                                await publisher._operation('PUB visible.loop {"TextNotice":' + malformed + '}')
                            except ServerRejection as error:
                                observations.append((repr(bad), 'broker rejection', error.code))
                            else:
                                event = await asyncio.wait_for(subscriber.next_event(), 5)
                                self.assertIs(type(event.error), CodecFailure)
                                observations.append((repr(bad), 'generated decoder rejection'))
                            self.assertTrue(received.empty())
                            await api.function_loop.exchange_output.publish(publisher, value)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        for bad in ['\n', 'x' * 257, 3]:
                            object.__setattr__(value.named, 'value', bad)
                            with self.assertRaises(CodecFailure):
                                await api.function_loop.exchange_output.publish(publisher, value)
                        object.__setattr__(value.named, 'value', text)
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        print('PY010 OBSERVED REJECTIONS: ' + repr(observations), flush=True)
                        if os.environ.get('AMS_GRA_PY010_EVIDENCE'):
                            evidence_path = Path(os.environ['AMS_GRA_PY010_EVIDENCE'])
                            evidence_path.mkdir(parents=True, exist_ok=False)
                            (evidence_path / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
                            shutil.copytree(path / 'visible_ascii_models', evidence_path / 'generated')
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    log.seek(0)
                    evidence = log.read()
                    self.assertIn('route PUB message', evidence)
                    self.assertIn('TextNotice', evidence)
                    print('PY010 BROKER: routed TextNotice; rejection attribution recorded per observed event; recovery and teardown passed', flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    if os.environ.get('AMS_GRA_PY010_EVIDENCE'):
                        evidence_path = Path(os.environ['AMS_GRA_PY010_EVIDENCE'])
                        evidence_path.mkdir(parents=True, exist_ok=True)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', evidence_path / 'broker.log')
            leaked = [task for task in asyncio.all_tasks() if task is not asyncio.current_task()
                      and task.get_name().startswith('oms-lacal-') and not task.done()]
            self.assertEqual(leaked, [])
            print('PY010 PINNED SLEET VISIBLE ASCII: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()
