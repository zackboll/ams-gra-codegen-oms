# PY-007 — Python pure-Record inheritance and abstract ancestry

## Source, authority and scope

Starting fetched main: `77066e8e2cf4875274c0227cee16dcf0dc593c05`, tree
`e088a7c68797c8da65ea2d1a92f85a963c22b949`. PR #114 was confirmed MERGED
and its Python source present before creating the clean isolated branch
`feature/py-007-record-inheritance` at
`/home/zboll/git/ams-gra-codegen-oms-py-007`. Other worktrees, including
PY-006 and concurrent Ada/CI work, were not modified. Final commit/tree,
incorporated base, remote/PR equality and exact-head hosted results belong in
the PR handoff; this document does not claim checks for an unidentified future
commit. Historical PY-001–006 evidence remains historical.

Rust/Cargo **1.95.0**; Python **3.11.17** and **3.13.16**; GNAT/GCC
**14.2.0**; pinned `websockets==17.2`. No dependency changes. Cargo.lock
SHA-256 remains
`45430d24281bdd7d22f41ea4d93d26fb0bd7bfe8746c223ccf9e56f5240a7f58`.

The official authority is the pinned CAL DOCX from OMS standard revision
`726272bd0390982a759c91a9cf4e13b81c2b510b`,
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`,
SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
The cached original was independently hash-verified and its `word/document.xml`
read, not inferred from dataclass representation. §§6.1.2–6.1.3 prescribe
particle/member QName mapping and flat complex-type objects. `$type` is used
when a type occurs where another type was expected. That substitution is
outside PY-007: every value here has its exact concrete declared type.

## One authoritative structural contract

`codegen_core::effective_record_fields(schema, target)` supplies borrowed
original field declarations in base-to-derived order. Python's small
`record_fields` adapter only translates its error into `CodegenError`; it
does not traverse inheritance, modify SchemaIr or flatten Choice content.
Naming, model preflight/rendering, codec wire preflight, encoding and decoding
all consume that same projection. The shared code handles ancestry depth,
empty extensions, missing/nonstructural bases, cycles and authored-name
collisions. Nonempty Choice segments cannot become Records.

Each concrete Record is one `@dataclass(frozen=True, slots=True, kw_only=True)`.
There is no Python class inheritance. A four-level `Leaf` remains `Leaf`,
with its own `__schema_name__`, not `Base` or `Third`. Optional members retain
`None`; bounded repeated members use immutable checked tuples, including
positive minima. Scalar checks still require exact types (`bool` is not an
integer), with unchanged domains/facets. There are no duplicate declarations
or duplicate per-class validation loops for inherited members.

The shared planner is still authoritative for validation and ordering.
Python's `emitted_declarations` handles its historical no-abstract-target
shortcut: ancestry-only abstract **Records** may appear as `Declaration`,
but have no generated class or codec. Naming reserves only the declarations
actually emitted, and examines every effective member of concrete Records.
Inherited/local normalized collisions and annotation hiding fail rather than
being suffixed. Existing leading-underscore/keyword prefixing remains the
PY-001 naming policy; tests execute prefixed dunder/sunder members safely.

`abstract_value_targets` examines genuine value positions (Record fields,
Choice alternatives, aliases, Lists and messages), never base edges. Python
rejects any demanded abstract structural target, including one that the
closed-world planner would elide as Task 026 absent-only. An `AbstractValue`
emission is also explicitly rejected. No concrete descendant is an implicit
default; there is no Union, Any, object carrier, dictionary or abstract
dataclass. OpenExtensions never silently closes an extensible value set.
Abstract Choice lowering and nonstructural abstract declarations remain
unsupported. Both explicit worlds generate identical bytes where no abstract
value target is demanded.

## Generated API and wire example

The ordinary XSD frontend and service-contract resolver produce `model.py`,
`service_api.py` and, only on request, `service_codec.py`. Model and API bytes
are identical with and without `--with-codec`. In the task-owned synthetic
OAM service:

```python
value = model.Leaf(
    base_required=7, base_optional="ancestor",
    base_values=(model.Token(2), model.Token(9)), required_values=(3, 4),
    linked=model.Token(5), local="leaf",
    nested=model.Nested(count=8, label="nested"),
)
text = service_codec.encode_Leaf(value)
decoded = service_codec.decode_Leaf(text)  # exact model.Leaf
```

Independent expected wire literal, in effective schema order:

```json
{"Base_Required":7,"Base_Optional":"ancestor","Base_Values":[2,9],"Required_Values":[3,4],"Linked":5,"Local":"leaf","Nested":{"Count":8,"Label":"nested"}}
```

`Base_Optional` and empty zero-minimum `Base_Values` are omitted when absent.
No nested Base/Derived object and no `$type` are emitted. A normalized IR
execution test independently changes an inherited authored member name to
`Authored-Required`, verifies that exact JSON key rather than its normalized
Python identifier, and checks recorded namespace metadata. The separate
shared-fixture service executes qualified non-OAM inherited keys. Python's
existing single-namespace restriction remains unchanged; cross-namespace
ancestry is not qualified here.

Outbound encoding recursively revalidates inherited/local fields and exact
named types, including deliberately corrupted frozen objects. Decoding checks
required inherited members, null, unknown keys, malformed JSON, cardinality
and indexed later elements. JSON escaped-key aliases count as duplicates.
The concrete constructor is not invoked after an invalid later argument.

## Synthetic execution

`tests/fixtures/python-inheritance/oam.xsd` covers concrete-to-concrete
inheritance, an abstract base with concrete descendants, four levels with an
empty intermediate, required/optional inherited members, bounded repetition
with zero and positive minima, a nested concrete derived Record, two concrete
descendants sharing an abstract ancestor and explicit concrete message
payloads. The original shared four-level fixture and base-only abstract
ancestry fixture also execute as positive model tests. Existing inherited
abstract-value-reference and zero-descendant optional negatives are retained.

The dedicated CLI suite executes **12 unittest methods × 2 worlds × 2
optimization policies × 2 import styles = 96 per interpreter**, on each of
Python 3.11 and 3.13. Probes compile actual generated modules, resolve hints,
check frozen/slots/keyword-only metadata, field order, exact identity,
independent JSON literals and constructor/codec failure paths. Repeated runs,
worlds and codec/no-codec model/API comparisons require byte equality.

The atomic negative CLI test has **14 schema cases × 2 worlds × 2 codec
policies = 56 selections**, with separate check and generation calls (**112
processes**) per execution. Cases: abstract field, abstract payload, abstract
Choice alternative, Choice ancestry, missing base, cycle, authored-name and
Python-name collisions, builtin/generated annotation hiding, unsupported
primitive/facet, unbounded and nillable inherited fields. No failed generation
may leave files. Runtime negatives additionally cover inherited type errors,
missing required JSON, later repeated items, corrupted outbound members,
unknown keys, duplicate aliases, `$type` and later nested failures.

The current-service gate has **five** offline adversarial control tests:
exact measured campaign, incomplete/duplicate selections, identity/process
tampering, edited-oracle READY/ambiguous results and historical fixture hashes.
These do not substitute for the actual external campaign.

Focused final-source executions on each target interpreter passed **16 Rust
backend tests** and **21 Rust CLI/service/codec/header/inheritance tests**, zero
failures or ignored tests. Their captured subprocess output accounts for
**374 Python unittest method executions per interpreter** (22 backend and 352
CLI-suite methods; 187 normal and 187 optimized). This includes the 96-method
dedicated inheritance matrix, not an additional 96 on top. Two-world probes
that use direct TestCase assertions without a unittest runner are not counted
as unittest methods. The pinned checker separately executes eight isolated
profiles × four literal tests (**32**) plus the **96** inheritance methods,
on each interpreter. The mock runtime separately passed **19 × two policies
= 38 tests per interpreter**.

## Pinned Sleet integration

The synthetic OAM service executes XSD → generated model/API/codec → existing
LacalRuntime → unmodified Sleet → generated decoder → typed handler.
Sleet revision remains `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`.
`scripts/run-real-sleet-test.sh` reuses its one checkout/build and invokes
`scripts/check-py007-sleet.sh` alongside the unchanged PY-005/PY-006 tests.
No transport implementation is changed.

Python 3.11 and 3.13, normal and `-O`, each executed one asynchronous integration
test successfully. Each subscribes, publishes, verifies every inherited/local
member and nested concrete type, injects an invalid later inherited Token,
observes broker rejection and decoder rejection, receives a subsequent valid
publication, rejects corrupt outbound data, receives valid data again,
unsubscribes and closes deterministically without runtime task leaks.
Required marker: **`PY007 PINNED SLEET INHERITED RECORD: PASSED`**.
This is synthetic broker qualification, not a real-UCI READY claim. Complete
shared-campaign results and existing markers are recorded in the PR handoff.

## Actual pinned UCI BEFORE/AFTER

No schemas are committed or downloaded inside the repository. Existing
external roots were reused and hash-verified:

| Release | Revision | Root SHA-256 |
| --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

`before.json` and `after.json` in the task-owned fixture directory preserve
all **24** measurements (2 releases × 3 real global messages × 2 explicit
worlds × codec/no-codec), process exits, complete stdout/stderr, selected
closure counts, source/schema identities, contract hashes and report hashes.
The contract text and message identities are unchanged from PY-006. Baseline
source output and a preserved starting-main executable were frozen externally
before production edits. `controlled-before.json` retains all four expected
failures for the shared pure-Record fixture selected through an ordinary
service contract.

`selected-closures.json` additionally preserves six BEFORE/AFTER resolved
service-plan reports and requires their byte identity. `controlled-after.json`
records all four successful shared-fixture selections and artifact hashes;
their codecs execute independent qualified-name JSON literals on both Python
versions and optimization policies (eight method executions).

BEFORE first blocker for every real-service selection:
`unsupported Python IR construct: abstract declarations are unsupported`.

AFTER (each row represents both worlds, with and without codec):

| Release | Message | Actual first blocker |
| --- | --- | --- |
| 2.5 | PositionReport | `unsupported Python IR construct: primitive Float64` |
| 2.5 | RDMA_InitializeSetup | `unsupported Python IR construct: primitive DateTime` |
| 2.5 | SubsystemStream | `unsupported Python IR construct: primitive DateTime` |
| 2.6 | PositionReport | `unsupported Python IR construct: primitive Float64` |
| 2.6 | RDMA_InitializeSetup | `unsupported Python IR construct: unsupported constraints` |
| 2.6 | SubsystemStream | `unsupported Python IR construct: primitive Binary` |

All process exits are 1; model/API remain NOT READY. Requested codecs remain
**not measured**, not READY or independently NOT READY. No real service became
READY; no real-UCI Python execution or wire-vector qualification is claimed.
The 24 report hashes change because the first diagnostics change; semantic
closures and source/contract identities remain unchanged. The generic 2.6
RDMA constraint diagnostic is recorded as observed, not guessed into a more
specific supported profile.

## PY-006 CI compatibility and regression

The historical `python-header-profiles/before.json` and `after.json` are
unchanged, and their exact hashes are checked before the current campaign.
The PY-006 checker defaults to its original historical comparison. Its only
new comparison option supplies a versioned service baseline; the eight
isolated selections still execute actual pinned declarations, verify exact
generated artifact hashes and run independent literal tests in normal/`-O`
and standalone/package modes. PY-007 selects its measured current baseline
through a narrowly factored wrapper, never rewriting historical service
results to resemble new diagnostics.

The wrapper requires both root pins, all real global messages, unique and
complete 24-selection results, exact process/stdout/stderr and contract
identities, explicit per-stage verdicts/first blockers and selected closures.
Unexpected READY transitions fail even if a text oracle is edited: a future
gain requires generated execution and independent vectors before extending
this gate. It additionally executes the inherited fixture's 96 Python tests.
Missing, truncated, duplicate or ambiguous results fail closed.

Fast CI extends the existing Python 3.11/3.13 matrix; Deep extends the existing
root-qualified job and one Sleet build. No duplicate heavyweight job is added.
The CI split guard and adversarial mutations cover the new mandatory gates
without weakening existing source-identity, permissions or failure checks.

Backend/model/API/report parity against the preserved starting main passes:
six Ada/Rust/C++ full models, twelve legacy selected model/API generations,
twelve readiness reports, two coverage reports, eight non-inheritance Python
model/API generations and reports. Canonical digest remains
`b0749e258b6484b568c07b60f81b855353565488a5b26fc3ed224b160cfab235`.
The PY-001 golden model remains byte-identical. The old PY-004 inheritance
negative now specifically demands a nonempty Choice base; pure Record
inheritance is executed positively rather than still expected to fail.

Reproduction and final qualification commands:

```bash
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --workspace --all-targets --locked
cargo +1.95.0 clippy --workspace --all-targets --locked -- -D warnings
for interpreter in python3.11 python3.13; do
  PYTHON="$interpreter" cargo +1.95.0 test --locked -p ams-gra-oms-backend-python -- --nocapture
  PYTHON="$interpreter" cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
    --test python_cli --test python_service --test python_codec \
    --test python_header_profiles --test python_inheritance -- --nocapture
done
AMS_GRA_REQUIRE_GNAT=1 cargo +1.95.0 test --workspace --locked
python3 scripts/test-py007-uci.py
python3 scripts/check-py007-uci.py --generator target/release/ams-gra-codegen-oms \
  --output /external/py007-current
bash scripts/check-ci-split.sh
bash scripts/test-check-ci-split.sh
git diff --check
```

Exact final-source workspace passed/failed/ignored/executed totals, focused
suite totals and hosted check identities are recorded in delivery evidence,
not inferred from successful individual probes. Until all mandatory final-head
jobs pass the disposition is **HOSTED_CI_PENDING**; unresolved correctness
failures require **CORRECTIVE_WORK_REQUIRED**. Independent review and merge
remain the owner's decision.

## Remaining boundaries and possible PY-008

No abstract-value polymorphism/closed sums, Choice lowering, Python class
inheritance, `$type` substitution, Task 026 absent-only lowering, general
named restriction ancestry, new primitive families/facets, arbitrary patterns,
unbounded/nillable values or transport behavior are introduced. The real-UCI
first blockers above remain genuine. A narrowly scoped PY-008 could qualify
one unsupported primitive family and its exact wire semantics, with new
independent vectors; broad polymorphic-value support requires a separate
world-aware design. Neither is implemented merely to force a UCI READY gain.