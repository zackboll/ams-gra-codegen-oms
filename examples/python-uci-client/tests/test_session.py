"""Application session lifecycle against a local OWP peer and actual generated code.

The peer follows python/oms-lacal-runtime/tests/test_mock.py: a real WebSocket
server with the owp subprotocol. It plays the BROKER role only; the client side
is the unchanged runtime plus the actual generated UCI 2.5 package.
"""
import asyncio
import copy
import io
import json
import unittest

from websockets.asyncio.server import serve
from websockets.exceptions import ConnectionClosed

from support import bindings_directory, cleanup_bindings, client, golden

INFO = {'version': '1.0', 'server_id': 'mock', 'system_label': 'Mock',
        'uuids': {'system': '550e8400-e29b-41d4-a716-446655440000',
                  'service': '550e8400-e29b-41d4-a716-446655440001'}}
BINDINGS = None


def setUpModule():
    global BINDINGS
    BINDINGS = client.load_bindings(bindings_directory('2.5'), '2.5')


def tearDownModule():
    cleanup_bindings()


def envelope(body):
    return json.dumps({'RDMA_InitializeSetup': body})


class Peer:
    def __init__(self, *, init_error=None, sub_error=None, subprotocol='owp'):
        self.init_error, self.sub_error, self.subprotocol = init_error, sub_error, subprotocol
        self.frames, self.sids = [], []
        self.subscribed, self.unsubscribed = asyncio.Event(), asyncio.Event()
        self.socket = None

    async def __aenter__(self):
        self.server = await serve(self.handle, '127.0.0.1', 0,
                                  subprotocols=[self.subprotocol] if self.subprotocol else None)
        self.url = 'ws://127.0.0.1:' + str(self.server.sockets[0].getsockname()[1])
        return self

    async def __aexit__(self, *exc):
        self.server.close()
        await self.server.wait_closed()

    async def handle(self, ws):
        self.socket = ws
        try:
            async for frame in ws:
                self.frames.append(frame)
                if frame.startswith('INIT '):
                    if self.init_error:
                        await ws.send('-ERR ' + self.init_error + ' private-detail')
                        continue
                    await ws.send('+OK')
                    await ws.send('INFO ' + json.dumps(INFO))
                elif frame.startswith('SUB '):
                    if self.sub_error:
                        await ws.send('-ERR ' + self.sub_error + ' private-detail')
                        continue
                    self.sids.append(frame.split()[1])
                    await ws.send('+OK')
                    self.subscribed.set()
                elif frame.startswith('UNSUB '):
                    await ws.send('+OK')
                    self.unsubscribed.set()
        except ConnectionClosed:
            pass

    async def deliver(self, text, sid=None):
        await self.socket.send(f'MSG {sid or self.sids[-1]} {text}')


def options(url, **extra):
    values = dict(url=url, service_id='python-uci-client', schema_release='2.5',
                  bindings=bindings_directory('2.5'), duration=30)
    values.update(extra)
    return client.Options(**values)


async def until(predicate, timeout=5):
    async with asyncio.timeout(timeout):
        while not predicate():
            await asyncio.sleep(.01)


class Session(unittest.IsolatedAsyncioTestCase):
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
            options(peer.url, **extra), bindings=BINDINGS, callback=callback or self.received.append,
            stop=self.stop, stdout=self.out, stderr=self.err, buffer_size=buffer_size))

    async def test_typed_values_reach_callback_in_order_then_clean_stop(self):
        async with Peer() as peer:
            task = self.start(peer)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            self.assertEqual(peer.frames[-1], f'SUB {peer.sids[0]} RDMA_InitializeSetup uci.rdma')
            self.assertEqual(json.loads(peer.frames[0].split(' ', 1)[1]),
                             dict(versions=['1.0'], schema='002.5.0',
                                  service_id='python-uci-client', verbose=True))
            first, second = golden('2.5'), copy.deepcopy(golden('2.5'))
            second['MessageData']['RDMA_InitializeSetupID']['UUID'] = '00000000-0000-0000-0000-000000000000'
            second['MessageData']['Endpoint'] = [{'IPv6_Endpoint': {'IPv6_Address': 'fE80::aBcD', 'Prefix': 64}}]
            second['MessageData']['RDMA_Provider'] = 'VERBS'
            for body in (first, second):
                await peer.deliver(envelope(body))
            await until(lambda: len(self.received) == 2)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual(result.status, client.EXIT_OK, self.err.getvalue())
        self.assertEqual([type(v) for v in self.received], [BINDINGS.model.RDMA_InitializeSetupMT] * 2)
        self.assertEqual(self.received[0], BINDINGS.codec.decode_RDMA_InitializeSetupMT(json.dumps(first)))
        typed = self.received[1].messagedata
        self.assertIs(typed.rdma_provider, BINDINGS.model.RDMA_ProviderEnum.VERBS)
        self.assertEqual(typed.endpoint[0].alternative, 'IPv6_Endpoint')
        self.assertIs(type(typed.endpoint[0].value.ipv6_address), BINDINGS.model.IPv6_AddressType)
        self.assertEqual((typed.endpoint[0].value.ipv6_address.value, typed.endpoint[0].value.prefix),
                         ('fE80::aBcD', 64))
        self.assertEqual(client.summarize(self.received[0])['endpoints'],
                         [('IPv4', '192.0.2.10', None), ('IPv6', '2001:db8::1', None)])
        self.assertIn(f'UNSUB {peer.sids[0]}', peer.frames)
        self.assertEqual((result.stop_reason, result.received, result.subscribed, result.unsubscribed,
                          result.closed), ('stopped', 2, True, True, True))
        await self.assert_no_leaked_tasks()

    async def test_invalid_payloads_are_reported_and_subscription_recovers(self):
        bad_uuid = copy.deepcopy(golden('2.5'))
        bad_uuid['MessageHeader']['Timestamp'] = '2026-02-30T03:04:05Z'
        wrong_name = json.dumps({'Log': golden('2.5')})
        duplicate = '{"RDMA_InitializeSetup":{},"RDMA_InitializeSetup":{}}'
        async with Peer() as peer:
            task = self.start(peer)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            for text in (envelope(bad_uuid), wrong_name, duplicate, 'not json', envelope({})):
                await peer.deliver(text)
            await peer.deliver(envelope(golden('2.5')))
            await until(lambda: len(self.received) == 1)
            await until(lambda: self.err.getvalue().count('REJECTED') == 5)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        errors = self.err.getvalue()
        self.assertEqual((result.status, result.rejected, result.received),
                         (client.EXIT_DEGRADED, 5, 1), errors)
        self.assertEqual(errors.count('REJECTED RDMA_InitializeSetup payload (CodecFailure'), 2)
        self.assertEqual(errors.count('REJECTED RDMA_InitializeSetup payload (ProtocolError'), 3)
        self.assertNotIn('2026-02-30', errors + self.out.getvalue())
        self.assertTrue(result.unsubscribed)
        await self.assert_no_leaked_tasks()

    async def test_callback_failure_is_isolated_and_reported(self):
        calls = []

        def failing(value):
            calls.append(value)
            if len(calls) == 1:
                raise RuntimeError('private application detail')

        async with Peer() as peer:
            task = self.start(peer, callback=failing)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            await peer.deliver(envelope(golden('2.5')))
            await peer.deliver(envelope(golden('2.5')))
            await until(lambda: len(calls) == 2)
            self.stop.set()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.callback_failures, result.received),
                         (client.EXIT_DEGRADED, 1, 2))
        self.assertIn('CALLBACK FAILED #1 (RuntimeError)', self.err.getvalue())
        self.assertNotIn('private application detail', self.err.getvalue())
        await self.assert_no_leaked_tasks()

    async def test_server_rejection_of_init_and_sub(self):
        for peer_options, phase in ((dict(init_error='Unsupported-Service'), 'CONNECT'),
                                    (dict(sub_error='Illegal-State'), 'SUBSCRIBE')):
            with self.subTest(phase=phase):
                async with Peer(**peer_options) as peer:
                    result = await asyncio.wait_for(self.start(peer), 10)
                self.assertEqual(result.status, client.EXIT_REJECTED)
                self.assertIn(f'{phase} REJECTED by server: ', self.err.getvalue())
                self.assertNotIn('private-detail', self.err.getvalue())
                self.assertFalse(result.subscribed or result.unsubscribed)
                self.assertTrue(result.closed)
                await self.assert_no_leaked_tasks()

    async def test_missing_owp_negotiation_is_a_connect_failure(self):
        for subprotocol in (None, 'other'):
            with self.subTest(subprotocol=subprotocol):
                async with Peer(subprotocol=subprotocol) as peer:
                    result = await asyncio.wait_for(self.start(peer), 10)
                    self.assertFalse([f for f in peer.frames if f.startswith('INIT')])
                self.assertEqual(result.status, client.EXIT_CONNECT)
                self.assertIn('CONNECT FAILED (ProtocolError', self.err.getvalue())
                await self.assert_no_leaked_tasks()

    async def test_duration_expiry_and_message_limit_complete_cleanly(self):
        async with Peer() as peer:
            loop = asyncio.get_running_loop()
            started = loop.time()
            result = await asyncio.wait_for(self.start(peer, duration=1), 10)
            self.assertGreaterEqual(loop.time() - started, 1)
            self.assertIn(f'UNSUB {peer.sids[0]}', peer.frames)
        self.assertEqual((result.status, result.stop_reason, result.received), (client.EXIT_OK, 'duration', 0))
        self.assertIn('CLOSED received=0', self.out.getvalue())
        async with Peer() as peer:
            task = self.start(peer, max_messages=2)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            for _ in range(3):
                await peer.deliver(envelope(golden('2.5')))
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.stop_reason, result.received, len(self.received)),
                         (client.EXIT_OK, 'max-messages', 2, 2))
        await self.assert_no_leaked_tasks()

    async def test_connection_loss_after_subscribe_is_a_session_failure(self):
        async with Peer() as peer:
            task = self.start(peer)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            peer.socket.transport.abort()
            result = await asyncio.wait_for(task, 10)
        self.assertEqual((result.status, result.stop_reason), (client.EXIT_SESSION, 'failure'))
        self.assertIn('SESSION FAILED (TransportFailure', self.err.getvalue())
        self.assertTrue(result.closed)
        await self.assert_no_leaked_tasks()

    async def test_cancellation_during_slow_callback_unsubscribes_and_joins(self):
        entered, release = asyncio.Event(), asyncio.Event()

        async def slow(value):
            entered.set()
            await release.wait()

        async with Peer() as peer:
            task = self.start(peer, callback=slow)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            await peer.deliver(envelope(golden('2.5')))
            await asyncio.wait_for(entered.wait(), 5)
            task.cancel()
            with self.assertRaises(asyncio.CancelledError):
                await asyncio.wait_for(task, 10)
            await until(lambda: peer.unsubscribed.is_set())
        self.assertIn('CLOSED received=1', self.out.getvalue())
        await self.assert_no_leaked_tasks()

    async def test_bounded_buffer_backpressure_and_no_stale_callback_after_teardown(self):
        release, calls = asyncio.Event(), []

        async def slow(value):
            calls.append(value)
            await release.wait()

        async with Peer() as peer:
            task = self.start(peer, callback=slow, buffer_size=1)
            await asyncio.wait_for(peer.subscribed.wait(), 5)
            # 1 in callback + 1 application inbox + 1 runtime dispatch + <=16
            # runtime queue: bounded, no unbounded application buffering.
            for _ in range(6):
                await peer.deliver(envelope(golden('2.5')))
            await until(lambda: len(calls) == 1)
            await asyncio.sleep(.2)
            self.assertEqual(len(calls), 1)
            self.stop.set()
            await until(lambda: peer.unsubscribed.is_set())
            # Five decoded values are still buffered (inbox/runtime queue).
            # Teardown discards them: none may reach the callback afterwards.
            release.set()
            result = await asyncio.wait_for(task, 10)
            await asyncio.sleep(.2)
        self.assertEqual(len(calls), 1)
        self.assertEqual((result.status, result.received), (client.EXIT_OK, 1))
        await self.assert_no_leaked_tasks()

    async def test_buffer_size_is_validated(self):
        for bad in (0, client.MAX_BUFFER + 1, True, 1.5):
            with self.assertRaises(ValueError):
                await client.run_client(options('ws://127.0.0.1:9'), bindings=BINDINGS, buffer_size=bad)


if __name__ == '__main__':
    unittest.main()
