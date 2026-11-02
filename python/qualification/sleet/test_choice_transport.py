"""PY-015 uses existing pinned-Sleet discovery, not a new CI lane."""
import importlib.util
import io
from pathlib import Path
import sys
import unittest


class ChoiceTransport(unittest.TestCase):
    def test_generated_choice_delivery_and_recovery(self):
        directory = Path(__file__).resolve().parents[3] / 'python/oms-lacal-runtime/tests'
        sys.path.insert(0, str(directory))
        try:
            spec = importlib.util.spec_from_file_location('py015_transport', directory / 'test_choice_sleet.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            suite = unittest.defaultTestLoader.loadTestsFromTestCase(module.ChoiceSleetTests)
            log = io.StringIO()
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
            self.assertEqual(result.testsRun, 1, log.getvalue())
            self.assertTrue(result.wasSuccessful(), log.getvalue())
            self.assertEqual(result.skipped, [], log.getvalue())
        finally:
            sys.path.remove(str(directory))