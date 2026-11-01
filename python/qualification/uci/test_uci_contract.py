"""Initial mandatory pinned-UCI registration; real feature tests join here."""
import os
from pathlib import Path
import unittest


class UciRegistration(unittest.TestCase):
    def test_pinned_inputs_available(self):
        for name in ('AMS_GRA_UCI_2_5_ROOT', 'AMS_GRA_UCI_2_6_ROOT'):
            self.assertTrue(Path(os.environ[name]).is_file(), name)
