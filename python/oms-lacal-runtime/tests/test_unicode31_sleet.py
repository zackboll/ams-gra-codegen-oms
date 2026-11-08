"""PY-018 exact generated carriers over the existing pinned Sleet/runtime.

The schema is explicitly synthetic: full AO remains blocked by abstract values.
Decoder instrumentation separates broker policy from generated lexical checks.
"""
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


class Unicode31SleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_publish_receive_recovery_unsubscribe_and_teardown(self):
        binary = Path(os.environ['AMS_GRA_SLEET_BIN']).resolve()
        self.assertEqual(subprocess.check_output(['git', '-C', str(binary.parents[2]), 'rev-parse',
                         'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
                         ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b',
                          '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(binary.parents[2]), 'status', '--porcelain']), b'')
        for world in ['closed-schema', 'open-extensions']:
            await self.exercise(binary, world)

    async def exercise(self, binary, world):
        initial = set(asyncio.all_tasks())
        fixture = ROOT / 'tests/fixtures/python-unicode31'
        schema = fixture / 'oam.xsd'
        generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
        observations = []
        with TemporaryDirectory(prefix='py018-sleet-') as temporary:
            path = Path(temporary)
            package = 'unicode_' + world.replace('-', '_')
            subprocess.run([generator, 'service-generate', '--schema', str(schema), '--contract',
                str(fixture / 'service.yaml'), '--language', 'python', '--world', world,
                '--with-codec', '--output', str(path / package)], check=True, capture_output=True)
            sys.path.insert(0, str(path))
            self.addCleanup(sys.path.remove, str(path))
            for name in [package, package + '.model', package + '.service_codec', package + '.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            m = importlib.import_module(package + '.model')
            c = importlib.import_module(package + '.service_codec')
            api = importlib.import_module(package + '.service_api')
            def sample(date='20000231', time='200002312359', location='+12.123456-179.123456'):
                entry = m.EntryType(date=m.NITF_DateType(date), time=m.NITF_DateAndTimeType(time),
                                    location=m.NITF_MSTGTA_TargetLocationType(location))
                return m.Payload(required=m.NITF_DateType(date), entry=(entry,), selected=m.Selection.Entry(entry))
            values = [sample(), sample('2١۲𝟎0231', '2١۲𝟎02312359'), sample(' ' * 8, ' ' * 12)]
            calls = []
            def decoder(text):
                calls.append(text)
                return c.decode_Payload(text)
            registry = CodecRegistry([CodecBinding(OAM, 'UnicodeNotice', m.Payload,
                c.encode_Payload, decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-unicode"\nservice_uuid="550e8400-e29b-41d4-a716-446655440018"\n'
                'allowed_topics=["unicode.loop"]\n[[topic_bindings]]\ntopic="unicode.loop"\n'
                'allowed_messages=["UnicodeNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py018-test"\n'
                f'system_label="Python Unicode31"\nschema_path={json.dumps(str(schema))}\n'
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
                    runtime_config = RuntimeConfig(url, 'python-unicode', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher, \
                            connect(url, subprotocols=['owp']) as observer:
                        await observer.send('INIT ' + json.dumps(dict(versions=['1.0'], schema='002.5.0',
                            service_id='python-unicode', verbose=True)))
                        self.assertEqual(await observer.recv(), '+OK')
                        self.assertTrue((await observer.recv()).startswith('INFO '))
                        await observer.send('SUB independent UnicodeNotice unicode.loop')
                        self.assertEqual(await observer.recv(), '+OK')
                        received = asyncio.Queue()
                        handle = await api.function_loop.exchange_input.subscribe(subscriber, received.put_nowait)
                        for value in values:
                            before = len(calls)
                            await api.function_loop.exchange_output.publish(publisher, value)
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), value)
                            self.assertEqual(len(calls), before + 1)
                            frame = await asyncio.wait_for(observer.recv(), 5)
                            self.assertTrue(frame.startswith('MSG independent '), frame)
                            doc = json.loads(frame.split(' ', 2)[2])
                            entry = value.entry[0]
                            golden_entry = dict(Date=entry.date.value, Time=entry.time.value,
                                                Location=entry.location.value)
                            self.assertEqual(doc, {'UnicodeNotice': dict(Required=value.required.value,
                                Entry=[golden_entry], Selected={'Entry': golden_entry})})
                        for bad in ['20001301', '2𞥐000101', '2፩000101', '2000010', None, 123]:
                            doc = json.loads(c.encode_Payload(values[0]))
                            doc['Entry'][0]['Date'] = bad
                            body = json.dumps(doc)
                            before = len(calls)
                            try:
                                await publisher._operation('PUB unicode.loop {"UnicodeNotice":' + body + '}')
                            except ServerRejection as error:
                                self.assertEqual(len(calls), before)
                                observations.append(dict(value=bad, outcome='broker rejection', code=error.code))
                            else:
                                if bad == '2፩000101':
                                    # Old Nd accepted by generated decoder even if modern
                                    # broker classification happens to permit delivery.
                                    self.assertEqual((await asyncio.wait_for(received.get(), 5)).entry[0].date.value, bad)
                                    self.assertEqual(len(calls), before + 1)
                                    observations.append(dict(value=bad, outcome='valid Unicode31 delivery'))
                                else:
                                    event = await asyncio.wait_for(subscriber.next_event(), 5)
                                    if type(event.error) is ProtocolError:
                                        self.assertEqual(len(calls), before)
                                        observations.append(dict(value=bad, outcome='runtime envelope rejection'))
                                    else:
                                        self.assertIs(type(event.error), CodecFailure)
                                        self.assertEqual(len(calls), before + 1)
                                        observations.append(dict(value=bad, outcome='generated decoder rejection'))
                            self.assertTrue(received.empty())
                            await api.function_loop.exchange_output.publish(publisher, values[0])
                            self.assertEqual(await asyncio.wait_for(received.get(), 5), values[0])
                        self.assertTrue(any(row['outcome'] == 'broker rejection' for row in observations))
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        await api.function_loop.exchange_output.publish(publisher, values[0])
                        await asyncio.sleep(.1)
                        self.assertTrue(received.empty())
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    print('PY018 SLEET ' + world + ': ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY018_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence) / world
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / package, destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial if not task.done()], [])


if __name__ == '__main__':
    unittest.main()