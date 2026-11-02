

def _check_ipv6(value):
    # Exact Task 062 factored XSD language, not RFC/network-stack semantics.
    if type(value) is not str:
        raise TypeError('expected exact str')
    if not 2 <= len(value) <= 45:
        raise ValueError('IPv6 length out of range')
    if not value.isascii():
        raise ValueError('invalid IPv6 non-ASCII character')

    def hex_group(text):
        if len(text) > 4:
            return False
        for character in text:
            if character not in '0123456789abcdefABCDEF':
                return False
        return True

    def hex_colon(start):
        end = start
        while end < len(value) and value[end] in '0123456789abcdefABCDEF':
            end += 1
        if end - start <= 4 and end < len(value) and value[end] == ':':
            return end + 1
        return None

    def embedded_decimal(text):
        # Deliberately different from standalone IPv4: 001 and 009 are legal.
        count = 0
        for octet in text.split('.'):
            if not 1 <= len(octet) <= 3:
                return False
            for character in octet:
                if not '0' <= character <= '9':
                    return False
            if len(octet) == 3 and octet > '255':
                return False
            count += 1
        return count == 4

    def suffix(start):
        text = value[start:]
        if hex_group(text) or text == ':':
            return True
        end = hex_colon(start)
        if end is not None:
            tail = value[end:]
            if hex_group(tail) or tail == ':':
                return True
        return embedded_decimal(text)

    def after_prefix(start):
        for count in (0, 1, 2, 3, 4, 5):
            if suffix(start):
                return True
            if count == 5:
                break
            end = hex_colon(start)
            if end is None:
                break
            start = end
        return False

    end = hex_colon(0)
    if ((end is not None and after_prefix(end))
            or (value.startswith('::') and after_prefix(2))):
        return value
    raise ValueError('invalid IPv6 lexical spelling')