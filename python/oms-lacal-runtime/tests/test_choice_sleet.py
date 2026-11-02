"""Synthetic Choice transport; instrumented decoder distinguishes rejection sites."""
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


class ChoiceSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_choice_delivery_recovery_and_teardown(self):
        choice = os.environ['AMS_GRA_SLEET_BIN']
        checkout = Path(choice).resolve().parents[2]
        self.assertEqual(subprocess.check_output(
            ['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines(),
            ['e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b', '7e705f4eb5f658957bd414b4abc8207311f3c1e4'])
        self.assertEqual(subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']), b'')
        initial_tasks = set(asyncio.all_tasks())
        with TemporaryDirectory(prefix='py015-sleet-') as temporary:
            path = Path(temporary)
            schema = ROOT / 'tests/fixtures/python-choice/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT / 'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator, 'service-generate', '--schema', str(schema),
                '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                '--world', 'closed-schema', '--with-codec', '--output', str(path / 'choice_models')],
                check=True, capture_output=True)
            sys.path.insert(0, temporary)
            self.addCleanup(sys.path.remove, temporary)
            m = importlib.import_module('choice_models.model')
            c = importlib.import_module('choice_models.service_codec')
            api = importlib.import_module('choice_models.service_api')
            for name in ['choice_models', 'choice_models.model', 'choice_models.service_codec', 'choice_models.service_api']:
                self.addCleanup(sys.modules.pop, name, None)
            decoder_calls = []

            def observed_decoder(text):
                decoder_calls.append(text)
                return c.decode_Payload(text)

            registry = CodecRegistry([CodecBinding(OAM, 'ChoiceNotice', m.Payload, c.encode_Payload,
                                                   observed_decoder, c.CodecError)])
            services = path / 'services.d'
            services.mkdir()
            (services / 'python.toml').write_text(
                'service_id="python-choice"\nservice_uuid="550e8400-e29b-41d4-a716-446655440015"\n'
                'allowed_topics=["choice.loop"]\n[[topic_bindings]]\ntopic="choice.loop"\n'
                'allowed_messages=["ChoiceNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            config = path / 'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="py015-test"\n'
                f'system_label="Python Choice"\nschema_path={json.dumps(str(schema))}\n'
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
                    runtime_config = RuntimeConfig(f'ws://127.0.0.1:{port}', 'python-choice', '002.5.0')
                    async with LacalRuntime(runtime_config, registry) as subscriber, \
                            LacalRuntime(runtime_config, registry) as publisher:
                        received = asyncio.Queue()

                        def handler(payload):
                            self.assertIs(type(payload), m.Payload)
                            self.assertIs(type(payload.required), m.Selection)
                            self.assertIs(type(payload.positive), tuple)
                            received.put_nowait(payload)

                        handle = await api.function_loop.exchange_input.subscribe(subscriber, handler)
                        a = m.Owner.GovernmentIdentifier(m.Country.USA)
                        b = m.Owner.NATO_SpecialWord(m.Words('NATO:EXAMPLE'))
                        def sample(value=None, optional=None):
                            return m.Payload(required=m.Selection.Text('x') if value is None else value,
                                             positive=(a, b), inherited=(b, a), bounded=(a, a),
                                             optional=optional, repeated=(m.Selection.Text('x'), m.Selection.Nested(b)))

                        alternatives = [m.Selection.Text('x'), m.Selection.OtherText('x'),
                            m.Selection.Number(-3), m.Selection.Binary(m.Octets(b'\x00\xff')),
                            m.Selection.DirectBinary(b'\xff'), m.Selection.When('2026-01-02T03:04:05Z'),
                            m.Selection.Span('PT1.2300S'), m.Selection.Record(m.Detail(count=2)),
                            m.Selection.OtherRecord(m.Detail(count=2)), m.Selection.Nested(a),
                            m.Selection.Nested(b), m.Selection.schema_class('keyword')]
                        for selected in alternatives:
                            for optional in [None, a, b]:
                                value = sample(selected, optional)
                                await api.function_loop.exchange_output.publish(publisher, value)
                                actual = await asyncio.wait_for(received.get(), 5)
                                self.assertEqual(actual, value)
                                self.assertEqual(actual.positive, (a, b))
                        valid_body = json.loads(c.encode_Payload(sample()))
                        malformed = [json.dumps({**valid_body, 'Required': invalid}) for invalid in
                            [{}, {'Text': 'x', 'OtherText': 'y'}, {'Unknown': 'x'},
                             {'Text': None}, {'Number': True}, {'Nested': {'NATO_SpecialWord': 'BAD'}},
                             {'Record': {'Count': -1}}, {'$type': 'Selection'}, None, ['x'],
                             {'Span': 'P1DT'}, {'Span': 'PT.5S'}, {'Span': 'PT1M1H'},
                             {'Number': 1.0}]]
                        malformed.append(json.dumps({key: value for key, value in valid_body.items()
                                                     if key != 'Required'}))
                        malformed.append(json.dumps({**valid_body, 'Unknown': 'x'}))
                        # JSON duplicate-member evidence can pass a broker that
                        # collapses dictionaries but must reach our strict parser.
                        malformed.append(json.dumps(valid_body).replace(
                            '"Text": "x"', '"Text":"x","Text":"x"'))
                        for body in malformed:
                            before = len(decoder_calls)
                            try:
                                await publisher._operation('PUB choice.loop {"ChoiceNotice":' + body + '}')
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
                        self.assertTrue(any(row['outcome'] == 'generated decoder rejection' for row in observations),
                                        'no actual transport-to-decoder rejection')
                        self.assertTrue(any(row['outcome'] == 'broker rejection' for row in observations))
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                        self.assertEqual(len(subscriber._subscriptions), 0)
                    self.assertFalse(subscriber.is_connected)
                    self.assertFalse(publisher.is_connected)
                    print('PY015 SLEET OBSERVATIONS: ' + json.dumps(observations), flush=True)
                finally:
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=5)
                    evidence = os.environ.get('AMS_GRA_PY015_SLEET_EVIDENCE')
                    if evidence:
                        destination = Path(evidence)
                        destination.mkdir(parents=True, exist_ok=False)
                        log.flush()
                        shutil.copyfile(path / 'broker.log', destination / 'broker.log')
                        shutil.copytree(path / 'choice_models', destination / 'generated')
                        (destination / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
            await asyncio.sleep(0)
            self.assertEqual([task for task in asyncio.all_tasks() - initial_tasks if not task.done()], [])


if __name__ == '__main__':
    unittest.main()