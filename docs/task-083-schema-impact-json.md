# Task 083 — versioned static schema-impact JSON

Starting fetched `origin/main`: `051a634fa7a69f87a9c4d6330b6edee1bcc5c0bf`
(merged Tasks 078 and 081). Branch: `feature/083-schema-impact-json`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task083`.
Task-owned build/log storage: `/tmp/task083`.
PR #82's inspected head was `a348567ba3e97ea93d4948f3175ef06ecad7cb62`;
none of its changes are incorporated. Changed paths have zero overlap.

## CLI and API

```sh
ams-gra-codegen-oms schema-impact --before OLD.xsd --after NEW.xsd --format json
ams-gra-codegen-oms schema-impact --before OLD.xsd --after NEW.xsd \
  --before-overlay OLD_EXTENSION.xsd --after-overlay NEW_EXTENSION.xsd --format json
```

Default/text and TSV retain the Task 078 contracts. Only schema-impact accepts
JSON; schema-diff and service-impact still reject it. Each overlay stream retains
caller order independently. Singular options cannot repeat. Unknown flags,
unsupported formats, missing options/values and generation/service flags are
usage errors. Exit codes: 0 for a completed report, including findings; 1 for
loading, validation, analysis, serialization or writer failure; 2 for usage.
The existing error-aware stdout writer receives a completely serialized report.
Failed input produces no partial JSON. A failed physical write can, naturally,
leave a prefix at the destination and remains an execution failure.

`SchemaImpact::to_json() -> Result<String, serde_json::Error>` serializes the
existing typed `SchemaDiff` and `DependencyExposure` facts. Direct payloads come
from Task 069's `Value::Reference`, not Debug strings. Unexpected manually
constructed non-reference direct values fail explicitly. No new comparison,
traversal, schema loading, readiness or compatibility algorithm is introduced.

## Frozen schema-impact-json-v1 contract

One compact UTF-8 JSON object followed by exactly one LF. All listed fields are
required unless explicitly described as variant-specific. Key order is the order
below, implemented by serialization structs rather than map iteration.

Top level:

1. `format`: string, exactly `schema-impact-json-v1`.
2. `scope`: string, exactly `STATIC_NAMED_REFERENCES`.
3. `compatibility`: string, exactly `NOT_ASSESSED` (not a verdict).
4. `before`: snapshot object.
5. `after`: snapshot object.
6. `directUniqueMessages`: nonnegative integer, distinct direct-change QNames.
7. `directMessageChanges`: ordered direct-change array.

Each snapshot object, in order:

* `schemaVersion`: authored string or null. An empty string remains `""`.
* `types`: nonnegative integer, existing declaration inventory count.
* `messages`: nonnegative integer, existing message inventory count.
* `exposedUniqueMessages`: nonnegative integer, distinct message QNames in this
  snapshot's exposure records.
* `exposurePairs`: nonnegative integer, this snapshot's message/seed pairs.
* `exposures`: ordered exposure array.

Every QName is `[namespace_uri, local_name]`, exactly two strings. Prefixes,
concatenated display names and local-name-only keys are never identities.
An exposure has `message` (QName), `seed` (QName), `seedChange` (exactly `ADDED`,
`REMOVED` or `CHANGED`), then `witness` (ordered array of QNames).
Witnesses start at the named payload and end at the seed; the message-to-payload
edge is separate. The payload itself may be the seed (one-step witness).

A direct change has `message` (QName), `change` (existing Task 069 label:
`MESSAGE_ADDED`, `MESSAGE_REMOVED` or `MESSAGE_CHANGED`), then `before` and
`after` (payload reference objects or null for absence). Each reference has:

* `kind`: `named` or `primitive`.
* `qname`: required only for named references; omitted for primitives.
* `primitive`: required only for primitives; omitted for named references.
* `binaryEncoding`: required, null or `HexBinary` / `Base64Binary`.

Primitive labels are `Boolean`, `SignedInteger`, `UnsignedInteger`, `Decimal`,
`Float32`, `Float64`, `String`, `Binary`, `DateTime`, `Time`, `Duration`.
Binary provenance is exactly the optional identity stored on the TypeRef.
Unknown Binary provenance is null, distinct from both known encodings. Named
references have null provenance; no ancestry inference is performed. Absence is
never an empty string or an empty reference object.

## Semantics and determinism

Task 078 alone defines static dependency evidence. Direct payload equality does
not imply absence of transitive exposure or any compatibility conclusion.
Type property changes deduplicate by full seed QName. Optional/repeated named
members remain edges; cyclic graphs use the existing finite traversal.
BEFORE uses only BEFORE edges and present changed/removed seeds; AFTER uses only
AFTER edges and present changed/added seeds. No mixed-snapshot path, reverse
inheritance edge or derived-type enumeration is fabricated. Unreferenced added
concrete descendants produce no synthetic exposure.

Exposure order is the existing sorted typed result; witnesses preserve its
lexicographically smallest shortest full-QName path. Direct changes retain the
semantic comparator's order. Declaration-order permutations and repeated
serialization are byte-identical. There are no timestamps, absolute input paths,
temporary directories or random IDs in the report. Authored strings, including
control characters and malicious-looking UTF-8, survive JSON round trips.
Source paths and diagnostic text are not serialized.

Witness steps are not exposure pairs. Unique counts are per snapshot; BEFORE
and AFTER counts must not be summed into a cross-snapshot unique count.
Direct-change uniqueness is independent of exposure uniqueness.

This evidence does **not** establish backward compatibility, forward
compatibility, wire-format compatibility, runtime interoperability, safe upgrade
or deployment, generated model/API compatibility, service readiness, or
open-extension completeness. Backend renderability and generated-support
reachability are not implied. No compatible/breaking/safe verdict is emitted.

## Qualification and boundaries

Rust 1.95.0, locked dependencies. Added direct dependencies use already locked
serde 1.0.229 and serde_json 1.0.151. Cargo.lock changes only the CLI dependency
list (two entries); no package version/checksum changes.

Changed and empty JSON goldens were independently authored from fixture facts,
not generated by the Rust serializer. Both are exercised through the actual CLI.
All four original changed/empty text/TSV goldens remain byte-identical; their
existing assertions are retained. Default text, separate format parsing,
duplicate/unsupported options, JSON rejection on other commands, independent
overlay streams, malformed/unresolved input and broken writers are checked.
Synthetic tests cover nested/full-QName witnesses, snapshot isolation, added and
removed seeds/messages, primitive payloads, every primitive label, Binary lexical
provenance, repeated changes on one seed, multiple messages, optional/repeated
and cyclic edges, shortest-path ties, abstract descendant exclusion, namespace
identity, hostile strings, empty metadata, permutations and repeated bytes.

The existing ignored pinned test is augmented, not duplicated. Verified cached
roots and SecurityMarkings dependencies use the unchanged Task 078 hashes. It
parses complete JSON, checks inventories (5,557/5,570 types, 722/725 messages),
exposure counts (722/725 unique messages, 3,000/3,475 pairs), the three exact
added SystemSchedule message identities, every emitted pair and witness against
typed results, and the independently specified OrderOfBattle witness:
`OrderOfBattleMT -> MessageType -> SecurityInformationType` on both sides.
Historical summaries remain unchanged. Repeated JSON bytes match.

Local qualification commands (logs in `/tmp/task083`):

```sh
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
  -p ams-gra-oms-schema-diff -p ams-gra-oms-codegen-core
cargo +1.95.0 check --workspace --all-targets --locked
cargo +1.95.0 clippy --workspace --all-targets --locked -- -D warnings
git diff --check
```

The affected-package campaign passes 809 tests, with six existing opt-in tests
ignored; this includes 37 CLI library tests, 12 schema-impact integration tests,
4 schema-diff CLI controls, 7 semantic-engine tests and 6 static dependency
integration tests. The hash-verified release pinned campaign separately passes
one explicitly requested test. Formatting, workspace all-target check, warnings-
denied Clippy and whitespace checks pass. This is focused schema-only evidence,
not full real-UCI Deep certification or a Sleet campaign.

No Pages publication, comparison browser/packaging/validator/build changes,
workflow changes, runtime changes or backend feature work occur here. A future
separate UI task may consume this JSON alongside Task 082 semantic comparison;
that integration is not Task 083. Hosted exact-head CI is reported separately in
the PR/final handoff; this PR remains open for independent review.