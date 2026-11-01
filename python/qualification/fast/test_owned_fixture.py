"""Fixture-only extension example: no workflow/policy changes per new task."""
import json
import importlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class OwnedFixture(unittest.TestCase):
    def test_generated_codec(self):
        root = Path(__file__).resolve().parents[3]
        fixtures = Path(__file__).parent / 'fixtures/test_owned_fixture'
        generator = root / 'target/debug/ams-gra-codegen-oms'
        with tempfile.TemporaryDirectory(prefix='ci004-python-generated-') as temporary:
            subprocess.run([str(generator), 'service-generate', '--schema', str(fixtures / 'oam.xsd'),
                            '--contract', str(fixtures / 'service.yaml'), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', temporary + '/ci004_models'],
                           check=True, capture_output=True, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))
            sys.path.insert(0, temporary)
            try:
                codec = importlib.import_module('ci004_models.service_codec')
                self.assertEqual(codec._decode_Text256('abc', '$').value, 'abc')
                for invalid in ('\n', 'é', 'x' * 257):
                    with self.assertRaises(codec.CodecError):
                        codec._decode_Text256(invalid, '$')
            finally:
                sys.path.remove(temporary)
                for module in list(sys.modules):
                    if module == 'ci004_models' or module.startswith('ci004_models.'):
                        del sys.modules[module]

    def test_required_values(self):
        path = Path(__file__).parent / 'fixtures/test_owned_fixture/values.json'
        data = json.loads(path.read_text())
        self.assertEqual(set(data), {'valid', 'invalid'})
        for value in data['valid']:
            self.assertTrue(all(0x20 <= ord(c) <= 0x7e for c in value))
        for value in data['invalid']:
            self.assertFalse(all(0x20 <= ord(c) <= 0x7e for c in value))