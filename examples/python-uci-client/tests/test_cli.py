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
                     generator, golden, schema_root)

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

    def test_generator_output_is_unmodified_production_output(self):
        import subprocess
        with tempfile.TemporaryDirectory() as temporary:
            direct = Path(temporary) / 'direct'
            subprocess.run([str(generator()), 'service-generate', '--schema', str(schema_root('2.6')),
                            '--contract', str(APP / 'contracts/uci26-service.yaml'), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', str(direct)],
                           check=True, capture_output=True)
            built = bindings_directory('2.6')
            for name in client.GENERATED:
                self.assertEqual((direct / name).read_bytes(), (built / name).read_bytes(), name)

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


if __name__ == '__main__':
    unittest.main()
