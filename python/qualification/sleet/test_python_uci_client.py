"""PY-OMS-020/021 standalone Python UCI client joins the existing pinned-Sleet lane.

That lane already supplies the shared pinned broker, both verified UCI roots,
the qualified generator and the installed LA-CAL runtime on Python 3.11/3.13,
normal and -O. Each release is generated once here with the application's own
generate_bindings.py into external temporary storage, both RDMA-only and
--with-log (PY-OMS-021); the application suites then run in fresh interpreters
(they spawn CLI and broker processes) and every registered test must run and
pass with no skips.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / 'examples/python-uci-client'
SUITES = {'test_cli.py': 13, 'test_session.py': 10, 'test_real_sleet.py': 4, 'test_wss.py': 5,
          'test_multimessage.py': 12, 'test_real_sleet_with_log.py': 4}
MARKERS = {'test_real_sleet.py': ('PY-OMS-020 REAL SLEET UCI {}: PASSED',),
           'test_wss.py': ('PY-OMS-020 WSS UCI {}: PASSED', 'PY-OMS-021 WSS UCI {}: PASSED'),
           'test_real_sleet_with_log.py': ('PY-OMS-021 REAL SLEET UCI {}: PASSED',)}


def generator():
    for candidate in (os.environ.get('AMS_GRA_GENERATOR'),
                      ROOT / 'target/release/ams-gra-codegen-oms',
                      ROOT / 'target/debug/ams-gra-codegen-oms'):
        if candidate and Path(candidate).is_file():
            return str(candidate)
    raise AssertionError('missing production ams-gra-codegen-oms generator')


class PythonUciClient(unittest.TestCase):
    def test_standalone_rdma_client_suites(self):
        with tempfile.TemporaryDirectory(prefix='py-oms-020-qualification-') as bindings:
            for release in ('2.5', '2.6'):
                root = os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT']
                for suffix, mode in (('', []), ('-log', ['--with-log'])):
                    result = subprocess.run(
                        [sys.executable, str(APP / 'generate_bindings.py'), '--schema-release', release,
                         '--schema', root, '--output', str(Path(bindings) / (release + suffix)),
                         '--generator', generator(), *mode],
                        capture_output=True, text=True, timeout=1800,
                        env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'})
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            for pattern, expected in SUITES.items():
                with self.subTest(suite=pattern):
                    result = subprocess.run(
                        [sys.executable, *(['-O'] if sys.flags.optimize else []), '-W', 'error',
                         '-m', 'unittest', 'discover', '-s', str(APP / 'tests'), '-p', pattern, '-v'],
                        cwd=APP, capture_output=True, text=True, timeout=1800,
                        env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1',
                             'PY_OMS_020_BINDINGS': bindings})
                    log = result.stdout + result.stderr
                    self.assertEqual(result.returncode, 0, log)
                    self.assertRegex(log, rf'\nRan {expected} tests? in ')
                    self.assertRegex(log, r'\nOK\n')
                    self.assertNotRegex(log, r'skipped|expected failures|unexpected success')
                    for release in ('2.5', '2.6'):
                        for marker in MARKERS.get(pattern, ()):
                            self.assertIn(marker.format(release), log)
                    print(f'PY-OMS-020/021 {pattern}: {expected} passed', flush=True)
        # The application never leaves generated UCI sources in the repository.
        status = subprocess.run(['git', 'status', '--porcelain', '--', str(APP)], cwd=ROOT,
                                capture_output=True, text=True, check=True).stdout
        self.assertEqual(status, '')
