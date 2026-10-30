"""Production generated codec checks; every assertion remains active under -O."""
import dataclasses
import importlib
import json
from pathlib import Path
import sys
import unittest

# model_probe owns argument parsing and the independent shared corpus loader.
from model_probe import m, prefix, cases, sample, PAIRS

c = importlib.import_module(prefix + 'service_codec')


class Codecs(unittest.TestCase):
    def test_shared_corpus_on_both_wire_directions(self):
        for text, expected in cases():
            forged = object.__new__(m.Text256)
            object.__setattr__(forged, 'value', text)
            if expected is None:
                with self.assertRaises(c.CodecError):
                    c._decode_Text256(text, '$.Nested.Values[1]')
                with self.assertRaises(c.CodecError):
                    c._encode_Text256(forged, '$.Nested.Values[1]')
            else:
                self.assertEqual(c._decode_Text256(text, '$').value, expected)
                self.assertEqual(c._encode_Text256(forged, '$'), expected)

    def test_all_ten_codec_boundaries(self):
        for minimum, maximum in PAIRS:
            carrier = getattr(m, 'Text' + str(maximum))
            encode = getattr(c, '_encode_Text' + str(maximum))
            decode = getattr(c, '_decode_Text' + str(maximum))
            for length in [minimum - 1, minimum, maximum, maximum + 1]:
                text = ' ' * length
                forged = object.__new__(carrier)
                object.__setattr__(forged, 'value', text)
                if minimum <= length <= maximum:
                    self.assertEqual(encode(forged, '$'), text)
                    self.assertIs(type(decode(text, '$')), carrier)
                else:
                    with self.assertRaises(c.CodecError):
                        encode(forged, '$')
                    with self.assertRaises(c.CodecError):
                        decode(text, '$')

    def test_nested_inherited_optional_repeated_roundtrip_and_order(self):
        value = dataclasses.replace(sample(), optional=m.Text256(' '),
                                    restricted=m.Restricted(' ~ '),
                                    nested=m.Nested(value=m.Text256(' lead '),
                                                    values=(m.Text256('trail '), m.Text256('  '))))
        text = c.encode_Payload(value)
        self.assertEqual(c.decode_Payload(text), value)
        self.assertEqual(list(json.loads(text)),
                         ['Inherited', 'Direct', 'Named', 'Optional', 'Values', 'Nested', 'Restricted'])
        self.assertIn('"Named":"  kept  "', text)
        self.assertEqual(c.encode_Payload(value), text)

    def test_member_and_element_error_paths(self):
        body = json.loads(c.encode_Payload(sample()))
        for bad in ['', '\t', '\r', '\n', '\x00', '\x1f', '\x7f', 'é', 'x' * 257,
                    None, False, 4, [], {}]:
            body['Nested']['Values'] = ['ok', bad]
            with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Values\[1\]'):
                c.decode_Payload(json.dumps(body))
        # Strict JSON parsing rejects floats/surrogates before member traversal.
        for bad in [1.0, '\ud800']:
            body['Nested']['Values'] = ['ok', bad]
            with self.assertRaises(c.CodecError):
                c.decode_Payload(json.dumps(body))
            with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Values\[1\]'):
                c._decode_Text256(bad, '$.Nested.Values[1]')
        forged = object.__new__(m.Text256)
        for bad in ['\n', 3, None]:
            object.__setattr__(forged, 'value', bad)
            payload = dataclasses.replace(sample(), nested=m.Nested(value=m.Text256('ok'),
                                                                     values=(m.Text256('ok'), forged)))
            with self.assertRaisesRegex(c.CodecError, r'\$\.Nested\.Values\[1\]'):
                c.encode_Payload(payload)

    def test_exact_carrier_and_missing_storage(self):
        class Subclass(m.Text256):
            pass
        for bad in ['raw', m.Text20('ok'), Subclass('ok'), object.__new__(m.Text256)]:
            with self.assertRaises(c.CodecError):
                c._encode_Text256(bad, '$.Named')

    def test_strict_json_unchanged_and_recovery(self):
        valid = c.encode_Payload(sample())
        for bad in ['{"Named":"x","Named":"y"}', valid + 'x',
                    valid.replace('"Named":"  kept  "', '"Named":NaN'),
                    valid.replace('"Named":"  kept  "', '"Named":"\\ud800"')]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(bad)
        self.assertEqual(c.decode_Payload(valid), sample())


if __name__ == '__main__':
    unittest.main()