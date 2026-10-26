"""PY-006: actual generated header carriers through unmodified pinned Sleet."""
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

from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from support import ROOT, OAM


class HeaderSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_header_profiles_broker_receipt_rejection_and_recovery(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'pinned Sleet is mandatory; no skip')
        with TemporaryDirectory(prefix='py006-broker-') as temp:
            path = Path(temp)
            schema = ROOT / 'tests/fixtures/python-header-profiles/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                            '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(path / 'headers')],
                           check=True, capture_output=True)
            sys.path.insert(0, temp)
            self.addCleanup(sys.path.remove, temp)
            m = importlib.import_module('headers.model')
            c = importlib.import_module('headers.service_codec')
            api = importlib.import_module('headers.service_api')
            for name in ['headers', 'headers.model', 'headers.service_codec', 'headers.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            registry = CodecRegistry(CodecBinding(OAM, name, m.Payload, c.encode_Payload,
                                                 c.decode_Payload, c.CodecError)
                                     for name in ['HeaderNotice', 'HeaderOther'])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-header"\nservice_uuid="550e8400-e29b-41d4-a716-446655440006"\n'
                'allowed_topics=["header.loop"]\n[[topic_bindings]]\ntopic="header.loop"\n'
                'allowed_messages=["HeaderNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="python-header-test"\n'
                              f'system_label="Python Header"\nschema_path={json.dumps(str(schema))}\n'
                              'schema_version="002.5.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                              f'services_dir={json.dumps(str(services))}\n')
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([binary, str(config)], stdout=log, stderr=log,
                                          env={**os.environ, 'RUST_LOG': 'sleet=debug'})
                try:
                    async with asyncio.timeout(15):
                        while True:
                            if broker.poll() is not None:
                                log.flush()
                                log.seek(0)
                                self.fail('broker exited before validated OWP session: ' + log.read())
                            try:
                                _, writer = await asyncio.open_connection('127.0.0.1', port)
                                writer.close()
                                await writer.wait_closed()
                                break
                            except ConnectionRefusedError:
                                await asyncio.sleep(.05)
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-header', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def typed_handler(payload: m.Payload):
                            self.assertIs(type(payload), m.Payload)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, typed_handler)
                        version = '002.5.0ab_Release-2A'
                        uuid = '123E4567-e89B-42d3-A456-426614174000'
                        value = m.Payload(header=m.Header(version=m.SchemaVersion(version),
                                          uuid=m.UniversallyUniqueIdentifierType(uuid)),
                                          versions=(m.SchemaVersion('000.0.0'),), count=42)
                        await api.function_loop.exchange_output.publish(publisher, value)
                        actual = await asyncio.wait_for(received.get(), 5)
                        self.assertEqual(actual, value)
                        self.assertEqual(actual.header.version.value, version)
                        self.assertEqual(actual.header.uuid.value, uuid)
                        # Bypass only outbound validation to inject hostile wire data.
                        # The registered receive codec and raw envelope path stay real.
                        malformed = c.encode_Payload(value).replace(uuid, '00000000-0000-0000-0000-000000000001')
                        # The unmodified broker validates this XSD pattern and
                        # rejects it before routing. Do not weaken the schema
                        # just to force invalid data into the subscriber.
                        with self.assertRaises(ServerRejection) as rejected:
                            await publisher._operation('PUB header.loop {"HeaderNotice":' + malformed + '}')
                        self.assertEqual(rejected.exception.code, 'Invalid-Message')
                        with self.assertRaises(c.CodecError):
                            c.decode_Payload(malformed)
                        self.assertTrue(received.empty())
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        corrupted = m.UniversallyUniqueIdentifierType(uuid)
                        object.__setattr__(corrupted, 'value', 'bad')
                        bad_value = m.Payload(header=m.Header(version=m.SchemaVersion(version), uuid=corrupted),
                                              versions=value.versions, count=42)
                        with self.assertRaises(CodecFailure):
                            await api.function_loop.exchange_output.publish(publisher, bad_value)
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    log.flush()
                    log.seek(0)
                    evidence = log.read()
                    self.assertIn('route PUB message', evidence)
                    self.assertIn('HeaderNotice', evidence)
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
            print('PY006 PINNED SLEET HEADER RECEIPT AND TYPED DELIVERY: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()