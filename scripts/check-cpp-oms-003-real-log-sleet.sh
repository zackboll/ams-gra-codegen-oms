#!/usr/bin/env bash
# CPP-OMS-003 Stage C: authentic UCI `Log` publish/subscribe interoperability
# of the C++ LA-CAL runtime with the UNMODIFIED pinned Sleet broker, for UCI
# 2.5 AND UCI 2.6.
#
#   1. obtains the pinned UCI roots with the repository's fetch scripts (or
#      AMS_GRA_UCI_2_{5,6}_ROOT) and verifies root + SecurityMarkings SHA-256
#      against scripts/task102/admission.py;
#   2. checks out Sleet at the existing pin (e38f61d8, tree 7e705f4e), requires
#      a clean tree, and builds it with Sleet's own toolchain file, --locked;
#   3. per release: production `service-check --with-codec` on the unchanged
#      Task 102 contract must report closure 42|44, codec 41/41|43/43 READY;
#      `service-generate --with-codec` emits the headers the test compiles;
#   4. compiles tests/cpp-lacal/real_log_sleet.cpp (strict g++, strict clang++
#      when installed, g++ ASan+UBSan when usable), and runs EACH binary against
#      its own freshly started Sleet loaded with that release's full schema,
#      requiring every PASSED marker and the exact summary line.
#
# Usage: scripts/check-cpp-oms-003-real-log-sleet.sh [scratch-dir] [existing-sleet-checkout]
# Network is needed for the UCI/Sleet fetch unless both are supplied.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"

SLEET_REPO=https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet
SLEET_REV=e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b
SLEET_TREE=7e705f4eb5f658957bd414b4abc8207311f3c1e4
ROOT_25=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
MARK_25=4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
ROOT_26=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
MARK_26=ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
for pin in "$ROOT_25" "$MARK_25" "$ROOT_26" "$MARK_26"; do
  grep -q "$pin" scripts/task102/admission.py \
    || { echo "pin $pin is not in scripts/task102/admission.py" >&2; exit 1; }
done
grep -q "$SLEET_REV" scripts/run-real-sleet-test.sh \
  || { echo "Sleet pin differs from scripts/run-real-sleet-test.sh" >&2; exit 1; }

verify() { # file expected-sha256 label
  local actual
  actual="$(sha256sum "$1" | cut -d' ' -f1)"
  [[ "$actual" == "$2" ]] || { echo "$3 SHA-256 $actual != $2" >&2; exit 1; }
}

# ---- 1. pinned UCI ------------------------------------------------------------
root25="${AMS_GRA_UCI_2_5_ROOT:-}"
root26="${AMS_GRA_UCI_2_6_ROOT:-}"
[[ -n "$root25" ]] || root25="$(scripts/fetch-pinned-uci-2.5.sh "$scratch/uci25-checkout")"
[[ -n "$root26" ]] || root26="$(scripts/fetch-pinned-uci-2.6.sh "$scratch/uci26-checkout")"
root25="$(realpath "$root25")"; root26="$(realpath "$root26")"
verify "$root25" "$ROOT_25" "UCI 2.5 MessageDefinitions"
verify "$(dirname "$root25")/UCI_SecurityMarkings_v2_5_0.xsd" "$MARK_25" "UCI 2.5 SecurityMarkings"
verify "$root26" "$ROOT_26" "UCI 2.6 MessageDefinitions"
verify "$(dirname "$root26")/UCI_SecurityMarkings_v2_6_0.xsd" "$MARK_26" "UCI 2.6 SecurityMarkings"
echo "pinned UCI roots and SecurityMarkings verified"

# ---- 2. pinned, unmodified Sleet --------------------------------------------------
checkout="${2:-$scratch/sleet}"
if [[ ! -d "$checkout/.git" ]]; then
  git clone --quiet "$SLEET_REPO" "$checkout"
fi
git -C "$checkout" checkout --quiet --detach "$SLEET_REV"
[[ "$(git -C "$checkout" rev-parse HEAD)" == "$SLEET_REV" ]]
[[ "$(git -C "$checkout" rev-parse 'HEAD^{tree}')" == "$SLEET_TREE" ]]
[[ -z "$(git -C "$checkout" status --porcelain)" ]] || { echo "Sleet checkout is modified" >&2; exit 1; }
(cd "$checkout" && cargo build --release --locked -p sleet)
sleet_bin="$checkout/target/release/sleet"
echo "Sleet $SLEET_REV (tree $SLEET_TREE) clean; built with $(cd "$checkout" && rustc --version)"

# ---- 3. production readiness and generation ------------------------------------------
cli="${AMS_GRA_CLI:-}"
if [[ -z "$cli" ]]; then
  cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
  cli="$repo/target/release/ams-gra-codegen-oms"
fi
expect() { grep -qxF "$2" <<<"$1" || { echo "missing report line ($3): $2" >&2; echo "$1" >&2; exit 1; }; }

free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'; }

sleet_pid=""
sleet_dir=""
stop_sleet() {
  if [[ -n "$sleet_pid" ]]; then
    kill "$sleet_pid" 2>/dev/null || true
    wait "$sleet_pid" 2>/dev/null || true
    sleet_pid=""
  fi
  if [[ -n "$sleet_dir" ]]; then
    rm -rf "$sleet_dir"
    sleet_dir=""
  fi
}
trap stop_sleet EXIT

start_sleet() { # schema-root schema-version label
  sleet_dir="$scratch/sleet-run-$3"
  rm -rf "$sleet_dir"
  mkdir -p "$sleet_dir/services"
  cat > "$sleet_dir/services/service.toml" <<EOF
service_id = "task102-log"
service_uuid = "550e8400-e29b-41d4-a716-446655440102"
allowed_topics = ["task102-log"]
EOF
  local port
  port="$(free_port)"
  cat > "$sleet_dir/server.toml" <<EOF
bind_addr = "127.0.0.1:$port"
server_id = "cpp-oms-003"
system_label = "C++ generated UCI"
schema_path = "$1"
schema_version = "$2"
services_dir = "$sleet_dir/services"
EOF
  RUST_LOG=warn "$sleet_bin" "$sleet_dir/server.toml" >"$sleet_dir/sleet.log" 2>&1 &
  sleet_pid=$!
  local i
  for ((i = 0; i < 600; i++)); do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      sleet_url="ws://127.0.0.1:$port"
      return 0
    fi
    kill -0 "$sleet_pid" 2>/dev/null || { cat "$sleet_dir/sleet.log" >&2; echo "Sleet exited" >&2; exit 1; }
    sleep 0.1
  done
  echo "Sleet did not listen within 60 s" >&2
  exit 1
}

strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2)
asan=(-std=c++17 -g -fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer)
configs=("gcc-strict g++ strict")
if command -v clang++ >/dev/null 2>&1; then
  configs+=("clang-strict clang++ strict")
else
  echo "SKIPPED (not claimed as a pass): clang++ not installed"
fi
printf 'int main() { return 0; }\n' > "$scratch/usable.cpp"
if g++ "${asan[@]}" "$scratch/usable.cpp" -o "$scratch/usable" >/dev/null 2>&1 && "$scratch/usable"; then
  configs+=("gcc-asan-ubsan g++ asan")
else
  echo "SKIPPED (not claimed as a pass): ASan/UBSan unusable with g++"
fi

passed=()
for release in 2.5 2.6; do
  version="${release/./}"
  if [[ "$release" == 2.5 ]]; then root="$root25"; closure=42; emitted=41; else root="$root26"; closure=44; emitted=43; fi
  schema_version="002.${release#2.}.0"
  contract="tests/fixtures/task102/uci$version-service.yaml"
  common=(--schema "$root" --contract "$contract" --language cpp --world closed-schema)
  echo "== UCI $release Log"
  report="$("$cli" service-check "${common[@]}" --with-codec)"
  expect "$report" "selected oms messages: 1" "$release"
  expect "$report" "selected type closure: $closure" "$release"
  expect "$report" "status: READY" "$release"
  expect "$report" "codec renderable emitted declarations: $emitted/$emitted" "$release"
  expect "$report" "codec status: READY" "$release"
  gen="$scratch/gen-log$version"
  rm -rf "$gen"
  "$cli" service-generate "${common[@]}" --with-codec --output "$gen" >/dev/null
  (cd "$gen" && sha256sum oam.hpp service_api.hpp service_codec.hpp | sed 's/^/  sha256 /')
  grep -q 'message_name = "Log";' "$gen/service_api.hpp"

  for entry in "${configs[@]}"; do
    read -r name compiler flavour <<<"$entry"
    if [[ "$flavour" == strict ]]; then flags=("${strict[@]}"); else flags=("${asan[@]}"); fi
    out="$scratch/build-$version-$name"
    mkdir -p "$out"
    "$compiler" "${flags[@]}" -O1 -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/owp.cpp -o "$out/owp.o" &
    p1=$!
    "$compiler" "${flags[@]}" -O1 -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/runtime.cpp -o "$out/runtime.o" &
    p2=$!
    "$compiler" "${flags[@]}" -O1 -pthread "-DUCI_LOG_VERSION=$version" -I cpp/oms-lacal-runtime/include -I "$gen" \
      -c tests/cpp-lacal/real_log_sleet.cpp -o "$out/test.o"
    wait "$p1"; wait "$p2"
    "$compiler" "${flags[@]}" -pthread "$out/test.o" "$out/owp.o" "$out/runtime.o" -o "$out/real_log_sleet"

    start_sleet "$root" "$schema_version" "$version-$name"
    status=0
    output="$(CPP_OMS_003_SLEET_URL="$sleet_url" \
      CPP_OMS_003_GOLDEN="tests/fixtures/service-generate/task093-real-log$version.json" \
      ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 timeout 600 "$out/real_log_sleet" 2>"$out/stderr")" || status=$?
    stop_sleet
    printf '%s\n' "$output" | sed "s/^/  [$name] /"
    if ((status != 0)); then
      cat "$out/stderr" >&2
      echo "UCI $release $name: exit $status" >&2
      exit 1
    fi
    for marker in "GOLDEN: PASSED" "typed MSG Data=absent: PASSED" "typed MSG Data=empty: PASSED" \
                  "typed MSG Data=0080FF: PASSED" "SLEET AND RECOVERY: PASSED"; do
      grep -qxF "CPP-OMS-003 $schema_version $marker" <<<"$output" \
        || { echo "UCI $release $name: missing marker '$marker'" >&2; exit 1; }
    done
    grep -qE "^CPP_OMS_003_REAL_LOG_COMPLETE uci=$version tests=2 checks=[0-9]+$" <<<"$output" \
      || { echo "UCI $release $name: no completion line" >&2; exit 1; }
    passed+=("UCI $release $name ($(grep -oE 'checks=[0-9]+' <<<"$output" | tail -1))")
  done
done
printf 'passed: %s\n' "${passed[@]}"
echo "CPP-OMS-003 real Log through pinned Sleet $SLEET_REV: PASS (UCI 2.5 and 2.6)"
