# RUST-OMS-002 — authentic RDMA_InitializeSetup over pinned Sleet

## Scope and generation

Starting main: `ab438eab77d0b202a033bec2b2fa99508bbebc8f`, tree
`02c0df930e7af0c8e487994ac25e5a69d1104a33`, fetched and verified before
implementation. Branch: `feature/rust-oms-002-real-rdma`, dedicated worktree
`/home/zboll/git/ams-gra-codegen-oms-rust-oms-002`. Final commit/tree and hosted
run identity belong in the PR handoff, not a self-referential source file.

Production `service-check --language rust --world closed-schema --with-codec`
and `service-generate` use the unchanged Task103 contracts and complete authentic
schemas. Actual Rust results (not inferred from Ada's projection):

| Release | Global element / Rust payload | Selected/renderable types | Codec declarations | Readiness |
| --- | --- | --- | --- | --- |
| UCI 2.5 | `RDMA_InitializeSetup` / `RDMAInitializeSetupMT` | 50/50 | 49/49 | READY, no blockers |
| UCI 2.6 | `RDMA_InitializeSetup` / `RDMAInitializeSetupMT` | 50/50 | 49/49 | READY, no blockers |

The authentic XSD type is `RDMA_InitializeSetupMT`; the Rust backend's established
identifier normalization emits `RDMAInitializeSetupMT`. OWP uses the **global
element**, never either type spelling. No backend, common IR, runtime, codec or
transport correction was necessary. Nothing changes Ada, Python, C++ or Sleet
sources, and no schemas or generated model bytes are committed.

The facade build now shares root/include verification and production generation
between Log and RDMA. Each service/release is generated independently, both with
and without codecs. All existing generated Rust files must be byte-identical
except the expected `service_codec` module inclusion in `service_api.rs`.
Readiness reports remain beside the actual output in Cargo's `OUT_DIR`.

Separate `real_uci_rdma25` and `real_uci_rdma26` modules preserve release-specific
carriers. In particular `WhitespaceVisibleString1024Type` in 2.5 permits empty
text and collapse-normalizes whitespace; 2.6 requires at least one character and
preserves whitespace. Version-specific assertions exercise those differences;
neither module aliases the other. The generated Header `DateTimeType` constructors
both reject malformed timestamps.

## Authentic authority

The existing pinned fetch scripts are reused without modification:

| Input | Revision / SHA-256 |
| --- | --- |
| UCI 2.5 release | `093610b7753944059360d3236770ab446d039556` |
| 2.5 MessageDefinitions | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.5 SecurityMarkings include | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| UCI 2.6 release | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| 2.6 MessageDefinitions | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| 2.6 SecurityMarkings include | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| Sleet revision | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Rust | 1.95.0 |

Both authentic roots and includes are hash-checked before generation. Setting
`AMS_GRA_REQUIRE_REAL_UCI_RDMA` requires **both** roots. A configured nonexistent
or wrong input fails the build, never turns into a skip. Ordinary no-UCI builds
retain explicit diagnostic stubs; those are not real-Sleet qualification.

## Test design and explicit counts

`real_uci_rdma` contains **four tests**, two per release: direct model/codec cases
and actual broker messaging/rejection/recovery. Explicit qualification requires
4 passed, 0 failed, 0 ignored plus both releases' success markers.

Each release has six independently specified positive documents:

1. Unchanged Task092 golden (IPv4 `192.0.2.10`, IPv6 `2001:db8::1`).
2. IPv4-only `198.51.100.42`.
3. IPv6-only `2001:db8::42`.
4. Reversed IPv6/IPv4 ordering.
5. Four alternating endpoints plus USA/NATO/CAN/NATO security Choices.
6. Explicit `Endpoint: []`; canonical generated encoding omits this member.

Every document is published independently through pinned `CalClient`, then through
the generated output endpoint using typed endpoint/security construction. The
independent observer checks actual OWP `MSG` JSON, while the generated input
handler's model is inspected directly: both UUIDs, timestamp, schema version,
LIVE mode, U classification, TCP provider, endpoint cardinality, each IPv4/IPv6
Choice identity/address/order, and each OwnerProducer identity/order. This is not
just comparing an encoder with its own decoder. Direct decoding also accepts the
canonical omitted-endpoint representation.

Per release: **12 positive corpus deliveries**, **10 document-recovery deliveries**,
**1 recovery after unauthorized topic**, and **1 live typed witness delivery after
UNSUB** = **24 valid typed deliveries** (48 across both releases). Each valid wire
publication is also independently observed. The removed subscription must remain
silent while both a second generated typed input and raw wire observer stay live.

Required negative cases and actual pinned-Sleet boundaries:

| Case | Required boundary / observation |
| --- | --- |
| Invalid UUID variant nibble | Broker `FacetViolation`, UUID pattern |
| Schema version `002.5` | Broker `FacetViolation`, SchemaVersion pattern |
| IPv4 `999.0.0.1` | Broker `FacetViolation`, IPv4_Address pattern |
| IPv6 `not-an-ipv6` | Broker `FacetViolation`, IPv6_Address pattern |
| Both endpoint Choice arms | Broker `ChoiceViolation` |
| Unknown `NotAnEndpoint` Choice member | Broker `UnknownMember` |
| Numeric Endpoint instead of array | Broker `OccurrenceMismatch` |
| Missing RDMA_InitializeSetupID | Broker `MissingRequiredMember` |
| Global element `RDMA_InitializeSetupMT` | Broker `UnknownGlobalElement` |
| Unauthorized topic | OWP `Illegal-State`, not authorized |
| Timestamp `not-a-dateZ` | Broker accepts/routes; generated Rust decoder rejects |

That is **11 broker-path negative attempts per release**, 22 total: 20 broker/OWP
rejections and 2 broker-routed invalid timestamps. Every attempt requires zero
invalid typed callbacks. The timestamp must produce
`RuntimeEvent::SubscriptionDecodeError` / `MessageDecodeError::Codec` with its
Timestamp diagnostic; a valid message must then reach the **same subscription**.
All ten document cases have immediate same-subscription recovery assertions.
All nine payload failures are independently submitted to each generated decoder
(18 direct negatives), with contextual diagnostics. Checked model constructors
also reject UUID/version/IPv4/IPv6/timestamp invalid values. No stricter IPv6
network-stack parsing is imposed on the authentic schema's lexical language.

Broker startup/configuration, OWP INIT connections and raw publication helpers
are shared with existing Log tests. Transport remains `SleetRuntime` and pinned
`sleet-client`; no second WebSocket implementation. Runtime close, broker kill/
wait and temporary broker-directory cleanup are checked before success markers;
RAII still cleans up on failures.

`real_rdma_consumer` adds **one test** that creates an out-of-workspace Cargo
package. It includes both actual generated `service_api.rs` files and depends
only on public runtime crates and serde_json, not the facade test crate. Rust
1.95.0 compiles and executes both golden consumers, requiring both markers.

## Reproduce and CI

From the repository root:

```bash
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh /tmp/rdma-uci25)"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh /tmp/rdma-uci26)"
AMS_GRA_RUST_RDMA_ONLY=1 bash scripts/run-real-sleet-test.sh
```

An optional clean existing Sleet checkout may be passed as the script's first
argument. The script verifies/builds the pinned revision with Sleet's own lockfile.
Focused RDMA mode runs Log's four regression tests, the external consumer's one
test, and RDMA's four tests, all without ignored cases or silent skips. Test output
is captured per test (`--show-output`) so exact whole-line evidence markers cannot
be corrupted by concurrent test-status output.

The existing **main-only** `real-sleet` job requires RDMA and reuses its already
fetched roots and one pinned Sleet build. No new job, timeout change, runner move,
Deep PR matrix, CI-SELECT gate, CI-BENCH or PERF-009 change. Lightweight exact-head
PR smoke remains the hosted pre-merge check. Complete hosted Deep qualification
is intentionally main-only and cannot be claimed before independent review/merge.

Local evidence is outside the source at `/home/zboll/rust-oms-002-evidence`.
Execution results and exact final/hosted identities are recorded in the PR.
This qualifies **message-level RDMA setup interoperability**, not a physical RDMA
connection, arbitrary optional fields, TLS/authentication, open-world dispatch or
other broker revisions. The PR remains open, non-draft, unmerged, with auto-merge
disabled.