#!/usr/bin/env python3
"""CPP-OMS-005 negative controls for the bounded-ASCII C++ codec qualification.

Each control copies the GENERATED headers of the synthetic fixture
(tests/fixtures/cpp-codec/bounded-ascii), weakens exactly one validation or
mapping step, rebuilds tests/cpp-codec/bounded_ascii_consumer.cpp and requires
the consumer to FAIL. A control that still passes means the consumer cannot see
that weakness, and this driver exits non-zero.

    python3 -I bounded_ascii_mutations.py <generated-dir> <consumer.cpp> <scratch-dir> [compiler]

Prints one line per control and finally
`CPP_OMS_005_MUTATIONS detected=N/N`.
"""
import concurrent.futures
import os
import shutil
import subprocess
import sys
from pathlib import Path

# (label, generated file, exact original text, replacement, which occurrence)
# Every original text is asserted to occur at least `occurrence` times, so a
# generator change that moves the target fails loudly instead of silently
# mutating nothing.
MUTATIONS = [
    # --- the generated MODEL's validator (the codec's only authority) ---
    ("model-empty-ignores-length", "oam.hpp",
     "if (!(text.size() == kLength)) {", "if (false) {", 1),          # EmptyMarkerType
    ("model-fixed-ignores-class", "oam.hpp",
     "if (!is_member(character)) {", "if (false) {", 2),              # TailType
    ("model-max-length-off-by-one", "oam.hpp",
     "text.size() <= kMaxLength", "text.size() <= kMaxLength + 1", 1),  # NameType
    ("model-min-length-off-by-one", "oam.hpp",
     "text.size() >= kMinLength", "text.size() + 1 >= kMinLength", 6),  # PinType (4..5)
    ("model-case-insensitive", "oam.hpp",
     "return (ordinal >= 0x41 && ordinal <= 0x5A);",
     "return (ordinal >= 0x41 && ordinal <= 0x5A) || (ordinal >= 0x61 && ordinal <= 0x7A);", 1),  # LaunchType
    ("model-accepts-non-ascii", "oam.hpp",
     "return (ordinal >= 0x20 && ordinal <= 0x7E);", "return ordinal >= 0x20;", 1),  # VisibleType
    ("model-accepts-control", "oam.hpp",
     "return ordinal == 0x20 ||", "return ordinal == 0x20 || ordinal == 0x09 ||", 1),  # TailType: TAB
    # --- the generated CODEC's use of the model ---
    ("codec-trims-before-create", "service_codec.hpp",
     "std::optional<Carrier> checked = Carrier::create(std::string_view(*text));",
     "std::string_view trimmed(*text); while (!trimmed.empty() && trimmed.front() == ' ') trimmed.remove_prefix(1); "
     "while (!trimmed.empty() && trimmed.back() == ' ') trimmed.remove_suffix(1); "
     "std::optional<Carrier> checked = Carrier::create(trimmed);", 1),
    ("codec-present-empty-decodes-absent", "service_codec.hpp",
     'const Value* m = find_member(value, "UnassignAll");\n        if (m != nullptr) {',
     'const Value* m = find_member(value, "UnassignAll");\n        '
     'if (m != nullptr && !(m->kind == Value::Kind::String && m->text.empty())) {', 1),
    ("codec-present-empty-encodes-absent", "service_codec.hpp",
     "if (value.unassignall) {", "if (value.unassignall && !value.unassignall->value().empty()) {", 1),
    ("codec-number-as-text", "service_codec.hpp",
     "inline const std::string* get_text(const Value& value, CodecError& error, const Path* path) {\n"
     "    if (!expect_kind(value, Value::Kind::String, error, path)) {",
     "inline const std::string* get_text(const Value& value, CodecError& error, const Path* path) {\n"
     "    if (value.kind != Value::Kind::Number && !expect_kind(value, Value::Kind::String, error, path)) {", 1),
    ("codec-wrong-error-path", "service_codec.hpp",
     'const Value* m = find_member(value, "UnassignAll");\n        if (m != nullptr) {\n'
     '            const Path p{path, "UnassignAll", 0, false};',
     'const Value* m = find_member(value, "UnassignAll");\n        if (m != nullptr) {\n'
     '            const Path p{path, "unassignall", 0, false};', 1),
    ("codec-unescaped-quote", "service_codec.hpp",
     "if (c == '\"') {\n            out += \"\\\\\\\"\";", "if (false) {\n            out += \"\\\\\\\"\";", 1),
]


def mutate(text, old, new, occurrence):
    start = -1
    for _ in range(occurrence):
        start = text.find(old, start + 1)
        if start < 0:
            raise SystemExit(f"mutation target not found ({occurrence}x): {old!r}")
    return text[:start] + new + text[start + len(old):]


def control(generated, consumer, scratch, compiler, mutation):
    label, name, old, new, occurrence = mutation
    work = scratch / f"mutant-{label}"
    shutil.rmtree(work, ignore_errors=True)
    shutil.copytree(generated, work)
    target = work / name
    target.write_text(mutate(target.read_text(), old, new, occurrence))
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
    print(f"CPP_OMS_005_MUTATIONS detected={detected}/{len(MUTATIONS)}")
    if detected != len(MUTATIONS):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
