"""Real unchanged pinned Sleet: the actual CLI receives authentic UCI 2.5 / 2.6.

Each release is exercised independently with its own pinned-schema broker. An
independent raw-websocket publisher/observer (support.RawPeer) owns publication
and witnesses the broker MSG; the application is the real CLI process.
Required per release: genuine INIT/INFO and SUB acknowledgement (CLI output
and broker debug log), authorized independent PUB, actual MSG delivery,
generated typed decoding with correct selected fields, broker rejection of
invalid/unauthorized publication, generated-decoder rejection of a payload the
broker accepts, UNSUB, clean exit and no callback after teardown.
"""
import asyncio
import json
import re
import signal
import unittest

from support import (TOPIC, Broker, RawPeer, bindings_directory, cleanup_bindings, client,
                     golden, run_cli, variant)

V4 = {'IPv4_Endpoint': {'IPv4_Address': '198.51.100.42', 'Prefix': 24}}
V6 = {'IPv6_Endpoint': {'IPv6_Address': 'fE80::aBcD'}}
UUIDS = ['5b0f6d4e-1c2a-4b7e-9a3d-0123456789ab', '00000000-0000-0000-0000-000000000000',
         'aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee']


def tearDownModule():
    cleanup_bindings()


def cli_arguments(url, release, **extra):
    arguments = ['--url', url, '--service-id', 'python-uci-client',
                 '--schema-release', release, '--bindings', bindings_directory(release)]
    for key, value in extra.items():
        arguments += ['--' + key.replace('_', '-'), value]
    return arguments


def expected_lines(release):
    schema = client.SCHEMA_VERSIONS[release]
    prefix = 'RECEIVED RDMA_InitializeSetup #{} type=RDMA_InitializeSetupMT setup-id={} provider={} '
    return [
        prefix.format(1, UUIDS[0], 'TCP') + f'mode=LIVE schema={schema} timestamp=2026-01-02T03:04:05Z '
        'classification=U endpoints=1 [IPv4 198.51.100.42/24]',
        prefix.format(2, UUIDS[1], 'VERBS') + f'mode=LIVE schema={schema} timestamp=2026-06-30T23:59:59.5Z '
        'classification=U endpoints=2 [IPv6 fE80::aBcD, IPv4 198.51.100.42/24]',
        prefix.format(3, UUIDS[2], 'UDP') + f'mode=LIVE schema={schema} timestamp=2026-01-02T03:04:05Z '
        'classification=U endpoints=0 []']


def bodies(release):
    return [variant(release, UUIDS[0], [V4], 'TCP'),
            variant(release, UUIDS[1], [V6, V4], 'VERBS', '2026-06-30T23:59:59.5Z'),
            variant(release, UUIDS[2], [], 'UDP')]


async def exercise(test, release, url, publisher, observer, extra=(), env=None):
    """Drive one CLI session; return (code, lines, stderr, witnessed)."""
    valid = bodies(release)
    # Pinned Sleet does not check Gregorian day ranges; the generated
    # decoder's checked dateTime carrier does. Broker accepts, decoder rejects.
    decoder_invalid = variant(release, UUIDS[0], timestamp='2026-02-30T03:04:05Z')
    broker_invalid = variant(release, endpoints=[{'IPv4_Endpoint': {'IPv4_Address': '01.2.3.4'}}])
    witnessed = {}

    async def drive(line, process):
        if not line.startswith('SUBSCRIBED '):
            return
        # Independent authorized publication only after the CLI's SUB +OK.
        for body in valid[:2]:
            test.assertEqual(await publisher.publish(body), ('+OK', []))
        witnessed['broker'] = (await publisher.publish(broker_invalid))[0]
        witnessed['unauthorized'] = (await publisher.publish(golden(release), name='Log'))[0]
        test.assertEqual(await publisher.publish(decoder_invalid), ('+OK', []))
        test.assertEqual(await publisher.publish(valid[2]), ('+OK', []))
        witnessed['msgs'] = [await observer.recv() for _ in range(4)]

    code, lines, stderr = await run_cli(cli_arguments(url, release, duration_seconds='4', **dict(extra)),
                                        until=drive, env=env)
    test.assertRegex(witnessed['broker'], r'^-ERR Invalid-Message ')
    test.assertRegex(witnessed['unauthorized'], r'^-ERR Illegal-State ')
    test.assertEqual([json.loads(m.split(' ', 2)[2]) for m in witnessed['msgs']],
                     [{'RDMA_InitializeSetup': b} for b in (*valid[:2], decoder_invalid, valid[2])])
    test.assertTrue(all(m.startswith('MSG witness ') for m in witnessed['msgs']))
    return code, lines, stderr


def assert_session(test, release, code, lines, stderr):
    output = '\n'.join(lines)
    test.assertEqual(code, client.EXIT_DEGRADED, output + stderr)  # exactly one decoder rejection
    test.assertEqual([line for line in lines if line.startswith('RECEIVED ')], expected_lines(release))
    test.assertEqual(stderr.count('REJECTED RDMA_InitializeSetup payload '
                                  '(CodecFailure: payload decoder rejected message)'), 1, stderr)
    test.assertNotIn('2026-02-30', output + stderr)
    sid = re.search(r'^SUBSCRIBED RDMA_InitializeSetup topic=uci\.rdma sid=(\S+) '
                    r'payload=RDMA_InitializeSetupMT duration=4s$', output, re.M)
    test.assertIsNotNone(sid, output)
    test.assertEqual(lines[-2:], [f'UNSUBSCRIBED sid={sid[1]}',
                                  'CLOSED received=3 rejected=1 callback-failures=0'])
    return sid[1]


class RealSleet(unittest.IsolatedAsyncioTestCase):
    async def authentic_release(self, release):
        schema = client.SCHEMA_VERSIONS[release]
        async with Broker(release) as broker, RawPeer(broker.url, schema) as publisher, \
                RawPeer(broker.url, schema) as observer:
            self.assertEqual(await observer.status(f'SUB witness RDMA_InitializeSetup {TOPIC}'), ('+OK', []))
            code, lines, stderr = await exercise(self, release, broker.url, publisher, observer)
            # After teardown: the observer still receives, the CLI is gone.
            self.assertEqual(await publisher.publish(bodies(release)[0]), ('+OK', []))
            self.assertTrue((await observer.recv()).startswith('MSG witness '))
            await asyncio.sleep(.2)
            events = broker.events()
        sid = assert_session(self, release, code, lines, stderr)
        self.assertIn(f'CONNECTED ws://127.0.0.1:{broker.port} owp=1.0 service-id=python-uci-client '
                      f'schema={schema}', lines)
        # Broker-side evidence, independent of the CLI's own reporting.
        self.assertIn(f'initialized schema={schema} service_id=python-uci-client', events)
        self.assertIn(f'subscription registered sid={sid} topic={TOPIC} '
                      'message_name=RDMA_InitializeSetup', events)
        self.assertIn(f'subscription removed sid={sid}', events)
        # Only the CLI unsubscribed; the witness connection simply closed.
        self.assertEqual(events.count('dispatch UNSUB'), 1)
        print(f'PY-OMS-020 REAL SLEET UCI {release}: PASSED received=3 decoder-rejected=1 '
              f'broker-rejected=1 unauthorized-rejected=1 sid={sid}', flush=True)

    async def test_uci25_authentic_rdma_reception(self):
        await self.authentic_release('2.5')

    async def test_uci26_authentic_rdma_reception(self):
        await self.authentic_release('2.6')

    async def test_broker_rejects_wrong_release_service_and_message(self):
        async with Broker('2.6') as broker:
            # 2.5 bindings fix INIT schema 002.5.0; a 2.6 broker refuses it.
            code, _, stderr = await run_cli(cli_arguments(broker.url, '2.5'))
            self.assertEqual(code, client.EXIT_REJECTED, stderr)
            self.assertIn('CONNECT REJECTED by server: Unsupported-Schema', stderr)
            arguments = cli_arguments(broker.url, '2.6')
            arguments[arguments.index('--service-id') + 1] = 'not-configured'
            code, _, stderr = await run_cli(arguments)
            self.assertEqual(code, client.EXIT_REJECTED, stderr)
            self.assertIn('CONNECT REJECTED by server: Unsupported-Service', stderr)
        async with Broker('2.5', allowed_messages=('Log',)) as broker:
            code, lines, stderr = await run_cli(cli_arguments(broker.url, '2.5'))
            self.assertEqual(code, client.EXIT_REJECTED, stderr)
            self.assertIn('SUBSCRIBE REJECTED by server: Illegal-State', stderr)
            self.assertEqual(lines[-1], 'CLOSED received=0 rejected=0 callback-failures=0')

    async def test_signals_unsubscribe_close_and_report(self):
        for signum, expected in ((signal.SIGTERM, 143), (signal.SIGINT, 130)):
            with self.subTest(signal=signum.name):
                async with Broker('2.5') as broker, RawPeer(broker.url, '002.5.0') as publisher:
                    async def drive(line, process):
                        if line.startswith('SUBSCRIBED '):
                            self.assertEqual(await publisher.publish(golden('2.5')), ('+OK', []))
                        elif line.startswith('RECEIVED '):
                            process.send_signal(signum)

                    code, lines, stderr = await run_cli(
                        cli_arguments(broker.url, '2.5', duration_seconds='60'), until=drive, timeout=30)
                    events = broker.events()
                self.assertEqual(code, expected, stderr)
                self.assertTrue(lines[-2].startswith('UNSUBSCRIBED sid='), lines)
                self.assertEqual(lines[-1], 'CLOSED received=1 rejected=0 callback-failures=0')
                self.assertIn('subscription removed sid=', events)


if __name__ == '__main__':
    unittest.main()
