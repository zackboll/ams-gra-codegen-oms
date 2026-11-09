"""PY-019 uses the existing pinned-Sleet discovery lane."""
import importlib.util
import io
from pathlib import Path
import sys
import unittest


class AbstractValuesTransport(unittest.TestCase):
    def test_known_subtypes_decoder_attribution_and_cleanup(self):
        directory = Path(__file__).resolve().parents[3] / 'python/oms-lacal-runtime/tests'
        sys.path.insert(0, str(directory))
        try:
            spec = importlib.util.spec_from_file_location('py019_transport', directory / 'test_abstract_values_sleet.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            log = io.StringIO()
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(
                unittest.defaultTestLoader.loadTestsFromTestCase(module.AbstractValuesSleetTests))
            self.assertEqual(result.testsRun, 1, log.getvalue())
            self.assertTrue(result.wasSuccessful(), log.getvalue())
            self.assertEqual(result.skipped, [], log.getvalue())
        finally:
            sys.path.remove(str(directory))