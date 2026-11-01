"""Independent PY-012 model and wire oracles; assertions survive python -O."""
import dataclasses
import importlib
import json
from pathlib import Path
import sys
import typing
import unittest

destination, style, selection = Path(sys.argv[1]).resolve(), sys.argv[2], sys.argv[3]
sys.path.insert(0, str(destination if style == 'standalone' else destination.parent))
prefix = '' if style == 'standalone' else destination.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if selection == 'codec' else None
sys.argv = [sys.argv[0]]
BASE = {'Required': [7], 'At-Least-Three': ['a', 'b', 'a']}
LEXICAL = '2000-02-29T24:00:00.000Z'


def value(**fields):
    return m.Payload(required=(7,), at_least_three=('a', 'b', 'a'), **fields)


class ModelPositive(unittest.TestCase):
    def test_defaults_and_exact_minimum(self):
        v = value()
        for field in dataclasses.fields(v):
            self.assertIs(type(getattr(v, field.name)), tuple)
        self.assertEqual(v.strings, ())
        self.assertEqual(v.inherited_values, ())
        self.assertEqual(v.at_least_three, ('a', 'b', 'a'))
        self.assertEqual(m.Payload(required=(1, 2), at_least_three=('a',) * 4).required, (1, 2))

    def test_scalars_carriers_and_order(self):
        v = value(strings=('λ', '', 'λ'), flags=(True, False, True),
                  integers=(2**100, -2**100), unsigned=(0, 2**32 - 1), floats=(1.5, -0.0),
                  words=(m.Word('NATO:aBc-X_'),), visible=(m.Visible('a B'),),
                  states=(m.State.ON, m.State.OFF, m.State.ON), inherited_values=('z', 'a', 'z'))
        self.assertEqual(v.strings, ('λ', '', 'λ'))
        self.assertEqual(v.inherited_values, ('z', 'a', 'z'))
        self.assertEqual(v.words[0].value, 'NATO:aBc-X_')
        self.assertEqual(v.states, (m.State.ON, m.State.OFF, m.State.ON))

    def test_datetime_normalization_and_nested_records(self):
        child = m.Child(words=(m.Word('NATO:A'), m.Word('NATO:A')))
        v = value(times=(' \t' + LEXICAL + '\n', '2001-01-01T00:00:00+14:00'),
                  instants=(m.Instant(LEXICAL),), groups=(m.Group(children=(child, child)),))
        self.assertEqual(v.times, (LEXICAL, '2001-01-01T00:00:00+14:00'))
        self.assertEqual(v.groups[0].children, (child, child))

    def test_frozen_slots_and_annotations(self):
        v = value()
        self.assertFalse(hasattr(v, '__dict__'))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            v.strings = ('x',)
        hints = typing.get_type_hints(m.Payload)
        self.assertEqual(hints['strings'], tuple[str, ...])
        self.assertEqual(hints['words'], tuple[m.Word, ...])

    def test_large_sequence(self):
        entries = tuple(range(8193))
        v = m.Payload(required=entries, at_least_three=('a', 'b', 'a'))
        self.assertEqual(v.required, entries)
        self.assertIs(v.required, entries)  # Immutable input needs no mutable ownership copy.


class ModelNegative(unittest.TestCase):
    def test_required_initialization_and_counts(self):
        for fields in [{}, {'required': (1,)}, {'at_least_three': ('a',) * 3},
                       {'required': (), 'at_least_three': ('a',) * 3},
                       *({'required': (1,), 'at_least_three': ('a',) * n} for n in range(3))]:
            with self.subTest(fields=fields), self.assertRaises((TypeError, ValueError)):
                m.Payload(**fields)

    def test_exact_tuple(self):
        class TupleSubclass(tuple):
            pass
        for entries in [[], ['a'], iter(['a']), TupleSubclass(('a',)), None, 'a']:
            with self.subTest(container=type(entries)), self.assertRaises(TypeError):
                value(strings=entries)

    def test_first_middle_last_elements(self):
        for field, good, bad in [
            ('strings', 'a', 3), ('flags', True, 1), ('integers', 3, True),
            ('unsigned', 3, -1), ('floats', 1.5, 1), ('times', LEXICAL, '2000-02-30T00:00:00Z'),
            ('instants', m.Instant(LEXICAL), LEXICAL), ('words', m.Word('NATO:A'), 'NATO:A'),
            ('visible', m.Visible('x'), 'x'), ('states', m.State.ON, 'ON'),
            ('groups', m.Group(), {}), ('inherited_values', 'a', False),
        ]:
            for index in range(3):
                entries = [good] * 3
                entries[index] = bad
                with self.subTest(field=field, index=index), self.assertRaises((TypeError, ValueError)):
                    value(**{field: tuple(entries)})

    def test_bounded_unchanged(self):
        self.assertEqual(value(bounded=(1, 2, 3)).bounded, (1, 2, 3))
        with self.assertRaises(ValueError):
            value(bounded=(1, 2, 3, 4))


class CodecPositive(unittest.TestCase):
    def test_literal_minimum_and_omission(self):
        text = '{"Required":[7],"At-Least-Three":["a","b","a"]}'
        self.assertEqual(c.encode_Payload(value()), text)
        self.assertEqual(c.decode_Payload(text), value())
        self.assertEqual(c.decode_Payload(text[:-1] + ',"Strings":[],"Inherited-Values":[]}'), value())

    def test_independent_full_wire_tree(self):
        expected = dict(BASE, **{
            'Inherited-Values': ['z', 'a', 'z'], 'Strings': ['λ', '', 'λ'],
            'Flags': [True, False, True], 'Integers': [2**100, -2**100],
            'Unsigned': [0, 4294967295], 'Floats': [1.5, -0.0], 'Times': [LEXICAL],
            'Instants': [LEXICAL], 'Words': ['NATO:aBc-X_', 'NATO:A'], 'Visible': ['a B'],
            'States': ['ON', 'OFF', 'ON'], 'Groups': [{'Children': [{'Words': ['NATO:A', 'NATO:A']}]}],
        })
        v = value(inherited_values=('z', 'a', 'z'), strings=('λ', '', 'λ'), flags=(True, False, True),
                  integers=(2**100, -2**100), unsigned=(0, 4294967295), floats=(1.5, -0.0),
                  times=(LEXICAL,), instants=(m.Instant(LEXICAL),), words=(m.Word('NATO:aBc-X_'), m.Word('NATO:A')),
                  visible=(m.Visible('a B'),), states=(m.State.ON, m.State.OFF, m.State.ON),
                  groups=(m.Group(children=(m.Child(words=(m.Word('NATO:A'), m.Word('NATO:A'))),)),))
        self.assertEqual(json.loads(c.encode_Payload(v)), expected)
        self.assertEqual(c.decode_Payload(json.dumps(expected)), v)

    def test_above_minimum_and_large_array(self):
        for size in [1, 3, 4, 8193]:
            entries = list(range(size))
            wire = dict(BASE, Required=entries)
            v = m.Payload(required=tuple(entries), at_least_three=('a', 'b', 'a'))
            self.assertEqual(c.decode_Payload(json.dumps(wire)), v)
            self.assertEqual(json.loads(c.encode_Payload(v)), wire)


class CodecNegative(unittest.TestCase):
    def test_missing_and_insufficient(self):
        for field, minimum in [('Required', 1), ('At-Least-Three', 3)]:
            wire = dict(BASE)
            del wire[field]
            with self.subTest(field=field), self.assertRaisesRegex(c.CodecError, field):
                c.decode_Payload(json.dumps(wire))
            for size in range(minimum):
                wire[field] = [1 if field == 'Required' else 'a'] * size
                with self.subTest(field=field, size=size), self.assertRaisesRegex(c.CodecError, field):
                    c.decode_Payload(json.dumps(wire))

    def test_wrong_containers(self):
        for field in ['Required', 'Strings', 'Inherited-Values', 'Groups', 'Times']:
            for bad in [None, 'scalar', 1, True, {}]:
                with self.subTest(field=field, bad=bad), self.assertRaisesRegex(c.CodecError, field):
                    c.decode_Payload(json.dumps(dict(BASE, **{field: bad})))

    def test_first_middle_last_paths(self):
        for field, good, bad in [
            ('Strings', 'a', 3), ('Flags', True, 1), ('Integers', 3, True),
            ('Unsigned', 3, -1), ('Floats', 1.5, True), ('Times', LEXICAL, 'bad'),
            ('Instants', LEXICAL, '2000-01-01T00:00:00'), ('Words', 'NATO:A', 'NATO:A1'),
            ('Visible', 'x', '\n'), ('States', 'ON', 'INVALID'),
            ('Groups', {}, 3), ('Inherited-Values', 'a', False),
        ]:
            for index in range(3):
                entries = [good] * 3
                entries[index] = bad
                with self.subTest(field=field, index=index), self.assertRaises(c.CodecError) as caught:
                    c.decode_Payload(json.dumps(dict(BASE, **{field: entries})))
                self.assertIn(f'$.{field}[{index}]', str(caught.exception))

    def test_nested_paths(self):
        wire = dict(BASE, Groups=[{'Children': [{'Words': ['NATO:A', 'NATO:1']}]}])
        with self.assertRaisesRegex(c.CodecError, r'\$\.Groups\[0\]\.Children\[0\]\.Words\[1\]'):
            c.decode_Payload(json.dumps(wire))
        child = m.Child(words=(m.Word('NATO:A'),))
        object.__setattr__(child.words[0], 'value', 'NATO:1')
        with self.assertRaisesRegex(c.CodecError, r'\$\.Groups\[0\]\.Children\[0\]\.Words\[0\]'):
            c.encode_Payload(value(groups=(m.Group(children=(child,)),)))

    def test_outbound_forged_elements_and_containers(self):
        for field, carrier, good, bad in [('words', m.Word, 'NATO:A', 'NATO:1'),
                                           ('visible', m.Visible, 'x', '\n'),
                                           ('instants', m.Instant, LEXICAL, 'bad')]:
            for index in range(3):
                entries = [carrier(good) for _ in range(3)]
                object.__setattr__(entries[index], 'value', bad)
                with self.subTest(field=field, index=index), self.assertRaises(c.CodecError) as caught:
                    c.encode_Payload(value(**{field: tuple(entries)}))
                self.assertIn(f'[{index}]', str(caught.exception))
            missing = object.__new__(carrier)
            with self.subTest(field=field), self.assertRaises(c.CodecError):
                c.encode_Payload(value(**{field: (missing,)}))
        for field, bad in [('strings', []), ('required', ()), ('at_least_three', ('x',)),
                           ('flags', (True, 1)), ('integers', (True,)), ('times', ('bad',)),
                           ('bounded', (1, 2, 3, 4))]:
            v = value()
            object.__setattr__(v, field, bad)
            with self.subTest(field=field), self.assertRaises(c.CodecError):
                c.encode_Payload(v)
        for field, good, bad in [('strings', 'a', 3), ('flags', True, 1),
                                  ('integers', 3, True), ('unsigned', 3, -1),
                                  ('floats', 1.5, True), ('times', LEXICAL, 'bad'),
                                  ('states', m.State.ON, 'ON'), ('groups', m.Group(), {}),
                                  ('inherited_values', 'a', '\ud800'), ('required', 7, 2**31)]:
            for index in range(3):
                entries = [good] * 3
                entries[index] = bad
                v = value()
                object.__setattr__(v, field, tuple(entries))
                with self.subTest(field=field, index=index), self.assertRaises(c.CodecError) as caught:
                    c.encode_Payload(v)
                self.assertIn(f'[{index}]', str(caught.exception))

    def test_strict_json_and_unknown_members(self):
        for text in ['{"Required":[7],"Required":[8],"At-Least-Three":["a","b","c"]}',
                     '{"Required":[NaN]}', '{"Required":[Infinity]}', 'null', '[]',
                     json.dumps(dict(BASE, extra=[])), json.dumps(dict(BASE, **{'$type': 'Payload'})),
                     json.dumps(dict(BASE, Strings=['\ud800'])),
                     json.dumps(dict(BASE, Groups=[{'Children': [], 'extra': 1}]))]:
            with self.subTest(text=text), self.assertRaises(c.CodecError):
                c.decode_Payload(text)


suite = unittest.TestSuite()
for group in [ModelPositive, ModelNegative] + ([CodecPositive, CodecNegative] if c else []):
    suite.addTests(unittest.defaultTestLoader.loadTestsFromTestCase(group))
result = unittest.TextTestRunner(verbosity=2).run(suite)
raise SystemExit(not result.wasSuccessful())