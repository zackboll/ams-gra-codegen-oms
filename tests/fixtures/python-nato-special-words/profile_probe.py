"""Execute the original pinned declaration; isolated profile, not service readiness."""
import importlib
import json
from pathlib import Path
import sys
import unittest

# Reuse the single Task 042 corpus loader, while importing the generated model
# selected by the caller (which retains the authentic UCI declaration name).
name = sys.argv.pop(3)
from model_probe import m, prefix, cases, ALPHABET
c = importlib.import_module(prefix + 'service_codec')
carrier = getattr(m, name)
encode = getattr(c, 'encode_' + name)
decode = getattr(c, 'decode_' + name)


class PinnedProfile(unittest.TestCase):
    def test_original_declaration_corpus_on_ingress_and_egress(self):
        for text, expected in cases():
            forged = object.__new__(carrier)
            object.__setattr__(forged, 'value', text)
            if expected is None:
                with self.assertRaises(ValueError):
                    carrier(text)
                with self.assertRaises(c.CodecError):
                    decode(json.dumps(text))
                with self.assertRaises(c.CodecError):
                    encode(forged)
            else:
                self.assertEqual(carrier(text).value, expected)
                self.assertEqual(decode(json.dumps(text)).value, expected)
                self.assertEqual(json.loads(encode(forged)), expected)

    def test_original_declaration_alphabet_and_lengths(self):
        for char in ALPHABET:
            for length in [5, 6, 260, 261, 262]:
                text = 'NATO:' + char * (length - 5)
                if 6 <= length <= 261:
                    self.assertEqual(json.loads(encode(decode(json.dumps(text)))), text)
                else:
                    with self.assertRaises(c.CodecError):
                        decode(json.dumps(text))


if __name__ == '__main__':
    unittest.main()