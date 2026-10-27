

# PY-008: private binary64 support, emitted only for Float64 selections.
import struct as _struct


def _float64_bound(bits):
    return _struct.unpack('>d', bits.to_bytes(8, 'big'))[0]


def _check_float64(value, lower=None, lower_inclusive=True,
                   upper=None, upper_inclusive=True):
    _check(value, float)
    # Acceptance comparisons deliberately reject NaN for every bounded domain.
    if lower is not None and not (value >= lower if lower_inclusive else value > lower):
        raise ValueError('value outside Float64 lower bound')
    if upper is not None and not (value <= upper if upper_inclusive else value < upper):
        raise ValueError('value outside Float64 upper bound')