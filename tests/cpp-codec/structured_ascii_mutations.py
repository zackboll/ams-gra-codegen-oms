#!/usr/bin/env python3
"""CPP-OMS-006 negative controls for the structured-ASCII C++ codec qualification.

Each control copies the GENERATED headers of the shared Task 059 fixture
(tests/fixtures/service-generate/codec-structured-ascii), weakens exactly one
validation or mapping step, rebuilds tests/cpp-codec/structured_ascii_consumer.cpp
and requires the consumer to FAIL. If a control still passes, the consumer
cannot see that weakness, and this driver exits non-zero.

    python3 -I structured_ascii_mutations.py <generated-dir> <consumer.cpp> <scratch-dir> [compiler]

Prints one line per control and finally
`CPP_OMS_006_MUTATIONS detected=N/N`.
"""
import concurrent.futures
import os
import shutil
import subprocess
import sys
from pathlib import Path

FILE_NAME_STEM = ("byte == 0x2D || (byte >= 0x30 && byte <= 0x39) || (byte >= 0x41 && byte <= 0x5A) "
                  "|| byte == 0x5F || (byte >= 0x61 && byte <= 0x7A)")
FILE_NAME_EXTENSION = ("(byte >= 0x2D && byte <= 0x2E) || (byte >= 0x30 && byte <= 0x39) "
                       "|| (byte >= 0x41 && byte <= 0x5A) || byte == 0x5F || (byte >= 0x61 && byte <= 0x7A)")
DOT_LITERAL = ("if (text.size() - pos < 1 || !(static_cast<unsigned char>(text[pos + 0]) == 0x2E)) return false;\n"
               "        pos += 1;\n")
IMO_LITERAL = ("if (text.size() - pos < 3 || !(static_cast<unsigned char>(text[pos + 0]) == 0x49 && "
               "static_cast<unsigned char>(text[pos + 1]) == 0x4D && "
               "static_cast<unsigned char>(text[pos + 2]) == 0x4F)) return false;\n        pos += 3;\n")

# (label, generated file, anchor, [(exact original text, replacement), ...])
# Each original text must occur after the anchor (the first occurrence after
# it is replaced). A generator change that moves the target therefore fails
# loudly instead of silently mutating nothing.
MUTATIONS = [
    # --- the generated MODEL's validators (the codec's only authority) ---
    ("model-file-name-stem-accepts-dot", "oam.hpp", "class FileNameType {",
     [(f"if (!({FILE_NAME_STEM})) break;", f"if (!({FILE_NAME_STEM} || byte == 0x2E)) break;")]),
    ("model-file-name-empty-extension", "oam.hpp", "class FileNameType {",
     [(f"if (!({FILE_NAME_EXTENSION})) break;\n                ++pos; ++count;\n            }}\n"
       "            if (count < 1u) return false;",
       f"if (!({FILE_NAME_EXTENSION})) break;\n                ++pos; ++count;\n            }}\n")]),
    ("model-file-name-empty-stem", "oam.hpp", "class FileNameType {",
     [(f"if (!({FILE_NAME_STEM})) break;\n                ++pos; ++count;\n            }}\n"
       "            if (count < 1u) return false;",
       f"if (!({FILE_NAME_STEM})) break;\n                ++pos; ++count;\n            }}\n")]),
    ("model-file-name-no-dot-literal", "oam.hpp", "class FileNameType {", [(DOT_LITERAL, "")]),
    ("model-file-name-max-length-off-by-one", "oam.hpp", "class FileNameType {",
     [("text.size() <= 255", "text.size() <= 256")]),
    ("model-file-name-accepts-non-ascii", "oam.hpp", "class FileNameType {",
     [(f"if (!({FILE_NAME_STEM})) break;", f"if (!({FILE_NAME_STEM} || byte >= 0x80)) break;")]),
    ("model-file-name-accepts-tab", "oam.hpp", "class FileNameType {",
     [(f"if (!({FILE_NAME_EXTENSION})) break;", f"if (!({FILE_NAME_EXTENSION} || byte == 0x09)) break;")]),
    ("model-imo-no-prefix", "oam.hpp", "class ImoType {", [(IMO_LITERAL, "pos += 3;\n")]),
    ("model-imo-six-digits", "oam.hpp", "class ImoType {",
     [("if (!(text.size() == 10)) return false;", "if (!(text.size() == 9 || text.size() == 10)) return false;"),
      ("if (count < 7u) return false;", "if (count < 6u) return false;")]),
    ("model-prf-greedy-prefix", "oam.hpp", "class PrfType {",
     [("const std::size_t limit = (text.size() == 4 ? 1u : 0u);", "const std::size_t limit = 1u;")]),
    ("model-prf-never-prefix", "oam.hpp", "class PrfType {",
     [("const std::size_t limit = (text.size() == 4 ? 1u : 0u);", "const std::size_t limit = 0u;")]),
    ("model-prf-any-prefix", "oam.hpp", "class PrfType {",
     [("if (!(byte == 0x31)) break;", "if (!(byte >= 0x30 && byte <= 0x39)) break;")]),
    ("model-prf-class-widened", "oam.hpp", "class PrfType {",
     [("(byte >= 0x31 && byte <= 0x37)", "(byte >= 0x30 && byte <= 0x39)")]),
    ("model-octal-accepts-eight", "oam.hpp", "class OctalType {",
     [("(byte >= 0x30 && byte <= 0x37)", "(byte >= 0x30 && byte <= 0x38)")]),
    ("model-model-code-accepts-empty", "oam.hpp", "class ModelType {",
     [("            if (count < 1u) return false;\n", "")]),
    # --- the generated CODEC's use of the model ---
    ("codec-trims-before-create", "service_codec.hpp", None,
     [("std::optional<Carrier> checked = Carrier::create(std::string_view(*text));",
       "std::string_view trimmed(*text); while (!trimmed.empty() && trimmed.front() == ' ') trimmed.remove_prefix(1); "
       "while (!trimmed.empty() && trimmed.back() == ' ') trimmed.remove_suffix(1); "
       "std::optional<Carrier> checked = Carrier::create(trimmed);")]),
    ("codec-uppercases-before-create", "service_codec.hpp", None,
     [("std::optional<Carrier> checked = Carrier::create(std::string_view(*text));",
       "std::string upper(*text); for (char& c : upper) { if (c >= 'a' && c <= 'z') c = static_cast<char>(c - 32); } "
       "std::optional<Carrier> checked = Carrier::create(std::string_view(upper));")]),
    ("codec-number-as-text", "service_codec.hpp", None,
     [("inline const std::string* get_text(const Value& value, CodecError& error, const Path* path) {\n"
       "    if (!expect_kind(value, Value::Kind::String, error, path)) {",
       "inline const std::string* get_text(const Value& value, CodecError& error, const Path* path) {\n"
       "    if (value.kind != Value::Kind::Number && !expect_kind(value, Value::Kind::String, error, path)) {")]),
    ("codec-drops-invalid-optional", "service_codec.hpp", 'const Value* m = find_member(value, "FileName");',
     [("            f2 = decode_t002(*m, error, &p);\n            if (!f2) {\n                return std::nullopt;\n",
       "            f2 = decode_t002(*m, error, &p);\n            if (false) {\n                return std::nullopt;\n")]),
    ("codec-wrong-error-path", "service_codec.hpp", 'const Value* m = find_member(value, "FileName");',
     [('const Path p{path, "FileName", 0, false};', 'const Path p{path, "Filename", 0, false};')]),
    ("codec-encode-omits-file-name", "service_codec.hpp", None,
     [("if (value.filename) {", "if (false) {")]),
]


def mutate(text, anchor, old, new):
    start = 0
    if anchor is not None:
        start = text.find(anchor)
        if start < 0:
            raise SystemExit(f"mutation anchor not found: {anchor!r}")
    at = text.find(old, start)
    if at < 0:
        raise SystemExit(f"mutation target not found after {anchor!r}: {old!r}")
    return text[:at] + new + text[at + len(old):]


def control(generated, consumer, scratch, compiler, mutation):
    label, name, anchor, edits = mutation
    work = scratch / f"mutant-{label}"
    shutil.rmtree(work, ignore_errors=True)
    shutil.copytree(generated, work)
    target = work / name
    text = target.read_text()
    for old, new in edits:
        text = mutate(text, anchor, old, new)
    target.write_text(text)
    program = work / "consumer"
    build = subprocess.run([compiler, "-std=c++17", "-O1", "-I", str(work), str(consumer),
                            "-o", str(program)], capture_output=True, text=True)
    if build.returncode != 0:
        return label, None, f"does not compile (a control must build):\n{build.stderr}"
    run = subprocess.run([str(program)], capture_output=True, text=True)
    first = next((line for line in run.stderr.splitlines() if line.strip()), "")
    return label, run.returncode != 0, first.strip()[:120]


def main():
    if len(sys.argv) not in (4, 5):
        raise SystemExit(__doc__)
    generated, consumer, scratch = (Path(a).resolve() for a in sys.argv[1:4])
    compiler = sys.argv[4] if len(sys.argv) == 5 else "g++"
    scratch.mkdir(parents=True, exist_ok=True)
    workers = max(1, min(len(MUTATIONS), os.cpu_count() or 1))
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        results = list(pool.map(lambda mutation: control(generated, consumer, scratch, compiler, mutation),
                                MUTATIONS))
    detected = 0
    for label, caught, detail in results:
        if caught is None:
            raise SystemExit(f"mutant {label} {detail}")
        print(f"mutation control {label}: {'detected' if caught else 'NOT DETECTED'} ({detail})")
        detected += int(caught)
    print(f"CPP_OMS_006_MUTATIONS detected={detected}/{len(MUTATIONS)}")
    if detected != len(MUTATIONS):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
