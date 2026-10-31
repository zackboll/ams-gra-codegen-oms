"""Initial mandatory Fast registration; future PY tests are auto-discovered."""
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]


class FastRegistration(unittest.TestCase):
    def test_python_and_model_sources(self):
        self.assertGreaterEqual(sys.version_info[:2], (3, 11))
        self.assertTrue((ROOT / 'crates/backend-python/src/lib.rs').is_file())
