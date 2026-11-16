"""PY-OMS-020 standalone Python UCI client joins the existing pinned-Sleet lane.

That lane already supplies the shared pinned broker, both verified UCI roots,
the qualified generator and the installed LA-CAL runtime on Python 3.11/3.13,
normal and -O. The application suite is executed in a fresh interpreter (it
spawns the CLI and broker processes) and every test must run and pass.
"""
import os
from pathlib import Path
import re
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / 'examples/python-uci-client'
SUITES = {'test_cli.py': 9, 'test_session.py': 10, 'test_real_sleet.py': 4, 'test_wss.py': 4}


class PythonUciClient(unittest.TestCase):
    def test_standalone_rdma_client_suites(self):
        for pattern, expected in SUITES.items():
            with self.subTest(suite=pattern):
                result = subprocess.run(
                    [sys.executable, *(['-O'] if sys.flags.optimize else []), '-W', 'error',
                     '-m', 'unittest', 'discover', '-s', str(APP / 'tests'), '-p', pattern, '-v'],
                    cwd=APP, capture_output=True, text=True, timeout=900,
                    env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'})
                log = result.stdout + result.stderr
                self.assertEqual(result.returncode, 0, log)
                self.assertRegex(log, rf'\nRan {expected} tests? in ')
                self.assertRegex(log, r'\nOK\n')
                self.assertNotRegex(log, r'skipped|expected failures|unexpected success', log)
                if pattern == 'test_real_sleet.py':
                    for release in ('2.5', '2.6'):
                        self.assertIn(f'PY-OMS-020 REAL SLEET UCI {release}: PASSED', log)
                if pattern == 'test_wss.py':
                    for release in ('2.5', '2.6'):
                        self.assertIn(f'PY-OMS-020 WSS UCI {release}: PASSED', log)
                print(re.sub(r'\s+', ' ', f'PY-OMS-020 {pattern}: {expected} passed'), flush=True)
        # The application never leaves generated UCI sources in the repository.
        status = subprocess.run(['git', 'status', '--porcelain', '--', str(APP)], cwd=ROOT,
                                capture_output=True, text=True, check=True).stdout
        self.assertEqual(status, '')
