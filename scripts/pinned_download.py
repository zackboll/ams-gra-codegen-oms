#!/usr/bin/env python3
"""Bounded HTTPS recovery for exact CI tool archives; never retry qualification.

Release-asset API URLs are independent official retrieval paths, not mirrors.
Every path (including an existing local archive) must satisfy the same pin and
archive-integrity check before atomic publication. Alire still checks its index
origin hashes after its download hook returns.
"""
import argparse
import gzip
import hashlib
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import time
import zipfile


PINS = {
    'alire': (
        'https://github.com/alire-project/alire/releases/download/v2.1.1/alr-2.1.1-bin-x86_64-linux.zip',
        'https://api.github.com/repos/alire-project/alire/releases/assets/429362820',
        'sha256', '09c66bcd8c35dd4b97b72c3d9b76e44caa6964a2db35aba069f396f00f1f64c7', 'zip'),
    'actionlint': (
        'https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz',
        'https://api.github.com/repos/rhysd/actionlint/releases/assets/384924896',
        'sha256', '8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8', 'tar.gz'),
    'gnat': (
        'https://github.com/alire-project/GNAT-FSF-builds/releases/download/gnat-16.1.0-1/gnat-x86_64-linux-16.1.0-1.tar.gz',
        'https://api.github.com/repos/alire-project/GNAT-FSF-builds/releases/assets/458865500',
        'sha256', '9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703', 'tar.gz'),
    'gprbuild': (
        'https://github.com/alire-project/GNAT-FSF-builds/releases/download/gprbuild-26.0.0-1/gprbuild-x86_64-linux-26.0.0-1.tar.gz',
        'https://api.github.com/repos/alire-project/GNAT-FSF-builds/releases/assets/397032100',
        'sha256', 'e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b', 'tar.gz'),
    'gnatcoll': (
        'https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v26.0.0.zip',
        'https://codeload.github.com/AdaCore/gnatcoll-core/zip/refs/tags/v26.0.0',
        'sha512', 'cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b', 'zip'),
}
ATTEMPTS = 3
DELAYS = (2, 4)


def validate(path, pin):
    digest = hashlib.new(pin[2])
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    if digest.hexdigest() != pin[3]:
        raise RuntimeError('pinned archive checksum mismatch: ' + str(path))
    if pin[4] == 'zip':
        with zipfile.ZipFile(path) as archive:
            if not archive.namelist() or archive.testzip() is not None:
                raise RuntimeError('invalid/empty ZIP archive')
    else:
        # Drain gzip as well as reading tar headers: tar may stop before the
        # gzip footer and otherwise miss a truncated compressed stream.
        with gzip.open(path, 'rb') as stream:
            while stream.read(1024 * 1024):
                pass
        with tarfile.open(path, 'r:gz') as archive:
            if not archive.getmembers():
                raise RuntimeError('empty tar archive')


def transient(code, diagnostic):
    return code in (5, 6, 7, 18, 28, 52, 55, 56) or (
        code == 22 and re.search(r'error: (502|503|504)\b', diagnostic) is not None)


def download(name, destination):
    pin = PINS[name]
    destination = Path(destination)
    if destination.exists():
        validate(destination, pin)
        print('Verified existing pinned archive:', destination, flush=True)
        return
    for url in pin[:2]:
        for attempt in range(ATTEMPTS):
            with tempfile.NamedTemporaryFile(dir=destination.parent, prefix='.pinned-', delete=False) as file:
                part = Path(file.name)
            try:
                command = ['curl', '--fail', '--location', '--silent', '--show-error',
                           '--proto', '=https', '--proto-redir', '=https',
                           '--connect-timeout', '30', '--max-time', '300']
                if url.startswith('https://api.github.com/'):
                    command += ['--header', 'Accept: application/octet-stream']
                command += ['--output', str(part), url]
                result = subprocess.run(command, capture_output=True, text=True)
                print(f'{name}: path={url} attempt={attempt + 1}/{ATTEMPTS} curl_exit={result.returncode}', flush=True)
                if result.returncode == 0:
                    validate(part, pin)  # Integrity failures are fatal, never retried.
                    part.replace(destination)
                    return
                print(result.stderr, end='', flush=True)
                if not transient(result.returncode, result.stderr):
                    raise RuntimeError(f'non-transient pinned download failure: curl exit {result.returncode}')
            finally:
                part.unlink(missing_ok=True)
            if attempt + 1 < ATTEMPTS:
                time.sleep(DELAYS[attempt])
    raise RuntimeError(f'all pinned retrieval attempts exhausted: {name}')


def verify_version(name, executable):
    expected = {'alire': 'alr 2.1.1', 'actionlint': '1.7.12'}[name]
    flag = '--version' if name == 'alire' else '-version'
    output = subprocess.check_output([str(executable), flag], text=True).splitlines()
    if not output or output[0] != expected:
        raise RuntimeError('wrong pinned tool version: ' + name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-version', choices=['alire', 'actionlint'])
    parser.add_argument('tool', help='pin name, or exact Alire index origin URL')
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    if args.verify_version:
        if args.tool != args.verify_version:
            parser.error('version verification must name the same tool')
        verify_version(args.tool, args.destination)
        return
    name = args.tool
    if name not in PINS:
        matches = [key for key, pin in PINS.items() if pin[0] == name]
        if len(matches) != 1:
            parser.error('unknown or ambiguous pinned origin URL')
        name = matches[0]
    download(name, args.destination)


if __name__ == '__main__':
    main()