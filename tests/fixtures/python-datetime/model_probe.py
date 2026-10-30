"""Execute codec-free generated model/API imports under both Python policies."""
import dataclasses
import importlib
from pathlib import Path
import sys
import unittest

path = Path(sys.argv[1]).resolve()
style = sys.argv[2]
sys.path.insert(0, str(path if style == 'standalone' else path.parent))
prefix = '' if style == 'standalone' else path.name + '.'
m = importlib.import_module(prefix + 'model')
importlib.import_module(prefix + 'service_api')
sys.argv[1:] = []


class ModelOnly(unittest.TestCase):
    def test_checked_storage(self):
        value = '2000-02-29T24:00:00.000Z'
        payload = m.Payload(inherited=value, direct='\t' + value + '\r',
                            named=m.Instant(value), values=(m.Instant(value),),
                            nested=m.Nested(value=m.Instant(value)))
        self.assertEqual(payload.direct, value)
        self.assertIsNone(payload.optional)
        with self.assertRaises(dataclasses.FrozenInstanceError):
            payload.direct = value
        self.assertFalse(hasattr(payload, '__dict__'))

    def test_invalid_calendar_and_named_timezone(self):
        for value in ['1900-02-29T00:00:00Z', '2000-02-29T00:00:00+14:00', 3, True]:
            with self.assertRaises((TypeError, ValueError)):
                m.Instant(value)
        with self.assertRaises(ValueError):
            m.Nested(value=m.Instant('2000-01-01T00:00:00Z'), directs=('2000-01-01T24:00:01',))


if __name__ == '__main__':
    unittest.main()
