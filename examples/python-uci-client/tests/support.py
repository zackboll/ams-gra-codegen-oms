"""Shared python-uci-client test support. Nothing here frames OWP for the client.

The independent publisher/observer is a raw `websockets` OWP peer that never
imports the application, the runtime or generated code. It publishes
independently authored Task092 RDMA golden documents and PY-016 Log golden
documents (or targeted edits of them).
"""
import asyncio
import copy
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import tempfile

from websockets.asyncio.client import connect

APP = Path(__file__).resolve().parents[1]
ROOT = APP.parents[1]
sys.dont_write_bytecode = True  # never leave caches in the source tree
sys.path.insert(0, str(APP))
import generate_bindings  # noqa: E402
import python_uci_client as client  # noqa: E402

SLEET_REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
SLEET_TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
SERVICE = 'python-uci-client'
SERVICE_UUID = '550e8400-e29b-41d4-a716-446655440020'
TOPIC = 'uci.rdma'
LOG_TOPIC = 'uci.log'
TLS_HELPER = ROOT / 'examples/rust-uci-client/tests/support/tls_endpoint.py'


def golden(release):
    """Independent Task092 golden payload body for the release."""
    return json.loads((ROOT / 'tests/fixtures/service-generate' /
                       f'task092-real-rdma{release.replace(".", "")}.json').read_text())


def log_golden(release):
    """Independent PY-016 full-schema Log golden body for the release."""
    return json.loads((ROOT / 'tests/fixtures/python-whitespace-visible/goldens' /
                       f'{release}-Log.json').read_text())


def log_variant(release, severity=None, timestamp=None, log_timestamp=None):
    body = copy.deepcopy(log_golden(release))
    if severity is not None:
        body['MessageData']['Severity'] = severity
    if timestamp is not None:
        body['MessageHeader']['Timestamp'] = timestamp
    if log_timestamp is not None:
        body['MessageData']['Timestamp'] = log_timestamp
    return body


def variant(release, setup_uuid=None, endpoints=None, provider=None, timestamp=None):
    body = copy.deepcopy(golden(release))
    if setup_uuid is not None:
        body['MessageData']['RDMA_InitializeSetupID']['UUID'] = setup_uuid
    if endpoints is not None:
        body['MessageData']['Endpoint'] = endpoints
    if provider is not None:
        body['MessageData']['RDMA_Provider'] = provider
    if timestamp is not None:
        body['MessageHeader']['Timestamp'] = timestamp
    return body


def schema_root(release):
    name = 'AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'
    value = os.environ.get(name)
    if not value or not Path(value).is_file():
        raise RuntimeError(f'missing pinned UCI root {name}')
    return Path(value)


def generator():
    for candidate in (os.environ.get('AMS_GRA_GENERATOR'),
                      ROOT / 'target/release/ams-gra-codegen-oms',
                      ROOT / 'target/debug/ams-gra-codegen-oms'):
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    raise RuntimeError('missing production ams-gra-codegen-oms generator')


_GENERATED = {}


def bindings_directory(release, with_log=False):
    """Generated bindings for one release, in external temporary storage.

    PY_OMS_020_BINDINGS may name a directory holding `2.5`/`2.6` (RDMA-only)
    and `2.5-log`/`2.6-log` (--with-log) outputs of generate_bindings.py (the
    qualification wrapper generates each once per interpreter). They are still
    digest-verified by load_bindings. Otherwise generate once per test module;
    modules call `cleanup_bindings`.
    """
    key = release + ('-log' if with_log else '')
    shared = os.environ.get('PY_OMS_020_BINDINGS')
    if shared:
        directory = Path(shared) / key
        if not (directory / generate_bindings.MANIFEST).is_file():
            raise RuntimeError(f'PY_OMS_020_BINDINGS lacks generated {key} bindings')
        return directory
    if key not in _GENERATED:
        holder = tempfile.TemporaryDirectory(prefix=f'py-oms-02x-{key}-')
        output = Path(holder.name) / 'bindings'
        try:
            generate_bindings.generate(release, schema_root(release), output, generator(),
                                       with_log=with_log)
        except BaseException:
            holder.cleanup()
            raise
        _GENERATED[key] = (holder, output)
    return _GENERATED[key][1]


def cleanup_bindings():
    while _GENERATED:
        _GENERATED.popitem()[1][0].cleanup()


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


async def wait_port(port, process, timeout=60):
    async with asyncio.timeout(timeout):
        while True:
            if process.poll() is not None:
                raise RuntimeError('process exited before listening')
            try:
                _, writer = await asyncio.open_connection('127.0.0.1', port)
                writer.close()
                await writer.wait_closed()
                return
            except OSError:
                await asyncio.sleep(.05)


async def stop_process(process, timeout=5):
    """Terminate and reap a helper process without blocking the event loop."""
    if process.poll() is None:
        process.terminate()
    loop = asyncio.get_running_loop()
    deadline = loop.time() + timeout
    while process.poll() is None:
        if loop.time() >= deadline:
            process.kill()
            deadline = loop.time() + timeout
        await asyncio.sleep(.02)


def sleet_binary():
    """Unchanged pinned Sleet only; a missing/other broker is an error, not a skip."""
    value = os.environ.get('AMS_GRA_SLEET_BIN')
    if not value or not Path(value).is_file():
        raise RuntimeError('AMS_GRA_SLEET_BIN must name the pinned Sleet release binary')
    binary = Path(value).resolve()
    checkout = binary.parents[2]
    identity = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD',
                                        'HEAD^{tree}'], text=True).splitlines()
    status = subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain'])
    if identity != [SLEET_REV, SLEET_TREE] or status:
        raise RuntimeError('AMS_GRA_SLEET_BIN is not a clean pinned Sleet checkout')
    return binary


class Broker:
    """One isolated unchanged Sleet process for one release (debug log retained)."""

    def __init__(self, release, *, schema=None, allowed_messages=('RDMA_InitializeSetup',),
                 bindings=None):
        """`bindings` maps authorized topic -> allowed global messages. The
        default is the PY-OMS-020 single uci.rdma binding (`allowed_messages`)."""
        self.release, self.binary = release, sleet_binary()
        self.directory = tempfile.TemporaryDirectory(prefix=f'py-oms-020-sleet-{release}-')
        path = Path(self.directory.name)
        services = path / 'services.d'
        services.mkdir()
        bindings = bindings if bindings is not None else {TOPIC: tuple(allowed_messages)}
        text = (f'service_id="{SERVICE}"\nservice_uuid="{SERVICE_UUID}"\n'
                f'allowed_topics={json.dumps(list(bindings))}\n')
        for topic, messages in bindings.items():
            text += (f'[[topic_bindings]]\ntopic="{topic}"\n'
                     f'allowed_messages={json.dumps(list(messages))}\n')
        (services / 'client.toml').write_text(text)
        self.port = free_port()
        self.url = f'ws://127.0.0.1:{self.port}'
        schema = schema or schema_root(release)
        (path / 'sleet.toml').write_text(
            f'bind_addr="127.0.0.1:{self.port}"\nserver_id="py-oms-020"\n'
            f'system_label="python-uci-client qualification"\nschema_path={json.dumps(str(schema))}\n'
            f'schema_version="002.{release[-1]}.0"\n'
            f'system_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
            f'services_dir={json.dumps(str(services))}\n')
        self.log_path = path / 'broker.log'
        self.process = None

    async def __aenter__(self):
        self.log = self.log_path.open('w')
        self.process = subprocess.Popen(
            [str(self.binary), str(Path(self.directory.name) / 'sleet.toml')],
            stdout=self.log, stderr=subprocess.STDOUT,
            env={**os.environ, 'RUST_LOG': 'sleet=debug', 'NO_COLOR': '1'})
        await wait_port(self.port, self.process)
        return self

    async def __aexit__(self, *exc):
        await stop_process(self.process)
        self.log.close()
        self.directory.cleanup()

    def events(self):
        """Broker-side debug evidence with ANSI styling removed."""
        return re.sub(r'\x1b\[[0-9;]*m', '', self.log_path.read_text())


class RawPeer:
    """Independent OWP publisher/observer over raw websockets (not oms_lacal)."""

    def __init__(self, url, schema_version, *, ssl_context=None):
        self.url, self.schema_version, self.ssl_context = url, schema_version, ssl_context

    async def __aenter__(self):
        options = dict(subprotocols=['owp'], proxy=None)
        if self.ssl_context is not None:
            options['ssl'] = self.ssl_context
        self.ws = await connect(self.url, **options)
        await self.ws.send('INIT ' + json.dumps(dict(
            versions=['1.0'], schema=self.schema_version, service_id=SERVICE, verbose=True)))
        ack, info = await self.recv(), await self.recv()
        if ack != '+OK' or not info.startswith('INFO '):
            raise RuntimeError('independent peer INIT failed')
        return self

    async def __aexit__(self, *exc):
        await self.ws.close()

    async def recv(self, timeout=10):
        return await asyncio.wait_for(self.ws.recv(), timeout)

    async def status(self, frame):
        """Send one status-bearing operation; collect interleaved MSG frames."""
        await self.ws.send(frame)
        messages = []
        while True:
            reply = await self.recv()
            if reply.startswith('MSG '):
                messages.append(reply)
                continue
            return reply, messages

    async def publish(self, body, name='RDMA_InitializeSetup', topic=TOPIC):
        text = body if isinstance(body, str) else json.dumps({name: body})
        return await self.status(f'PUB {topic} {text}')

    async def publish_log(self, body):
        return await self.publish(body, name='Log', topic=LOG_TOPIC)


async def run_cli(arguments, *, until=None, timeout=60, env=None):
    """Execute the real CLI as a subprocess. Returns (code, stdout, stderr).

    `until(line)` is awaited for every stdout line, letting a test drive the
    independent publisher at protocol-meaningful points (e.g. after SUBSCRIBED).
    """
    process = await asyncio.create_subprocess_exec(
        sys.executable, *(['-O'] if sys.flags.optimize else []),
        str(APP / 'python_uci_client.py'), *map(str, arguments),
        stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1', **(env or {})})
    lines = []
    try:
        async with asyncio.timeout(timeout):
            stderr_task = asyncio.create_task(process.stderr.read())
            while line := await process.stdout.readline():
                text = line.decode().rstrip('\n')
                lines.append(text)
                if until is not None:
                    await until(text, process)
            code = await process.wait()
            return code, lines, (await stderr_task).decode()
    finally:
        if process.returncode is None:
            process.kill()
            await process.wait()
