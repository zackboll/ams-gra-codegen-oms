"""Independent literal oracles executed against actual generated modules."""
import dataclasses
import importlib
import json
from pathlib import Path
import py_compile
import sys
import typing
import unittest

dest = Path(sys.argv[1])
style = sys.argv[2]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
api = importlib.import_module(prefix + 'service_api')
EXPECTED = ('{"Base_Required":7,"Base_Optional":"ancestor","Base_Values":[2,9],'
            '"Required_Values":[3,4],"Linked":5,"Local":"leaf",'
            '"Nested":{"Count":8,"Label":"nested"}}')
ABSENT = ('{"Base_Required":7,"Required_Values":[3],"Linked":5,"Local":"leaf",'
          '"Nested":{"Count":8,"Label":"nested"}}')


def value(**changes) -> m.Leaf:
    args = dict(base_required=7, base_optional='ancestor', base_values=(m.Token(2), m.Token(9)),
                required_values=(3, 4), linked=m.Token(5), local='leaf',
                nested=m.Nested(count=8, label='nested'))
    args.update(changes)
    return m.Leaf(**args)


class InheritedRecords(unittest.TestCase):
    def test_compile_and_hints(self):
        for file in dest.glob('*.py'):
            py_compile.compile(str(file), doraise=True)
        for cls in [m.Token, m.Middle, m.Third, m.Leaf, m.Nested, m.Sibling]:
            self.assertTrue(typing.get_type_hints(cls))

    def test_shape_identity_and_metadata(self):
        self.assertFalse(hasattr(m, 'Base'))
        fields = dataclasses.fields(m.Leaf)
        self.assertEqual([f.name for f in fields], ['base_required', 'base_optional', 'base_values',
                         'required_values', 'linked', 'local', 'nested'])
        self.assertEqual([f.metadata['schema_name'] for f in fields], ['Base_Required', 'Base_Optional',
                         'Base_Values', 'Required_Values', 'Linked', 'Local', 'Nested'])
        self.assertEqual({f.metadata['schema_namespace'] for f in fields},
                         {'https://www.vdl.afrl.af.mil/programs/oam'})
        self.assertEqual(m.Leaf.__schema_name__, ('https://www.vdl.afrl.af.mil/programs/oam', 'Leaf'))
        self.assertEqual(m.Leaf.__bases__, (object,))
        self.assertFalse(hasattr(value(), '__dict__'))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            value().local = 'mutated'
        with self.assertRaises(TypeError):
            m.Leaf(7)

    def test_literal_encode_and_decode(self):
        self.assertEqual(c.encode_Leaf(value()), EXPECTED)
        decoded = c.decode_Leaf(EXPECTED)
        self.assertIs(type(decoded), m.Leaf)
        self.assertEqual(decoded, value())
        self.assertIs(type(decoded.nested), m.Nested)
        self.assertEqual([t.value for t in decoded.base_values], [2, 9])

    def test_absence_and_zero_minimum(self):
        expected = value(base_optional=None, base_values=(), required_values=(3,))
        self.assertEqual(c.encode_Leaf(expected), ABSENT)
        self.assertEqual(c.decode_Leaf(ABSENT), expected)
        self.assertEqual(m.Middle(base_required=1, required_values=(2,)).base_values, ())

    def test_shared_ancestor_and_concrete_base(self):
        sibling = m.Sibling(base_required=1, required_values=(2,), other=True)
        self.assertEqual(c.encode_Sibling(sibling), '{"Base_Required":1,"Required_Values":[2],"Other":true}')
        self.assertIs(type(c.decode_Sibling(c.encode_Sibling(sibling))), m.Sibling)
        self.assertEqual(m.Nested(count=8, label='nested').count, 8)
        self.assertFalse(isinstance(m.Nested(count=8, label='nested'), m.ConcreteBase))

    def test_constructor_inherited_constraints(self):
        for changes in [dict(base_required=True), dict(base_required='7'), dict(base_values=[m.Token(2)]),
                        dict(base_values=(m.Token(2), 9)), dict(required_values=()),
                        dict(required_values=(3, False)), dict(base_optional=9),
                        dict(base_values=(m.Token(1),) * 4), dict(nested=m.ConcreteBase(count=8))]:
            with self.subTest(changes=changes), self.assertRaises((TypeError, ValueError)):
                value(**changes)
        with self.assertRaises(ValueError):
            m.Token(10)
        with self.assertRaises(TypeError):
            m.Leaf(local='missing')

    def test_missing_required_and_null(self):
        for key in ['Base_Required', 'Required_Values', 'Linked', 'Local', 'Nested']:
            doc = json.loads(EXPECTED)
            del doc[key]
            with self.subTest(key=key), self.assertRaises(c.CodecError):
                c.decode_Leaf(json.dumps(doc))
        for key in json.loads(EXPECTED):
            doc = json.loads(EXPECTED)
            doc[key] = None
            with self.subTest(key=key), self.assertRaises(c.CodecError):
                c.decode_Leaf(json.dumps(doc))

    def test_later_array_elements_are_indexed(self):
        for key, bad in [('Base_Values', [2, 10]), ('Required_Values', [3, True])]:
            doc = json.loads(EXPECTED)
            doc[key] = bad
            with self.subTest(key=key), self.assertRaisesRegex(c.CodecError, key + r'\[1\]'):
                c.decode_Leaf(json.dumps(doc))

    def test_unknown_discriminator_duplicates_and_malformed(self):
        for text in [EXPECTED[:-1] + ',"$type":"Sibling"}', EXPECTED[:-1] + ',"Unknown":1}',
                     EXPECTED[:-1] + ',"Base_Required":8}',
                     EXPECTED[:-1] + ',"\\u0042ase_Required":8}', '{', '[]', 'null',
                     EXPECTED.replace('"Base_Required":7', '"Base_Required":true')]:
            with self.subTest(text=text), self.assertRaises(c.CodecError):
                c.decode_Leaf(text)

    def test_corrupted_outbound_fields(self):
        for field, bad in [('base_required', True), ('base_optional', 9), ('base_values', (m.Token(2), 9)),
                           ('required_values', (3, False)), ('local', 4), ('nested', m.ConcreteBase(count=8))]:
            corrupted = value()
            object.__setattr__(corrupted, field, bad)
            with self.subTest(field=field), self.assertRaises(c.CodecError):
                c.encode_Leaf(corrupted)
        corrupted = value()
        object.__setattr__(corrupted.base_values[1], 'value', 10)
        with self.assertRaisesRegex(c.CodecError, r'Base_Values\[1\]'):
            c.encode_Leaf(corrupted)

    def test_no_partial_value_on_later_error(self):
        created = []
        original = m.Leaf.__post_init__
        try:
            m.Leaf.__post_init__ = lambda self: (created.append(self), original(self))[-1]
            with self.assertRaises(c.CodecError):
                c.decode_Leaf(EXPECTED.replace('"Label":"nested"', '"Label":false'))
            self.assertEqual(created, [])
        finally:
            m.Leaf.__post_init__ = original

    def test_exact_declared_type_only(self):
        with self.assertRaises(c.CodecError):
            c.encode_Leaf(m.Third(base_required=7, required_values=(3,), linked=m.Token(5)))
        class Subclass(m.Leaf):
            pass
        derived = Subclass(base_required=7, required_values=(3,), linked=m.Token(5),
                           local='leaf', nested=m.Nested(count=8, label='nested'))
        with self.assertRaises(c.CodecError):
            c.encode_Leaf(derived)
        with self.assertRaises(TypeError):
            value(nested=object())


unittest.main(argv=['py007'], verbosity=2)