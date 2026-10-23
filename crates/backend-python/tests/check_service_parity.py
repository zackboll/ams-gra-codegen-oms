"""PY-003 delivery parity including selected models and service wrappers.

Usage: python3 <this-file> BASELINE_BINARY CANDIDATE_BINARY EVIDENCE_JSON
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory

from check_existing_backends import snapshot as model_snapshot


def snapshot(binary, output):
    evidence = model_snapshot(binary, output)
    fixtures = Path(__file__).resolve().parents[3] / 'tests/fixtures/service-generate'
    for world in ('closed-schema', 'open-extensions'):
        for language in ('ada', 'rust', 'cpp'):
            for contract in ('only-a.yaml', 'non-uci.yaml'):
                dest = output / 'service' / world / language / contract
                args = [str(binary), 'service-generate', '--schema', str(fixtures / 'root.xsd'),
                        '--contract', str(fixtures / contract), '--language', language,
                        '--world', world, '--output', str(dest)]
                result = subprocess.run(args, capture_output=True, check=True)
                evidence[f'service/{world}/{language}/{contract}'] = {
                    'stdout': result.stdout.decode().replace(str(dest), '<OUTPUT>'),
                    'stderr': result.stderr.decode(),
                    'files': {str(f.relative_to(dest)): hashlib.sha256(f.read_bytes()).hexdigest()
                              for f in sorted(dest.rglob('*')) if f.is_file()},
                }
    return evidence


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    with TemporaryDirectory(prefix='py003-parity-') as temp:
        results = {label: snapshot(Path(binary).resolve(), Path(temp) / label)
                   for label, binary in zip(('before', 'after'), sys.argv[1:3])}
    if results['before'] != results['after']:
        raise RuntimeError('baseline model/wrapper/readiness/coverage parity mismatch')
    Path(sys.argv[3]).write_text(json.dumps(results, indent=2) + '\n', encoding='utf-8')
    print('PY003 BASELINE PARITY: PASSED (6 full model runs, 12 selected model/wrapper runs, '
          '12 readiness reports, 2 coverage reports)')
    print('Canonical candidate evidence SHA256: ' +
          hashlib.sha256(json.dumps(results['after'], sort_keys=True).encode()).hexdigest())


if __name__ == '__main__':
    main()