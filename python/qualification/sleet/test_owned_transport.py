"""Owned extension runs the inherited real generated-code transport probe."""
import importlib.util
import io
import json
from pathlib import Path
import sys
import unittest


class OwnedTransport(unittest.TestCase):
    def test_actual_transport_and_decoder_recovery(self):
        root = Path(__file__).resolve().parents[3]
        directory = root / 'python/oms-lacal-runtime/tests'
        expected = json.loads((Path(__file__).parent / 'fixtures/test_owned_transport/required.json').read_text())
        self.assertEqual(expected, {'required_tests': 1, 'skips': 0})
        sys.path.insert(0, str(directory))
        try:
            spec = importlib.util.spec_from_file_location('ci004_actual_transport', directory / 'test_visible_ascii_sleet.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            suite = unittest.defaultTestLoader.loadTestsFromTestCase(module.VisibleAsciiSleetTests)
            log = io.StringIO()
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
            self.assertEqual(result.testsRun, expected['required_tests'], log.getvalue())
            self.assertTrue(result.wasSuccessful(), log.getvalue())
            self.assertEqual(len(result.skipped), expected['skips'], log.getvalue())
            self.assertEqual(result.expectedFailures, [], log.getvalue())
        finally:
            sys.path.remove(str(directory))