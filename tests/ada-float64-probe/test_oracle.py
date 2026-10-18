"""Independent binary-search checks and explicit tie/overflow expectations."""
from fractions import Fraction
import random
import unittest
import oracle


def reference(value):
    if value >= oracle.exact(oracle.MAX_FINITE) + Fraction(2) ** 970:
        return "OVERFLOW"
    lower, upper = 0, oracle.MAX_FINITE
    while lower < upper:
        middle = (lower + upper + 1) // 2
        if oracle.exact(middle) <= value:
            lower = middle
        else:
            upper = middle - 1
    if lower < oracle.MAX_FINITE:
        down = value - oracle.exact(lower)
        up = oracle.exact(lower + 1) - value
        if up < down or (up == down and lower & 1):
            lower += 1
    return lower


class OracleTests(unittest.TestCase):
    def test_fixed(self):
        for token, expected in {
            "-0": "8000000000000000", "-0.0": "8000000000000000",
            "1": "3FF0000000000000", "1.0000000000000002": "3FF0000000000001",
            "1.2345678901234567": "3FF3C0CA428C59FB",
            "9007199254740993": "4340000000000000",
            "1e999999999999999999999": "OVERFLOW",
            "-1e-99999999999999999999": "8000000000000000",
            "-0e99999999999999999999": "8000000000000000",
        }.items():
            self.assertEqual(oracle.oracle(token), expected, token)

    def test_ties(self):
        for bits in [0, 1, 2, 0xFFFFFFFFFFFFE, 0xFFFFFFFFFFFFF,
                     0x3FF0000000000000, 0x3FF0000000000001, oracle.MAX_FINITE - 1]:
            midpoint = (oracle.exact(bits) + oracle.exact(bits + 1)) / 2
            expected = bits + (bits & 1)
            self.assertEqual(oracle.quantize(midpoint), expected)
            self.assertEqual(oracle.oracle(oracle.decimal(midpoint)), f"{expected:016X}")
        boundary = oracle.exact(oracle.MAX_FINITE) + Fraction(2) ** 970
        self.assertEqual(oracle.quantize(boundary - 1), oracle.MAX_FINITE)
        self.assertEqual(oracle.quantize(boundary), "OVERFLOW")

    def test_independent_search(self):
        rng = random.Random(9602)
        for _ in range(400):
            bits = rng.randrange(oracle.MAX_FINITE)
            low, high = oracle.exact(bits), oracle.exact(bits + 1)
            for value in [low, (low + high) / 2, (low + 3 * high) / 4]:
                self.assertEqual(oracle.quantize(value), reference(value))

    def test_budget_and_syntax(self):
        for token in ["+1", " 1", "01", "1.", "NaN", "0x1p0", "1x", "1e", "1" * 16385]:
            with self.assertRaises(ValueError, msg=token[:30]):
                oracle.oracle(token)
        # Exponent alone must not determine overflow: long mantissas compensate.
        self.assertEqual(oracle.oracle("0." + "0" * 1000 + "1e1001"), "3FF0000000000000")


if __name__ == "__main__":
    unittest.main()