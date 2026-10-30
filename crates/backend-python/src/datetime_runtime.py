

# XML Schema 1.0 Part 2, sections 3.2.7, 4.3.6 and Appendix D.
# Lexical storage only: no datetime conversion, precision loss or canonicalization.
import re as _datetime_re

_datetime_lexical = _datetime_re.compile(
    r'-?([0-9]{4,})-([0-9]{2})-([0-9]{2})T'
    r'([0-9]{2}):([0-9]{2}):([0-9]{2})(?:\.([0-9]+))?'
    r'(Z|[+-][0-9]{2}:[0-9]{2})?')


def _check_datetime(value: str, zulu: bool = False) -> str:
    _check(value, str)
    # Only XML whitespace is collapsed; Unicode whitespace must remain invalid.
    replaced = value.replace('\t', ' ').replace('\n', ' ').replace('\r', ' ')
    value = ' '.join(part for part in replaced.split(' ') if part)
    match = _datetime_lexical.fullmatch(value)
    if match is None:
        raise ValueError('invalid XML Schema 1.0 dateTime lexical spelling')
    year, month, day, hour, minute, second, fraction, zone = match.groups()
    if not year.strip('0') or (len(year) > 4 and year[0] == '0'):
        raise ValueError('invalid XML Schema 1.0 dateTime year')
    # Leap-year divisibility needs only the last four digits, including BCE.
    # Never parse an unbounded year through Python's int-string safety limit.
    y = int(year[-4:])
    month, day = int(month), int(day)
    hour, minute, second = int(hour), int(minute), int(second)
    days = (31, 29 if y % 4 == 0 and (y % 100 != 0 or y % 400 == 0) else 28,
            31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
    if not 1 <= month <= 12 or not 1 <= day <= days[month - 1]:
        raise ValueError('invalid XML Schema dateTime Gregorian date')
    if hour > 24 or minute > 59 or second > 60:
        raise ValueError('invalid XML Schema dateTime clock')
    # The established Task 036 contract accepts lexical second 60 with arbitrary
    # fractions; hour 24 requires exactly zero minutes, seconds and fraction.
    if hour == 24 and (minute or second or (fraction and fraction.strip('0'))):
        raise ValueError('invalid XML Schema dateTime hour 24')
    if zone and zone != 'Z':
        zh, zm = int(zone[1:3]), int(zone[4:6])
        if zh > 14 or zm > 59 or (zh == 14 and zm != 0):
            raise ValueError('invalid XML Schema dateTime timezone')
    if zulu and zone != 'Z':
        raise ValueError('DateTime Zulu profile requires literal Z')
    return value
