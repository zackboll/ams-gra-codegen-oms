"""PY-019 checked closed values and reconciled original-fixture contracts."""
import os
from pathlib import Path
import subprocess
import sys
import unittest


class AbstractValues(unittest.TestCase):
    def test_independent_transport_and_evidence_adversaries(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run([sys.executable, str(root / 'scripts/test_py019_evidence.py')],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Ran 2 tests', result.stderr)

    def test_synthetic_models_codecs_and_open_refusal(self):
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms',
             '--test', 'python_abstract_values'], cwd=root,
            env=dict(os.environ, PYTHON=sys.executable, PYTHONDONTWRITEBYTECODE='1'),
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('3 passed; 0 failed; 0 ignored;', result.stdout)

    def test_original_fixture_record_choice_and_output_atomicity(self):
        root = Path(__file__).resolve().parents[3]
        for package, target, count in [('ams-gra-oms-backend-python', 'models', 13),
                                       ('ams-gra-oms-backend-python', 'abstract_values', 2),
                                       ('ams-gra-codegen-oms', 'python_cli', 6),
                                       ('ams-gra-codegen-oms', 'python_inheritance', 3)]:
            with self.subTest(package=package, target=target):
                result = subprocess.run(
                    ['cargo', 'test', '--locked', '-p', package, '--test', target],
                    cwd=root, env=dict(os.environ, PYTHON=sys.executable,
                                       PYTHONDONTWRITEBYTECODE='1'),
                    capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(f'{count} passed; 0 failed; 0 ignored;', result.stdout)