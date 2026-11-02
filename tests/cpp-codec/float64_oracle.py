"""CPP-OMS-004 independent numerical oracle driver for the generated C++ codec.

Expected values come only from exact integer/rational arithmetic: the unchanged
Task 096 oracle (tests/ada-float64-probe/oracle.py) for decimal -> binary64,
and the shortest-digit search below for binary64 -> decimal. Nothing here uses
host floating-point conversion, so a defect shared by the C++ encoder and
decoder cannot hide behind a round trip.

  corpus OUT [--focused]   regenerate the frozen Task 096 corpus into OUT/full
                           (a new directory), require its manifest hashes to
                           equal tests/ada-float64-probe/corpus-manifest.json,
                           and with --focused also write the PR-smoke subset
                           OUT/focused-{conversions,encoding}.tsv
  check-encoding TSV       verify every `BITS<TAB>TOKEN` row written by
                           `float64_consumer encode`
  selftest                 negative controls: the checker must reject wrong
                           values, non-shortest digits, a non-closest choice
                           and the wrong layout
"""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
PROBE = HERE.parent / "ada-float64-probe"
sys.path.insert(0, str(PROBE))
import oracle  # noqa: E402  (unchanged Task 096 oracle)

SIGN = 1 << 63
FOCUSED_ENCODING_STRIDE = 128


def decimal_exponent(value):
    """floor(log10(value)) for a positive Fraction, exactly."""
    estimate = len(str(value.numerator)) - len(str(value.denominator))
    while Fraction(10) ** estimate > value:
        estimate -= 1
    while Fraction(10) ** (estimate + 1) <= value:
        estimate += 1
    return estimate


def shortest(bits):
    """(digits, n) for a finite nonzero magnitude: the fewest significant
    decimal digits that round back to `bits` (nearest/ties-even), the closest
    such decimal if two qualify (an exact tie picks the even digit string).
    The value is 0.digits * 10**n."""
    magnitude = bits & (SIGN - 1)
    value = oracle.exact(magnitude)
    exponent = decimal_exponent(value)
    for k in range(1, 18):
        scale = Fraction(10) ** (exponent - k + 1)
        low = (value / scale).__floor__()
        found = []
        for candidate in {low, low + 1}:
            if oracle.quantize(candidate * scale) == magnitude:
                found.append(candidate)
        if found:
            found.sort(key=lambda c: (abs(c * scale - value), c % 2))
            chosen = found[0]
            text = str(chosen)
            n = len(text) + (exponent - k + 1)
            return text.rstrip("0"), n
    raise AssertionError(f"no 17-digit representation for {bits:016X}")


def ecma_layout(digits, n, negative):
    """ECMA-262 Number::toString layout, with the sign kept for -0."""
    k = len(digits)
    if k <= n <= 21:
        body = digits + "0" * (n - k)
    elif 0 < n <= 21:
        body = digits[:n] + "." + digits[n:]
    elif -6 < n <= 0:
        body = "0." + "0" * (-n) + digits
    else:
        mantissa = digits[0] + ("." + digits[1:] if k > 1 else "")
        body = mantissa + ("e+" if n - 1 >= 0 else "e-") + str(abs(n - 1))
    return ("-" if negative else "") + body


def expected_token(bits):
    if bits & (SIGN - 1) == 0:
        return "-0" if bits & SIGN else "0"
    digits, n = shortest(bits)
    return ecma_layout(digits, n, bool(bits & SIGN))


def check_row(bits, token):
    """None when `token` is the exact expected encoding of `bits`."""
    if not oracle.NUMBER.fullmatch(token):
        return "not a JSON number"
    parsed = oracle.oracle(token)
    if parsed != f"{bits:016X}":
        return f"value {parsed} != {bits:016X}"
    expected = expected_token(bits)
    if token != expected:
        return f"not the shortest/closest ECMAScript spelling (expected {expected})"
    return None


def check_encoding(path):
    rows = mismatches = 0
    for line in Path(path).read_text().splitlines():
        hexbits, token = line.split("\t")
        rows += 1
        problem = check_row(int(hexbits, 16), token)
        if problem:
            mismatches += 1
            print(f"{hexbits} {token}: {problem}", file=sys.stderr)
    print(f"CPP_OMS_004_ORACLE_ENCODING rows={rows} mismatches={mismatches}")
    return 0 if rows and not mismatches else 1


def corpus(out, focused):
    out = Path(out)
    full = out / "full"
    summary = oracle.generate(full)
    frozen = json.loads((PROBE / "corpus-manifest.json").read_text())
    if summary["hashes"] != frozen["hashes"]:
        print(f"corpus hashes {summary['hashes']} != frozen {frozen['hashes']}", file=sys.stderr)
        return 1
    print(f"CPP_OMS_004_CORPUS conversions={summary['conversion_rows']} "
          f"encodings={summary['encoding_rows']} sha256-conversions="
          f"{summary['hashes']['conversions.tsv']} sha256-encoding={summary['hashes']['encoding.tsv']}")
    if focused:
        rows = (full / "conversions.tsv").read_text().splitlines(keepends=True)
        chosen = [row for row in rows if not row.startswith("all-exponent-exact\t")]
        (out / "focused-conversions.tsv").write_text("".join(chosen))
        encodings = (full / "encoding.tsv").read_text().splitlines(keepends=True)
        sample = encodings[::FOCUSED_ENCODING_STRIDE] + [encodings[-1]]
        (out / "focused-encoding.tsv").write_text("".join(sample))
        digest = hashlib.sha256((out / "focused-conversions.tsv").read_bytes()).hexdigest()
        print(f"CPP_OMS_004_FOCUSED conversions={len(chosen)} encodings={len(sample)} "
              f"sha256-conversions={digest}")
    return 0


def selftest():
    one, tenth = 0x3FF0000000000000, 0x3FB999999999999A
    cases = {
        (one, "1"): None,
        (tenth, "0.1"): None,
        (SIGN, "-0"): None,
        (0, "0"): None,
        (1, "5e-324"): None,
        (0x444B1AE4D6E2EF50, "1e+21"): None,
        (0x3E7AD7F29ABCAF48, "1e-7"): None,
        (0x7FEFFFFFFFFFFFFF, "1.7976931348623157e+308"): None,
    }
    wrong = {
        (tenth, "0.10000000000000001"): "shortest",     # round-trips, not shortest
        (tenth, "0.09999999999999999"): "value",        # a different binary64
        (one, "1.0"): "shortest",                       # layout: no ".0"
        (SIGN, "0"): "value",                           # lost zero sign
        (0x444B1AE4D6E2EF50, "1e21"): "shortest",       # layout: exponent sign
        (0x444B1AE4D6E2EF50, "1000000000000000000000"): "shortest",
        (0x3E7AD7F29ABCAF48, "0.0000001"): "shortest",  # layout boundary
        (1, "4.9406564584124654e-324"): "shortest",
        (0x7FEFFFFFFFFFFFFF, "1.7976931348623158e+308"): "shortest",  # not closest
        (one, "NaN"): "JSON",
    }
    failures = 0
    for (bits, token), expected in {**cases, **wrong}.items():
        problem = check_row(bits, token)
        ok = problem is None if expected is None else problem is not None
        if not ok:
            failures += 1
            print(f"selftest {bits:016X} {token}: {problem}", file=sys.stderr)
    print(f"CPP_OMS_004_ORACLE_SELFTEST cases={len(cases) + len(wrong)} failures={failures}")
    return 0 if not failures else 1


def main(argv):
    if len(argv) >= 2 and argv[0] == "corpus":
        return corpus(argv[1], "--focused" in argv[2:])
    if len(argv) == 2 and argv[0] == "check-encoding":
        return check_encoding(argv[1])
    if argv == ["selftest"]:
        return selftest()
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
