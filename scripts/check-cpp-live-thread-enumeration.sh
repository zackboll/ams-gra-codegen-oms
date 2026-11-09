#!/usr/bin/env bash
# Test-only Linux task enumeration; no runtime, schema or broker changes.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$(realpath -m "${1:?scratch required}")"
mkdir -p "$out"
for compiler in g++ clang++; do
    for mode in strict asan tsan; do
        flags=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -O1 -g -pthread)
        case "$mode" in
            asan) flags+=(-fsanitize=address,undefined -fno-sanitize-recover=all -fno-pie -no-pie);;
            tsan) flags+=(-fsanitize=thread -fno-pie -no-pie);;
        esac
        label="${compiler//+/p}-$mode"
        "$compiler" "${flags[@]}" "$repo/tests/cpp-lacal/thread_enumeration_test.cpp" -o "$out/$label"
        ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 TSAN_OPTIONS=halt_on_error=1 \
            "$out/$label" > "$out/$label.stdout" 2> "$out/$label.stderr"
        grep -qxF 'CPP_LIVE_THREAD_ENUMERATION tests=6 checks=6094' "$out/$label.stdout"
        echo "$label PASS (6 tests / 6094 checks)"
    done
done