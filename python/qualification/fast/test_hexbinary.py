"""PY-013 joins existing Fast discovery; no new workflow lane."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


class HexBinary(unittest.TestCase):
    def test_generated_models_and_codecs(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms', '--test', 'python_hexbinary'],
            cwd=root, env=dict(os.environ, PYTHON=sys.executable, PYTHONDONTWRITEBYTECODE='1'),
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('2 passed; 0 failed; 0 ignored;', result.stdout)