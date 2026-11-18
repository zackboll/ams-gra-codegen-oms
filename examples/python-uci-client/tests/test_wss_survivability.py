"""Certificate-verified WSS, three-message mode, to real pinned Sleet (PY-OMS-022).

Reuses test_wss's TLS terminator (examples/rust-uci-client/tests/support/
tls_endpoint.py, unchanged) and isolated trust. Verification is never
disabled; trust is extended only by --ca-file. The untrusted-CA, wrong-hostname
and missing-owp controls stay in test_wss.py and apply unchanged.
"""
import ssl
import subprocess
import sys
import tempfile
import unittest

from support import TLS_HELPER, Broker, RawPeer, bindings_directory, cleanup_bindings, client, run_cli
import test_real_sleet_with_survivability as three
import test_wss as wss


def setUpModule():
    wss.CERTIFICATES = tempfile.TemporaryDirectory(prefix='py-oms-022-tls-')
    subprocess.run([sys.executable, str(TLS_HELPER), wss.CERTIFICATES.name, '--generate'],
                   check=True, capture_output=True)
    for release in ('2.5', '2.6'):
        bindings_directory(release, with_log=True, with_survivability=True)


def tearDownModule():
    cleanup_bindings()
    wss.CERTIFICATES.cleanup()


class WssSurvivability(unittest.IsolatedAsyncioTestCase):
    async def test_verified_wss_receives_rdma_log_and_survivability(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                schema = client.SCHEMA_VERSIONS[release]
                observer_tls = ssl.create_default_context(cafile=wss.ca())
                async with Broker(release, bindings=three.ALL) as broker, \
                        wss.Terminator(broker.port, 'valid') as tls:
                    url = f'wss://localhost:{tls.port}'
                    async with RawPeer(url, schema, ssl_context=observer_tls) as publisher, \
                            RawPeer(url, schema, ssl_context=observer_tls) as observer:
                        await three.witness_all(observer)
                        code, lines, stderr = await three.exercise(
                            self, release, url, publisher, observer,
                            extra=dict(ca_file=wss.ca()), env=wss.ISOLATED)
                    events = broker.events()
                sids = three.assert_session(self, release, code, lines, stderr)
                self.assertIn(f'CONNECTED wss://localhost:{tls.port} owp=1.0 '
                              f'service-id=python-uci-client schema={schema}', lines)
                three.assert_broker_ownership(self, events, sids, peers=2)
                self.assertIn('localhost', tls.handshakes())  # SNI carried the verified name
                print(f'PY-OMS-022 WSS UCI {release}: PASSED sids={",".join(sids)}', flush=True)

    async def test_three_message_mode_keeps_certificate_and_hostname_verification(self):
        for identity, host, ca_name, expected in (
                ('untrusted', 'localhost', 'ca', 'unable to get local issuer'),
                ('hostname', 'localhost', 'ca', 'Hostname mismatch')):
            with self.subTest(identity=identity):
                async with Broker('2.6', bindings=three.ALL) as broker, \
                        wss.Terminator(broker.port, identity) as tls:
                    code, lines, stderr = await run_cli(
                        three.cli_arguments(f'wss://{host}:{tls.port}', '2.6', ca_file=wss.ca(ca_name)),
                        env=wss.ISOLATED)
                    events = broker.events()
                self.assertEqual(code, client.EXIT_CONNECT, stderr)
                self.assertIn('CONNECT FAILED: TLS certificate verification failed', stderr)
                self.assertIn(expected, stderr)
                self.assertNotIn('initialized schema=', events)  # nothing reached OWP INIT
                self.assertEqual(lines, ['CLOSED received=0 rejected=0 callback-failures=0 '
                                         'RDMA_InitializeSetup=0/0 Log=0/0 SurvivabilityRiskLevel=0/0'])


if __name__ == '__main__':
    unittest.main()
