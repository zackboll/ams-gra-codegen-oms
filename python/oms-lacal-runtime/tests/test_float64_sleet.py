"""PY-008: generated Float64 models/codecs through the shared pinned Sleet."""
import asyncio
import importlib
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from support import ROOT, OAM


class Float64SleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_float64_delivery_rejection_and_recovery(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'pinned Sleet mandatory; no skip')
        with TemporaryDirectory(prefix='py008-broker-') as temp:
            path = Path(temp)
            schema = ROOT / 'tests/fixtures/python-float64/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                            '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(path / 'float64')],
                           check=True, capture_output=True)
            sys.path.insert(0, temp)
            self.addCleanup(sys.path.remove, temp)
            m = importlib.import_module('float64.model')
            c = importlib.import_module('float64.service_codec')
            api = importlib.import_module('float64.service_api')
            for name in ['float64', 'float64.model', 'float64.service_codec', 'float64.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            registry = CodecRegistry([CodecBinding(OAM, 'FloatNotice', m.Payload, c.encode_Payload,
                                                  c.decode_Payload, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-float64"\nservice_uuid="550e8400-e29b-41d4-a716-446655440008"\n'
                'allowed_topics=["float64.loop"]\n[[topic_bindings]]\ntopic="float64.loop"\n'
                'allowed_messages=["FloatNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="python-float64-test"\n'
                              f'system_label="Python Float64"\nschema_path={json.dumps(str(schema))}\n'
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
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-float64', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def typed_handler(payload: m.Payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.named), m.Scalar)
                            self.assertIs(type(payload.limited), m.Bounded)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, typed_handler)
                        value = m.Payload(inherited=-0.0, direct=1.0000000000000002,
                                          named=m.Scalar(-0.0), limited=m.Bounded(1.25),
                                          values=(m.Bounded(-1.0), m.Bounded(1.5)),
                                          nested=m.Nested(value=m.Bounded(0.5)), integer=123,
                                          text='Infinity', optional=5e-324)
                        await api.function_loop.exchange_output.publish(publisher, value)
                        actual = await asyncio.wait_for(received.get(), 5)
                        self.assertEqual(actual, value)
                        for lhs, rhs in [(actual.inherited, value.inherited), (actual.direct, value.direct),
                                         (actual.named.value, value.named.value), (actual.optional, value.optional),
                                         (actual.limited.value, value.limited.value)]:
                            self.assertEqual(struct.pack('>d', lhs), struct.pack('>d', rhs))
                        malformed = c.encode_Payload(value).replace('"Limited":1.25', '"Limited":2.0')
                        # Pinned Sleet is the actual wire validator. A broker
                        # rejection is not counted as handler/decoder delivery.
                        with self.assertRaises(ServerRejection) as rejected:
                            await publisher._operation('PUB float64.loop {"FloatNotice":' + malformed + '}')
                        self.assertEqual(rejected.exception.code, 'Invalid-Message')
                        with self.assertRaisesRegex(c.CodecError, r'\$\.Limited'):
                            c.decode_Payload(malformed)
                        self.assertTrue(received.empty())
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        object.__setattr__(value.limited, 'value', 2.0)
                        with self.assertRaises(CodecFailure):
                            await api.function_loop.exchange_output.publish(publisher, value)
                        object.__setattr__(value.limited, 'value', 1.25)
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    log.seek(0)
                    evidence = log.read()
                    self.assertIn('route PUB message', evidence)
                    self.assertIn('FloatNotice', evidence)
                    print('PY008 BROKER: Invalid-Message for exclusive upper bound; subsequent valid delivery', flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
            leaked = [task for task in asyncio.all_tasks() if task is not asyncio.current_task()
                      and task.get_name().startswith('oms-lacal-') and not task.done()]
            self.assertEqual(leaked, [])
            print('PY008 PINNED SLEET FLOAT64: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()