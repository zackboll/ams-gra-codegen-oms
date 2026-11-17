"""PY-OMS-021 --with-log session lifecycle: one connection, two generated subscriptions.

A local OWP peer (same shape as test_session.Peer) plays the BROKER role only;
the client side is the unchanged runtime plus the actual generated combined
UCI 2.5 RDMA_InitializeSetup + Log package. The peer can reject, abort on or
withhold the acknowledgement of a chosen SUB so partial setup is deterministic.
"""
import asyncio
import io
import json
import unittest

from websockets.asyncio.server import serve
from websockets.exceptions import ConnectionClosed

from support import (bindings_directory, cleanup_bindings, client, golden, log_golden,
                     log_variant, variant)

INFO = {'version': '1.0', 'server_id': 'mock', 'system_label': 'Mock',
        'uuids': {'system': '550e8400-e29b-41d4-a716-446655440000',
                  'service': '550e8400-e29b-41d4-a716-446655440001'}}
BINDINGS = RDMA_ONLY = None
UUIDS = ['5b0f6d4e-1c2a-4b7e-9a3d-0123456789ab', '00000000-0000-0000-0000-000000000000',
         'aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee']


def setUpModule():
    global BINDINGS, RDMA_ONLY
    BINDINGS = client.load_bindings(bindings_directory('2.5', with_log=True), '2.5', with_log=True)
    RDMA_ONLY = client.load_bindings(bindings_directory('2.5'), '2.5')


def tearDownModule():
    cleanup_bindings()


def rdma(body):
    return json.dumps({'RDMA_InitializeSetup': body})


def log(body):
    return json.dumps({'Log': body})


class Peer:
    """Local OWP broker. `sub_error`/`abort_on`/`hold`/`double_ack` name a message."""

    def __init__(self, *, sub_error=None, abort_on=None, hold=None, double_ack=None):
        self.sub_error, self.abort_on, self.hold, self.double_ack = sub_error, abort_on, hold, double_ack
        self.frames, self.sids, self.connections = [], {}, 0
        self.subscribed = {'RDMA_InitializeSetup': asyncio.Event(), 'Log': asyncio.Event()}
        self.sub_seen = {'RDMA_InitializeSetup': asyncio.Event(), 'Log': asyncio.Event()}
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
                  bindings=bindings_directory('2.5', with_log=True), duration=30, with_log=True)
    values.update(extra)
    return client.Options(**values)


async def until(predicate, timeout=5):
    async with asyncio.timeout(timeout):
        while not predicate():
            await asyncio.sleep(.01)


class MultiMessage(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.initial_tasks = set(asyncio.all_tasks())

    async def assert_no_leaked_tasks(self):
        await asyncio.sleep(0)
        current = asyncio.current_task()
        self.assertEqual([t for t in asyncio.all_tasks() - self.initial_tasks
                          if t is not current and not t.done()], [])

    def start(self, peer, callback=None, buffer_size=4, bindings=None, **extra):
        self.out, self.err, self.received, self.stop = io.StringIO(), io.StringIO(), [], asyncio.Event()
        return asyncio.create_task(client.run_client(
            options(peer.url, **extra), bindings=bindings or BINDINGS,
            callback=callback or self.received.append, stop=self.stop,
            stdout=self.out, stderr=self.err, buffer_size=buffer_size))

    async def both_subscribed(self, peer):
        for event in peer.subscribed.values():
            await asyncio.wait_for(event.wait(), 5)
        await until(lambda: 'READY ' in self.out.getvalue())

    def sub_frames(self, peer):
        return [frame for frame in peer.frames if frame.startswith('SUB ')]

    async def test_interleaved_typed_delivery_on_one_connection(self):
        model = BINDINGS.model
        rdma_bodies = [variant('2.5', UUIDS[0]), variant('2.5', UUIDS[1], provider='VERBS'),
                       variant('2.5', UUIDS[2], provider='UDP')]
        log_bodies = [log_variant('2.5', 'INFO'), log_variant('2.5', 'WARNING'),
                      log_variant('2.5', 'FATAL', log_timestamp='2026-06-30T23:59:59.5Z')]
        async with Peer() as peer:
            task = self.start(peer)
            await self.both_subscribed(peer)
            rdma_sid, log_sid = peer.sids['RDMA_InitializeSetup'], peer.sids['Log']
            self.assertNotEqual(rdma_sid, log_sid)
            self.assertEqual(self.sub_frames(peer), [f'SUB {rdma_sid} RDMA_InitializeSetup uci.rdma',
                                                     f'SUB {log_sid} Log uci.log'])
            # One INIT, one connection; READY only after both acknowledgements.
            self.assertEqual(len([f for f in peer.frames if f.startswith('INIT ')]), 1)
            lines = self.out.getvalue().splitlines()
            self.assertEqual([line.split()[0] for line in lines],
                             ['CONNECTED', 'SUBSCRIBED', 'SUBSCRIBED', 'READY'])
            self.assertEqual(lines[3], 'READY subscriptions=2 messages=RDMA_InitializeSetup,Log '
                                       'duration=30s')
            for rdma_body, log_body in zip(rdma_bodies, log_bodies):
                await peer.deliver('Log', log(log_body))
                await peer.deliver('RDMA_InitializeSetup', rdma(rdma_body))
            await until(lambda: len(self.received) == 6)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual(result.status, client.EXIT_OK, self.err.getvalue())
        self.assertEqual(peer.connections, 1)
        rdma_values = [v for v in self.received if isinstance(v, model.RDMA_InitializeSetupMT)]
        log_values = [v for v in self.received if isinstance(v, model.LogMT)]
        # Actual generated types; ordered within each subscription; no duplicates.
        self.assertEqual(len(rdma_values) + len(log_values), 6)
        self.assertEqual([v.messagedata.rdma_initializesetupid.uuid.value for v in rdma_values], UUIDS)
        self.assertEqual([v.messagedata.severity for v in log_values],
                         [model.LogSeverityEnum.INFO, model.LogSeverityEnum.WARNING,
                          model.LogSeverityEnum.FATAL])
        self.assertEqual(log_values[0], BINDINGS.codec.decode_LogMT(json.dumps(log_bodies[0])))
        self.assertEqual({client.message_name(v) for v in self.received}, {'Log', 'RDMA_InitializeSetup'})
        self.assertEqual(client.summarize_log(log_values[2])['log_timestamp'], '2026-06-30T23:59:59.5Z')
        self.assertEqual(result.received_by_message, {'RDMA_InitializeSetup': 3, 'Log': 3})
        self.assertEqual(result.subscription_ids, [rdma_sid, log_sid])
        # Both owned SIDs released, in reverse SUB order, before close.
        self.assertEqual(peer.unsubscribed, [log_sid, rdma_sid])
        self.assertEqual(result.unsubscribed_ids, [log_sid, rdma_sid])
        lines = self.out.getvalue().splitlines()
        self.assertEqual(lines[-3:], [f'UNSUBSCRIBED sid={log_sid} message=Log',
                                      f'UNSUBSCRIBED sid={rdma_sid} message=RDMA_InitializeSetup',
                                      'CLOSED received=6 rejected=0 callback-failures=0 '
                                      'RDMA_InitializeSetup=3/0 Log=3/0'])
        self.assertTrue(result.unsubscribed and result.closed)
        await self.assert_no_leaked_tasks()

    async def test_default_printing_callback_formats_both_families(self):
        out, err, stop = io.StringIO(), io.StringIO(), asyncio.Event()
        async with Peer() as peer:
            task = asyncio.create_task(client.run_client(
                options(peer.url), bindings=BINDINGS, stop=stop, stdout=out, stderr=err))
            for event in peer.subscribed.values():
                await asyncio.wait_for(event.wait(), 5)
            await peer.deliver('RDMA_InitializeSetup', rdma(variant('2.5', UUIDS[0])))
            await until(lambda: 'RECEIVED RDMA' in out.getvalue())
            await peer.deliver('Log', log(log_golden('2.5')))
            await until(lambda: 'RECEIVED Log' in out.getvalue())
            stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual(result.status, client.EXIT_OK, err.getvalue())
        received = [line for line in out.getvalue().splitlines() if line.startswith('RECEIVED ')]
        self.assertTrue(received[0].startswith(f'RECEIVED RDMA_InitializeSetup #1 '
                                               f'type=RDMA_InitializeSetupMT setup-id={UUIDS[0]} '))
        self.assertEqual(received[1], 'RECEIVED Log #2 type=LogMT severity=INFO '
                                      'log-timestamp=2026-01-02T03:04:05Z mode=LIVE schema=002.5.0 '
                                      'timestamp=2026-01-02T03:04:05Z classification=U '
                                      'service-id=- subsystem-id=- components=0 capabilities=0')
        # Free text, binary data and security caveats are never printed.
        for private in ('PY016', 'Independent full-schema golden', '00FF', 'NATO:EXAMPLE', 'a b'):
            self.assertNotIn(private, out.getvalue() + err.getvalue())
        await self.assert_no_leaked_tasks()

    async def test_no_cross_subscription_decoding_and_decoder_recovery(self):
        bad_date = log_variant('2.5', timestamp='2026-02-30T03:04:05Z')
        bad_severity = log_variant('2.5', 'LOUD')
        async with Peer() as peer:
            task = self.start(peer)
            await self.both_subscribed(peer)
            # Each SID unwraps only its own global element: a Log document on
            # the RDMA SID (and vice versa) is an envelope rejection that never
            # reaches the other family's generated decoder.
            await peer.deliver('RDMA_InitializeSetup', log(log_golden('2.5')))
            await peer.deliver('Log', rdma(golden('2.5')))
            # Generated LogMT decoder rejections (the envelope is correct).
            await peer.deliver('Log', log(bad_date))
            await peer.deliver('Log', log(bad_severity))
            # RDMA and Log reception both continue afterwards.
            await peer.deliver('RDMA_InitializeSetup', rdma(golden('2.5')))
            await peer.deliver('Log', log(log_golden('2.5')))
            await until(lambda: len(self.received) == 2)
            await until(lambda: self.err.getvalue().count('REJECTED') == 4)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        errors = self.err.getvalue()
        self.assertEqual(result.status, client.EXIT_DEGRADED, errors)
        self.assertEqual(errors.count('REJECTED RDMA_InitializeSetup payload (ProtocolError'), 1, errors)
        self.assertEqual(errors.count('REJECTED Log payload (ProtocolError'), 1, errors)
        self.assertEqual(errors.count('REJECTED Log payload (CodecFailure: payload decoder rejected '
                                      'message)'), 2, errors)
        self.assertEqual(result.rejected_by_message, {'RDMA_InitializeSetup': 1, 'Log': 3})
        self.assertEqual(result.received_by_message, {'RDMA_InitializeSetup': 1, 'Log': 1})
        self.assertEqual(sorted(type(v).__name__ for v in self.received),
                         ['LogMT', 'RDMA_InitializeSetupMT'])
        self.assertNotIn('2026-02-30', errors + self.out.getvalue())
        self.assertNotIn('LOUD', errors + self.out.getvalue())
        self.assertEqual(len(peer.unsubscribed), 2)
        await self.assert_no_leaked_tasks()

    async def test_callback_failure_in_one_family_is_isolated(self):
        calls = []

        def failing(value):
            calls.append(value)
            if isinstance(value, BINDINGS.model.LogMT):
                raise RuntimeError('private application detail')

        async with Peer() as peer:
            task = self.start(peer, callback=failing)
            await self.both_subscribed(peer)
            await peer.deliver('Log', log(log_golden('2.5')))
            await peer.deliver('RDMA_InitializeSetup', rdma(golden('2.5')))
            await peer.deliver('RDMA_InitializeSetup', rdma(golden('2.5')))
            await until(lambda: len(calls) == 3)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.callback_failures, result.received),
                         (client.EXIT_DEGRADED, 1, 3))
        self.assertEqual(result.received_by_message, {'Log': 1, 'RDMA_InitializeSetup': 2})
        self.assertRegex(self.err.getvalue(), r'CALLBACK FAILED #\d \(RuntimeError\) message=Log')
        self.assertNotIn('private application detail', self.err.getvalue())
        await self.assert_no_leaked_tasks()

    async def test_rejected_second_subscription_releases_the_first(self):
        async with Peer(sub_error='Log') as peer:
            result = await asyncio.wait_for(self.start(peer), 10)
            await asyncio.wait_for(peer.closed.wait(), 5)
        rdma_sid = peer.sids['RDMA_InitializeSetup']
        self.assertEqual(result.status, client.EXIT_REJECTED)
        self.assertIn('SUBSCRIBE REJECTED by server: Illegal-State message=Log', self.err.getvalue())
        self.assertNotIn('private-detail', self.err.getvalue())
        self.assertNotIn('READY', self.out.getvalue())
        # No ghost first subscription: it was explicitly UNSUBed, then closed.
        self.assertEqual(peer.unsubscribed, [rdma_sid])
        self.assertEqual(result.subscription_ids, [rdma_sid])
        self.assertEqual(result.unsubscribed_ids, [rdma_sid])
        self.assertFalse(result.subscribed)
        self.assertTrue(result.closed)
        self.assertEqual(self.out.getvalue().splitlines()[-2:], [
            f'UNSUBSCRIBED sid={rdma_sid} message=RDMA_InitializeSetup',
            'CLOSED received=0 rejected=0 callback-failures=0 RDMA_InitializeSetup=0/0 Log=0/0'])
        await self.assert_no_leaked_tasks()

    async def test_rejected_first_subscription_never_sends_the_second(self):
        async with Peer(sub_error='RDMA_InitializeSetup') as peer:
            result = await asyncio.wait_for(self.start(peer), 10)
        self.assertEqual(result.status, client.EXIT_REJECTED)
        self.assertEqual(len(self.sub_frames(peer)), 1)
        self.assertEqual((peer.unsubscribed, result.subscription_ids), ([], []))
        self.assertIn('SUBSCRIBE REJECTED by server: Illegal-State message=RDMA_InitializeSetup',
                      self.err.getvalue())
        await self.assert_no_leaked_tasks()

    async def test_connection_lost_during_second_subscription(self):
        async with Peer(abort_on='Log') as peer:
            result = await asyncio.wait_for(self.start(peer), 10)
        self.assertEqual(result.status, client.EXIT_SESSION)
        self.assertIn('SUBSCRIBE FAILED (', self.err.getvalue())
        self.assertNotIn('READY', self.out.getvalue())
        # The first SID cannot be UNSUBed on a dead connection; connection close
        # releases it at the broker. It is reported, never silently dropped.
        rdma_sid = peer.sids['RDMA_InitializeSetup']
        self.assertIn(f'UNSUBSCRIBE SKIPPED sid={rdma_sid} message=RDMA_InitializeSetup',
                      self.err.getvalue())
        self.assertEqual(peer.unsubscribed, [])
        self.assertTrue(result.closed and not result.unsubscribed)
        await self.assert_no_leaked_tasks()

    async def test_failure_after_both_acknowledgements_is_never_ready(self):
        async with Peer(double_ack='Log') as peer:
            result = await asyncio.wait_for(self.start(peer), 10)
        errors = self.err.getvalue()
        self.assertEqual(result.status, client.EXIT_SESSION, errors)
        # The unsolicited +OK fails the session either before READY is
        # evaluated (NOT READY) or immediately after; both end in exit 6.
        self.assertTrue('NOT READY' in errors or 'SESSION FAILED (ProtocolError' in errors, errors)
        self.assertEqual(self.received, [])
        self.assertTrue(result.closed)
        self.assertEqual(peer.connections, 1)
        await self.assert_no_leaked_tasks()

    async def test_cancellation_between_subscriptions(self):
        async with Peer(hold='Log') as peer:
            task = self.start(peer)
            await asyncio.wait_for(peer.subscribed['RDMA_InitializeSetup'].wait(), 5)
            await asyncio.wait_for(peer.sub_seen['Log'].wait(), 5)
            task.cancel()
            with self.assertRaises(asyncio.CancelledError):
                await asyncio.wait_for(task, 10)
            # The Log acknowledgement is ambiguous: the runtime fails closed and
            # closes the one connection, which releases the RDMA SID too.
            await asyncio.wait_for(peer.closed.wait(), 5)
        self.assertNotIn('READY', self.out.getvalue())
        self.assertIn('CLOSED received=0', self.out.getvalue())
        self.assertEqual(self.received, [])
        await self.assert_no_leaked_tasks()

    async def test_shutdown_with_pending_messages_of_both_families(self):
        release, calls = asyncio.Event(), []

        async def slow(value):
            calls.append(value)
            await release.wait()

        async with Peer() as peer:
            task = self.start(peer, callback=slow, buffer_size=1)
            await self.both_subscribed(peer)
            for _ in range(4):
                await peer.deliver('RDMA_InitializeSetup', rdma(golden('2.5')))
                await peer.deliver('Log', log(log_golden('2.5')))
            await until(lambda: len(calls) == 1)
            await asyncio.sleep(.2)
            self.assertEqual(len(calls), 1)
            self.stop.set()
            await until(lambda: len(peer.unsubscribed) == 2)
            release.set()
            result = await asyncio.wait_for(task, 10)
            await asyncio.sleep(.2)
        # Values still queued in either subscription were discarded on teardown.
        self.assertEqual(len(calls), 1)
        self.assertEqual((result.status, result.received), (client.EXIT_OK, 1))
        await self.assert_no_leaked_tasks()

    async def test_max_messages_counts_both_families(self):
        async with Peer() as peer:
            task = self.start(peer, max_messages=3)
            await self.both_subscribed(peer)
            await peer.deliver('Log', log(log_golden('2.5')))
            await peer.deliver('RDMA_InitializeSetup', rdma(golden('2.5')))
            await until(lambda: len(self.received) == 2)
            await peer.deliver('Log', log(log_golden('2.5')))
            await peer.deliver('RDMA_InitializeSetup', rdma(golden('2.5')))
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.stop_reason, result.received, len(self.received)),
                         (client.EXIT_OK, 'max-messages', 3, 3))
        self.assertEqual(sum(result.received_by_message.values()), 3)
        self.assertEqual(len(peer.unsubscribed), 2)
        await self.assert_no_leaked_tasks()

    async def test_mode_and_bindings_must_agree(self):
        for with_log, bindings in ((True, RDMA_ONLY), (False, BINDINGS)):
            with self.subTest(with_log=with_log):
                with self.assertRaises(client.BindingsError):
                    await client.run_client(options('ws://127.0.0.1:9', with_log=with_log),
                                            bindings=bindings)
        await self.assert_no_leaked_tasks()


if __name__ == '__main__':
    unittest.main()
