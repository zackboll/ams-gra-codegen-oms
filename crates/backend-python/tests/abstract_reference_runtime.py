"""Execute the original inhabited fixture, as a Record or normalized Choice."""
import dataclasses
import typing
import unittest
import model as m

NS = 'urn:backend-abstract-value-reference'


def holder(value):
    return m.Holder.Value(value) if HOLDER_CHOICE else m.Holder(value=value)


class AbstractReference(unittest.TestCase):
    def test_exact_inventory_and_inherited_field(self):
        classes = {name for name, value in vars(m).items()
                   if isinstance(value, type) and value.__module__ == m.__name__}
        self.assertEqual(classes, {'Base', 'Derived', 'Holder'})
        self.assertEqual([f.name for f in dataclasses.fields(m.Base)], ['alternative', 'value'])
        self.assertEqual([f.name for f in dataclasses.fields(m.Derived)], ['value'])
        self.assertIs(typing.get_type_hints(m.Base)['value'], m.Derived)
        self.assertEqual(m.Base.__schema_name__, (NS, 'Base'))
        self.assertEqual(m.Derived.__schema_name__, (NS, 'Derived'))
        self.assertFalse(hasattr(m.Base, 'from_Base'))

    def test_explicit_checked_construction_and_immutability(self):
        concrete = m.Derived(value='inherited')
        selected = m.Base.from_Derived(concrete)
        self.assertEqual(selected.alternative, (NS, 'Derived'))
        self.assertIs(type(selected.value), m.Derived)
        self.assertEqual(selected.value.value, 'inherited')
        self.assertEqual(holder(selected).value, selected)
        self.assertEqual(selected, m.Base((NS, 'Derived'), m.Derived(value='inherited')))
        self.assertEqual(hash(selected), hash(m.Base.from_Derived(concrete)))
        self.assertFalse(hasattr(selected, '__dict__'))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            selected.value = None

    def test_wrong_classes_missing_and_invalid_members(self):
        for invalid in [None, {}, 'payload', m.Base.from_Derived(m.Derived(value='x'))]:
            with self.subTest(invalid=invalid), self.assertRaises(TypeError):
                m.Base.from_Derived(invalid)
        with self.assertRaises(TypeError):
            holder(m.Derived(value='not wrapped'))
        for construct in [lambda: m.Derived(), lambda: m.Base((NS, 'Derived')),
                          lambda: m.Holder(), lambda: m.Derived(value=1)]:
            with self.assertRaises(TypeError):
                construct()
        class DerivedSubclass(m.Derived):
            pass
        with self.assertRaises(TypeError):
            m.Base.from_Derived(DerivedSubclass(value='x'))
        class BaseSubclass(m.Base):
            pass
        with self.assertRaises(TypeError):
            holder(BaseSubclass((NS, 'Derived'), m.Derived(value='x')))

    def test_unknown_abstract_or_malformed_selection(self):
        for tag in [(NS, 'Base'), (NS, 'Unknown'), ('urn:private', 'Derived'),
                    'Derived', (NS,), (NS, 1), [NS, 'Derived']]:
            with self.subTest(tag=tag), self.assertRaises((TypeError, ValueError)):
                m.Base(tag, m.Derived(value='x'))

    def test_forged_storage_is_recursively_rejected(self):
        for member, invalid in [('alternative', (NS, 'Unknown')), ('value', None)]:
            selected = m.Base.from_Derived(m.Derived(value='x'))
            object.__setattr__(selected, member, invalid)
            with self.assertRaises((TypeError, ValueError)):
                holder(selected)
        concrete = m.Derived(value='x')
        object.__setattr__(concrete, 'value', 3)
        with self.assertRaises(TypeError):
            m.Base.from_Derived(concrete)
        selected = m.Base.from_Derived(m.Derived(value='x'))
        object.__delattr__(selected.value, 'value')
        with self.assertRaises(AttributeError):
            holder(selected)
        with self.assertRaises(AttributeError):
            holder(object.__new__(m.Base))


unittest.main()