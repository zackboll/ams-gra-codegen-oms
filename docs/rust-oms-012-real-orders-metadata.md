# RUST-OMS-012 — authentic OrdersMetadata Rust interoperability

## Scope and immutable inputs

This qualification uses the production Rust generator, checked models, generated
JSON codec and service endpoints, and existing LA-CAL `SleetRuntime`. It does not
add OrdersMetadata to the standalone client. No production generator, schema IR,
runtime, schema, Sleet, other-language implementation or workflow is modified.
Historical Task063 and RUST-OMS-010 evidence is unchanged.

The isolated branch starts at fetched `origin/main`
`aadf62203761bdafb0d89f8b3c714b97bdd7c5f0`, the merge of PR #196. Its opt-in
five-message standalone client is present and remains unchanged. The worktree is
`/home/zboll/git/ams-gra-codegen-oms-orders-metadata`; local build products and
qualification logs are outside the repository at
`/home/zboll/rust-oms-012-evidence/`.

Both authentic schemas are independently selected and verified using the
repository's established pins, not reconstructed subsets:

| Release | Public standard revision | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 |
| --- | --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` (`v2.5`) | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` (`v2.6`) | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

The original qualification used Sleet `e38f61d8`. PR #208's downstream
requalification uses the clean, unmodified candidate revision
`94796daf55ebd1134dce0925e6a204a05ad3423b`, tree
`47da595b9078cac1a9351e8290d1ae7f3976367f`. The existing runner checks these
identities and clean status, builds with Sleet's own pinned toolchain/lockfile,
and reuses that single checkout/build. Qualification Rust uses **1.95.0** and
locked dependencies.

## Production generation and measured readiness

The separate contracts under `/tests/fixtures/rust-oms-012/` select the global
`OrdersMetadata` in `https://www.vdl.afrl.af.mil/programs/oam`, input `f/e` and
output `f/publish`, both on `orders.metadata`, without a subscription group.
The production `service-check` and `service-generate` commands run separately
for each release with `--language rust --world closed-schema --with-codec`.

| Release | Selected/renderable messages | Selected/renderable model types | Renderable codec declarations | Model / codec |
| --- | --- | --- | --- | --- |
| 2.5 | 1 / 1 | 82 / 82 | 80 / 80 | READY / READY |
| 2.6 | 1 / 1 | 82 / 82 | 80 / 80 | READY / READY |

These counts were measured from the production reports, not copied from
Task063, inferred from another backend or hardcoded into readiness checks.
Build-time gates require model and codec READY and retain `readiness.txt` and
`generation.txt` under each release's generated output directory. A non-READY
report fails before generation. No generator capability was added.

Each independently generated module contains `model::OrdersMetadataMT`,
`service_codec::ServiceCodec`, `service_api::function_f::exchange_e::subscribe`
and `service_api::function_f::exchange_publish::publish`. The tests verify the
global message and topic descriptors and distinct release-specific Rust type
identities. Generation without codecs also runs, and model/API source parity
is checked after removing only the expected codec sibling-module declaration.

## Independent JSON and model oracles

The normal and second JSON documents are authored independently of generated
encoding. The former carries qualifier `1`; the latter carries `999`, a distinct
metadata UUID and timestamp, description, source-system ID, object state,
associated message and time span. Tests compare complete generated encoding
against those expected JSON values. Separate checked-constructor tests rebuild
the nested sequence with `USMTFSerialNumberOfQualifierType::new` and generated
`UnboundedVec::new`; no direct Serde model deserialization or handwritten
constraint implementation is used.

The numeric path is:

```text
MessageData.OrdersAssociation[0].Orders.ATO[0].Identifier.QualifierSerialNumber
```

Each release has 32 independent negative payload cases, all rejected at the
selected broker before routing. The direct codec test independently rejects
all 32, including the unchanged malformed timestamp `not-a-dateZ`.
The live test additionally exercises incorrect INIT version, global identity
and unauthorized topic as separate controls.

Both releases accept integers `1` and `999`. Both reject `0`, `1000`, `-1`,
`1.0`, `1.5`, string `"1"`, boolean `true`, `null`, object and array values at
that member. The per-release negative JSON matrices independently specify the
mutation path, value or omission, expected diagnostic family and rejection
boundary. Required top-level security/header/data, metadata ID, association,
orders object and header fields are omitted individually. Additional cases
cover empty required associations, scalar/array/object mismatches along the
nested path, invalid schema-version lexical form, invalid mode and invalid
calendar timestamp.

Optional qualifier omission is accepted, independently of explicit `null`.
Omitting the optional Identifier or ATO list is also accepted. Normal documents
omit optional object state, source system, associated message and time span;
second documents independently exercise their presence.

Release distinctions are intentional:

* UCI 2.5's `USMTF_SerialNumberOfQualifierType` has bounds `1..999` **and**
  pattern `[0-9]{1,3}`; UCI 2.6 retains the bounds without that pattern.
  Their generated checked numeric carriers admit the same JSON integer values.
* The UCI 2.6 second document uses `MessageType: "SYSTEM_SCHEDULE"`, which is
  absent from UCI 2.5. A cross-release negative proves 2.5 rejects that value.
  The Rust variant spelling `SYSTEMSCHEDULE` is not the JSON wire spelling.
* `SchemaVersion` is the authentic checked lexical type, not a release-specific
  singleton. Tests preserve the expected header version in each fixture and
  reject malformed versions, but explicitly establish that a well-formed peer
  release version is not itself a model constraint. Session version negotiation
  is tested separately with a rejected broker INIT connection.

Global identity is not inferred from payload shape: the production envelope
helper rejects `OrdersMetadataMT`, missing/multiple members and a non-object
root. The actual broker also rejects the wrong global element and unauthorized
topic independently of payload validity.

## Live broker, decoder and lifecycle evidence

Each release starts the existing real-Sleet support harness with its complete
verified schema. One production `SleetRuntime<ServiceCodec>` subscribes through
the generated input endpoint and publishes through its generated output
endpoint. An independent `CalClient` observer must receive and compare actual
OWP `MSG` payloads; a successful `PUB` write is never treated as delivery or
acceptance acknowledgement. Another independent `CalClient` publishes the
distinguishable second document, which must reach a typed generated
`OrdersMetadataMT` callback and re-encode to the independently authored JSON.

Live invalid cases require broker rejection (`ClientError::Server`, no observer
MSG, typed callback or subscription decoder event). They do not accept an
alternate decoder boundary. The separate direct `ServiceCodec.decode_payload`
test retains generated-decoder rejection coverage without requiring a validating
broker to forward malformed input. Valid typed publication, independent observer
JSON comparison and typed decoding are required after every payload negative to
prove the original runtime and subscription remain usable.

Observed qualifier diagnostics preserve release differences: UCI 2.5 rejects
`1000` and `-1` at its `pattern` facet and `0` at `minInclusive`; UCI 2.6 uses
the numeric inclusive-bound facets. Floating-point/string/boolean/null/object
representations produce `TypeMismatch`; an array at the scalar member produces
`OccurrenceMismatch`. Although `not-a-dateZ` matches the schema's `.+Z` suffix
pattern, it is not a valid XSD dateTime. The selected candidate rejects it with
OWP `Invalid-Message` (`ClientError::Server` / `InvalidMessage`) and the exact
detail observed for both releases:

```text
schema validation failed: FacetViolation { path: "OrdersMetadata.MessageHeader.Timestamp", facet: "lexicalRepresentation", detail: "value 'not-a-dateZ' is not a valid xs:dateTime" }
```

The live assertion checks this complete detail and error code, not an arbitrary
`FacetViolation`. No MSG or decoder event is expected. The direct codec test
separately requires a Timestamp diagnostic. Neither schemas nor invalid values
are weakened; cleanup and per-case original-subscription recovery are unchanged.

A second generated subscription witnesses valid traffic after removing the
original subscription. Only the remaining handler may run. Both subscriptions
are explicitly removed. The established worker thread-local teardown barrier
proves `close` remains blocked until that exact live worker is released, then
the closing thread is joined. Independent clients and executor are dropped.
On Linux, live established TCP connections are witnessed and must reach zero
**before** stopping Sleet; TIME_WAIT is not counted as a live leak. The broker
is stopped/reaped, temporary configuration removed and its listener rebound.

## External consumer and fail-closed runner

The small consumer source is
`/tests/fixtures/rust-oms-012/consumer.rs`. Its integration-test driver creates
an independent Cargo package outside the repository, includes actual generated
source modules, and links only production runtime/API crates and `serde_json`.
It type-checks generated subscribe/publish against production `SleetRuntime`,
inspects the public checked numeric member, compares complete encoding to the
authored second documents and runs one test per release. All-target checks and
Clippy `-D warnings` run in the independent package too.

`consumer.Cargo.lock` is the Cargo-pruned production lock subset, not a fresh
dependency resolution. A regression checks every dependency's name, version,
source and checksum against the repository lock. Consumer commands use
`--locked --offline`; the lock bytes must remain unchanged.

`AMS_GRA_RUST_ORDERS_METADATA_ONLY=1` selects the focused mode in the existing
real-Sleet runner. It requires both authentic roots before any broker build,
reuses the single pinned checkout/build and runs exactly five qualification
tests plus one external-consumer driver (containing its two release tests).
It requires both-release model/codec, typed/wire interoperability, cleanup and
consumer pass markers; missing markers, skips, ignored tests, wrong counts,
zero tests and failed commands cannot pass. Adversarial wrapper tests exercise
these failure modes against the actual runner. Ordinary main-only real-Sleet
execution also runs this qualification when both roots are configured. No new
CI lanes or workflow changes are introduced; full Deep CI stays main-only.

## Reproduction

Use absolute paths for the repository, clean Sleet checkout and external build
directory. Fetch scripts print the authentic verified roots:

```bash
REPO=/absolute/path/to/repository
EVIDENCE=/absolute/path/outside/repository/rust-oms-012
mkdir -p "$EVIDENCE"
export AMS_GRA_UCI_2_5_ROOT="$(bash "$REPO/scripts/fetch-pinned-uci-2.5.sh" "$EVIDENCE/uci25")"
export AMS_GRA_UCI_2_6_ROOT="$(bash "$REPO/scripts/fetch-pinned-uci-2.6.sh" "$EVIDENCE/uci26")"
export CARGO_TARGET_DIR="$EVIDENCE/target"
export CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3
cd "$REPO"
AMS_GRA_RUST_ORDERS_METADATA_ONLY=1 bash "$REPO/scripts/run-real-sleet-test.sh" /absolute/path/to/clean-sleet-checkout
python3 -B -m unittest discover -s "$REPO/scripts" -p test_rust_orders_metadata_runner.py -v
```

The actual production command pair, independently for `MINOR=5` and `MINOR=6`:

```bash
MINOR=5
ROOT="$AMS_GRA_UCI_2_5_ROOT" # use AMS_GRA_UCI_2_6_ROOT with MINOR=6
cargo +1.95.0 run --release --locked -p ams-gra-codegen-oms -- service-check \
  --schema "$ROOT" --contract "$REPO/tests/fixtures/rust-oms-012/uci2$MINOR-service.yaml" \
  --language rust --world closed-schema --with-codec
cargo +1.95.0 run --release --locked -p ams-gra-codegen-oms -- service-generate \
  --schema "$ROOT" --contract "$REPO/tests/fixtures/rust-oms-012/uci2$MINOR-service.yaml" \
  --language rust --world closed-schema --with-codec --output "$EVIDENCE/generated2$MINOR"
```

Regression commands (the runner exports `AMS_GRA_SLEET_BIN` internally; export
the same pinned binary when invoking Cargo directly):

```bash
export AMS_GRA_SLEET_BIN=/absolute/path/to/clean-sleet-checkout/target/release/sleet
cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-api \
  -p ams-gra-oms-runtime-rust -p ams-gra-oms-runtime-rust-facade-tests \
  --all-targets -- --test-threads=1
cargo +1.95.0 fmt --all -- --check
rustfmt +1.95.0 --edition 2024 --check \
  "$REPO/tests/fixtures/rust-oms-012/consumer.rs" \
  "$REPO/crates/runtime-rust-facade-tests/tests/real_orders_metadata_cases/mod.rs"
cargo +1.95.0 check --release --locked --workspace --all-targets
cargo +1.95.0 clippy --release --locked --workspace --all-targets -- -D warnings
for MINOR in 5 6; do
  export AMS_GRA_UCI_RELEASE="2.$MINOR"
  if [ "$MINOR" = 5 ]; then export AMS_GRA_UCI_ROOT="$AMS_GRA_UCI_2_5_ROOT";
  else export AMS_GRA_UCI_ROOT="$AMS_GRA_UCI_2_6_ROOT"; fi
  MANIFEST="$REPO/examples/rust-uci-client/Cargo.toml"
  cargo +1.95.0 fmt --manifest-path "$MANIFEST" -- --check
  cargo +1.95.0 check --locked --all-targets --manifest-path "$MANIFEST"
  cargo +1.95.0 clippy --locked --all-targets --manifest-path "$MANIFEST" -- -D warnings
  cargo +1.95.0 test --locked --manifest-path "$MANIFEST" -- --include-ignored --nocapture --test-threads=1
done
python3 -B -m unittest discover -s "$REPO/scripts" -p test_pr_smoke.py -v
for BACKEND in rust python ada cpp; do
  RUSTUP_TOOLCHAIN=1.95.0 python3 "$REPO/scripts/pr_smoke.py" run "$BACKEND"
done
```

PR smoke expects its normal repository `target` path; local qualification uses
an ignored `target` symlink to an external build directory to keep artifacts
outside source. The baseline before edits passed the five existing authentic
FileMetadata tests and seven runtime lifecycle tests. Final local counts,
commit/tree SHAs, changed-file inventory and exact-head hosted statuses are
reported on the delivery PR rather than embedding self-referential hashes here.

## Limitations

This qualifies independent same-release WS sessions, not mixed-release sessions,
OrdersMetadata application integration, reconnect policy or new WSS behavior.
The existing four-/five-message client and its WSS tests remain regression
coverage only. Duplicate-key detection is not a payload-codec property because
`serde_json::Value` has already collapsed duplicate members. The known
historical FileMetadata SHA-256 incompatibility is corrected by the selected
candidate. Its remaining valid-input compatibility failures (XML Schema midnight,
hexBinary XML whitespace normalization and OMS's XSD 1.0 leap-second profile),
upstream acceptance and canonical pin are separate unresolved adoption blockers.
Non-Linux hosts do not provide the Linux established-socket evidence. Schema
validity is not a claim of operational order semantics beyond the authentic
checked XSD/OMS JSON model. No Sleet patch or schema relaxation is used.