# Authentic generated Rust FileMetadata over pinned Sleet

## Scope and inputs

This is metadata representation and message interoperability qualification, not
file transfer or digest verification. No file contents are read or hashed. It
uses production Rust models, `ServiceCodec`, generated input/output endpoints,
and `SleetRuntime`; no production changes or WSS dependencies are introduced.
The existing Task059 FileMetadata regression remains unchanged.

The dedicated worktree starts from main commit
`ebb25de9c15f7dd26ce2ce319bc800191d7d9cfc`, tree
`735ab7537274d5851598f464359cea1c68a80130`. The separately owned RUST-OMS-009
branch is not used or modified.

The complete authentic release schemas are used unchanged, independently for
generation and each broker session. The existing facade build verifies both the
message root and its security-markings include before generation:

| Release | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 |
| --- | --- | --- |
| 2.5 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

Pinned Sleet is the clean, unmodified checkout at
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`. The existing runner verifies the
revision and clean status, then builds it with its own toolchain and lockfile.

## Production readiness and generated properties

The two contracts in `tests/fixtures/rust-oms-010/` select global `FileMetadata`,
with input `f/e` and output `f/publish` on authorized topic `file.metadata`.
Production `service-check` and `service-generate` run separately for each release
with `--language rust --world closed-schema --with-codec`.

| Release | Selected/renderable messages | Selected/renderable types | Codec declarations | Model / codec |
| --- | --- | --- | --- | --- |
| 2.5 | 1/1 | 52/52 | 50/50 | READY / READY |
| 2.6 | 1/1 | 54/54 | 52/52 | READY / READY |

These are observed production Rust counts, not inferred from another backend.
Build-time assertions require these results and retain `readiness.txt` in each
generated directory. There are no unsupported selected declarations. Each model
and API is also generated without codecs: all Rust source must be byte-identical
after removing only the expected codec sibling-module inclusion. Distinct
`TypeId`s prove independent version-specific `FileMetadataMT` models. Both
endpoints must name global `FileMetadata`, not its payload type.

Authentic UCI 2.6 adds optional `FileSize`; its independent documents include
4096 bytes. Both releases permit optional `SHA_2_Hash` with authentic
`SHA_2_256_HashType`, emitted as checked Rust `SHA2256HashType`.

## Independent lexical and binary oracles

The normal, second and hash JSON documents are authored independently of
generated encoders. They include distinct metadata UUIDs, filename, description,
source, creation source, modification flag, timestamps and security markings.
Tests inspect generated fields and compare the complete encoding with those
authored documents; roundtripping alone is not the oracle.

Filename positives are `Report_01.txt`, `archive.tar.bz2`, and exactly 255
characters (251 uppercase `A`s followed by `.txt`). Rejection cases cover
`report.`, empty text, missing basename/extension, spaces, slash, tab, non-ASCII,
256 characters, wrong JSON types, and omission. Checked lexical values are
compared unchanged: no path, Unicode, case or platform conversion is performed.

The hash oracle is a literal array of 32 bytes, `00` through `1F`, and separately
fixed uppercase hex
`000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F`.
Tests cover checked constructor bounds, exact decoded bytes, uppercase encoding,
lowercase decoding, invalid digits, odd hex length, too-short/long values, wrong
JSON types, and omitted optional hash staying omitted.

## Transport boundary and lifecycle assertions

Each release starts a separate full-schema broker and runtime session. A
generated checked publication and a distinguishable independent `CalClient`
publication must both reach the generated typed handler. A separate pinned
`CalClient` observer receives actual routed OWP `MSG` payloads. The 255-character
filename also traverses the real broker.

Local checked-model rejection, direct generated-codec rejection, broker-side
rejection and subscription-decoder rejection are tested separately. Broker
rejection requires a server error, no observer message, no invalid callback and
no fabricated subscription error. Broker-admitted malformed documents require
an actual observed invalid message and `SubscriptionDecodeError` carrying the
original subscription ID, global identity and codec member path. Every negative
is followed by valid traffic on the same active subscription.

The production runtime retains non-verbose OWP semantics. Its typed publication
returns after writing, not after broker acknowledgement; broker rejection of
the valid hash must arrive asynchronously as `RuntimeEvent::ServerError`.
Independent publishers use the pinned client's verbose default and must receive
`ClientError::Server` for rejected documents. Neither outcome is reported as a
generated-decoder failure or connection closure.

The known pinned broker length-facet boundary is explicit: it counts hexBinary
lexical characters, not decoded bytes. Consequently a valid 32-byte / 64-digit
hash is expected to be rejected by Sleet before routing. A 16-byte / 32-digit
hash and 32 invalid hex characters pass that broker facet but must be rejected
by the generated subscription decoder. This is not a Rust defect, and neither
the broker nor authentic schema is patched to conceal it.

Wrong global element and unauthorized topic are independent broker-rejection
probes. Incorrect schema negotiation is tested on a separate `INIT` connection,
not by mixing models on the active session. This terminal handshake rejection
must not stop the valid original subscription.

A second generated handler witnesses valid traffic after removing the original
handler. Only the second may receive it. Both are explicitly unsubscribed.
The established worker TLS teardown barrier proves that `close` waits for the
exact live worker's teardown before returning; the closing thread is joined.
Independent clients/executor are dropped, the broker is stopped and reaped,
its temporary configuration is removed and its listener can be rebound.
On Linux the test first witnesses live established TCP connections and then
requires zero established connections to that broker port **before** stopping
the broker. TIME_WAIT sockets are not counted as leaked connections.

## Reproduction and integration

```bash
export AMS_GRA_UCI_2_5_ROOT=/absolute/path/to/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/absolute/path/to/UCI_MessageDefinitions_v2_6_0.xsd
AMS_GRA_RUST_FILE_METADATA_ONLY=1 bash /absolute/path/to/repository/scripts/run-real-sleet-test.sh /absolute/path/to/clean-sleet-checkout
```

The focused mode requires both roots and fails closed before building Sleet if
either is missing. It uses the existing checkout/build and runs no standalone
application. The ordinary real-Sleet path also runs this qualification when
both roots are configured, without new workflows or runner lanes.

The runner requires exactly five FileMetadata tests and one external-consumer
test, both-release model/codec and live interoperability markers, both-release
subscription removal/cleanup markers and independent consumer markers. Skips,
ignored tests and zero-test runs cannot satisfy those requirements.

The external Cargo consumer includes the actual generated source directories
and depends only on production runtime/API crates and `serde_json`, never the
facade crate. It decodes authentic metadata, inspects checked filename/hash
values, encodes with `ServiceCodec`, checks release identity and type-checks
generated subscribe/publish functions against production `SleetRuntime`.

Local evidence logs are retained outside the source tree at
`/home/zboll/rust-oms-010-evidence/`. Final local/hosted check results, commit/tree
and any blockers are reported on the delivery PR rather than using
self-referential source hashes here.

Remaining limitations: pinned Sleet cannot route the valid optional hash with
its current length-facet semantics. Hash-free authentic metadata transport is
qualified independently. This does not qualify file I/O, digest computation,
cross-version sessions, WSS, reconnect policy or operational applications.