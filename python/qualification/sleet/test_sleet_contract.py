"""Initial mandatory pinned-Sleet registration; no second broker build."""
import os
from pathlib import Path
import unittest


class SleetRegistration(unittest.TestCase):
    def test_shared_pinned_broker_available(self):
        self.assertTrue(Path(os.environ['AMS_GRA_SLEET_BIN']).is_file())
