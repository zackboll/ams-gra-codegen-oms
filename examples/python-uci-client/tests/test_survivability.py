"""PY-OMS-022 --with-log --with-survivability lifecycle: one connection, three subscriptions.

A local OWP peer (same shape as test_multimessage.Peer) plays the BROKER role
only; the client side is the unchanged runtime plus the actual generated
three-message UCI 2.5 package. The peer can reject, abort on or withhold the
acknowledgement of a chosen SUB so partial setup is deterministic.
"""
import asyncio
import io
import json
import unittest

from websockets.asyncio.server import serve
from websockets.exceptions import ConnectionClosed

from support import (bindings_directory, cleanup_bindings, client, golden, log_golden,
                     log_variant, survivability_golden, survivability_variant, variant)

INFO = {'version': '1.0', 'server_id': 'mock', 'system_label': 'Mock',
        'uuids': {'system': '550e8400-e29b-41d4-a716-446655440000',
                  'service': '550e8400-e29b-41d4-a716-446655440001'}}
RDMA, LOG, SURV = 'RDMA_InitializeSetup', 'Log', 'SurvivabilityRiskLevel'
NAMES = (RDMA, LOG, SURV)
BINDINGS = None
UUIDS = ['5b0f6d4e-1c2a-4b7e-9a3d-0123456789ab', '00000000-0000-0000-0000-000000000000',
         'aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee']


def setUpModule():
    global BINDINGS
    BINDINGS = client.load_bindings(bindings_directory('2.5', with_log=True, with_survivability=True),
                                    '2.5', with_log=True, with_survivability=True)


def tearDownModule():
    cleanup_bindings()


def wire(name, body):
    return json.dumps({name: body})


class Peer:
    """Local OWP broker. `sub_error`/`abort_on`/`hold`/`double_ack` name a message."""

    def __init__(self, *, sub_error=None, abort_on=None, hold=None, double_ack=None):
        self.sub_error, self.abort_on, self.hold, self.double_ack = sub_error, abort_on, hold, double_ack
        self.frames, self.sids, self.connections = [], {}, 0
        self.subscribed = {name: asyncio.Event() for name in NAMES}
        self.sub_seen = {name: asyncio.Event() for name in NAMES}
        self.unsubscribed = []
        self.closed = asyncio.Event()
        self.socket = None

    async def __aenter__(self):
        self.server = await serve(self.handle, '127.0.0.1', 0, subprotocols=['owp'])
        self.url = 'ws://127.0.0.1:' + str(self.server.sockets[0].getsockname()[1])
        return self

    async def __aexit__(self, *exc):
        self.server.close()
        await self.server.wait_closed()

    async def handle(self, ws):
        self.connections += 1
        self.socket = ws
        try:
            async for frame in ws:
                self.frames.append(frame)
                if frame.startswith('INIT '):
                    await ws.send('+OK')
                    await ws.send('INFO ' + json.dumps(INFO))
                elif frame.startswith('SUB '):
                    _, sid, message, _topic = frame.split()
                    self.sub_seen[message].set()
                    if message == self.abort_on:
                        ws.transport.abort()
                        return
                    if message == self.hold:
                        continue  # never acknowledged
                    if message == self.sub_error:
                        await ws.send('-ERR Illegal-State private-detail')
                        continue
                    self.sids[message] = sid
                    await ws.send('+OK')
                    if message == self.double_ack:
                        await ws.send('+OK')  # unsolicited: a protocol failure
                    self.subscribed[message].set()
                elif frame.startswith('UNSUB '):
                    self.unsubscribed.append(frame.split()[1])
                    await ws.send('+OK')
        except ConnectionClosed:
            pass
        finally:
            self.closed.set()

    async def deliver(self, message, text):
        await self.socket.send(f'MSG {self.sids[message]} {text}')


def options(url, **extra):
    values = dict(url=url, service_id='python-uci-client', schema_release='2.5',
                  bindings=bindings_directory('2.5', with_log=True, with_survivability=True),
                  duration=30, with_log=True, with_survivability=True)
    values.update(extra)
    return client.Options(**values)


async def until(predicate, timeout=5):
    async with asyncio.timeout(timeout):
        while not predicate():
            await asyncio.sleep(.01)


class Survivability(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.initial_tasks = set(asyncio.all_tasks())

    async def assert_no_leaked_tasks(self):
        await asyncio.sleep(0)
        current = asyncio.current_task()
        self.assertEqual([t for t in asyncio.all_tasks() - self.initial_tasks
                          if t is not current and not t.done()], [])

    def start(self, peer, callback=None, buffer_size=4, **extra):
        self.out, self.err, self.received, self.stop = io.StringIO(), io.StringIO(), [], asyncio.Event()
        return asyncio.create_task(client.run_client(
            options(peer.url, **extra), bindings=BINDINGS,
            callback=callback or self.received.append, stop=self.stop,
            stdout=self.out, stderr=self.err, buffer_size=buffer_size))

    async def all_subscribed(self, peer):
        for event in peer.subscribed.values():
            await asyncio.wait_for(event.wait(), 5)
        await until(lambda: 'READY ' in self.out.getvalue())

    def sub_frames(self, peer):
        return [frame for frame in peer.frames if frame.startswith('SUB ')]

    async def test_interleaved_typed_delivery_of_three_families_on_one_connection(self):
        model = BINDINGS.model
        rdma_bodies = [variant('2.5', UUIDS[0]), variant('2.5', UUIDS[1], provider='VERBS'),
                       variant('2.5', UUIDS[2], provider='UDP')]
        log_bodies = [log_variant('2.5', 'INFO'), log_variant('2.5', 'WARNING'),
                      log_variant('2.5', 'FATAL')]
        surv_bodies = [survivability_variant('2.5', UUIDS[2], [('HIGH', ['NONE'])]),
                       survivability_variant('2.5', UUIDS[0], [('LOW', ['TRACK', 'KILL'])]),
                       survivability_variant('2.5', UUIDS[1], [('MEDIUM', ['ALL']), ('NONE', ['NONE'])])]
        async with Peer() as peer:
            task = self.start(peer)
            await self.all_subscribed(peer)
            sids = [peer.sids[name] for name in NAMES]
            self.assertEqual(len(set(sids)), 3)
            self.assertEqual(self.sub_frames(peer), [f'SUB {sids[0]} RDMA_InitializeSetup uci.rdma',
                                                     f'SUB {sids[1]} Log uci.log',
                                                     f'SUB {sids[2]} SurvivabilityRiskLevel '
                                                     'uci.survivability'])
            # One INIT, one connection; READY only after all three acknowledgements.
            self.assertEqual(len([f for f in peer.frames if f.startswith('INIT ')]), 1)
            lines = self.out.getvalue().splitlines()
            self.assertEqual([line.split()[0] for line in lines],
                             ['CONNECTED', 'SUBSCRIBED', 'SUBSCRIBED', 'SUBSCRIBED', 'READY'])
            self.assertEqual(lines[3], f'SUBSCRIBED SurvivabilityRiskLevel topic=uci.survivability '
                                       f'sid={sids[2]} payload=SurvivabilityRiskLevelMT')
            self.assertEqual(lines[4], 'READY subscriptions=3 messages=RDMA_InitializeSetup,Log,'
                                       'SurvivabilityRiskLevel duration=30s')
            for rdma_body, log_body, surv_body in zip(rdma_bodies, log_bodies, surv_bodies):
                await peer.deliver(SURV, wire(SURV, surv_body))
                await peer.deliver(LOG, wire(LOG, log_body))
                await peer.deliver(RDMA, wire(RDMA, rdma_body))
            await until(lambda: len(self.received) == 9)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual(result.status, client.EXIT_OK, self.err.getvalue())
        self.assertEqual(peer.connections, 1)
        by_type = {t: [v for v in self.received if type(v) is t]
                   for t in (model.RDMA_InitializeSetupMT, model.LogMT, model.SurvivabilityRiskLevelMT)}
        self.assertEqual(sum(map(len, by_type.values())), 9)  # actual generated types only
        # Ordered within each subscription; no duplicates; correct typed values.
        self.assertEqual([v.messagedata.rdma_initializesetupid.uuid.value
                          for v in by_type[model.RDMA_InitializeSetupMT]], UUIDS)
        self.assertEqual([v.messagedata.severity.value for v in by_type[model.LogMT]],
                         ['INFO', 'WARNING', 'FATAL'])
        survs = by_type[model.SurvivabilityRiskLevelMT]
        self.assertEqual([v.messagedata.survivabilityrisklevelid.uuid.value for v in survs],
                         [UUIDS[2], UUIDS[0], UUIDS[1]])
        self.assertEqual(survs[1].messagedata.risksetting[0].risklevel[1].allowedrisk,
                         model.VulnerabilityTypeEnum.KILL)
        self.assertEqual(survs[0], BINDINGS.codec.decode_SurvivabilityRiskLevelMT(
            json.dumps(surv_bodies[0])))
        self.assertEqual({client.message_name(v) for v in self.received}, set(NAMES))
        self.assertEqual(result.received_by_message, {RDMA: 3, LOG: 3, SURV: 3})
        self.assertEqual(result.subscription_ids, sids)
        # Every owned SID released, in reverse SUB order, before close.
        self.assertEqual(peer.unsubscribed, sids[::-1])
        self.assertEqual(result.unsubscribed_ids, sids[::-1])
        self.assertEqual(self.out.getvalue().splitlines()[-4:], [
            f'UNSUBSCRIBED sid={sids[2]} message=SurvivabilityRiskLevel',
            f'UNSUBSCRIBED sid={sids[1]} message=Log',
            f'UNSUBSCRIBED sid={sids[0]} message=RDMA_InitializeSetup',
            'CLOSED received=9 rejected=0 callback-failures=0 '
            'RDMA_InitializeSetup=3/0 Log=3/0 SurvivabilityRiskLevel=3/0'])
        self.assertTrue(result.unsubscribed and result.closed)
        await self.assert_no_leaked_tasks()

    async def test_default_printing_callback_formats_all_three_families(self):
        out, err, stop = io.StringIO(), io.StringIO(), asyncio.Event()
        async with Peer() as peer:
            task = asyncio.create_task(client.run_client(
                options(peer.url), bindings=BINDINGS, stop=stop, stdout=out, stderr=err))
            for event in peer.subscribed.values():
                await asyncio.wait_for(event.wait(), 5)
            await peer.deliver(RDMA, wire(RDMA, variant('2.5', UUIDS[0])))
            await until(lambda: 'RECEIVED RDMA' in out.getvalue())
            await peer.deliver(LOG, wire(LOG, log_golden('2.5')))
            await until(lambda: 'RECEIVED Log' in out.getvalue())
            labelled = survivability_variant('2.5', settings=[('LOW', ['TRACK', 'KILL']), ('HIGH', ['NONE'])])
            labelled['MessageData']['SurvivabilityRiskLevelID']['DescriptiveLabel'] = 'private label'
            await peer.deliver(SURV, wire(SURV, labelled))
            await until(lambda: 'RECEIVED Surv' in out.getvalue())
            stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual(result.status, client.EXIT_OK, err.getvalue())
        received = [line for line in out.getvalue().splitlines() if line.startswith('RECEIVED ')]
        self.assertTrue(received[0].startswith(f'RECEIVED RDMA_InitializeSetup #1 '
                                               f'type=RDMA_InitializeSetupMT setup-id={UUIDS[0]} '))
        # PY-OMS-021 Log output, byte-for-byte.
        self.assertEqual(received[1], 'RECEIVED Log #2 type=LogMT severity=INFO '
                                      'log-timestamp=2026-01-02T03:04:05Z mode=LIVE schema=002.5.0 '
                                      'timestamp=2026-01-02T03:04:05Z classification=U '
                                      'service-id=- subsystem-id=- components=0 capabilities=0')
        self.assertEqual(received[2], 'RECEIVED SurvivabilityRiskLevel #3 type=SurvivabilityRiskLevelMT '
                                      'risk-level-id=12345678-1234-1234-8234-123456789abc mode=LIVE '
                                      'schema=002.5.0 timestamp=2026-01-02T03:04:05Z classification=U '
                                      'risk-settings=2 [LOW:TRACK+KILL, HIGH:NONE]')
        # Free text and security caveats are never printed.
        for private in ('private label', 'PY016', 'Independent full-schema golden', '00FF',
                        'NATO:EXAMPLE', 'a b'):
            self.assertNotIn(private, out.getvalue() + err.getvalue())
        await self.assert_no_leaked_tasks()

    async def test_no_cross_subscription_decoding_and_decoder_recovery(self):
        bad_date = survivability_variant('2.5', timestamp='2026-02-30T03:04:05Z')
        bad_enum = survivability_variant('2.5', settings=[('URGENT', ['NONE'])])
        async with Peer() as peer:
            task = self.start(peer)
            await self.all_subscribed(peer)
            # Each SID unwraps only its own global element: another family's
            # document on a SID is an envelope rejection that never reaches
            # that other family's generated decoder.
            await peer.deliver(SURV, wire(RDMA, golden('2.5')))
            await peer.deliver(SURV, wire(LOG, log_golden('2.5')))
            await peer.deliver(RDMA, wire(SURV, survivability_golden('2.5')))
            await peer.deliver(LOG, wire(SURV, survivability_golden('2.5')))
            # The payload class name is never accepted as the global element.
            await peer.deliver(SURV, wire('SurvivabilityRiskLevelMT', survivability_golden('2.5')))
            # Generated SurvivabilityRiskLevelMT decoder rejections (envelope correct).
            await peer.deliver(SURV, wire(SURV, bad_date))
            await peer.deliver(SURV, wire(SURV, bad_enum))
            # All three families continue afterwards.
            await peer.deliver(RDMA, wire(RDMA, golden('2.5')))
            await peer.deliver(LOG, wire(LOG, log_golden('2.5')))
            await peer.deliver(SURV, wire(SURV, survivability_golden('2.5')))
            await until(lambda: len(self.received) == 3)
            await until(lambda: self.err.getvalue().count('REJECTED') == 7)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        errors = self.err.getvalue()
        self.assertEqual(result.status, client.EXIT_DEGRADED, errors)
        self.assertEqual(errors.count('REJECTED SurvivabilityRiskLevel payload (ProtocolError'), 3, errors)
        self.assertEqual(errors.count('REJECTED RDMA_InitializeSetup payload (ProtocolError'), 1, errors)
        self.assertEqual(errors.count('REJECTED Log payload (ProtocolError'), 1, errors)
        self.assertEqual(errors.count('REJECTED SurvivabilityRiskLevel payload (CodecFailure: payload '
                                      'decoder rejected message)'), 2, errors)
        self.assertNotIn('REJECTED RDMA_InitializeSetup payload (CodecFailure', errors)
        self.assertNotIn('REJECTED Log payload (CodecFailure', errors)
        self.assertEqual(result.rejected_by_message, {SURV: 5, RDMA: 1, LOG: 1})
        self.assertEqual(result.received_by_message, {RDMA: 1, LOG: 1, SURV: 1})
        self.assertEqual(sorted(type(v).__name__ for v in self.received),
                         ['LogMT', 'RDMA_InitializeSetupMT', 'SurvivabilityRiskLevelMT'])
        for private in ('2026-02-30', 'URGENT'):
            self.assertNotIn(private, errors + self.out.getvalue())
        self.assertEqual(len(peer.unsubscribed), 3)
        await self.assert_no_leaked_tasks()

    async def test_callback_failure_in_one_family_is_isolated(self):
        calls = []

        def failing(value):
            calls.append(value)
            if isinstance(value, BINDINGS.model.SurvivabilityRiskLevelMT):
                raise RuntimeError('private application detail')

        async with Peer() as peer:
            task = self.start(peer, callback=failing)
            await self.all_subscribed(peer)
            await peer.deliver(RDMA, wire(RDMA, golden('2.5')))
            await peer.deliver(SURV, wire(SURV, survivability_golden('2.5')))
            await peer.deliver(LOG, wire(LOG, log_golden('2.5')))
            await peer.deliver(SURV, wire(SURV, survivability_golden('2.5')))
            await peer.deliver(RDMA, wire(RDMA, golden('2.5')))
            await until(lambda: len(calls) == 5)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.callback_failures, result.received),
                         (client.EXIT_DEGRADED, 2, 5))
        self.assertEqual(result.received_by_message, {RDMA: 2, SURV: 2, LOG: 1})
        self.assertEqual(self.err.getvalue().count(') message=SurvivabilityRiskLevel'), 2)
        self.assertRegex(self.err.getvalue(), r'CALLBACK FAILED #\d \(RuntimeError\) '
                                              r'message=SurvivabilityRiskLevel')
        self.assertNotIn('message=Log', self.err.getvalue())
        self.assertNotIn('private application detail', self.err.getvalue())
        self.assertEqual(len(peer.unsubscribed), 3)
        await self.assert_no_leaked_tasks()

    async def test_rejected_subscription_at_each_position_releases_earlier_sids(self):
        for position, name in enumerate(NAMES):
            with self.subTest(rejected=name):
                async with Peer(sub_error=name) as peer:
                    result = await asyncio.wait_for(self.start(peer), 10)
                    await asyncio.wait_for(peer.closed.wait(), 5)
                earlier = [peer.sids[n] for n in NAMES[:position]]
                self.assertEqual(result.status, client.EXIT_REJECTED)
                self.assertIn(f'SUBSCRIBE REJECTED by server: Illegal-State message={name}',
                              self.err.getvalue())
                self.assertNotIn('private-detail', self.err.getvalue())
                self.assertNotIn('READY', self.out.getvalue())
                # No later SUB is ever sent; no ghost earlier subscription.
                self.assertEqual(len(self.sub_frames(peer)), position + 1)
                self.assertEqual(peer.unsubscribed, earlier[::-1])
                self.assertEqual(result.subscription_ids, earlier)
                self.assertEqual(result.unsubscribed_ids, earlier[::-1])
                self.assertFalse(result.subscribed)
                self.assertTrue(result.closed)
                self.assertEqual(result.unsubscribed, bool(earlier))
                self.assertEqual(self.out.getvalue().splitlines()[-1],
                                 'CLOSED received=0 rejected=0 callback-failures=0 '
                                 'RDMA_InitializeSetup=0/0 Log=0/0 SurvivabilityRiskLevel=0/0')
                await self.assert_no_leaked_tasks()

    async def test_connection_lost_during_third_subscription(self):
        async with Peer(abort_on=SURV) as peer:
            result = await asyncio.wait_for(self.start(peer), 10)
        self.assertEqual(result.status, client.EXIT_SESSION)
        errors = self.err.getvalue()
        self.assertIn('SUBSCRIBE FAILED (', errors)
        self.assertIn('message=SurvivabilityRiskLevel', errors)
        self.assertNotIn('READY', self.out.getvalue())
        # Earlier SIDs cannot be UNSUBed on a dead connection; connection close
        # releases them at the broker. They are reported, never claimed released.
        rdma_sid, log_sid = peer.sids[RDMA], peer.sids[LOG]
        self.assertLess(errors.index(f'UNSUBSCRIBE SKIPPED sid={log_sid} message=Log'),
                        errors.index(f'UNSUBSCRIBE SKIPPED sid={rdma_sid} message=RDMA_InitializeSetup'))
        self.assertNotIn('UNSUBSCRIBED', self.out.getvalue())
        self.assertEqual(peer.unsubscribed, [])
        self.assertEqual((result.subscription_ids, result.unsubscribed_ids), ([rdma_sid, log_sid], []))
        self.assertTrue(result.closed and not result.unsubscribed)
        await self.assert_no_leaked_tasks()

    async def test_cancellation_between_each_pair_of_subscriptions(self):
        for held in (LOG, SURV):
            with self.subTest(held=held):
                async with Peer(hold=held) as peer:
                    task = self.start(peer)
                    await asyncio.wait_for(peer.sub_seen[held].wait(), 5)
                    task.cancel()
                    with self.assertRaises(asyncio.CancelledError):
                        await asyncio.wait_for(task, 10)
                    # The in-flight acknowledgement is ambiguous: the runtime fails
                    # closed and closes the one connection, releasing earlier SIDs.
                    await asyncio.wait_for(peer.closed.wait(), 5)
                self.assertNotIn('READY', self.out.getvalue())
                self.assertIn('CLOSED received=0', self.out.getvalue())
                self.assertEqual(self.received, [])
                self.assertEqual(peer.connections, 1)
                await self.assert_no_leaked_tasks()

    async def test_failure_immediately_after_final_acknowledgement_is_never_ready(self):
        async with Peer(double_ack=SURV) as peer:
            result = await asyncio.wait_for(self.start(peer), 10)
        errors = self.err.getvalue()
        self.assertEqual(result.status, client.EXIT_SESSION, errors)
        # The unsolicited +OK fails the session either before READY is
        # evaluated (NOT READY) or immediately after; both end in exit 6.
        self.assertTrue('NOT READY' in errors or 'SESSION FAILED (ProtocolError' in errors, errors)
        self.assertEqual(self.received, [])
        self.assertEqual(len(result.subscription_ids), 3)
        self.assertTrue(result.closed)
        self.assertEqual(peer.connections, 1)
        await self.assert_no_leaked_tasks()

    async def test_shutdown_with_queued_data_of_all_three_families(self):
        release, calls = asyncio.Event(), []

        async def slow(value):
            calls.append(value)
            await release.wait()

        async with Peer() as peer:
            task = self.start(peer, callback=slow, buffer_size=1)
            await self.all_subscribed(peer)
            for _ in range(4):
                await peer.deliver(RDMA, wire(RDMA, golden('2.5')))
                await peer.deliver(LOG, wire(LOG, log_golden('2.5')))
                await peer.deliver(SURV, wire(SURV, survivability_golden('2.5')))
            await until(lambda: len(calls) == 1)
            await asyncio.sleep(.2)
            self.assertEqual(len(calls), 1)  # bounded: the inbox holds the rest back
            self.stop.set()
            await until(lambda: len(peer.unsubscribed) == 3)
            release.set()
            result = await asyncio.wait_for(task, 10)
            await asyncio.sleep(.2)
        # Values still queued in any subscription were discarded on teardown:
        # no stale callback ran after shutdown began.
        self.assertEqual(len(calls), 1)
        self.assertEqual((result.status, result.received), (client.EXIT_OK, 1))
        self.assertEqual(peer.unsubscribed, [peer.sids[n] for n in NAMES][::-1])
        await self.assert_no_leaked_tasks()

    async def test_max_messages_counts_all_three_families(self):
        async with Peer() as peer:
            task = self.start(peer, max_messages=4)
            await self.all_subscribed(peer)
            await peer.deliver(SURV, wire(SURV, survivability_golden('2.5')))
            await peer.deliver(LOG, wire(LOG, log_golden('2.5')))
            await peer.deliver(RDMA, wire(RDMA, golden('2.5')))
            await until(lambda: len(self.received) == 3)
            await peer.deliver(SURV, wire(SURV, survivability_golden('2.5')))
            await peer.deliver(LOG, wire(LOG, log_golden('2.5')))
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.stop_reason, result.received, len(self.received)),
                         (client.EXIT_OK, 'max-messages', 4, 4))
        self.assertEqual(sum(result.received_by_message.values()), 4)
        self.assertEqual(result.received_by_message[SURV], 2)
        self.assertEqual(len(peer.unsubscribed), 3)
        await self.assert_no_leaked_tasks()

    async def test_mode_and_bindings_must_agree(self):
        two = client.load_bindings(bindings_directory('2.5', with_log=True), '2.5', with_log=True)
        for extra, bindings in ((dict(with_survivability=False), BINDINGS),
                                (dict(with_log=False, with_survivability=False), BINDINGS),
                                (dict(), two)):
            with self.subTest(extra=extra):
                with self.assertRaises(client.BindingsError):
                    await client.run_client(options('ws://127.0.0.1:9', **extra), bindings=bindings)
        with self.assertRaises(ValueError):
            await client.run_client(options('ws://127.0.0.1:9', with_log=False), bindings=BINDINGS)
        await self.assert_no_leaked_tasks()


if __name__ == '__main__':
    unittest.main()
