"""CI-SELECT-003 disposable pinned-UCI-owned selection proof."""
import os
from pathlib import Path
import unittest

class AcceptanceUci(unittest.TestCase):
    def test_pinned_roots_and_generator(self):
        for version in ('2_5', '2_6'):
            root = os.environ['AMS_GRA_UCI_' + version + '_ROOT']
            self.assertTrue(Path(root).is_file())
