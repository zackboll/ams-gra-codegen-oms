#!/usr/bin/env python3
"""Offline authority derivation: takes the retained UnicodeData-3.1.0.txt path."""
import hashlib
from pathlib import Path
import re
import sys

source = Path(sys.argv[1]).read_bytes()
assert hashlib.sha256(source).hexdigest() == "8e57884da0da3a66782b8a6332a601fccbcae9ed0060e12501aca02fa56ffecd"
points = [int(row.split(";")[0], 16) for row in source.decode().splitlines() if row.split(";")[2] == "Nd"]
ranges = []
for point in points:
    if ranges and ranges[-1][1] + 1 == point:
        ranges[-1][1] = point
    else:
        ranges.append([point, point])
assert len(points) == 248 and len(ranges) == 21
repo = Path(__file__).resolve().parent.parent
model = (repo / "crates/codegen-core/src/unicode_string.rs").read_text().split("pub const UNICODE31_ND:")[1].split("];", 1)[0]
actual = [[int(a, 16), int(b, 16)] for a, b in re.findall(r"\(0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+)\)", model)]
assert actual == ranges, (actual, ranges)
print("TASK066 UNICODE 3.1 AUTHORITY: PASSED (248 code points, 21 ranges)")