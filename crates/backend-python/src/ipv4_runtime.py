

def _check_ipv4(value):
    if type(value) is not str:
        raise TypeError('expected exact str')
    if not 7 <= len(value) <= 15:
        raise ValueError('IPv4 length out of range')
    count = 0
    for octet in value.split('.'):
        if not 1 <= len(octet) <= 3:
            raise ValueError('invalid IPv4 octet width')
        for character in octet:
            if not '0' <= character <= '9':
                raise ValueError('invalid IPv4 decimal character')
        if len(octet) > 1 and octet[0] == '0':
            raise ValueError('IPv4 leading zero')
        if len(octet) == 3 and octet > '255':
            raise ValueError('IPv4 octet out of range')
        count += 1
    if count != 4:
        raise ValueError('expected four IPv4 octets')
    return value