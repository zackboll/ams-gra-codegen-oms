"""Independent lexical oracle and actual generated model/API/codec execution."""
import dataclasses
import importlib
import itertools
import json
from pathlib import Path
import re
import sys
import unittest

FIXTURE = Path(__file__).resolve().parent
generated = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(generated if sys.argv[2] == 'standalone' else generated.parent))
prefix = '' if sys.argv[2] == 'standalone' else generated.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
importlib.import_module(prefix + 'service_api')
sys.argv[1:] = []

# Test-only independent regex oracle, intentionally unlike the production scanner.
ORACLE = re.compile(r'-?P(?:[0-9]+Y)?(?:[0-9]+M)?(?:[0-9]+D)?'
                    r'(?:T(?:[0-9]+H)?(?:[0-9]+M)?(?:[0-9]+(?:\.[0-9]+)?S)?)?')


def collapse(text):
    return re.sub('[ \t\r\n]+', ' ', text).strip(' ')


def oracle(text):
    text = collapse(text)
    return bool(ORACLE.fullmatch(text) and text not in ('P', '-P') and not text.endswith('T'))


def unescape(text):
    return re.sub(r'\\(u\{[0-9A-Fa-f]+\}|[stnr\\])',
                  lambda match: chr(int(match[1][2:-1], 16)) if match[1].startswith('u{')
                  else {'s': ' ', 't': '\t', 'n': '\n', 'r': '\r', '\\': '\\'}[match[1]], text)


def sample():
    return m.Payload(inherited='P12M', direct='PT36H', named=m.Span('-P0D'),
                     values=(m.Span('PT1.2300S'),), nested=m.Nested(value=m.Span('P1Y')),
                     namedunbounded=(m.Span('P0D'),))


class DurationTests(unittest.TestCase):
    def test_frozen_task057_corpus(self):
        counts = [0, 0]
        for line in (FIXTURE.parent / 'temporal/duration.txt').read_text().splitlines():
            if not line or line.startswith('#'):
                continue
            valid = line.startswith('VALID ')
            counts[0 if valid else 1] += 1
            text, expected = map(unescape, line[6:].split(' => ')) if valid else (unescape(line[8:]), None)
            self.assertEqual(oracle(text), valid, line)
            if valid:
                self.assertEqual(m.Span(text).value, expected)
                self.assertEqual(dataclasses.replace(sample(), direct=text).direct, expected)
                self.assertEqual(c._decode_Span(text, '$').value, expected)
                self.assertEqual(c._encode_Span(m.Span(text), '$'), expected)
            else:
                with self.assertRaises(ValueError, msg=line):
                    m.Span(text)
                with self.assertRaises(ValueError, msg=line):
                    dataclasses.replace(sample(), direct=text)
                with self.assertRaises(c.CodecError, msg=line):
                    c._decode_Span(text, '$')
        self.assertEqual(counts, [33, 67])

    def test_independent_controls(self):
        candidates = set()
        components = ('1Y', '2M', '3D', 'T', '4H', '5M', '6.00S')
        for size in range(4):
            candidates.update('P' + ''.join(parts) for parts in itertools.product(components, repeat=size))
        candidates.update(['P１D', 'PT١S', '\u00a0P1D', 'P1D\x00', 'P1D\ud800',
                           '\vP1D', '\fP1D', 'PT1e9S', '-PT0.000S', ' P1D\r\n'])
        for text in candidates:
            try:
                actual = m.Span(text).value
            except ValueError:
                self.assertFalse(oracle(text), repr(text))
            else:
                self.assertTrue(oracle(text), repr(text))
                self.assertEqual(actual, collapse(text))

    def test_long_components_and_lexical_identity(self):
        for text in ['P' + '9' * 8192 + 'Y', 'PT' + '0' * 8192 + '.' + '9' * 8192 + 'S',
                     '-P0D', '-PT0.000S', 'PT1.2300S', 'P12M', 'P1Y']:
            value = dataclasses.replace(sample(), direct=text, named=m.Span(text), unbounded=(text,))
            wire = json.loads(c.encode_Payload(value))
            self.assertEqual(wire['Direct'], text)
            self.assertEqual(wire['Named'], text)
            self.assertEqual(c.decode_Payload(json.dumps(wire)), value)
        self.assertNotEqual(m.Span('P12M'), m.Span('P1Y'))
        with self.assertRaises(TypeError):
            m.Span('P1Y') < m.Span('P12M')

    def test_shapes_and_literal_json(self):
        value = dataclasses.replace(sample(), optional=' \t-PT0.00S\r',
            namedoptional=m.Span(' P1D '), restricted=m.Restricted('PT0S'),
            unbounded=('P0D', '-PT0.000S'),
            nested=m.Nested(value=m.Span('PT1.200S'), directs=(' P1D ', 'PT2H')))
        wire = json.loads(c.encode_Payload(value))
        self.assertEqual(wire, {'Inherited': 'P12M', 'Direct': 'PT36H', 'Named': '-P0D',
            'Values': ['PT1.2300S'], 'NamedUnbounded': ['P0D'],
            'Optional': '-PT0.00S', 'NamedOptional': 'P1D', 'Restricted': 'PT0S',
            'Unbounded': ['P0D', '-PT0.000S'],
            'Nested': {'Value': 'PT1.200S', 'Directs': ['P1D', 'PT2H']}})
        self.assertEqual(c.decode_Payload(json.dumps(wire)), value)
        self.assertNotIn('Optional', json.loads(c.encode_Payload(sample())))

    def test_forged_storage_and_exact_carriers(self):
        class Text(str):
            pass
        class Span(m.Span):
            pass
        for bad in ['P', None, 1, Text('P0D')]:
            carrier = m.Span('P0D')
            object.__setattr__(carrier, 'value', bad)
            with self.assertRaises(c.CodecError):
                c._encode_Span(carrier, '$')
            value = sample()
            object.__setattr__(value, 'direct', bad)
            with self.assertRaisesRegex(c.CodecError, r'\$\.Direct:'):
                c.encode_Payload(value)
        with self.assertRaises(c.CodecError):
            c._encode_Span(object.__new__(m.Span), '$')
        with self.assertRaises(c.CodecError):
            c._encode_Span(Span('P0D'), '$')
        carrier = m.Span('P0D')
        object.__setattr__(carrier, 'value', ' \t-P0D\r\n')
        self.assertEqual(c._encode_Span(carrier, '$'), '-P0D')

    def test_indexed_ingress_and_egress_errors(self):
        for key, field, good in [('Values', 'values', m.Span('P0D')),
                                 ('Unbounded', 'unbounded', 'P0D'),
                                 ('NamedUnbounded', 'namedunbounded', m.Span('P0D'))]:
            wire = json.loads(c.encode_Payload(sample()))
            wire[key] = ['P0D', 'PT']
            with self.assertRaisesRegex(c.CodecError, r'\$\.' + key + r'\[1\]:'):
                c.decode_Payload(json.dumps(wire))
            value = sample()
            object.__setattr__(value, field, (good, 'PT'))
            with self.assertRaisesRegex(c.CodecError, r'\$\.' + key + r'\[1\]:'):
                c.encode_Payload(value)

    def test_strict_json_and_types(self):
        for text in ['null', 'true', '1', '[]', '{}']:
            with self.assertRaises(c.CodecError):
                c._decode_Span(json.loads(text), '$')
        wire = json.loads(c.encode_Payload(sample()))
        for key in ['Optional', 'NamedOptional']:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(json.dumps({**wire, key: None}))
        for text in ['{"Direct":"P0D","Direct":"P1D"}',
                     json.dumps({**wire, 'Unknown': 'P0D'}),
                     json.dumps({**wire, '$type': 'Payload'}),
                     json.dumps({**wire, 'Direct': '\ud800'})]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(text)

    def test_inherited_nested_and_normalized_decode(self):
        wire = json.loads(c.encode_Payload(sample()))
        wire['Inherited'] = '\t P12M \n'
        wire['Nested']['Directs'] = ['P1D', ' P ']
        with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Directs\[1\]:'):
            c.decode_Payload(json.dumps(wire))
        wire['Nested']['Directs'] = ['P1D', ' PT0.000S ']
        value = c.decode_Payload(json.dumps(wire))
        self.assertEqual(value.inherited, 'P12M')
        self.assertEqual(value.nested.directs, ('P1D', 'PT0.000S'))


if __name__ == '__main__':
    unittest.main()