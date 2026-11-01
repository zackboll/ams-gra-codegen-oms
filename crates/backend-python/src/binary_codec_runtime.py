

def _binary_value(value, path, lower=0, upper=None):
    _expect(value, bytes, path)
    if len(value) < lower or (upper is not None and len(value) > upper):
        raise CodecError(f'{path}: Binary octet length outside {lower}..{upper}')
    return value


def _binary_encode(value, path, lower=0, upper=None):
    return _binary_value(value, path, lower, upper).hex().upper()


def _binary_decode(value, path, lower=0, upper=None):
    _expect(value, str, path)
    # xs:hexBinary fixes whiteSpace=collapse. Surrounding XML whitespace is
    # removable; any internal XML whitespace remains invalid in its grammar.
    text = value.strip(' \t\r\n')
    if len(text) % 2:
        raise CodecError(f'{path}: odd hexBinary digit count')
    for digit in text:
        if digit not in '0123456789ABCDEFabcdef':
            raise CodecError(f'{path}: invalid hexBinary digit')
    # The permissive parser is used only AFTER enforcing the exact ASCII grammar.
    return _binary_value(bytes.fromhex(text), path, lower, upper)