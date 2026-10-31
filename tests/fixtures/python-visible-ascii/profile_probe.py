"""Actual pinned declaration, both imports/policies, no type-name admission."""
import importlib
import json
from pathlib import Path
import sys
import unittest

path = Path(sys.argv[1]).resolve()
style, name = sys.argv[2:4]
minimum, maximum = map(int, sys.argv[4:6])
sys.path.insert(0, str(path if style == 'standalone' else path.parent))
prefix = '' if style == 'standalone' else path.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
carrier = getattr(m, name)
encode, decode = getattr(c, 'encode_' + name), getattr(c, 'decode_' + name)
sys.argv[1:] = []


class Pinned(unittest.TestCase):
    def test_boundaries_and_lexical_preservation(self):
        for text in [' ' * minimum, '~' * maximum, ' ' + 'x' * (maximum - 2) + ' ']:
            self.assertEqual(carrier(text).value, text)
            self.assertEqual(decode(encode(carrier(text))).value, text)
            self.assertEqual(json.loads(encode(carrier(text))), text)

    def test_invalid_and_forged(self):
        for bad in ['', 'x' * (maximum + 1), '\t', '\r', '\n', '\x00', '\x1f', '\x7f', 'é', '\ud800', 3, None]:
            with self.assertRaises(c.CodecError):
                decode(json.dumps(bad))
            forged = object.__new__(carrier)
            object.__setattr__(forged, 'value', bad)
            with self.assertRaises(c.CodecError):
                encode(forged)


if __name__ == '__main__':
    unittest.main()