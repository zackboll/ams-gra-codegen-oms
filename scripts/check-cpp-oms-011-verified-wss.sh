#!/usr/bin/env bash
# Usage: scripts/check-cpp-oms-011-verified-wss.sh [external-scratch-parent]
# CPP_OMS_011_OFFLINE=1 skips real UCI/Sleet admission only.
# CPP_OMS_011_SANITIZERS=1 runs ASan/UBSan and probes TSan.
# CPP_OMS_011_COMPILERS defaults to "g++-14 clang++-19".
set -euo pipefail
umask 077
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
parent="$(realpath -- "${1:-${TMPDIR:-/tmp}}")"
case "$parent/" in "$repo/"*) echo 'Use scratch outside the source tree' >&2; exit 2;; esac
scratch="$(mktemp -d "$parent/cpp-oms-011.XXXXXX")"
echo "CPP-OMS-011 evidence: $scratch"
cd "$repo"
unset CPP_OMS_011_CA CPP_OMS_011_WSS_URL SSL_CERT_FILE SSL_CERT_DIR PYTHONOPTIMIZE
read -r -a compilers <<< "${CPP_OMS_011_COMPILERS:-g++-14 clang++-19}"
strict=(-std=c++17 -O1 -pthread -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2)
includes=(-I cpp/oms-lacal-runtime/include)
tls=(-DAMS_GRA_LACAL_ENABLE_TLS=1)
ssl_libs=(-lssl -lcrypto)
cli="${AMS_GRA_CLI:-$scratch/codegen-target/release/ams-gra-codegen-oms}"
if [[ -z "${AMS_GRA_CLI:-}" ]]; then
    CARGO_TARGET_DIR="$scratch/codegen-target" cargo build --release --locked -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
fi
for service in runtime-test runtime-oam; do
    "$cli" service-generate --schema "tests/fixtures/service-generate/$service.xsd" \
        --contract "tests/fixtures/service-generate/$service.yaml" --language cpp \
        --world closed-schema --with-codec --output "$scratch/$service" >/dev/null
done

declare -A roots=() versions=()
offline="${CPP_OMS_011_OFFLINE:-0}"
sleet_rev=e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b
sleet_tree=7e705f4eb5f658957bd414b4abc8207311f3c1e4
if [[ "$offline" != 1 ]]; then
    roots[25]="${AMS_GRA_UCI_2_5_ROOT:-$(scripts/fetch-pinned-uci-2.5.sh "$scratch/uci25")}"
    roots[26]="${AMS_GRA_UCI_2_6_ROOT:-$(scripts/fetch-pinned-uci-2.6.sh "$scratch/uci26")}"
    versions=([25]=002.5.0 [26]=002.6.0)
    for release in 25 26; do
        roots[$release]="$(realpath -- "${roots[$release]}")"
        if [[ "$release" == 25 ]]; then
            root_sha=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
            markings_sha=4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
            closure=42; emitted=41
        else
            root_sha=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
            markings_sha=ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
            closure=44; emitted=43
        fi
        grep -qF "$root_sha" scripts/task102/admission.py
        grep -qF "$markings_sha" scripts/task102/admission.py
        printf '%s  %s\n' "$root_sha" "${roots[$release]}" "$markings_sha" \
            "$(dirname "${roots[$release]}")/UCI_SecurityMarkings_v2_${release:1}_0.xsd" | sha256sum --check
        common=(--schema "${roots[$release]}" --contract "tests/fixtures/task102/uci$release-service.yaml"
                --language cpp --world closed-schema --with-codec)
        "$cli" service-check "${common[@]}" > "$scratch/admission-$release.txt"
        for line in 'selected oms messages: 1' "selected type closure: $closure" 'status: READY' \
                    "codec renderable emitted declarations: $emitted/$emitted" 'codec status: READY'; do
            grep -qxF "$line" "$scratch/admission-$release.txt"
        done
        "$cli" service-generate "${common[@]}" --output "$scratch/log-$release" >/dev/null
        (cd "$scratch/log-$release"; sha256sum oam.hpp service_api.hpp service_codec.hpp)
    done
    grep -qF "$sleet_rev" scripts/run-real-sleet-test.sh
    git clone --quiet --no-hardlinks \
        "${CPP_OMS_011_SLEET_SOURCE:-https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet}" "$scratch/sleet"
    git -C "$scratch/sleet" checkout -q --detach "$sleet_rev"
    [[ "$(git -C "$scratch/sleet" rev-parse HEAD)" == "$sleet_rev" ]]
    [[ "$(git -C "$scratch/sleet" rev-parse HEAD^{tree})" == "$sleet_tree" ]]
    [[ -z "$(git -C "$scratch/sleet" status --porcelain)" ]]
    (cd "$scratch/sleet"; rustc --version; CARGO_TARGET_DIR="$scratch/sleet-target" cargo build --locked --release -p sleet)
    printf 'Sleet %s tree %s\n' "$sleet_rev" "$sleet_tree"
    sha256sum "$scratch/sleet-target/release/sleet"
else
    echo 'OFFLINE: pinned Sleet and real UCI were NOT run'
fi

sleet_pid=
stop_sleet() {
    if [[ -n "$sleet_pid" ]]; then
        kill "$sleet_pid" 2>/dev/null || true
        wait "$sleet_pid"
        sleet_pid=
    fi
}
trap 'stop_sleet || true' EXIT
start_sleet() {
    local release="$1" out="$2" i
    mkdir -p "$out/services"
    sleet_port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
    cat > "$out/services/log.toml" <<EOF_SERVICE
service_id = "task102-log"
service_uuid = "550e8400-e29b-41d4-a716-446655440102"
allowed_topics = ["task102-log"]
EOF_SERVICE
    cat > "$out/server.toml" <<EOF_SERVER
bind_addr = "127.0.0.1:$sleet_port"
server_id = "cpp-oms-011"
system_label = "CPP-OMS-011 qualification"
schema_path = "${roots[$release]}"
schema_version = "${versions[$release]}"
services_dir = "$out/services"
EOF_SERVER
    RUST_LOG=info "$scratch/sleet-target/release/sleet" "$out/server.toml" > "$out/sleet.log" 2>&1 &
    sleet_pid=$!
    for ((i=0; i<600; ++i)); do
        if (exec 3<>"/dev/tcp/127.0.0.1/$sleet_port") 2>/dev/null; then return; fi
        kill -0 "$sleet_pid" 2>/dev/null || break
        sleep 0.1
    done
    cat "$out/sleet.log" >&2
    return 1
}

build_runtime() {
    local c="$1" out="$2"; shift 2
    "$c" "$@" "${includes[@]}" -c cpp/oms-lacal-runtime/src/runtime.cpp -o "$out/runtime.o"
    "$c" "$@" "${includes[@]}" -c cpp/oms-lacal-runtime/src/owp.cpp -o "$out/owp.o"
}
build_probe() {
    local c="$1" out="$2"; shift 2
    "$c" "$@" "${includes[@]}" tests/cpp-lacal/runtime_tls_probe.cpp "$out/runtime.o" "$out/owp.o" \
        -Wl,--wrap=getaddrinfo "${ssl_libs[@]}" -o "$out/probe"
}
matrix() {
    python3 tests/cpp-lacal/verified_wss.py offline "$1/probe" "$1/matrix" | tee "$1/matrix.jsonl"
    grep -qx CPP_OMS_011_WSS_MATRIX_COMPLETE "$1/matrix.jsonl"
}

cat > "$scratch/versions.cpp" <<'EOF_VERSION'
#include <boost/version.hpp>
#include <boost/beast/version.hpp>
#include <boost/asio/version.hpp>
#include <openssl/crypto.h>
#include <iostream>
static_assert(BOOST_VERSION >= 108300, "qualify with Boost 1.83+");
int main() {
    std::cout << "Boost " << BOOST_LIB_VERSION << " (" << BOOST_VERSION << "), " << BOOST_BEAST_VERSION_STRING
              << ", Asio " << BOOST_ASIO_VERSION << ", " << OpenSSL_version(OPENSSL_VERSION) << '\n';
}
EOF_VERSION

for compiler in "${compilers[@]}"; do
    command -v "$compiler"
    "$compiler" --version | sed -n '1,2p'
    out="$scratch/$(basename "$compiler")"
    mkdir "$out"
    "$compiler" "${strict[@]}" "$scratch/versions.cpp" "${ssl_libs[@]}" -o "$out/versions"
    "$out/versions"
    build_runtime "$compiler" "$out" "${strict[@]}" "${tls[@]}"
    build_probe "$compiler" "$out" "${strict[@]}" "${tls[@]}"
    matrix "$out"
    for suite in owp_unit runtime_mock runtime_shutdown runtime_oam; do
        case "$suite" in
            owp_unit) marker='CPP_OMS_003_OWP_UNIT_COMPLETE tests=6 checks=101'; gen=runtime-test ;;
            runtime_mock) marker='CPP_OMS_003_MOCK_COMPLETE tests=30 checks=364'; gen=runtime-test ;;
            runtime_shutdown) marker='CPP_OMS_003_SHUTDOWN_COMPLETE tests=12 checks=217'; gen=runtime-test ;;
            runtime_oam) marker='CPP_OMS_003_OAM_COMPLETE tests=1 checks=13'; gen=runtime-oam ;;
        esac
        "$compiler" "${strict[@]}" "${tls[@]}" "${includes[@]}" -I "$scratch/$gen" \
            "tests/cpp-lacal/${suite}_test.cpp" "$out/runtime.o" "$out/owp.o" "${ssl_libs[@]}" -o "$out/$suite"
        timeout 120 "$out/$suite" | tee "$out/$suite.log"
        grep -qxF "$marker" "$out/$suite.log"
    done
    if [[ "$offline" != 1 ]]; then
        for release in 25 26; do
            "$compiler" "${strict[@]}" "${tls[@]}" "${includes[@]}" -I "$scratch/log-$release" \
                -DUCI_LOG_VERSION="$release" tests/cpp-lacal/real_log_sleet.cpp "$out/runtime.o" "$out/owp.o" \
                "${ssl_libs[@]}" -o "$out/log-$release"
            golden="tests/fixtures/service-generate/task093-real-log$release.json"
            start_sleet "$release" "$out/plain-$release"
            CPP_OMS_003_GOLDEN="$golden" CPP_OMS_003_SLEET_URL="ws://127.0.0.1:$sleet_port" \
                timeout 120 "$out/log-$release" | tee "$out/plain-$release.log"
            stop_sleet
            grep -qE "^CPP_OMS_003_REAL_LOG_COMPLETE uci=$release tests=2 checks=[0-9]+$" "$out/plain-$release.log"
            start_sleet "$release" "$out/wss-$release"
            python3 tests/cpp-lacal/verified_wss.py relay-log "$out/matrix/pki" "$sleet_port" "$out/log-$release" \
                "$golden" "$out/wss-$release/wire.json" "${versions[$release]}" | tee "$out/wss-$release.log"
            stop_sleet
            grep -qF "CPP_OMS_011_SLEET_WSS_COMPLETE ${versions[$release]} " "$out/wss-$release.log"
        done
    fi
done

if [[ "${CPP_OMS_011_SANITIZERS:-0}" == 1 ]]; then
    compiler="${CPP_OMS_011_SANITIZER_COMPILER:-clang++-19}"
    export ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 UBSAN_OPTIONS=print_stacktrace=1:halt_on_error=1
    export TSAN_OPTIONS=halt_on_error=1
    out="$scratch/asan-ubsan"
    mkdir "$out"
    san=(-std=c++17 -O1 -pthread -fsanitize=address,undefined -fno-omit-frame-pointer -fno-sanitize-recover=all)
    build_runtime "$compiler" "$out" "${san[@]}" "${tls[@]}"
    build_probe "$compiler" "$out" "${san[@]}" "${tls[@]}"
    matrix "$out"
    out="$scratch/tsan"
    mkdir "$out"
    san=(-std=c++17 -O1 -pthread -fsanitize=thread)
    printf 'int main() { return 0; }\n' > "$out/check.cpp"
    if "$compiler" "${san[@]}" "$out/check.cpp" -o "$out/check" && "$out/check" > "$out/startup.log" 2>&1; then
        build_runtime "$compiler" "$out" "${san[@]}" "${tls[@]}"
        build_probe "$compiler" "$out" "${san[@]}" "${tls[@]}"
        matrix "$out"
    else
        cat "$out/startup.log" 2>/dev/null || true
        echo 'TSan unavailable (not qualified)'
    fi
fi
echo 'CPP-OMS-011 qualification complete for the configurations listed above'