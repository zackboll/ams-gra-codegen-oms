"""Independent literal oracles, also executed under -O and package imports."""
import importlib
import json
from pathlib import Path
import sys
import unittest
from dataclasses import FrozenInstanceError

dest = Path(sys.argv[1])
style = sys.argv[2]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if (dest / 'service_codec.py').exists() else None


class Choice(unittest.TestCase):
    def owners(self):
        return m.Owner.GovernmentIdentifier(m.Country.USA), m.Owner.NATO_SpecialWord(m.Words('NATO:EXAMPLE'))

    def test_model(self):
        a, b = self.owners()
        self.assertEqual(a.alternative, 'GovernmentIdentifier')
        self.assertEqual(m.Owner.__schema_name__[1], 'Owner')
        self.assertNotEqual(m.Selection.Text('x'), m.Selection.OtherText('x'))
        self.assertEqual(m.Selection.schema_class('x').alternative, 'schema_class')
        self.assertEqual(m.Selection.When(' 2026-01-02T03:04:05Z ').value, '2026-01-02T03:04:05Z')
        self.assertEqual(m.Selection.Span(' PT1.2300S ').value, 'PT1.2300S')
        with self.assertRaises(FrozenInstanceError):
            a.value = m.Country.GBR
        self.assertFalse(hasattr(a, '__dict__'))
        for args in [(), ('Text',), ('bad', 'x'), ('Number', True), ('Text', 1),
                     ('Nested', m.Country.USA), (1, 'x'), ('Number', 2**31)]:
            with self.subTest(args=args), self.assertRaises((TypeError, ValueError)):
                m.Selection(*args)
        for value in [m.Selection.Text('x'), m.Selection.OtherText('x'), m.Selection.Number(-3),
                      m.Selection.Binary(m.Octets(b'\0\xff')), m.Selection.DirectBinary(b'\xff'),
                      m.Selection.When('2026-01-02T03:04:05Z'), m.Selection.Span('PT1.2300S'),
                      m.Selection.Record(m.Detail(count=2)), m.Selection.OtherRecord(m.Detail(count=2)),
                      m.Selection.Nested(a)]:
            self.assertIs(type(m.Payload(required=value, positive=(a, b)).required), m.Selection)
        for kwargs in [dict(positive=()), dict(positive=[a]), dict(positive=(a, 'USA')),
                       dict(positive=(a,), bounded=(a, b, a, b)), dict(positive=(a,), optional='USA')]:
            with self.assertRaises((TypeError, ValueError)):
                m.Payload(required=m.Selection.Text('x'), **kwargs)
        class Sub(m.Owner):
            pass
        with self.assertRaises(TypeError):
            m.Payload(required=m.Selection.Text('x'), positive=(Sub('GovernmentIdentifier', m.Country.USA),))
        broken = m.Owner.GovernmentIdentifier(m.Country.USA)
        object.__setattr__(broken, 'value', 'USA')
        with self.assertRaises(TypeError):
            m.Selection.Nested(broken)
        words = m.Words('NATO:EXAMPLE')
        object.__setattr__(words, 'value', 'invalid')
        with self.assertRaises(ValueError):
            m.Owner.NATO_SpecialWord(words)

    def test_codec(self):
        if c is None:
            return  # Model-only selection deliberately has no codec artifact.
        a, b = self.owners()
        goldens = [
            (m.Selection.Text('x'), '{"Text":"x"}'),
            (m.Selection.OtherText('x'), '{"OtherText":"x"}'),
            (m.Selection.Number(-3), '{"Number":-3}'),
            (m.Selection.Flag(True), '{"Flag":true}'),
            (m.Selection.Real(1.5), '{"Real":1.5}'),
            (m.Selection.Binary(m.Octets(b'\0\xff')), '{"Binary":"00FF"}'),
            (m.Selection.DirectBinary(b'\xff'), '{"DirectBinary":"FF"}'),
            (m.Selection.When('2026-01-02T03:04:05Z'), '{"When":"2026-01-02T03:04:05Z"}'),
            (m.Selection.Span('PT1.2300S'), '{"Span":"PT1.2300S"}'),
            (m.Selection.Record(m.Detail(count=2)), '{"Record":{"Count":2}}'),
            (m.Selection.OtherRecord(m.Detail(count=2)), '{"OtherRecord":{"Count":2}}'),
            (m.Selection.Nested(a), '{"Nested":{"GovernmentIdentifier":"USA"}}'),
            (m.Selection.schema_class('x'), '{"class":"x"}'),
        ]
        for value, golden in goldens:
            text = '{"Required":' + golden + ',"Positive":[{"GovernmentIdentifier":"USA"}]}'
            payload = m.Payload(required=value, positive=(a,))
            self.assertEqual(c.encode_Payload(payload), text)
            self.assertEqual(c.decode_Payload(text), payload)
        for owners in [(a, b), (b, a), (a, a), (b, b)]:
            payload = m.Payload(required=m.Selection.Text('x'), positive=owners, optional=b,
                                bounded=owners, inherited=owners, repeated=(m.Selection.Text('x'),) * 2)
            text = c.encode_Payload(payload)
            self.assertEqual(c.decode_Payload(text), payload)
            self.assertEqual(json.loads(text)['Positive'], [json.loads(c._dumps(c._encode_Owner(x, '$'))) for x in owners])
        for invalid in ['{}', '{"Text":"a","OtherText":"b"}', '{"Unknown":"x"}', 'null', '["x"]',
                        '{"Text":null}', '{"$type":"Selection"}', '{"Text":1}', '{"Number":true}',
                        '{"Number":2147483648}', '{"Text":"a","Text":"a"}',
                        '{"Text":"a","Te\\u0078t":"b"}', '{"Nested":{"NATO_SpecialWord":"BAD"}}',
                        '{"Record":{"Count":-1}}', '{"Binary":"GG"}', '{"When":"BAD"}',
                        '{"Span":"PT"}', '{"Text":"\\ud800"}', '{"Number":1.0}', '{"Number":NaN}']:
            text = '{"Required":' + invalid + ',"Positive":[{"GovernmentIdentifier":"USA"}]}'
            with self.subTest(invalid=invalid), self.assertRaises(c.CodecError):
                c.decode_Payload(text)
        for index in range(3):
            values = [{'Text': 'x'}] * 3
            values[index] = {'Number': True}
            with self.assertRaises(c.CodecError) as caught:
                c.decode_Payload(json.dumps({'Required': {'Text': 'x'}, 'Repeated': values,
                                             'Positive': [{'GovernmentIdentifier': 'USA'}]}))
            self.assertIn(f'$.Repeated[{index}].Number:', str(caught.exception))
        for text in ['{}', '{"Required":{"Text":"x"},"Positive":[]}',
                     '{"Required":{"Text":"x"}}']:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(text)
        for attr, bad in [('alternative', 'wrong'), ('alternative', 1), ('value', 2)]:
            broken = m.Selection.Text('x')
            object.__setattr__(broken, attr, bad)
            with self.assertRaises(c.CodecError):
                c._encode_Selection(broken, '$.Required')
        for value, attribute, bad in [
            (m.Selection.Real(1.5), 'value', True),
            (m.Selection.When('2026-01-02T03:04:05Z'), 'value', 'BAD'),
            (m.Selection.Span('PT1S'), 'value', 'PT'),
            (m.Selection.DirectBinary(b'x'), 'value', '78'),
        ]:
            object.__setattr__(value, attribute, bad)
            with self.assertRaises(c.CodecError):
                c._encode_Selection(value, '$')
        empty = object.__new__(m.Selection)
        with self.assertRaises(c.CodecError):
            c._encode_Selection(empty, '$.Required')
        class Sub(m.Selection):
            pass
        with self.assertRaises(c.CodecError):
            c._encode_Selection(Sub('Text', 'x'), '$')
        nested = m.Detail(count=1)
        wrapped = m.Selection.Record(nested)
        object.__setattr__(nested, 'count', True)
        with self.assertRaises(c.CodecError) as caught:
            c._encode_Selection(wrapped, '$.Required')
        self.assertIn('$.Required.Record.Count', str(caught.exception))
        words = m.Words('NATO:EXAMPLE')
        wrapped = m.Owner.NATO_SpecialWord(words)
        object.__setattr__(words, 'value', 'BAD')
        with self.assertRaises(c.CodecError):
            c._encode_Owner(wrapped, '$')


unittest.main(argv=[sys.argv[0]], verbosity=2)