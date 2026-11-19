#!/usr/bin/env bash
# CPP-OMS-008: qualification of the standalone C++ UCI Log receiver
# (examples/cpp-uci-client) against the UNMODIFIED pinned Sleet, separately for
# authentic UCI 2.5 and UCI 2.6.
#
#   1. pinned UCI roots (fetch scripts or AMS_GRA_UCI_2_{5,6}_ROOT) and the
#      pinned, clean Sleet checkout (e38f61d8, tree 7e705f4e), built --locked;
#   2. offline: tests/unit_test.cpp (strict g++, and clang++ when installed);
#   3. per release: examples/cpp-uci-client/build.sh (readiness, generation into
#      the scratch directory, strict C++17 build); a fresh Sleet loaded with
#      that release's full schema and a service authorized for Log on uci.log;
#      then the APPLICATION BINARY is run with an independent wire observer and
#      publisher (tests/wire_peer.cpp on tests/cpp-lacal/raw_owp_client.hpp):
#      typed reception, duration bound, SIGINT/SIGTERM, wrong release, connection
#      refusal, unauthorized SUB, WSS refusal, and the scripted negative peer
#      (no/wrong owp, never-upgrade timeout, SUB timeout, transport loss);
#      then tests/receiver_test.cpp (receiver core: order, fields, decoder vs
#      broker rejection, wrong global element, queue overflow, pending
#      shutdown, callback failure, unauthorized SUB) against the same Sleet.
#
# Usage: scripts/check-cpp-oms-008-log-client.sh [scratch-dir] [existing-sleet-checkout]
# Environment: AMS_GRA_CLI, AMS_GRA_UCI_2_5_ROOT, AMS_GRA_UCI_2_6_ROOT,
# CPP_UCI_CLIENT_RELEASES (default "2.5 2.6").
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"

SLEET_REPO=https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet
SLEET_REV=e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b
SLEET_TREE=7e705f4eb5f658957bd414b4abc8207311f3c1e4
grep -q "$SLEET_REV" scripts/run-real-sleet-test.sh \
  || { echo "Sleet pin differs from scripts/run-real-sleet-test.sh" >&2; exit 1; }
releases="${CPP_UCI_CLIENT_RELEASES:-2.5 2.6}"

# ---- 1. pinned inputs ------------------------------------------------------------
root25="${AMS_GRA_UCI_2_5_ROOT:-}"
root26="${AMS_GRA_UCI_2_6_ROOT:-}"
[[ -n "$root25" ]] || root25="$(scripts/fetch-pinned-uci-2.5.sh "$scratch/uci25-checkout")"
[[ -n "$root26" ]] || root26="$(scripts/fetch-pinned-uci-2.6.sh "$scratch/uci26-checkout")"
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
echo "Sleet $SLEET_REV (tree $SLEET_TREE) clean; $(cd "$checkout" && rustc --version); binary sha256 $(sha256sum "$sleet_bin" | cut -d' ' -f1)"

cli="${AMS_GRA_CLI:-}"
if [[ -z "$cli" ]]; then
  cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
  cli="$repo/target/release/ams-gra-codegen-oms"
fi
export AMS_GRA_CLI="$cli"

strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -Wshadow -Wold-style-cast
        -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual -Wnull-dereference -Wdouble-promotion -Wformat=2)
client_dir="$repo/examples/cpp-uci-client"
runtime=("$repo/cpp/oms-lacal-runtime/src/owp.cpp" "$repo/cpp/oms-lacal-runtime/src/runtime.cpp")
include=(-I "$repo/cpp/oms-lacal-runtime/include" -I "$client_dir/src" -I "$client_dir/tests" -I "$repo/tests/cpp-lacal")

# ---- 2. offline unit test ---------------------------------------------------------------
compilers=(g++)
command -v clang++ >/dev/null 2>&1 && compilers+=(clang++) || echo "SKIPPED (not claimed): clang++ not installed"
for cxx in "${compilers[@]}"; do
  "$cxx" "${strict[@]}" -O1 -pthread "${include[@]}" "$client_dir/tests/unit_test.cpp" "${runtime[@]}" -o "$scratch/unit-$cxx"
  out="$("$scratch/unit-$cxx")"
  echo "  [$cxx] $out"
  grep -qE '^CPP_OMS_008_CLIENT_UNIT_COMPLETE checks=[0-9]+$' <<<"$out" || { echo "unit test ($cxx) failed" >&2; exit 1; }
done

# ---- Sleet lifecycle ----------------------------------------------------------------------
free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'; }
sleet_pid="" bg_pids=()
stop_sleet() {
  if [[ -n "$sleet_pid" ]]; then kill "$sleet_pid" 2>/dev/null || true; wait "$sleet_pid" 2>/dev/null || true; sleet_pid=""; fi
}
cleanup() {
  for pid in "${bg_pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  stop_sleet
}
trap cleanup EXIT

start_sleet() { # schema-root schema-version label
  local dir="$scratch/sleet-run-$3" port i
  rm -rf "$dir"; mkdir -p "$dir/services"
  # The application's service: authorized for exactly topic uci.log, bound to Log.
  cat > "$dir/services/cpp-uci-client.toml" <<'EOF'
service_id = "cpp-uci-client"
service_uuid = "550e8400-e29b-41d4-a716-446655440108"
allowed_topics = ["uci.log"]

[[topic_bindings]]
topic = "uci.log"
allowed_messages = ["Log"]
EOF
  # Independent observer/publisher: a separate service identity.
  cat > "$dir/services/cpp-uci-wire.toml" <<'EOF'
service_id = "cpp-uci-wire"
service_uuid = "550e8400-e29b-41d4-a716-446655440109"
allowed_topics = ["uci.log"]
EOF
  # Known to Sleet but NOT authorized for uci.log.
  cat > "$dir/services/cpp-uci-unauthorized.toml" <<'EOF'
service_id = "cpp-uci-unauthorized"
service_uuid = "550e8400-e29b-41d4-a716-446655440110"
allowed_topics = ["other.topic"]
EOF
  port="$(free_port)"
  cat > "$dir/server.toml" <<EOF
bind_addr = "127.0.0.1:$port"
server_id = "cpp-oms-008"
system_label = "CPP-OMS-008 standalone C++ UCI Log receiver"
schema_path = "$1"
schema_version = "$2"
services_dir = "$dir/services"
EOF
  RUST_LOG=sleet=debug NO_COLOR=1 "$sleet_bin" "$dir/server.toml" >"$dir/sleet.log" 2>&1 &
  sleet_pid=$!
  for ((i = 0; i < 600; i++)); do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      sleet_url="ws://127.0.0.1:$port"; sleet_port="$port"; sleet_log="$dir/sleet.log"
      return 0
    fi
    kill -0 "$sleet_pid" 2>/dev/null || { cat "$dir/sleet.log" >&2; echo "Sleet exited" >&2; exit 1; }
    sleep 0.1
  done
  echo "Sleet did not listen within 60 s" >&2; exit 1
}

fail() { echo "FAIL: $*" >&2; exit 1; }
expect_line() { grep -qE "$2" "$1" || { sed 's/^/    | /' "$1" >&2; fail "$3: missing /$2/ in $1"; }; }
refuse_line() { if grep -qE "$2" "$1"; then sed 's/^/    | /' "$1" >&2; fail "$3: unexpected /$2/ in $1"; fi; }
expect_exit() { [[ "$1" == "$2" ]] || fail "$3: exit $1, expected $2"; }

# Runs the application in the background; sets app_pid, app_out.
start_app() { # label args...
  app_out="$cell/$1.out"; shift
  "$app" "$@" >"$app_out" 2>"$app_out.err" &
  app_pid=$!
}
wait_for() { # file regex seconds label
  local i
  for ((i = 0; i < $3 * 20; i++)); do grep -qE "$2" "$1" 2>/dev/null && return 0; sleep 0.05; done
  sed 's/^/    | /' "$1" >&2 || true
  fail "$4: /$2/ did not appear in $1 within $3 s"
}
# Must run in THIS shell (not $(...)): only the parent can wait for its job.
wait_app() { status=0; wait "$app_pid" || status=$?; }

# Sleet's own debug log lines for the APPLICATION's connection: Sleet records
# the service identity in the connection span as service_id="cpp-uci-client".
app_sleet_lines() { grep -F 'service_id="cpp-uci-client"}' "$sleet_log" | grep -cF "$1" || true; }

passed=()
for release in $releases; do
  v="${release/./}"
  if [[ "$release" == 2.5 ]]; then root="$root25"; other=26; else root="$root26"; other=25; fi
  schema="002.${release#2.}.0"
  echo "== UCI $release ($schema)"
  build="$scratch/build"
  examples/cpp-uci-client/build.sh --release "$release" --schema "$root" --out "$build" | sed 's/^/  /'
  app="$build/bin/cpp-uci-client-uci$v"
  cell="$scratch/cell-$v"
  rm -rf "$cell"; mkdir -p "$cell"
  test_flags=("${strict[@]}" -O1 -pthread "${include[@]}" -I "$build/generated/uci$v")
  g++ "${test_flags[@]}" "$client_dir/tests/wire_peer.cpp" -o "$cell/wire_peer"
  g++ "${test_flags[@]}" "$client_dir/tests/negative_peer.cpp" -o "$cell/negative_peer"
  g++ "${test_flags[@]}" "-DCPP_UCI_CLIENT_UCI_RELEASE=$v" \
    "-DCPP_UCI_CLIENT_SCHEMA_SHA256=\"$(sha256sum "$root" | cut -d' ' -f1)\"" \
    "$client_dir/tests/receiver_test.cpp" "${runtime[@]}" -o "$cell/receiver_test"
  "$cell/wire_peer" check-golden "$schema" "tests/fixtures/service-generate/task093-real-log$v.json" | sed 's/^/  /'

  # --help documents the build-time release; nothing connects.
  "$app" --help >"$cell/help.out"
  expect_line "$cell/help.out" "BUILT FOR UCI $release \\(OWP INIT schema $schema\\)" "help"

  start_sleet "$root" "$schema" "$v"
  wire=("$cell/wire_peer")

  # ---- A. typed reception through the application binary -------------------------------
  "${wire[@]}" observe "$sleet_url" cpp-uci-wire "$schema" 25 "$cell/observed.jsonl" >"$cell/observer.out" 2>&1 &
  observer_pid=$!; bg_pids+=("$observer_pid")
  wait_for "$cell/observer.out" '^OBSERVER READY$' 15 "observer"
  start_app typed --url "$sleet_url" --service-id cpp-uci-client --max-messages 5 --duration-seconds 30
  wait_for "$app_out" '^READY sid=sub-1 ' 15 "typed READY"
  "${wire[@]}" publish "$sleet_url" cpp-uci-wire "$schema" v1,v2,decoder-bad,v3,broker-bad,logmt,v4,v5 \
    >"$cell/publisher.out" 2>&1
  wait_app
  expect_exit "$status" 0 "typed"
  for n in 1 2 3 4 5; do
    sev=INFO; [[ $((n % 2)) == 1 ]] && sev=WARNING
    expect_line "$app_out" "^RECEIVED Log #$n severity=$sev classification=U schema=$schema header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:0${n}Z system=12345678-1234-1234-8234-123456789abc" "typed #$n"
  done
  expect_line "$app_out" '^RECEIVED Log #1 .* service=550e8400-e29b-41d4-a716-446655440108 subsystem=550e8400-e29b-41d4-a716-446655440208$' "typed identity"
  expect_line "$app_out" '^DECODE-ERROR Log rejected by the client: codec path=LogMT\.MessageData\.Data ' "decoder rejection"
  expect_line "$app_out" '^STOP max-messages=5 reached$' "bound"
  expect_line "$app_out" '^UNSUBSCRIBED sid=sub-1 acknowledged by the broker \(\+OK\)$' "UNSUB"
  expect_line "$app_out" '^CLOSED close=ok callbacks=5 callbacks-after-unsubscribe=0 .* delivered=5 decode-errors=1 .* runtime-threads=0 .* resources-released=yes$' "close"
  refuse_line "$app_out" 'Task093|Independent real Log|0080FF|NATO|"000"' "no raw Log text or rejected content"
  expect_line "$cell/publisher.out" '^PUB decoder-bad -> \+OK$' "decoder-bad admitted by Sleet"
  expect_line "$cell/publisher.out" '^PUB broker-bad -> -ERR Invalid-Message .*Severity' "broker rejection"
  expect_line "$cell/publisher.out" '^PUB logmt -> -ERR Invalid-Message .*LogMT' "LogMT global element rejected"
  kill "$observer_pid" 2>/dev/null || true; wait "$observer_pid" 2>/dev/null || true
  # The independent observer saw exactly the admitted documents, byte for byte, in order.
  "${wire[@]}" render "$schema" v1,v2,decoder-bad,v3,v4,v5 >"$cell/expected.jsonl"
  cmp -s "$cell/expected.jsonl" "$cell/observed.jsonl" \
    || { diff "$cell/expected.jsonl" "$cell/observed.jsonl" | head -5 >&2; fail "observed MSG text differs from the published documents"; }
  # Sleet's own record of the application's connection (span service_id=cpp-uci-client).
  [[ "$(app_sleet_lines "initialized schema=$schema service_id=cpp-uci-client")" == 1 ]] || fail "Sleet: INIT/INFO"
  [[ "$(app_sleet_lines 'subscription registered sid=sub-1 topic=uci.log message_name=Log')" == 1 ]] || fail "Sleet: SUB"
  [[ "$(app_sleet_lines 'subscription removed sid=sub-1')" == 1 ]] || fail "Sleet: UNSUB"
  for ((i = 0; i < 100; i++)); do [[ "$(app_sleet_lines 'disconnected')" == 1 ]] && break; sleep 0.05; done
  [[ "$(app_sleet_lines 'disconnected')" == 1 ]] || fail "Sleet: disconnect"
  echo "  typed: 5 RECEIVED, 1 DECODE-ERROR, 2 broker -ERR, observer MSG=6 byte-exact, Sleet INIT/SUB/UNSUB/disconnect, close clean"

  # ---- B. duration bound with no traffic ------------------------------------------------
  start_app duration --url "$sleet_url" --service-id cpp-uci-client --duration-seconds 1 --max-messages 3
  wait_app
  expect_exit "$status" 0 "duration"
  expect_line "$app_out" '^STOP duration-seconds=1 elapsed after 0 Log message\(s\)$' "duration"
  expect_line "$app_out" '^UNSUBSCRIBED sid=sub-1 ' "duration UNSUB"
  expect_line "$app_out" ' resources-released=yes$' "duration close"

  # ---- C. SIGINT / SIGTERM during reception (clean UNSUB + close) ---------------------------
  for sig in INT TERM; do
    start_app "sig$sig" --url "$sleet_url" --service-id cpp-uci-client --duration-seconds 60 --max-messages 1000
    wait_for "$app_out" '^READY ' 15 "SIG$sig READY"
    "${wire[@]}" publish "$sleet_url" cpp-uci-wire "$schema" v1,v2 >/dev/null
    wait_for "$app_out" '^RECEIVED Log #2 ' 15 "SIG$sig reception"
    kill "-$sig" "$app_pid"
    wait_app
    code=130; [[ "$sig" == TERM ]] && code=143
    expect_exit "$status" "$code" "SIG$sig"
    expect_line "$app_out" "^STOP signal=$((code - 128)) after 2 Log message\\(s\\)$" "SIG$sig stop"
    expect_line "$app_out" '^UNSUBSCRIBED sid=sub-1 acknowledged by the broker' "SIG$sig UNSUB"
    expect_line "$app_out" ' callbacks-after-unsubscribe=0 .* runtime-threads=0 .* resources-released=yes$' "SIG$sig close"
  done

  # ---- D. negative cases against the real broker -------------------------------------------
  # Wrong schema selection: the OTHER release's binary advertises its own build-time schema.
  wrong_app="$build/bin/cpp-uci-client-uci$other"
  if [[ ! -x "$wrong_app" ]]; then
    other_release="${other:0:1}.${other:1:1}"
    if [[ "$other" == 25 ]]; then other_root="$root25"; else other_root="$root26"; fi
    examples/cpp-uci-client/build.sh --release "$other_release" --schema "$other_root" --out "$build" >"$cell/build-other.log"
  fi
  status=0; "$wrong_app" --url "$sleet_url" --service-id cpp-uci-client --duration-seconds 2 \
    >"$cell/wrong-release.out" 2>&1 || status=$?
  expect_exit "$status" 3 "wrong release"
  expect_line "$cell/wrong-release.out" '^CONNECT-FAILED kind=broker_rejected owp=Unsupported-Schema ' "wrong release"
  refuse_line "$cell/wrong-release.out" '^READY' "wrong release"
  # Unauthorized Log subscription: a known service without the uci.log topic.
  status=0; "$app" --url "$sleet_url" --service-id cpp-uci-unauthorized --duration-seconds 2 \
    >"$cell/unauthorized.out" 2>&1 || status=$?
  expect_exit "$status" 4 "unauthorized"
  expect_line "$cell/unauthorized.out" \
    '^SUBSCRIBE-REJECTED by the broker: kind=broker_rejected owp=Illegal-State .*not authorized to subscribe to topic uci.log' \
    "unauthorized"
  refuse_line "$cell/unauthorized.out" '^READY' "unauthorized"
  # Unknown service: INIT rejected.
  status=0; "$app" --url "$sleet_url" --service-id not-configured --duration-seconds 2 >"$cell/unknown.out" 2>&1 || status=$?
  expect_exit "$status" 3 "unknown service"
  expect_line "$cell/unknown.out" '^CONNECT-FAILED kind=broker_rejected owp=Unsupported-Service ' "unknown service"
  # Native WSS and invalid arguments: refused before any connection (Sleet sees none).
  before="$(grep -c 'accepted' "$sleet_log" || true)"
  status=0; "$app" --url "wss://127.0.0.1:$sleet_port" --service-id cpp-uci-client >"$cell/wss.out" 2>&1 || status=$?
  expect_exit "$status" 2 "wss"
  expect_line "$cell/wss.out" 'native wss:// \(TLS\) is not supported' "wss"
  for args in "--url ws://127.0.0.1:$sleet_port" "--service-id cpp-uci-client" \
              "--url ws://127.0.0.1:$sleet_port --service-id cpp-uci-client --max-messages 0" \
              "--url ws://127.0.0.1:$sleet_port --service-id cpp-uci-client --duration-seconds 3601" \
              "--url http://127.0.0.1:$sleet_port --service-id cpp-uci-client" \
              "--url ws://127.0.0.1:$sleet_port --service-id cpp-uci-client --schema 002.5.0"; do
    status=0
    # shellcheck disable=SC2086
    "$app" $args >"$cell/cli.out" 2>&1 || status=$?
    expect_exit "$status" 2 "cli '$args'"
    expect_line "$cell/cli.out" '^ERROR ' "cli '$args'"
  done
  sleep 0.3
  [[ "$(grep -c 'accepted' "$sleet_log" || true)" == "$before" ]] || fail "wss/invalid CLI: a connection reached Sleet"
  echo "  negative (real Sleet): wrong release, unauthorized SUB, unknown service, wss, invalid CLI"

  # ---- E. receiver core against the same Sleet ------------------------------------------------
  out="$(CPP_UCI_CLIENT_SLEET_URL="$sleet_url" timeout 300 "$cell/receiver_test" 2>"$cell/receiver.err")" \
    || { cat "$cell/receiver.err" >&2; fail "receiver_test"; }
  printf '%s\n' "$out" | sed 's/^/  [receiver] /'
  grep -qE "^CPP_OMS_008_RECEIVER_COMPLETE schema=$schema tests=7 checks=[0-9]+$" <<<"$out" || fail "receiver_test summary"
  stop_sleet

  # ---- F. connection refusal (Sleet stopped) and scripted transport failures ------------------------
  status=0; "$app" --url "$sleet_url" --service-id cpp-uci-client >"$cell/refused.out" 2>&1 || status=$?
  expect_exit "$status" 3 "refused"
  expect_line "$cell/refused.out" '^CONNECT-FAILED kind=connect_failed .*Connection refused' "refused"
  negative() { # mode expected-exit regex
    "$cell/negative_peer" "$1" 12 >"$cell/peer-$1.out" 2>&1 &
    local peer=$! port="" i s=0 started
    bg_pids+=("$peer")
    for ((i = 0; i < 100; i++)); do
      port="$(sed -n 's/^PORT //p' "$cell/peer-$1.out")"; [[ -n "$port" ]] && break; sleep 0.05
    done
    [[ -n "$port" ]] || fail "negative peer $1 did not start"
    started=$SECONDS
    "$app" --url "ws://127.0.0.1:$port" --service-id cpp-uci-client --duration-seconds 30 >"$cell/neg-$1.out" 2>&1 || s=$?
    expect_exit "$s" "$2" "$1"
    expect_line "$cell/neg-$1.out" "$3" "$1"
    expect_line "$cell/neg-$1.out" ' runtime-threads=0 .*resources-released=yes$' "$1 cleanup"
    (( SECONDS - started <= 15 )) || fail "$1: not bounded ($((SECONDS - started)) s)"
    kill "$peer" 2>/dev/null || true; wait "$peer" 2>/dev/null || true
  }
  negative no-owp 3 "^CONNECT-FAILED kind=handshake_rejected .*did not select subprotocol 'owp' \\(got ''\\)"
  negative wrong-owp 3 "^CONNECT-FAILED kind=handshake_rejected .*\\(got 'not-owp'\\)"
  negative reject-400 3 '^CONNECT-FAILED kind=handshake_rejected .*HTTP 400'
  negative never-upgrade 3 '^CONNECT-FAILED kind=timeout '
  negative silent-sub 5 '^SUBSCRIBE-FAILED kind=timeout '
  negative drop-after-sub 5 '^CONNECTION-LOST kind=connection_closed '
  echo "  negative (transport): refused, no owp, wrong owp, HTTP 400, connect timeout, SUB timeout, transport loss"
  stop_sleet
  passed+=("UCI $release")
done
printf 'passed: %s\n' "${passed[@]}"
echo "CPP-OMS-008 standalone C++ UCI Log receiver through pinned Sleet $SLEET_REV: PASS ($releases)"
