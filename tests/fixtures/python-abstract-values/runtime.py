"""Independently authored typed/JSON oracle; also executed by optimized Python."""
import dataclasses
import importlib
import json
from pathlib import Path
import sys
import unittest

destination = Path(sys.argv[1])
style, mode = sys.argv[2:]
sys.path.insert(0, str(destination if style == 'standalone' else destination.parent))
prefix = '' if style == 'standalone' else destination.name + '.'
m = importlib.import_module(prefix + 'model')
api = importlib.import_module(prefix + 'service_api')
NS = 'https://www.vdl.afrl.af.mil/programs/oam'


def variants():
    return [m.Base.from_Zeta(m.Zeta(inherited=1, text='z')),
            m.Base.from_Deep_Leaf(m.Deep_Leaf(inherited=2, enabled=True, count=-3)),
            m.Base.from_Alpha(m.Alpha(inherited=3, real=1.5))]


WIRES = [{'Inherited': 1, 'Text': 'z', '$type': 'Zeta'},
         {'Inherited': 2, 'Enabled': True, 'Count': -3, '$type': 'Deep-Leaf'},
         {'Inherited': 3, 'Real': 1.5, '$type': 'Alpha'}]


def payload(required=None, optional=None):
    a, b, d = variants()
    return m.Payload(required=a if required is None else required, optional=optional,
                     repeated=(a, b, d), bounded=(d,), nested=m.Selection.Abstract(b),
                     concrete=m.Zeta(inherited=4, text='concrete'))


_UNSET = object()


def expected(required=_UNSET, optional=_UNSET):
    result = {'Required': WIRES[0] if required is _UNSET else required,
              'Repeated': WIRES, 'Bounded': [WIRES[2]],
              'Nested': {'Abstract': WIRES[1]},
              'Concrete': {'Inherited': 4, 'Text': 'concrete'}}
    if optional is not _UNSET:
        result['Optional'] = optional
    return result


class Models(unittest.TestCase):
    def test_exact_typed_variants_and_source_names(self):
        for value, concrete, wire in zip(variants(), [m.Zeta, m.Deep_Leaf, m.Alpha], WIRES):
            self.assertIs(type(value), m.Base)
            self.assertIs(type(value.value), concrete)
            self.assertEqual(value.alternative, (NS, wire['$type']))
            self.assertEqual(concrete.__schema_name__, (NS, wire['$type']))
        self.assertFalse(hasattr(m, 'Middle'))
        self.assertEqual(api.function_loop.exchange_input.Payload, m.Payload)

    def test_immutable_equality_and_hash(self):
        self.assertEqual(variants(), variants())
        self.assertEqual(hash(variants()[0]), hash(variants()[0]))
        value = variants()[0]
        self.assertFalse(hasattr(value, '__dict__'))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            value.value = None

    def test_explicit_selection_and_exact_payload(self):
        for tag in [(NS, 'Middle'), (NS, 'Base'), (NS, 'Private'), (NS, 'Deep_Leaf'),
                    ('urn:wrong', 'Zeta'), ('Zeta',), 'Zeta', [NS, 'Zeta'], (NS, 1)]:
            with self.subTest(tag=tag), self.assertRaises((TypeError, ValueError)):
                m.Base(tag, variants()[0].value)
        for wrong in [None, {}, variants()[1].value, variants()[0], 1]:
            with self.assertRaises(TypeError):
                m.Base.from_Zeta(wrong)
        with self.assertRaises(TypeError):
            m.Base((NS, 'Zeta'))
        class Child(m.Zeta):
            pass
        with self.assertRaises(TypeError):
            m.Base.from_Zeta(Child(inherited=1, text='x'))

    def test_inherited_member_validation(self):
        for invalid in [-1, True, '1']:
            with self.assertRaises((TypeError, ValueError)):
                m.Zeta(inherited=invalid, text='x')
        concrete = m.Zeta(inherited=1, text='x')
        object.__setattr__(concrete, 'inherited', -1)
        with self.assertRaises(ValueError):
            m.Base.from_Zeta(concrete)

    def test_forged_nested_storage_is_revalidated(self):
        for member, invalid in [('alternative', (NS, 'Private')), ('value', None)]:
            value = variants()[0]
            object.__setattr__(value, member, invalid)
            for construct in [lambda: payload(value), lambda: m.Selection.Abstract(value),
                              lambda: payload(optional=value)]:
                with self.assertRaises((TypeError, ValueError)):
                    construct()
        value = object.__new__(m.Base)
        with self.assertRaises((AttributeError, TypeError, ValueError)):
            payload(value)

    def test_required_optional_repeated_and_choice(self):
        for value in variants():
            self.assertEqual(payload(value, value).optional, value)
            self.assertEqual(m.Selection.Abstract(value).value, value)
        with self.assertRaises(TypeError):
            payload(variants()[0].value)
        with self.assertRaises(TypeError):
            m.Payload(required=variants()[0], repeated=[variants()[0]], bounded=(variants()[0],),
                      nested=m.Selection.Text('x'), concrete=variants()[0].value)


if mode == 'codec':
    c = importlib.import_module(prefix + 'service_codec')

    class Codecs(unittest.TestCase):
        def test_oracle_null_missing_and_valid_neighbor(self):
            healthy = {'Required': {'Inherited': 1, 'Text': 'z', '$type': 'Zeta'},
                       'Repeated': [{'Inherited': 1, 'Text': 'z', '$type': 'Zeta'},
                                    {'Inherited': 2, 'Enabled': True, 'Count': -3, '$type': 'Deep-Leaf'},
                                    {'Inherited': 3, 'Real': 1.5, '$type': 'Alpha'}],
                       'Bounded': [{'Inherited': 3, 'Real': 1.5, '$type': 'Alpha'}],
                       'Nested': {'Abstract': {'Inherited': 2, 'Enabled': True, 'Count': -3,
                                               '$type': 'Deep-Leaf'}},
                       'Concrete': {'Inherited': 4, 'Text': 'concrete'}}
            self.assertEqual(expected(), healthy)
            valid_text = json.dumps(healthy)
            self.assertEqual(c.decode_Payload(valid_text), payload())
            self.assertNotIn('Optional', json.loads(valid_text))
            missing = expected()
            del missing['Required']
            self.assertNotIn('Required', json.loads(json.dumps(missing)))
            observations = []
            for wire, path, reason in [
                    (expected(required=None), '$.Required', 'expected exact dict'),
                    (missing, '$', "missing required member 'Required'"),
                    (expected(optional=None), '$.Optional', 'expected exact dict')]:
                text = json.dumps(wire)
                self.assertNotEqual(text, valid_text)
                if 'Required' in wire and wire['Required'] is None:
                    self.assertIn('"Required": null', text)
                    self.assertIsNone(json.loads(text)['Required'])
                if 'Optional' in wire:
                    self.assertIn('"Optional": null', text)
                    self.assertIsNone(json.loads(text)['Optional'])
                # Invoke the actual generated public codec, never a mock.
                with self.assertRaises(c.CodecError) as caught:
                    c.decode_Payload(text)
                self.assertIn(path + ':', str(caught.exception))
                self.assertIn(reason, str(caught.exception))
                observations.append({'input': text, 'error': str(caught.exception)})
                self.assertEqual(c.decode_Payload(valid_text), payload())
            print('PY019R1 ORACLE ' + json.dumps(observations), flush=True)

        def test_all_variants_independent_wire_parity(self):
            for value, wire in zip(variants(), WIRES):
                for optional, optional_wire in [(None, None)] + list(zip(variants(), WIRES)):
                    typed = payload(value, optional)
                    text = c.encode_Payload(typed)
                    # Model None denotes absence, not an explicit JSON null.
                    wire_expected = (expected(wire) if optional is None
                                     else expected(wire, optional_wire))
                    self.assertEqual(json.loads(text), wire_expected)
                    self.assertEqual(c.decode_Payload(json.dumps(wire_expected)), typed)
                    self.assertEqual(c.encode_Payload(c.decode_Payload(text)), text)
            self.assertNotIn('$type', c._encode_Zeta(m.Zeta(inherited=1, text='z'), '$'))

        def test_invalid_discriminators(self):
            invalids = [{}, {'Inherited': 1, 'Text': 'z'}]
            invalids += [dict(WIRES[0], **{'$type': value}) for value in
                         [None, 1, True, [], {}, '', 'Base', 'Middle', 'Private', 'Deep_Leaf',
                          '{urn:private}Zeta', '{' + NS + '}Zeta']]
            for invalid in invalids:
                with self.subTest(invalid=invalid), self.assertRaises(c.CodecError):
                    c.decode_Payload(json.dumps(expected(invalid)))

        def test_known_invalid_content_never_falls_back(self):
            invalids = [{'Inherited': 2, 'Text': 'z', '$type': 'Alpha'},
                        {'Text': 'z', '$type': 'Zeta'},
                        dict(WIRES[0], Inherited=-1), dict(WIRES[0], Text=2),
                        dict(WIRES[0], Secret='private'), None, []]
            for invalid in invalids:
                with self.subTest(invalid=invalid), self.assertRaises(c.CodecError):
                    c.decode_Payload(json.dumps(expected(invalid)))

        def test_nested_indexed_diagnostics(self):
            for key, invalid, path in [
                    ('Required', dict(WIRES[0], Inherited=-1), '$.Required.Inherited'),
                    ('Repeated', [dict(WIRES[0], Text=None)], '$.Repeated[0].Text'),
                    ('Nested', {'Abstract': dict(WIRES[1], Count=True)}, '$.Nested.Abstract.Count'),
                    ('Optional', dict(WIRES[0], **{'$type': 'private'}), '$.Optional.$type')]:
                wire = expected()
                wire[key] = invalid
                with self.assertRaises(c.CodecError) as caught:
                    c.decode_Payload(json.dumps(wire))
                self.assertIn(path, str(caught.exception))

        def test_duplicate_members_nested_and_indexed(self):
            for location, path in [('Required', '$.Required'), ('Repeated', '$.Repeated[0]'),
                                   ('Nested', '$.Nested.Abstract')]:
                text = json.dumps(expected())
                if location == 'Required':
                    text = text.replace('"$type": "Zeta"', '"$type":"Zeta","$type":"Zeta"', 1)
                elif location == 'Repeated':
                    text = text.replace('"Repeated": [{"Inherited": 1', '"Repeated": [{"Inherited": 1,"Inherited": 1', 1)
                else:
                    text = text.replace('"Nested": {"Abstract": {"Inherited": 2', '"Nested": {"Abstract": {"Inherited": 2,"Inherited": 2', 1)
                with self.assertRaises(c.CodecError) as caught:
                    c.decode_Payload(text)
                self.assertIn(path, str(caught.exception))
                self.assertIn('duplicate member', str(caught.exception))

        def test_forged_encoder_tags_classes_and_storage(self):
            for member, invalid in [('alternative', (NS, 'private')), ('alternative', (NS, 'Deep-Leaf')),
                                    ('alternative', [NS, 'Zeta']), ('value', None),
                                    ('value', variants()[1].value)]:
                value = payload()
                object.__setattr__(value.required, member, invalid)
                with self.assertRaises(c.CodecError):
                    c.encode_Payload(value)
            value = payload()
            object.__delattr__(value.required.value, 'inherited')
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(value)
            self.assertIn('$.Required.Inherited', str(caught.exception))

        def test_unexpected_concrete_type_marker_and_payload_class(self):
            wire = expected()
            wire['Concrete']['$type'] = 'Zeta'
            with self.assertRaises(c.CodecError):
                c.decode_Payload(json.dumps(wire))
            class Child(m.Base):
                pass
            forged = Child((NS, 'Zeta'), variants()[0].value)
            with self.assertRaises(c.CodecError):
                c._encode_Base(forged, '$')


unittest.main(argv=['py019-runtime'], verbosity=2)