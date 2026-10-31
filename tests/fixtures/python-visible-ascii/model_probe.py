"""Independent Task 039 corpus and all-ten-domain generated model checks."""
import dataclasses
import importlib
from pathlib import Path
import re
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
PAIRS = [(1, n) for n in (20, 32, 64, 81, 128, 256, 480, 512, 1024)] + [(2, 4)]


def cases():
    def unescape(text):
        escapes = {'s': ' ', 't': '\t', 'r': '\r', 'n': '\n', '\\': '\\'}
        return re.sub(r'\\(?:u\{([0-9a-fA-F]+)\}|([strn\\]))',
                      lambda match: chr(int(match[1], 16)) if match[1] else escapes[match[2]], text)
    for line in (FIXTURE.parent / 'string/visible-ascii.txt').read_text().splitlines():
        if line.startswith('VALID '):
            source, expected = line[6:].split(' => ')
            yield unescape(source), unescape(expected)
        elif line == 'INVALID' or line.startswith('INVALID '):
            yield unescape(line[8:]), None


def sample(text='  kept  '):
    return m.Payload(inherited=m.Text256(text), direct='\t Unicode é\n', named=m.Text256(text),
                     values=(m.Text256(text),), nested=m.Nested(value=m.Text256(text)))


class Models(unittest.TestCase):
    def test_frozen_shared_corpus(self):
        count = 0
        for source, expected in cases():
            count += 1
            with self.subTest(source=repr(source)):
                if expected is None:
                    with self.assertRaises(ValueError):
                        m.Text256(source)
                else:
                    self.assertEqual(m.Text256(source).value, expected)
        self.assertEqual(count, 60)

    def test_all_ten_boundaries_and_ascii_interval(self):
        for minimum, maximum in PAIRS:
            carrier = getattr(m, 'Text' + str(maximum))
            for char in [' ', '!', '~', 'A']:
                for length in [minimum - 1, minimum, maximum, maximum + 1]:
                    text = char * length
                    with self.subTest(bounds=(minimum, maximum), char=char, length=length):
                        if minimum <= length <= maximum:
                            self.assertEqual(carrier(text).value, text)
                        else:
                            with self.assertRaises(ValueError):
                                carrier(text)
            for point in [*range(128), 0x80, 0xa0, 0xe9, 0xd800, 0xdfff, 0x1f600]:
                text = chr(point) * minimum
                if 0x20 <= point <= 0x7e:
                    self.assertEqual(carrier(text).value, text)
                else:
                    with self.assertRaises(ValueError):
                        carrier(text)

    def test_exact_types_and_storage(self):
        class StringSubclass(str):
            pass
        for bad in [None, True, 1, 1.0, b'abc', ['a'], StringSubclass('abc')]:
            with self.assertRaises(TypeError):
                m.Text256(bad)
        value = sample()
        self.assertEqual(value.named.value, '  kept  ')
        self.assertEqual(value.direct, '\t Unicode é\n')
        self.assertIsNone(value.optional)
        self.assertEqual(m.Restricted('  kept  ').value, '  kept  ')
        with self.assertRaises(dataclasses.FrozenInstanceError):
            value.named.value = 'changed'
        self.assertFalse(hasattr(value.named, '__dict__'))
        with self.assertRaises(TypeError):
            dataclasses.replace(value, named='raw')
        with self.assertRaises(ValueError):
            dataclasses.replace(value, values=())


if __name__ == '__main__':
    unittest.main()