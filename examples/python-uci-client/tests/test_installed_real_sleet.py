"""PY-OMS-023: the INSTALLED python-uci-client console script against real Sleet.

The wheel is built, installed offline into a fresh environment and launched
from a directory outside the repository, with deployed copies of the
generated bindings (see installed.py). The PY-OMS-020/021/022 campaign helpers
are reused unchanged: the same unchanged pinned Sleet broker configuration,
the same independent raw-websocket publisher/observer, the same golden
messages and the same assertions. Only the launched program differs:
support.LAUNCHER selects the installed `python-uci-client` executable and
binding directories resolve to the deployed copies.

This is a representative subset, not a fourth copy of the full campaigns:
three-message sessions for UCI 2.5 and 2.6, one RDMA-only and one --with-log
session, an unauthorized third subscription, a signal stop, and one verified
WSS session with its hostname-verification negative control.
"""
import asyncio
import re
import signal
import ssl
import subprocess
import sys
import tempfile
import unittest

import support
from support import TLS_HELPER, Broker, RawPeer, client, run_cli
import installed
import test_real_sleet as rdma_only
import test_real_sleet_with_log as with_log
import test_real_sleet_with_survivability as three
import test_wss as wss

INSTALLATION = None
_ORIGINAL = {}
# Module globals the reused helpers resolve bindings through.
_REBOUND = (rdma_only, with_log, three)


def setUpModule():
    global INSTALLATION
    INSTALLATION = installed.Installation().setup()
    support.LAUNCHER = INSTALLATION
    for module in _REBOUND:
        _ORIGINAL[module] = module.bindings_directory
        module.bindings_directory = INSTALLATION.deployed
    wss.CERTIFICATES = tempfile.TemporaryDirectory(prefix='py-oms-023-tls-')
    subprocess.run([sys.executable, str(TLS_HELPER), wss.CERTIFICATES.name, '--generate'],
                   check=True, capture_output=True)


def tearDownModule():
    try:
        support.LAUNCHER = None
        for module, original in _ORIGINAL.items():
            module.bindings_directory = original
        if wss.CERTIFICATES is not None:
            wss.CERTIFICATES.cleanup()
        if INSTALLATION is not None:
            INSTALLATION.cleanup()
    finally:
        support.cleanup_bindings()


class InstalledRealSleet(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.launches = INSTALLATION.launches

    def tearDown(self):
        # Every CLI session of this test was the installed console script.
        self.assertGreater(INSTALLATION.launches, self.launches)

    def assert_installed_session(self, stderr):
        self.assertNotIn('DECOY', stderr)
        self.assertNotIn(str(support.ROOT), stderr)

    async def three_message_release(self, release):
        schema = client.SCHEMA_VERSIONS[release]
        async with Broker(release, bindings=three.ALL) as broker:
            async with RawPeer(broker.url, schema) as publisher, RawPeer(broker.url, schema) as observer:
                await three.witness_all(observer)
                code, lines, stderr = await three.exercise(self, release, broker.url, publisher, observer)
                await asyncio.sleep(.2)
                events = broker.events()
        self.assertEqual(broker.process.returncode, 0)  # clean broker termination
        self.assert_installed_session(stderr)
        sids = three.assert_session(self, release, code, lines, stderr)
        self.assertIn(f'CONNECTED ws://127.0.0.1:{broker.port} owp=1.0 service-id=python-uci-client '
                      f'schema={schema}', lines)
        three.assert_broker_ownership(self, events, sids, peers=2)
        print(f'PY-OMS-023 INSTALLED REAL SLEET UCI {release}: PASSED received=9 rdma=3 log=3 '
              f'survivability=3 decoder-rejected=1 sids={",".join(sids)}', flush=True)

    async def test_uci25_installed_three_messages(self):
        await self.three_message_release('2.5')

    async def test_uci26_installed_three_messages(self):
        await self.three_message_release('2.6')

    async def test_installed_rdma_only_and_with_log_modes(self):
        # Mode A (format 1) on UCI 2.5 and mode B (format 2) on UCI 2.6.
        async with Broker('2.5') as broker, RawPeer(broker.url, '002.5.0') as publisher, \
                RawPeer(broker.url, '002.5.0') as observer:
            self.assertEqual(await observer.status(f'SUB witness RDMA_InitializeSetup {support.TOPIC}'),
                             ('+OK', []))
            code, lines, stderr = await rdma_only.exercise(self, '2.5', broker.url, publisher, observer)
            await asyncio.sleep(.2)
            events = broker.events()
        self.assert_installed_session(stderr)
        sid = rdma_only.assert_session(self, '2.5', code, lines, stderr)
        self.assertIn(f'subscription removed sid={sid}', events)
        self.assertEqual(events.count('dispatch UNSUB'), 1)
        async with Broker('2.6', bindings=with_log.BOTH) as broker, \
                RawPeer(broker.url, '002.6.0') as publisher, RawPeer(broker.url, '002.6.0') as observer:
            await with_log.witness_both(observer)
            code, lines, stderr = await with_log.exercise(self, '2.6', broker.url, publisher, observer)
            await asyncio.sleep(.2)
            events = broker.events()
        self.assert_installed_session(stderr)
        rdma_sid, log_sid = with_log.assert_session(self, '2.6', code, lines, stderr)
        with_log.assert_broker_ownership(self, events, rdma_sid, log_sid, peers=2)
        print(f'PY-OMS-023 INSTALLED MODES A/B: PASSED rdma-only-2.5 sid={sid} '
              f'with-log-2.6 sids={rdma_sid},{log_sid}', flush=True)

    async def test_installed_unauthorized_third_subscription_exits_5(self):
        async with Broker('2.6', bindings=with_log.BOTH) as broker:
            code, lines, stderr = await run_cli(three.cli_arguments(broker.url, '2.6'))
            await asyncio.sleep(.2)
            events = broker.events()
        self.assert_installed_session(stderr)
        self.assertEqual(code, client.EXIT_REJECTED, stderr)
        self.assertIn('SUBSCRIBE REJECTED by server: Illegal-State message=SurvivabilityRiskLevel', stderr)
        output = '\n'.join(lines)
        self.assertNotIn('READY', output)
        rdma_sid = re.search(r'^SUBSCRIBED RDMA_InitializeSetup topic=uci\.rdma sid=(\S+)', output, re.M)[1]
        log_sid = re.search(r'^SUBSCRIBED Log topic=uci\.log sid=(\S+)', output, re.M)[1]
        self.assertEqual(lines[-3:], [
            f'UNSUBSCRIBED sid={log_sid} message=Log',
            f'UNSUBSCRIBED sid={rdma_sid} message=RDMA_InitializeSetup',
            'CLOSED received=0 rejected=0 callback-failures=0 '
            'RDMA_InitializeSetup=0/0 Log=0/0 SurvivabilityRiskLevel=0/0'])
        self.assertEqual(events.count('subscription registered'), 2)
        self.assertEqual(events.count('dispatch UNSUB'), 2)
        print('PY-OMS-023 INSTALLED UNAUTHORIZED THIRD SUB: PASSED exit=5', flush=True)

    async def test_installed_sigterm_releases_all_subscriptions(self):
        async with Broker('2.6', bindings=three.ALL) as broker, RawPeer(broker.url, '002.6.0') as publisher:
            async def drive(line, process):
                if line.startswith('READY '):
                    self.assertEqual(await publisher.publish_survivability(
                        support.survivability_golden('2.6')), ('+OK', []))
                elif line.startswith('RECEIVED '):
                    process.send_signal(signal.SIGTERM)

            code, lines, stderr = await run_cli(three.cli_arguments(broker.url, '2.6', duration_seconds='60'),
                                                until=drive, timeout=30)
            await asyncio.sleep(.2)
            events = broker.events()
        self.assert_installed_session(stderr)
        self.assertEqual(code, 143, stderr)  # 128 + SIGTERM through the console-script launcher
        self.assertEqual([line.split()[-1] for line in lines[-4:-1]],
                         ['message=SurvivabilityRiskLevel', 'message=Log', 'message=RDMA_InitializeSetup'])
        self.assertEqual(lines[-1], 'CLOSED received=1 rejected=0 callback-failures=0 '
                                    'RDMA_InitializeSetup=0/0 Log=0/0 SurvivabilityRiskLevel=1/0')
        self.assertEqual(events.count('subscription removed sid='), 3)
        print('PY-OMS-023 INSTALLED SIGTERM: PASSED exit=143 unsubscribed=3', flush=True)

    async def test_installed_verified_wss(self):
        release, schema = '2.6', client.SCHEMA_VERSIONS['2.6']
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
        self.assert_installed_session(stderr)
        sids = three.assert_session(self, release, code, lines, stderr)
        self.assertIn(f'CONNECTED wss://localhost:{tls.port} owp=1.0 '
                      f'service-id=python-uci-client schema={schema}', lines)
        three.assert_broker_ownership(self, events, sids, peers=2)
        self.assertIn('localhost', tls.handshakes())  # SNI carried the verified name
        # Negative control: hostname verification stays enabled when installed.
        async with Broker(release, bindings=three.ALL) as broker, \
                wss.Terminator(broker.port, 'hostname') as tls:
            code, lines, stderr = await run_cli(
                three.cli_arguments(f'wss://localhost:{tls.port}', release, ca_file=wss.ca()),
                env=wss.ISOLATED)
            events = broker.events()
        self.assertEqual(code, client.EXIT_CONNECT, stderr)
        self.assertIn('CONNECT FAILED: TLS certificate verification failed', stderr)
        self.assertIn('Hostname mismatch', stderr)
        self.assertNotIn('initialized schema=', events)  # nothing reached OWP INIT
        print(f'PY-OMS-023 INSTALLED WSS UCI {release}: PASSED sids={",".join(sids)} '
              'hostname-mismatch=refused', flush=True)


if __name__ == '__main__':
    unittest.main()
