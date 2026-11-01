"""Independent octet/hexadecimal oracles; unittest remains active under -O."""
import importlib
import json
from pathlib import Path
import sys
import unittest

dest = Path(sys.argv[1])
style, mode = sys.argv[2:4]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if mode == 'codec' else None
NS = '' if len(sys.argv) < 5 else '{' + sys.argv[4] + '}'
ALL_HEX = (
    '000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F'
    '202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F'
    '404142434445464748494A4B4C4D4E4F505152535455565758595A5B5C5D5E5F'
    '606162636465666768696A6B6C6D6E6F707172737475767778797A7B7C7D7E7F'
    '808182838485868788898A8B8C8D8E8F909192939495969798999A9B9C9D9E9F'
    'A0A1A2A3A4A5A6A7A8A9AAABACADAEAFB0B1B2B3B4B5B6B7B8B9BABBBCBDBEBF'
    'C0C1C2C3C4C5C6C7C8C9CACBCCCDCECFD0D1D2D3D4D5D6D7D8D9DADBDCDDDEDF'
    'E0E1E2E3E4E5E6E7E8E9EAEBECEDEEEFF0F1F2F3F4F5F6F7F8F9FAFBFCFDFEFF'
)


def wire(**fields):
    return json.dumps({NS + key: value for key, value in fields.items()}, separators=(',', ':'))


class BytesSubclass(bytes):
    pass


class CarrierSubclass(m.HexBinaryType):
    pass


class TupleSubclass(tuple):
    pass


class Binary(unittest.TestCase):
    def test_models_exact_types_and_storage(self):
        for value in [b'', b'\x00', b'\x00\x0a\xee\xff', bytes(range(256))]:
            self.assertIs(m.HexBinaryType(value).value, value)
            self.assertIs(m.Payload(required=value).required, value)
        for bad in [bytearray(b'a'), memoryview(b'a'), [0], '00', BytesSubclass(b'a'), 0, None]:
            with self.subTest(bad=type(bad)):
                with self.assertRaises(TypeError):
                    m.HexBinaryType(bad)
                with self.assertRaises(TypeError):
                    m.Payload(required=bad)
                with self.assertRaises(TypeError):
                    m.Payload(required=b'', bounded=(b'', bad, b''))
        carrier = m.HexBinaryType(b'abc')
        self.assertFalse(hasattr(carrier, '__dict__'))
        with self.assertRaises(AttributeError):
            carrier.value = b'other'

    def test_octet_domains_and_restriction_chain(self):
        for name, good, bad in [
            ('Exact', [2], [0, 1, 3]), ('IFF_RegisterType', [2], [0, 1, 3]),
            ('Minimum', [1, 2, 20], [0]), ('Maximum', [0, 1, 2], [3]),
            ('Interval', [1, 2, 3], [0, 4]), ('Zero', [0], [1]),
        ]:
            cls = getattr(m, name)
            for n in good:
                self.assertEqual(cls(b'\xff' * n).value, b'\xff' * n)
            for n in bad:
                with self.assertRaises(ValueError):
                    cls(b'\xff' * n)
        self.assertEqual(m.Payload(required=b'').inherited, ())
        with self.assertRaises(ValueError):
            m.Child(data=())
        with self.assertRaises(ValueError):
            m.Payload(required=b'', bounded=(b'',) * 4)
        with self.assertRaises(TypeError):
            m.Payload(required=b'', bounded=[b''])
        with self.assertRaises(TypeError):
            m.Payload(required=b'', bounded=TupleSubclass((b'',)))

    @unittest.skipIf(c is None, 'model-only selection')
    def test_independent_hex_vectors(self):
        vectors = [(b'', ''), (b'\x00', '00'), (b'\x0a', '0A'), (b'\xee', 'EE'),
                   (b'\x00\x0a\xee\xff', '000AEEFF'), (bytes(range(256)), ALL_HEX)]
        for octets, expected in vectors:
            self.assertEqual(c.encode_Payload(m.Payload(required=octets)), wire(Required=expected))
            self.assertEqual(c.decode_Payload(wire(Required=expected.lower())).required, octets)
        self.assertEqual(c.encode_Payload(c.decode_Payload(wire(Required='aAbBcCdDeEfF'))),
                         wire(Required='AABBCCDDEEFF'))
        self.assertEqual(len(ALL_HEX), 512)

    @unittest.skipIf(c is None, 'model-only selection')
    def test_whitespace_and_invalid_lexical_forms(self):
        for text in ['', ' \t\r\n', '\n00aAff\r\t ', '00AaFF']:
            expected = b'' if not text.strip(' \t\r\n') else b'\x00\xaa\xff'
            self.assertEqual(c.decode_Payload(wire(Required=text)).required, expected)
        for whitespace in [' ', '\t', '\r', '\n']:
            self.assertEqual(c.decode_Payload(wire(Required=whitespace + '00' + whitespace)).required, b'\0')
            with self.assertRaises(c.CodecError):
                c.decode_Payload(wire(Required='00' + whitespace + 'FF'))
        for bad in ['0', 'AAA', '0x00', '00 00', '0 0', '00\t00', '00\r\nAA',
                    '00-00', 'GG', 'éé', '１２', '٠٠', '\u00a000', '00\u00a0',
                    '\v00', '\f00', False, 0, [], {}, None]:
            with self.subTest(bad=bad), self.assertRaises(c.CodecError) as caught:
                c.decode_Payload(wire(Required=bad))
            self.assertIn('$.' + NS + 'Required', str(caught.exception))

    @unittest.skipIf(c is None, 'model-only selection')
    def test_named_validation_both_directions(self):
        value = m.Payload(required=b'', named=m.HexBinaryType(b'\xff'),
                          min=m.Minimum(b'\x00'), max=m.Maximum(b''),
                          range=m.Interval(b'\x80\xff'), empty=m.Zero(b''))
        expected = wire(Required='', Named='FF', Min='00', Max='', Range='80FF', Empty='')
        self.assertEqual(c.encode_Payload(value), expected)
        self.assertEqual(c.decode_Payload(expected), value)
        value = m.Payload(required=b'', named=m.HexBinaryType(b''))
        object.__setattr__(value, 'named', CarrierSubclass(b''))
        with self.assertRaises(c.CodecError):
            c.encode_Payload(value)
        for bad in [bytearray(b'a'), memoryview(b'a'), [0], '00', BytesSubclass(b'a'), 0, None]:
            value = m.Payload(required=b'')
            object.__setattr__(value, 'required', bad)
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(value)
            self.assertIn('$.' + NS + 'Required', str(caught.exception))
        for key, text in [('Min', ''), ('Max', '000000'), ('Range', ''), ('Empty', '00')]:
            with self.assertRaises(c.CodecError) as caught:
                c.decode_Payload(wire(Required='', **{key: text}))
            self.assertIn('$.' + NS + key, str(caught.exception))
        for cls in [m.HexBinaryType, m.IFF_RegisterType]:
            for bad in [None, '00', bytearray(b'ab'), BytesSubclass(b'ab'), b'x']:
                if cls is m.HexBinaryType and type(bad) is bytes:
                    continue
                forged = object.__new__(cls)
                if bad is not None:
                    object.__setattr__(forged, 'value', bad)
                value = m.Payload(required=b'', **({'named': forged} if cls is m.HexBinaryType
                                                 else {'many': (forged,)}))
                with self.assertRaises(c.CodecError) as caught:
                    c.encode_Payload(value)
                self.assertIn('$.' + NS + ('Named' if cls is m.HexBinaryType else 'Many[0]'),
                              str(caught.exception))

    @unittest.skipIf(c is None, 'model-only selection')
    def test_repetition_nested_and_inherited_paths(self):
        value = m.Payload(required=b'\x80\xff', optional=b'', bounded=(b'', b'\x0a'),
                          many=(m.IFF_RegisterType(b'\x00\xff'),) * 3,
                          inherited=(b'\xaa', b''),
                          groups=(m.Child(data=(m.IFF_RegisterType(b'\x80\xff'),)),))
        expected = wire(Inherited=['AA', ''], Required='80FF', Optional='', Bounded=['', '0A'],
                        Many=['00FF'] * 3, Groups=[{NS + 'Data': ['80FF']}])
        self.assertEqual(c.encode_Payload(value), expected)
        self.assertEqual(c.decode_Payload(expected), value)
        for key in ['Bounded', 'Inherited', 'Many']:
            for index in range(3):
                items = ['00FF'] * 3
                items[index] = 'bad'
                with self.assertRaises(c.CodecError) as caught:
                    c.decode_Payload(wire(Required='', **{key: items}))
                self.assertIn('$.' + NS + key + f'[{index}]', str(caught.exception))
                value = m.Payload(required=b'')
                values = [b'ab'] * 3 if key != 'Many' else [m.IFF_RegisterType(b'ab')] * 3
                values[index] = 'bad'
                object.__setattr__(value, key.lower(), tuple(values))
                with self.assertRaises(c.CodecError) as caught:
                    c.encode_Payload(value)
                self.assertIn('$.' + NS + key + f'[{index}]', str(caught.exception))
        with self.assertRaises(c.CodecError) as caught:
            c.decode_Payload(wire(Required='', Groups=[{NS + 'Data': ['00FF', '00']}]))
        self.assertIn('$.' + NS + 'Groups[0].' + NS + 'Data[1]', str(caught.exception))


unittest.main(argv=[sys.argv[0]])