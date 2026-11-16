"""Certificate-verified WSS to real pinned Sleet through the existing TLS terminator.

Reuses examples/rust-uci-client/tests/support/tls_endpoint.py unchanged: it
generates a local CA, a rogue CA and leaf identities, and forwards bytes to
Sleet without interpreting OWP. Host trust stores are never modified; trust is
extended only by the application's --ca-file. Verification is never disabled.
"""
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import unittest

from websockets.asyncio.server import serve

from support import (TLS_HELPER, TOPIC, Broker, RawPeer, bindings_directory, cleanup_bindings,
                     client, free_port, run_cli, stop_process, wait_port)
from test_real_sleet import assert_session, cli_arguments, exercise

CERTIFICATES = None
# Child processes must not trust the generated CA through any host default.
ISOLATED = {'SSL_CERT_FILE': os.devnull, 'SSL_CERT_DIR': os.devnull}


def setUpModule():
    global CERTIFICATES
    CERTIFICATES = tempfile.TemporaryDirectory(prefix='py-oms-020-tls-')
    subprocess.run([sys.executable, str(TLS_HELPER), CERTIFICATES.name, '--generate'],
                   check=True, capture_output=True)
    for release in ('2.5', '2.6'):
        bindings_directory(release)


def tearDownModule():
    cleanup_bindings()
    CERTIFICATES.cleanup()


def ca(name='ca'):
    return str(Path(CERTIFICATES.name) / f'{name}.pem')


class Terminator:
    """TLS front for one backend port, serving the named leaf identity."""

    def __init__(self, backend, identity):
        self.backend, self.identity, self.port = backend, identity, free_port()

    async def __aenter__(self):
        self.process = subprocess.Popen(
            [sys.executable, str(TLS_HELPER), CERTIFICATES.name, '--backend', str(self.backend),
             '--port', str(self.port), '--identity', self.identity, '--epoch', self.identity],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        await wait_port(self.port, self.process)
        return self

    async def __aexit__(self, *exc):
        await stop_process(self.process)

    def handshakes(self):
        path = Path(CERTIFICATES.name) / f'{self.identity}-handshakes.txt'
        return path.read_text().split() if path.exists() else []


class Wss(unittest.IsolatedAsyncioTestCase):
    async def test_verified_wss_receives_authentic_rdma(self):
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                schema = client.SCHEMA_VERSIONS[release]
                observer_tls = ssl.create_default_context(cafile=ca())
                async with Broker(release) as broker, Terminator(broker.port, 'valid') as tls:
                    url = f'wss://localhost:{tls.port}'
                    async with RawPeer(url, schema, ssl_context=observer_tls) as publisher, \
                            RawPeer(url, schema, ssl_context=observer_tls) as observer:
                        self.assertEqual(await observer.status(
                            f'SUB witness RDMA_InitializeSetup {TOPIC}'), ('+OK', []))
                        code, lines, stderr = await exercise(
                            self, release, url, publisher, observer,
                            extra=dict(ca_file=ca()), env=ISOLATED)
                    events = broker.events()
                sid = assert_session(self, release, code, lines, stderr)
                self.assertIn(f'CONNECTED wss://localhost:{tls.port} owp=1.0 '
                              f'service-id=python-uci-client schema={schema}', lines)
                self.assertIn(f'subscription removed sid={sid}', events)
                self.assertIn('localhost', tls.handshakes())  # SNI carried the verified name
                print(f'PY-OMS-020 WSS UCI {release}: PASSED sid={sid}', flush=True)

    async def assert_rejected(self, identity, host, ca_name, expected):
        async with Broker('2.5') as broker, Terminator(broker.port, identity) as tls:
            code, lines, stderr = await run_cli(
                cli_arguments(f'wss://{host}:{tls.port}', '2.5', ca_file=ca(ca_name)), env=ISOLATED)
            events = broker.events()
        self.assertEqual(code, client.EXIT_CONNECT, stderr)
        self.assertIn('CONNECT FAILED: TLS certificate verification failed', stderr)
        self.assertIn(expected, stderr)
        self.assertNotIn('initialized schema=', events)  # nothing reached OWP INIT
        self.assertEqual(lines, ['CLOSED received=0 rejected=0 callback-failures=0'])

    async def test_untrusted_certificate_is_rejected(self):
        await self.assert_rejected('untrusted', 'localhost', 'ca', 'unable to get local issuer')
        # Trusting only the rogue CA cannot validate the legitimate server either.
        await self.assert_rejected('valid', 'localhost', 'rogue-ca', 'unable to get local issuer')
        # Without --ca-file only (isolated) system trust applies.
        async with Broker('2.5') as broker, Terminator(broker.port, 'valid') as tls:
            code, _, stderr = await run_cli(cli_arguments(f'wss://localhost:{tls.port}', '2.5'),
                                            env=ISOLATED)
        self.assertEqual(code, client.EXIT_CONNECT, stderr)
        self.assertIn('TLS certificate verification failed', stderr)

    async def test_wrong_hostname_is_rejected(self):
        await self.assert_rejected('hostname', 'localhost', 'ca', 'Hostname mismatch')
        await self.assert_rejected('valid', '127.0.0.1', 'ca', 'IP address mismatch')

    async def test_wss_without_owp_negotiation_is_rejected(self):
        frames = []

        async def handler(ws):
            async for frame in ws:
                frames.append(frame)

        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(Path(CERTIFICATES.name) / 'valid.pem',
                                Path(CERTIFICATES.name) / 'valid.key')
        async with serve(handler, '127.0.0.1', 0, ssl=context) as server:  # no subprotocols
            port = server.sockets[0].getsockname()[1]
            code, _, stderr = await run_cli(
                cli_arguments(f'wss://localhost:{port}', '2.5', ca_file=ca()), env=ISOLATED)
        self.assertEqual(code, client.EXIT_CONNECT, stderr)
        self.assertIn('CONNECT FAILED (ProtocolError', stderr)
        self.assertEqual(frames, [])  # INIT is never sent without negotiated owp


if __name__ == '__main__':
    unittest.main()
