"""PY-006 independent literal oracles; assertions remain active under -O."""
import dataclasses
import importlib
import json
from pathlib import Path
import py_compile
import sys
import typing
import unittest

generated, style, root = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
del sys.argv[1:]
for source in generated.glob('*.py'):
    py_compile.compile(str(source), doraise=True)
sys.path.insert(0, str(generated if style == 'standalone' else generated.parent))
prefix = '' if style == 'standalone' else generated.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
a = importlib.import_module(prefix + 'service_api')
UUID = m.UniversallyUniqueIdentifierType
V = m.SchemaVersion
NIL = '00000000-0000-0000-0000-000000000000'
MIXED = '123E4567-e89B-42d3-A456-426614174000'
VERSION = '002.5.0ab_Release-2A'
WIRE = ('{"Header":{"Version":"002.5.0ab_Release-2A",'
        '"UUID":"123E4567-e89B-42d3-A456-426614174000"},'
        '"Versions":["000.0.0","999.99.99zz_a"],'
        '"UUIDs":["00000000-0000-0000-0000-000000000000",'
        '"123E4567-e89B-42d3-A456-426614174000"],"Count":42}')


def sample():
    return m.Payload(header=m.Header(version=V(VERSION), uuid=UUID(MIXED)),
                     versions=(V('000.0.0'), V('999.99.99zz_a')),
                     uuids=(UUID(NIL), UUID(MIXED)), count=42)


def unescape(text):
    for old, new in [(r'\s', ' '), (r'\t', '\t'), (r'\n', '\n'), (r'\r', '\r'), (r'\\', '\\')]:
        text = text.replace(old, new)
    return text


class HeaderProfiles(unittest.TestCase):
    def test_frozen_corpora(self):
        for filename, carrier, encode, decode, expected_count in [
                ('schema-version.txt', V, c.encode_SchemaVersion, c.decode_SchemaVersion, 41),
                ('uuid.txt', UUID, c.encode_UniversallyUniqueIdentifierType,
                 c.decode_UniversallyUniqueIdentifierType, 78)]:
            count = 0
            for line in (root / 'tests/fixtures/string' / filename).read_text().splitlines():
                if not line or line.startswith('#'):
                    continue
                count += 1
                valid = line.startswith('VALID ')
                # Schema-version's historical corpus uses alignment padding.
                text = line[len('VALID') if valid else len('INVALID'):].lstrip(' ')
                text, _, stored = text.partition(' => ')
                text = unescape(text)
                with self.subTest(filename=filename, text=text):
                    if valid:
                        value = carrier(text)
                        self.assertEqual(value.value, unescape(stored))
                        self.assertEqual(encode(value), json.dumps(text, ensure_ascii=False))
                        self.assertEqual(decode(json.dumps(text)), value)
                    else:
                        with self.assertRaises(ValueError): carrier(text)
                        with self.assertRaises(c.CodecError): decode(json.dumps(text))
            self.assertEqual(count, expected_count)

    def test_uuid_version_variant_cartesian(self):
        for version in '0123456789abcdefABCDEF':
            for variant in '0123456789abcdefABCDEF':
                value = f'123e4567-e89b-{version}2d3-{variant}456-426614174000'
                if version in '12345' and variant in '89abAB':
                    self.assertEqual(UUID(value).value, value)
                else:
                    with self.assertRaises(ValueError): UUID(value)
        self.assertEqual(UUID(NIL).value, NIL)
        for value in [NIL[:-1] + '1', '00000000-0000-1000-0000-000000000000',
                      '00000000-0000-0000-8000-000000000000']:
            with self.assertRaises(ValueError): UUID(value)

    def test_exact_inputs_unicode_and_spelling(self):
        class String(str): pass
        for carrier, good in [(V, VERSION), (UUID, MIXED)]:
            for bad in [None, True, 1, 1.0, b'text', [], {}, String(good)]:
                with self.assertRaises(TypeError): carrier(bad)
            for bad in [good + '\n', ' ' + good, good + '\t', good + '\0',
                        good + '\ud800', good + 'é', good + '\u0085']:
                with self.assertRaises(ValueError): carrier(bad)
            self.assertEqual(carrier(good).value, good)
        for text in ['000.001.000.000', '002.5', '002.5.0ABC',
                     '002.5.0_' + 'a' * 46, '002.5.0ab_' + 'a' * 46]:
            with self.assertRaises(ValueError): V(text)
        self.assertEqual(len(V('999.99.99zz_' + 'a' * 45).value), 57)

    def test_model_contract_and_shared_identity(self):
        for carrier, text, name in [(V, VERSION, 'SchemaVersion'),
                                     (UUID, MIXED, 'UniversallyUniqueIdentifierType')]:
            value = carrier(text)
            self.assertEqual(typing.get_type_hints(carrier)['value'], str)
            self.assertEqual(carrier.__schema_name__[1], name)
            self.assertFalse(hasattr(value, '__dict__'))
            with self.assertRaises(dataclasses.FrozenInstanceError): value.value = text
        self.assertIs(a.function_loop.exchange_input.Payload, m.Payload)
        self.assertIs(a.function_loop.exchange_other.Payload, m.Payload)
        self.assertNotEqual(a.function_loop.exchange_input.MESSAGE_NAME,
                            a.function_loop.exchange_other.MESSAGE_NAME)
        with self.assertRaises(TypeError): m.Header(version=VERSION, uuid=UUID(NIL))
        with self.assertRaises(TypeError): m.Header(version=V(VERSION), uuid=None)
        with self.assertRaises(TypeError):
            m.Payload(header=sample().header, versions=[V(VERSION)], count=1)

    def test_literal_wire_and_optional_nested_values(self):
        self.assertEqual(c.encode_Payload(sample()), WIRE)
        self.assertEqual(c.decode_Payload(WIRE), sample())
        self.assertEqual(c.encode_SchemaVersion(V(VERSION)), '"002.5.0ab_Release-2A"')
        self.assertEqual(c.encode_UniversallyUniqueIdentifierType(UUID(MIXED)),
                         '"123E4567-e89B-42d3-A456-426614174000"')
        value = m.Header(version=V('000.0.0'), uuid=UUID(NIL),
                         optionalversion=V(VERSION), optionaluuid=UUID(MIXED))
        expected = ('{"Version":"000.0.0","UUID":"' + NIL + '",'
                    '"OptionalVersion":"' + VERSION + '","OptionalUUID":"' + MIXED + '"}')
        self.assertEqual(c._dumps(c._encode_Header(value, '$')), expected)
        self.assertEqual(c._decode_Header(c._loads(expected), '$'), value)

    def test_strict_decoder_paths_later_items_and_duplicates(self):
        for bad in ['null', '1', 'true', '[]', '{}', '1.0']:
            for decode in [c.decode_SchemaVersion, c.decode_UniversallyUniqueIdentifierType]:
                with self.assertRaises(c.CodecError): decode(bad)
        for text, path in [
            (WIRE.replace('"999.99.99zz_a"', '"bad"'), '$.Versions[1]'),
            (WIRE.replace('"UUIDs":["' + NIL + '","' + MIXED + '"]',
                          '"UUIDs":["' + NIL + '",null]'), '$.UUIDs[1]'),
            (WIRE.replace('"Version":"' + VERSION + '"', '"Version":null'), '$.Header.Version'),
        ]:
            with self.assertRaises(c.CodecError) as error: c.decode_Payload(text)
            self.assertIn(path, str(error.exception))
        for text in [WIRE[:-1] + ',"$type":"Unknown"}',
                     WIRE[:-1] + ',"Unknown":0}',
                     WIRE[:-1] + ',"\\u0043ount":43}',
                     WIRE.replace('"Version":', '"Version":"000.0.0","\\u0056ersion":'),
                     WIRE.replace('"Count":42', '"Count":4.2e1'),
                     WIRE.replace(MIXED, '\\ud800'), WIRE.replace('"Count":42', '"Count":null')]:
            with self.assertRaises(c.CodecError): c.decode_Payload(text)

    def test_encoder_rechecks_corrupted_values(self):
        for carrier, encode in [(V, c.encode_SchemaVersion),
                                 (UUID, c.encode_UniversallyUniqueIdentifierType)]:
            with self.assertRaises(c.CodecError): encode(object.__new__(carrier))
            for bad in ['bad', None, 1, '\ud800']:
                value = carrier(VERSION if carrier is V else MIXED)
                object.__setattr__(value, 'value', bad)
                with self.assertRaises(c.CodecError): encode(value)
        value = sample()
        object.__setattr__(value.uuids[1], 'value', 'bad')
        with self.assertRaises(c.CodecError) as error: c.encode_Payload(value)
        self.assertIn('$.UUIDs[1]', str(error.exception))
        for bad in [(), (V(VERSION),) * 4, (V(VERSION), 'bad')]:
            value = sample()
            object.__setattr__(value, 'versions', bad)
            with self.assertRaises(c.CodecError): c.encode_Payload(value)


if __name__ == '__main__':
    unittest.main(verbosity=2)