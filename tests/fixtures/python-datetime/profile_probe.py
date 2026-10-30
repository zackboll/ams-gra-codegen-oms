"""Execute actual pinned UCI DateTimeType, not a local-name inference."""
import importlib
import json
from pathlib import Path
import sys
import unittest

path = Path(sys.argv[1]).resolve()
style = sys.argv[2]
sys.path.insert(0, str(path if style == 'standalone' else path.parent))
prefix = '' if style == 'standalone' else path.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
sys.argv[1:] = []


class PinnedDateTime(unittest.TestCase):
    def test_checked_round_trip(self):
        value = '2000-02-29T23:59:60.123456789012345678900Z'
        self.assertEqual(m.DateTimeType('\t' + value + '\r').value, value)
        self.assertEqual(c.encode_DateTimeType(m.DateTimeType(value)), json.dumps(value))
        self.assertEqual(c.decode_DateTimeType(json.dumps(value)).value, value)

    def test_invalid_and_forged(self):
        for bad in ['1900-02-29T00:00:00Z', '2000-02-29T00:00:00+00:00', 'invalidZ', 3, None]:
            with self.assertRaises(c.CodecError):
                c.decode_DateTimeType(json.dumps(bad))
        value = m.DateTimeType('2000-01-01T00:00:00Z')
        object.__setattr__(value, 'value', 'invalidZ')
        with self.assertRaises(c.CodecError):
            c.encode_DateTimeType(value)


if __name__ == '__main__':
    unittest.main()
