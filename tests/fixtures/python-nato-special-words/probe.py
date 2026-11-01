"""Production generated codec checks; every assertion remains active under -O."""
import dataclasses
import importlib
import json
from pathlib import Path
import sys
import unittest

# model_probe owns argument parsing and the independent shared corpus loader.
from model_probe import m, prefix, cases, sample

c = importlib.import_module(prefix + 'service_codec')


class Codecs(unittest.TestCase):
    def test_shared_corpus_on_both_wire_directions(self):
        for text, expected in cases():
            forged = object.__new__(m.SpecialWord)
            object.__setattr__(forged, 'value', text)
            if expected is None:
                with self.assertRaises(c.CodecError):
                    c._decode_SpecialWord(text, '$.Nested.Values[1]')
                with self.assertRaises(c.CodecError):
                    c._encode_SpecialWord(forged, '$.Nested.Values[1]')
            else:
                self.assertEqual(c._decode_SpecialWord(text, '$').value, expected)
                self.assertEqual(c._encode_SpecialWord(forged, '$'), expected)

    def test_codec_boundaries_and_full_alphabet(self):
        from model_probe import ALPHABET
        for char in ALPHABET:
            for length in [5, 6, 260, 261, 262]:
                text = 'NATO:' + char * (length - 5)
                forged = object.__new__(m.SpecialWord)
                object.__setattr__(forged, 'value', text)
                if 6 <= length <= 261:
                    self.assertEqual(c._encode_SpecialWord(forged, '$'), text)
                    self.assertEqual(c._decode_SpecialWord(text, '$').value, text)
                else:
                    with self.assertRaises(c.CodecError):
                        c._encode_SpecialWord(forged, '$')
                    with self.assertRaises(c.CodecError):
                        c._decode_SpecialWord(text, '$')

    def test_nested_inherited_optional_repeated_roundtrip_and_order(self):
        value = dataclasses.replace(sample(), optional=m.SpecialWord('NATO:_'),
                                    restricted=m.Restricted('NATO:-_'),
                                    nested=m.Nested(value=m.SpecialWord('NATO:Lead'),
                                                    values=(m.SpecialWord('NATO:trail_'), m.SpecialWord('NATO:__'))))
        text = c.encode_Payload(value)
        self.assertEqual(c.decode_Payload(text), value)
        self.assertEqual(list(json.loads(text)),
                         ['Inherited', 'Direct', 'Named', 'Optional', 'Values', 'Nested', 'Restricted'])
        self.assertIn('"Named":"NATO:aBc-X_"', text)
        self.assertEqual(c.encode_Payload(value), text)

    def test_member_and_element_error_paths(self):
        body = json.loads(c.encode_Payload(sample()))
        for bad in ['', '\t', '\r', '\n', '\x00', '\x1f', '\x7f', 'é', 'x' * 257,
                    None, False, 4, [], {}]:
            body['Nested']['Values'] = ['NATO:ok', bad]
            with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Values\[1\]'):
                c.decode_Payload(json.dumps(body))
        # Strict JSON parsing rejects floats/surrogates before member traversal.
        for bad in [1.0, '\ud800']:
            body['Nested']['Values'] = ['NATO:ok', bad]
            with self.assertRaises(c.CodecError):
                c.decode_Payload(json.dumps(body))
            with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Values\[1\]'):
                c._decode_SpecialWord(bad, '$.Nested.Values[1]')
        forged = object.__new__(m.SpecialWord)
        for bad in ['\n', 3, None]:
            object.__setattr__(forged, 'value', bad)
            payload = dataclasses.replace(sample(), nested=m.Nested(value=m.SpecialWord('NATO:ok'),
                                                                     values=(m.SpecialWord('NATO:ok'), forged)))
            with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Values\[1\]'):
                c.encode_Payload(payload)

    def test_exact_carrier_and_missing_storage(self):
        class Subclass(m.SpecialWord):
            pass
        class StringSubclass(str):
            pass
        with self.assertRaises(c.CodecError):
            c._decode_SpecialWord(StringSubclass('NATO:A'), '$.Named')
        forged = object.__new__(m.SpecialWord)
        object.__setattr__(forged, 'value', StringSubclass('NATO:A'))
        for bad in ['raw', m.Restricted('NATO:ok'), Subclass('NATO:ok'), object.__new__(m.SpecialWord), forged]:
            with self.assertRaises(c.CodecError):
                c._encode_SpecialWord(bad, '$.Named')

    def test_strict_json_unchanged_and_recovery(self):
        valid = c.encode_Payload(sample())
        for bad in ['{"Named":"x","Named":"y"}', valid + 'x',
                    valid.replace('"Named":"NATO:aBc-X_"', '"Named":NaN'),
                    valid.replace('"Named":"NATO:aBc-X_"', '"Named":"\\ud800"')]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(bad)
        self.assertEqual(c.decode_Payload(valid), sample())


if __name__ == '__main__':
    unittest.main()