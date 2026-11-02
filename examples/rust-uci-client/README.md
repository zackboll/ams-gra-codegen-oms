# Standalone generated Rust LA-CAL client

A real Cargo application, not a test facade. One
`SleetRuntime<ServiceCodec>` connects to Sleet and hosts both generated Log and
RDMA input endpoints. Optional publications use their matching generated output
endpoints. Handlers receive generated `LogMT`/`RDMAInitializeSetupMT` values and
clone them into a channel; the application thread prints typed fields and owns
all blocking runtime operations.

This package is its own workspace, outside the root workspace's default tests.
Its path dependencies are **production** runtime API/runtime/generator crates.
Build from a complete checkout of this repository; it is not a crates.io package.
No UCI schemas or generated UCI model source are committed. `build.rs` calls the
production `service-generate --language rust --world closed-schema --with-codec`
on the existing `tests/fixtures/rust-oms-003/uci25-service.yaml` or
`uci26-service.yaml`, then compiles the actual generated source from `OUT_DIR`.
No test-only facade package is linked. The acceptance test reuses existing test
source helpers and independent Task092/093 goldens, not their generated facade.

## Prerequisites and authentic inputs

Run the following shell commands from the repository root. Requirements: Git,
Rust **1.95.0** (including rustfmt and Clippy), Python 3 (2.6 archive extraction),
and GNU coreutils (`sha256sum`, `realpath`). Network access to GitLab/GitHub and
Cargo's registry is needed on the first build. Use your organization's normal
Git credentials if access requires authentication; do not embed tokens in URLs.

```bash
rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy
export CLIENT_SCRATCH="$(mktemp -d)"  # outside the repository; keep for this session
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh "$CLIENT_SCRATCH/uci25")"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh "$CLIENT_SCRATCH/uci26")"
```

The scripts verify the public source identities in
`https://gitlab.com/open-arsenal/uci/standard.git`:

| Release | Source commit | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 |
| --- | --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

The build independently verifies **both** schema hashes. `AMS_GRA_UCI_ROOT`
names the **MessageDefinitions file**, not a directory. Its sibling
SecurityMarkings file must be present. The hash-pinned root includes only that
file. Missing variables/files, unsupported releases and hash mismatches fail
the build with a refetch/selection diagnostic. Cargo watches both environment
variables, the selected schema path, resolved root, SecurityMarkings and service
contract; switching releases does not require `cargo clean`.

## Select and build

```bash
export AMS_GRA_UCI_RELEASE=2.5
export AMS_GRA_UCI_ROOT="$AMS_GRA_UCI_2_5_ROOT"
export CLIENT_SUFFIX=25
export CLIENT_SCHEMA=002.5.0
# For 2.6 instead use:
# export AMS_GRA_UCI_RELEASE=2.6 AMS_GRA_UCI_ROOT="$AMS_GRA_UCI_2_6_ROOT"
# export CLIENT_SUFFIX=26 CLIENT_SCHEMA=002.6.0

cargo +1.95.0 build --locked --manifest-path examples/rust-uci-client/Cargo.toml
export CLIENT_BIN="$PWD/examples/rust-uci-client/target/debug/rust-uci-client"
"$CLIENT_BIN" --help
```

The binary's INIT schema is compiled as `002.5.0` or `002.6.0`, matching the
selected release. There is no CLI/environment override at runtime. A payload's
`MessageHeader.SchemaVersion` must also agree. To use a different Cargo target
directory, adjust `CLIENT_BIN` accordingly.

## Start unmodified pinned Sleet

```bash
git clone https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet "$CLIENT_SCRATCH/sleet"
git -C "$CLIENT_SCRATCH/sleet" checkout --detach e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b
test -z "$(git -C "$CLIENT_SCRATCH/sleet" status --porcelain)"
(cd "$CLIENT_SCRATCH/sleet" && cargo +1.95.0 build --release --locked -p sleet)

mkdir -p "$CLIENT_SCRATCH/services"
cat > "$CLIENT_SCRATCH/services/client.toml" <<'TOML'
service_id = "rust-uci-client"
service_uuid = "550e8400-e29b-41d4-a716-446655440005"
allowed_topics = ["rust-oms-003-log", "rust-oms-003-rdma"]
TOML
cat > "$CLIENT_SCRATCH/server.toml" <<TOML
bind_addr = "127.0.0.1:9000"
server_id = "rust-uci-example"
system_label = "Generated Rust UCI application"
schema_path = "$AMS_GRA_UCI_ROOT"
schema_version = "$CLIENT_SCHEMA"
services_dir = "$CLIENT_SCRATCH/services"
TOML
"$CLIENT_SCRATCH/sleet/target/release/sleet" "$CLIENT_SCRATCH/server.toml" \
  > "$CLIENT_SCRATCH/sleet.stdout" 2> "$CLIENT_SCRATCH/sleet.stderr" &
export SLEET_PID=$!
trap 'kill "$SLEET_PID" 2>/dev/null; wait "$SLEET_PID" 2>/dev/null' EXIT
# Wait for Sleet's "listening" startup line before running the client:
tail -f "$CLIENT_SCRATCH/sleet.stdout"
```

Stop **tail only** with Ctrl-C once Sleet reports listening. If it exits early,
inspect both log files. Choose an unused port if 9000 is occupied and update
`--url`. Pinned Sleet's `allowed_topics` grants both PUB and SUB; it is not
direction-specific authorization. Both message names are authored by the
generated contracts, not inferred from Rust type names. Restart Sleet with a
matching schema/config after switching releases.

## Run and publish

```bash
"$CLIENT_BIN" --url ws://127.0.0.1:9000 --service-id rust-uci-client \
  --duration-seconds 5 \
  --publish-log "tests/fixtures/service-generate/task093-real-log${CLIENT_SUFFIX}.json" \
  --publish-rdma "tests/fixtures/service-generate/task092-real-rdma${CLIENT_SUFFIX}.json"
```

Files contain OMS JSON **payload bodies**, without `{"Log": ...}` or
`{"RDMA_InitializeSetup": ...}` envelopes. The runtime owns envelopes and OWP.
The generated `ServiceCodec` validates values through checked model constructors;
generated models have no separate `Validate` trait. All selected files are
decoded and their typed header versions checked **before connecting**, so a bad
second file cannot cause a partial first publication. This is generated-model
validation, not a claim of complete XSD validation by the application.

Expected output (SIDs/order can differ):

```text
CONNECTED service=rust-uci-client UCI=2.5 INIT.schema=002.5.0
SUBSCRIBED Log sid=... RDMA_InitializeSetup sid=... (frames written)
PUBLISHED Log: frame written; broker acceptance not acknowledged
PUBLISHED RDMA_InitializeSetup: frame written; broker acceptance not acknowledged
RECEIVED Log system=12345678-1234-1234-8234-123456789abc severity=INFO label="Task093"
RECEIVED RDMA_InitializeSetup setup=12345678-1234-1234-8234-123456789abc provider=TCP endpoints=2
UNSUBSCRIBED Log: frame written; handler removed
UNSUBSCRIBED RDMA_InitializeSetup: frame written; handler removed
CLOSED runtime; worker joined
COMPLETE bounded execution; no runtime errors observed (not a broker acceptance guarantee)
```

Omit both publication options for a receive-only run. An external OWP client
authorized for these topics can publish the corresponding enveloped documents;
the same two typed handlers print them. `--duration-seconds` defaults to 5 and
accepts integers 1..=3600. Unknown/duplicate flags and missing values are errors.
No CLI framework, reconnect, replay, TLS or authentication policy is added.

Failed input example (nonzero exit; no connection or publication):

```bash
printf '{}\n' > "$CLIENT_SCRATCH/bad.json"
"$CLIENT_BIN" --url ws://127.0.0.1:9000 --service-id rust-uci-client \
  --publish-log "$CLIENT_SCRATCH/bad.json"
# ERROR: generated codec rejected .../bad.json: LogMT: missing required member "SecurityInformation"
```

Invalid JSON syntax is reported separately as `invalid input JSON`. Connection,
subscription, publication, unsubscribe and shutdown errors return nonzero.
`ServerError`, `SubscriptionDecodeError` (SID/message/cause, no typed delivery),
protocol errors and connection closure are printed as failures. The application
does not retry them. Both subscriptions are explicitly removed on the normal
path; errors after setup also attempt both removals and close. Early setup errors
drop the runtime, whose production Drop signals and joins its worker.

**Non-verbose OWP caveat:** successful operations mean frames were written, not
acknowledged by Sleet. Polling is bounded (finite batches and 20 ms waits), and
observed server errors are never treated as success or attributed to an operation
without evidence. A quiet final queue is not an acceptance barrier; events that
arrive after the bounded observation window can race with close. Independent MSG
observation is the delivery evidence, not the application's `PUBLISHED` line.

## Focused real-Sleet acceptance and local checks

On Linux, with both fetched roots exported as above:

```bash
# Verifies clean exact Sleet source, builds it once and exercises BOTH releases.
AMS_GRA_RUST_CLIENT_ONLY=1 scripts/run-real-sleet-test.sh "$CLIENT_SCRATCH/sleet"

# Individual selected-release commands (live test is explicitly opt-in):
cargo +1.95.0 check --locked --all-targets --manifest-path examples/rust-uci-client/Cargo.toml
cargo +1.95.0 clippy --locked --all-targets --manifest-path examples/rust-uci-client/Cargo.toml -- -D warnings
cargo +1.95.0 fmt --manifest-path examples/rust-uci-client/Cargo.toml -- --check
AMS_GRA_SLEET_BIN="$CLIENT_SCRATCH/sleet/target/release/sleet" \
  cargo +1.95.0 test --locked --manifest-path examples/rust-uci-client/Cargo.toml \
  -- --include-ignored --nocapture --test-threads=1
git diff --check
```

The test starts isolated servers, executes the actual application CLI, witnesses
both published OWP MSG documents with an independent pinned CalClient, externally
publishes distinct Log/RDMA values and checks typed output. It also covers broker
rejection, codec/decode rejection, server permission errors and connection closure,
then verifies bounded exit, explicit unsubscribe, joined worker output and reaped
application/server processes. Existing Linux socket/worker helpers and independent
goldens are reused. It is intentionally not the RUST-OMS-004 qualification corpus.
The existing main-only real-Sleet job invokes it with the same Sleet build; no new
runner lane or qualification framework is introduced. Root-workspace Rust
backend/runtime regressions and lightweight PR smoke remain unchanged.

When finished with the manual server:

```bash
kill "$SLEET_PID"
wait "$SLEET_PID" || true
trap - EXIT
```