"""DISPOSABLE NEGATIVE CONTROL: deliberately fails mandatory Fast qualification.

This test must never be merged. It verifies a selected job failure is rejected
by the independent final gate rather than treated as an allowed skip.
"""
import unittest

class AcceptanceNegative(unittest.TestCase):
    def test_expected_qualification_rejection(self):
        self.fail("CI_SELECT_003_EXPECTED_FAILURE: disposable negative acceptance")
