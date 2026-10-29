"""Execute actual generated sources; unittest checks remain active under -O."""
import dataclasses
from decimal import Decimal
import importlib
import json
import math
from pathlib import Path
import py_compile
import struct
import sys
import unittest

dest = Path(sys.argv[1]).resolve()
style = sys.argv[2]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
a = importlib.import_module(prefix + 'service_api')


def bits(value):
    return struct.pack('>d', value).hex()


def literal(**changes):
    value = dict(Inherited=-0.0, Direct=1.0000000000000002, Named=5e-324,
                 Limited=1.25, Values=[-1.0, 1.5], Nested={'Value': 0.5},
                 Integer=123, Text='Infinity')
    value.update(changes)
    return json.dumps(value, allow_nan=False, separators=(',', ':'))


class Float64(unittest.TestCase):
    def test_literal_and_frozen_types(self):
        v = c.decode_Payload(literal())
        self.assertIs(type(v), m.Payload)
        self.assertIs(type(v.named), m.Scalar)
        self.assertIs(type(v.limited), m.Bounded)
        self.assertIs(type(v.nested), m.Nested)
        self.assertIs(type(v.direct), float)
        self.assertIs(type(v.values), tuple)
        self.assertEqual(bits(v.inherited), '8000000000000000')
        self.assertEqual(bits(v.named.value), '0000000000000001')
        self.assertEqual(v.text, 'Infinity')
        self.assertEqual(v.integer, 123)
        self.assertEqual(c.encode_Payload(v), literal())
        self.assertFalse(hasattr(v, '__dict__'))
        self.assertFalse(hasattr(m, 'Ancestor'))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            v.direct = 0.0

    def test_exact_types(self):
        class Subclass(float):
            pass
        for bad in [0, True, '1', Decimal('1'), Subclass(1), None]:
            with self.subTest(bad=bad), self.assertRaises(TypeError):
                m.Scalar(bad)
            v = c.decode_Payload(literal())
            object.__setattr__(v, 'direct', bad)
            with self.assertRaises(c.CodecError):
                c.encode_Payload(v)
        with self.assertRaises(TypeError):
            m.Nested(value=m.Scalar(0.5))

    def test_special_mapping(self):
        for text, value in [('NaN', float('nan')), ('Infinity', float('inf')),
                            ('-Infinity', -float('inf'))]:
            self.assertEqual(c.encode_Scalar(m.Scalar(value)), json.dumps(text))
            got = c.decode_Scalar(json.dumps(text)).value
            self.assertTrue(math.isnan(got) if text == 'NaN' else got == value)
        for bad in ['nan', 'inf', '+Infinity', '−Infinity', '1.25', '-0', 'NAN']:
            with self.subTest(bad=bad), self.assertRaises(c.CodecError):
                c.decode_Scalar(json.dumps(bad))
        for bad in ['NaN', 'Infinity', '-Infinity']:
            with self.assertRaises(c.CodecError):
                c.decode_Scalar(bad)

    def test_ieee_ranges(self):
        for zero in [0.0, -0.0]:
            m.Bounded(zero)
            m.Nonpositive(zero)
            with self.assertRaises(ValueError):
                m.Positive(zero)
        m.Positive(float('inf'))
        m.Nonpositive(-float('inf'))
        m.Positive(5e-324)
        m.Bounded(-1.0)
        for bad in [float('nan'), float('inf'), -float('inf'), 2.0, -1.0000000000000002]:
            with self.assertRaises(ValueError):
                m.Bounded(bad)
        for cls in [m.Positive, m.Nonpositive, m.Singleton, m.Restricted]:
            with self.assertRaises(ValueError):
                cls(float('nan'))
        m.Singleton(1.0000000000000002)
        for bad in [1.0, 1.0000000000000004]:
            with self.assertRaises(ValueError):
                m.Singleton(bad)
        m.Restricted(1.0)
        with self.assertRaises(ValueError):
            m.Restricted(1.5)

    def test_bound_bits(self):
        # Expected literal IR binary64 bits, not generated decimal display text.
        self.assertEqual(bits(m._float64_bound(0x3ff0000000000001)), '3ff0000000000001')
        self.assertEqual(bits(m.Singleton(1.0000000000000002).value), '3ff0000000000001')

    def test_numeric_grammar_rounding_and_overflow(self):
        for token, expected in [('-0', '8000000000000000'), ('-0e9999999', '8000000000000000'),
                                ('9007199254740993', '4340000000000000'),
                                ('1.00000000000000011102230246251565404236316680908203125', '3ff0000000000000'),
                                ('-1e-9999999999999999999', '8000000000000000')]:
            self.assertEqual(bits(c.decode_Scalar(token).value), expected)
        for token in ['01', '+1', '.1', '1.', '1e', '--0', '1e309', '-1e309', 'null', 'true', '{}', '[]']:
            with self.subTest(token=token), self.assertRaises(c.CodecError):
                c.decode_Scalar(token)

    def test_adjacent_integer_boundary(self):
        for token in ['1.0', '1e0', '-0.0']:
            with self.assertRaisesRegex(c.CodecError, r'\$\.Integer'):
                c.decode_Payload(literal().replace('"Integer":123', '"Integer":' + token))
        old_limit = sys.get_int_max_str_digits()
        token = '9' * 5000
        v = c.decode_Payload(literal().replace('"Integer":123', '"Integer":' + token))
        self.assertIs(type(v.integer), int)
        self.assertEqual(c.encode_Payload(v), literal().replace('"Integer":123', '"Integer":' + token))
        self.assertEqual(sys.get_int_max_str_digits(), old_limit)

    def test_paths_and_invalid_later_items(self):
        for value in [[0.0, '1.0'], [0.0, 2.0], [0.0, None]]:
            with self.assertRaisesRegex(c.CodecError, r'\$\.Values\[1\]'):
                c.decode_Payload(literal(Values=value))
        v = c.decode_Payload(literal())
        object.__setattr__(v.values[1], 'value', float('nan'))
        with self.assertRaisesRegex(c.CodecError, r'\$\.Values\[1\]'):
            c.encode_Payload(v)
        for value in [[], [0.0] * 4, None]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(literal(Values=value))

    def test_optional_and_nested(self):
        v = c.decode_Payload(literal(Optional=-0.0, Positive='Infinity', Nonpositive='-Infinity',
                                     Restricted=1.0, Singleton=1.0000000000000002))
        self.assertEqual(bits(v.optional), '8000000000000000')
        self.assertEqual(v.positive.value, float('inf'))
        self.assertEqual(v.nonpositive.value, -float('inf'))
        with self.assertRaisesRegex(c.CodecError, r'\$\.Nested.Value'):
            c.decode_Payload(literal(Nested={'Value': 2.0}))
        with self.assertRaises(c.CodecError):
            c.decode_Payload(literal(Optional=None))

    def test_unknown_missing_and_duplicate(self):
        for bad in [literal(Unknown=1), literal(**{'$type': 'Payload'}),
                    literal().replace('"Direct":1.0000000000000002,', ''),
                    literal().replace('"Direct":', '"Direct":0,"\\u0044irect":'),
                    literal().replace('"Value":0.5', '"Value":0.5,"\\u0056alue":1')]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(bad)

    def test_metadata_and_registration(self):
        for name in ['model.py', 'service_api.py', 'service_codec.py']:
            py_compile.compile(str(dest / name), doraise=True)
        self.assertEqual(m.Payload.__schema_name__, ('https://www.vdl.afrl.af.mil/programs/oam', 'Payload'))
        fields = dataclasses.fields(m.Payload)
        self.assertEqual(fields[0].name, 'inherited')
        self.assertEqual(dict(fields[0].metadata), dict(schema_name='Inherited', schema_namespace=m.Payload.__schema_name__[0]))
        # Sharing a payload must not duplicate generated codec operations.
        source = (dest / 'service_codec.py').read_text()
        self.assertEqual(source.count('\ndef decode_Payload('), 1)


if __name__ == '__main__':
    unittest.main(argv=['py008'], verbosity=2)