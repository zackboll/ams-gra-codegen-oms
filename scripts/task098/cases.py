"""Task 098 independent oracle inputs: frozen Task 061 corpus + authored documents.

Nothing here validates Time. Expectations come only from the frozen corpus
(`tests/fixtures/temporal/time-zulu.txt`) and from hand-written documents; the
generated Ada model constructor remains the sole lexical authority under test.
"""
import copy
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "tests/fixtures/temporal/time-zulu.txt"
CORPUS_SHA256 = "6367d1e786939fc9e2e136a92f67c18495aea03c88fd770c21e366865d83bdee"


def unescape(field):
    """The shared Task 061 corpus escape vocabulary (backend tests/common)."""
    if field == "<empty>":
        return ""
    out, index = [], 0
    while index < len(field):
        character = field[index]
        index += 1
        if character != "\\":
            out.append(character)
            continue
        code = field[index]
        index += 1
        if code in "stnr\\":
            out.append({"s": " ", "t": "\t", "n": "\n", "r": "\r", "\\": "\\"}[code])
        elif code == "u" and field[index] == "{":
            end = field.index("}", index)
            out.append(chr(int(field[index + 1:end], 16)))
            index = end + 1
        else:
            raise RuntimeError(f"unknown corpus escape in {field!r}")
    return "".join(out)


def load_corpus():
    """[(input, expected-or-None)], verified against the historical 56/13/43 shape."""
    data = CORPUS.read_bytes()
    if hashlib.sha256(data).hexdigest() != CORPUS_SHA256:
        raise RuntimeError("frozen Task 061 Time corpus changed")
    cases = []
    for line in data.decode().splitlines():
        line = line.rstrip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("VALID "):
            text, expected = line[6:].split(" => ")
            cases.append((unescape(text), unescape(expected)))
        elif line == "INVALID":
            cases.append(("", None))
        elif line.startswith("INVALID "):
            cases.append((unescape(line[8:]), None))
        else:
            raise RuntimeError(f"malformed corpus line {line!r}")
    valid = sum(expected is not None for _, expected in cases)
    if (len(cases), valid, len(cases) - valid) != (56, 13, 43):
        raise RuntimeError(f"historical corpus shape changed: {len(cases)}/{valid}")
    return cases


class Number(str):
    """Explicit JSON-number kind tag; a quoted numeral stays a plain str."""


def decode(text):
    return json.loads(text, parse_float=Number, parse_int=Number)


def same(actual, expected):
    """Structural equality requiring identical JSON kinds and exact binary64 bits."""
    if isinstance(expected, dict):
        return (isinstance(actual, dict) and actual.keys() == expected.keys()
                and all(same(actual[key], value) for key, value in expected.items()))
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(actual) == len(expected)
                and all(same(a, e) for a, e in zip(actual, expected, strict=True)))
    if isinstance(expected, bool):
        return actual is expected
    if isinstance(expected, (Number, int, float)):
        return isinstance(actual, Number) and (
            struct.pack("!d", float(actual)) == struct.pack("!d", float(expected)))
    return type(actual) is str and actual == expected


def base_document(profile):
    """Independently authored valid payload; every Time position populated."""
    document = {
        "Required": "12:34:56Z",
        "Optional": "23:59:60.999999Z",
        "Pick": {"At": "24:00:00.000Z"},
        "Leg": {"Start": "00:00:00Z", "Stop": "12:34:56.5000Z"},
        "Bounded": ["01:02:03Z", "04:05:06.7Z", "07:08:09Z"],
        "Unbounded": ["10:11:12Z", "13:14:15.000Z"],
        "Positive": ["16:17:18Z", "19:20:21Z", "22:23:24.25Z"],
        "Legs": [{"Start": "05:06:07Z"}, {"Start": "08:09:10Z", "Stop": "11:12:13Z"}],
        "Picks": [{"At": "14:15:16Z"}, {"Label": "12:34:56+00:00"}],
    }
    if profile == "float32":
        document["Single"] = 0.5
    if profile == "lossless":
        document["Leg"]["Distance"] = 1.0000000000000002
        for leg in document["Legs"]:
            leg["Distance"] = -0.0
        document.update({
            "Stamp": "2024-02-29T12:34:56Z", "DirectStamp": "2026-10-15T01:02:03+05:30",
            "Elapsed": "P3M", "DirectElapsed": "PT2S", "Data": "00FF80",
            "Single": 0.5, "Double": 5e-324, "Ratio": 1.0,
        })
    return document


def minimum_document(profile):
    document = base_document(profile)
    for key in ["Optional", "Bounded", "Unbounded", "Legs", "Picks"]:
        document.pop(key)
    document["Leg"].pop("Stop")
    document["Positive"] = document["Positive"][:2]
    return document


# Every distinct Time composition: required, optional, Choice, nested required
# and optional, bounded, zero-minimum unbounded, positive-minimum unbounded
# (Additional part), repeated nested Record, repeated Choice.
POSITIONS = [
    ["Required"], ["Optional"], ["Pick", "At"], ["Leg", "Start"], ["Leg", "Stop"],
    ["Bounded", 1], ["Unbounded", 1], ["Positive", 2], ["Legs", 1, "Stop"], ["Picks", 0, "At"],
]


def replaced(document, path, value):
    result = copy.deepcopy(document)
    target = result
    for step in path[:-1]:
        target = target[step]
    target[path[-1]] = value
    return result


def qualify_keys(value, namespace, oam):
    if namespace == oam:
        return value
    if isinstance(value, dict):
        return {f"{{{namespace}}}{key}": qualify_keys(child, namespace, oam) for key, child in value.items()}
    if isinstance(value, list):
        return [qualify_keys(child, namespace, oam) for child in value]
    return value


def build_cases(profile, corpus):
    """[(document, expected document or None)] -- None means Codec_Error."""
    base = base_document(profile)
    cases = [(base, base), (minimum_document(profile), minimum_document(profile))]
    # The complete frozen corpus at a required member and a Choice alternative.
    for path in [["Required"], ["Pick", "At"]]:
        for text, expected in corpus:
            cases.append((replaced(base, path, text),
                          None if expected is None else replaced(base, path, expected)))
    # Every composition: XML whitespace collapse, non-Z offset, number, null.
    for path in POSITIONS:
        cases.append((replaced(base, path, " 24:00:00Z\t"), replaced(base, path, "24:00:00Z")))
        cases.append((replaced(base, path, "12:34:56+00:00"), None))
        cases.append((replaced(base, path, 123456), None))
        cases.append((replaced(base, path, None), None))
    return cases
