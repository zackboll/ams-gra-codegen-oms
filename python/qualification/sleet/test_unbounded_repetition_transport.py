"""PY-012 transport through the existing pinned-Sleet discovery entrypoint."""
import importlib.util
import io
from pathlib import Path
import sys
import unittest


class UnboundedRepetitionTransport(unittest.TestCase):
    def test_generated_transport_and_recovery(self):
        directory = Path(__file__).resolve().parents[3] / 'python/oms-lacal-runtime/tests'
        sys.path.insert(0, str(directory))
        try:
            spec = importlib.util.spec_from_file_location('py012_transport', directory / 'test_unbounded_repetition_sleet.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            suite = unittest.defaultTestLoader.loadTestsFromTestCase(module.UnboundedSleetTests)
            log = io.StringIO()
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
            self.assertEqual(result.testsRun, 1, log.getvalue())
            self.assertTrue(result.wasSuccessful(), log.getvalue())
            self.assertEqual(result.skipped, [], log.getvalue())
        finally:
            sys.path.remove(str(directory))