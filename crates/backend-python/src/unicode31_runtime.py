

def _check_unicode31(value, profile):
    if type(value) is not str:
        raise TypeError("expected exact str for Unicode String")
    length, branches = _unicode31_profiles[profile]
    if len(value) != length:
        raise ValueError(f"expected {length} Unicode code points")
    for branch in branches:
        for index, token in enumerate(branch):
            character = value[index]
            if token is None:
                scalar = ord(character)
                if not any(first <= scalar <= last for first, last in _unicode31_nd):
                    break
            elif character not in token:
                break
        else:
            return
    raise ValueError("invalid Unicode 3.1 String lexical spelling")