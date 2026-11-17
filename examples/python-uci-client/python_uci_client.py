#!/usr/bin/env python3
"""Standalone Python UCI client: typed RDMA_InitializeSetup (and optional Log)
reception from Sleet.

Uses only the existing Python OMS LA-CAL runtime (`oms_lacal`) and the actual
generated `model.py`, `service_api.py` and `service_codec.py` produced by
generate_bindings.py. It neither frames OWP nor decodes UCI JSON itself.
With --with-log, one connection carries two generated subscriptions.
"""
import asyncio
from dataclasses import dataclass, field
import hashlib
import inspect
import itertools
import json
from pathlib import Path
import signal
import ssl
import sys
from types import ModuleType
from urllib.parse import urlsplit

from oms_lacal import (
    CodecBinding, CodecFailure, CodecInternalFailure, CodecRegistry, InvalidInput,
    LacalError, LacalRuntime, ProtocolError, RuntimeConfig, ServerRejection,
    UnknownSubscription,
)

PROGRAM = 'python-uci-client'
APP = Path(__file__).resolve().parent
MANIFEST = 'python-uci-client-build.json'
GENERATED = ('model.py', 'service_codec.py', 'service_api.py')  # execution order
SCHEMA_VERSIONS = {'2.5': '002.5.0', '2.6': '002.6.0'}
OAM = 'https://www.vdl.afrl.af.mil/programs/oam'
MESSAGE = 'RDMA_InitializeSetup'
PAYLOAD = 'RDMA_InitializeSetupMT'
LOG_MESSAGE = 'Log'
LOG_PAYLOAD = 'LogMT'
# (wire global-element name, generated payload class, generated contract topic).
# The global element is the OWP/SUB message name; the payload class is only
# the generated Python type. They are never derived from each other.
RDMA_FAMILY = (MESSAGE, PAYLOAD, 'uci.rdma')
LOG_FAMILY = (LOG_MESSAGE, LOG_PAYLOAD, 'uci.log')
# Manifest format -> selected message families. Format 1 is the PY-OMS-020
# RDMA-only manifest; format 2 is the --with-log combined manifest.
MANIFEST_FAMILIES = {1: (RDMA_FAMILY,), 2: (RDMA_FAMILY, LOG_FAMILY)}

EXIT_OK = 0
EXIT_INTERNAL = 1
EXIT_USAGE = 2
EXIT_BINDINGS = 3
EXIT_CONNECT = 4
EXIT_REJECTED = 5
EXIT_SESSION = 6
EXIT_DEGRADED = 7

DEFAULT_DURATION = 10
MAX_DURATION = 86400
MAX_MESSAGES = 1_000_000
DEFAULT_BUFFER = 16
MAX_BUFFER = 4096
DRAIN_TIMEOUT = 5.0
MAX_DISPLAYED_ENDPOINTS = 8

HELP = f"""usage: {PROGRAM} --url ws[s]://HOST[:PORT][/PATH] --service-id ID
                         --schema-release {{2.5|2.6}} --bindings DIR [OPTIONS]

Receive typed UCI RDMA_InitializeSetup messages from a Sleet OMS endpoint
(and, with --with-log, typed UCI Log messages on the same connection).

Required:
  --url URL               Sleet endpoint: ws:// or certificate-verified wss://
  --service-id ID         OWP service identity authorized by the broker
  --schema-release R      UCI release of the generated bindings: 2.5 or 2.6
  --bindings DIR          Directory produced by generate_bindings.py for R

Options:
  --duration-seconds N    Receive for N seconds after SUB is acknowledged,
                          1..{MAX_DURATION} (default: {DEFAULT_DURATION})
  --max-messages N        Stop early after N typed messages, 1..{MAX_MESSAGES}
  --ca-file PEM           Additional trusted CA certificates for wss:// only.
                          Certificate and hostname verification stay enabled.
  --with-log              Also receive typed UCI Log (LogMT) on topic uci.log
                          over the same connection. Requires bindings made by
                          generate_bindings.py --with-log. --max-messages then
                          counts both message families together.
  --help                  Show this help and exit

Exit status:
  0    completed (duration or --max-messages); UNSUB and close succeeded
  2    malformed or contradictory arguments; no connection was opened
  3    missing, modified or release-mismatched generated bindings
  4    connection failed (transport, TLS verification, WebSocket/OWP negotiation)
  5    the server rejected INIT or SUB
  6    the session failed after subscribing (connection lost, protocol, limits)
  7    completed, but payloads were rejected or the callback failed
  130  SIGINT / 143 SIGTERM: stopped, then UNSUB and close succeeded
"""


class UsageError(Exception):
    pass


class BindingsError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class Options:
    url: str
    service_id: str
    schema_release: str
    bindings: Path
    duration: int = DEFAULT_DURATION
    max_messages: int | None = None
    ca_file: Path | None = None
    with_log: bool = False


def _bounded_integer(flag, text, upper):
    # Validate the spelling and length before int(): converting an arbitrarily
    # long digit string is costly and, past sys.get_int_max_str_digits(),
    # raises ValueError instead of producing a usage error.
    if not (text.isascii() and text.isdigit()) or (len(text) > 1 and text[0] == '0'):
        raise UsageError(f'{flag} expects a canonical decimal integer')
    if len(text) > len(str(upper)):
        raise UsageError(f'{flag} must be between 1 and {upper}')
    value = int(text)
    if not 1 <= value <= upper:
        raise UsageError(f'{flag} must be between 1 and {upper}')
    return value


def parse_arguments(argv):
    """Strict parser. Returns None for --help; raises UsageError otherwise.

    Every argument, including the URL, service identifier, TLS trust file and
    their combination, is validated here, before any network activity.
    """
    argv = list(argv)
    if any(type(item) is not str for item in argv):
        raise UsageError('arguments must be strings')
    if argv == ['--help']:
        return None
    flags = ('--url', '--service-id', '--schema-release', '--bindings',
             '--duration-seconds', '--max-messages', '--ca-file')
    switches = ('--with-log',)  # presence-only; never consumes a value
    values = {}
    items = iter(argv)
    for flag in items:
        if flag == '--help':
            raise UsageError('--help cannot be combined with other arguments')
        if flag not in flags + switches:
            raise UsageError(f'unknown argument {flag!r}')
        if flag in values:
            raise UsageError(f'duplicate argument {flag}')
        if flag in switches:
            values[flag] = True
            continue
        value = next(items, None)
        if value is None or not value or value.startswith('--'):
            raise UsageError(f'{flag} requires a value')
        values[flag] = value
    for flag in ('--url', '--service-id', '--schema-release', '--bindings'):
        if flag not in values:
            raise UsageError(f'missing required argument {flag}')
    if values['--schema-release'] not in SCHEMA_VERSIONS:
        raise UsageError('--schema-release must be 2.5 or 2.6')
    options = Options(
        url=values['--url'], service_id=values['--service-id'],
        schema_release=values['--schema-release'], bindings=Path(values['--bindings']),
        duration=(_bounded_integer('--duration-seconds', values['--duration-seconds'], MAX_DURATION)
                  if '--duration-seconds' in values else DEFAULT_DURATION),
        max_messages=(_bounded_integer('--max-messages', values['--max-messages'], MAX_MESSAGES)
                      if '--max-messages' in values else None),
        ca_file=Path(values['--ca-file']) if '--ca-file' in values else None,
        with_log='--with-log' in values)
    runtime_config(options)
    return options


def _is_wss(url):
    return url[:6].lower() == 'wss://'


def tls_context(ca_file=None):
    """System trust, optionally EXTENDED by a CA file. Verification is never relaxed."""
    context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH)
    if ca_file is not None:
        try:
            context.load_verify_locations(cafile=str(ca_file))
        except (OSError, ssl.SSLError, ValueError):
            raise UsageError(f'--ca-file {str(ca_file)!r} is not a readable PEM CA file') from None
    if context.verify_mode != ssl.CERT_REQUIRED or not context.check_hostname:
        raise UsageError('certificate and hostname verification must remain enabled')
    return context


def runtime_config(options):
    """The runtime's own RuntimeConfig validation owns URL/identifier policy."""
    wss = _is_wss(options.url)
    if options.ca_file is not None and not wss:
        raise UsageError('--ca-file requires a wss:// URL')
    context = tls_context(options.ca_file) if wss else None
    try:
        return RuntimeConfig(options.url, options.service_id,
                             SCHEMA_VERSIONS[options.schema_release], tls_context=context)
    except InvalidInput as error:
        raise UsageError(f'invalid --url or --service-id ({error})') from None


@dataclass(frozen=True, slots=True)
class Family:
    """One generated input exchange: wire message, payload type and topic."""
    message: str          # OWP global element, e.g. 'Log'
    payload_name: str     # generated payload class name, e.g. 'LogMT'
    topic: str            # generated contract topic, e.g. 'uci.log'
    receive: type         # generated function_<id>.exchange_receive endpoint
    payload_type: type    # the actual generated model class


@dataclass(frozen=True, slots=True)
class Bindings:
    """Actual generated modules plus the generated receive endpoints.

    `receive`/`payload_type` remain the RDMA endpoint and type (PY-OMS-020).
    `families` lists every selected message in subscription order.
    """
    release: str
    schema_version: str
    directory: Path
    model: ModuleType
    codec: ModuleType
    api: ModuleType
    receive: type
    payload_type: type
    registry: CodecRegistry = field(repr=False)
    families: tuple = ()
    with_log: bool = False


_PACKAGES = itertools.count()


def _execute_package(directory, sources):
    """Execute the VERIFIED bytes as one private package; no path search/TOCTOU."""
    name = f'_python_uci_client_bindings_{next(_PACKAGES)}'
    package = ModuleType(name)
    package.__path__ = []
    package.__package__ = name
    loaded = {name: package}
    sys.modules[name] = package
    try:
        for filename in GENERATED:
            local = filename.removesuffix('.py')
            module = ModuleType(name + '.' + local)
            module.__file__ = str(directory / filename)
            module.__package__ = name
            sys.modules[module.__name__] = loaded[module.__name__] = module
            exec(compile(sources[filename], module.__file__, 'exec'), module.__dict__)
            setattr(package, local, module)
    except Exception as error:
        for key in loaded:
            sys.modules.pop(key, None)
        raise BindingsError(f'generated bindings failed to load ({type(error).__name__})') from None
    return package


def _described_families(manifest, families):
    """Does the manifest name exactly these selected messages (and nothing else)?"""
    if manifest['format'] == 1:  # PY-OMS-020 RDMA-only manifest, unchanged
        return (manifest.get('message') == MESSAGE and manifest.get('payload_type') == PAYLOAD
                and 'messages' not in manifest and 'with_log' not in manifest)
    return (manifest.get('messages') == [dict(message=m, payload_type=p, topic=t)
                                         for m, p, t in families]
            and manifest.get('with_log') is True
            and 'message' not in manifest and 'payload_type' not in manifest)


def load_bindings(directory, release, with_log=False):
    """Load generate_bindings.py output; refuse modified, wrong-release or wrong-mode bindings.

    with_log=False needs a PY-OMS-020 RDMA-only manifest (format 1);
    with_log=True needs a combined RDMA + Log manifest (format 2).
    """
    directory = Path(directory)
    try:
        manifest = json.loads((directory / MANIFEST).read_text(encoding='utf-8'))
    except FileNotFoundError:
        raise BindingsError(f'{directory} has no {MANIFEST}; run generate_bindings.py first') from None
    except (OSError, UnicodeError, ValueError):
        raise BindingsError(f'unreadable {MANIFEST} in {directory}') from None
    if (type(manifest) is not dict or type(manifest.get('format')) is not int or
            manifest['format'] not in MANIFEST_FAMILIES or
            manifest.get('application') != PROGRAM or type(manifest.get('files')) is not dict or
            sorted(manifest['files']) != sorted(GENERATED)):
        raise BindingsError(f'{MANIFEST} is not a python-uci-client build manifest')
    if manifest.get('release') != release:
        raise BindingsError(f'bindings in {directory} were generated for UCI '
                            f'{manifest.get("release")!r}, not --schema-release {release}')
    combined = manifest['format'] == 2
    if combined and not with_log:
        raise BindingsError(f'bindings in {directory} were generated with --with-log; '
                            'pass --with-log or use RDMA-only bindings')
    if with_log and not combined:
        raise BindingsError(f'bindings in {directory} select only {MESSAGE}; --with-log needs '
                            'bindings from generate_bindings.py --with-log')
    families = MANIFEST_FAMILIES[manifest['format']]
    names = ' + '.join(message for message, _, _ in families)
    if (manifest.get('schema_version') != SCHEMA_VERSIONS[release] or
            not _described_families(manifest, families) or
            manifest.get('world') != 'closed-schema' or manifest.get('with_codec') is not True):
        raise BindingsError(f'{MANIFEST} does not describe closed-schema {names} codec bindings')
    sources = {}
    for filename in GENERATED:
        try:
            sources[filename] = (directory / filename).read_bytes()
        except OSError:
            raise BindingsError(f'missing generated {filename} in {directory}') from None
        if hashlib.sha256(sources[filename]).hexdigest() != manifest['files'][filename]:
            raise BindingsError(f'generated {filename} was modified after generation; regenerate')
    package = _execute_package(directory, sources)
    model, codec, api = package.model, package.service_codec, package.service_api
    inputs = [exchange for function in getattr(api, 'FUNCTIONS', ())
              for exchange in getattr(function, 'EXCHANGES', ())
              if getattr(exchange, 'DIRECTION', None) == 'input']
    selected, codecs = [], []
    for message, payload_name, topic in families:
        # The wire global element (MESSAGE_NAME) selects the generated
        # endpoint; the payload class must be the one that endpoint declares.
        exchanges = [exchange for exchange in inputs
                     if getattr(exchange, 'MESSAGE_NAME', None) == message]
        payload_type = getattr(model, payload_name, None)
        if (len(exchanges) != 1 or getattr(exchanges[0], 'MESSAGE_NAMESPACE', None) != OAM or
                getattr(exchanges[0], 'TOPIC', None) != topic or
                getattr(exchanges[0], 'Payload', None) is not payload_type or
                getattr(payload_type, '__schema_name__', None) != (OAM, payload_name) or
                not callable(getattr(exchanges[0], 'subscribe', None))):
            raise BindingsError(f'generated service API has no unique {message} input exchange')
        try:
            codecs.append(CodecBinding(
                OAM, message, payload_type, getattr(codec, 'encode_' + payload_name),
                getattr(codec, 'decode_' + payload_name), codec.CodecError))
        except (AttributeError, InvalidInput):
            raise BindingsError(f'generated service codec lacks the {payload_name} codec') from None
        selected.append(Family(message, payload_name, topic, exchanges[0], payload_type))
    if len(inputs) != len(families):
        raise BindingsError(f'generated service API selects input exchanges other than {names}')
    try:
        registry = CodecRegistry(codecs)
    except InvalidInput:
        raise BindingsError('generated codecs conflict in the codec registry') from None
    return Bindings(release, SCHEMA_VERSIONS[release], directory, model, codec, api,
                    selected[0].receive, selected[0].payload_type, registry,
                    tuple(selected), combined)


def endpoint_summary(endpoint):
    """Typed NetworkEndpointType Choice -> (family, address, prefix)."""
    if endpoint.alternative == 'IPv4_Endpoint':
        return 'IPv4', endpoint.value.ipv4_address.value, endpoint.value.prefix
    if endpoint.alternative == 'IPv6_Endpoint':
        return 'IPv6', endpoint.value.ipv6_address.value, endpoint.value.prefix
    raise ValueError('unknown NetworkEndpointType alternative')


def summarize(value):
    """Selected typed fields only.

    Every reported value is lexically constrained by the schema and already
    checked by the generated model (UUIDs, enumerations, timestamps, IPv4/IPv6
    carriers, integers). Free-text members (descriptions, labels, endpoint
    names, security caveats) are reported only as present/absent.
    """
    header, data = value.messageheader, value.messagedata
    return dict(
        setup_id=data.rdma_initializesetupid.uuid.value,
        initialize_id=(data.rdma_initializeid.uuid.value
                       if data.rdma_initializeid is not None else None),
        provider=data.rdma_provider.value,
        classification=value.securityinformation.classification.value,
        system_id=header.systemid.uuid.value,
        timestamp=header.timestamp.value,
        schema_version=header.schemaversion.value,
        mode=header.mode.value,
        endpoints=[endpoint_summary(endpoint) for endpoint in data.endpoint],
        host=endpoint_summary(data.host) if data.host is not None else None,
        endpoint_name_present=data.endpointname is not None)


def _endpoint_text(endpoint):
    family, address, prefix = endpoint
    return f'{family} {address}' + ('' if prefix is None else f'/{prefix}')


def message_name(value):
    """Wire global-element name for an actual generated payload value.

    Callbacks receive either generated RDMA_InitializeSetupMT or LogMT values;
    this maps the generated class identity (its `__schema_name__`) back to the
    OWP message. It never infers a wire name from the class name itself.
    """
    schema_name = getattr(type(value), '__schema_name__', None)
    for message, payload, _ in (RDMA_FAMILY, LOG_FAMILY):
        if schema_name == (OAM, payload):
            return message
    raise TypeError('not a generated RDMA_InitializeSetupMT or LogMT value')


def _uuid_or_none(identifier):
    return identifier.uuid.value if identifier is not None else None


def summarize_log(value):
    """Selected schema-checked LogMT fields only.

    Reports the LogSeverityEnum, checked timestamps, header enumerations and
    UUID identities, and the number of component/capability identities.
    Label, Details, Data, ServiceUpTime, descriptive labels and security
    caveats are never reported (free text, binary or untrusted content).
    """
    header, data = value.messageheader, value.messagedata
    return dict(
        severity=data.severity.value,
        log_timestamp=data.timestamp.value,
        classification=value.securityinformation.classification.value,
        system_id=header.systemid.uuid.value,
        timestamp=header.timestamp.value,
        schema_version=header.schemaversion.value,
        mode=header.mode.value,
        service_id=_uuid_or_none(data.serviceid),
        subsystem_id=_uuid_or_none(data.subsystemid),
        components=len(data.componentid),
        capabilities=len(data.capabilityid))


def format_log_summary(index, value):
    summary = summarize_log(value)
    return (f'RECEIVED {LOG_MESSAGE} #{index} type={type(value).__name__} '
            f'severity={summary["severity"]} log-timestamp={summary["log_timestamp"]} '
            f'mode={summary["mode"]} schema={summary["schema_version"]} '
            f'timestamp={summary["timestamp"]} classification={summary["classification"]} '
            f'service-id={summary["service_id"] or "-"} '
            f'subsystem-id={summary["subsystem_id"] or "-"} '
            f'components={summary["components"]} capabilities={summary["capabilities"]}')


def format_summary(index, value):
    if message_name(value) == LOG_MESSAGE:
        return format_log_summary(index, value)
    summary = summarize(value)
    endpoints = summary['endpoints']
    shown = [_endpoint_text(item) for item in endpoints[:MAX_DISPLAYED_ENDPOINTS]]
    if len(endpoints) > MAX_DISPLAYED_ENDPOINTS:
        shown.append(f'+{len(endpoints) - MAX_DISPLAYED_ENDPOINTS} more')
    text = (f'RECEIVED {MESSAGE} #{index} type={type(value).__name__} '
            f'setup-id={summary["setup_id"]} provider={summary["provider"]} '
            f'mode={summary["mode"]} schema={summary["schema_version"]} '
            f'timestamp={summary["timestamp"]} classification={summary["classification"]} '
            f'endpoints={len(endpoints)} [{", ".join(shown)}]')
    if summary['host'] is not None:
        text += f' host={_endpoint_text(summary["host"])}'
    return text


@dataclass(slots=True)
class SessionResult:
    status: int = EXIT_OK
    stop_reason: str = ''
    received: int = 0
    rejected: int = 0
    callback_failures: int = 0
    server_errors: int = 0
    subscribed: bool = False
    unsubscribed: bool = False
    closed: bool = False
    failure: int | None = None
    # Per global message (e.g. {'RDMA_InitializeSetup': 2, 'Log': 1}).
    received_by_message: dict = field(default_factory=dict)
    rejected_by_message: dict = field(default_factory=dict)
    # Every SID this session owned, in SUB order, and those it released.
    subscription_ids: list = field(default_factory=list)
    unsubscribed_ids: list = field(default_factory=list)


def _line(stream):
    def write(text):
        print(text, file=stream, flush=True)
    return write


def _endpoint_label(url):
    parts = urlsplit(url)  # never echo path/query, which may carry deployment data
    return f'{parts.scheme}://{parts.netloc}'


_END = object()


class _Session:
    """One connection, one generated subscription per selected message family,
    one bounded application inbox shared (in arrival order) by all of them."""

    def __init__(self, options, bindings, callback, stop, out, err, buffer_size):
        self.options, self.bindings, self.callback = options, bindings, callback
        self.stop, self.out, self.err = stop, out, err
        self.result = SessionResult()
        self.finished = asyncio.Event()
        # Bounded application buffer. A full inbox back-pressures the runtime's
        # own bounded per-subscription queue; nothing is silently dropped.
        self.inbox = asyncio.Queue(buffer_size)
        self.runtime = LacalRuntime(runtime_config(options), bindings.registry)
        self.combined = bindings.with_log
        # Owned subscriptions in SUB order: [(family, handle)]. Teardown
        # releases exactly these; nothing else is ever unsubscribed.
        self.owned = []
        self.families_by_sid = {}
        self.subscribing = None  # family whose SUB is in flight
        self.handle = None       # first (RDMA) handle, as in PY-OMS-020
        self.consumer = self.monitor = None
        self.accepting = True

    def _suffix(self, family):
        # RDMA-only output stays byte-identical to PY-OMS-020.
        return f' message={family.message}' if self.combined else ''

    def _family(self, sid):
        # A rejection can be reported before subscribe() returns its handle;
        # SUBs are strictly sequential, so an unrecorded SID is the one in flight.
        # RDMA-only mode has a single family and reports it as PY-OMS-020 did.
        if not self.combined:
            return self.bindings.families[0]
        return self.families_by_sid.get(sid) or self.subscribing

    def finish(self, reason, failure=None):
        if failure is not None and self.result.failure is None:
            self.result.failure = failure
        if not self.finished.is_set():
            self.result.stop_reason = reason
            self.finished.set()

    def _deliverer(self, family):
        async def deliver(value):
            # Runs in this subscription's runtime-owned dispatch task with the
            # value decoded by this subscription's own registered codec.
            if self.accepting:
                await self.inbox.put((family, value))
        return deliver

    async def consume(self):
        limit = self.options.max_messages
        while (item := await self.inbox.get()) is not _END:
            family, value = item
            if not self.accepting or (limit is not None and self.result.received >= limit):
                continue
            if type(value) is not family.payload_type:
                # Defensive: the runtime already enforces the registered type.
                self.result.rejected += 1
                self.err(f'REJECTED {family.message} payload (unexpected generated type)')
                continue
            self.result.received += 1
            counts = self.result.received_by_message
            counts[family.message] = counts.get(family.message, 0) + 1
            try:
                outcome = self.callback(value)
                if inspect.isawaitable(outcome):
                    await outcome
            except Exception as error:
                self.result.callback_failures += 1
                self.err(f'CALLBACK FAILED #{self.result.received} ({type(error).__name__})'
                         + self._suffix(family))
            if limit is not None and self.result.received >= limit:
                self.finish('max-messages')

    async def watch(self):
        while True:
            event = await self.runtime.next_event()
            error, sid = event.error, event.subscription_id
            name = type(error).__name__
            if sid is not None and isinstance(error, (CodecFailure, CodecInternalFailure, ProtocolError)):
                # Runtime envelope (ProtocolError) or generated decoder rejection.
                # The subscription stays active; the message is not delivered.
                family = self._family(sid)
                message = family.message if family is not None else 'unowned-subscription'
                self.result.rejected += 1
                counts = self.result.rejected_by_message
                counts[message] = counts.get(message, 0) + 1
                self.err(f'REJECTED {message} payload ({name}: {error})')
            elif isinstance(error, UnknownSubscription):
                self.err('IGNORED message for an unknown subscription')
            elif isinstance(error, ServerRejection):
                self.result.server_errors += 1
                self.err(f'SERVER ERROR {error.code}')
            else:
                self.err(f'SESSION FAILED ({name}: {error})')
                self.finish('failure', EXIT_SESSION)
            if self.runtime.terminal_error is not None:
                self.finish('failure', EXIT_SESSION)

    def _connect_failure(self, error):
        if isinstance(error, ServerRejection):
            self.err(f'CONNECT REJECTED by server: {error.code}')
            return EXIT_REJECTED
        cause = error.__context__
        if isinstance(cause, ssl.SSLCertVerificationError):
            # Certificate chain or hostname verification failed; never bypassed.
            self.err(f'CONNECT FAILED: TLS certificate verification failed ({cause.verify_message})')
        else:
            self.err(f'CONNECT FAILED ({type(error).__name__}: {error})')
        return EXIT_CONNECT

    async def _subscribe(self, family):
        """SUB one generated endpoint. True only after its +OK; ownership is
        recorded at once so teardown can always release it."""
        self.subscribing = family
        try:
            handle = await family.receive.subscribe(self.runtime, self._deliverer(family))
        except ServerRejection as error:
            self.subscribing = None
            self.err(f'SUBSCRIBE REJECTED by server: {error.code}' + self._suffix(family))
            self.result.failure = EXIT_REJECTED
            return False
        except LacalError as error:
            self.subscribing = None
            self.err(f'SUBSCRIBE FAILED ({type(error).__name__}: {error})' + self._suffix(family))
            self.result.failure = EXIT_SESSION
            return False
        self.owned.append((family, handle))
        self.families_by_sid[handle.id] = family
        self.subscribing = None
        self.result.subscription_ids.append(handle.id)
        if self.handle is None:
            self.handle = handle
        if self.combined:
            self.out(f'SUBSCRIBED {family.message} topic={family.receive.TOPIC} sid={handle.id} '
                     f'payload={family.payload_type.__name__}')
        else:
            self.out(f'SUBSCRIBED {family.message} topic={family.receive.TOPIC} sid={handle.id} '
                     f'payload={family.payload_type.__name__} '
                     f'duration={self.options.duration}s')
        return True

    async def run(self):
        try:
            try:
                await self.runtime.connect()
            except LacalError as error:
                self.result.failure = self._connect_failure(error)
                return
            self.out(f'CONNECTED {_endpoint_label(self.options.url)} owp=1.0 '
                     f'service-id={self.options.service_id} schema={self.bindings.schema_version}')
            self.monitor = asyncio.create_task(self.watch(), name='python-uci-client-events')
            self.consumer = asyncio.create_task(self.consume(), name='python-uci-client-callback')
            # Strictly sequential SUBs on the one runtime: a failed later SUB
            # stops setup, and teardown releases every earlier owned SID.
            for family in self.bindings.families:
                if not await self._subscribe(family):
                    return
            if self.combined:
                if self.runtime.terminal_error is not None or not self.runtime.is_connected:
                    # Every SUB was acknowledged but the session then failed:
                    # never report a combined client as ready.
                    self.err('NOT READY: session failed after subscription setup')
                    self.finish('failure', EXIT_SESSION)
                    return
                self.out(f'READY subscriptions={len(self.owned)} '
                         f'messages={",".join(family.message for family, _ in self.owned)} '
                         f'duration={self.options.duration}s')
            self.result.subscribed = True
            waits = [asyncio.create_task(self.finished.wait()), asyncio.create_task(self.stop.wait())]
            try:
                await asyncio.wait(waits, timeout=self.options.duration,
                                   return_when=asyncio.FIRST_COMPLETED)
            finally:
                for task in waits:
                    task.cancel()
                await asyncio.gather(*waits, return_exceptions=True)
            self.finish('stopped' if self.stop.is_set() else 'duration')
        finally:
            await self.teardown()

    async def teardown(self):
        """UNSUB every owned SID, close, then stop the callback. Safe after any
        failure, partial setup or cancel.

        Owned subscriptions are released in reverse SUB order. An SID that is
        no longer active (the runtime already failed) cannot be UNSUBed; the
        runtime close below ends the connection, and the broker drops every
        subscription of a closed connection, so no ghost SID survives.
        Order matters: the runtime is closed (joining its receive and dispatch
        tasks) before the application consumer is ended, so no producer can
        refill the bounded inbox and no callback runs after teardown returns.
        """
        self.accepting = False
        try:
            for family, handle in reversed(self.owned):
                if not handle.active:
                    if self.combined:
                        self.err(f'UNSUBSCRIBE SKIPPED sid={handle.id}{self._suffix(family)} '
                                 '(session already failed; released by connection close)')
                    continue
                try:
                    await handle.unsubscribe()
                    self.result.unsubscribed_ids.append(handle.id)
                    self.out(f'UNSUBSCRIBED sid={handle.id}' + self._suffix(family))
                except LacalError as error:
                    self.err(f'UNSUBSCRIBE FAILED ({type(error).__name__})' + self._suffix(family))
                    self.result.failure = self.result.failure or EXIT_SESSION
            self.result.unsubscribed = (bool(self.owned) and
                                        len(self.result.unsubscribed_ids) == len(self.owned))
        finally:
            try:
                # Idempotent and cancellation-safe; joins runtime-owned tasks.
                await self.runtime.close()
                if self.consumer is not None and not self.consumer.done():
                    while not self.inbox.empty():
                        self.inbox.get_nowait()
                    self.inbox.put_nowait(_END)
                    try:
                        await asyncio.wait_for(asyncio.shield(self.consumer), DRAIN_TIMEOUT)
                    except TimeoutError:
                        self.err('CALLBACK did not finish during shutdown; cancelled')
                        self.result.failure = self.result.failure or EXIT_SESSION
            finally:
                await _join(task for task in (self.consumer, self.monitor) if task is not None)
                self.result.closed = True
                closed = (f'CLOSED received={self.result.received} rejected={self.result.rejected} '
                          f'callback-failures={self.result.callback_failures}')
                if self.combined:
                    closed += ' ' + ' '.join(
                        f'{family.message}={self.result.received_by_message.get(family.message, 0)}'
                        f'/{self.result.rejected_by_message.get(family.message, 0)}'
                        for family in self.bindings.families)
                self.out(closed)


async def _join(tasks):
    """Cancel and join application tasks, even if the caller is cancelled meanwhile."""
    tasks = list(tasks)
    for task in tasks:
        task.cancel()
    cancelled = False
    while not all(task.done() for task in tasks):
        try:
            await asyncio.shield(asyncio.gather(*tasks, return_exceptions=True))
        except asyncio.CancelledError:
            cancelled = True
    if cancelled:
        raise asyncio.CancelledError()


def printing_callback(stream=None):
    """Default application callback: print selected typed fields.

    The `#N` index counts every typed message the callback receives, across
    all enabled subscriptions, in delivery order.
    """
    write = _line(stream or sys.stdout)
    counter = itertools.count(1)

    def callback(value):
        write(format_summary(next(counter), value))
    return callback


async def run_client(options, *, bindings=None, callback=None, stop=None,
                     stdout=None, stderr=None, buffer_size=DEFAULT_BUFFER):
    """Receive typed values until duration, limit or stop.

    `callback(value)` receives each actual generated RDMA_InitializeSetupMT
    instance (and, with `options.with_log`, each generated LogMT instance)
    outside the runtime receive path; it may be async. Values of one
    subscription arrive in order; the two families may interleave. Use
    `isinstance(value, bindings.model.LogMT)` or `message_name(value)` to
    tell them apart. Returns a SessionResult whose `status` is the exit code.
    """
    if type(buffer_size) is not int or not 1 <= buffer_size <= MAX_BUFFER:
        raise ValueError(f'buffer_size must be between 1 and {MAX_BUFFER}')
    bindings = bindings or load_bindings(options.bindings, options.schema_release,
                                         options.with_log)
    if bindings.release != options.schema_release:
        raise BindingsError('generated bindings do not match --schema-release')
    if bindings.with_log is not options.with_log:
        raise BindingsError('generated bindings do not match --with-log')
    session = _Session(options, bindings, callback or printing_callback(stdout),
                       stop or asyncio.Event(), _line(stdout or sys.stdout),
                       _line(stderr or sys.stderr), buffer_size)
    await session.run()
    result = session.result
    if result.failure is not None:
        result.status = result.failure
    elif result.rejected or result.callback_failures:
        result.status = EXIT_DEGRADED
    else:
        result.status = EXIT_OK
    return result


async def _main(options, bindings):
    loop = asyncio.get_running_loop()
    stop, signals = asyncio.Event(), []

    def request_stop(signum):
        signals.append(signum)
        stop.set()

    for signum in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(signum, request_stop, signum)
    try:
        result = await run_client(options, bindings=bindings, stop=stop)
    finally:
        for signum in (signal.SIGINT, signal.SIGTERM):
            loop.remove_signal_handler(signum)
    if signals and result.status in (EXIT_OK, EXIT_DEGRADED):
        return 128 + signals[0]
    return result.status


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    try:
        options = parse_arguments(argv)
    except UsageError as error:
        print(f'{PROGRAM}: error: {error}\nTry: {PROGRAM} --help', file=sys.stderr)
        return EXIT_USAGE
    if options is None:
        print(HELP, end='')
        return EXIT_OK
    try:
        bindings = load_bindings(options.bindings, options.schema_release, options.with_log)
    except BindingsError as error:
        print(f'{PROGRAM}: error: {error}', file=sys.stderr)
        return EXIT_BINDINGS
    return asyncio.run(_main(options, bindings))


if __name__ == '__main__':
    raise SystemExit(main())
