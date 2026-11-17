"""Real unchanged pinned Sleet: the actual CLI receives RDMA and Log on ONE connection.

PY-OMS-021 (--with-log), separately for UCI 2.5 and 2.6. The broker authorizes
uci.rdma for RDMA_InitializeSetup and uci.log for Log. An independent
raw-websocket publisher/observer (support.RawPeer, never oms_lacal or the app)
publishes PY-016 Log goldens and Task092 RDMA goldens (targeted edits only),
interleaved, and witnesses each MSG on both topics. Broker rejection
(publisher -ERR, no MSG at the observer) is kept distinct from generated
decoder rejection (publisher +OK, MSG witnessed, CLI CodecFailure).
"""
import asyncio
import json
import re
import signal
import unittest

from support import (LOG_TOPIC, TOPIC, Broker, RawPeer, bindings_directory, cleanup_bindings,
                     client, log_golden, log_variant, run_cli, variant)
from test_real_sleet import UUIDS, V4, V6

BOTH = {TOPIC: ('RDMA_InitializeSetup',), LOG_TOPIC: ('Log',)}


def tearDownModule():
    cleanup_bindings()


def cli_arguments(url, release, with_log=True, **extra):
    arguments = ['--url', url, '--service-id', 'python-uci-client', '--schema-release', release,
                 '--bindings', bindings_directory(release, with_log=with_log)]
    if with_log:
        arguments.append('--with-log')
    for key, value in extra.items():
        arguments += ['--' + key.replace('_', '-'), value]
    return arguments


def rdma_bodies(release):
    return [variant(release, UUIDS[0], [V4], 'TCP'),
            variant(release, UUIDS[1], [V6, V4], 'VERBS', '2026-06-30T23:59:59.5Z'),
            variant(release, UUIDS[2], [], 'UDP')]


def log_bodies(release):
    return [log_variant(release, 'INFO'),
            log_variant(release, 'WARNING', log_timestamp='2026-06-30T23:59:59.5Z'),
            log_variant(release, 'FATAL')]


def expected_families(release):
    schema = client.SCHEMA_VERSIONS[release]
    rdma = 'RECEIVED RDMA_InitializeSetup #N type=RDMA_InitializeSetupMT setup-id={} provider={} '
    log = ('RECEIVED Log #N type=LogMT severity={} log-timestamp={} mode=LIVE schema=' + schema +
           ' timestamp=2026-01-02T03:04:05Z classification=U service-id=- subsystem-id=- '
           'components=0 capabilities=0')
    return dict(
        rdma=[rdma.format(UUIDS[0], 'TCP') + f'mode=LIVE schema={schema} timestamp=2026-01-02T03:04:05Z '
              'classification=U endpoints=1 [IPv4 198.51.100.42/24]',
              rdma.format(UUIDS[1], 'VERBS') + f'mode=LIVE schema={schema} '
              'timestamp=2026-06-30T23:59:59.5Z classification=U endpoints=2 '
              '[IPv6 fE80::aBcD, IPv4 198.51.100.42/24]',
              rdma.format(UUIDS[2], 'UDP') + f'mode=LIVE schema={schema} timestamp=2026-01-02T03:04:05Z '
              'classification=U endpoints=0 []'],
        log=[log.format('INFO', '2026-01-02T03:04:05Z'),
             log.format('WARNING', '2026-06-30T23:59:59.5Z'),
             log.format('FATAL', '2026-01-02T03:04:05Z')])


async def witness_both(observer):
    for frame in (f'SUB witness-rdma RDMA_InitializeSetup {TOPIC}', f'SUB witness-log Log {LOG_TOPIC}'):
        if await observer.status(frame) != ('+OK', []):
            raise AssertionError('independent observer could not subscribe')


async def exercise(test, release, url, publisher, observer, extra=(), env=None):
    """Drive one --with-log CLI session; return (code, lines, stderr)."""
    rdma, logs = rdma_bodies(release), log_bodies(release)
    # Pinned Sleet does not check Gregorian day ranges; the generated LogMT
    # decoder's checked dateTime carrier does. Broker accepts, decoder rejects.
    decoder_invalid = log_variant(release, timestamp='2026-02-30T03:04:05Z')
    broker_invalid = log_variant(release, 'LOUD')  # not a LogSeverityEnum value
    witnessed = {}

    async def drive(line, process):
        if not line.startswith('READY '):
            return
        # Independent publication only after BOTH SUB acknowledgements.
        ok = ('+OK', [])
        test.assertEqual(await publisher.publish(rdma[0]), ok)
        test.assertEqual(await publisher.publish_log(logs[0]), ok)
        test.assertEqual(await publisher.publish(rdma[1]), ok)
        witnessed['broker'] = (await publisher.publish_log(broker_invalid))[0]
        witnessed['unauthorized'] = (await publisher.publish(rdma[0], topic=LOG_TOPIC))[0]
        test.assertEqual(await publisher.publish_log(decoder_invalid), ok)
        # Valid RDMA (and Log) reception continues after a Log decoder failure.
        test.assertEqual(await publisher.publish(rdma[2]), ok)
        test.assertEqual(await publisher.publish_log(logs[1]), ok)
        test.assertEqual(await publisher.publish_log(logs[2]), ok)
        witnessed['msgs'] = [await observer.recv() for _ in range(7)]

    code, lines, stderr = await run_cli(cli_arguments(url, release, duration_seconds='5', **dict(extra)),
                                        until=drive, env=env)
    test.assertRegex(witnessed['broker'], r'^-ERR Invalid-Message ')
    test.assertRegex(witnessed['unauthorized'], r'^-ERR Illegal-State ')
    # Both families witnessed on the wire, interleaved, in publication order;
    # rejected broker traffic never reached any subscriber.
    test.assertEqual([(m.split(' ', 2)[1], json.loads(m.split(' ', 2)[2])) for m in witnessed['msgs']], [
        ('witness-rdma', {'RDMA_InitializeSetup': rdma[0]}), ('witness-log', {'Log': logs[0]}),
        ('witness-rdma', {'RDMA_InitializeSetup': rdma[1]}), ('witness-log', {'Log': decoder_invalid}),
        ('witness-rdma', {'RDMA_InitializeSetup': rdma[2]}), ('witness-log', {'Log': logs[1]}),
        ('witness-log', {'Log': logs[2]})])
    return code, lines, stderr


def assert_session(test, release, code, lines, stderr):
    """Exact typed CLI output; returns (rdma_sid, log_sid)."""
    output = '\n'.join(lines)
    test.assertEqual(code, client.EXIT_DEGRADED, output + stderr)  # exactly one decoder rejection
    rdma_sid = re.search(r'^SUBSCRIBED RDMA_InitializeSetup topic=uci\.rdma sid=(\S+) '
                         r'payload=RDMA_InitializeSetupMT$', output, re.M)
    log_sid = re.search(r'^SUBSCRIBED Log topic=uci\.log sid=(\S+) payload=LogMT$', output, re.M)
    test.assertIsNotNone(rdma_sid, output)
    test.assertIsNotNone(log_sid, output)
    rdma_sid, log_sid = rdma_sid[1], log_sid[1]
    test.assertNotEqual(rdma_sid, log_sid)
    test.assertEqual([line.split()[0] for line in lines[:4]],
                     ['CONNECTED', 'SUBSCRIBED', 'SUBSCRIBED', 'READY'])
    test.assertEqual(lines[3], 'READY subscriptions=2 messages=RDMA_InitializeSetup,Log duration=5s')
    received = [line for line in lines if line.startswith('RECEIVED ')]
    # Global #N indexes are unique and complete: no duplicate or lost callback.
    test.assertEqual(sorted(int(re.search(r' #(\d+) ', line)[1]) for line in received), list(range(1, 7)))
    normalized = [re.sub(r' #\d+ ', ' #N ', line, count=1) for line in received]
    expected = expected_families(release)
    # Ordered delivery within each subscription; correct generated typed values.
    test.assertEqual([line for line in normalized if line.startswith('RECEIVED RDMA_')], expected['rdma'])
    test.assertEqual([line for line in normalized if line.startswith('RECEIVED Log ')], expected['log'])
    test.assertEqual(stderr.count('REJECTED Log payload (CodecFailure: payload decoder rejected message)'),
                     1, stderr)
    test.assertNotIn('REJECTED RDMA_InitializeSetup', stderr)
    for private in ('2026-02-30', 'LOUD', 'PY016', 'Independent full-schema golden', '00FF', 'NATO:EXAMPLE'):
        test.assertNotIn(private, output + stderr)
    test.assertEqual(lines[-3:], [f'UNSUBSCRIBED sid={log_sid} message=Log',
                                  f'UNSUBSCRIBED sid={rdma_sid} message=RDMA_InitializeSetup',
                                  'CLOSED received=6 rejected=1 callback-failures=0 '
                                  'RDMA_InitializeSetup=3/0 Log=3/1'])
    return rdma_sid, log_sid


def assert_broker_ownership(test, events, rdma_sid, log_sid, peers):
    """Broker-side evidence: one CLI connection, one INIT, two SUBs, two UNSUBs."""
    test.assertEqual(events.count('initialized schema='), peers + 1)  # peers + exactly one CLI INIT
    owner = re.search(r'connection\{id=(\d+)[^}]*\}: sleet::server: subscription registered '
                      rf'sid={re.escape(rdma_sid)} topic=uci\.rdma message_name=RDMA_InitializeSetup', events)
    test.assertIsNotNone(owner, events)
    prefix = rf'connection\{{id={owner[1]}[^}}]*\}}: sleet::server: '
    test.assertRegex(events, prefix + rf'subscription registered sid={re.escape(log_sid)} '
                     r'topic=uci\.log message_name=Log')
    for sid in (rdma_sid, log_sid):
        test.assertRegex(events, prefix + rf'subscription removed sid={re.escape(sid)}\n')
    # Only the CLI unsubscribed; the witness connection simply closed.
    test.assertEqual(events.count('dispatch UNSUB'), 2)
    test.assertEqual(events.count('subscription registered'), 4)  # 2 witness + 2 CLI


class RealSleetWithLog(unittest.IsolatedAsyncioTestCase):
    async def combined_release(self, release):
        schema = client.SCHEMA_VERSIONS[release]
        async with Broker(release, bindings=BOTH) as broker:
            async with RawPeer(broker.url, schema) as publisher, RawPeer(broker.url, schema) as observer:
                await witness_both(observer)
                code, lines, stderr = await exercise(self, release, broker.url, publisher, observer)
                # After teardown: the observer still receives both families,
                # the CLI (already exited) produced no further output.
                self.assertEqual(await publisher.publish(rdma_bodies(release)[0]), ('+OK', []))
                self.assertEqual(await publisher.publish_log(log_golden(release)), ('+OK', []))
                for _ in range(2):
                    self.assertTrue((await observer.recv()).startswith('MSG witness-'))
                await asyncio.sleep(.2)
                events = broker.events()
        self.assertEqual(broker.process.returncode, 0)  # clean broker termination
        rdma_sid, log_sid = assert_session(self, release, code, lines, stderr)
        self.assertIn(f'CONNECTED ws://127.0.0.1:{broker.port} owp=1.0 service-id=python-uci-client '
                      f'schema={schema}', lines)
        assert_broker_ownership(self, events, rdma_sid, log_sid, peers=2)
        print(f'PY-OMS-021 REAL SLEET UCI {release}: PASSED received=6 rdma=3 log=3 '
              f'decoder-rejected=1 broker-rejected=1 unauthorized-rejected=1 '
              f'sids={rdma_sid},{log_sid}', flush=True)

    async def test_uci25_rdma_and_log_on_one_connection(self):
        await self.combined_release('2.5')

    async def test_uci26_rdma_and_log_on_one_connection(self):
        await self.combined_release('2.6')

    async def test_unauthorized_log_subscription_releases_rdma(self):
        # The broker authorizes only uci.rdma (the PY-OMS-020 configuration).
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                async with Broker(release) as broker:
                    code, lines, stderr = await run_cli(cli_arguments(broker.url, release))
                    await asyncio.sleep(.2)
                    events = broker.events()
                self.assertEqual(code, client.EXIT_REJECTED, stderr)
                self.assertIn('SUBSCRIBE REJECTED by server: Illegal-State message=Log', stderr)
                self.assertNotIn('READY', '\n'.join(lines))
                rdma_sid = re.search(r'^SUBSCRIBED RDMA_InitializeSetup topic=uci\.rdma sid=(\S+)',
                                     '\n'.join(lines), re.M)[1]
                self.assertEqual(lines[-2:], [
                    f'UNSUBSCRIBED sid={rdma_sid} message=RDMA_InitializeSetup',
                    'CLOSED received=0 rejected=0 callback-failures=0 RDMA_InitializeSetup=0/0 Log=0/0'])
                # Independent broker evidence: RDMA registered then removed by
                # an explicit UNSUB; Log never registered. No ghost subscription.
                self.assertIn(f'subscription registered sid={rdma_sid} topic=uci.rdma', events)
                self.assertIn(f'subscription removed sid={rdma_sid}', events)
                self.assertIn('not authorized to subscribe to topic uci.log', events)
                # Only a registration line has "topic=... message_name=..." order.
                self.assertNotIn('topic=uci.log message_name=Log', events)
                self.assertEqual(events.count('subscription registered'), 1)
                self.assertEqual(events.count('dispatch UNSUB'), 1)

    async def test_signal_with_both_subscriptions_releases_both(self):
        async with Broker('2.6', bindings=BOTH) as broker, RawPeer(broker.url, '002.6.0') as publisher:
            async def drive(line, process):
                if line.startswith('READY '):
                    self.assertEqual(await publisher.publish_log(log_golden('2.6')), ('+OK', []))
                elif line.startswith('RECEIVED '):
                    process.send_signal(signal.SIGTERM)

            code, lines, stderr = await run_cli(cli_arguments(broker.url, '2.6', duration_seconds='60'),
                                                until=drive, timeout=30)
            await asyncio.sleep(.2)
            events = broker.events()
        self.assertEqual(code, 143, stderr)
        self.assertTrue(lines[-3].startswith('UNSUBSCRIBED sid=') and lines[-3].endswith(' message=Log'))
        self.assertTrue(lines[-2].endswith(' message=RDMA_InitializeSetup'))
        self.assertEqual(lines[-1], 'CLOSED received=1 rejected=0 callback-failures=0 '
                                    'RDMA_InitializeSetup=0/0 Log=1/0')
        self.assertEqual(events.count('subscription removed sid='), 2)


if __name__ == '__main__':
    unittest.main()
