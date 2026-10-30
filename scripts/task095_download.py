#!/usr/bin/env python3
"""Bounded recovery for the isolated Task095 pinned provider archive only.

Alire still verifies every index-origin hash. The known GNATCOLL archive is
also verified here before atomic publication; partial or corrupt downloads
never become Alire's destination. No build or qualification is retried.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time

URL = 'https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v26.0.0.zip'
SHA512 = 'cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b'
ATTEMPTS = 3
DELAYS = (2, 4)


def transient(code, output):
    if code in (5, 6, 7, 18, 28, 52, 55, 56):
        return 'transient curl transport exit ' + str(code)
    if code == 22 and re.search(rb'error: (408|429|500|502|503|504)\b', output):
        return 'transient HTTP response'
    return None


def download(url, destination, evidence):
    destination = Path(destination)
    evidence = Path(evidence)
    if destination.exists():
        raise RuntimeError('refusing an existing archive destination')
    identity = hashlib.sha256((url + '\0' + str(destination)).encode()).hexdigest()[:20]
    budget = ATTEMPTS if url == URL else 1
    for attempt in range(1, budget + 1):
        prefix = evidence / f'download-{identity}-{attempt}'
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix='.task095-', suffix='.part', delete=False) as temporary:
            part = Path(temporary.name)
        command = ['curl', '--fail', '--location', '--silent', '--show-error']
        if url == URL:
            command += ['--connect-timeout', '30', '--max-time', '180']
        command += ['--output', str(part), url]
        try:
            with prefix.with_suffix('.command.json').open('x') as record:
                record.write(json.dumps(dict(command=command, cwd=str(Path.cwd())), indent=2) + '\n')
            with prefix.with_suffix('.log').open('xb') as output:
                result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT)
            prefix.with_suffix('.exit').write_text(str(result.returncode) + '\n')
            reason = transient(result.returncode, prefix.with_suffix('.log').read_bytes())
            if result.returncode == 0:
                if url == URL and hashlib.sha512(part.read_bytes()).hexdigest() != SHA512:
                    prefix.with_suffix('.decision.log').write_text('fatal: pinned GNATCOLL archive hash mismatch; no retry\n')
                    raise RuntimeError('pinned GNATCOLL archive hash mismatch')
                part.replace(destination)
                prefix.with_suffix('.decision.log').write_text('complete: atomic archive publication; Alire origin verification remains mandatory\n')
                return
            retry = reason is not None and attempt < budget
            delay = DELAYS[attempt - 1] if retry else 0
            prefix.with_suffix('.decision.log').write_text(json.dumps(dict(
                reason=reason or 'non-transient curl failure', retry=retry,
                attempt=attempt, budget=budget, delay_seconds=delay)) + '\n')
            if not retry:
                raise RuntimeError(f'archive download failed: curl exit {result.returncode}; attempts={attempt}/{budget}')
        finally:
            part.unlink(missing_ok=True)
        time.sleep(delay)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('url')
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    download(args.url, args.destination, args.evidence)


if __name__ == '__main__':
    main()