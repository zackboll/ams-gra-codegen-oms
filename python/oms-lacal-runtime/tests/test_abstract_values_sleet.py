"""Real pinned Sleet; exact typed subtype delivery and measured rejection site."""
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

from websockets.asyncio.client import connect
from oms_lacal import CodecBinding, CodecFailure, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from oms_lacal.wire import ProtocolError
from support import ROOT, OAM


class AbstractValuesSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_known_subtypes_recovery_unsubscribe_and_cleanup(self):
        binary = Path(os.environ['AMS_GRA_SLEET_BIN']).resolve()
        checkout = binary.parents[2]
        self.assertEqual(subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
            ['94796daf55ebd1134dce0925e6a204a05ad3423b', '47da595b9078cac1a9351e8290d1ae7f3976367f'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        observations = []
        with TemporaryDirectory(prefix='py019-sleet-') as temporary:
            path = Path(temporary)
            schema = ROOT / 'tests/fixtures/python-abstract-values/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            package = 'abstract_models'
            subprocess.run([generator, 'service-generate', '--schema', str(schema), '--contract',
                str(schema.with_name('service.yaml')), '--language', 'python', '--world',
                'closed-schema', '--with-codec', '--output', str(path / package)],
                check=True, capture_output=True)
            sys.path.insert(0, temporary)
            self.addCleanup(sys.path.remove, temporary)
            for name in [package, package + '.model', package + '.service_codec', package + '.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            m = importlib.import_module(package + '.model')
            c = importlib.import_module(package + '.service_codec')
            api = importlib.import_module(package + '.service_api')
            calls = []
            decoder_errors = []

            def decoder(text):
                calls.append(text)
                try:
                    return c.decode_Payload(text)
                except c.CodecError as error:
                    decoder_errors.append(dict(input=text, category=type(error).__name__, path=str(error).split(':', 1)[0]))
                    raise

            registry = CodecRegistry([CodecBinding(OAM, 'AbstractNotice', m.Payload,
                c.encode_Payload, decoder, c.CodecError)])
            a = m.Base.from_Zeta(m.Zeta(inherited=1, text='z'))
            b = m.Base.from_Deep_Leaf(m.Deep_Leaf(inherited=2, enabled=True, count=-3))
            d = m.Base.from_Alpha(m.Alpha(inherited=3, real=1.5))
            wires = [{'Inherited': 1, 'Text': 'z', '$type': 'Zeta'},
                     {'Inherited': 2, 'Enabled': True, 'Count': -3, '$type': 'Deep-Leaf'},
                     {'Inherited': 3, 'Real': 1.5, '$type': 'Alpha'}]

            def sample(selected=a, optional=None):
                return m.Payload(required=selected, optional=optional, repeated=(a, b, d),
                    bounded=(d,), nested=m.Selection.Abstract(b), concrete=m.Zeta(inherited=4, text='c'))

            def golden(selected=wires[0], optional=None):
                body = {'Required': selected, 'Repeated': wires, 'Bounded': [wires[2]],
                        'Nested': {'Abstract': wires[1]}, 'Concrete': {'Inherited': 4, 'Text': 'c'}}
                if optional is not None:
                    body['Optional'] = optional
                return body

            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-abstract"\nservice_uuid="550e8400-e29b-41d4-a716-446655440019"\n'
                'allowed_topics=["abstract.loop"]\n[[topic_bindings]]\ntopic="abstract.loop"\n'
                'allowed_messages=["AbstractNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py019-test"\n'
                f'system_label="Python Closed Abstract"\nschema_path={json.dumps(str(schema))}\n'
                'schema_version="002.5.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                f'services_dir={json.dumps(str(services))}\n')
            with (path / 'broker.log').open('w+') as log:
                broker = subprocess.Popen([str(binary), str(config)], stdout=log, stderr=log,
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
                    runtime_config = RuntimeConfig(url, 'python-abstract', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher, \
                            connect(url, subprotocols=['owp']) as observer:
                        await observer.send('INIT ' + json.dumps(dict(versions=['1.0'], schema='002.5.0',
                            service_id='python-abstract', verbose=True)))
                        self.assertEqual(await observer.recv(), '+OK')
                        self.assertTrue((await observer.recv()).startswith('INFO '))
                        await observer.send('SUB independent AbstractNotice abstract.loop')
                        self.assertEqual(await observer.recv(), '+OK')
                        received = asyncio.Queue()

                        def handler(value):
                            self.assertIs(type(value), m.Payload)
                            self.assertIs(type(value.required), m.Base)
                            received.put_nowait(value)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, handler)
                        for selected, wire, concrete in zip([a, b, d], wires, [m.Zeta, m.Deep_Leaf, m.Alpha]):
                            for optional, optional_wire in [(None, None), (b, wires[1])]:
                                value = sample(selected, optional)
                                before = len(calls)
                                await api.function_loop.exchange_output.publish(publisher, value)
                                delivered = await asyncio.wait_for(received.get(), 5)
                                self.assertEqual(delivered, value)
                                self.assertIs(type(delivered.required.value), concrete)
                                self.assertEqual(len(calls), before + 1)
                                frame = await asyncio.wait_for(observer.recv(), 5)
                                self.assertTrue(frame.startswith('MSG independent '), frame)
                                self.assertEqual(json.loads(frame.split(' ', 2)[2]),
                                    {'AbstractNotice': golden(wire, optional_wire)})
                                observations.append(dict(outcome='typed delivery', subtype=wire['$type'],
                                    optional=optional is not None, raw_frame=frame))
                        # Schema-valid, but outside the documented PY-004
                        # concrete-position codec subset. This is deliberately
                        # NOT classified as malformed abstract data. PY-019
                        # retains that established boundary outside abstract
                        # positions (which do require the concrete $type).
                        candidate = ('{"Required":{"Inherited":1,"Text":"z","$type":"Zeta"},'
                                     '"Bounded":[{"Inherited":3,"Real":1.5,"$type":"Alpha"}],'
                                     '"Nested":{"Text":"n"},'
                                     '"Concrete":{"Inherited":4,"Text":"c","$type":"Zeta"}}')
                        with self.assertRaises(c.CodecError) as direct:
                            c.decode_Payload(candidate)
                        self.assertEqual(str(direct.exception).split(':', 1)[0], '$.Concrete')
                        before = len(calls)
                        errors_before = len(decoder_errors)
                        envelope = '{"AbstractNotice":' + candidate + '}'
                        # _operation completes only after the actual +OK;
                        # rejection raises ServerRejection instead.
                        await publisher._operation('PUB abstract.loop ' + envelope)
                        frame = await asyncio.wait_for(observer.recv(), 5)
                        self.assertEqual(frame, 'MSG independent ' + envelope)
                        event = await asyncio.wait_for(subscriber.next_event(), 5)
                        self.assertIs(type(event.error), CodecFailure)
                        self.assertEqual(event.subscription_id, handle.id)
                        self.assertEqual(len(calls), before + 1)
                        self.assertEqual(calls[-1], candidate)
                        self.assertEqual(len(decoder_errors), errors_before + 1)
                        self.assertEqual(decoder_errors[-1]['path'], '$.Concrete')
                        self.assertTrue(received.empty())
                        observations.append(dict(input=candidate, outcome='generated decoder rejection',
                            classification='schema-valid unsupported concrete-position discriminator',
                            broker_ack='+OK', raw_frame=frame, subscription_id=event.subscription_id,
                            decoder_delta=1, decoder_input=calls[-1], category=decoder_errors[-1]['category'],
                            path=decoder_errors[-1]['path'], runtime_event=type(event.error).__name__))
                        await api.function_loop.exchange_output.publish(publisher, sample())
                        self.assertEqual(await asyncio.wait_for(received.get(), 5), sample())
                        frame = await asyncio.wait_for(observer.recv(), 5)
                        self.assertEqual(json.loads(frame.split(' ', 2)[2]), {'AbstractNotice': golden()})
                        await observer.send('UNSUB independent')
                        self.assertEqual(await observer.recv(), '+OK')
                        invalids = [golden({'Inherited': 1, 'Text': 'z'}),
                                    golden({'Inherited': 1, 'Text': 'z', '$type': None}),
                                    golden({'Inherited': 1, 'Text': 'z', '$type': 'Private'}),
                                    golden({'Inherited': 1, 'Text': 'z', '$type': 'Middle'}),
                                    golden({'Inherited': 1, 'Text': 'z', '$type': 'Alpha'}),
                                    golden({'Inherited': -1, 'Text': 'z', '$type': 'Zeta'})]
                        bodies = [json.dumps(doc) for doc in invalids]
                        bodies.append(json.dumps(golden()).replace('"$type": "Zeta"',
                            '"$type":"Zeta","$type":"Zeta"', 1))
                        for body in bodies:
                            before = len(calls)
                            try:
                                await publisher._operation('PUB abstract.loop {"AbstractNotice":' + body + '}')
                            except ServerRejection as error:
                                self.assertEqual(len(calls), before)
                                observations.append(dict(input=body, outcome='broker rejection', code=error.code,
                                    decoder_delta=len(calls) - before))
                            else:
                                event = await asyncio.wait_for(subscriber.next_event(), 5)
                                if type(event.error) is ProtocolError:
                                    self.assertEqual(len(calls), before)
                                    observations.append(dict(input=body, outcome='runtime envelope rejection',
                                        decoder_delta=len(calls) - before, runtime_event=type(event.error).__name__))
                                else:
                                    self.assertIs(type(event.error), CodecFailure)
                                    self.assertEqual(len(calls), before + 1)
                                    observations.append(dict(input=body, outcome='generated decoder rejection',
                                        decoder_input=calls[-1], error=str(event.error)))
                            self.assertTrue(received.empty())
                            await api.function_loop.exchange_output.publish(publisher, sample())
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), sample())
                        self.assertTrue(any(row['outcome'] == 'broker rejection' for row in observations))
                        self.assertTrue(any(row['outcome'] == 'generated decoder rejection' for row in observations))
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        before = len(calls)
                        await api.function_loop.exchange_output.publish(publisher, sample())
                        await asyncio.sleep(.1)
                        self.assertTrue(received.empty())
                        self.assertEqual(len(calls), before)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                        self.assertTrue(handle._task.done())
                        observations.append(dict(outcome='unsubscribe and callback silence',
                            decoder_delta=len(calls) - before, subscriptions=len(subscriber._subscriptions)))
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    self.assertTrue(subscriber._receiver.done())
                    self.assertTrue(publisher._receiver.done())
                    print('PY019 SLEET OBSERVATIONS ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY019_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence)
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / package, destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])


if __name__ == '__main__':
    unittest.main()