

import builtins as _header_builtins


def _uci_schema_version(text: str) -> None:
    # Exact Task 037 expression, not a general XSD pattern interpreter.
    if not 7 <= len(text) <= 57:
        raise ValueError('UCI schema version: expected 7..57 characters')
    head, separator, tail = text.partition('_')
    if separator and (not 1 <= len(tail) <= 45 or _header_builtins.any(
            not ('a' <= char <= 'z' or 'A' <= char <= 'Z' or
                 '0' <= char <= '9' or char == '-') for char in tail)):
        raise ValueError('UCI schema version: invalid underscore suffix')
    groups = head.split('.')
    if len(groups) != 3:
        raise ValueError('UCI schema version: expected three numeric groups')
    first, middle, last = groups
    at = 0
    while at < len(last) and '0' <= last[at] <= '9':
        at += 1
    suffix = last[at:]
    if (len(first) != 3 or not _header_builtins.all('0' <= char <= '9' for char in first)
            or not 1 <= len(middle) <= 2
            or not _header_builtins.all('0' <= char <= '9' for char in middle)
            or not 1 <= at <= 2 or len(suffix) > 2
            or not _header_builtins.all('a' <= char <= 'z' for char in suffix)):
        raise ValueError('UCI schema version: invalid numeric groups or letter suffix')


def _uci_uuid(text: str) -> None:
    # Exact Task 038 union: nil OR version [1-5], variant [89abAB].
    # No uuid.UUID conversion, normalization, or host RFC-version policy.
    if len(text) != 36:
        raise ValueError('UCI UUID: expected exactly 36 characters')
    if text == '00000000-0000-0000-0000-000000000000':
        return
    for index, char in _header_builtins.enumerate(text):
        if index in (8, 13, 18, 23):
            valid = char == '-'
        else:
            valid = '0' <= char <= '9' or 'a' <= char <= 'f' or 'A' <= char <= 'F'
        if not valid:
            raise ValueError('UCI UUID: invalid hexadecimal character or hyphen placement')
    if text[14] not in '12345' or text[19] not in '89abAB':
        raise ValueError('UCI UUID: invalid version or variant')