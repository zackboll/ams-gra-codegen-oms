#!/usr/bin/env python3
"""Independent authored profile witnesses; never imports production matchers.
Run only to intentionally regenerate frozen evidence from retained authority.
"""
import hashlib
from pathlib import Path
import sys

data = Path(sys.argv[1]).read_bytes()
assert hashlib.sha256(data).hexdigest() == "8e57884da0da3a66782b8a6332a601fccbcae9ed0060e12501aca02fa56ffecd"
nd = {int(line.split(";")[0], 16) for line in data.decode().splitlines() if line.split(";")[2] == "Nd"}
ranges = []
for point in sorted(nd):
    if ranges and ranges[-1][1] + 1 == point:
        ranges[-1][1] = point
    else:
        ranges.append([point, point])
assert len(nd) == 248 and len(ranges) == 21
repo = Path(__file__).resolve().parent.parent
fixture = repo / "tests/fixtures/string"
(fixture / "task066-unicode31-nd.tsv").write_text("# UnicodeData-3.1.0.txt Nd; authority SHA256 8e57884da0da3a66782b8a6332a601fccbcae9ed0060e12501aca02fa56ffecd\n" + "".join(f"{a:06X}\t{b:06X}\n" for a, b in ranges))
rows = []
def add(profile, valid, value, reason):
    if isinstance(value, str):
        value = value.encode()
    rows.append(f"{profile}\t{int(valid)}\t{value.hex()}\t{reason}")

# Expected acceptance is positional inspection against authored expressions.
# Character 1 (dates) and character 2 (coordinate decimal) are true \d slots.
for profile, base, digit_slot, ascii_slot in [(0, "200001010000", 1, 0), (1, "20000101", 1, 0), (2, "+12.123456+012.123456", 2, 1)]:
    add(profile, True, base, "authored ASCII positive")
    points = set(range(48, 58)) | {0x0661, 0x06F2, 0x1D7CE, 0x00B2, 0x2160, 0x0345, 0x41, 0x7C0, 0x104A0, 0x1E950, 0, 10, 0x10FFFF}
    for first, last in ranges:
        points.update([first, last, (first + last) // 2, first - 1, last + 1])
    for point in sorted(points):
        value = base[:digit_slot] + chr(point) + base[digit_slot + 1:]
        add(profile, point in nd, value, f"true Nd slot U+{point:06X}")
    for point in (0x0661, 0x06F2, 0x1D7CE):
        value = base[:ascii_slot] + chr(point) + base[ascii_slot + 1:]
        add(profile, False, value, "Nd in explicit ASCII class")
    for value in (base[:-1], base + "0", "X" + base, base + "X", "", " " * (len(base)-1), " " * (len(base)+1)):
        add(profile, False, value, "independent length/whole-string rejection")
    for bad in [b"\x80", b"\xC2", b"\xE0\xA0", b"\xF0\x90\x80", b"\xC0\xAF", b"\xED\xA0\x80", b"\xF4\x90\x80\x80", b"\xE2A\x80", b"\xF5\x80\x80\x80", b"\xFE", b"\xFF"]:
        add(profile, False, base[:digit_slot].encode() + bad + base[digit_slot+1:].encode(), "malformed UTF-8")
        add(profile, False, base.encode() + bad, "malformed UTF-8 at final byte")
    if profile in (0, 1):
        add(profile, True, "2١۲𝟑0101" + ("0000" if profile == 0 else ""), "mixed two/four-byte Unicode31 Nd year slots")
        add(profile, True, " " * len(base), "authored space branch")
        for month in ("01", "09", "10", "11", "12"):
            for day in ("01", "09", "10", "29", "30", "31"):
                value = "2١00" + month + day + ("2359" if profile == 0 else "")
                add(profile, True, value, "authored month/day alternatives; not calendar validity")
        for at, replacement in [(4,"2"),(5,"١"),(6,"4"),(7,"١")]:
            add(profile, False, base[:at]+replacement+base[at+1:], "explicit ASCII class violation")
        if profile == 0:
            for time in ("2400", "0060", "2١00", "000١"):
                add(profile, False, base[:8]+time, "authored time ASCII classes")
    else:
        add(profile, True, "+1١.123456+012.12345١", "two-byte Nd at final character")
        add(profile, True, "+1١.123456+012.12345𝟎", "four-byte Nd at final character")
        for value in ("000000.00N0000000.00E", "895959.99S1795959.99W", "+8١.123456-179.123456", "+12.123456+1٧2.123456"):
            add(profile, value != "+12.123456+1٧2.123456", value, "coordinate authored alternative witness")
        for at, replacement in [(0,"N"),(1,"9"),(3,","),(10,"N"),(11,"2"),(14,",")]:
            add(profile, False, base[:at]+replacement+base[at+1:], "coordinate explicit ASCII/punctuation violation")
        add(profile, False, " " * 21, "no authored space branch")
(fixture / "task066-corpus.tsv").write_text("# profile-id\texpected\tUTF8-bytes-hex\tindependent expectation attribution\n" + "\n".join(rows) + "\n")
print(f"TASK066 CORPUS FROZEN: {len(rows)} cases")