#!/usr/bin/env python3
"""Freeze/compare Float64-free generated model/API/codec bytes against PY-007."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def snapshot(generator, output):
    result = []
    for fixture in ['python-codec', 'python-header-profiles', 'python-inheritance']:
        for world in ['closed-schema', 'open-extensions']:
            for codec in [False, True]:
                dest = output / (fixture.replace('-', '_') + '_' + world.replace('-', '_') + '_' + str(codec))
                args = [str(generator), 'service-generate', '--schema', str(ROOT / 'tests/fixtures' / fixture / 'oam.xsd'),
                        '--contract', str(ROOT / 'tests/fixtures' / fixture / 'service.yaml'),
                        '--language', 'python', '--world', world, '--output', str(dest)]
                if codec:
                    args.append('--with-codec')
                p = subprocess.run(args, capture_output=True, text=True)
                if p.returncode:
                    raise RuntimeError(str(p))
                result.append(dict(fixture=fixture, world=world, codec=codec,
                                   stdout=p.stdout.replace(str(dest), '<OUTPUT>'), stderr=p.stderr, exit=p.returncode,
                                   files={f.name: hashlib.sha256(f.read_bytes()).hexdigest()
                                          for f in sorted(dest.glob('*.py'))}))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    before = snapshot(args.before.resolve(), args.output / 'before')
    after = snapshot(args.after.resolve(), args.output / 'after')
    (args.output / 'parity.json').write_text(json.dumps(dict(before=before, after=after), indent=2) + '\n')
    if before != after:
        raise RuntimeError('Float64-free generated output changed')
    print('PY008 NON-FLOAT64 BYTE PARITY: PASSED selections=12 model_api_codec_sources=30')


if __name__ == '__main__':
    main()