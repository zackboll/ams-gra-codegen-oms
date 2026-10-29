"""PY-007: generated concrete inheritance through unmodified pinned Sleet."""
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


class InheritanceSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_inherited_record_delivery_rejection_and_recovery(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'pinned Sleet is mandatory; no skip')
        with TemporaryDirectory(prefix='py007-broker-') as temp:
            path = Path(temp)
            schema = ROOT / 'tests/fixtures/python-inheritance/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                            '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(path / 'inherited')],
                           check=True, capture_output=True)
            sys.path.insert(0, temp)
            self.addCleanup(sys.path.remove, temp)
            m = importlib.import_module('inherited.model')
            c = importlib.import_module('inherited.service_codec')
            api = importlib.import_module('inherited.service_api')
            for name in ['inherited', 'inherited.model', 'inherited.service_codec', 'inherited.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            registry = CodecRegistry([CodecBinding(OAM, 'InheritedNotice', m.Leaf, c.encode_Leaf,
                                                  c.decode_Leaf, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-inherited"\nservice_uuid="550e8400-e29b-41d4-a716-446655440007"\n'
                'allowed_topics=["inherited.loop"]\n[[topic_bindings]]\ntopic="inherited.loop"\n'
                'allowed_messages=["InheritedNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="python-inherited-test"\n'
                              f'system_label="Python Inherited"\nschema_path={json.dumps(str(schema))}\n'
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
                                self.fail('broker exited before OWP session: ' + log.read())
                            try:
                                _, writer = await asyncio.open_connection('127.0.0.1', port)
                                writer.close()
                                await writer.wait_closed()
                                break
                            except ConnectionRefusedError:
                                await asyncio.sleep(.05)
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-inherited', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def typed_handler(payload: m.Leaf):
                            self.assertIs(type(payload), m.Leaf)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, typed_handler)
                        value = m.Leaf(base_required=7, base_optional='ancestor',
                                       base_values=(m.Token(2), m.Token(9)), required_values=(3, 4),
                                       linked=m.Token(5), local='leaf', nested=m.Nested(count=8, label='nested'))
                        await api.function_loop.exchange_output.publish(publisher, value)
                        actual = await asyncio.wait_for(received.get(), 5)
                        self.assertEqual(actual, value)
                        self.assertEqual(actual.base_required, 7)
                        self.assertEqual(actual.base_optional, 'ancestor')
                        self.assertEqual([v.value for v in actual.base_values], [2, 9])
                        self.assertEqual(actual.required_values, (3, 4))
                        self.assertEqual(actual.linked.value, 5)
                        self.assertEqual(actual.local, 'leaf')
                        self.assertIs(type(actual.nested), m.Nested)
                        self.assertEqual((actual.nested.count, actual.nested.label), (8, 'nested'))
                        # Inject hostile wire data without bypassing the broker
                        # or the receive decoder. The later inherited item is
                        # outside Token's schema-derived inclusive domain.
                        malformed = c.encode_Leaf(value).replace('"Base_Values":[2,9]', '"Base_Values":[2,10]')
                        with self.assertRaises(ServerRejection) as rejected:
                            await publisher._operation('PUB inherited.loop {"InheritedNotice":' + malformed + '}')
                        self.assertEqual(rejected.exception.code, 'Invalid-Message')
                        with self.assertRaisesRegex(c.CodecError, r'Base_Values\[1\]'):
                            c.decode_Leaf(malformed)
                        self.assertTrue(received.empty())
                        await api.function_loop.exchange_output.publish(publisher, value)
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                        corrupted = m.Token(2)
                        object.__setattr__(corrupted, 'value', 10)
                        bad = m.Leaf(base_required=7, required_values=(3,), base_values=(corrupted,),
                                     linked=m.Token(5), local='leaf', nested=value.nested)
                        with self.assertRaises(CodecFailure):
                            await api.function_loop.exchange_output.publish(publisher, bad)
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
                    self.assertIn('InheritedNotice', evidence)
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
            print('PY007 PINNED SLEET INHERITED RECORD: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()