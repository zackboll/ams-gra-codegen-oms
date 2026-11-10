#!/usr/bin/env python3
"""Bound Ubuntu APT operations, retaining signed indexes and required tools."""
from pathlib import Path
import os
import re
import signal
import subprocess
import tempfile
import time


OPTIONS = [
    '-o', 'Acquire::Retries=2',
    '-o', 'Acquire::http::Timeout=20',
    '-o', 'Acquire::https::Timeout=20',
    '-o', 'DPkg::Lock::Timeout=30',
    '-o', 'APT::Update::Error-Mode=any',
]


def execute(command, timeout):
    start = time.monotonic()
    print('APT_STAGE start', command, 'timeout_seconds=', timeout, flush=True)
    process = subprocess.Popen(command, start_new_session=True)
    try:
        status = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        # APT has acquisition/dpkg children: kill the whole private group so a
        # timed-out attempt cannot retain locks beneath the fallback attempt.
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        status = 124
    print('APT_STAGE end elapsed_seconds=', round(time.monotonic() - start, 3),
          'exit=', status, flush=True)
    return status


def install(options):
    if execute(['apt-get', *options, 'update'], 180) != 0:
        return False
    return execute(['apt-get', *options, 'install', '--no-install-recommends',
                    '-y', 'gnat', 'strace'], 300) == 0


def fallback_sources(source_root, destination):
    """Copy only official Ubuntu definitions, using the HTTPS archive fallback.

    APT still verifies Release signatures with the runner's existing keyrings.
    Never trust an unsigned mirror or modify the host's source definitions.
    """
    candidates = [source_root / 'sources.list']
    candidates += sorted((source_root / 'sources.list.d').glob('*.sources'))
    candidates += sorted((source_root / 'sources.list.d').glob('*.list'))
    count = 0
    for path in candidates:
        if not path.is_file():
            continue
        text = path.read_text()
        sections = re.split(r'\n\s*\n', text) if path.suffix == '.sources' else text.splitlines()
        official = []
        for section in sections:
            urls = re.findall(r'https?://[^\s]+', section)
            if not urls or not all(re.fullmatch(
                    r'https?://(?:azure\.archive|archive|security)\.ubuntu\.com/ubuntu/?',
                    url) for url in urls):
                continue
            official.append(re.sub(
                r'https?://(?:azure\.archive|archive|security)\.ubuntu\.com/ubuntu/?',
                'https://archive.ubuntu.com/ubuntu/', section))
        if official:
            suffix = '.sources' if path.suffix == '.sources' else '.list'
            (destination / f'ubuntu-{count}{suffix}').write_text('\n\n'.join(official) + '\n')
            count += 1
    if not count:
        raise RuntimeError('No recognized official Ubuntu sources; refusing unverified fallback')


def main():
    if os.geteuid() != 0:
        raise SystemExit('Run as root (sudo python3 scripts/install-rust-ci-dependencies.py)')
    os.environ['DEBIAN_FRONTEND'] = 'noninteractive'
    if not install(OPTIONS):
        print('Primary APT attempt failed; retrying signed official Ubuntu HTTPS archive', flush=True)
        with tempfile.TemporaryDirectory(prefix='rust-ci-apt-') as directory:
            root = Path(directory)
            root.chmod(0o755)
            fallback_sources(Path('/etc/apt'), root)
            options = OPTIONS + ['-o', 'Dir::Etc::sourcelist=/dev/null',
                                 '-o', f'Dir::Etc::sourceparts={root}']
            if not install(options):
                raise SystemExit('Both bounded APT attempts failed; dependencies are mandatory')
    for command in [['dpkg-query', '-W', 'gnat', 'strace'],
                    ['gnatmake', '--version'], ['strace', '-V']]:
        if execute(command, 20) != 0:
            raise SystemExit('Required Rust/Ada/Python regression dependency verification failed')


if __name__ == '__main__':
    main()