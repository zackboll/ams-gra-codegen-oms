#!/usr/bin/env python3
"""Negative controls: weakened model, bypassed carrier, wrong case/path/parser.

Every mutant must COMPILE and then fail the independent consumer. Compile errors
are not evidence that the consumer detects a semantic regression.
"""
from pathlib import Path
import shutil
import subprocess
import sys

generated, consumer, scratch = map(Path, sys.argv[1:])
codec = (generated / "service_codec.hpp").read_text()
model = (generated / "oam.hpp").read_text()
mutations = [
    ("carrier-length", "oam.hpp", "if (octets < min_octets || octets > max_octets) return std::nullopt;", "static_cast<void>(octets);"),
    ("codec-bypass", "service_codec.hpp", "auto result = M::Digest::create(std::move(*octets));",
     "auto result = M::Digest::create(std::vector<std::uint8_t>(32, 0));"),
    ("canonical-case", "service_codec.hpp", 'static const char digits[] = "0123456789ABCDEF";',
     'static const char digits[] = "0123456789abcdef";'),
    ("error-path", "service_codec.hpp", 'return fail(error, path, "value is not valid for Digest");',
     'return fail(error, nullptr, "value is not valid for Digest");'),
    ("odd-length", "service_codec.hpp", "if (collapsed.size() % 2 != 0) {", "if (false) {"),
]
for name, filename, before, after in mutations:
    out = scratch / name
    out.mkdir(parents=True, exist_ok=True)
    for header in generated.glob("*.hpp"):
        shutil.copyfile(header, out / header.name)
    text = model if filename == "oam.hpp" else codec
    assert text.count(before) == 1, (name, text.count(before))
    (out / filename).write_text(text.replace(before, after))
    binary = out / "consumer"
    subprocess.run(["g++", "-std=c++17", "-Wall", "-Wextra", "-Werror", "-I", str(out),
                    str(consumer), "-o", str(binary)], check=True)
    result = subprocess.run([str(binary)], capture_output=True)
    assert result.returncode != 0, f"undetected mutation: {name}"
    print(f"detected: {name}")
print("CPP_OMS_007_MUTATIONS detected=5/5")