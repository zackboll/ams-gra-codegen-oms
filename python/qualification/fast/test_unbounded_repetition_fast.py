"""PY-012 uses the existing main Fast discovery, without new PR gates."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


class UnboundedRepetition(unittest.TestCase):
    def test_generated_models_and_codecs(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms',
             '--test', 'python_unbounded_repetition', '--', '--nocapture'], cwd=root,
            env=dict(os.environ, PYTHON=sys.executable, PYTHONDONTWRITEBYTECODE='1'),
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('3 passed; 0 failed; 0 ignored;', result.stdout)

    def test_versioned_readiness_evidence(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run([sys.executable, str(root / 'scripts/test-py012-uci.py')],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Ran 2 tests', result.stderr)
        self.assertIn('\nOK\n', result.stderr)