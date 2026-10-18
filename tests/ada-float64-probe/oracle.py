"""Exact decimal -> IEEE binary64 nearest/ties-even; no host floating arithmetic.

The quantizer rounds an integer quotient at the binary64 lattice spacing.
The independent reference in test_oracle.py instead searches ordered encodings.
Input work is bounded by MAX_TOKEN, including exponent spellings: exponent
classification precedes construction of powers of ten. This is an oracle
resource bound, not a proposed relaxation of the JSON document limit.
"""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import random
import re
import sys

MAX_TOKEN = 16384
SIGN = 1 << 63
MAX_FINITE = 0x7FEFFFFFFFFFFFFF
NUMBER = re.compile(r"(-?)(0|[1-9][0-9]*)(?:\.([0-9]+))?(?:[eE]([+-]?[0-9]+))?\Z", re.ASCII)
sys.set_int_max_str_digits(MAX_TOKEN * 2)


def exact(bits):
    """Finite encoding to a signed exact rational (zero sign lives in bits)."""
    magnitude = bits & (SIGN - 1)
    exponent, fraction = magnitude >> 52, magnitude & ((1 << 52) - 1)
    if exponent == 2047:
        raise ValueError("nonfinite encoding")
    mantissa = fraction if exponent == 0 else fraction + (1 << 52)
    power = -1074 if exponent == 0 else exponent - 1075
    value = Fraction(mantissa << max(power, 0), 1 << max(-power, 0))
    return -value if bits & SIGN else value


def decimal(value, negative_zero=False):
    """Exact terminating decimal, supporting denominators containing only 2/5."""
    sign = "-" if value < 0 or (not value and negative_zero) else ""
    value = abs(value)
    denominator = value.denominator
    twos = fives = 0
    while denominator % 2 == 0:
        denominator //= 2
        twos += 1
    while denominator % 5 == 0:
        denominator //= 5
        fives += 1
    if denominator != 1:
        raise ValueError("nonterminating decimal")
    places = max(twos, fives)
    digits = str(value.numerator * 2 ** (places - twos) * 5 ** (places - fives))
    if not places:
        return sign + digits
    digits = digits.rjust(places + 1, "0")
    return sign + digits[:-places] + "." + digits[-places:]


def rational_token(text):
    if len(text) > MAX_TOKEN:
        raise ValueError("oracle token budget")
    match = NUMBER.fullmatch(text)
    if not match:
        raise ValueError("not a JSON number")
    sign, whole, fraction, exponent = match.groups()
    fraction = fraction or ""
    digits = (whole + fraction).lstrip("0")
    if not digits:
        return bool(sign), Fraction(0)
    exponent = exponent or "0"
    significant_exponent = exponent.lstrip("+-").lstrip("0")
    # More than six exponent digits dwarfs every admitted mantissa length.
    if len(significant_exponent) > 6:
        return bool(sign), "ZERO" if exponent.startswith("-") else "OVERFLOW"
    power = int(exponent) - len(fraction)
    adjusted = len(digits) - 1 + power
    if adjusted > 308:
        return bool(sign), "OVERFLOW"
    if adjusted < -324:
        return bool(sign), "ZERO"
    integer = int(digits)
    return bool(sign), Fraction(integer * 10 ** max(power, 0), 10 ** max(-power, 0))


def quantize(value):
    if not value:
        return 0
    numerator, denominator = value.numerator, value.denominator
    exponent = numerator.bit_length() - denominator.bit_length()
    if (numerator < denominator << exponent) if exponent >= 0 else (numerator << -exponent < denominator):
        exponent -= 1
    spacing = max(exponent - 52, -1074)
    quotient, remainder = divmod(numerator << max(-spacing, 0), denominator << max(spacing, 0))
    divisor = denominator << max(spacing, 0)
    if 2 * remainder > divisor or (2 * remainder == divisor and quotient & 1):
        quotient += 1
    if quotient == 1 << 53:
        quotient >>= 1
        spacing += 1
    if spacing > 971:
        return "OVERFLOW"
    if spacing == -1074 and quotient < 1 << 52:
        return quotient
    return ((spacing + 1075) << 52) | (quotient - (1 << 52))


def oracle(text):
    negative, value = rational_token(text)
    result = 0 if value == "ZERO" else "OVERFLOW" if value == "OVERFLOW" else quantize(value)
    return result if result == "OVERFLOW" else f"{result | (SIGN if negative else 0):016X}"


def generate(destination):
    destination.mkdir(parents=True, exist_ok=False)
    patterns = {0, 1, 2, 0xFFFFFFFFFFFFF, 0x10000000000000,
                0x3FEFFFFFFFFFFFFF, 0x3FF0000000000000, 0x3FF0000000000001,
                0x3FF0000000000002, MAX_FINITE}
    for exponent in range(2047):
        for fraction in [0, 1, (1 << 51) - 1, (1 << 52) - 2, (1 << 52) - 1]:
            patterns.add((exponent << 52) | fraction)
    encodings = sorted(patterns | {p | SIGN for p in patterns})
    rows = {}

    def add(group, token):
        rows.setdefault(token, (group, oracle(token)))

    for bits in encodings:
        add("all-exponent-exact", decimal(exact(bits), negative_zero=bool(bits & SIGN)))
    for token in ["1.0000000000000002", "1.2345678901234567", "9007199254740993", "-0", "-0.0",
                  "0.0", "1e99999999999999999999999", "-1e-99999999999999999999999",
                  "-0e99999999999999999999999", "1." + "0" * 4000 + "1",
                  "0." + "0" * 4000 + "1", "1e+0000000000000308", "1e-0000000000000324"]:
        add("explicit", token)
    rng = random.Random(9601)
    midpoints = {p for p in patterns if p < MAX_FINITE and (p >> 52) % 127 == 0}
    midpoints.update([0, 1, 2, 0xFFFFFFFFFFFFE, 0xFFFFFFFFFFFFF, 0x10000000000000,
                      0x3FEFFFFFFFFFFFFF, 0x3FF0000000000000, 0x3FF0000000000001, MAX_FINITE - 1])
    midpoints.update(rng.randrange(MAX_FINITE) for _ in range(384))
    for bits in sorted(midpoints):
        midpoint = (exact(bits) + exact(bits + 1)) / 2
        epsilon = min((exact(bits + 1) - exact(bits)) / 10, Fraction(1, 10 ** 1100))
        for value in [midpoint - epsilon, midpoint, midpoint + epsilon]:
            for sign in [1, -1]:
                add("midpoint-even" if bits % 2 == 0 else "midpoint-odd", decimal(sign * value))
    overflow = exact(MAX_FINITE) + Fraction(2) ** 970
    for value in [overflow - 1, overflow, overflow + 1]:
        for sign in [1, -1]:
            add("overflow-midpoint", decimal(sign * value))
    for power in range(-324, 309):
        for sign in ["", "-"]:
            add("power-ten", sign + "1e" + str(power))
        center = Fraction(10) ** power
        for offset in [-1, 1]:
            neighbor = center * (1 + Fraction(offset, 10 ** 20))
            for sign in [1, -1]:
                add("power-ten-neighbor", decimal(sign * neighbor))
    for center in [1 << 53, 1 << 63, (1 << 63) - 1]:
        for offset in range(-4, 5):
            for sign in [1, -1]:
                add("integer-boundary", str(sign * (center + offset)))
    (destination / "conversions.tsv").write_text("".join(
        f"{group}\t{expected}\t{token}\n" for token, (group, expected) in rows.items()))
    (destination / "encoding.tsv").write_text("".join(f"{bits:016X}\n" for bits in encodings))
    groups = {}
    for group, _ in rows.values():
        groups[group] = groups.get(group, 0) + 1
    summary = {"seed": 9601, "conversion_rows": len(rows), "encoding_rows": len(encodings),
               "finite_exponent_bands": 2047, "groups": groups,
               "hashes": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted(destination.glob("*.tsv"))}}
    (destination / "manifest.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    return summary


if __name__ == "__main__":
    print(json.dumps(generate(Path(sys.argv[1])), indent=2, sort_keys=True))