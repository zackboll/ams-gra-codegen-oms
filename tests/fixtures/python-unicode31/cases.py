"""Independent positional boundary expectations, not a production matcher.

Also retain the unchanged independently authored Task066 UTF-8 corpus. Invalid
UTF-8 rows have no Python str representation and are not counted as Python cases.
"""
import json
from pathlib import Path


def cases():
    result = []
    corpus = Path(__file__).parents[1] / 'string/task066-corpus.tsv'
    for line in corpus.read_text().splitlines():
        if line.startswith('#'):
            continue
        profile, expected, encoded, reason = line.split('\t')
        try:
            text = bytes.fromhex(encoded).decode('utf-8')
        except UnicodeDecodeError:
            continue
        result.append((int(profile), text, bool(int(expected))))
    # Independently enumerate explicit ASCII month/day/hour/minute classes.
    # February 31 is schema-lexically valid; this is NOT calendar validation.
    for month in range(14):
        for day in range(33):
            for profile in (0, 1):
                text = f'2000{month:02}{day:02}' + ('2359' if profile == 0 else '')
                result.append((profile, text, 1 <= month <= 12 and 1 <= day <= 31))
    for hour in range(26):
        for minute in range(62):
            result.append((0, f'20000231{hour:02}{minute:02}', hour < 24 and minute < 60))
    # Deliberately authored Unicode 3.1/modern classification boundaries.
    for profile, base, slot in [(0, '200001010000', 1), (1, '20000101', 1),
                                (2, '+12.123456+012.123456', 2)]:
        for point, expected in [(0x1D7CE, True), (0x1369, True), (0x1371, True),
                                (0x1372, False), (0x1E950, False), (0xBE6, False),
                                (0xBE7, True), (0x301, False), (0xFF10, True)]:
            result.append((profile, base[:slot] + chr(point) + base[slot + 1:], expected))
        for bad in ('\u0301', '\0', '\t', '\r', '\n', '\u00a0', 'é', '🙂'):
            result.append((profile, base[:slot] + bad + base[slot + 1:], False))
        result.append((profile, base[:slot] + 'e\u0301' + base[slot + 1:], False))
    # Both coordinate alternatives and their longitude branches, ASCII-only
    # positions and punctuation are inspected independently of generated tables.
    for text in ['000000.00N0000000.00E', '895959.99S1795959.99W',
                 '+89.999999-179.999999', '-00.000000+000.000000',
                 '+1፩.𝟎١２345-17፩.123456']:
        result.append((2, text, True))
    for text in ['900000.00N0000000.00E', '006000.00N0000000.00E',
                 '000060.00N0000000.00E', '000000,00N0000000.00E',
                 '000000.00n0000000.00E', '000000.00N1800000.00E',
                 '+90.000000+000.000000', '+00.000000+180.000000',
                 '−00.000000+000.000000', ' ' * 21]:
        result.append((2, text, False))
    return result


if __name__ == '__main__':
    print(json.dumps(cases(), ensure_ascii=True))