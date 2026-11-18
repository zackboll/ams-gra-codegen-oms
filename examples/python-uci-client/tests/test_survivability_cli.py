"""PY-OMS-022 --with-log --with-survivability: arguments, format-3 bindings and summary.

No test here opens a network connection (test_cli.no_network). The existing
PY-OMS-020/021 suites stay unchanged; this suite covers only the new mode.
"""
import contextlib
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from support import (APP, bindings_directory, cleanup_bindings, client, generate_bindings,
                     generator, golden, log_golden, schema_root, survivability_golden,
                     survivability_variant)
from test_cli import args, edit_source, rewrite, run_main

THREE = ['--with-log', '--with-survivability']
MESSAGES = [
    dict(message='RDMA_InitializeSetup', payload_type='RDMA_InitializeSetupMT', topic='uci.rdma'),
    dict(message='Log', payload_type='LogMT', topic='uci.log'),
    dict(message='SurvivabilityRiskLevel', payload_type='SurvivabilityRiskLevelMT',
         topic='uci.survivability')]


def tearDownModule():
    cleanup_bindings()


def three(release):
    return bindings_directory(release, with_log=True, with_survivability=True)


def load(release):
    return client.load_bindings(three(release), release, with_log=True, with_survivability=True)


class Arguments(unittest.TestCase):
    def test_survivability_requires_log_before_any_network(self):
        options = client.parse_arguments(args() + THREE)
        self.assertEqual((options.with_log, options.with_survivability, options.manifest_format),
                         (True, True, 3))
        self.assertTrue(client.parse_arguments(['--with-survivability'] + args('2.6')
                                               + ['--with-log']).with_survivability)
        # Existing modes and defaults are unchanged.
        self.assertEqual((client.parse_arguments(args()).manifest_format,
                          client.parse_arguments(args()).with_survivability), (1, False))
        self.assertEqual(client.parse_arguments(args() + ['--with-log']).manifest_format, 2)
        for argv in (args() + ['--with-survivability'],
                     ['--with-survivability'] + args(bindings=three('2.5')),
                     args() + THREE + ['--with-survivability'], args() + ['--with-survivability=yes'],
                     args() + ['--with-log', '--with-survivability', 'true'],
                     args() + ['--with-survival', '--with-log'], ['--with-survivability', '--help']):
            with self.subTest(argv=argv):
                code, out, err = run_main(argv)
                self.assertEqual((code, out), (client.EXIT_USAGE, ''), err)
        _, _, err = run_main(args(bindings=three('2.5')) + ['--with-survivability'])
        self.assertIn('--with-survivability requires --with-log', err)
        code, out, _ = run_main(['--help'])
        self.assertIn('--with-survivability', out)
        self.assertIn('uci.survivability', out)

    def test_generation_helper_enforces_the_same_mode_rule(self):
        with tempfile.TemporaryDirectory() as temporary, \
                contextlib.redirect_stderr(io.StringIO()) as err:
            with self.assertRaises(SystemExit) as caught:
                generate_bindings.main(['--schema-release', '2.5', '--schema', str(schema_root('2.5')),
                                        '--output', str(Path(temporary) / 'o'),
                                        '--with-survivability', '--generator', str(generator())])
            self.assertEqual(caught.exception.code, 2)
            self.assertIn('--with-survivability requires --with-log', err.getvalue())
            with self.assertRaises(generate_bindings.BuildError):
                generate_bindings.generate('2.5', schema_root('2.5'), Path(temporary) / 'p',
                                           generator(), with_survivability=True)
            with self.assertRaises(generate_bindings.BuildError):
                generate_bindings.contract('2.5', with_survivability=True)
            self.assertEqual(sorted(p.name for p in Path(temporary).iterdir()), [])
        # Old contracts are selected exactly as before.
        self.assertEqual([generate_bindings.contract('2.6', *mode).name
                          for mode in ((), (True,), (True, True))],
                         ['uci26-service.yaml', 'uci26-rdma-log-service.yaml',
                          'uci26-rdma-log-survivability-service.yaml'])


class ThreeMessageBindings(unittest.TestCase):
    def test_actual_three_message_package_for_each_release(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                directory = three(release)
                manifest = json.loads((directory / generate_bindings.MANIFEST).read_text())
                self.assertEqual((manifest['format'], manifest['with_log'], manifest['with_survivability'],
                                  manifest['release'], manifest['schema_version'], manifest['world'],
                                  manifest['with_codec']),
                                 (3, True, True, release, client.SCHEMA_VERSIONS[release],
                                  'closed-schema', True))
                self.assertEqual(manifest['messages'], MESSAGES)  # exact ordered inventory
                self.assertNotIn('message', manifest)
                contract = generate_bindings.contract(release, with_log=True, with_survivability=True)
                self.assertEqual((manifest['contract'], manifest['contract_sha256']),
                                 (contract.relative_to(APP.parents[1]).as_posix(),
                                  generate_bindings.digest(contract)))
                pin = generate_bindings.PINS[release]
                self.assertEqual((manifest['schema_sha256'], manifest['include']),
                                 (pin['root'], {pin['include'][0]: pin['include'][1]}))
                self.assertEqual(manifest['generator_sha256'], generate_bindings.digest(generator()))
                self.assertEqual(manifest['files'], {name: generate_bindings.digest(directory / name)
                                                     for name in client.GENERATED})
                self.assertEqual(sorted(p.name for p in directory.iterdir()),
                                 sorted([*client.GENERATED, generate_bindings.MANIFEST]))
                self.assertFalse(APP.parents[1] in directory.resolve().parents)
                bindings = load(release)
                self.assertTrue(bindings.with_log and bindings.with_survivability)
                model, codec, api = bindings.model, bindings.codec, bindings.api
                identities = [(f.message, f.payload_name, f.topic, f.receive.MESSAGE_NAME,
                               f.receive.TOPIC, f.receive.Payload) for f in bindings.families]
                self.assertEqual(identities, [
                    ('RDMA_InitializeSetup', 'RDMA_InitializeSetupMT', 'uci.rdma',
                     'RDMA_InitializeSetup', 'uci.rdma', model.RDMA_InitializeSetupMT),
                    ('Log', 'LogMT', 'uci.log', 'Log', 'uci.log', model.LogMT),
                    ('SurvivabilityRiskLevel', 'SurvivabilityRiskLevelMT', 'uci.survivability',
                     'SurvivabilityRiskLevel', 'uci.survivability', model.SurvivabilityRiskLevelMT)])
                # The actual generated endpoint: function_survivability.exchange_receive.
                endpoint = api.function_survivability.exchange_receive
                self.assertIs(bindings.families[2].receive, endpoint)
                self.assertEqual((endpoint.MESSAGE_NAMESPACE, endpoint.DIRECTION,
                                  endpoint.SUBSCRIPTION_GROUP), (client.OAM, 'input', None))
                self.assertEqual([f.ID for f in api.FUNCTIONS], ['rdma', 'log', 'survivability'])
                # Receive-only: no publication endpoint was generated.
                self.assertEqual([e.DIRECTION for f in api.FUNCTIONS for e in f.EXCHANGES], ['input'] * 3)
                self.assertEqual(model.SurvivabilityRiskLevelMT.__schema_name__,
                                 (client.OAM, 'SurvivabilityRiskLevelMT'))
                # Registry keyed by the wire global element, never the payload class.
                binding = bindings.registry.lookup(client.OAM, 'SurvivabilityRiskLevel')
                self.assertIs(binding.payload_type, model.SurvivabilityRiskLevelMT)
                self.assertIs(binding.encode, codec.encode_SurvivabilityRiskLevelMT)
                self.assertIs(binding.decode, codec.decode_SurvivabilityRiskLevelMT)
                self.assertIs(binding.error_type, codec.CodecError)
                with self.assertRaises(client.InvalidInput):
                    bindings.registry.lookup(client.OAM, 'SurvivabilityRiskLevelMT')
                value = codec.decode_SurvivabilityRiskLevelMT(json.dumps(survivability_golden(release)))
                self.assertIs(type(value), model.SurvivabilityRiskLevelMT)
                self.assertEqual(json.loads(codec.encode_SurvivabilityRiskLevelMT(value)),
                                 survivability_golden(release))
                log = codec.decode_LogMT(json.dumps(log_golden(release)))
                rdma = codec.decode_RDMA_InitializeSetupMT(json.dumps(golden(release)))
                self.assertEqual([client.message_name(v) for v in (rdma, log, value)],
                                 ['RDMA_InitializeSetup', 'Log', 'SurvivabilityRiskLevel'])
                with self.assertRaises(TypeError):
                    client.message_name(value.messagedata)

    def test_survivability_summary_is_bounded_and_schema_checked(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                codec = load(release).codec
                decode = codec.decode_SurvivabilityRiskLevelMT
                value = decode(json.dumps(survivability_golden(release)))
                self.assertEqual(client.format_summary(4, value),
                                 'RECEIVED SurvivabilityRiskLevel #4 type=SurvivabilityRiskLevelMT '
                                 'risk-level-id=12345678-1234-1234-8234-123456789abc mode=LIVE '
                                 f'schema={client.SCHEMA_VERSIONS[release]} '
                                 'timestamp=2026-01-02T03:04:05Z classification=U '
                                 'risk-settings=1 [HIGH:NONE]')
                # The schema maximum (4 settings x 5 levels) is shown in full.
                risks = ['ACQUISITION', 'TRACK', 'LAUNCH', 'INTERCEPT', 'KILL']
                full = [(p, risks) for p in ('HIGH', 'MEDIUM', 'LOW', 'NONE')]
                line = client.format_summary(1, decode(json.dumps(survivability_variant(
                    release, settings=full))))
                self.assertTrue(line.endswith('risk-settings=4 [' + ', '.join(
                    f'{p}:' + '+'.join(risks) for p, _ in full) + ']'), line)
                # Free text, caveats, exposure durations and system IDs are never printed.
                labelled = survivability_golden(release)
                data = labelled['MessageData']
                data['SurvivabilityRiskLevelID']['DescriptiveLabel'] = 'private label'
                data['AppliesToSystemID'] = [{'UUID': 'aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee'}]
                data['RiskSetting'][0]['RiskLevel'][0]['MaximumContinuousExposure'] = {
                    'CompositeExposureTime': 'PT1M', 'CumulativeExposureTime': 'PT7M'}
                line = client.format_summary(1, decode(json.dumps(labelled)))
                for private in ('private label', 'PT1M', 'PT7M', 'aaaaaaaa-', 'NATO:EXAMPLE', 'USA', 'a b'):
                    self.assertNotIn(private, line)
                # The display bound holds independently of the schema bound.
                self.assertEqual(client._bounded(list('abcdef'), client.MAX_DISPLAYED_ALLOWED_RISKS),
                                 ['a', 'b', 'c', 'd', 'e', '+1 more'])
                for too_many in ([('HIGH', ['NONE'])] * 5, [('HIGH', ['TRACK'] * 6)]):
                    with self.assertRaises(codec.CodecError):
                        decode(json.dumps(survivability_variant(release, settings=too_many)))

    def test_mode_release_and_wrong_bindings_exit_3_without_network(self):
        cases = [
            (args('2.5', three('2.5')), 'were generated with --with-log --with-survivability'),
            (args('2.5', three('2.5')) + ['--with-log'],
             'were generated with --with-log --with-survivability'),
            (args('2.5', bindings_directory('2.5', with_log=True)) + THREE,
             'select only RDMA_InitializeSetup + Log; --with-survivability needs'),
            (args('2.5', bindings_directory('2.5')) + THREE,
             'select only RDMA_InitializeSetup; --with-survivability needs'),
            (args('2.6', three('2.5')) + THREE, "generated for UCI '2.5', not --schema-release 2.6"),
            (args('2.5', three('2.6')) + THREE, "generated for UCI '2.6', not --schema-release 2.5")]
        for argv, message in cases:
            with self.subTest(message=message, argv=argv[-3:]):
                code, _, err = run_main(argv)
                self.assertEqual(code, client.EXIT_BINDINGS, err)
                self.assertIn(message, err)

    def test_missing_altered_and_relabelled_bindings_fail_closed(self):
        codec_def = 'def decode_SurvivabilityRiskLevelMT('
        edits = [
            ('missing-file', lambda d: (d / 'service_api.py').unlink(), 'missing generated service_api.py'),
            ('missing-manifest', lambda d: (d / generate_bindings.MANIFEST).unlink(),
             'run generate_bindings.py first'),
            ('modified-model', lambda d: edit_source(d, 'model.py', 'class SurvivabilityRiskLevelMT:',
                                                     'class SurvivabilityRiskLevelMX:'),
             'model.py was modified'),
            ('modified-api', lambda d: edit_source(d, 'service_api.py', 'uci.survivability', 'uci.surv'),
             'service_api.py was modified'),
            ('missing-codec', lambda d: edit_source(d, 'service_codec.py', codec_def,
                                                    'def _x_SurvivabilityRiskLevelMT(', resign=True),
             'generated service codec lacks the SurvivabilityRiskLevelMT codec'),
            # Re-signed endpoint whose wire name became the payload class name.
            ('payload-as-wire-name', lambda d: edit_source(
                d, 'service_api.py', 'MESSAGE_NAME = "SurvivabilityRiskLevel"',
                'MESSAGE_NAME = "SurvivabilityRiskLevelMT"', resign=True),
             'generated service API has no unique SurvivabilityRiskLevel input exchange'),
            ('wrong-topic', lambda d: edit_source(d, 'service_api.py', 'TOPIC = "uci.survivability"',
                                                  'TOPIC = "uci.log"', resign=True),
             'generated service API has no unique SurvivabilityRiskLevel input exchange'),
            ('manifest-two-messages', lambda d: rewrite(d, lambda m: m['messages'].pop()),
             'does not describe closed-schema'),
            ('manifest-reordered', lambda d: rewrite(d, lambda m: m['messages'].reverse()),
             'does not describe closed-schema'),
            ('manifest-payload-as-message', lambda d: rewrite(
                d, lambda m: m['messages'][2].update(message='SurvivabilityRiskLevelMT')),
             'does not describe closed-schema'),
            ('manifest-wrong-topic', lambda d: rewrite(
                d, lambda m: m['messages'][2].update(topic='uci.log')), 'does not describe closed-schema'),
            ('manifest-no-marker', lambda d: rewrite(d, lambda m: m.pop('with_survivability')),
             'is not a python-uci-client build manifest'),
            ('manifest-marker-false', lambda d: rewrite(d, lambda m: m.update(with_survivability=False)),
             'is not a python-uci-client build manifest'),
            ('manifest-wrong-release', lambda d: rewrite(d, lambda m: m.update(release='2.6')),
             "generated for UCI '2.6', not --schema-release 2.5"),
            ('manifest-wrong-schema-version', lambda d: rewrite(
                d, lambda m: m.update(schema_version='002.6.0')), 'does not describe closed-schema'),
            ('manifest-unknown-format', lambda d: rewrite(d, lambda m: m.update(format=4)),
             'is not a python-uci-client build manifest')]
        with tempfile.TemporaryDirectory() as temporary:
            for name, edit, message in edits:
                with self.subTest(case=name):
                    copy = Path(temporary) / name
                    shutil.copytree(three('2.5'), copy)
                    edit(copy)
                    code, _, err = run_main(args('2.5', copy) + THREE)
                    self.assertEqual(code, client.EXIT_BINDINGS, err)
                    self.assertIn(message, err)
            # Formats 1 and 2 never carry the survivability marker.
            for source, extra in ((bindings_directory('2.5'), []),
                                  (bindings_directory('2.5', with_log=True), ['--with-log'])):
                with self.subTest(case=f'marked-format-{len(extra) + 1}'):
                    copy = Path(temporary) / f'marked-{len(extra)}'
                    shutil.copytree(source, copy)
                    rewrite(copy, lambda m: m.update(with_survivability=True))
                    code, _, err = run_main(args('2.5', copy) + extra)
                    self.assertEqual(code, client.EXIT_BINDINGS, err)
                    self.assertIn('is not a python-uci-client build manifest', err)
            # Format-2 sources relabelled as format 3: digests match, but the
            # generated API/codec have no Survivability endpoint or codec.
            copy = Path(temporary) / 'forged'
            shutil.copytree(bindings_directory('2.5', with_log=True), copy)
            rewrite(copy, lambda m: m.update(format=3, with_survivability=True, messages=MESSAGES))
            code, _, err = run_main(args('2.5', copy) + THREE)
            self.assertEqual(code, client.EXIT_BINDINGS, err)
            self.assertIn('generated service API has no unique SurvivabilityRiskLevel input exchange', err)
            # Format-3 sources relabelled as format 2: the extra input exchange is refused.
            copy = Path(temporary) / 'downgraded'
            shutil.copytree(three('2.5'), copy)
            rewrite(copy, lambda m: (m.pop('with_survivability'),
                                     m.update(format=2, messages=MESSAGES[:2])))
            code, _, err = run_main(args('2.5', copy) + ['--with-log'])
            self.assertEqual(code, client.EXIT_BINDINGS, err)
            self.assertIn('selects input exchanges other than RDMA_InitializeSetup + Log', err)


if __name__ == '__main__':
    unittest.main()
