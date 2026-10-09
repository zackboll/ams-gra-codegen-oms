# Task 072 — Open-world abstract-value runtime contract (ADR)

## 1. Executive decision

**RUNTIME_CONTRACT_NOT_ESTABLISHED** — proposed design, not implementation
authorization. The unresolved gate is **validated service use of a concrete
descendant unknown to the generated schema**, not whether bytes can be stored.

Prefer **B: statically typed known cases plus an owned opaque quarantine case**
for a bounded research implementation. Do not yet choose it as the production
OpenExtensions model. Dynamic registration is optional future work, not a
prerequisite for quarantine and not a way to turn unknown JSON into trusted data.
The minimum useful design is specified below so the next review can resolve
specific decisions rather than repeat Task 068's limitations.

An opaque object is **not a schema-validated abstract value**. Receiving it does
not establish derivation, inherited constraints, allowed service use, or peer
acceptance. No unknown object may reach today's generated typed handlers or
publish wrappers. No Open service is newly READY. `NotClosedUnderOpenExtensions`
and all existing closed representations remain unchanged.

Starting main: `a444b2d7ecc96207ad720fcaf71b8d78d240013a` (fetched and verified;
Tasks 068–071 merged via PRs 68–71). Branch:
`feature/072-open-world-value-contract`. Isolated worktree:
`/home/zboll/git/ams-gra-codegen-oms-task072`. No existing Task 072 branch/PR
was found. The original clean Task 059 worktree and sibling worktrees were not
edited. Research/cache directory: `/home/zboll/task072-research`.

## 2. Evidence authority and immutable identities

| Source | Identity and authority |
|---|---|
| UCI 2.5 | `https://gitlab.com/open-arsenal/uci/standard.git`, `093610b7753944059360d3236770ab446d039556`; root SHA-256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| UCI 2.6 | same repository, `78eb61b6112c8bffa40820c33124b57787fc5bd9`; extracted root SHA-256 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| CAL wire specification | `https://gitlab.com/open-arsenal/oms/standard.git`, `726272bd0390982a759c91a9cf4e13b81c2b510b`, `docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`, SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7` |
| CAL reading aid | same revision, unofficial Markdown SHA-256 `5798da43991d5b554ed7575fa55c350aacece483d36841738e57db123f4ee98f`; not substituted for official authority |
| Sleet transport/validator | `https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet`, `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`; implementation evidence, not a normative extension to CAL |
| XML Schema | W3C XML Schema 1.0 Structures, Second Edition (2004), §3.4.6 Type Derivation OK (Complex), referenced directly by CAL §6.1.2; Task 051 records its retrieved source identity |

Task 072 rechecked both UCI root hashes, cloned/checkouted the immutable OMS
revision, checked the official DOCX and reading-aid hashes, and read
`word/document.xml` §6.1.2–6.1.5. Also inspected pinned Sleet
`sleet/src/validator.rs::resolve_effective_complex_type_for_value` and the actual
Rust JSON seam. UCI document identities and PET quotations remain frozen in
`tests/fixtures/open-world/task068-input-manifest.txt`; their historical metrics
are not silently re-certified against this main.

### What is normative, what is not

| Question | Evidence and conclusion |
|---|---|
| Concrete identity | CAL §6.1.2 requires `$type` when a type is used where another was expected. Its value is the concrete complexType name, bare for the exact OAM namespace, `{namespace}local` otherwise. This is an expanded QName encoding, not an XML prefix, host identifier, version, or local-only discriminator. |
| Legitimate private descendants | UCI's abstract-extension annotations (Task 027) explicitly allow types not documented in the open schema; PET design text (Task 068 manifest) describes polymorphism. This supports generation-time private overlays, not universal runtime acceptance of every asserted QName. |
| Unknown to this generator | A private derived type can be valid against a larger associated schema even if absent from the generator's schema. CAL does not require this generator to accept or validate it without that schema. |
| Validation | CAL §6.1.5: “An OMS JSON message is valid if and only if its schema-validity assessment is valid with respect to its XML Schema.” §6.1.2 references valid derivation. A discriminator alone is not proof. |
| Unknown members | §6.1.3 prescribes members from the content model. No general “ignore all unfamiliar fields” rule was found. A valid private descendant may have unfamiliar members; stripping them cannot preserve that value. Retaining all members is our carrier policy, not a CAL unknown-field mandate. |
| Round trip | No exact lexical decode/re-encode mandate was found in the inspected sections. Object particle order may vary (§6.1.2); §6.1.5 maps objects into schema-valid particle order. Semantic validation is required; universal JSON-value equality is not specified as a round-trip test. |
| Duplicates/order/whitespace | No duplicate-key preservation or duplicate-discriminator resolution rule was found. Member ordering is explicitly flexible for sequences. No transport whitespace preservation requirement was found. Rejecting duplicate keys is proposed security policy. |
| Numbers | §6.1.5.4 maps characters of numbers into character information items. Do not assume changing numerical spelling is safe for every unknown schema/facet. No blanket numerical canonicalization permission or spelling-preservation obligation was found. |
| Publishing/forwarding | Syntactic JSON and an asserted `$type` do not satisfy §6.1.5. No unconditional permission to publish an unvalidated opaque object was established. Separate deployment authorization and schema validation are necessary. |

The inspected evidence is bounded: absence of a rule here is not proof that no
other program-specific CAL/security contract exists. CAL 2.5 and UCI 2.6 are not
automatically a certified interoperable pair. The old UCI 2.3.2 C++ API in Task
027 is secondary corroboration only, not this ADR's wire authority.

Pinned Sleet looks up `$type` in its loaded schema, rejects unknown candidates,
abstract instantiation, and non-derived candidates. Task 051 also measured its
non-OAM Clark-name rejection. No assumption that pinned Sleet can route arbitrary
private-namespace descendants is justified. Peer schema/namespace capability is
an external prerequisite; we do not patch Sleet in this task.

## 3. Current architecture and future modification seams

Generation-time knowledge and runtime receipt are different operations:

* Task 029 `load_schema_set_with_overlays` composes additive declarations,
  rejects duplicate QNames, preserves caller ordering, and uses private schemas
  **before** emission. Closed generation then includes private concrete variants.
* `GenerationWorld::ClosedSchemaSet` explicitly asserts the supplied universe
  exhaustive. `OpenExtensions` makes no such assertion, including zero-known
  targets. Abstract ancestry is not itself an abstract **value** demand.
* Task 032 `resolve_service_plan` and
  `project_service_generation_schema` bind the plan to IR, retain selected and
  generated-support closures separately, and expand support through
  `admit_abstract_value`. Open abstract-value demand fails there with
  `NotClosedUnderOpenExtensions` before any backend. The independent
  `abstract_value_projection_for_ref` enforces the same guard.
* `project_abstract_value`, `classify_abstract_value_topology`,
  `classify_abstract_value_semantics`, emission planning, coverage/readiness,
  generated-support binding, and backend name preflight would need a **new
  explicit open representation policy**, not reinterpretation of a closed sum.
* Each backend's `render_abstract_value` emits a closed descendant enumeration:
  Ada Kind-discriminated record; Rust enum with owned concrete cases and
  conditional equality derives; C++17 `std::variant`. No fallback exists.
  Existing recursion and single-namespace boundaries still apply.
* Task 051 preserves member namespaces independently of type namespaces.
  `backend-rust/src/service_codec.rs::type_name` and
  `Renderer::abstract_value` dispatch on the concrete full wire name;
  `check_members` rejects extras/conflicting known shapes. Known decode calls
  checked constructors and handles flattened inherited members.
* `runtime-rust/src/codec.rs::OmsJsonCodec<P>` currently accepts/returns
  `serde_json::Value`; provider is `Send + Sync + 'static`. Models do not derive
  Serde. `envelope.rs::unwrap_global_element` parses raw text into a Value before
  removing the one-member global QName envelope. Duplicate keys and lexemes are
  already unavailable to the payload codec.
* `SleetRuntime<C>` publish encodes the typed body then wraps its global QName.
  Subscribe unwraps then decodes before `FnMut(&P)` dispatch; decode failure
  becomes an event, never a typed callback. `PublishAdapter<P>` and
  `SubscribeAdapter<P,H>` in the dependency-free runtime-api crate keep generic
  typed arguments; concrete runtime thread bounds are not API-trait mandates.
  Task 071 route allow-lists authorize global message/topic pairs, not subtype
  validity. They cannot be used as subtype-validation attestations.

Future raw quarantine requires changes **before Value parsing**, in addition
to generated codec/model changes. The envelope, framing-size enforcement,
structured/raw codec seam, runtime decode-event delivery and opt-in service
wrappers must be reviewed together. Preserve existing traits for closed clients;
add separate versioned quarantine interfaces instead of silently changing `P`.
No transport connection, callback, retry or subscription lifecycle changes are
authorized by this ADR.

## 4. Alternatives

Scores describe possible future designs, not implemented capability.

| Criterion | A Exhaustive variants | B Known + opaque | C Dynamic registry only | D Hybrid known/registry/opaque |
|---|---|---|---|---|
| Runtime safety | Strong inside asserted closed universe; unknown rejected | Safe quarantine if trust states explicit; not typed validation | Safe only with trusted codecs/ancestry metadata; unregistered rejected | Same B boundary plus registry risk |
| Ada/SPARK | Existing definite discriminated records | Bounded private carrier feasible; parser/proofs needed | Dispatch/access and proof boundary costly | B feasible; C outside initial SPARK scope |
| Rust/traits | Existing enum/conditional derives | Owned bytes fit enum, Clone/Send/Sync; equality conditional | Erased traits complicate Clone/Eq/Send/Sync | Both costs; no automatic dyn equality |
| C++17 | Existing variant | Owned buffer variant/RAII feasible | Erasure/vtables and codec ABI | Both costs; ABI versioning needed |
| Ownership/lifetime | Value ownership | Deep copy or immutable shared ownership | Stable registry snapshot + owned result | Must specify both |
| Codec | Static dispatch, fail closed | Raw ingress retention + separately validated known decode | Trusted lookup/validation; no fallback | Raw fallback still required |
| Recursion | Current by-value gate | Opaque finite bytes; known recursion needs bounded indirection | Dynamic recursion still bounded | No automatic recursion solution |
| Memory bounds | Collections require policy | Byte/token/depth/queue budgets required | Registry + codec + payload budgets | Largest budget surface |
| Compatibility | Existing API | Opt-in new case is source-breaking for exhaustive matches | New runtime state/API | Versioned opt-in API |
| Cross-language parity | Models implemented | Uniform quarantine operations possible; codecs not yet present in Ada/C++ | Proof/ABI differences substantial | Explicit divergence required |
| Complexity/maintenance | Low | Moderate, duplicate-aware raw parser essential | High, trust and concurrent registry management | Highest |
| Unknown descendant | Cannot represent | Retain identity/payload without claiming validity | Only registered types; otherwise cannot represent | Can quarantine unregistered types |

**A** remains the production contract. It cannot carry a future descendant and
must not be relabelled Open. **B** is the smallest research choice. Store the
entire derived object, not just extension fields or an empty Unknown marker.
**C** is not inherently open: finite registration plus no fallback remains a
closed admitted runtime set. **D** is a possible later extension of B but has no
present justification for its additional public type-erasure/proof burden.

If C/D is later adopted: registration key is `(schema-profile identity,
concrete QName)`, with certified abstract ancestry and codec version; duplicate
registration is an error even when pointers match. Build a registry before
connecting, freeze it into immutable snapshots, and retain the snapshot for all
dependent values. No unload while a value/codec is live. Concurrent lookup is
read-only. Reject in-place late registration/replacement; a new snapshot affects
only new decoding and explicit revalidation. Unknown fallback obeys B. Neither
network metadata nor arbitrary plugin installation is trust evidence.

## 5. Candidate language-neutral identity and ownership contract

These are **proposed implementation policies** for research B, not statements
that CAL mandates an opaque API.

Conceptual states:

```text
Known<T> = validated generated descendant, full concrete QName
Opaque = expected abstract QName + asserted concrete QName
       + immutable owned UTF-8 derived-object bytes + schema-profile identity
       + syntax/resource-check provenance (never schema-valid provenance)
ValidatedExtension = reserved future state; requires reviewed validator contract
```

1. Identity is an exact `(namespace URI, local name)` pair. No prefix resolution,
   URI normalization, case folding, `EXT` suffix heuristic or local-name lookup.
   Preserve original `$type` spelling with the bytes, plus parsed full QName.
   OAM bare names decode only into the exact OAM URI. Qualified non-OAM names
   never alias bare names. Emit known values using current canonical wire rules.
2. Exactly one string discriminator is required at each demanded abstract value.
   Reject missing, empty, malformed, duplicate or non-string discriminators.
   Proposed strict grammar accepts valid XML local names and brace-delimited
   nonempty namespace identities without ambiguous braces. Do not invent a
   no-namespace wire encoding; absent namespace remains unsupported.
3. Reject duplicate keys throughout a quarantined object before map construction,
   including different spellings resolving to one member QName. Reject ambiguous
   QName identities/registrations. Two equal local names in distinct namespaces
   remain distinct. A syntactically good unknown QName is an **assertion**, not
   evidence that it derives from the expected base.
4. Carrier bytes include the complete object and its `$type`; never a delta,
   borrowed slice, partial member map, or payload-less marker. Copy before
   returning from transport processing. The expected abstract QName/profile are
   provenance supplied by the receiving context, not attacker-controlled proof.
5. Values remain valid after callback return if explicitly retained via owned
   copy/clone. Callback references themselves must not escape. Byte views borrow
   the carrier only, not transport buffers. Release frees storage once; no hidden
   runtime dependency. Mutation is unavailable; edits create a new unvalidated
   object and repeat syntax/budget checks.
6. Uniform semantics: retain/release, fallible deep copy, inspect identity/trust
   state, immutable byte view. Moves transfer ownership where the language offers
   moves; Ada assignment may copy. Shared immutable storage is optional only
   after lifetime/refcount/budget review. Registry lookup never changes old values.
7. Opaque equality is exact QName/profile/byte equality, **not UCI semantic
   equality**. Known equality remains conditional on descendant capabilities.
   No total Eq promise is added to sums containing floating/temporal values.

## 6. Candidate codec and preservation policy

Decode state machine:

1. Enforce framing/body quotas before allocation; tokenize UTF-8 JSON with
   duplicate detection, depth and token budgets before building maps.
2. Validate global envelope and selected message/topic as today. Locate each
   abstract position using its generated schema context. Preserve exact byte
   spans of unknown objects; their recursive syntax remains subject to budgets.
3. Parse `$type` identity. For known concrete QNames, require permitted ancestry
   and complete existing checked decode (including inherited members, occurrence,
   choice, scalar/facet restrictions). Any known-type validation failure rejects;
   **never downgrade a malformed known type into Opaque**.
4. For syntactically admissible unknown QNames, a separately enabled quarantine
   path may retain Opaque. Today's typed path continues to reject. Validate only
   surrounding structure that is actually known, without pretending to validate
   the unknown descendant's member set. Unknown inherited restrictions or
   derivation cannot be established by checking base fields alone.
5. Syntax/identity/resource errors yield deterministic categorized errors with
   paths and bounded diagnostics. Do not log whole private payloads. Allocation
   failure never produces a partial value or calls a handler.

Opaque re-encoding uses the original immutable object bytes, including all
unknown nested members, array ordering, strings, numeric lexemes and internal
whitespace. This conservative **policy** avoids choosing numerical normalization
semantics without the schema. Exact full OWP/message lexical replay is not
promised: outer envelope whitespace/order, topic and framing are runtime-owned.
Known encode remains semantic/current canonical behavior, not byte replay.
Serialization must not pass opaque bytes through `serde_json::Value` first.
This requires a reviewed raw-subtree seam; the current seam is inadequate.

Duplicate objects are rejected, not preserved or resolved last-wins. Object
member ordering and whitespace are stored only as a consequence of byte
ownership, not because CAL requires them. Large numbers must be scanned without
conversion/precision loss; validation/conversion is a separate operation.
No canonical signing/hash interpretation is established by this design.

## 7. Validation, security and application permissions

| Operation on Opaque | Candidate permission |
|---|---|
| Inspect | Identity, profile, trust state, byte length; explicit read-only diagnostic parsing under the same budgets. Never typed field access or authorization decisions from unvalidated fields. |
| Retain | Yes, owned immutable quarantine with account/queue/lifetime budgets. |
| Convert to known | Only explicit revalidation against a trusted matching schema/profile/ancestry and full checked decode; no cast, base-field extraction or automatic registry upgrade. |
| Subscribe | A distinct opt-in diagnostic/quarantine callback may receive unvalidated input; ordinary generated typed subscription may not. Network subscription permission does not validate content. |
| Forward | No default permission. A future separately authorized relay must establish message/schema validity and route/security policy or obtain an explicitly reviewed external-validation attestation; raw retention alone is insufficient. |
| Publish | Not through current generated service API. Future publication requires full validation plus authorization and peer extension capability. |
| Generated service argument | Not valid as today's typed argument. A future open envelope must explicitly distinguish unvalidated state; accepting its shape must not imply publishing it is legal. |

Inherited base members can have additional derived restrictions and participate
in the complete derived particle structure. Checking a known base projection is
useful diagnostics but not full validation. A registered handwritten codec must
prove trusted schema derivation and all effective constraints, not merely parse
some JSON. Even a trusted transport peer does not supply that proof implicitly.

The smallest outstanding normative/program decisions are:

1. Which associated private-schema authority/profile is trusted for validation,
   and what evidence proves QName ancestry and complete effective constraints?
2. Is a schema-unvalidated quarantine/relay path permissible at the adopting CAL
   application boundary, and, if relay is wanted, what validator/attestation and
   security policy authorize it? No unconditional relay exemption is assumed.
3. What pinned peer/schema configuration accepts private QNames and which
   language-neutral raw codec boundary can preserve them without bypassing
   validation? Current Sleet/Value behavior is not that evidence.

Until these are answered, the full runtime contract remains unestablished.
Ownership/grammar/bounds policies do not need new standards mandates but do need
explicit implementation review and cross-language proof/tests.

## 8. Backend feasibility and explicit divergence

### Ada/SPARK

For research B use a **private definite bounded carrier** whose storage capacity
comes from an explicit configuration/generic bound, with lengths for namespace,
local name and UTF-8 bytes, plus validity/trust discriminants. This can be stored
in a known/opaque discriminated record and bounded repeated collection. A
zero-known target has only the opaque arm; it is not a null record. All lengths
and index arithmetic require proofs; unused storage is not exposed. Capacity
affects generated layout and must be recorded in the profile.

An unconstrained array/discriminant produces indefinite types and cannot simply
replace every definite field/collection element. Controlled heap ownership is
possible in Ada outside SPARK but requires Adjust/Finalize, failure handling,
aliasing and pool rules; it is not automatic proof-compatible ownership. Access
types for recursive known values require a reviewed bounded storage pool or
arena with stable handles, ownership and acyclic graph invariants. Do not use
unchecked access/conversion or unbounded default allocation. A SPARK-facing
carrier can prove capacity and lifetime invariants while a parser is outside the
proof boundary; parser validation claims remain explicit assumptions until
proved/tested. Ada codec/runtime are not yet implemented, so no end-to-end SPARK
certification is claimed. Reject research configurations without bounded storage.

### Rust (1.95 MSRV for runtime/test slice)

An opt-in `KnownCase(T) | Opaque(OwnedOpaque)` enum can own private `Vec<u8>`
storage and separate QName strings without exposing Serde types. `Send`/`Sync`
are feasible with immutable owned storage; callbacks/providers retain existing
thread constraints. `Clone` can deep-copy, but allocation may abort under normal
Rust allocation policy: add a fallible `try_clone` path for bounded admission,
and do not promise recoverability from every OOM. `try_reserve`/checked arithmetic
are required for explicit resource checks; an allocator-level abort remains a
documented platform divergence. Eq/PartialEq stay conditional on all known
descendants; byte equality on Opaque does not fix temporal/float semantics.
Recursive known values need `Box`/arena handles plus depth/node budgets, not
infinite enum layouts. Dynamic `dyn` codecs do not automatically implement Clone
or Eq. New variants break exhaustive matches, so closed generated APIs stay
unchanged and open output must be explicitly versioned/opt-in. Public models
must not acquire `serde_json::Value` merely because the transport uses Serde.

### C++17

Use opt-in `std::variant<Known..., OwnedOpaque>` (opaque-only if no known cases),
private RAII-owned byte/QName buffers, e.g. `std::vector<std::uint8_t>`. Deep copy
owns independent storage; move transfers it. Private constructors enforce
admission. Allocate/validate into a temporary before committing to preserve
strong exception safety; handle `std::bad_alloc`, `length_error` and
`valueless_by_exception` without publishing partial data. Nonthrowing moves
where feasible simplify invariant maintenance. No borrowed transport pointers.
Recursive known cases need owned indirection or bounded arena handles with cycle
rejection. C++17 supplies no stable cross-compiler variant/vector ABI: version
generated layouts and registry codec interfaces; do not promise ABI stability
when adding cases. Existing C++ codec/runtime absence is still a blocker.

**Uniform:** full identity, immutable owned bytes, trust-state distinction,
duplicate rejection, quotas, explicit validation, no implicit publish/forward.
**Explicit divergence:** Ada configured inline capacity versus Rust/C++ heap
buffers; Ada assignment versus moves; SPARK proof boundary; Rust allocator abort
versus C++ exceptions/Ada storage errors; no universal semantic equality or ABI.
These divergences require acceptance tests, not silent backend feature enablement.

## 9. Resource limits and recursion

No arbitrary production numeric limit is selected. A future profile must define
finite budgets for: raw frame/message/object bytes, QName bytes, maximum JSON
nesting, total tokens/nodes, string/numeric-token lengths, object members, array
elements, repeated abstract values, aggregate decoded storage, queued retained
values/bytes and encoding output. Per-object limits alone do not bound aggregate
repetition. Transport limits provide an upper bound but do not replace model and
queue budgets. Missing required budget configuration fails before connecting.

Check lengths and multiplication/addition before reserve/copy; avoid truncation
between transport sizes and backend index types. Enforce limits before deep parse
or allocation; count raw nesting even for unknown objects. Encode also consumes
budget and must not send a partial PUB. Return SizeLimit, DepthLimit,
CollectionLimit, InvalidIdentity, InvalidSyntax, ValidationUnavailable or
AllocationFailure categories as appropriate; no retry loop on bad input.

Opaque bytes are a finite serialized tree and need no model-level recursive
access. Known recursive models are a separate follow-up: bounded ownership tree
or arena with no pointer cycles, depth/node accounting and iterative bounded
destruction/encoding. A finite JSON input does not make an infinitely sized
by-value Ada record/Rust enum/C++ variant legal. Current recursive generation
gate remains. Budget tests must cover exact boundary and boundary+1 after limits
are chosen; parser defaults are not the contract.

## 10. Executable evidence

No opaque prototype is added: raw-byte preservation at the current Value seam
would be a misleading partial runtime implementation. New evidence uses actual
production frontend/IR/projection/backends and generated codecs, in test-only
files. Existing Task 068 inventories/private fixtures remain unchanged.

| Case | Deterministic evidence and expected outcome |
|---|---|
| Known descendant | `task072_runtime_contract`: public fixture yields exactly PublicA; Closed projection succeeds; Open exact Base guard. |
| Generation-time private | Same test loads Task 068 private overlay; exactly PublicA, PrivateB; Closed succeeds; Open guard unchanged. Task 029 backend controls and Task 068 private codec probe supply existing compiler/round-trip evidence. |
| Future unknown | `task072_wire_boundary`: `{urn:future}FutureShape` rejected with exact generated-codec error; no typed value/re-encode. |
| Zero known | Existing zero XSD: exact NoConcreteDescendants for closed sum query; Open exact ExtensionBase guard. |
| Equal local names | Valid synthetic IR adds `{urn:other}PublicA` with same base; projection retains both full QNames; all three backends reject multi-namespace generation; Open guard unchanged. This is an IR probe, not a newly supported XSD/backend namespace feature. |
| Recursive abstract | Synthetic IR adds Base-valued field to PublicA; exact cycle Base → PublicA → Base; Closed service projection rejects; Open guard rejects. |
| Missing/malformed/conflicting identity | Generated Shape codec corpus: missing/string mismatch, empty/malformed/foreign/bare identities and conflicting CircleShape fields reject. Duplicate raw discriminator probe demonstrates last-key-wins **before** codec: one order accepts known, reversed order rejects future. This exposes a prerequisite, not a duplicate-safety claim. |
| Large/deep unknown | 4096-byte string and 32-level structured stimulus with future identity reject exactly, not retained. 256-level raw array hits Serde recursion error. These stimuli are not production limits and do not prove pre-allocation bounds. |
| Real UCI | Opt-in ignored-by-default test loads one hash-verified pinned root and projects only DataRecordListManagementRequest. Closed succeeds, Open exact ManagedListBaseType failure. Run for both releases; no schema-wide campaign. |

The real vertical has 43 selected + 4 support declarations in Task 068's frozen
inventory, one acyclic known concrete descendant (`ForeignKeyMapML`), and optional
AddRecord/DeleteRecord positions. Root schema inspection confirms these fields
and inheritance. Model readiness in Ada/Rust/C++ is not Ada/C++ codec readiness.
The first implementation experiment should add a same-namespace private managed
list in a scratch overlay, generate Closed controls, then compare a future
unknown managed-list object at an explicitly quarantined raw boundary. Test a
separate non-OAM descendant as a peer-compatibility rejection control.

## 11. Implementation prerequisites and bounded follow-ups

1. **Resolve validation/authorization gate:** pin adopting private-schema and
   CAL/security rules; decide whether quarantine is legal, and separately whether
   externally validated forwarding is legal. Define attestation scope/revocation
   if used. No guard change before review of these answers.
2. **Raw ingress research only:** duplicate-aware UTF-8 scanner, exact object-span
   ownership and configurable quotas; prove it retains numbers and members before
   Value normalization. Deterministic grammar, duplicate, boundary+1 and allocation
   fault tests. No service readiness change.
3. **Three-backend bounded carrier research:** Ada/SPARK definite storage and
   proof boundary; Rust 1.95 owned carrier/fallible admission; C++17 RAII and
   exception-safety probes. Prove callback-independent lifetime/copy/release and
   reviewed parity/divergence. Keep registries/known recursion out of this slice.
4. **First real vertical:** DataRecordListManagementRequest / ManagedListBaseType,
   known ForeignKeyMapML, generation-time private and future-unknown controls;
   full inherited-validation negative corpus and peer-capability evidence against
   unmodified pinned transport. No claim that unknown publication succeeds.
5. **Explicit service/API integration review:** versioned opt-in quarantine APIs,
   error/event semantics and policy provenance; only then reconsider projection,
   coverage, support binding and guard admission in one coordinated change.
6. **Later independent work:** dynamic registration and recursive known-model
   ownership only if needed, each with bounded lifetime/concurrency/proof tests.

Do not begin Task 073 from this PR. Approval of documentation is not authorization
to implement any of these slices or to remove the guard.

## 12. Validation and review gate

Local Rust 1.95 validation passed: `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets`, and
`cargo clippy --workspace --all-targets -- -D warnings`. The focused suite passed
225 tests with zero failures (the pinned opt-in test is ignored in the ordinary
synthetic run). This includes Task 068 generated-codec regression (1), new
Task 072 synthetic/model probes (3) and wire-boundary probes (3), Task 051 QName
codec (8) and projection (4), service codec CLI (11), service generation CLI (68),
core service generation (16), readiness (30), generated-support binding (12),
abstract-value core controls, Task 028 (6/backend), Task 029 (2/backend),
generated codec (7), and Rust runtime (16). Task 068 fixture accounting and all
six frozen readiness message sets passed. `git diff --check` passed.

The Task 068 private codec control was regenerated via the production CLI in
Task 072 scratch storage and executed successfully: PrivateB semantic round trip,
PrivateC unknown-subtype rejection, mismatched PublicA members and unknown Secret
member rejection. The standalone research executable reports generated unused
code warnings; workspace clippy remains warning-clean. No scratch executable or
generated model is added to production crates.

Multi-hour full pinned UCI/Sleet campaigns are intentionally excluded: these
changes add an ADR and focused research regressions, not runtime features.
Hosted CI certification is a separate exact-head publication gate, not implied
by the architecture status. Final pushed head and hosted results belong in the
PR/final report, not a self-referential immutable-source claim.

Reproduction from `/home/zboll/git/ams-gra-codegen-oms-task072` (Rust 1.95):

```sh
cargo +1.95.0 test -p ams-gra-codegen-oms --test task072_runtime_contract
cargo +1.95.0 test -p ams-gra-oms-runtime-rust-facade-tests --test task072_wire_boundary
# Repeat for each pinned release root; the opt-in test checks its SHA-256 itself.
AMS_GRA_TASK072_UCI_ROOT=/tmp/task068/uci25/03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd \
  cargo +1.95.0 test --release -p ams-gra-codegen-oms --test task072_runtime_contract \
  pinned_data_record_list_management_request -- --ignored --exact
AMS_GRA_TASK072_UCI_ROOT=/tmp/task068/uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd \
  cargo +1.95.0 test --release -p ams-gra-codegen-oms --test task072_runtime_contract \
  pinned_data_record_list_management_request -- --ignored --exact
```

Source roots are external, not vendored. Fresh roots can be obtained with the
existing pinned fetch scripts into a task-specific scratch directory; substitute
their printed absolute paths above. No sibling worktree artifacts are modified.
The debug pinned probe was stopped after several minutes of CPU-bound schema
processing without a result and rerun in release mode; no debug pinned pass is
claimed. An earlier duplicate scratch invocation was also stopped; only final
test-source release outcomes certify the real vertical.

Both final-source pinned release probes passed (UCI 2.5 and 2.6, one test each,
about six seconds each after compilation): exact 43 selected + 4 support,
ForeignKeyMapML identity, successful Ada/Rust/C++ model rendering, and exact
Open ManagedListBaseType guard. These are projection/render tests, not new
three-language opaque-carrier compiler tests or Sleet interoperability claims.