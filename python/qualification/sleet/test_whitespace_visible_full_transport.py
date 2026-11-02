"""PY-016 uses existing pinned-Sleet discovery, not a new CI lane."""
import importlib.util
import io
from pathlib import Path
import sys
import unittest


class FullUciWhitespaceVisibleTransport(unittest.TestCase):
    def test_full_log_delivery_and_recovery(self):
        directory = Path(__file__).resolve().parents[3] / 'python/oms-lacal-runtime/tests'
        sys.path.insert(0, str(directory))
        try:
            spec = importlib.util.spec_from_file_location('py016_transport', directory / 'test_whitespace_visible_full_sleet.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            suite = unittest.defaultTestLoader.loadTestsFromTestCase(module.FullUciWhitespaceVisibleSleetTests)
            log = io.StringIO()
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
            self.assertEqual(result.testsRun, 1, log.getvalue())
            self.assertTrue(result.wasSuccessful(), log.getvalue())
            self.assertEqual(result.skipped, [], log.getvalue())
        finally:
            sys.path.remove(str(directory))