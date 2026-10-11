# Task 073 — Unknown-subtype validation and quarantine authority

## 1. Executive disposition

**VALIDATION_AND_AUTHORIZATION_CONTRACT_NOT_ESTABLISHED**.

CAL establishes schema validity and concrete identity, but the inspected evidence
does not designate a runtime authority for schemas absent at generation time,
or grant quarantine holders permission to publish. A deployment can propose such
an authority; feasibility is not a reviewed, interoperable contract. The safest
currently supported path is an explicitly supplied private schema at generation
time, under ClosedSchemaSet, with ordinary checked generated APIs. Unvalidated
opaque retention would be a separate, unsupported quarantine facility, never an
OpenExtensions readiness shortcut.

This independent research change adds tests and documentation only. It changes
no production model, frontend/IR, codec/runtime API, dispatch, Sleet policy,
readiness, world semantics, guard or CI requirement. No new Open service is READY.
Task 072 [PR #72](https://github.com/zackboll/ams-gra-codegen-oms/pull/72) is an
independent pending change, inspected at the immutable head below; none of its
new files is required to compile, test or read this deliverable.

## 2. Immutable identities and isolation

| Input | Immutable identity |
|---|---|
| Fetched starting origin/main | `a444b2d7ecc96207ad720fcaf71b8d78d240013a` |
| Merged Task 068 evidence | `docs/task-068-open-world-abstract-values.md` and `tests/fixtures/open-world/task068-input-manifest.txt` at starting main |
| Pending PR #72 read-only context | `717dff01133525b0ce0d657fbbb6f9371f7052d5` |
| UCI repository | `https://gitlab.com/open-arsenal/uci/standard.git` |
| UCI 2.5 revision / root SHA-256 | `093610b7753944059360d3236770ab446d039556` / `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| UCI 2.6 revision / extracted root SHA-256 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` / `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| OMS repository / revision | `https://gitlab.com/open-arsenal/oms/standard.git` / `726272bd0390982a759c91a9cf4e13b81c2b510b` |
| Official CAL DOCX | `docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx` |
| Official CAL DOCX SHA-256 | `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7` |
| Sleet repository / revision | `https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet` / `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Toolchain used | Rust `1.95.0`; existing stable toolchain file and pinned Cargo lock unchanged |

The original checkout was clean at `263df8be24439f368b64cfde56b9b0c70a93e226`
on Task 059. Remote/local branch searches and hosted PR search found no Task 073
branch or PR. Created `/home/zboll/git/ams-gra-codegen-oms-task073` directly from
the fetched SHA, branch `research/073-unknown-subtype-authority`. No sibling
worktree edited; no Task 072 merge, cherry-pick or rebase. Research output is
under `/home/zboll/task073-research`, not a sibling target directory.

Rechecked both cached UCI root hashes and official CAL DOCX hash; inspected the
DOCX XML directly and verified the cached OMS and Cargo Sleet checkout revisions.
These identities establish reproducibility, **not signer authentication**.
Full historical UCI/PET document identities remain in the merged Task 068
manifest. This task does not recertify its entire schema-wide inventory.

## 3. Normative evidence versus implementation and proposed policy

CAL §6.1.2 requires `$type` for a complex type used where another was expected,
with a valid derivation relationship (referencing XML Schema complex type
derivation). Its spelling is the concrete type's local name in the exact OAM
namespace, otherwise `{namespace}local`. XML prefixes, namespace URLs that look
trusted, and successful discriminator lookup are not proof of schema validity.

CAL §6.1.5: “An OMS JSON message is valid if and only if its schema-validity
assessment is valid with respect to its XML Schema.” Associated document/element
information items and content-model assessment matter. §6.1.3 does not supply a
general ignore-unfamiliar-members rule. §6.1.2 allows varying sequence member
order; §6.1.5.4 maps numeric characters into character information items. No
general duplicate-resolution, lexical canonicalization or exact-byte round-trip
rule was established in these sections.

UCI annotations and merged Task 068 support private concrete descendants and
generation-time extensions; they do not establish how an incoming private
schema becomes authoritative. No inspected CAL rule prescribes a signature,
certificate, trust registry, or revocation protocol for these bundles. All such
mechanisms below are **deployment-policy proposals**, not invented OMS mandates.
Absence in this bounded review does not rule out program-specific security rules.
CAL 2.5 plus UCI 2.6 is not by itself an interoperability certification.

Pinned Sleet implementation evidence:

- `sleet/src/validator.rs::resolve_effective_complex_type_for_value` looks up
  `$type` in loaded complex types, rejects unknown/abstract/non-derived candidates.
  `validate_complex_type` checks declared/inherited members and occurrences.
- `sleet/src/server.rs` publication checks the initialized service's allowed
  topics, optional topic/message bindings, then `validate_oms_json` before routing.
  Subscription also checks service routes. This is not a quarantine permission
  system. Optional absent message bindings are not least-privilege authorization.
- `sleet/src/config.rs` and `main.rs` load schema and service policy at startup.
  This provides deployment association, not a documented runtime schema-trust
  registration/revocation contract. Service identifiers/UUIDs alone do not prove
  cryptographic identity.
- Its local-name schema lookup does not establish private Clark-QName support;
  merged Task 051 QName evidence already demonstrates that limitation. A broker
  accepting known OAM values cannot be assumed to accept private namespaces.

## 4. Current architecture and possible trusted boundary

`crates/codegen-core/src/world.rs` defines OpenExtensions without assuming the
known descendant set exhaustive. `service_generation.rs::admit_abstract_value`
returns `NotClosedUnderOpenExtensions` before backend emission. Closed projection
admits transitive known concrete descendants and their support dependencies;
abstract inheritance ancestry is not itself an abstract-value demand.

`crates/xsd-frontend/src/lib.rs::load_schema_set_with_overlays` is additive,
generation-time composition. It loads the primary closure then ordered overlays,
rejects duplicate QNames, unresolved types and bad dependency namespaces/locations.
Top-level overlays must share the primary namespace; ordinary imported
dependencies can differ. Primary schema version is retained. These checks and IR
member provenance establish structural source relationships, not authenticity.
Pinned fetch scripts check release/root identities; provenance paths and hashes
must not be confused with an authorization decision.

Service plans also verify a semantic schema binding before projection, preventing
reuse against same-named but changed IR. This internal checked-model identity is
not an authenticated deployment trust manifest or a runtime validation permit.

Rust generated service wrappers select explicit message/endpoint identities and
typed payloads. Generated codecs validate the supported checked model, including
known concrete cases and unfamiliar-member rejection; they are not full arbitrary
XSD validators. Runtime `worker.rs` delivers borrowed raw text by subscription ID;
`lib.rs` subscription closure calls `envelope.rs::unwrap_global_element`, then
`OmsJsonCodec::decode_payload`, then the typed handler. Publishing uses checked
typed values, codec encoding, global-element wrapping, and the worker/client.

A possible future boundary is **before** `unwrap_global_element` destroys raw
information and **outside** existing typed dispatch. It would own bounded wire
data and quarantine unknowns; a separate trusted schema assessor and operation
authorizer would mediate release. A schema-validated opaque object still cannot
be injected into a generated typed handler: only an existing compatible generated
variant decoded by its checked codec may cross that boundary. Future unsupported
types remain opaque or require regeneration. Never cast bytes into models or
substitute a known variant merely because the base type matches.

## 5. Trust-source alternatives

| Approach | Supplier and association (proposal) | Benefits / unresolved costs |
|---|---|---|
| Explicit offline trusted bundle | Program schema owner supplies reviewed manifest; deployment operator pins it to service and operation profile | Reproducible, air-gapped, smallest initial surface; updates/revocation require explicit rollout; approval identity still must be specified |
| Deployment-provisioned private extension | Extension owner supplies definitions; deployment authority approves dependencies and binds them to a particular service installation | Fits private extensions and existing closed-overlay workflow; does not authenticate arbitrary sender-provided schemas; namespace/backend/broker constraints remain |
| Runtime-loaded registered bundle | Authorized registrar supplies immutable bundle; deployment registry controls admission and epochs | Dynamic rollout, but requires authenticated registrar, transactional snapshots, dependency closure, rollback/revocation, validator isolation and race semantics not present today |
| Unvalidated opaque quarantine | Sender supplies bytes, not trust; local retention policy controls custody | Can preserve evidence without knowing schema; **no publish or service-forward permission**, no generated conversion, bounded lifetime required |

Recommend evaluating an offline deployment-approved bundle first. Hash pinning
detects substitution relative to an approved digest but does not authenticate who
approved it. A signature/certificate might be selected by a deployment, but this
task selects no existing standard mechanism. Never fetch executable or schema
resources automatically from a QName, payload URL or sender-selected location.

An authoritative bundle proposal must identify: issuer/approver and authenticated
delivery channel; immutable manifest digest; UCI/CAL/profile versions; exact
expanded QNames and declaration versions; complete imports/includes, base types,
member types, facets, substitutions and relevant constraints; validator identity
and supported semantics; service/deployment/operation/trust-context bindings;
validity interval and policy epoch; update/revocation/conflict handling. Unsupported
assessment semantics fail closed, not “valid with warnings.”

An old base schema cannot assess future descendant members and constraints merely
by validating inherited fields. It needs the actual authoritative derived
declaration and dependency closure. A generic extension hook or wildcard is not
proof that this concrete derived object is valid. Closure is relative to a pinned
bundle, never a claim that all future extensions have been enumerated.

Reject missing dependencies and namespace/version conflicts before bundle admission.
Same local names in different namespaces are distinct. Same QName with changed
bytes/version is a new schema identity, never last-wins replacement. Do not guess
between ambiguous active bundles. Unavailable, stale, revoked or ambiguous
authority leaves data quarantined (subject to retention permission) or rejected;
no validation badge or outbound permission survives by default.

## 6. Proposed fail-closed state machine and permissions

This is a decision contract candidate, not an implemented carrier:

```text
ReceivedUnparsed -> ParsedUnknown -> QuarantinedOpaque
       |                 |                 |
       +-----------------+-----------------> RejectedOrExpired
                                           |
QuarantinedOpaque -- trusted full assessment --> SchemaValidatedOpaque
SchemaValidatedOpaque -- scoped live grant --> AuthorizedForOperation
AuthorizedForOperation -- atomic recheck/use --> operation only
```

No state transition is caused solely by possession, `$type` recognition, JSON
parsing or a sender's “trusted” flag. Malformed/duplicate/budget-exhausting input
is rejected, not promoted to well-formed unknown. Even ParsedUnknown requires
bounded syntax and exact discriminator inspection; it has no publication rights.
Validation records refer to immutable bytes, bundle, validator/profile and base
value position. Authorization adds principal, service identity, operation,
destination/topic/message, trust context, policy epoch and expiry. Missing
identity guarantees must be explicitly designed, not asserted as supported.

| Permission | Proposed independent requirement |
|---|---|
| Retain | Custody/storage quota, purpose and TTL; not schema validity |
| Inspect metadata | Access-controlled minimal identity/size/status; identity is asserted until assessed |
| Inspect payload | Separate confidentiality permission; redaction and audited access, not automatic logging |
| Decode registered type | Approved codec/schema binding, sandbox/budgets; decoding is not publish permission |
| Forward | Explicit destination and operation grant plus required validation; diagnostic export only under a separate non-OMS custody policy |
| Publish | Full authoritative message assessment and live service/topic/message grant, peer capability |
| Convert generated typed value | Exact supported generated variant, checked decoder/model construction; never unknown-to-known coercion |

Schema validity is mandatory before any proposed **service-use release** from
quarantine. Inspection/retention can occur in quarantine without it. Whether
validated custody should still be called quarantine is terminology, not permission.
Forward and publish are separate grants; neither follows from retain or decode.

TOCTOU: bind assessment to owned immutable bytes and bundle digest; bind grants
to policy epoch, scope and expiry. Recheck at actual queue dequeue/send, not just
enqueue. Policy replacement/revocation invalidates pending grants. Define atomic
snapshot/use or cancel-and-revalidate semantics and what happens to in-flight
sends; a wall-clock check alone cannot eliminate the race. Never reuse a permit
for a changed payload, destination, service or schema. The pure test predicate
demonstrates invalidation logic only, not an atomic runtime solution.

## 7. Raw wire and codec preservation contract

Pinned `sleet-client::ReceivedMessage` owns `Arc<str>` raw OMS JSON; worker dispatch
receives `&str`. Earliest demonstrated irreversible OMS JSON loss is
`serde_json::from_str::<Value>` in `unwrap_global_element`, **before** its
one-member check or payload codec. OWP/WebSocket framing is already consumed;
this is not a packet capture contract. Invalid UTF-8 cannot inhabit `str`.

Proposed minimum ingress owns bounded UTF-8 message bytes plus exact value spans
and expanded discriminator QName; keeps unknown members/nested content; records
framing/message/service context; survives callback return with independent owned
storage. Borrowed transport references must not escape callbacks. Retain the full
envelope when validation needs message context, not only an extracted subtype.

| Concern | Current seam / required future decision |
|---|---|
| Duplicate keys / conflicting `$type` | Value keeps last occurrence, including duplicate envelope keys. Reject **all** duplicate keys recursively before Value normalization; identical duplicates are also ambiguous policy inputs |
| Numbers | `1e0` loses spelling; decimal floating representations can lose precision. Keep raw numeric tokens and assess with appropriate schema semantics; no float normalization of unknown data |
| Member order | Current map reserialization sorts in the pinned build. CAL allows particle order variation, but preserve wire order for evidence; canonicalization needs its own reviewed profile |
| Unknown members | Value retains them semantically; generated checked codec rejects unfamiliar fields. Never strip them to force a known variant |
| UTF-8 / escapes | Text ingress is UTF-8; Value decodes escapes and loses spelling, rejects lone surrogate escapes, accepts valid pairs. Reject invalid input before trust classification |
| Depth / size | Default JSON recursion limit rejects deep input; envelope has no explicit deployment payload-size quota. Neither is a complete resource contract |
| Ownership | Raw client is owned, dispatch borrow temporary, decoded typed values owned. A future retained unknown must copy/share immutable owned bytes under quota, not borrow |

No production parser or public codec API is changed here. Demonstrated lexical
loss is not a claim that CAL mandates byte-preserving codecs, nor that raw bytes
alone establish validation authority.

## 8. Resources and denial of service

Require finite configured budgets before accepting quarantine: frame/message
bytes, discriminator/string/number lengths, depth, tokens/nodes, members, arrays,
repeated values, decoded/retained aggregate bytes, queue count, validation CPU/time,
bundle/dependency size/count and output bytes. Bound before allocations and deep
assessment. Use checked arithmetic; handle allocation failure per language.
TTL/eviction, per-principal rate limits and bounded error telemetry prevent
quarantine from becoming a durable unauthenticated storage/logging sink. No
network import resolution, executable schemas or recursive dependency expansion
without a closed approved manifest. Schema compilation also consumes resources.

Recursive JSON is a finite serialized tree but can exhaust stack/CPU; recursive
generated model topology remains a separate guard. Budget exhaustion rejects
without partial publish, repeated retry, or truncated “validated” data. Exact
boundary/boundary+1 tests and policy-replacement races are implementation gates.
Synthetic thresholds in the new truth-table test are not recommended defaults.

## 9. Rust / Ada / C++ implications

Rust: preserve raw owned immutable storage before Value; keep authority objects
outside generated layouts; no Serde Value variant added to existing models.
Approved registered decode still passes checked codec construction. Heap failure,
Send/Sync ownership, bounded queues/destruction and cancellation need explicit
tests; allocator abort is not magically caught by a Result.

Ada/SPARK: bounded owned buffers and explicit status records, no escaping borrowed
transport pointers or unchecked conversions into discriminated models. Resource
capacities and assignment/copy cost need proof or reviewed runtime boundaries.
Dynamic schema assessment is outside a proven model unless separately justified.
No new Ada validator/runtime or GNATprove certification is claimed.

C++17: owned RAII bytes and immutable evidence handles; checked construction,
exception-safe temporary validation and quota checks; no casts or reinterpretation
into generated variants. ABI/version changes and allocation exceptions need
explicit handling. There is no production equivalent codec/runtime implementation
demonstrated here. Cross-language semantic/ownership equivalence remains a gate,
not permission to silently enable only one backend.

## 10. Executable evidence and limitations

New test-only files (no production library additions):

- `crates/runtime-rust-facade-tests/tests/task073_authority.rs`: known checked
  descendant round trip; unknown future/private and namespace-collision rejection;
  nested unknown-member rejection; duplicate discriminator/envelope collapse;
  escape/order/numeric loss; UTF-8 and surrogate cases; finite/deep recursive
  input and size observations; proposed trusted/untrusted private-bundle and
  epoch/expiry/service/context/grant decision table; synthetic budget exhaustion.
- `crates/cli/tests/task073_vertical.rs`: opt-in hash-checked UCI vertical using
  only production APIs at main, closed descendant/support counts, all three model
  renderers, exact unchanged Open guard.

The proposed gate assumes `validated` as an input. It does not validate schemas,
authenticate bundles, implement states, test concurrency, or grant real permissions.
Both “trusted private” and “untrusted private” decisions are **policy simulation**,
not a working registered-schema runtime. Existing Task 029 and Task 068 controls
provide the actual generation-time private-overlay/codec evidence. Pinned Sleet
source was inspected; no new Sleet interoperability success is claimed and no
multi-hour full-schema campaign is necessary.

### Final local validation

Rust 1.95 fmt check, workspace all-targets check and clippy with `-D warnings`
passed on the final test sources. Focused debug invocations reported 637 passing
tests, zero failures: 636 executed assertions plus the Task 071 real-Sleet test
that returned early because `AMS_GRA_SLEET_BIN` was unset (not live broker evidence).
The five new Task 073 tests all executed and passed. Runtime unit tests: 16;
generated codec: 7; QName codec: 8; dispatch/errors/lifecycle: 3/4/4;
Task 068 codec guard: 1. Core suite: 343 unit tests plus 140 integration tests;
CLI support-binding/codec/generation: 2/11/68. Task 028: 6 per backend;
Task 029: 2 per backend. All six frozen Task 068 readiness message sets and
fixture accounting passed. Both opt-in release vertical tests passed (one each
for UCI 2.5/2.6, approximately 6 seconds each), including three-backend rendering.
`git diff --check` passed.

Additional focused commands used:

```sh
cargo +1.95.0 test -p ams-gra-oms-runtime-rust-facade-tests \
  --test task073_authority --test task068_open_contract --test generated_codec \
  --test generated_codec_qname --test runtime_dispatch --test runtime_errors \
  --test runtime_lifecycle --test service_route_policy
cargo +1.95.0 test -p ams-gra-oms-runtime-rust
cargo +1.95.0 test -p ams-gra-oms-codegen-core
cargo +1.95.0 test -p ams-gra-codegen-oms \
  --test service_generate --test service_codec --test generated_support_binding
cargo +1.95.0 test -p ams-gra-oms-backend-rust -p ams-gra-oms-backend-ada \
  -p ams-gra-oms-backend-cpp task028
cargo +1.95.0 test -p ams-gra-oms-backend-rust -p ams-gra-oms-backend-ada \
  -p ams-gra-oms-backend-cpp task029
```

Initial attempts caught a test API argument-order error, invalid-UTF-8-literal
warning, and an assumed float rounding result. Corrected all before the final
clean run; the locked parser actually maps `9007199254740993.0` to
`9007199254740994.0`, illustrating why unknown numeric tokens need preservation.
Validation logs are external at `/home/zboll/task073-research/validation-final.log`
and `vertical.log`. Hosted exact-head CI is separate publication evidence.

Reproduce from `/home/zboll/git/ams-gra-codegen-oms-task073` with Rust 1.95:

```sh
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --workspace --all-targets
cargo +1.95.0 clippy --workspace --all-targets -- -D warnings
cargo +1.95.0 test -p ams-gra-oms-runtime-rust-facade-tests --test task073_authority
python3 scripts/check-task068-fixtures.py
# Run separately with each hash-pinned root (external, not vendored):
AMS_GRA_TASK073_UCI_ROOT=/tmp/task068/uci25/03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd \
  cargo +1.95.0 test --release -p ams-gra-codegen-oms --test task073_vertical -- --ignored
AMS_GRA_TASK073_UCI_ROOT=/tmp/task068/uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd \
  cargo +1.95.0 test --release -p ams-gra-codegen-oms --test task073_vertical -- --ignored
git diff --check
```

Fresh sources can use the existing pinned fetch scripts; cached roots were reused
read-only and hash-checked here. Root checks alone are not cryptographic proof of
every dependency; an implementation bundle must attest the whole closure.

## 11. Real UCI vertical

Both authoritative roots define:

```text
DataRecordListManagementRequest -> DataRecordListManagementRequestMT.MessageData
 -> DataRecordListManagementRequestMDT.{AddRecord,DeleteRecord}
 -> ManagedListBaseType (abstract)
 <- ForeignKeyMapML (concrete extension)
```

AddRecord/DeleteRecord are optional scalar values. ForeignKeyMapML contains
optional unbounded ForeignKeyPairListElement of ForeignKeyPairDRLE. ManagedListBaseType
annotations require ML extensions with list entries; its open-ended nature does
not make a future asserted concrete name self-validating. Frozen Task 068 evidence
has 43 selected + 4 support, Closed READY, Open blocked at ManagedListBaseType.
The new independent opt-in probe checks projection and rendering, not opaque
runtime handling or actual ForeignKeyMapML message publication.

For hypothetical `{urn:deployment-private}FutureManagedList`, trust must cover its
actual definition, derivation from the exact base version, members and complete
dependency closure, approval for this deployment/service, and validator capability.
Assessment must include inherited request/message constraints and the actual
AddRecord/DeleteRecord value position, not just the local object. Authorization
must separately permit the operation and destination under current policy and
confirm peer schema/namespace support. Existing top-level overlay/backends have
single-namespace restrictions; hypothetical cross-namespace support is not proven.
No successful runtime handling of this future subtype is claimed.

## 12. Remaining blockers and resolution paths

1. Designated authenticated approver/delivery mechanism and deployment binding.
2. Full OMS-JSON/XSD assessment profile, validator provenance, supported constructs
   and attested dependency closure (not frontend normalization alone).
3. Namespace/version conflict and transactional update/revocation semantics.
4. Operation-level custody/decode/forward/publish/conversion policy and authenticated
   service/principal identities; existing route allow-lists are insufficient.
5. Raw duplicate-aware bounded parsing and immutable evidence ownership contract.
6. Atomic policy recheck/use, queue expiry, cancellation and in-flight send rules.
7. Peer private-schema/QName interoperability and language resource/proof boundaries.

Next bounded research step: have deployment/security owners select one offline
bundle authority and one service-operation policy, then qualify a pinned external
validator on synthetic and this UCI vertical, including unsupported constraints,
dependency substitution and revoked-policy tests. Do not start a production relay.

## 13. Exact prerequisites for an implementation task

Approval must specify all seven blockers above, reconcile independent pending
PR #72 explicitly, and select finite budgets/error semantics and backend scope.
Require reviewed raw-ingress/ownership and validation evidence records; enforce
no service-use release without full assessment and an operation-specific live
grant; prove typed conversion still uses the generated checked model. Add negative
tests for same-QName replacement, wrong namespace/version, stale/revoked authority,
duplicate keys at every depth, missing dependencies, resource exhaustion and races.
Demonstrate approved peer behavior; specify rollback and audit without payload
leakage. Until then keep `NotClosedUnderOpenExtensions`, closed model layouts,
public APIs, readiness and CI requirements unchanged. Documentation approval is
not authority to remove the guard. Stop at review; do not merge or begin Task 074.