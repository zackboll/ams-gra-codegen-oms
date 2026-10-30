"""Execute actual generated sources. unittest checks remain active under -O."""
import dataclasses
import importlib
import json
from pathlib import Path
import sys
import unittest

FIXTURE = Path(__file__).resolve().parent
generated = Path(sys.argv[1]).resolve()
style = sys.argv[2]
sys.path.insert(0, str(generated if style == 'standalone' else generated.parent))
prefix = '' if style == 'standalone' else generated.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
importlib.import_module(prefix + 'service_api')
sys.argv[1:] = []
Z = '2024-02-29T00:00:00.123456789012345678900Z'
D = '2000-02-29T23:59:60+14:00'


def sample():
    return m.Payload(inherited='0001-01-01T00:00:00', direct=D, named=m.Instant(Z),
                     values=(m.Instant(Z),), nested=m.Nested(value=m.Instant(Z),
                     directs=(D, '-0004-02-29T24:00:00.000')))


class DateTimeTests(unittest.TestCase):
    def test_independent_corpus(self):
        cases = json.loads((FIXTURE / 'lexical.json').read_text())
        for value, expected in cases['valid']:
            with self.subTest(value=value):
                self.assertEqual(dataclasses.replace(sample(), direct=value).direct, expected)
                if expected.endswith('Z'):
                    self.assertEqual(m.Instant(value).value, expected)
                else:
                    with self.assertRaises(ValueError):
                        m.Instant(value)
        for value in cases['invalid']:
            with self.subTest(value=repr(value)):
                with self.assertRaises(ValueError):
                    dataclasses.replace(sample(), direct=value)
                with self.assertRaises(ValueError):
                    m.Instant(value)

    def test_shared_task036_046_contract(self):
        # Frozen independent corpus also drives compiled Ada/Rust/C++ tests.
        def unescape(value):
            for before, after in [('\\s', ' '), ('\\t', '\t'), ('\\n', '\n'), ('\\r', '\r'), ('\\\\', '\\')]:
                value = value.replace(before, after)
            return value
        count = 0
        for filename in ['datetime-zulu.txt', 'datetime-direct.txt']:
            for line in (FIXTURE.parent / 'temporal' / filename).read_text().splitlines():
                if not line or line.startswith('#'):
                    continue
                count += 1
                if line.startswith('VALID '):
                    value, expected = map(unescape, line[6:].split(' => '))
                    self.assertEqual(dataclasses.replace(sample(), direct=value).direct, expected)
                    if filename == 'datetime-zulu.txt':
                        self.assertEqual(m.Instant(value).value, expected)
                else:
                    value = unescape(line[8:] if line.startswith('INVALID ') else '')
                    if filename == 'datetime-zulu.txt':
                        with self.assertRaises(ValueError, msg=line):
                            m.Instant(value)
                    else:
                        with self.assertRaises(ValueError, msg=line):
                            dataclasses.replace(sample(), direct=value)
        self.assertGreater(count, 100)

    def test_unbounded_year_and_fraction(self):
        value = '1' + '0' * 5000 + '-02-29T23:59:60.' + '9' * 5000 + 'Z'
        self.assertEqual(m.Instant(value).value, value)
        self.assertEqual(c.decode_Payload(c.encode_Payload(dataclasses.replace(sample(), named=m.Instant(value)))).named.value, value)

    def test_normalized_storage_optional_repeated_inherited(self):
        p = dataclasses.replace(sample(), inherited='\t' + D + '\n', direct=' ' + D,
                                optional=Z + '\r', nested=m.Nested(value=m.Instant(' ' + Z), directs=(' ' + D,)))
        self.assertEqual((p.inherited, p.direct, p.optional, p.nested.directs), (D, D, Z, (D,)))
        self.assertIsNone(sample().optional)
        self.assertEqual(m.Restricted(' ' + Z).value, Z)
        with self.assertRaises(dataclasses.FrozenInstanceError):
            p.direct = Z
        self.assertFalse(hasattr(p, '__dict__'))
        self.assertFalse(hasattr(p.named, '__dict__'))

    def test_exact_types_and_cardinality(self):
        class Text(str):
            pass
        for bad in [None, True, 2024, 1.0, b'2024', Text(Z), [], {}]:
            with self.assertRaises(TypeError):
                m.Instant(bad)
            with self.assertRaises(TypeError):
                dataclasses.replace(sample(), direct=bad)
        for bad in [(), (m.Instant(Z),) * 4]:
            with self.assertRaises(ValueError):
                dataclasses.replace(sample(), values=bad)
        with self.assertRaises(TypeError):
            dataclasses.replace(sample(), values=[m.Instant(Z)])
        with self.assertRaises(ValueError):
            m.Nested(value=m.Instant(Z), directs=('invalid',))

    def test_independent_json_golden_and_order(self):
        golden = (FIXTURE / 'positive.json').read_text()
        self.assertEqual(c.decode_Payload(golden), sample())
        encoded = c.encode_Payload(sample())
        self.assertEqual(json.loads(encoded), json.loads(golden))
        self.assertEqual(list(json.loads(encoded)), ['Inherited', 'Direct', 'Named', 'Values', 'Nested'])
        self.assertIn(Z, encoded)
        self.assertEqual(c.decode_Payload(encoded), sample())

    def test_negative_json_paths(self):
        base = json.loads((FIXTURE / 'positive.json').read_text())
        for member, bad, path in json.loads((FIXTURE / 'negative.json').read_text()):
            with self.subTest(member=member, bad=bad):
                with self.assertRaises(c.CodecError) as caught:
                    c.decode_Payload(json.dumps({**base, member: bad}))
                self.assertIn(path + ':', str(caught.exception))
        for text in ['{}', 'null', '[]', '{"Named":1,"Named":2}',
                     json.dumps({**base, '{urn:wrong}Direct': D}),
                     json.dumps(base).replace('"Direct"', '"{https://www.vdl.afrl.af.mil/programs/oam}Direct"')]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(text)

    def test_outbound_forged_values(self):
        for attr, bad, path in [('direct', 'bad', '$.Direct'), ('optional', 3, '$.Optional')]:
            value = sample()
            object.__setattr__(value, attr, bad)
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(value)
            self.assertIn(path, str(caught.exception))
        for bad in ['badZ', D, 3]:
            value = sample()
            object.__setattr__(value.named, 'value', bad)
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(value)
            self.assertIn('$.Named', str(caught.exception))
        value = sample()
        object.__setattr__(value.nested, 'directs', (D, 'bad'))
        with self.assertRaises(c.CodecError) as caught:
            c.encode_Payload(value)
        self.assertIn('$.Nested.Directs[1]', str(caught.exception))


if __name__ == '__main__':
    unittest.main()
