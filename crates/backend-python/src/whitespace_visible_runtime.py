

def _check_whitespace_visible(value, policy, lower, upper, canonical=False):
    if type(value) is not str:
        raise TypeError('expected exact str')
    text = value
    if policy == 'collapse':
        # XML Schema Part 2 section 4.3.6: only these four characters
        # participate. No raw-input length bound; facets apply afterwards.
        result = []
        pending = False
        for character in value:
            if character in ' \t\n\r':
                pending = len(result) != 0
            else:
                if pending:
                    result.append(' ')
                    pending = False
                result.append(character)
        text = ''.join(result)
    elif policy != 'preserve':
        raise ValueError('unknown whitespace-visible policy')
    if not lower <= len(text) <= upper:
        raise ValueError('whitespace-visible length out of range')
    for character in text:
        if not (' ' <= character <= '~' or character in '\n\r'):
            raise ValueError('invalid whitespace-visible character')
    if canonical and text != value:
        raise ValueError('noncanonical whitespace-visible storage')
    return text