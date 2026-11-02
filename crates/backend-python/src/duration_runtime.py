

def _check_duration(value: str) -> str:
    # XML Schema 1.0 Part 2 sections 3.2.6.1 and 4.3.6. Lexical storage only.
    # Explicit scanner: no integer conversion, arithmetic or Unicode digits.
    _check(value, str)
    replaced = value.replace('\t', ' ').replace('\n', ' ').replace('\r', ' ')
    value = ' '.join(part for part in replaced.split(' ') if part)
    end = len(value)
    cursor = 1 if value.startswith('-') else 0
    if cursor == end or value[cursor] != 'P':
        raise ValueError('invalid XML Schema duration prefix')
    cursor += 1
    time = False
    component = False
    time_component = False
    previous = -1
    while cursor < end:
        if value[cursor] == 'T' and not time:
            time = True
            previous = -1
            cursor += 1
            continue
        start = cursor
        while cursor < end and '0' <= value[cursor] <= '9':
            cursor += 1
        if cursor == start:
            raise ValueError('duration component requires ASCII digits')
        fraction = cursor < end and value[cursor] == '.'
        if fraction:
            cursor += 1
            start = cursor
            while cursor < end and '0' <= value[cursor] <= '9':
                cursor += 1
            if cursor == start:
                raise ValueError('duration fraction requires ASCII digits')
        if cursor == end:
            raise ValueError('missing duration component designator')
        designator = value[cursor]
        rank = ('HMS' if time else 'YMD').find(designator)
        if rank <= previous or (fraction and (not time or designator != 'S')):
            raise ValueError('invalid or out-of-order duration component')
        previous = rank
        component = True
        if time:
            time_component = True
        cursor += 1
    if not component or (time and not time_component):
        raise ValueError('duration requires a component after P and T')
    return value