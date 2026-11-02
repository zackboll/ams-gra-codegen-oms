"""Model-only checks, also executed with -O and package-relative imports."""
import dataclasses
import importlib
from pathlib import Path
import sys
import unittest

generated = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(generated if sys.argv[2] == 'standalone' else generated.parent))
prefix = '' if sys.argv[2] == 'standalone' else generated.name + '.'
m = importlib.import_module(prefix + 'model')
sys.argv[1:] = []


def sample():
    return m.Payload(inherited='P12M', direct='PT36H', named=m.Span('-P0D'),
                     values=(m.Span('PT1.2300S'),),
                     nested=m.Nested(value=m.Span('P1Y')),
                     namedunbounded=(m.Span('P0D'),))


class ModelTests(unittest.TestCase):
    def test_shapes(self):
        value = dataclasses.replace(sample(), inherited=' \tP12M\r\n',
                                    optional=' -PT0.00S ', unbounded=(' P1D ', 'PT1.20S'),
                                    namedoptional=m.Span('P1Y'), restricted=m.Restricted('PT0S'))
        self.assertEqual(value.inherited, 'P12M')
        self.assertEqual(value.optional, '-PT0.00S')
        self.assertEqual(value.unbounded, ('P1D', 'PT1.20S'))
        self.assertIsNone(sample().optional)
        self.assertFalse(hasattr(value, '__dict__'))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            value.direct = 'P0D'
        with self.assertRaises(TypeError):
            m.Payload()
        for changes in [dict(values=()), dict(values=(m.Span('P0D'),) * 4),
                        dict(namedunbounded=()), dict(unbounded=['P0D']),
                        dict(inherited='P'), dict(nested=m.Span('P0D'))]:
            with self.assertRaises((TypeError, ValueError)):
                dataclasses.replace(sample(), **changes)

    def test_exact_types(self):
        class Text(str):
            pass
        for invalid in [None, True, 1, 1.0, b'P0D', ['P0D'], Text('P0D')]:
            with self.assertRaises(TypeError):
                m.Span(invalid)
            for field in ['inherited', 'direct']:
                with self.assertRaises(TypeError):
                    dataclasses.replace(sample(), **{field: invalid})
            with self.assertRaises(TypeError):
                dataclasses.replace(sample(), unbounded=(invalid,))


if __name__ == '__main__':
    unittest.main()