

# Demand-gated numeric ingress: never round before the schema position is known.
import math as _math


class _Number:
    __slots__ = ('token',)

    def __init__(self, token):
        self.token = token


_plain_scalar = _scalar
_plain_dumps = _dumps


def _scalar(value, path, expected, lower=None, upper=None):
    if expected is int and type(value) is _Number:
        if any(ch in value.token for ch in '.eE'):
            raise CodecError(f'{path}: expected integer JSON syntax')
        value = _parse_integer(value.token)
    return _plain_scalar(value, path, expected, lower, upper)


def _loads(text):
    _expect(text, str, '$')
    _unicode(text, '$')
    try:
        return _json.loads(text, object_pairs_hook=_pairs, parse_int=_Number,
                           parse_float=_Number, parse_constant=_reject_number)
    except _json.JSONDecodeError as error:
        raise CodecError(f'$: malformed JSON at line {error.lineno} column {error.colno}') from error


def _dumps(value):
    if type(value) is float:
        # CPython's shortest binary64 representation preserves signed zero.
        # Nonfinite values must already have become the exact OMS Strings.
        if not _math.isfinite(value):
            raise TypeError('internal codec tree contains a nonfinite number')
        return repr(value)
    return _plain_dumps(value)


def _checked_float64(value, path, lower, lower_inclusive, upper, upper_inclusive):
    try:
        _model._check_float64(value, lower, lower_inclusive, upper, upper_inclusive)
    except (TypeError, ValueError) as error:
        raise CodecError(f'{path}: {error}') from error
    return value


def _encode_float64(value, path, lower=None, lower_inclusive=True,
                    upper=None, upper_inclusive=True):
    _checked_float64(value, path, lower, lower_inclusive, upper, upper_inclusive)
    if _math.isnan(value):
        return 'NaN'
    if _math.isinf(value):
        return '-Infinity' if value < 0 else 'Infinity'
    return value


def _decode_float64(value, path, lower=None, lower_inclusive=True,
                    upper=None, upper_inclusive=True):
    if type(value) is _Number:
        # json.loads has already enforced the complete RFC 8259 number grammar.
        # Exactly one decimal -> binary64 conversion; no intermediate int/float.
        try:
            value = float(value.token)
        except (ValueError, OverflowError) as error:
            raise CodecError(f'{path}: invalid Float64 numeric conversion') from error
        if not _math.isfinite(value):
            raise CodecError(f'{path}: Float64 numeric overflow; use exact OMS special String')
    elif type(value) is str and value in ('NaN', 'Infinity', '-Infinity'):
        value = float(value)
    else:
        raise CodecError(f'{path}: expected JSON number or exact OMS Float64 special String')
    return _checked_float64(value, path, lower, lower_inclusive, upper, upper_inclusive)