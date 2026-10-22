# Generated OMS JSON payload-body codec. Requires Python 3.11+.
# No global-element envelope, polymorphism, transport or general XSD validator.
from __future__ import annotations
import json as _json

if __package__:
    from . import model as _model
else:
    import model as _model


class CodecError(ValueError):
    """Invalid payload JSON or a value outside this generated codec's subset.

    Messages include a schema-derived path where available. Unrelated internal
    errors are not caught or recategorized. Decode accepts JSON text (str), not
    an already-collapsed dict: duplicate-member evidence must not be lost.
    """


def _expect(value, expected, path):
    if type(value) is not expected:
        raise CodecError(f'{path}: expected exact {expected.__name__}')
    return value


def _unicode(value, path):
    # Reject lone surrogate code points; paired JSON escapes are decoded to a
    # scalar by json.loads. Output is Unicode text, always encodable as UTF-8.
    try:
        value.encode('utf-8', errors='strict')
    except UnicodeEncodeError as error:
        raise CodecError(f'{path}: invalid Unicode scalar string') from error
    return value


def _scalar(value, path, expected, lower=None, upper=None):
    _expect(value, expected, path)
    if lower is not None and value < lower:
        raise CodecError(f'{path}: below inclusive minimum')
    if upper is not None and value > upper:
        raise CodecError(f'{path}: above inclusive maximum')
    if expected is str:
        _unicode(value, path)
    return value


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        _unicode(key, '$')
        if key in result:
            raise CodecError(f'$: duplicate member {key!r}')
        result[key] = value
    return result


def _reject_number(token):
    raise CodecError('$: non-integral or non-standard JSON number is unsupported')


def _parse_integer(token):
    # Chunk conversion preserves arbitrary precision without changing Python's
    # process-global int-string safety setting (3.11+ defaults to 4300 digits).
    negative = token.startswith('-')
    digits = token[1:] if negative else token
    value = 0
    for offset in range(0, len(digits), 9):
        chunk = digits[offset:offset + 9]
        value = value * (10 ** len(chunk)) + int(chunk)
    return -value if negative else value


def _loads(text):
    _expect(text, str, '$')
    _unicode(text, '$')
    try:
        return _json.loads(text, object_pairs_hook=_pairs, parse_int=_parse_integer,
                           parse_float=_reject_number, parse_constant=_reject_number)
    except _json.JSONDecodeError as error:
        raise CodecError(f'$: malformed JSON at line {error.lineno} column {error.colno}') from error


def _integer_text(value):
    if value == 0:
        return '0'
    negative = value < 0
    value = abs(value)
    chunks = []
    while value:
        value, remainder = divmod(value, 1_000_000_000)
        chunks.append(remainder)
    return ('-' if negative else '') + str(chunks[-1]) + ''.join(
        f'{chunk:09d}' for chunk in reversed(chunks[:-1]))


def _dumps(value):
    # Only generated schema mappings create this tree. JSON string escaping is
    # delegated to the standard library; integral formatting never uses float.
    if type(value) is int:
        return _integer_text(value)
    if type(value) in (bool, str):
        return _json.dumps(value, ensure_ascii=False, allow_nan=False)
    if type(value) is list:
        return '[' + ','.join(_dumps(item) for item in value) + ']'
    if type(value) is dict:
        return '{' + ','.join(_json.dumps(key, ensure_ascii=False) + ':' + _dumps(item)
                              for key, item in value.items()) + '}'
    raise TypeError('internal codec tree contains an unsupported type')


def _object(value, allowed, path):
    _expect(value, dict, path)
    for key in value:
        if key not in allowed:
            raise CodecError(f'{path}: unknown or unsupported member {key!r}')
    return value


def _required(value, key, path):
    if key not in value:
        raise CodecError(f'{path}: missing required member {key!r}')
    return value[key]


def _array(value, key, path, lower, upper):
    if key not in value:
        if lower:
            raise CodecError(f'{path}: missing required member {key!r}')
        return []
    return _sequence(value[key], list, path + '.' + key, lower, upper)


def _sequence(value, expected, path, lower, upper):
    _expect(value, expected, path)
    if not lower <= len(value) <= upper:
        raise CodecError(f'{path}: occurrence count outside {lower}..{upper}')
    return value