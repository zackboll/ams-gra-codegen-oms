"""Run production-check removal mutants against independent regression tests."""
from pathlib import Path
import os
import subprocess
import sys
import tempfile
import unittest


class SecurityMutations(unittest.TestCase):
    def test_workflow_and_visibility_check_removal_is_detected(self):
        here = Path(__file__).resolve().parent
        original = (here / 'gate.py').read_text()
        mutants = [
            ("require(actual == trusted, 'PR modified effective signal workflow')",
             "require(True, 'PR modified effective signal workflow')",
             'test_pr_modifies_signal_rejected'),
            ("data.get('private') is True\n                and data.get('visibility') == 'private'",
             'True', 'test_private_execution_repository_visibility_fails_closed'),
        ]
        for before, after, test in mutants:
            self.assertEqual(original.count(before), 1)
            with self.subTest(test=test), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'gate.py').write_text(original.replace(before, after))
                (root / 'test_gate.py').write_bytes((here / 'test_gate.py').read_bytes())
                result = subprocess.run([sys.executable, '-m', 'unittest',
                                         'test_gate.Authorization.' + test],
                                        cwd=root, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
                                        capture_output=True, text=True, timeout=20)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn('FAIL:', result.stderr)


if __name__ == '__main__':
    unittest.main()