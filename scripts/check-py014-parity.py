#!/usr/bin/env python3
"""Duration-free byte parity against verified main, including PY-012 repetition."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before-generator', required=True, type=Path)
    parser.add_argument('--generator', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    schemas = []
    for fixture in ['python-service', 'python-codec', 'python-header-profiles',
                    'python-inheritance', 'python-float64', 'python-datetime',
                    'python-hexbinary', 'python-visible-ascii', 'python-nato-special-words', 'python-unbounded-repetition']:
        path = ROOT / 'tests/fixtures' / fixture / 'oam.xsd'
        schemas.append(path if path.exists() else path.with_name('root.xsd'))
    schemas.append(ROOT / 'tests/fixtures/python-unbounded-repetition/sleet.xsd')
    rows = []
    for schema in schemas:
        for world in ['closed-schema', 'open-extensions']:
            for codec in [False, True]:
                artifacts = []
                for stage, generator in [('before', args.before_generator), ('after', args.generator)]:
                    dest = args.output / f'{schema.parent.name}-{schema.stem}-{world}-{codec}-{stage}'
                    subprocess.run([str(generator.resolve()), 'service-generate', '--schema', str(schema),
                        '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                        '--world', world, '--output', str(dest)] + (['--with-codec'] if codec else []),
                        capture_output=True, text=True, check=True)
                    artifacts.append({p.name: p.read_bytes() for p in dest.glob('*.py')})
                if artifacts[0] != artifacts[1]:
                    raise RuntimeError(f'Duration-free bytes changed: {schema} {world} {codec}')
                if any(b'def _checked_duration' in source or b'def _check_duration' in source
                       for source in artifacts[1].values()):
                    raise RuntimeError('Duration helpers leaked into a Duration-free schema')
                rows.append(dict(schema=str(schema.relative_to(ROOT)), world=world, codec=codec,
                                 files={name: hashlib.sha256(source).hexdigest()
                                        for name, source in sorted(artifacts[1].items())}))
    (args.output / 'parity.json').write_text(json.dumps(rows, indent=2) + '\n')
    print(f'PY014 DURATION-FREE BYTE PARITY: PASSED selections={len(rows)} sources={sum(len(r["files"]) for r in rows)}')


if __name__ == '__main__':
    main()