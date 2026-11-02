# PY-015 — checked Python Choice models and OMS JSON codecs

## Scope and API

Python now admits concrete named normalized-IR Choices through the shared
`effective_choice_alternatives` projection. There are no shared IR, frontend,
classifier, dependency or other-language implementation changes in this task.
The shared emission plan orders dependencies; the Python preflight uses its
existing available-name set, not a recursively simulated readiness engine.

Each Choice is a frozen, slotted dataclass with a stable `__schema_name__`, a
required string `alternative`, and exactly one `value`. Case-preserving Python
factories explicitly select alternatives:

```python
owner = OwnerProducerChoiceType.GovernmentIdentifier(OwnerProducerEnum.USA)
special = OwnerProducerChoiceType.NATO_SpecialWord(
    NATO_SpecialWordsType("NATO:EXAMPLE")
)
```

The checked direct constructor accepts the emitted factory identifier as tag.
Unknown tags, wrong exact types, invalid domains and missing arguments fail.
Same-type alternatives remain distinct. Factories do not select by payload
type and return the exact generated class. There is no implicit selection.
Python keywords and unsafe identifiers use the established deterministic
escaping policy; collisions with storage, annotations, helpers and schema
types are rejected. Dunder names are escaped before they can shadow dataclass
methods. Validation uses no assertions and executes under `python -O`.

Only Choice-containing schemas acquire an explicit generated exact-class
validator table for revalidating nested carriers. Choice-free source bytes
remain unchanged. OMS codecs use explicit per-type mappings and exact
`FieldDecl::wire_name()` provenance, not dataclass introspection. A Choice
encodes as one selected member, never a host discriminator. Outbound payloads
are recursively revalidated by existing type-specific codecs.

Required and optional Record fields and bounded/unbounded repeated fields
reuse the existing tuple/count/omission code. The array counts occurrences of
whole single-member Choice objects. Inheritance and nested Choices use the
authoritative structural projection and dependency plan.

Unsupported: optional/repeated/nillable alternatives, Lists, abstract values,
recursive/unresolved dependencies, unsupported facets/aliases, mixed or
multiple structural Choice segments, and unqualified codec member identities.
No new XSD compositor or cardinality semantics are inferred.

## Authentic measurements

Initial independently verified main:

* commit `4f395938a700afd3a72f2b233c501031fe08ad1f`
* tree `96f0cda8c382f95cbf8246b36f8145a359470ee6`

The 40 BEFORE and 40 AFTER selections are retained in the versioned
`tests/fixtures/python-choice/evidence/v1` campaign. The AFTER production source
is commit `4ff5237344c274dd5f977e630c1a64222eb52042`, tree
`ba2fdd4c01ab9e9e711295dc23b573526d07312d`. Its recorded production inventory is
authenticated by an independently hard-coded digest, not future live files.
Every row records the original contract hash, closure and digest, pinned
root/include hashes, CLI stdout/stderr/exit, attempted generation, artifact
hashes and actual fresh/pre-existing output rejection. No artifacts were
generated for the blocked complete services.

Both pinned releases, both worlds, and both codec-selection policies have the
following measured first model/API boundary:

| Service | BEFORE | AFTER |
|---|---|---|
| Log | FGI_SourceOpenChoiceType | WhitespaceVisibleString1024Type constraints |
| PositionReport | FGI_SourceOpenChoiceType | WhitespaceVisibleString1024Type constraints |
| SubsystemStream | FGI_SourceOpenChoiceType | WhitespaceVisibleString1024Type constraints |
| SurvivabilityRiskLevel | FGI_SourceOpenChoiceType | WhitespaceVisibleString1024Type constraints |
| RDMA_InitializeSetup | IPv4_AddressType constraints | unchanged |

All 40 complete selections remain model/API NOT READY, exit 1. Requested
codecs are **not measured**, not declared READY or codec-unsupported. No
complete authentic Log or PositionReport transport claim is made. Their
inherited security/header closure cannot be completely generated without
implementing further capabilities outside this task.

Current-service baseline wiring alone points at the new 24-row legacy-service
projection. Historical PY-006–PY-014 evidence and inventories remain intact.

Concurrent main was independently fetched and verified before an ordinary
merge: commit `6166906bcd72e44f5df23d8470d6aaa7fefa32b5`, tree
`b1a407aa1fad8eb0a0fd7a984e3a330a7f226695`. Its C++ changes are inherited main
work, not PY-015 modifications to a peer backend or shared classifier.

## Executable qualification

The CLI target `python_choice` exercises both worlds, standalone/package
imports, model-only/codec generation and normal/optimized Python. Independent
literal vectors cover enums, NATO words, direct strings/integers/Boolean/
Float64, Binary, DateTime, Duration, concrete Records and nested Choices.
Controls exercise same-type alternatives, shared dependencies, inherited
repetition, both alternative orders, duplicates, exact types, corrupted
storage, wrong tags/payloads, first/middle/last malformed repeated elements,
Clark-qualified paths, strict Unicode/numbers and duplicate decoded keys.
Negative mutations verify atomic fresh/pre-existing output failures.

The backend Choice test also exercises normalized-IR ancestry, unqualified
wire identities, unsupported Binary lexical provenance and field facets.

The UCI discovery probe executes the four original security Choice
declarations from both hash-verified root/include sets. It tests every enum
and NATO alternative and repeated security content in an explicitly synthetic
surrounding Record. It reuses OwnerProducer content from existing independent
authentic Ada Log and pinned-Sleet PositionReport JSON documents. This is
isolated declaration qualification, not complete inherited-header or complete
service qualification.

Synthetic pinned-Sleet qualification uses unchanged revision
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`, and `websockets==17.2`.
For each world, 14 alternatives times three optional-field policies produce
42 typed publish/subscribe deliveries. Each malformed input is followed by a
valid recovery. Instrumentation observes 17 malformed cases per world:

* 13 broker rejections, without generated decoder invocation;
* 3 generated decoder rejections (invalid Duration lexicals);
* 1 runtime-envelope rejection (duplicate-member JSON), before the decoder.

Unsubscribe, disconnected state, broker shutdown and no-leaked-task checks are
mandatory. The test does not relabel envelope or broker rejection as generated
decoder coverage. Direct codec tests separately reject duplicate Choice keys.

New tests join existing Fast/UCI/Sleet source discovery; there is no new runner
lane, selector, dependency or lockfile change. Source-bound receipts reject
dirty source or changing generator hashes. Failed initial runs are retained
outside the repository rather than represented as passing results.

The independent Choice-free parity campaign covers 48 selections and 120
Python artifacts, including PY-014 Duration and PY-013 Binary. All four
interpreter/policy campaign executions passed before the concurrent-main
merge. An additional peer campaign compared 12 selections and 36 Ada/Rust/C++
artifacts byte-for-byte. Post-merge validation is recorded separately so the
concurrent C++ capability change is not attributed to this Python task.

Local build and execution logs, exact source receipts and failed-attempt logs
are under `/home/zboll/py015-evidence`; build outputs are isolated under
`/home/zboll/py015-build`. The final PR handoff reports exact-head local and
hosted outcomes, rather than treating this document as a passing receipt.