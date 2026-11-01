#!/usr/bin/env python3
"""Compare NATO-free generated model/API/codec bytes against verified main."""
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
    rows = []
    for fixture in ['python-service', 'python-codec', 'python-header-profiles',
                    'python-inheritance', 'python-float64', 'python-datetime', 'python-visible-ascii']:
        schema = ROOT / 'tests/fixtures' / fixture / 'oam.xsd'
        if not schema.exists():
            schema = schema.with_name('root.xsd')
        for world in ['closed-schema', 'open-extensions']:
            for codec in [False, True]:
                artifacts = []
                for stage, generator in [('before', args.before_generator), ('after', args.generator)]:
                    destination = args.output / f'{fixture}-{world}-{codec}-{stage}'
                    subprocess.run([str(generator.resolve()), 'service-generate', '--schema', str(schema),
                        '--contract', str(schema.with_name('service.yaml')), '--language', 'python',
                        '--world', world, '--output', str(destination)] + (['--with-codec'] if codec else []),
                        capture_output=True, text=True, check=True)
                    sources = {p.name: p.read_bytes() for p in destination.glob('*.py')}
                    if any(b'nato_special_words' in source for source in sources.values()):
                        raise RuntimeError('undemanded NATO helper')
                    artifacts.append(sources)
                if artifacts[0] != artifacts[1]:
                    raise RuntimeError(f'NATO-free bytes changed: {fixture} {world} {codec}')
                rows.append(dict(fixture=fixture, world=world, codec=codec,
                                 files={name: hashlib.sha256(source).hexdigest()
                                        for name, source in sorted(artifacts[1].items())}))
    (args.output / 'parity.json').write_text(json.dumps(rows, indent=2) + '\n')
    print(f'PY011 NATO-FREE BYTE PARITY: PASSED selections={len(rows)} sources={sum(len(r["files"]) for r in rows)}')


if __name__ == '__main__':
    main()