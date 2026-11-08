"""PY-018 joins existing Fast discovery without a new CI lane."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


class Unicode31Fast(unittest.TestCase):
    def test_separate_acceptance_gate_negative_controls(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run([sys.executable, str(root / 'scripts/test-py018-uci.py')],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Ran 3 tests', result.stderr)

    def test_independent_languages_models_codecs_and_boundaries(self):
        root = Path(__file__).resolve().parents[3]
        for package, target in [('ams-gra-oms-backend-python', 'unicode_strings'),
                                ('ams-gra-codegen-oms', 'python_unicode_strings')]:
            result = subprocess.run(['cargo', 'test', '--locked', '-p', package, '--test', target],
                                    cwd=root, env=dict(os.environ, PYTHON=sys.executable,
                                                       PYTHONDONTWRITEBYTECODE='1'),
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('2 passed; 0 failed; 0 ignored;', result.stdout)