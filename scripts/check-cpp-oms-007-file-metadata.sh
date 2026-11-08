#!/usr/bin/env bash
# Local/deep qualification only: authenticated UCI, strict toolchains,
# sanitizers, and REAL pinned Sleet via the existing C++ LA-CAL runtime.
# Usage: bash scripts/check-cpp-oms-007-file-metadata.sh SCRATCH SLEET_CHECKOUT
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m "${1:?scratch directory required}")"
checkout="$(realpath "${2:?existing pinned Sleet checkout required}")"
mkdir -p "$scratch"
cd "$repo"
[[ "$(git -C "$checkout" rev-parse HEAD)" == e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b ]]
[[ "$(git -C "$checkout" rev-parse 'HEAD^{tree}')" == 7e705f4eb5f658957bd414b4abc8207311f3c1e4 ]]
[[ -z "$(git -C "$checkout" status --porcelain)" ]]
(cd "$checkout" && cargo build --release --locked -p sleet)
sleet_bin="$checkout/target/release/sleet"
cli="${AMS_GRA_CLI:-$repo/target/release/ams-gra-codegen-oms}"
if [[ -z "${AMS_GRA_CLI:-}" ]]; then cargo build --locked --release -p ams-gra-codegen-oms; fi
root25="${AMS_GRA_UCI_2_5_ROOT:-}"
root26="${AMS_GRA_UCI_2_6_ROOT:-}"
[[ -n "$root25" ]] || root25="$(bash scripts/fetch-pinned-uci-2.5.sh "$scratch/uci25")"
[[ -n "$root26" ]] || root26="$(bash scripts/fetch-pinned-uci-2.6.sh "$scratch/uci26")"
verify() { [[ "$(sha256sum "$1" | cut -d' ' -f1)" == "$2" ]]; }
verify "$root25" ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
verify "$(dirname "$root25")/UCI_SecurityMarkings_v2_5_0.xsd" 4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
verify "$root26" af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
verify "$(dirname "$root26")/UCI_SecurityMarkings_v2_6_0.xsd" ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -Wshadow
        -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual -Wnull-dereference
        -Wdouble-promotion -Wformat=2)
sleet_pid=""
stop_sleet() {
    if [[ -n "$sleet_pid" ]]; then kill "$sleet_pid" 2>/dev/null || true; wait "$sleet_pid" 2>/dev/null || true; sleet_pid=""; fi
}
trap stop_sleet EXIT
start_sleet() {
    local root="$1" version="$2" label="$3" port
    port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
    local dir="$scratch/sleet-$label"
    mkdir -p "$dir/services"
    printf 'service_id = "cpp-oms-007"\nservice_uuid = "550e8400-e29b-41d4-a716-446655440007"\nallowed_topics = ["file.metadata"]\n' > "$dir/services/service.toml"
    cat > "$dir/server.toml" <<EOF
bind_addr = "127.0.0.1:$port"
server_id = "cpp-oms-007"
system_label = "C++ generated UCI"
schema_path = "$root"
schema_version = "$version"
services_dir = "$dir/services"
EOF
    RUST_LOG=warn "$sleet_bin" "$dir/server.toml" > "$dir/sleet.log" 2>&1 &
    sleet_pid=$!
    for ((i=0; i<600; i++)); do
        if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then sleet_url="ws://127.0.0.1:$port"; return; fi
        kill -0 "$sleet_pid" 2>/dev/null || { cat "$dir/sleet.log"; exit 1; }
        sleep .1
    done
    echo "Sleet start timed out" >&2; exit 1
}
fixture=tests/fixtures/cpp-codec/constrained-hex
"$cli" service-generate --schema "$fixture/constrained-hex.xsd" --contract "$fixture/service.yaml" \
    --language cpp --world closed-schema --with-codec --output "$scratch/synthetic"
python3 -I tests/cpp-codec/constrained_hex_mutations.py "$scratch/synthetic" \
    "$repo/tests/cpp-codec/constrained_hex_consumer.cpp" "$scratch/mutations"
configs=()
for compiler in g++ clang++; do
    command -v "$compiler" >/dev/null
    "$compiler" --version | head -1
    configs+=("$compiler strict" "$compiler asan" "$compiler tsan")
done
skips=0
failures=0
for entry in "${configs[@]}"; do
    read -r compiler flavour <<< "$entry"
    flags=("${strict[@]}" -O1 -g)
    if [[ "$flavour" == asan ]]; then flags+=(-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -fno-pie -no-pie); fi
    if [[ "$flavour" == tsan ]]; then
        flags+=(-fsanitize=thread -fno-omit-frame-pointer -fno-pie -no-pie)
        # Same established GCC/Boost.Asio TSan configuration as CPP-OMS-006.
        [[ "$compiler" != g++ ]] || flags+=(-Wno-tsan)
    fi
    label="${compiler//+/p}-$flavour"
    out="$scratch/$label"
    mkdir -p "$out"
    echo 'int main() { return 0; }' > "$out/probe.cpp"
    if ! "$compiler" "${flags[@]}" "$out/probe.cpp" -o "$out/probe" > "$out/probe-build.log" 2>&1 ||
       ! "$out/probe" > "$out/probe-run.log" 2>&1; then
        [[ "$flavour" != strict ]] || { echo "FAILED: $label compiler probe" >&2; exit 1; }
        echo "SKIPPED (not a pass): $label runtime unavailable; see $out/probe-*.log"
        skips=$((skips+1)); continue
    fi
    "$compiler" "${flags[@]}" -I "$scratch/synthetic" tests/cpp-codec/constrained_hex_consumer.cpp -o "$out/synthetic"
    summary="$("$out/synthetic")"
    [[ "$summary" == 'CPP_OMS_007_HEX checks=272 rejections=67' ]]; echo "$label $summary"
    compile_flags=()
    # -no-pie is a linker option; clang rejects it with -c under -Werror.
    for flag in "${flags[@]}"; do [[ "$flag" == -no-pie ]] || compile_flags+=("$flag"); done
    "$compiler" "${compile_flags[@]}" -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/owp.cpp -o "$out/owp.o" & p1=$!
    "$compiler" "${compile_flags[@]}" -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/runtime.cpp -o "$out/runtime.o" & p2=$!
    wait "$p1"; wait "$p2"
    for v in 25 26; do
        if [[ "$v" == 25 ]]; then root="$root25"; count=50; else root="$root26"; count=52; fi
        schema_version="002.${v#2}.0"
        contract="$scratch/loop-$v.yaml"
        sed -e "s/2.5/2.${v#2}/g" -e 's/HexNotice/FileMetadata/g' -e 's/constrained.hex/file.metadata/g' "$fixture/service.yaml" > "$contract"
        gen="$scratch/gen-$v"
        common=(--schema "$root" --contract "$contract" --language cpp --world closed-schema)
        "$cli" service-check "${common[@]}" --with-codec > "$scratch/readiness-$v.txt"
        grep -qxF "codec renderable emitted declarations: $count/$count" "$scratch/readiness-$v.txt"
        grep -qxF 'codec status: READY' "$scratch/readiness-$v.txt"
        "$cli" service-generate "${common[@]}" --with-codec --output "$gen" > /dev/null
        "$cli" service-generate "${common[@]}" --output "$gen-plain" > /dev/null
        for file in oam.hpp service_api.hpp; do cmp "$gen/$file" "$gen-plain/$file"; done
        include=(-DUCI_FILE_METADATA_VERSION="$v" -I "$gen" -I tests/cpp-codec -I tests/cpp-lacal)
        "$compiler" "${flags[@]}" "${include[@]}" tests/cpp-codec/file_metadata_consumer.cpp -o "$out/consumer-$v"
        summary="$("$out/consumer-$v")"
        [[ "$summary" == "CPP_OMS_007_FILE_METADATA uci=$v tests=2 checks=42" ]]; echo "$label $summary"
        "$compiler" "${flags[@]}" -pthread "${include[@]}" -I cpp/oms-lacal-runtime/include \
            tests/cpp-lacal/real_file_metadata_sleet.cpp "$out/owp.o" "$out/runtime.o" -o "$out/sleet-$v"
        start_sleet "$root" "$schema_version" "$v-$label"
        for test in real_file_metadata_without_hash real_file_metadata_with_hash; do
            status=0
            summary="$(CPP_OMS_007_SLEET_URL="$sleet_url" ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 \
                TSAN_OPTIONS=halt_on_error=1 timeout 120 "$out/sleet-$v" "$test" 2>"$out/$v-$test.stderr")" || status=$?
            cat "$out/$v-$test.stderr"
            if ((status != 0)); then
                echo "FAILED: $label UCI $v $test exit=$status"
                failures=$((failures+1))
            else
                [[ "$summary" == "CPP_OMS_007_REAL_FILE_METADATA uci=$v tests=1 checks=25" ]]
                echo "$label $test $summary"
            fi
        done
        stop_sleet
    done
done
echo "CPP-OMS-007 qualification completed; failures=$failures unavailable configurations=$skips (not passes)"
((failures == 0))
((skips == 0)) || exit 3