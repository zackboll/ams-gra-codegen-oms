"""PY-009: generated DateTime models/codecs through the shared pinned Sleet."""
import asyncio
import importlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig
from support import ROOT, OAM


class DateTimeSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_datetime_delivery_rejection_and_recovery(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'pinned Sleet mandatory; no skip')
        with TemporaryDirectory(prefix='py009-broker-') as temp:
            path = Path(temp)
            schema = ROOT / 'tests/fixtures/python-datetime/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                            '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(path / 'datetime_models')],
                           check=True, capture_output=True)
            sys.path.insert(0, temp)
            self.addCleanup(sys.path.remove, temp)
            m = importlib.import_module('datetime_models.model')
            c = importlib.import_module('datetime_models.service_codec')
            api = importlib.import_module('datetime_models.service_api')
            for name in ['datetime_models', 'datetime_models.model', 'datetime_models.service_codec', 'datetime_models.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            registry = CodecRegistry([CodecBinding(OAM, 'DateNotice', m.Payload, c.encode_Payload,
                                                  c.decode_Payload, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-datetime"\nservice_uuid="550e8400-e29b-41d4-a716-446655440009"\n'
                'allowed_topics=["datetime.loop"]\n[[topic_bindings]]\ntopic="datetime.loop"\n'
                'allowed_messages=["DateNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="python-datetime-test"\n'
                              f'system_label="Python DateTime"\nschema_path={json.dumps(str(schema))}\n'
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
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-datetime', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def typed_handler(payload: m.Payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.named), m.Instant)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, typed_handler)
                        zulu = '2024-02-29T00:00:00.123456789012345678900Z'
                        value = m.Payload(inherited='2000-02-29T23:59:60+14:00', direct=zulu,
                                          named=m.Instant(zulu), values=(m.Instant(zulu),),
                                          nested=m.Nested(value=m.Instant(zulu)), optional=zulu)
                        await api.function_loop.exchange_output.publish(publisher, value)
                        actual = await asyncio.wait_for(received.get(), 5)
                        self.assertEqual(actual, value)
                        self.assertEqual(actual.named.value, zulu)
                        self.assertEqual(actual.inherited, value.inherited)
                        malformed = c.encode_Payload(value).replace(zulu, '2023-02-29T00:00:00Z')
                        # Measured pinned Sleet admits this invalid Gregorian
                        # date. Require actual subscriber-side rejection before
                        # typed handler delivery, not a guessed broker verdict.
                        await publisher._operation('PUB datetime.loop {"DateNotice":' + malformed + '}')
                        event = await asyncio.wait_for(subscriber.next_event(), 5)
                        self.assertIs(type(event.error), CodecFailure)
                        with self.assertRaisesRegex(c.CodecError, r'\$\.Inherited|\$\.Direct'):
                            c.decode_Payload(malformed)
                        self.assertTrue(received.empty())
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        object.__setattr__(value.named, 'value', 'invalidZ')
                        with self.assertRaises(CodecFailure):
                            await api.function_loop.exchange_output.publish(publisher, value)
                        object.__setattr__(value.named, 'value', zulu)
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
                    self.assertIn('DateNotice', evidence)
                    print('PY009 BROKER: malformed calendar admitted; generated decoder rejected before handler; subsequent valid delivery', flush=True)
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
            print('PY009 PINNED SLEET DATETIME: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()
