#!/usr/bin/env python3
"""Independent Task 096 oracle versus actual generated Python Float64 codecs.

Regenerate the corpus separately with tests/ada-float64-probe/oracle.py. This
runner requires its frozen manifest and both TSV hashes, and never modifies
the process-global integer digit limit (including when importing the oracle).
"""
import argparse
import hashlib
import importlib
import json
from pathlib import Path
import struct
import sys

ROOT = Path(__file__).resolve().parents[1]


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def bits(value):
    return struct.pack('>d', value).hex().upper()


def independent_oracle():
    # The historical oracle sets a global limit on import. Suppress only that
    # setter in a private execution namespace: no mutation of the process limit.
    # All numeric cases here fit the default limit; huge exponent tokens are
    # classified by the oracle before converting their digits.
    path = ROOT / 'tests/ada-float64-probe/oracle.py'
    source = path.read_text().replace('sys.set_int_max_str_digits(MAX_TOKEN * 2)',
                                      '# PY008 harness: no process-global mutation')
    namespace = {'__name__': 'py008_independent_oracle', '__file__': str(path)}
    exec(compile(source, str(path), 'exec'), namespace)
    return namespace['oracle']


def qualify(generated, corpus, full):
    frozen = json.loads((ROOT / 'tests/ada-float64-probe/corpus-manifest.json').read_text())
    require(json.loads((corpus / 'manifest.json').read_text()) == frozen, 'corpus manifest mismatch')
    for name, digest in frozen['hashes'].items():
        require(hashlib.sha256((corpus / name).read_bytes()).hexdigest() == digest,
                'unverified corpus ' + name)
    sys.path.insert(0, str(generated))
    m = importlib.import_module('model')
    c = importlib.import_module('service_codec')
    oracle = independent_oracle()
    original_limit = sys.get_int_max_str_digits()
    conversions = encodings = overflows = 0
    groups = set()
    for line in (corpus / 'conversions.tsv').read_text().splitlines():
        group, expected, token = line.split('\t')
        if not full and group == 'all-exponent-exact':
            continue
        if expected == 'OVERFLOW':
            try:
                c.decode_Scalar(token)
            except c.CodecError:
                overflows += 1
            else:
                raise RuntimeError('overflow was accepted: ' + token)
        else:
            require(bits(c.decode_Scalar(token).value) == expected, 'decoder mismatch: ' + token)
        conversions += 1
        groups.add(group)
    for index, line in enumerate((corpus / 'encoding.tsv').read_text().splitlines()):
        if not full and index % 127:
            continue
        value = struct.unpack('>d', bytes.fromhex(line))[0]
        token = c.encode_Scalar(m.Scalar(value))
        require(not token.startswith('"'), 'finite encoding is not a JSON number')
        require(oracle(token) == line, 'independent encoding mismatch: ' + line + ' ' + token)
        encodings += 1
    # Controls deliberately introduce four distinct errors. Expected values come
    # from independent integer/rational oracle logic, not the generated decoder.
    controls = {}
    corruptions = [
        ('exponent-rounding', lambda: oracle('1e-323'), '0000000000000001'),
        ('signed-zero-normalization', lambda: oracle('-0'), bits(0.0)),
        ('ties-to-even', lambda: oracle('1.00000000000000011102230246251565404236316680908203125'),
         '3FF0000000000001'),
        ('special-string', lambda: '"Infinity"', '"inf"'),
    ]
    for name, independent_expected, corrupted in corruptions:
        try:
            require(corrupted == independent_expected(), 'deliberately corrupted ' + name)
        except RuntimeError:
            controls[name] = True
        else:
            controls[name] = False
    require(c.encode_Scalar(m.Scalar(float('inf'))) == '"Infinity"', 'special mapping mismatch')
    require(all(controls.values()), 'numerical negative control failed')
    require(conversions == (27167 if full else 6693), 'missing decoder cases')
    require(encodings == (20474 if full else 162), 'missing encoder cases')
    require(sys.get_int_max_str_digits() == original_limit, 'integer digit limit changed')
    return dict(completed=True, python=sys.version, optimized=not __debug__, full=full,
                conversions=conversions, overflow_rejections=overflows, encodings=encodings,
                mismatches=0, controls=controls, groups=sorted(groups),
                corpus=frozen, generated_sha256={name: hashlib.sha256((generated / name).read_bytes()).hexdigest()
                                                for name in ['model.py', 'service_api.py', 'service_codec.py']})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generated', type=Path, required=True)
    parser.add_argument('--corpus', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--full', action='store_true')
    args = parser.parse_args()
    result = qualify(args.generated.resolve(), args.corpus.resolve(), args.full)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print('PY008 INDEPENDENT NUMERIC: PASSED conversions={conversions} encodings={encodings} mismatches=0'.format(**result))


if __name__ == '__main__':
    main()