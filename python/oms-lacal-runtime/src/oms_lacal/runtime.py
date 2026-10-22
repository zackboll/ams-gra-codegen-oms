"""One receive owner, serialized verbose OWP operations, bounded typed dispatch."""
import asyncio
from dataclasses import dataclass, field
import inspect
import json
import math
import re
import ssl
from types import MappingProxyType
from typing import Awaitable, Callable, Generic, TypeVar
from urllib.parse import urlsplit

from websockets.asyncio.client import connect as websocket_connect
from websockets.exceptions import ConnectionClosed, InvalidHandshake

from .wire import (
    CodecFailure, CodecInternalFailure, HandlerFailure, InvalidInput, LacalError,
    NotConnected, OperationTimeout, ProtocolError, ResourceLimit, ServerRejection,
    TransportFailure, UnknownSubscription, global_name, identifier, info,
    server_error, unwrap,
)

P = TypeVar('P')


@dataclass(frozen=True, slots=True)
class CodecBinding(Generic[P]):
    message_namespace: str
    message_name: str
    payload_type: type[P]
    encode: Callable[[P], str]
    decode: Callable[[str], P]
    error_type: type[Exception]

    def __post_init__(self):
        global_name(self.message_namespace, self.message_name)
        if (not isinstance(self.payload_type, type) or not callable(self.encode) or
                not callable(self.decode) or not isinstance(self.error_type, type) or
                not issubclass(self.error_type, Exception) or
                self.error_type in (Exception, ValueError, TypeError)):
            raise InvalidInput('invalid explicit codec binding')


@dataclass(frozen=True, slots=True, init=False)
class CodecRegistry:
    _bindings: object

    def __init__(self, bindings):
        selected = {}
        for binding in bindings:
            if type(binding) is not CodecBinding:
                raise InvalidInput('registry entries must be CodecBinding')
            key = (binding.message_namespace, binding.message_name)
            old = selected.get(key)
            if old is not None and (old.payload_type is not binding.payload_type or
                    old.encode is not binding.encode or old.decode is not binding.decode or
                    old.error_type is not binding.error_type):
                raise InvalidInput('conflicting message codec registration')
            selected[key] = binding
        object.__setattr__(self, '_bindings', MappingProxyType(selected))

    def lookup(self, namespace: str, local: str) -> CodecBinding:
        global_name(namespace, local)
        try:
            return self._bindings[(namespace, local)]
        except KeyError:
            raise InvalidInput('message QName has no registered codec') from None


@dataclass(frozen=True, slots=True)
class RuntimeConfig:
    url: str
    service_id: str
    schema_version: str
    versions: tuple[str, ...] = ('1.0',)
    verbose: bool = True
    connect_timeout: float = 10.0
    operation_timeout: float = 10.0
    handler_timeout: float = 10.0
    close_timeout: float = 2.0
    max_message_size: int = 1024 * 1024
    websocket_queue_size: int = 16
    subscription_queue_size: int = 16
    event_queue_size: int = 64
    max_subscriptions: int = 64
    max_pending_operations: int = 64
    tls_context: ssl.SSLContext | None = field(default=None, repr=False, compare=False)

    def __post_init__(self):
        try:
            if type(self.url) is not str:
                raise ValueError()
            url = urlsplit(self.url)
            valid_url = url.scheme in ('ws', 'wss') and url.hostname and url.port != 0
        except (TypeError, ValueError):
            valid_url = False
        if not valid_url or url.username is not None or url.password is not None or url.fragment:
            raise InvalidInput('expected ws/wss URL without credentials or fragment')
        identifier(self.service_id, 'service')
        if type(self.schema_version) is not str or not self.schema_version or any(ch.isspace() for ch in self.schema_version):
            raise InvalidInput('explicit schema version required')
        if self.versions != ('1.0',) or type(self.versions) is not tuple or self.verbose is not True:
            raise InvalidInput('only OWP 1.0 verbose mode is qualified')
        for name in ('connect_timeout', 'operation_timeout', 'handler_timeout', 'close_timeout'):
            value = getattr(self, name)
            if type(value) not in (float, int) or not math.isfinite(value) or value <= 0:
                raise InvalidInput('timeouts must be positive finite numbers')
        for name in ('max_message_size', 'websocket_queue_size', 'subscription_queue_size', 'event_queue_size', 'max_subscriptions', 'max_pending_operations'):
            if type(getattr(self, name)) is not int or getattr(self, name) <= 0:
                raise InvalidInput('resource bounds must be positive integers')
        if self.tls_context is not None:
            if (url.scheme != 'wss' or not isinstance(self.tls_context, ssl.SSLContext) or
                    not self.tls_context.check_hostname or self.tls_context.verify_mode != ssl.CERT_REQUIRED):
                raise InvalidInput('WSS TLS must verify hostname and certificates')


@dataclass(frozen=True, slots=True)
class RuntimeEvent:
    error: LacalError
    subscription_id: str | None = None


class SubscriptionHandle(Generic[P]):
    """Connection-owned SID; explicit unsubscribe or runtime close invalidates it."""
    def __init__(self, runtime, sid, binding, handler):
        self._runtime = runtime
        self._sid = sid
        self._binding = binding
        self._handler = handler
        self._queue = asyncio.Queue(runtime.config.subscription_queue_size)
        self._task = None
        self._active = True

    @property
    def id(self) -> str:
        return self._sid

    @property
    def active(self) -> bool:
        return self._active and self._runtime.is_connected

    async def unsubscribe(self) -> None:
        await self._runtime.unsubscribe(self)

    async def __aenter__(self):
        if not self.active:
            raise NotConnected('subscription is inactive')
        return self

    async def __aexit__(self, *exc):
        if self.active:
            await self.unsubscribe()


class LacalRuntime:
    """Single-use async context manager implementing generated adapter methods.

    Adapter calls return coroutines; explicitly await generated endpoint calls.
    Handlers may return awaitables. Synchronous handlers must not block the loop.
    No reconnect, automatic resubscribe, replay, or transport retry is performed.
    """
    def __init__(self, config: RuntimeConfig, registry: CodecRegistry):
        if type(config) is not RuntimeConfig or type(registry) is not CodecRegistry:
            raise InvalidInput('explicit RuntimeConfig and CodecRegistry required')
        self.config = config
        self.registry = registry
        self._state = 'new'
        self._ws = None
        self._receiver = None
        self._closing_task = None
        self._connect_task = None
        self._operation_lock = asyncio.Lock()
        self._waiting_operations = 0
        self._pending = None
        self._subscriptions = {}
        self._next_sid = 0
        self._events = asyncio.Queue(config.event_queue_size)
        self._terminal = None
        self._info = None

    @property
    def is_connected(self) -> bool:
        return self._state == 'active'

    @property
    def server_info(self):
        # Return a copy; caller cannot mutate handshake state.
        return None if self._info is None else json.loads(json.dumps(self._info))

    @property
    def terminal_error(self):
        return self._terminal

    async def next_event(self) -> RuntimeEvent:
        return await self._events.get()

    def _event(self, error, sid=None):
        if self._events.full():
            self._events.get_nowait()
            error = ResourceLimit('runtime event queue exhausted')
            self._fail(error)
        self._events.put_nowait(RuntimeEvent(error, sid))

    def _fail(self, error):
        if self._terminal is None:
            self._terminal = error
        if self._state not in ('closing', 'closed'):
            self._state = 'failed'
        if self._pending is not None and not self._pending.done():
            self._pending.set_exception(error)
        for handle in self._subscriptions.values():
            handle._active = False
        if self._ws is not None:
            self._ws.transport.abort()

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, *exc):
        await self.close()

    async def connect(self) -> None:
        if self._state != 'new':
            raise InvalidInput('runtime is single-use')
        self._state = 'connecting'
        self._connect_task = asyncio.current_task()
        try:
            async with asyncio.timeout(self.config.connect_timeout):
                options = dict(subprotocols=['owp'], proxy=None,
                               max_size=self.config.max_message_size,
                               max_queue=self.config.websocket_queue_size,
                               open_timeout=self.config.connect_timeout,
                               close_timeout=self.config.close_timeout)
                if urlsplit(self.config.url).scheme == 'wss':
                    context = self.config.tls_context or ssl.create_default_context()
                    # SSLContext is mutable even inside frozen RuntimeConfig.
                    if not context.check_hostname or context.verify_mode != ssl.CERT_REQUIRED:
                        raise InvalidInput('WSS TLS verification was disabled')
                    options['ssl'] = context
                self._ws = await websocket_connect(self.config.url, **options)
                if self._ws.subprotocol != 'owp':
                    raise ProtocolError('OWP subprotocol was not negotiated')
                request = dict(versions=list(self.config.versions), schema=self.config.schema_version,
                               service_id=self.config.service_id, verbose=True)
                await self._ws.send('INIT ' + json.dumps(request, separators=(',', ':')))
                ack = await self._handshake_frame()
                if re.match(r'-ERR[ \t]', ack):
                    raise server_error(ack)
                if ack != '+OK':
                    raise ProtocolError('INIT requires +OK before INFO')
                frame = await self._handshake_frame()
                if re.match(r'-ERR[ \t]', frame):
                    raise server_error(frame)
                parts = re.split('[ \t]+', frame, maxsplit=1)
                if len(parts) != 2 or parts[0] != 'INFO':
                    raise ProtocolError('INIT requires INFO')
                self._accept_info(parts[1])
                self._state = 'active'
                self._receiver = asyncio.create_task(self._receive(), name='oms-lacal-receive')
        except asyncio.CancelledError:
            await self.close()
            raise
        except TimeoutError:
            await self.close()
            raise OperationTimeout('connection handshake timed out') from None
        except InvalidHandshake:
            await self.close()
            raise ProtocolError('WebSocket/OWP handshake rejected') from None
        except ConnectionClosed as error:
            await self.close()
            if error.sent is not None and error.sent.code == 1009:
                raise ResourceLimit('handshake WebSocket message size limit') from None
            raise TransportFailure('connection failed') from None
        except (MemoryError, RecursionError):
            await self.close()
            raise ResourceLimit('handshake parsing resource limit') from None
        except OSError:
            await self.close()
            raise TransportFailure('connection failed') from None
        except BaseException:
            await self.close()
            raise
        finally:
            self._connect_task = None

    async def _handshake_frame(self):
        frame = await self._ws.recv()
        if type(frame) is not str:
            raise ProtocolError('OWP requires text frames')
        return frame

    def _accept_info(self, text):
        value = info(text)
        if value['version'] not in self.config.versions:
            raise ProtocolError('INFO selected an unsupported OWP version')
        if self._info is not None and value['uuids'] != self._info['uuids']:
            raise ProtocolError('INFO changed initialized identity')
        self._info = value

    def _connected(self):
        if not self.is_connected:
            raise NotConnected('runtime is not connected')

    async def _operation(self, frame):
        self._connected()
        if len(frame.encode('utf-8')) > self.config.max_message_size:
            raise ResourceLimit('outgoing message size limit')
        if self._waiting_operations >= self.config.max_pending_operations:
            raise ResourceLimit('pending operation bound exhausted')
        self._waiting_operations += 1
        # Lock acquisition is bounded too, but a cancellation before acquiring
        # cannot desynchronize another operation's outstanding status.
        acquired = False
        started = False
        try:
            async with asyncio.timeout(self.config.operation_timeout):
                await self._operation_lock.acquire()
                acquired = True
                self._connected()
                self._pending = asyncio.get_running_loop().create_future()
                started = True
                await self._ws.send(frame)
                await self._pending
        except (TimeoutError, asyncio.CancelledError) as error:
            if started:
                failure = OperationTimeout('operation cancelled or timed out; acknowledgement is ambiguous')
                self._fail(failure)
                await self.close()
            if isinstance(error, asyncio.CancelledError):
                raise
            raise OperationTimeout('operation timed out') from None
        except (OSError, ConnectionClosed):
            self._fail(TransportFailure('operation transport failed'))
            await self.close()
            raise TransportFailure('operation transport failed') from None
        finally:
            self._waiting_operations -= 1
            if acquired:
                if self._pending is not None and not self._pending.done():
                    self._pending.cancel()
                # Retrieve failure even if send failed before awaiting it.
                if self._pending is not None and self._pending.done() and not self._pending.cancelled():
                    self._pending.exception()
                self._pending = None
                self._operation_lock.release()

    async def publish(self, *, message_namespace: str, message_name: str,
                      topic: str, payload: P) -> None:
        identifier(topic, 'topic')
        binding = self.registry.lookup(message_namespace, message_name)
        self._connected()
        if type(payload) is not binding.payload_type:
            raise InvalidInput('payload has incorrect registered type')
        try:
            body = binding.encode(payload)
        except binding.error_type:
            raise CodecFailure('payload encoder rejected value') from None
        except (RecursionError, MemoryError):
            raise ResourceLimit('payload encoder resource limit') from None
        if type(body) is not str:
            raise InvalidInput('registered encoder must return JSON text')
        name = global_name(message_namespace, message_name)
        envelope = '{' + json.dumps(name, ensure_ascii=False) + ':' + body + '}'
        await self._operation('PUB ' + topic + ' ' + envelope)

    async def subscribe(self, *, message_namespace: str, message_name: str, topic: str,
                        subscription_group: str | None,
                        handler: Callable[[P], None | Awaitable[None]]) -> SubscriptionHandle[P]:
        identifier(topic, 'topic')
        if subscription_group is not None:
            identifier(subscription_group, 'subscription group')
        binding = self.registry.lookup(message_namespace, message_name)
        self._connected()
        if not callable(handler):
            raise InvalidInput('subscription handler must be callable')
        if len(self._subscriptions) >= self.config.max_subscriptions:
            raise ResourceLimit('maximum subscriptions reached')
        self._next_sid += 1
        sid = identifier('sub-' + str(self._next_sid), 'subscription')
        handle = SubscriptionHandle(self, sid, binding, handler)
        self._subscriptions[sid] = handle
        handle._task = asyncio.create_task(self._dispatch(handle), name='oms-lacal-dispatch-' + sid)
        frame = 'SUB ' + sid + ' ' + global_name(message_namespace, message_name) + ' ' + topic
        if subscription_group is not None:
            frame += ' ' + subscription_group
        try:
            await self._operation(frame)
        except BaseException:
            await self._retire(handle)
            raise
        return handle

    async def _retire(self, handle):
        handle._active = False
        self._subscriptions.pop(handle.id, None)
        task = handle._task
        if task is not None and task is not asyncio.current_task():
            task.cancel()
            await asyncio.gather(task, return_exceptions=True)
        while not handle._queue.empty():
            handle._queue.get_nowait()

    async def unsubscribe(self, handle: SubscriptionHandle) -> None:
        if type(handle) is not SubscriptionHandle or handle._runtime is not self:
            raise InvalidInput('subscription belongs to another runtime')
        if not handle.active or self._subscriptions.get(handle.id) is not handle:
            raise NotConnected('subscription is inactive')
        # Invalidate locally even when server rejects or caller cancels UNSUB.
        await self._retire(handle)
        try:
            await self._operation('UNSUB ' + handle.id)
        except BaseException:
            # A locally retired but remotely live SID must not be left as a
            # ghost subscription after rejected/cancelled UNSUB (even if the
            # cancellation happened while waiting for another operation's lock).
            self._fail(ProtocolError('unsubscribe did not complete'))
            await self.close()
            raise

    async def _receive(self):
        try:
            async for frame in self._ws:
                if type(frame) is not str:
                    raise ProtocolError('OWP requires text frames')
                fields = re.split('[ \t]+', frame, maxsplit=2)
                if frame.rstrip(' \t') == '+OK' or fields[0] == '-ERR':
                    error = None if fields[0] == '+OK' else server_error(frame)
                    if self._pending is None or self._pending.done():
                        if error is None:
                            raise ProtocolError('unsolicited acknowledgement')
                        self._event(error)
                    elif error is None:
                        self._pending.set_result(None)
                    else:
                        self._pending.set_exception(error)
                elif fields[0] == 'INFO':
                    parts = re.split('[ \t]+', frame, maxsplit=1)
                    if len(parts) != 2:
                        raise ProtocolError('missing INFO payload')
                    self._accept_info(parts[1])
                elif fields[0] == 'MSG':
                    if len(fields) != 3 or not fields[2]:
                        raise ProtocolError('malformed MSG frame')
                    try:
                        identifier(fields[1], 'subscription')
                    except InvalidInput:
                        raise ProtocolError('invalid MSG subscription identifier') from None
                    handle = self._subscriptions.get(fields[1])
                    if handle is None or not handle._active:
                        self._event(UnknownSubscription('unknown subscription ID'), fields[1])
                    else:
                        try:
                            handle._queue.put_nowait(fields[2])
                        except asyncio.QueueFull:
                            raise ResourceLimit('subscription message queue exhausted') from None
                else:
                    raise ProtocolError('unsupported OWP server frame')
            if self._state == 'active':
                raise TransportFailure('connection closed')
        except asyncio.CancelledError:
            raise
        except ConnectionClosed as error:
            # websockets sends code 1009 when the configured receive bound trips.
            if error.sent is not None and error.sent.code == 1009:
                failure = ResourceLimit('incoming WebSocket message size limit')
            else:
                failure = TransportFailure('connection lost')
            self._event(failure)
            self._fail(failure)
        except (ProtocolError, ResourceLimit, TransportFailure) as error:
            self._event(error)
            self._fail(error)
        except (MemoryError, RecursionError):
            failure = ResourceLimit('receive resource limit')
            self._event(failure)
            self._fail(failure)
        finally:
            if self._state != 'closing':
                for handle in list(self._subscriptions.values()):
                    await self._retire(handle)

    async def _dispatch(self, handle):
        binding = handle._binding
        while handle._active:
            text = await handle._queue.get()
            try:
                body = unwrap(global_name(binding.message_namespace, binding.message_name), text)
                value = binding.decode(body)
                if type(value) is not binding.payload_type:
                    raise CodecFailure('decoder returned incorrect registered type')
            except binding.error_type:
                self._event(CodecFailure('payload decoder rejected message'), handle.id)
                continue
            except (ProtocolError, CodecFailure, ResourceLimit) as error:
                self._event(error, handle.id)
                continue
            except (MemoryError, RecursionError):
                self._event(ResourceLimit('payload decoder resource limit'), handle.id)
                continue
            except Exception:
                self._event(CodecInternalFailure('registered decoder failed internally'), handle.id)
                continue
            try:
                async with asyncio.timeout(self.config.handler_timeout):
                    result = handle._handler(value)
                    if inspect.isawaitable(result):
                        await result
            except asyncio.CancelledError:
                if not handle._active or self._state != 'active':
                    raise
                self._event(HandlerFailure('handler cancelled delivery'), handle.id)
            except TimeoutError:
                self._event(ResourceLimit('handler execution timeout'), handle.id)
            except Exception:
                self._event(HandlerFailure('subscription handler failed'), handle.id)

    async def close(self) -> None:
        """Idempotent and cancellation-safe; joins all runtime-owned tasks."""
        if self._closing_task is None:
            caller = asyncio.current_task()
            self._state = 'closing'
            for handle in self._subscriptions.values():
                handle._active = False
            self._closing_task = asyncio.create_task(self._close(caller), name='oms-lacal-close')
        try:
            await asyncio.shield(self._closing_task)
        except asyncio.CancelledError:
            await self._closing_task
            raise

    async def _close(self, caller):
        if self._pending is not None and not self._pending.done():
            self._pending.set_exception(NotConnected('runtime closed'))
        if self._connect_task is not None and self._connect_task is not caller:
            # Close an in-progress connect without awaiting its cleanup cycle.
            self._connect_task.cancel()
        tasks = [handle._task for handle in self._subscriptions.values() if handle._task is not None]
        if self._receiver is not None:
            tasks.append(self._receiver)
        tasks = [task for task in tasks if task is not caller]
        for task in tasks:
            task.cancel()
        if self._ws is not None:
            try:
                async with asyncio.timeout(self.config.close_timeout):
                    await self._ws.close()
            except (TimeoutError, ConnectionClosed, OSError):
                self._ws.transport.abort()
        await asyncio.gather(*tasks, return_exceptions=True)
        self._subscriptions.clear()
        self._state = 'closed'