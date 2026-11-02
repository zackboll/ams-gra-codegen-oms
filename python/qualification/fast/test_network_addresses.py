"""PY-017 participates in the existing source-discovered Fast lane."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


class NetworkAddresses(unittest.TestCase):
    def test_generated_models_codecs_and_independent_corpora(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms', '--test', 'python_network_addresses'],
            cwd=root, env=dict(os.environ, PYTHON=sys.executable, PYTHONDONTWRITEBYTECODE='1'),
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('2 passed; 0 failed; 0 ignored;', result.stdout)