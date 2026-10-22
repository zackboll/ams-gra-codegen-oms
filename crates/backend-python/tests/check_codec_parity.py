"""Compare exact PY-003 non-codec behavior and existing three-language evidence.

Usage: python3 <this-file> BASELINE_BINARY CANDIDATE_BINARY EVIDENCE_JSON
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory

from check_service_parity import snapshot as existing_snapshot


def snapshot(binary, output):
    evidence = existing_snapshot(binary, output)
    root = Path(__file__).resolve().parents[3]
    for world in ('closed-schema', 'open-extensions'):
        for schema, contract in [
            ('python-service/root.xsd', 'python-service/service.yaml'),
            ('python-codec/oam.xsd', 'python-codec/service.yaml'),
            ('python-codec/non-oam.xsd', 'python-codec/service.yaml'),
            ('python-service/root.xsd', 'service-generate/non-uci.yaml'),
        ]:
            args = ['--schema', str(root / 'tests/fixtures' / schema),
                    '--contract', str(root / 'tests/fixtures' / contract),
                    '--language', 'python', '--world', world]
            check = subprocess.run([str(binary), 'service-check', *args], capture_output=True, check=True)
            dest = output / 'python' / world / schema / contract
            generation = subprocess.run([str(binary), 'service-generate', *args, '--output', str(dest)],
                                        capture_output=True, check=True)
            evidence[f'python/{world}/{schema}/{contract}'] = {
                'check': check.stdout.decode(), 'check_stderr': check.stderr.decode(),
                'generation': generation.stdout.decode().replace(str(dest), '<OUTPUT>'),
                'generation_stderr': generation.stderr.decode(),
                'files': {str(f.relative_to(dest)): hashlib.sha256(f.read_bytes()).hexdigest()
                          for f in sorted(dest.rglob('*')) if f.is_file()},
            }
    return evidence


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    with TemporaryDirectory(prefix='py004-parity-') as temp:
        results = {label: snapshot(Path(binary).resolve(), Path(temp) / label)
                   for label, binary in zip(('before', 'after'), sys.argv[1:3])}
    Path(sys.argv[3]).write_text(json.dumps(results, indent=2) + '\n', encoding='utf-8')
    if results['before'] != results['after']:
        raise RuntimeError('model/API/report byte parity mismatch')
    print('PY004 BASELINE PARITY: PASSED (Ada/Rust/C++ 6 full models, 12 selected model/API runs, '
          '12 readiness reports, 2 coverage reports; Python 8 model/API runs and readiness reports)')
    print('Canonical candidate evidence SHA256: ' +
          hashlib.sha256(json.dumps(results['after'], sort_keys=True).encode()).hexdigest())


if __name__ == '__main__':
    main()