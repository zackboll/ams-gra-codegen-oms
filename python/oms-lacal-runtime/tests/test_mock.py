import asyncio
from dataclasses import replace, FrozenInstanceError
import json
import ssl
import subprocess
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import websockets
from websockets.asyncio.server import serve

from oms_lacal import (
    CodecBinding, CodecRegistry, CodecFailure, CodecInternalFailure, HandlerFailure,
    InvalidInput, LacalRuntime, NotConnected, OperationTimeout, ProtocolError,
    ResourceLimit, RuntimeConfig, ServerRejection, TransportFailure, UnknownSubscription,
)
from support import Generated, OAM

INFO = {'version':'1.0', 'server_id':'mock', 'system_label':'Mock',
        'uuids':{'system':'550e8400-e29b-41d4-a716-446655440000',
                 'service':'550e8400-e29b-41d4-a716-446655440001'}}


class Peer:
    def __init__(self, *, handshake='normal', ack=True):
        self.handshake = handshake
        self.ack = ack
        self.frames = asyncio.Queue()
        self.sockets = []
        self.connected = asyncio.Event()

    async def __aenter__(self):
        subprotocols = None if self.handshake == 'no-protocol' else ['owp']
        self.server = await serve(self.handle, '127.0.0.1', 0, subprotocols=subprotocols,
                                  max_size=2*1024*1024)
        self.url = 'ws://127.0.0.1:' + str(self.server.sockets[0].getsockname()[1])
        return self

    async def __aexit__(self, *exc):
        self.server.close()
        await self.server.wait_closed()

    async def handle(self, ws):
        self.sockets.append(ws)
        self.connected.set()
        try:
            initial = await ws.recv()
            await self.frames.put(initial)
            if self.handshake == 'silent':
                await ws.wait_closed()
                return
            if self.handshake == 'reject':
                await ws.send('-ERR Unsupported-Service secret-private-payload')
                return
            if self.handshake == 'wrong-order':
                await ws.send('INFO ' + json.dumps(INFO))
                return
            if self.handshake == 'binary':
                await ws.send(b'+OK')
                return
            if self.handshake == 'extra-ack':
                await ws.send('+OK extra')
                return
            await ws.send('+OK')
            data = dict(INFO)
            if self.handshake == 'bad-info':
                data['version'] = '9.0'
            if self.handshake == 'bad-uuid':
                data['uuids'] = {'system':'invalid','service':'invalid'}
            await ws.send('INFO ' + json.dumps(data))
            async for frame in ws:
                await self.frames.put(frame)
                if self.ack:
                    await ws.send('-ERR Illegal-State secret-private-payload' if ' forbidden' in frame else '+OK')
        except websockets.exceptions.ConnectionClosed:
            pass

    async def send(self, text):
        await self.sockets[-1].send(text)


class MockTests(unittest.IsolatedAsyncioTestCase):
    @classmethod
    def setUpClass(cls):
        cls.generated = Generated()
        cls.foreign = Generated('urn:python:codec')

    @classmethod
    def tearDownClass(cls):
        cls.generated.close()
        cls.foreign.close()

    async def asyncTearDown(self):
        await asyncio.sleep(0)
        leaked = [task for task in asyncio.all_tasks() if task is not asyncio.current_task()
                  and task.get_name().startswith('oms-lacal-') and not task.done()]
        self.assertEqual(leaked, [], 'runtime-owned tasks must be joined')

    def config(self, peer, **changes):
        return RuntimeConfig(peer.url, 'explicit-service-id', '002.5.0', **changes)

    async def event(self, runtime, expected):
        result = await asyncio.wait_for(runtime.next_event(), 2)
        self.assertIsInstance(result.error, expected)
        self.assertNotIn('secret-private-payload', str(result.error))
        return result

    async def test_init_info_publish_generated_facade_and_closure(self):
        g = self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer), g.registry()) as runtime:
                initial = await peer.frames.get()
                self.assertEqual(json.loads(initial[5:]), dict(versions=['1.0'], schema='002.5.0',
                    service_id='explicit-service-id', verbose=True))
                self.assertEqual(runtime.server_info, INFO)
                await g.api.function_codec.exchange_output.publish(runtime, g.sample())
                frame = await peer.frames.get()
                self.assertEqual(frame, 'PUB codec.out ' + g.envelope('OtherReport'))
                self.assertTrue(runtime.is_connected)
            self.assertFalse(runtime.is_connected)
            await runtime.close()
            with self.assertRaises(NotConnected):
                await g.api.function_codec.exchange_output.publish(runtime, g.sample())

    async def test_subscribe_sid_groups_interleaved_info_unsubscribe(self):
        g = self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer), g.registry()) as runtime:
                await peer.frames.get()
                seen1, seen2 = asyncio.Queue(), asyncio.Queue()
                first = await g.api.function_codec.exchange_input.subscribe(runtime, seen1.put_nowait)
                second = await runtime.subscribe(message_namespace=OAM,message_name='SelectedReport',
                    topic='other.topic',subscription_group='workers',handler=seen2.put_nowait)
                self.assertEqual(await peer.frames.get(), 'SUB sub-1 SelectedReport codec.in')
                self.assertEqual(await peer.frames.get(), 'SUB sub-2 SelectedReport other.topic workers')
                await peer.send('INFO\t' + json.dumps(INFO))
                await peer.send('MSG\t ' + second.id + '\t' + g.envelope())
                self.assertEqual(await asyncio.wait_for(seen2.get(),2),g.sample())
                self.assertTrue(seen1.empty())
                await peer.send('MSG ' + first.id + ' ' + g.envelope())
                self.assertEqual(await asyncio.wait_for(seen1.get(),2),g.sample())
                await first.unsubscribe()
                self.assertFalse(first.active)
                self.assertEqual(await peer.frames.get(), 'UNSUB sub-1')
                with self.assertRaises(NotConnected): await first.unsubscribe()
                await second.unsubscribe()

    async def test_handshake_failures_and_connect_timeout(self):
        for mode, error in [('no-protocol',ProtocolError),('wrong-order',ProtocolError),
                            ('bad-info',ProtocolError),('binary',ProtocolError),
                            ('bad-uuid',ProtocolError),('extra-ack',ProtocolError),
                            ('reject',ServerRejection),('silent',OperationTimeout)]:
            async with Peer(handshake=mode) as peer:
                runtime=LacalRuntime(self.config(peer,connect_timeout=.15), self.generated.registry())
                with self.subTest(mode=mode), self.assertRaises(error): await runtime.connect()
                self.assertFalse(runtime.is_connected)
                await runtime.close()

    async def test_server_rejection_does_not_poison_following_operations(self):
        g=self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                with self.assertRaises(ServerRejection) as raised:
                    await runtime.publish(message_namespace=OAM,message_name='SelectedReport',topic='forbidden',payload=g.sample())
                self.assertEqual(raised.exception.code,'Illegal-State')
                self.assertNotIn('secret',str(raised.exception))
                await g.api.function_codec.exchange_output.publish(runtime,g.sample())

    async def test_envelope_duplicates_lexemes_qnames_and_recovery(self):
        g=self.generated
        body=g.codec.encode_Payload(g.sample())
        cases=[('[]',ProtocolError),('{}',ProtocolError),
               ('{"OtherReport":'+body+'}',ProtocolError),
               ('{"SelectedReport":'+body+',"extra":{}}',ProtocolError),
               ('{"SelectedReport":'+body+',"\\u0053electedReport":'+body+'}',ProtocolError),
               ('{"SelectedReport":{"x":1,"\\u0078":2}}',ProtocolError),
               ('{"SelectedReport":{"unknown":[{"x":1,"\\u0078":2}]}}',ProtocolError),
               ('{"SelectedReport":'+body.replace('9007199254740993','9.007199254740993e15')+'}',CodecFailure),
               ('{"SelectedReport":'+body.replace('"State":"class"','"State":null')+'}',CodecFailure),
               ('{"SelectedReport":'+body[:-1]+',"$type":"Private"}}',CodecFailure),
               ('{"SelectedReport":'+body+'} trailing',ProtocolError)]
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                seen=asyncio.Queue()
                handle=await g.api.function_codec.exchange_input.subscribe(runtime,seen.put_nowait)
                for text, error in cases:
                    with self.subTest(text=text[:30]):
                        await peer.send('MSG '+handle.id+' '+text)
                        event=await self.event(runtime,error)
                        self.assertEqual(event.subscription_id,handle.id)
                        self.assertTrue(seen.empty())
                        await peer.send('MSG '+handle.id+' '+g.envelope())
                        self.assertEqual(await asyncio.wait_for(seen.get(),2),g.sample())
                await peer.send('MSG sub-999 '+g.envelope())
                await self.event(runtime,UnknownSubscription)
                huge=g.sample();object.__setattr__(huge.big,'value',10**5000)
                await peer.send('MSG '+handle.id+' '+g.envelope(value=huge))
                self.assertEqual((await asyncio.wait_for(seen.get(),2)).big.value,10**5000)
                # Parser depth exhaustion is a resource failure, not malformed
                # data or a successful handler call, and other traffic recovers.
                await peer.send('MSG '+handle.id+' {"SelectedReport":'+('['*10000)+'0'+(']'*10000)+'}')
                await self.event(runtime,ResourceLimit)
                await peer.send('MSG '+handle.id+' '+g.envelope())
                self.assertEqual(await asyncio.wait_for(seen.get(),2),g.sample())

    async def test_non_oam_clark_identity_mock_only(self):
        g=self.foreign
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                await peer.frames.get()
                seen=asyncio.Queue()
                handle=await g.api.function_codec.exchange_input.subscribe(runtime,seen.put_nowait)
                self.assertEqual(await peer.frames.get(),'SUB sub-1 {urn:python:codec}SelectedReport codec.in')
                await g.api.function_codec.exchange_output.publish(runtime,g.sample())
                self.assertEqual(await peer.frames.get(),'PUB codec.out '+g.envelope('OtherReport'))
                await peer.send('MSG '+handle.id+' '+g.envelope())
                self.assertEqual(await asyncio.wait_for(seen.get(),2),g.sample())

    async def test_handler_failure_and_async_isolation(self):
        g=self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                def broken(value): raise ValueError('secret-private-payload')
                first=await g.api.function_codec.exchange_input.subscribe(runtime,broken)
                seen=asyncio.Queue()
                async def async_handler(value): await seen.put(value)
                second=await g.api.function_codec.exchange_repeat.subscribe(runtime,async_handler)
                await peer.send('MSG '+first.id+' '+g.envelope())
                await self.event(runtime,HandlerFailure)
                await peer.send('MSG '+second.id+' '+g.envelope())
                self.assertEqual(await asyncio.wait_for(seen.get(),2),g.sample())

    async def test_input_validation_and_registry_before_frames(self):
        g=self.generated
        binding=CodecBinding(OAM,'SelectedReport',g.model.Payload,g.codec.encode_Payload,g.codec.decode_Payload,g.codec.CodecError)
        registry=CodecRegistry([binding,binding])
        with self.assertRaises(InvalidInput): CodecRegistry([binding,replace(binding,encode=lambda x:'{}')])
        with self.assertRaises(FrozenInstanceError): registry._bindings={}
        with self.assertRaises(TypeError): registry._bindings[('x','y')]=binding
        for changes in [dict(verbose=False),dict(service_id='bad id'),dict(max_subscriptions=0),dict(operation_timeout=float('nan')),
                        dict(url='ftp://localhost'),dict(url='wss://user:secret@localhost')]:
            args=dict(url='ws://localhost',service_id='svc',schema_version='002.5.0');args.update(changes)
            with self.assertRaises(InvalidInput): RuntimeConfig(**args)
        insecure=ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT);insecure.check_hostname=False;insecure.verify_mode=ssl.CERT_NONE
        with self.assertRaises(InvalidInput): RuntimeConfig('wss://localhost','svc','v',tls_context=insecure)
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                await peer.frames.get()
                for topic in ['bad topic','', 'λ', 'x\ny']:
                    with self.assertRaises(InvalidInput):
                        await runtime.publish(message_namespace=OAM,message_name='SelectedReport',topic=topic,payload=g.sample())
                with self.assertRaises(InvalidInput):
                    await runtime.publish(message_namespace=OAM,message_name='Missing',topic='ok',payload=g.sample())
                with self.assertRaises(InvalidInput):
                    await runtime.subscribe(message_namespace=OAM,message_name='SelectedReport',topic='ok',subscription_group='bad group',handler=lambda value:None)
                self.assertTrue(peer.frames.empty())

    async def test_operation_timeout_and_cancellation_are_terminal(self):
        g=self.generated
        for cancelled in [False,True]:
            async with Peer(ack=False) as peer:
                runtime=LacalRuntime(self.config(peer,operation_timeout=.15),g.registry())
                await runtime.connect();await peer.frames.get()
                operation=asyncio.create_task(g.api.function_codec.exchange_output.publish(runtime,g.sample()))
                await peer.frames.get()
                if cancelled: operation.cancel()
                with self.assertRaises(asyncio.CancelledError if cancelled else OperationTimeout): await operation
                self.assertFalse(runtime.is_connected)
                await runtime.close()

    async def test_connect_and_dispatch_cancellation_and_close(self):
        g=self.generated
        async with Peer(handshake='silent') as peer:
            runtime=LacalRuntime(self.config(peer),g.registry())
            task=asyncio.create_task(runtime.connect())
            await peer.frames.get();task.cancel()
            with self.assertRaises(asyncio.CancelledError): await task
            await runtime.close()
        async with Peer() as peer:
            runtime=LacalRuntime(self.config(peer),g.registry());await runtime.connect()
            entered=asyncio.Event();finished=asyncio.Event()
            async def handler(value):
                entered.set()
                try: await asyncio.Event().wait()
                finally: finished.set()
            handle=await g.api.function_codec.exchange_input.subscribe(runtime,handler)
            await peer.send('MSG '+handle.id+' '+g.envelope());await asyncio.wait_for(entered.wait(),2)
            await runtime.close();self.assertTrue(finished.is_set());self.assertFalse(handle.active)

    async def test_transport_binary_and_message_size_failures(self):
        g=self.generated
        for frame, expected in [(b'binary',ProtocolError),('x'*2000,ResourceLimit),(None,TransportFailure)]:
            async with Peer() as peer:
                runtime=LacalRuntime(self.config(peer,max_message_size=1024),g.registry());await runtime.connect()
                if frame is None: await peer.sockets[-1].close()
                else: await peer.send(frame)
                await self.event(runtime,expected)
                self.assertFalse(runtime.is_connected);await runtime.close()

    async def test_subscription_limit_and_bounded_queues(self):
        g=self.generated
        async with Peer() as peer:
            runtime=LacalRuntime(self.config(peer,max_subscriptions=1,subscription_queue_size=1,event_queue_size=1),g.registry())
            await runtime.connect();entered=asyncio.Event()
            async def stalled(value): entered.set();await asyncio.Event().wait()
            handle=await g.api.function_codec.exchange_input.subscribe(runtime,stalled)
            with self.assertRaises(ResourceLimit): await g.api.function_codec.exchange_repeat.subscribe(runtime,stalled)
            await peer.send('MSG '+handle.id+' '+g.envelope());await asyncio.wait_for(entered.wait(),2)
            for _ in range(3):
                try: await peer.send('MSG '+handle.id+' '+g.envelope())
                except websockets.exceptions.ConnectionClosed: break
            await self.event(runtime,ResourceLimit);await runtime.close()

    async def test_msg_before_ack_and_serialized_pending_operations(self):
        g=self.generated
        async with Peer(ack=False) as peer:
            async with LacalRuntime(self.config(peer,max_pending_operations=2),g.registry()) as runtime:
                await peer.frames.get();seen=asyncio.Queue()
                subscribing=asyncio.create_task(g.api.function_codec.exchange_input.subscribe(runtime,seen.put_nowait))
                self.assertTrue((await peer.frames.get()).startswith('SUB sub-1 '))
                await peer.send('MSG sub-1 '+g.envelope())
                self.assertEqual(await asyncio.wait_for(seen.get(),2),g.sample())
                await peer.send('INFO '+json.dumps(INFO));await peer.send('+OK')
                handle=await subscribing
                pub1=asyncio.create_task(g.api.function_codec.exchange_output.publish(runtime,g.sample()))
                await peer.frames.get()
                pub2=asyncio.create_task(g.api.function_codec.exchange_output.publish(runtime,g.sample()))
                await asyncio.sleep(.01);self.assertTrue(peer.frames.empty())
                with self.assertRaises(ResourceLimit):
                    await g.api.function_codec.exchange_output.publish(runtime,g.sample())
                pub2.cancel()
                with self.assertRaises(asyncio.CancelledError): await pub2
                self.assertTrue(runtime.is_connected)
                await peer.send('+OK');await pub1
                self.assertTrue(handle.active)

    async def test_close_while_connecting_pending_or_from_handler(self):
        g=self.generated
        async with Peer(handshake='silent') as peer:
            runtime=LacalRuntime(self.config(peer),g.registry())
            connecting=asyncio.create_task(runtime.connect());await peer.frames.get()
            await asyncio.wait_for(runtime.close(),2)
            with self.assertRaises(asyncio.CancelledError): await connecting
        async with Peer(ack=False) as peer:
            runtime=LacalRuntime(self.config(peer),g.registry());await runtime.connect();await peer.frames.get()
            pending=asyncio.create_task(g.api.function_codec.exchange_output.publish(runtime,g.sample()))
            await peer.frames.get();await runtime.close()
            with self.assertRaises(NotConnected): await pending
        async with Peer() as peer:
            runtime=LacalRuntime(self.config(peer),g.registry());await runtime.connect()
            finished=asyncio.Event()
            async def handler(value): await runtime.close();finished.set()
            handle=await g.api.function_codec.exchange_input.subscribe(runtime,handler)
            await peer.send('MSG '+handle.id+' '+g.envelope())
            await asyncio.wait_for(finished.wait(),2);await runtime.close()

    async def test_handler_timeout_event_bound_and_rejected_sub_cleanup(self):
        g=self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer,handler_timeout=.05,max_subscriptions=1),g.registry()) as runtime:
                with self.assertRaises(ServerRejection):
                    await runtime.subscribe(message_namespace=OAM,message_name='SelectedReport',topic='forbidden',subscription_group=None,handler=lambda value:None)
                async def stalled(value): await asyncio.Event().wait()
                handle=await g.api.function_codec.exchange_input.subscribe(runtime,stalled)
                await peer.send('MSG '+handle.id+' '+g.envelope());await self.event(runtime,ResourceLimit)
                self.assertTrue(handle.active)
        async with Peer() as peer:
            runtime=LacalRuntime(self.config(peer,event_queue_size=1),g.registry());await runtime.connect()
            await peer.send('MSG absent '+g.envelope());await peer.send('MSG absent '+g.envelope())
            async with asyncio.timeout(2):
                while runtime.is_connected: await asyncio.sleep(.01)
            event=await self.event(runtime,ResourceLimit)
            self.assertEqual(runtime._events.qsize(),0)
            self.assertIsInstance(runtime.terminal_error,ResourceLimit);await runtime.close()

    async def test_codec_failure_and_internal_error_distinction(self):
        g=self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                value=g.sample()
                object.__setattr__(value.signed,'value',True)
                with self.assertRaises(CodecFailure):
                    await g.api.function_codec.exchange_output.publish(runtime,value)
        def broken(text): raise RuntimeError('secret-private-payload')
        registry=CodecRegistry([CodecBinding(OAM,'SelectedReport',g.model.Payload,g.codec.encode_Payload,broken,g.codec.CodecError)])
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),registry) as runtime:
                handle=await g.api.function_codec.exchange_input.subscribe(runtime,lambda value:None)
                await peer.send('MSG '+handle.id+' '+g.envelope())
                await self.event(runtime,CodecInternalFailure)

    async def test_unsolicited_error_bad_protocol_and_unsubscribe_cancellation(self):
        g=self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer),g.registry()) as runtime:
                await peer.send('-ERR Illegal-State secret-private-payload')
                await self.event(runtime,ServerRejection)
                self.assertTrue(runtime.is_connected)
                await peer.send('+OK extra')
                await self.event(runtime,ProtocolError)
        async with Peer() as peer:
            runtime=LacalRuntime(self.config(peer),g.registry());await runtime.connect()
            handle=await g.api.function_codec.exchange_input.subscribe(runtime,lambda value:None)
            peer.ack=False
            while not peer.frames.empty(): peer.frames.get_nowait()
            operation=asyncio.create_task(handle.unsubscribe());self.assertEqual(await peer.frames.get(),'UNSUB sub-1')
            operation.cancel()
            with self.assertRaises(asyncio.CancelledError): await operation
            self.assertFalse(handle.active);self.assertFalse(runtime.is_connected);await runtime.close()

    async def test_handler_cancellation_and_outgoing_limit(self):
        g=self.generated
        async with Peer() as peer:
            async with LacalRuntime(self.config(peer,max_message_size=1024),g.registry()) as runtime:
                await peer.frames.get()
                value=g.sample();object.__setattr__(value,'note','x'*2048)
                with self.assertRaises(ResourceLimit): await g.api.function_codec.exchange_output.publish(runtime,value)
                self.assertTrue(peer.frames.empty());self.assertTrue(runtime.is_connected)
                def cancelled(value): raise asyncio.CancelledError()
                handle=await g.api.function_codec.exchange_input.subscribe(runtime,cancelled)
                await peer.send('MSG '+handle.id+' '+g.envelope());await self.event(runtime,HandlerFailure)
                self.assertTrue(handle.active)

    async def test_wss_trust_and_hostname_verification(self):
        # Actual TLS sockets: default trust rejects a self-signed peer, explicit
        # trusted CA succeeds, and hostname mismatch still rejects.
        with TemporaryDirectory(prefix='py005-tls-') as temp:
            # TLS negative controls intentionally reset handshakes. Keep other
            # asynchronous errors visible rather than suppressing all diagnostics.
            loop=asyncio.get_running_loop();previous=loop.get_exception_handler()
            def expected_reset(loop,context):
                if isinstance(context.get('exception'),ConnectionResetError): return
                if previous: previous(loop,context)
                else: loop.default_exception_handler(context)
            loop.set_exception_handler(expected_reset)
            self.addCleanup(loop.set_exception_handler,previous)
            path=Path(temp);key=path/'key.pem';cert=path/'cert.pem'
            subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes',
                '-keyout',str(key),'-out',str(cert),'-days','1','-subj','/CN=localhost',
                '-addext','subjectAltName=DNS:localhost'],check=True,capture_output=True)
            server_context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);server_context.load_cert_chain(cert,key)
            peer=Peer()
            async with serve(peer.handle,'127.0.0.1',0,subprotocols=['owp'],ssl=server_context) as server:
                port=server.sockets[0].getsockname()[1]
                url=f'wss://localhost:{port}'
                runtime=LacalRuntime(RuntimeConfig(url,'svc','002.5.0'),self.generated.registry())
                with self.assertRaises(TransportFailure): await runtime.connect()
                await runtime.close()
                trusted=ssl.create_default_context(cafile=str(cert))
                async with LacalRuntime(RuntimeConfig(url,'svc','002.5.0',tls_context=trusted),self.generated.registry()) as runtime:
                    self.assertTrue(runtime.is_connected)
                runtime=LacalRuntime(RuntimeConfig(f'wss://127.0.0.1:{port}','svc','002.5.0',tls_context=trusted),self.generated.registry())
                with self.assertRaises(TransportFailure): await runtime.connect()
                await runtime.close()


if __name__ == '__main__': unittest.main()