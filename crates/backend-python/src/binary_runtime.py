

def _check_binary(value, lower=0, upper=None):
    # Length is measured in octets; never coerce or normalize stored values.
    _check(value, bytes)
    if len(value) < lower or (upper is not None and len(value) > upper):
        raise ValueError(f'Binary octet length outside {lower}..{upper}')