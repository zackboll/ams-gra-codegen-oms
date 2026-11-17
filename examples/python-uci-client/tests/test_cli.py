"""CLI arguments, exit codes, release selection and generated package loading.

No test here opens a network connection: argument and binding errors must be
reported before any connection attempt (asserted by a refusing socket patch).
"""
import contextlib
import io
import json
from pathlib import Path
import shutil
import socket
import ssl
import tempfile
import unittest
from unittest import mock

from support import (APP, bindings_directory, cleanup_bindings, client, generate_bindings,
                     generator, golden, log_golden, schema_root)

URL = 'ws://127.0.0.1:9'


def tearDownModule():
    cleanup_bindings()


def args(release='2.5', bindings='unused', **extra):
    result = ['--url', URL, '--service-id', 'python-uci-client',
              '--schema-release', release, '--bindings', str(bindings)]
    for key, value in extra.items():
        result += ['--' + key.replace('_', '-'), str(value)]
    return result


@contextlib.contextmanager
def no_network():
    def refuse(*_args, **_kwargs):
        raise AssertionError('network access attempted before validation completed')
    with mock.patch.object(socket, 'create_connection', refuse), \
            mock.patch.object(socket.socket, 'connect', refuse):
        yield


def run_main(argv):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err), no_network():
        code = client.main(argv)
    return code, out.getvalue(), err.getvalue()


class Arguments(unittest.TestCase):
    def test_help_prints_usage_and_exit_statuses(self):
        code, out, err = run_main(['--help'])
        self.assertEqual((code, err), (0, ''))
        for text in ('--url', '--service-id', '--schema-release', '--duration-seconds',
                     '--ca-file', '--bindings', '--max-messages', 'Exit status'):
            self.assertIn(text, out)

    def test_valid_arguments_and_defaults(self):
        options = client.parse_arguments(args('2.6'))
        self.assertEqual((options.schema_release, options.duration, options.max_messages,
                          options.ca_file), ('2.6', client.DEFAULT_DURATION, None, None))
        options = client.parse_arguments(args(duration_seconds=86400, max_messages=3))
        self.assertEqual((options.duration, options.max_messages), (86400, 3))
        for url in ('ws://localhost', 'ws://[::1]:9000/owp', 'wss://broker.example:443/x?y=1'):
            client.parse_arguments(['--url', url] + args()[2:])

    def test_malformed_and_contradictory_arguments_exit_2_without_network(self):
        cases = [
            [], ['--help', '--url', URL], args() + ['--help'], ['--wat'], args() + ['--verbose'],
            args()[:-2], args() + ['--url', URL], ['--url'] + args()[2:] + ['--url'],
            ['--url', '--service-id'] + args()[2:], ['--url', ''] + args()[2:],
            args(release='2.7'), args(release='25'), args(release=''),
            args(duration_seconds=0), args(duration_seconds=86401), args(duration_seconds=-1),
            args(duration_seconds='1.5'), args(duration_seconds='07'), args(duration_seconds='١'),
            args(max_messages=0), args(max_messages='x'),
            # Oversized values are rejected before int(), both inside and past
            # sys.get_int_max_str_digits() (4300 by default).
            args(duration_seconds='9' * 6), args(max_messages='1' + '0' * 6 + '1'),
            args(duration_seconds='9' * 5000), args(max_messages='9' * 5000),
            args(duration_seconds='1' * 100_000), args(max_messages='0' * 5000),
            ['--url', 'http://127.0.0.1'] + args()[2:], ['--url', 'ws://'] + args()[2:],
            ['--url', 'ws://user:pw@host'] + args()[2:], ['--url', 'ws://host/#frag'] + args()[2:],
            ['--url', 'tcp://host:1'] + args()[2:],
            args()[:2] + ['--service-id', 'bad id'] + args()[4:],
            args()[:2] + ['--service-id', 'ünicode'] + args()[4:],
            # --ca-file is meaningful only for verified wss://, and must be a real CA file.
            args(ca_file=__file__),
            ['--url', 'wss://localhost'] + args()[2:] + ['--ca-file', '/nonexistent/ca.pem'],
            ['--url', 'wss://localhost'] + args()[2:] + ['--ca-file', __file__],
            ['--url=ws://x'] + args()[2:], args()[:-1] + [Path('b')],
        ]
        for argv in cases:
            with self.subTest(argv=argv):
                code, out, err = run_main(argv)
                self.assertEqual(code, client.EXIT_USAGE, err)
                self.assertEqual(out, '')
                self.assertIn('error:', err)

    def test_wss_ca_file_extends_but_never_disables_verification(self):
        with tempfile.TemporaryDirectory() as temporary:
            ca = Path(temporary) / 'ca.pem'
            ca.write_text(ssl.DER_cert_to_PEM_cert(self.self_signed_der(temporary)))
            options = client.parse_arguments(['--url', 'wss://localhost'] + args()[2:] + ['--ca-file', str(ca)])
            config = client.runtime_config(options)
            self.assertEqual(config.tls_context.verify_mode, ssl.CERT_REQUIRED)
            self.assertTrue(config.tls_context.check_hostname)
            self.assertGreaterEqual(config.tls_context.cert_store_stats()['x509_ca'], 1)
        config = client.runtime_config(client.parse_arguments(['--url', 'wss://localhost'] + args()[2:]))
        self.assertEqual(config.tls_context.verify_mode, ssl.CERT_REQUIRED)
        self.assertTrue(config.tls_context.check_hostname)
        self.assertIsNone(client.runtime_config(client.parse_arguments(args())).tls_context)

    @staticmethod
    def self_signed_der(directory):
        import subprocess
        key, cert = Path(directory) / 'k.pem', Path(directory) / 'c.pem'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(key),
                        '-out', str(cert), '-days', '1', '-subj', '/CN=test-ca',
                        '-addext', 'basicConstraints=critical,CA:TRUE'],
                       check=True, capture_output=True)
        return ssl.PEM_cert_to_DER_cert(cert.read_text())

    def test_selected_release_fixes_init_schema_version(self):
        for release, version in (('2.5', '002.5.0'), ('2.6', '002.6.0')):
            config = client.runtime_config(client.parse_arguments(args(release)))
            self.assertEqual(config.schema_version, version)
            self.assertEqual((config.versions, config.verbose), (('1.0',), True))


class GeneratedBindings(unittest.TestCase):
    """Actual production generation for both authentic releases."""

    def test_actual_generated_package_for_each_release(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                directory = bindings_directory(release)
                manifest = json.loads((directory / generate_bindings.MANIFEST).read_text())
                self.assertEqual((manifest['release'], manifest['schema_version'], manifest['message'],
                                  manifest['payload_type'], manifest['world'], manifest['with_codec']),
                                 (release, client.SCHEMA_VERSIONS[release], 'RDMA_InitializeSetup',
                                  'RDMA_InitializeSetupMT', 'closed-schema', True))
                self.assertEqual(sorted(p.name for p in directory.iterdir()),
                                 sorted([*client.GENERATED, generate_bindings.MANIFEST]))
                bindings = client.load_bindings(directory, release)
                receive = bindings.receive
                # Global message identity is distinct from the generated payload type.
                self.assertEqual((receive.MESSAGE_NAMESPACE, receive.MESSAGE_NAME, receive.TOPIC,
                                  receive.DIRECTION, receive.SUBSCRIPTION_GROUP),
                                 (client.OAM, 'RDMA_InitializeSetup', 'uci.rdma', 'input', None))
                self.assertIs(receive.Payload, bindings.model.RDMA_InitializeSetupMT)
                self.assertEqual(bindings.payload_type.__schema_name__, (client.OAM, 'RDMA_InitializeSetupMT'))
                binding = bindings.registry.lookup(client.OAM, 'RDMA_InitializeSetup')
                self.assertIs(binding.decode, bindings.codec.decode_RDMA_InitializeSetupMT)
                self.assertIs(binding.error_type, bindings.codec.CodecError)
                value = bindings.codec.decode_RDMA_InitializeSetupMT(json.dumps(golden(release)))
                self.assertIs(type(value), bindings.model.RDMA_InitializeSetupMT)
                self.assertEqual(json.loads(bindings.codec.encode_RDMA_InitializeSetupMT(value)), golden(release))

    def test_manifest_binds_pins_contract_generator_and_generated_files(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                directory = bindings_directory(release)
                manifest = json.loads((directory / generate_bindings.MANIFEST).read_text())
                pin = generate_bindings.PINS[release]
                contract = generate_bindings.contract(release)
                self.assertEqual(manifest['schema_sha256'], generate_bindings.digest(schema_root(release)))
                self.assertEqual(manifest['schema_sha256'], pin['root'])
                self.assertEqual(manifest['include'], {pin['include'][0]: pin['include'][1]})
                self.assertEqual((manifest['contract'], manifest['contract_sha256']),
                                 (contract.relative_to(APP.parents[1]).as_posix(),
                                  generate_bindings.digest(contract)))
                self.assertEqual(manifest['generator_sha256'], generate_bindings.digest(generator()))
                self.assertEqual(manifest['files'], {name: generate_bindings.digest(directory / name)
                                                     for name in client.GENERATED})
                # Unmodified production output: the generator's own header.
                self.assertTrue((directory / 'service_api.py').read_text().startswith(
                    '# Generated Python service API.'))
                self.assertIn('MESSAGE_NAME = "RDMA_InitializeSetup"',
                              (directory / 'service_api.py').read_text())

    def test_release_mismatch_and_tampering_exit_3_without_network(self):
        code, _, err = run_main(args('2.6', bindings_directory('2.5')))
        self.assertEqual(code, client.EXIT_BINDINGS)
        self.assertIn("generated for UCI '2.5', not --schema-release 2.6", err)
        with tempfile.TemporaryDirectory() as temporary:
            copy = Path(temporary) / 'b'
            shutil.copytree(bindings_directory('2.5'), copy)
            code, _, err = run_main(args('2.5', Path(temporary) / 'missing'))
            self.assertEqual(code, client.EXIT_BINDINGS)
            with (copy / 'service_codec.py').open('a') as tampered:
                tampered.write('\n# modified\n')
            code, _, err = run_main(args('2.5', copy))
            self.assertEqual(code, client.EXIT_BINDINGS)
            self.assertIn('service_codec.py was modified', err)
            (copy / 'service_codec.py').unlink()
            code, _, err = run_main(args('2.5', copy))
            self.assertEqual(code, client.EXIT_BINDINGS)
            (copy / generate_bindings.MANIFEST).write_text('{"format": 1}')
            code, _, err = run_main(args('2.5', copy))
            self.assertEqual(code, client.EXIT_BINDINGS)

    def test_generation_helper_refuses_wrong_or_unsafe_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / 'out'
            for release, schema, output, message in [
                    ('2.6', schema_root('2.5'), target, 'not the pinned UCI 2.6 root'),
                    ('2.5', schema_root('2.5'), APP / 'build-output', 'inside the repository'),
                    ('2.4', schema_root('2.5'), target, 'unsupported schema release'),
                    ('2.5', Path(temporary) / 'missing.xsd', target, 'schema root not found')]:
                with self.subTest(message=message):
                    with self.assertRaises(generate_bindings.BuildError) as caught:
                        generate_bindings.generate(release, schema, output, generator())
                    self.assertIn(message, str(caught.exception))
            target.mkdir()
            (target / 'existing').write_text('user data')
            with self.assertRaises(generate_bindings.BuildError):
                generate_bindings.generate('2.5', schema_root('2.5'), target, generator())
            self.assertEqual((target / 'existing').read_text(), 'user data')
            self.assertFalse((APP / 'build-output').exists())
        self.check_repository_reached_through_symlinked_ancestor_is_refused()

    def check_repository_reached_through_symlinked_ancestor_is_refused(self):
        # Part of the test above: the qualification lane pins this suite's test count.
        repository = APP.parents[1]
        before = sorted(p.name for p in APP.iterdir()), sorted(p.name for p in repository.iterdir())
        with tempfile.TemporaryDirectory() as temporary:
            temporary = Path(temporary)
            (temporary / 'app').symlink_to(APP, target_is_directory=True)
            (temporary / 'repo').symlink_to(repository, target_is_directory=True)
            (temporary / 'dangling').symlink_to(APP / 'not-yet-created')
            (temporary / 'chain').symlink_to(temporary / 'app')
            for output in (temporary / 'app' / 'build-output', temporary / 'repo' / 'x' / 'y',
                           temporary / 'dangling' / 'out', temporary / 'chain' / 'out'):
                with self.subTest(output=output):
                    with self.assertRaises(generate_bindings.BuildError) as caught:
                        generate_bindings.generate('2.5', schema_root('2.5'), output, generator())
                    self.assertIn('inside the repository', str(caught.exception))
            (temporary / 'loop-a').symlink_to(temporary / 'loop-b')
            (temporary / 'loop-b').symlink_to(temporary / 'loop-a')
            with self.assertRaises(generate_bindings.BuildError) as caught:
                generate_bindings.generate('2.5', schema_root('2.5'), temporary / 'loop-a' / 'out',
                                           generator())
            self.assertIn('cannot resolve output directory', str(caught.exception))
            # A symlinked ancestor that leads OUTSIDE the repository stays usable,
            # and the real (resolved) destination is what gets written.
            (temporary / 'external').mkdir()
            (temporary / 'external-alias').symlink_to(temporary / 'external')
            _, _, output, _ = generate_bindings._check_inputs(
                '2.5', schema_root('2.5'), temporary / 'external-alias' / 'out', generator())
            self.assertEqual(output, (temporary / 'external').resolve() / 'out')
        self.assertEqual(before, (sorted(p.name for p in APP.iterdir()),
                                  sorted(p.name for p in repository.iterdir())))


def rewrite(directory, change):
    path = directory / generate_bindings.MANIFEST
    manifest = json.loads(path.read_text())
    change(manifest)
    path.write_text(json.dumps(manifest))


def edit_source(directory, name, old, new, resign=False):
    path = directory / name
    text = path.read_text()
    if old not in text:
        raise AssertionError(f'{old!r} not found in generated {name}')
    path.write_text(text.replace(old, new))
    if resign:  # isolate the API/codec check from the digest check
        rewrite(directory, lambda m: m['files'].update({name: generate_bindings.digest(path)}))


class CombinedBindings(unittest.TestCase):
    """PY-OMS-021 --with-log: one combined generated package, two typed families."""

    def test_with_log_argument_and_default_mode(self):
        self.assertFalse(client.parse_arguments(args()).with_log)
        self.assertTrue(client.parse_arguments(args() + ['--with-log']).with_log)
        self.assertTrue(client.parse_arguments(['--with-log'] + args('2.6')).with_log)
        for argv in (args() + ['--with-log', '--with-log'], args() + ['--with-log=yes'],
                     args() + ['--with-log', 'true'], args() + ['--with-logs'],
                     ['--with-log', '--help']):
            with self.subTest(argv=argv):
                code, out, err = run_main(argv)
                self.assertEqual((code, out), (client.EXIT_USAGE, ''), err)
        code, out, _ = run_main(['--help'])
        self.assertIn('--with-log', out)

    def test_actual_combined_package_for_each_release(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                directory = bindings_directory(release, with_log=True)
                manifest = json.loads((directory / generate_bindings.MANIFEST).read_text())
                self.assertEqual((manifest['format'], manifest['with_log'], manifest['release'],
                                  manifest['schema_version'], manifest['world'], manifest['with_codec']),
                                 (2, True, release, client.SCHEMA_VERSIONS[release], 'closed-schema', True))
                self.assertEqual(manifest['messages'], [
                    dict(message='RDMA_InitializeSetup', payload_type='RDMA_InitializeSetupMT',
                         topic='uci.rdma'),
                    dict(message='Log', payload_type='LogMT', topic='uci.log')])
                self.assertNotIn('message', manifest)
                contract = generate_bindings.contract(release, with_log=True)
                self.assertEqual((manifest['contract'], manifest['contract_sha256']),
                                 (contract.relative_to(APP.parents[1]).as_posix(),
                                  generate_bindings.digest(contract)))
                self.assertEqual(manifest['files'], {name: generate_bindings.digest(directory / name)
                                                     for name in client.GENERATED})
                bindings = client.load_bindings(directory, release, with_log=True)
                self.assertTrue(bindings.with_log)
                model, codec = bindings.model, bindings.codec
                identities = [(f.message, f.payload_name, f.topic, f.receive.MESSAGE_NAME,
                               f.receive.TOPIC, f.receive.Payload) for f in bindings.families]
                self.assertEqual(identities, [
                    ('RDMA_InitializeSetup', 'RDMA_InitializeSetupMT', 'uci.rdma',
                     'RDMA_InitializeSetup', 'uci.rdma', model.RDMA_InitializeSetupMT),
                    ('Log', 'LogMT', 'uci.log', 'Log', 'uci.log', model.LogMT)])
                # The wire global element is not the payload class name.
                self.assertEqual(model.LogMT.__schema_name__, (client.OAM, 'LogMT'))
                self.assertIs(bindings.receive, bindings.families[0].receive)
                log = bindings.registry.lookup(client.OAM, 'Log')
                self.assertEqual((log.payload_type, log.decode, log.error_type),
                                 (model.LogMT, codec.decode_LogMT, codec.CodecError))
                with self.assertRaises(client.InvalidInput):
                    bindings.registry.lookup(client.OAM, 'LogMT')
                value = codec.decode_LogMT(json.dumps(log_golden(release)))
                self.assertIs(type(value), model.LogMT)
                self.assertEqual(json.loads(codec.encode_LogMT(value)), log_golden(release))
                self.assertEqual(client.message_name(value), 'Log')
                rdma = codec.decode_RDMA_InitializeSetupMT(json.dumps(golden(release)))
                self.assertEqual(client.message_name(rdma), 'RDMA_InitializeSetup')
                self.assertEqual(client.summarize_log(value)['severity'], 'INFO')

    def test_mode_release_and_tamper_mismatches_exit_3_without_network(self):
        cases = [
            (args('2.5', bindings_directory('2.5')) + ['--with-log'], 'select only RDMA_InitializeSetup'),
            (args('2.5', bindings_directory('2.5', with_log=True)), 'were generated with --with-log'),
            (args('2.6', bindings_directory('2.5', with_log=True)) + ['--with-log'],
             "generated for UCI '2.5', not --schema-release 2.6")]
        for argv, message in cases:
            with self.subTest(message=message):
                code, _, err = run_main(argv)
                self.assertEqual(code, client.EXIT_BINDINGS, err)
                self.assertIn(message, err)
        log_codec = 'def decode_LogMT('
        edits = [
            ('modified-codec', lambda d: edit_source(d, 'service_codec.py', log_codec, 'def _x_LogMT('),
             'service_codec.py was modified'),
            ('modified-api', lambda d: edit_source(d, 'service_api.py', 'uci.log', 'uci.lag'),
             'service_api.py was modified'),
            # Re-signed: digests match, but the generated LogMT decoder is gone.
            ('missing-log-codec', lambda d: edit_source(d, 'service_codec.py', log_codec,
                                                        'def _x_LogMT(', resign=True),
             'generated service codec lacks the LogMT codec'),
            # Re-signed: digests match, but the generated Log endpoint is gone.
            ('missing-log-api', lambda d: edit_source(d, 'service_api.py', 'MESSAGE_NAME = "Log"',
                                                      'MESSAGE_NAME = "Lag"', resign=True),
             'generated service API has no unique Log input exchange'),
            ('manifest-without-log', lambda d: rewrite(d, lambda m: m['messages'].pop()),
             'does not describe closed-schema'),
            ('manifest-renamed-payload', lambda d: rewrite(
                d, lambda m: m['messages'][1].update(payload_type='LogMDT')),
             'does not describe closed-schema'),
            ('manifest-mixed-format', lambda d: rewrite(d, lambda m: m.update(message='Log')),
             'does not describe closed-schema'),
            ('manifest-unknown-format', lambda d: rewrite(d, lambda m: m.update(format=3)),
             'is not a python-uci-client build manifest')]
        with tempfile.TemporaryDirectory() as temporary:
            for name, edit, message in edits:
                with self.subTest(case=name):
                    copy = Path(temporary) / name
                    shutil.copytree(bindings_directory('2.5', with_log=True), copy)
                    edit(copy)
                    code, _, err = run_main(args('2.5', copy) + ['--with-log'])
                    self.assertEqual(code, client.EXIT_BINDINGS, err)
                    self.assertIn(message, err)

    def test_combined_manifest_cannot_relabel_rdma_only_sources(self):
        # RDMA-only generated files under a forged combined manifest: digests
        # match, but the generated API/codec have no Log endpoint or LogMT codec.
        with tempfile.TemporaryDirectory() as temporary:
            copy = Path(temporary) / 'forged'
            shutil.copytree(bindings_directory('2.5'), copy)
            rewrite(copy, lambda m: (m.pop('message'), m.pop('payload_type'), m.update(
                format=2, with_log=True,
                messages=[dict(item) for item in generate_bindings.COMBINED_MESSAGES])))
            code, _, err = run_main(args('2.5', copy) + ['--with-log'])
            self.assertEqual(code, client.EXIT_BINDINGS, err)
            self.assertIn('generated service API has no unique Log input exchange', err)


if __name__ == '__main__':
    unittest.main()
