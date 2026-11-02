# Generated Rust DLZ message interoperability

## Scope and source

This qualification covers UCI 2.5 and 2.6 DLZ message exchange only. It adds no
engagement calculations, targeting, launch-zone computation, transport runtime,
temporal parser, or handwritten payload codec. Production generation was already
ready; no generator or runtime implementation changes were needed.

The dedicated branch starts at current main
`4f395938a700afd3a72f2b233c501031fe08ad1f`, tree
`96f0cda8c382f95cbf8246b36f8145a359470ee6`. The supplied reference
`9d8faf7b2675acc7d675f961c6c2d48bbc43d526` is an ancestor. At inspection PR #167
was already merged into main; its branch was not modified and its standalone
application is not used by this qualification.

## Authentic inputs and production readiness

The complete, unchanged release schemas are used for generation and by Sleet.
Their include closure is not rewritten or reduced. The existing facade build
verifies MessageDefinitions and SecurityMarkings before generation:

| Release | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 |
| --- | --- | --- |
| 2.5 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

The contracts under `tests/fixtures/rust-oms-006/` preserve Task101's selected
global `DLZ`, input exchange `f/e`, and authorized topic `task098-dlz`, adding
only output exchange `f/publish`. Each release runs actual production
`service-check` and `service-generate` with
`--language rust --world closed-schema --with-codec`.

| Release | Selected/renderable messages | Selected/renderable types | Codec declarations | Readiness |
| --- | --- | --- | --- | --- |
| 2.5 | 1/1 | 54/54 | 53/53 | model and codec READY, no diagnostics |
| 2.6 | 1/1 | 54/54 | 53/53 | model and codec READY, no diagnostics |

Build-time assertions require those actual Rust counts, not the prior Ada
results. Each generated directory retains `readiness.txt`. Generation with and
without codecs must have byte-identical Rust model/API files after removing only
the codec sibling-module declaration from the latter API. Both independently
generated model, codec, and typed endpoint surfaces are compiled. Distinct
`TypeId`s rule out a cross-version type alias; callbacks check the corresponding
`002.5.0` or `002.6.0` schema identity. Both endpoints must name global `DLZ`,
never payload type `DLZMT` or schema type `DLZ_MT`.

## Independent numerical and temporal expectations

The Task101 full DLZ fixtures remain unchanged. The compact corpus snapshots
are derived from `scripts/task101/cases.py` and checked against that source by
the existing runner. There are ten Task101 positive documents and fourteen
negative documents per release; the Rust test adds one independent Float32
document and focused broker-boundary negatives.

The actual generated model fields, decoded from direct inputs and observed
broker-routed messages, are checked against fixed IEEE-754 bit patterns:

| Float64 value | Expected bits |
| --- | --- |
| `1.0000000000000002` | `3ff0000000000001` |
| `-0.0` | `8000000000000000` |
| `5e-324` | `0000000000000001` |
| `1.7976931348623157e308` | `7fefffffffffffff` |
| `-12.5` | `c029000000000000` |
| `0.25` | `3fd0000000000000` |

The actual generated `UnitBallFloatType` vector components are independently
checked as Float32, including one ULP above 0.5 (`3f000001`), negative zero
(`80000000`), and the least positive subnormal (`00000001`). Typed publication
constructs the interesting numeric carriers from these fixed bit patterns.
Neither generic JSON equality nor encoder/decoder symmetry is the numerical
oracle. Float32 wire numbers can acquire a different intermediate binary64
expansion during parsing/reserialization; their generated Float32 bits must
still match exactly. All other document members remain compared.

Checked `TimeType` values preserve `12:34:56.5000Z`, `24:00:00.000Z`,
`23:59:60Z`, long fractional spellings, and each individually omitted optional
Time. The existing constructor collapses surrounding XML whitespace and retains
the resulting lexical spelling. No calendar/timezone conversion or arithmetic
is performed. Malformed lexical values are rejected by both direct model
construction and generated decoding.

## Broker versus generated-decoder boundaries

Tests use the existing `SleetRuntime`, production-generated `ServiceCodec` and
endpoints, and pinned `CalClient`. Each release starts its own broker using only
its complete schema; no sessions mix releases. An independent CalClient
subscription observes actual OWP `MSG` payloads for every successful publication.
An external publisher also exercises independently authored input documents.

Pinned Sleet rejects missing required TimeOfIntercept, numeric/null Time,
numeric timezone offset, quoted Float64, a negative constrained distance, both
OwnerProducer Choice arms, the wrong global name, and unauthorized topics.
The tests require server errors and no routed invalid message or typed callback.

Two observed pinned-broker boundaries are intentionally kept distinct:

* **Surrounding whitespace:** Sleet rejects the raw tab/newline Time document
  with a pattern FacetViolation before routing. Generated decoding accepts and
  collapses that valid lexical input. Generated typed publication then routes
  the normalized 50-digit fractional spelling, which is checked exactly.
* **`not-a-timeZ`:** Sleet admits and actually routes the invalid DLZ. The raw
  observer must receive that document; the generated codec must reject it.
  `SubscriptionDecodeError` must carry the exact original subscription ID,
  global `DLZ` identity, codec error, and TimeOfIntercept path. No invalid typed
  callback may fire. A subsequent valid DLZ must reach the same subscription.

Every focused negative is followed by valid typed publication and observed
delivery without replacing the original subscription. Unauthorized-topic
recovery is witnessed before removing that subscription. A second generated
subscriber proves the connection remains live after the original unsubscribes;
only the second receives the subsequent publication. Both unsubscribe cleanly,
the runtime closes, and the broker process is reaped.

Duplicate-member behavior remains the existing `LegacyLastWins` policy, not a
DLZ-specific parser. Direct tests cover valid-last and invalid-last Time members;
the existing Task077 live runtime test exercises both configured policies.

## Reproduction and external consumer

Use Rust 1.95.0 and supply both authentic release roots:

```bash
export AMS_GRA_UCI_2_5_ROOT=/absolute/path/to/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/absolute/path/to/UCI_MessageDefinitions_v2_6_0.xsd
AMS_GRA_RUST_DLZ_ONLY=1 bash /absolute/path/to/repository/scripts/run-real-sleet-test.sh /absolute/path/to/clean-sleet-checkout
```

The existing runner checks out and builds unmodified Sleet revision
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` (tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`) with its pinned toolchain. Missing
either root or mismatched root/include hashes fail closed. The focused mode
does not execute the standalone client or unrelated qualifications. Default
execution also includes DLZ when both roots are available, so the existing
main-only `real-sleet` job picks it up without workflow or runner-lane changes.

The runner requires five actual DLZ tests, one actual external-consumer test,
both release-specific model/codec and live-delivery markers, and both external
consumer markers. Diagnostic skip tests cannot satisfy these checks.

`real_dlz_consumer` creates a separate Cargo package depending only on the
production runtime crates and serde_json, includes both actual generated source
directories, compiles typed publication/subscription against SleetRuntime, and
executes codec, bit, temporal, and message-identity assertions. It does not
depend on the test-only facade crate.

Local logs and exit records are retained outside the source tree at
`/home/zboll/rust-oms-006-evidence/`. Initial failed probes are retained, not
counted as successful qualification. Final commit/tree and local/hosted gate
results are reported on the delivery PR to avoid self-referential hashes here.

Remaining limitations are the pinned broker's documented admission/whitespace
boundaries, not weakened Rust validation. General deployment, reconnect, TLS,
hardware, operational behavior, and cross-version session mixing remain out of
scope. Hosted PR smoke does not substitute for local complete-schema live
interoperability qualification.