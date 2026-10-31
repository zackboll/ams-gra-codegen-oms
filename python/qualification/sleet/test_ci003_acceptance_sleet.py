"""CI-SELECT-003 disposable pinned-Sleet-owned selection proof."""
import os
from pathlib import Path
import unittest

class AcceptanceSleet(unittest.TestCase):
    def test_shared_pinned_sleet_binary(self):
        self.assertTrue(Path(os.environ['AMS_GRA_SLEET_BIN']).is_file())
