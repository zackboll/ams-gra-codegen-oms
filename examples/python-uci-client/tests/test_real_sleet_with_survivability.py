"""Real unchanged pinned Sleet: the actual CLI receives RDMA, Log and
SurvivabilityRiskLevel on ONE connection.

PY-OMS-022 (--with-log --with-survivability), separately for UCI 2.5 and 2.6.
The broker authorizes uci.rdma for RDMA_InitializeSetup, uci.log for Log and
uci.survivability for SurvivabilityRiskLevel. An independent raw-websocket
publisher/observer (support.RawPeer, never oms_lacal or the app) publishes
Task092 RDMA, PY-016 Log and PY-016 SurvivabilityRiskLevel goldens (targeted
edits only), interleaved, and witnesses each MSG on all three topics. Broker
rejection (publisher -ERR, no MSG at the observer) is kept distinct from
generated decoder rejection (publisher +OK, MSG witnessed, CLI CodecFailure).
"""
import asyncio
import json
import re
import signal
import unittest

from support import (LOG_TOPIC, SURVIVABILITY_TOPIC, TOPIC, Broker, RawPeer, bindings_directory,
                     cleanup_bindings, client, log_golden, run_cli, survivability_golden,
                     survivability_variant)
from test_real_sleet import UUIDS
import test_real_sleet_with_log as with_log

ALL = {TOPIC: ('RDMA_InitializeSetup',), LOG_TOPIC: ('Log',),
       SURVIVABILITY_TOPIC: ('SurvivabilityRiskLevel',)}
NAMES = ('RDMA_InitializeSetup', 'Log', 'SurvivabilityRiskLevel')
WITNESS = ('witness-rdma', 'witness-log', 'witness-survivability')


def tearDownModule():
    cleanup_bindings()


def cli_arguments(url, release, **extra):
    arguments = ['--url', url, '--service-id', 'python-uci-client', '--schema-release', release,
                 '--bindings', bindings_directory(release, with_log=True, with_survivability=True),
                 '--with-log', '--with-survivability']
    for key, value in extra.items():
        arguments += ['--' + key.replace('_', '-'), value]
    return arguments


def survivability_bodies(release):
    return [survivability_variant(release, UUIDS[2], [('HIGH', ['NONE'])]),
            survivability_variant(release, UUIDS[0], [('LOW', ['TRACK', 'KILL']), ('MEDIUM', ['ALL'])]),
            survivability_variant(release, UUIDS[1], [('NONE', ['ACQUISITION'])],
                                  timestamp='2026-06-30T23:59:59.5Z')]


def expected_survivability(release):
    schema = client.SCHEMA_VERSIONS[release]
    line = ('RECEIVED SurvivabilityRiskLevel #N type=SurvivabilityRiskLevelMT risk-level-id={} '
            'mode=LIVE schema=' + schema + ' timestamp={} classification=U risk-settings={}')
    return [line.format(UUIDS[2], '2026-01-02T03:04:05Z', '1 [HIGH:NONE]'),
            line.format(UUIDS[0], '2026-01-02T03:04:05Z', '2 [LOW:TRACK+KILL, MEDIUM:ALL]'),
            line.format(UUIDS[1], '2026-06-30T23:59:59.5Z', '1 [NONE:ACQUISITION]')]


async def witness_all(observer):
    for sid, (topic, (message,)) in zip(WITNESS, ALL.items()):
        if await observer.status(f'SUB {sid} {message} {topic}') != ('+OK', []):
            raise AssertionError('independent observer could not subscribe')


async def exercise(test, release, url, publisher, observer, extra=(), env=None):
    """Drive one three-message CLI session; return (code, lines, stderr)."""
    rdma, logs = with_log.rdma_bodies(release), with_log.log_bodies(release)
    survs = survivability_bodies(release)
    # Pinned Sleet does not check Gregorian day ranges; the generated
    # SurvivabilityRiskLevelMT decoder's checked dateTime carrier does.
    decoder_invalid = survivability_variant(release, timestamp='2026-02-30T03:04:05Z')
    # Not a RiskPrioritizationEnum value: the broker's schema validation rejects it.
    broker_invalid = survivability_variant(release, settings=[('URGENT', ['NONE'])])
    witnessed, delivered = {}, asyncio.Queue()

    async def publish_all():
        # Independent publication only after ALL THREE SUB acknowledgements.
        # Each deliverable publication waits for the CLI's typed callback line
        # of the previous one, so the CLI's arrival order is the publication
        # order: interleaved by construction, never by a cross-subscription
        # ordering assumption.
        ok = ('+OK', [])

        async def deliver(reply):
            test.assertEqual(await reply, ok)
            witnessed.setdefault('cli', []).append(await asyncio.wait_for(delivered.get(), 10))

        await deliver(publisher.publish(rdma[0]))
        await deliver(publisher.publish_survivability(survs[0]))
        await deliver(publisher.publish_log(logs[0]))
        await deliver(publisher.publish_survivability(survs[1]))
        await deliver(publisher.publish(rdma[1]))
        witnessed['broker'] = (await publisher.publish_survivability(broker_invalid))[0]
        witnessed['unauthorized'] = (await publisher.publish(rdma[0], topic=SURVIVABILITY_TOPIC))[0]
        test.assertEqual(await publisher.publish_survivability(decoder_invalid), ok)
        # All three families continue after a Survivability decoder failure.
        await deliver(publisher.publish(rdma[2]))
        await deliver(publisher.publish_log(logs[1]))
        await deliver(publisher.publish_survivability(survs[2]))
        await deliver(publisher.publish_log(logs[2]))
        witnessed['msgs'] = [await observer.recv() for _ in range(10)]

    tasks = []

    async def drive(line, process):
        if line.startswith('READY '):
            tasks.append(asyncio.create_task(publish_all()))
        elif line.startswith('RECEIVED '):
            delivered.put_nowait(line.split()[1])

    code, lines, stderr = await run_cli(cli_arguments(url, release, duration_seconds='5', **dict(extra)),
                                        until=drive, env=env)
    test.assertEqual(len(tasks), 1, lines)
    await tasks[0]  # every deliverable publication reached the CLI before it exited
    # Interleaved arrival at the generated typed callback, in publication order.
    test.assertEqual(witnessed['cli'], ['RDMA_InitializeSetup', 'SurvivabilityRiskLevel', 'Log',
                                        'SurvivabilityRiskLevel', 'RDMA_InitializeSetup',
                                        'RDMA_InitializeSetup', 'Log', 'SurvivabilityRiskLevel', 'Log'])
    test.assertRegex(witnessed['broker'], r'^-ERR Invalid-Message ')
    test.assertRegex(witnessed['unauthorized'], r'^-ERR Illegal-State ')
    # All three global messages witnessed on the wire, interleaved, in
    # publication order; rejected broker traffic never reached any subscriber.
    test.assertEqual([(m.split(' ', 2)[1], json.loads(m.split(' ', 2)[2])) for m in witnessed['msgs']], [
        ('witness-rdma', {'RDMA_InitializeSetup': rdma[0]}),
        ('witness-survivability', {'SurvivabilityRiskLevel': survs[0]}),
        ('witness-log', {'Log': logs[0]}),
        ('witness-survivability', {'SurvivabilityRiskLevel': survs[1]}),
        ('witness-rdma', {'RDMA_InitializeSetup': rdma[1]}),
        ('witness-survivability', {'SurvivabilityRiskLevel': decoder_invalid}),
        ('witness-rdma', {'RDMA_InitializeSetup': rdma[2]}),
        ('witness-log', {'Log': logs[1]}),
        ('witness-survivability', {'SurvivabilityRiskLevel': survs[2]}),
        ('witness-log', {'Log': logs[2]})])
    return code, lines, stderr


def assert_session(test, release, code, lines, stderr):
    """Exact typed CLI output; returns the three SIDs in SUB order."""
    output = '\n'.join(lines)
    test.assertEqual(code, client.EXIT_DEGRADED, output + stderr)  # exactly one decoder rejection
    sids = []
    for message, topic, payload in (('RDMA_InitializeSetup', r'uci\.rdma', 'RDMA_InitializeSetupMT'),
                                    ('Log', r'uci\.log', 'LogMT'),
                                    ('SurvivabilityRiskLevel', r'uci\.survivability',
                                     'SurvivabilityRiskLevelMT')):
        match = re.search(rf'^SUBSCRIBED {message} topic={topic} sid=(\S+) payload={payload}$',
                          output, re.M)
        test.assertIsNotNone(match, output)
        sids.append(match[1])
    test.assertEqual(len(set(sids)), 3)  # three distinct subscription SIDs
    test.assertEqual([line.split()[0] for line in lines[:5]],
                     ['CONNECTED', 'SUBSCRIBED', 'SUBSCRIBED', 'SUBSCRIBED', 'READY'])
    test.assertEqual(lines[4], 'READY subscriptions=3 messages=RDMA_InitializeSetup,Log,'
                               'SurvivabilityRiskLevel duration=5s')
    received = [line for line in lines if line.startswith('RECEIVED ')]
    # Global #N indexes are unique and complete: no duplicate or lost callback.
    test.assertEqual(sorted(int(re.search(r' #(\d+) ', line)[1]) for line in received), list(range(1, 10)))
    normalized = [re.sub(r' #\d+ ', ' #N ', line, count=1) for line in received]
    expected = with_log.expected_families(release)
    # Ordered delivery within each subscription; correct generated typed values.
    # RDMA and Log lines are byte-identical to the PY-OMS-021 expectations.
    test.assertEqual([line for line in normalized if line.startswith('RECEIVED RDMA_')], expected['rdma'])
    test.assertEqual([line for line in normalized if line.startswith('RECEIVED Log ')], expected['log'])
    test.assertEqual([line for line in normalized if line.startswith('RECEIVED Surv')],
                     expected_survivability(release))
    test.assertEqual(stderr.count('REJECTED SurvivabilityRiskLevel payload (CodecFailure: payload '
                                  'decoder rejected message)'), 1, stderr)
    test.assertNotIn('REJECTED RDMA_InitializeSetup', stderr)
    test.assertNotIn('REJECTED Log', stderr)
    for private in ('2026-02-30', 'URGENT', 'LOUD', 'PY016', 'Independent full-schema golden', '00FF',
                    'NATO:EXAMPLE'):
        test.assertNotIn(private, output + stderr)
    test.assertEqual(lines[-4:], [f'UNSUBSCRIBED sid={sids[2]} message=SurvivabilityRiskLevel',
                                  f'UNSUBSCRIBED sid={sids[1]} message=Log',
                                  f'UNSUBSCRIBED sid={sids[0]} message=RDMA_InitializeSetup',
                                  'CLOSED received=9 rejected=1 callback-failures=0 '
                                  'RDMA_InitializeSetup=3/0 Log=3/0 SurvivabilityRiskLevel=3/1'])
    return sids


def assert_broker_ownership(test, events, sids, peers):
    """Broker-side evidence: one CLI connection, one INIT, three SUBs, three UNSUBs."""
    test.assertEqual(events.count('initialized schema='), peers + 1)  # peers + exactly one CLI INIT
    owner = re.search(r'connection\{id=(\d+)[^}]*\}: sleet::server: subscription registered '
                      rf'sid={re.escape(sids[0])} topic=uci\.rdma message_name=RDMA_InitializeSetup', events)
    test.assertIsNotNone(owner, events)
    prefix = rf'connection\{{id={owner[1]}[^}}]*\}}: sleet::server: '
    test.assertRegex(events, prefix + rf'subscription registered sid={re.escape(sids[1])} '
                     r'topic=uci\.log message_name=Log')
    test.assertRegex(events, prefix + rf'subscription registered sid={re.escape(sids[2])} '
                     r'topic=uci\.survivability message_name=SurvivabilityRiskLevel')
    # The payload class name never appeared as an OWP global message.
    test.assertNotIn('message_name=SurvivabilityRiskLevelMT', events)
    for sid in sids:
        test.assertRegex(events, prefix + rf'subscription removed sid={re.escape(sid)}\n')
    # Only the CLI unsubscribed; the witness connection simply closed.
    test.assertEqual(events.count('dispatch UNSUB'), 3)
    test.assertEqual(events.count('subscription registered'), 6)  # 3 witness + 3 CLI


class RealSleetWithSurvivability(unittest.IsolatedAsyncioTestCase):
    async def three_message_release(self, release):
        schema = client.SCHEMA_VERSIONS[release]
        async with Broker(release, bindings=ALL) as broker:
            async with RawPeer(broker.url, schema) as publisher, RawPeer(broker.url, schema) as observer:
                await witness_all(observer)
                code, lines, stderr = await exercise(self, release, broker.url, publisher, observer)
                # After teardown: the observer still receives all three families;
                # the CLI (already exited) produced no further output.
                self.assertEqual(await publisher.publish(with_log.rdma_bodies(release)[0]), ('+OK', []))
                self.assertEqual(await publisher.publish_log(log_golden(release)), ('+OK', []))
                self.assertEqual(await publisher.publish_survivability(survivability_golden(release)),
                                 ('+OK', []))
                for _ in range(3):
                    self.assertTrue((await observer.recv()).startswith('MSG witness-'))
                await asyncio.sleep(.2)
                events = broker.events()
        self.assertEqual(broker.process.returncode, 0)  # clean broker termination
        sids = assert_session(self, release, code, lines, stderr)
        self.assertIn(f'CONNECTED ws://127.0.0.1:{broker.port} owp=1.0 service-id=python-uci-client '
                      f'schema={schema}', lines)
        assert_broker_ownership(self, events, sids, peers=2)
        print(f'PY-OMS-022 REAL SLEET UCI {release}: PASSED received=9 rdma=3 log=3 survivability=3 '
              f'decoder-rejected=1 broker-rejected=1 unauthorized-rejected=1 '
              f'sids={",".join(sids)}', flush=True)

    async def test_uci25_three_messages_on_one_connection(self):
        await self.three_message_release('2.5')

    async def test_uci26_three_messages_on_one_connection(self):
        await self.three_message_release('2.6')

    async def test_unauthorized_survivability_subscription_releases_rdma_and_log(self):
        # The broker authorizes only uci.rdma and uci.log (the PY-OMS-021 configuration).
        for release in ('2.5', '2.6'):
            with self.subTest(release=release):
                async with Broker(release, bindings=with_log.BOTH) as broker:
                    code, lines, stderr = await run_cli(cli_arguments(broker.url, release))
                    await asyncio.sleep(.2)
                    events = broker.events()
                self.assertEqual(broker.process.returncode, 0)
                self.assertEqual(code, client.EXIT_REJECTED, stderr)
                self.assertIn('SUBSCRIBE REJECTED by server: Illegal-State message=SurvivabilityRiskLevel',
                              stderr)
                output = '\n'.join(lines)
                self.assertNotIn('READY', output)
                rdma_sid = re.search(r'^SUBSCRIBED RDMA_InitializeSetup topic=uci\.rdma sid=(\S+)',
                                     output, re.M)[1]
                log_sid = re.search(r'^SUBSCRIBED Log topic=uci\.log sid=(\S+)', output, re.M)[1]
                self.assertEqual(lines[-3:], [
                    f'UNSUBSCRIBED sid={log_sid} message=Log',
                    f'UNSUBSCRIBED sid={rdma_sid} message=RDMA_InitializeSetup',
                    'CLOSED received=0 rejected=0 callback-failures=0 '
                    'RDMA_InitializeSetup=0/0 Log=0/0 SurvivabilityRiskLevel=0/0'])
                # Independent broker evidence: RDMA and Log registered then removed
                # by explicit UNSUBs; Survivability never registered. No ghost SID.
                for sid, topic in ((rdma_sid, 'uci.rdma'), (log_sid, 'uci.log')):
                    self.assertIn(f'subscription registered sid={sid} topic={topic}', events)
                    self.assertIn(f'subscription removed sid={sid}', events)
                self.assertIn('not authorized to subscribe to topic uci.survivability', events)
                self.assertNotIn('topic=uci.survivability message_name=SurvivabilityRiskLevel', events)
                self.assertEqual(events.count('subscription registered'), 2)
                self.assertEqual(events.count('dispatch UNSUB'), 2)

    async def test_signal_with_three_subscriptions_releases_all(self):
        async with Broker('2.6', bindings=ALL) as broker, RawPeer(broker.url, '002.6.0') as publisher:
            async def drive(line, process):
                if line.startswith('READY '):
                    self.assertEqual(await publisher.publish_survivability(survivability_golden('2.6')),
                                     ('+OK', []))
                elif line.startswith('RECEIVED '):
                    process.send_signal(signal.SIGTERM)

            code, lines, stderr = await run_cli(cli_arguments(broker.url, '2.6', duration_seconds='60'),
                                                until=drive, timeout=30)
            await asyncio.sleep(.2)
            events = broker.events()
        self.assertEqual(code, 143, stderr)
        self.assertEqual([line.split()[-1] for line in lines[-4:-1]],
                         ['message=SurvivabilityRiskLevel', 'message=Log', 'message=RDMA_InitializeSetup'])
        self.assertTrue(all(line.startswith('UNSUBSCRIBED sid=') for line in lines[-4:-1]))
        self.assertEqual(lines[-1], 'CLOSED received=1 rejected=0 callback-failures=0 '
                                    'RDMA_InitializeSetup=0/0 Log=0/0 SurvivabilityRiskLevel=1/0')
        self.assertEqual(events.count('subscription removed sid='), 3)


if __name__ == '__main__':
    unittest.main()
