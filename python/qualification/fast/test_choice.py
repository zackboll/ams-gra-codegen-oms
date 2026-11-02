"""PY-015 registers with the existing source-discovered Fast lane."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


class Choice(unittest.TestCase):
    def test_generated_models_and_codecs(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms', '--test', 'python_choice'],
            cwd=root, env=dict(os.environ, PYTHON=sys.executable, PYTHONDONTWRITEBYTECODE='1'),
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('3 passed; 0 failed; 0 ignored;', result.stdout)

    def test_authenticated_service_delta(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run([sys.executable, str(root / 'scripts/test-py015-uci.py')],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Ran 4 tests', result.stderr)