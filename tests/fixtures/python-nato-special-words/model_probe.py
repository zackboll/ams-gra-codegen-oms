"""Frozen Task 042 corpus plus independent, optimization-safe lexical checks."""
import dataclasses
import importlib
from pathlib import Path
import re
import string
import sys
import unittest

FIXTURE = Path(__file__).resolve().parent
path = Path(sys.argv[1]).resolve()
style = sys.argv[2]
sys.path.insert(0, str(path if style == 'standalone' else path.parent))
prefix = '' if style == 'standalone' else path.name + '.'
m = importlib.import_module(prefix + 'model')
importlib.import_module(prefix + 'service_api')
sys.argv[1:] = []
ALPHABET = string.ascii_uppercase + string.ascii_lowercase + '-_'


def cases():
    def unescape(text):
        if text == '<empty>':
            return ''
        escapes = {'s': ' ', 't': '\t', 'r': '\r', 'n': '\n', '\\': '\\'}
        return re.sub(r'\\(?:u\{([0-9a-fA-F]+)\}|([strn\\]))',
                      lambda match: chr(int(match[1], 16)) if match[1] else escapes[match[2]], text)
    for line in (FIXTURE.parent / 'string/nato-special-words.txt').read_text().splitlines():
        if line.startswith('VALID '):
            source, expected = line[6:].split(' => ')
            yield unescape(source), unescape(expected)
        elif line.startswith('INVALID '):
            yield unescape(line[8:]), None


def sample(text='NATO:aBc-X_'):
    return m.Payload(inherited=m.SpecialWord(text), direct='\t Unicode é\n',
                     named=m.SpecialWord(text), values=(m.SpecialWord(text),),
                     nested=m.Nested(value=m.SpecialWord(text)))


class Models(unittest.TestCase):
    def test_frozen_shared_corpus(self):
        count = 0
        for source, expected in cases():
            count += 1
            with self.subTest(source=repr(source)):
                if expected is None:
                    with self.assertRaises(ValueError):
                        m.SpecialWord(source)
                else:
                    self.assertEqual(m.SpecialWord(source).value, expected)
        self.assertEqual(count, 113)

    def test_boundaries_full_alphabet_and_invalid_characters(self):
        for char in ALPHABET:
            for length in [5, 6, 260, 261, 262]:
                text = 'NATO:' + char * (length - 5)
                if 6 <= length <= 261:
                    self.assertEqual(m.SpecialWord(text).value, text)
                else:
                    with self.assertRaises(ValueError):
                        m.SpecialWord(text)
        self.assertEqual(m.SpecialWord('NATO:' + ALPHABET).value, 'NATO:' + ALPHABET)
        for point in [*range(256), 0x391, 0x410, 0x2011, 0xff21, 0xd800, 0xdfff, 0x1f600]:
            char = chr(point)
            if char not in ALPHABET:
                for text in ['NATO:' + char, 'NATO:A' + char + 'Z']:
                    with self.assertRaises(ValueError):
                        m.SpecialWord(text)

    def test_exact_types_frozen_slots_and_existing_field_mechanisms(self):
        class StringSubclass(str):
            pass
        for bad in [None, True, 1, 1.0, b'NATO:A', ['NATO:A'], StringSubclass('NATO:A')]:
            with self.assertRaises(TypeError):
                m.SpecialWord(bad)
        value = sample()
        self.assertEqual(value.named.value, 'NATO:aBc-X_')
        self.assertEqual(value.direct, '\t Unicode é\n')
        self.assertIsNone(value.optional)
        self.assertEqual(m.Restricted('NATO:A').value, 'NATO:A')
        self.assertEqual(m.any('unconstrained').value, 'unconstrained')
        with self.assertRaises(dataclasses.FrozenInstanceError):
            value.named.value = 'NATO:changed'
        self.assertFalse(hasattr(value.named, '__dict__'))
        with self.assertRaises(TypeError):
            dataclasses.replace(value, named='NATO:raw')
        with self.assertRaises(ValueError):
            dataclasses.replace(value, values=())


if __name__ == '__main__':
    unittest.main()