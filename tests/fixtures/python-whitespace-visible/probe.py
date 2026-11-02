"""Executable synthetic profile checks; assertions remain active under -O."""
import dataclasses
import importlib
import json
from pathlib import Path
import sys
import unittest

directory = Path(sys.argv.pop(1)).resolve()
style = sys.argv.pop(1)
if style == 'package':
    sys.path.insert(0, str(directory.parent))
    prefix = directory.name + '.'
else:
    sys.path.insert(0, str(directory))
    prefix = ''
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if (directory / 'service_codec.py').exists() else None
namespace = sys.argv.pop(1) if len(sys.argv) > 1 else ''


def key(name):
    return ('{' + namespace + '}' if namespace else '') + name


class WhitespaceVisible(unittest.TestCase):
    def test_profiles_and_boundaries(self):
        for carrier, lower, upper, collapse in [
            (m.FoldShort, 0, 1024, True), (m.FoldLong, 0, 4096, True),
            (m.KeepEmpty, 0, 4096, False), (m.KeepShort, 1, 1024, False),
            (m.KeepLong, 1, 4096, False), (m.Derived, 0, 1024, True),
            (m.Leaf, 0, 1024, True),
        ]:
            for size in [0, 1, 1024, 4096, upper + 1]:
                with self.subTest(carrier=carrier, size=size):
                    if lower <= size <= upper:
                        self.assertEqual(carrier('x' * size).value, 'x' * size)
                    else:
                        with self.assertRaises(ValueError):
                            carrier('x' * size)
            original = '  a \t b\n' if collapse else ' a\n\r  b '
            self.assertEqual(carrier(original).value, 'a b' if collapse else original)
            if collapse:
                self.assertEqual(carrier('\r\n\t').value, '')
                self.assertEqual(carrier(' \t\r\n' * 4000 + 'a' + '\n' * 4000 + 'b ').value, 'a b')
            else:
                with self.assertRaises(ValueError):
                    carrier('a\tb')
            class Sub(str):
                pass
            for invalid in [Sub('x'), None, 1, True, b'x', ['x']]:
                with self.assertRaises(TypeError):
                    carrier(invalid)
            with self.assertRaises(TypeError):
                carrier()
            value = carrier('x')
            self.assertFalse(hasattr(value, '__dict__'))
            with self.assertRaises(dataclasses.FrozenInstanceError):
                value.value = 'y'

    def test_every_character(self):
        for carrier in [m.FoldShort, m.KeepShort]:
            for point in list(range(256)) + [0x2003, 0x2028, 0xd800, 0xdfff, 0x10000, 0x10ffff]:
                character = chr(point)
                with self.subTest(carrier=carrier, point=point):
                    if 32 <= point <= 126 or point in (10, 13) or (carrier is m.FoldShort and point == 9):
                        self.assertIs(type(carrier('a' + character + 'b').value), str)
                    else:
                        with self.assertRaises(ValueError):
                            carrier('a' + character + 'b')
        self.assertEqual(m.KeepShort(r'\n\r').value, r'\n\r')

    def payload(self):
        return m.Payload(required=m.FoldShort(' a\t b '), preserved=m.KeepShort(' a\n\r  b '),
                         optional=m.KeepEmpty(''), inherited=(m.FoldLong(''),),
                         bounded=(m.KeepLong('a\nb'),), positive=(m.Leaf(''),),
                         detail=m.Detail(text=m.KeepEmpty('')),
                         repeated=(m.Outer.Nested(m.Selection.Fold(m.Leaf(' x '))),
                                   m.Outer.Nested(m.Selection.Record(m.Detail(text=m.KeepEmpty(' y '))))))

    def test_recursive_choices(self):
        self.assertEqual(self.payload().required.value, 'a b')
        self.assertIsNone(m.Payload(required=m.FoldShort(''), preserved=m.KeepShort('x'), positive=(m.Leaf(''),)).optional)
        for invalid in ['', None, 7, ' x ', '\x00']:
            broken = m.Leaf('x')
            object.__setattr__(broken, 'value', invalid)
            if invalid == '':
                self.assertEqual(m.Selection.Fold(broken).value.value, '')
            else:
                with self.assertRaises((TypeError, ValueError)):
                    m.Selection.Fold(broken)
        missing = object.__new__(m.Leaf)
        with self.assertRaises(AttributeError):
            m.Selection.Fold(missing)
        outer = m.Outer.Nested(m.Selection.Fold(m.Leaf('x')))
        object.__setattr__(outer.value.value, 'value', '\x00')
        with self.assertRaises(ValueError):
            m.Payload(required=m.FoldShort(''), preserved=m.KeepShort('x'), positive=(m.Leaf(''),), repeated=(outer,))

    def test_codec(self):
        if c is None:
            self.assertFalse((directory / 'service_codec.py').exists())
            return
        payload = self.payload()
        wire = c.encode_Payload(payload)
        document = json.loads(wire)
        self.assertEqual(document[key('Required')], 'a b')
        self.assertEqual(document[key('Preserved')], ' a\n\r  b ')
        self.assertEqual(document[key('Optional')], '')
        self.assertEqual(c.decode_Payload(wire), payload)
        self.assertEqual(c.encode_Payload(c.decode_Payload(wire)), wire)
        document[key('Required')] = ' \t' * 4000 + 'a \n b '
        self.assertEqual(c.decode_Payload(json.dumps(document)).required.value, 'a b')
        for name, invalid in [('Required', '\x00'), ('Preserved', ''), ('Preserved', '\t'),
                              ('Preserved', '\u00a0'), ('Required', None), ('Required', 1),
                              ('Required', []), ('Required', '\ud800')]:
            document = json.loads(wire)
            document[key(name)] = invalid
            with self.assertRaises(c.CodecError) as caught:
                c.decode_Payload(json.dumps(document))
            self.assertIn('$.' + key(name) + ':', str(caught.exception))
        for name, invalid in [('Required', ' x '), ('Preserved', ''), ('Required', 1), ('Required', '\x7f')]:
            payload = self.payload()
            object.__setattr__(getattr(payload, name.lower()), 'value', invalid)
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(payload)
            self.assertIn('$.' + key(name) + ':', str(caught.exception))
        payload = self.payload()
        object.__delattr__(payload.required, 'value')
        with self.assertRaises(c.CodecError):
            c.encode_Payload(payload)
        class TextSubclass(str):
            pass
        with self.assertRaises(c.CodecError):
            c._decode_FoldShort(TextSubclass('x'), '$.Required')
        payload = self.payload()
        object.__setattr__(payload.required, 'value', TextSubclass('x'))
        with self.assertRaises(c.CodecError):
            c.encode_Payload(payload)
        class Sub(m.FoldShort):
            pass
        payload = self.payload()
        object.__setattr__(payload, 'required', Sub('x'))
        with self.assertRaises(c.CodecError):
            c.encode_Payload(payload)

    def test_codec_repeated_paths_and_strict_parser(self):
        if c is None:
            self.assertFalse((directory / 'service_codec.py').exists())
            return
        for index in range(3):
            document = json.loads(c.encode_Payload(self.payload()))
            document[key('Repeated')] = [{key('Nested'): {key('Fold'): 'x'}} for _ in range(3)]
            document[key('Repeated')][index][key('Nested')][key('Fold')] = '\x00'
            path = '$.' + key('Repeated') + f'[{index}].' + key('Nested') + '.' + key('Fold') + ':'
            with self.assertRaises(c.CodecError) as caught:
                c.decode_Payload(json.dumps(document))
            self.assertIn(path, str(caught.exception))
            values = [m.Outer.Nested(m.Selection.Fold(m.Leaf('x'))) for _ in range(3)]
            object.__setattr__(values[index].value.value, 'value', ' x ')
            payload = self.payload()
            object.__setattr__(payload, 'repeated', tuple(values))
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(payload)
            self.assertIn(path, str(caught.exception))
        for invalid in ['null', '[]', '{}', '{"Required":"x","Required":"y"}',
                        '{"Unknown":"x"}', '{"Required":NaN}']:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(invalid)


if __name__ == '__main__':
    unittest.main()