# PY-008 — checked Python Float64 models and lossless OMS JSON codecs

## Dependency, authority and ownership

Starting freshly fetched main is `4978c1c5728d062b9ea64bbcdb39aa96b18361a3`,
tree `63a37eaeafc38bd7f6ddfadf4dfc99919393000a`. PR #117 is **OPEN**, not
merged, at verified PY-007 head `d76a36d2cb9d18bcc8b825b5134a65686f361deb`,
tree `bb28f2e70e0c2b1c75ed19ff390f8c1df40773c3`. That head already contains
the fetched main. This is an isolated stacked branch,
`feature/py-008-python-float64`, based on that exact dependency and targeting
`feature/py-007-record-inheritance`. No parent branch edits, merge, rebase or
force-push are authorized. It must remain unmerged until independent PY-007
approval/merge and ordinary integration plus requalification against main.
PR #116 remains open at preflight; its unmerged CI changes are not imported.

Worktree: `/home/zboll/git/ams-gra-codegen-oms-py-008`; task-owned build and
raw evidence: `/home/zboll/py008-build` and `/home/zboll/py008-evidence`.
Concurrent Ada/CI work and shared targets are untouched. The source/dependency
identities, final commit/tree, exact changed-file manifest, remote/PR equality
and exact-final-head hosted checks belong in the delivery handoff. Local
success is not a hosted pass. **HOSTED_CI_PENDING** until mandatory exact-head
Fast and Deep pass; unresolved failures require **CORRECTIVE_WORK_REQUIRED**.

The normative authority was downloaded directly from `open-arsenal/oms` at
`726272bd0390982a759c91a9cf4e13b81c2b510b`,
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`.
Its verified SHA-256 is
`b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
The official `word/document.xml` was read, not only another backend's output.
OMSC-SPC-013 Rev B §6.1.4 maps `xs:double` to JSON **number** or exactly
`"NaN"`, `"Infinity"`, `"-Infinity"`. Finite quoted numeric text is not a
Float64 value. §§6.1.2–6.1.3 retain flat inherited members and their QNames.

## Model API and semantic boundary

Python 3.11+ native `float` is the only binary64 representation. Named
declarations remain frozen, slotted dataclasses with `value: float`; direct
unconstrained members use `float` inside frozen keyword-only Records. Checks
require `type(value) is float`: no implicit `int`, `bool`, String, Decimal,
NumPy-scalar or subclass conversion and no binary32 intermediate. Nothing in
production validation depends on `assert` or changes under `python -O`.

The shared `floating_domain(PrimitiveKind::Float64, effective_constraints)`
alone classifies range facets. Inclusive/exclusive lower, upper, mixed and
equal-inclusive singleton domains use its classifications. Bounds are emitted
as original 64-bit hexadecimal integers and unpacked with standard-library
`struct`, not reconstructed from rounded display text. Rust tests verify the
emitted bits and compare generated acceptance against the shared classifier
across ten domain shapes and IEEE edges.

Acceptance comparisons use `not (value >= lower)` etc., rejecting NaN for any
constrained domain. Unconstrained names/direct fields admit NaN and infinities.
One-sided domains retain ordinary infinity ordering; finite two-sided ranges
exclude infinities. Both zero signs compare equally, so an exclusive zero
boundary rejects both. Subnormal values are not flushed or widened.

Float64 named restriction ancestry is intentionally narrow: every declaration
must already be a concrete `Primitive(Float64)` with classifier-supported
effective constraints and compatible Float64 provenance. Each effective child
domain must be contained in its immediate parent's domain. Missing, cyclic,
incompatible or unresolved ancestry is rejected, including external IR that
drops inherited constraints. This does not generalize named ancestry to any
other family. Field-local constrained direct Float64 remains unsupported.
Wrong-width bounds, contradictory/empty ranges, ambiguous same-side facets,
nonfinite bounds and lexical/length facets fail closed before artifacts.

Shared selected-service projection/readiness/preflight is unchanged; there is
no second service resolver. Existing `effective_record_fields` governs inherited
metadata and order. Exact named types, nested Records, optional members and
bounded tuples compose normally. Multiple message QNames can share one typed
payload and its codec functions. Decoders construct the actual frozen classes.

## Lossless numeric ingress and output

Float64-free selections emit the original model/API/codec runtime bytes. Only
Float64 selections append private model/codec helpers; there is no public runtime
package, new flag or dependency. Existing base runtime files are unchanged.

Strict `json.loads` hooks retain every number as a private `_Number` lexeme.
The expected schema position determines its conversion:

* Float64: one `float(original_token)` decimal-to-binary64 conversion, correctly
  rounded nearest/ties-to-even on both qualified Python versions. Preserve `-0`,
  decimal/exponent signs, finite subnormals and rounded signed-zero underflow.
  Overflow to infinity is rejected as `CodecError`, not accepted as a special.
* Integer: reject every fractional/exponent spelling, then use the pre-existing
  chunked arbitrary-precision integer parser. Numbers longer than Python's
  default integer-string limit still work, without changing that global limit.
* Every other position rejects numeric proxies through existing exact checks.

JSON grammar remains the standard library's strict grammar. Bare nonstandard
NaN/Infinity constants are rejected; duplicate decoded member keys are rejected
at every depth, including escaped duplicates. Quoted finite numbers and wrong
special spelling/case/sign are rejected. Ordinary String members are unchanged.

Finite encoding uses Python's shortest binary64 `repr`, including `-0.0`.
Generated encoding validates outbound exact types and domains before mapping
nonfinite values to the three OMS Strings. No bare nonstandard number is emitted.
OMS does **not** preserve NaN payload/signaling/sign bits. Encoder correctness
is checked by an independent exact-rational oracle, not merely its own decoder.

`CodecError` retains schema member/index paths (including invalid later repeated
elements). Only explicit conversion/validation exceptions are categorized;
arbitrary programming failures are not swallowed. No unbounded conversion cache,
global digit-limit change, new message-size bypass or resource-exhaustion
recovery is introduced. PY-005 still passes original body substrings directly to
the generated decoder. There are no Ada/glibc/GNATCOLL platform assumptions in
Python production code. Standard-library generation is platform-independent;
actual numerical execution here is CPython 3.11.17 and 3.13.16 on Linux/x86-64,
not a claim about every alternative interpreter or hostile floating environment.

## Independent numerical qualification

The unchanged Task 096 `tests/ada-float64-probe/oracle.py` regenerated a fresh
task-owned corpus. The manifest exactly matches the frozen historical manifest:

| Corpus | Cases | SHA-256 |
|---|---:|---|
| conversions.tsv | 27,167 | `14c86c5fa674d279d198947dad03f71f517a40b583729df1184bbf6596a6451b` |
| encoding.tsv | 20,474 | `b1ad323c7d5d88c465bf966dc2edc1249c68982ad817044fa5579ce65e9abc82` |

All 2,047 finite exponent bands, signed zeros, subnormal/normal boundaries,
largest finite, adjacent values, decimal powers, midpoint-even/midpoint-odd,
underflow and overflow are covered. Each of four full runs (3.11 and 3.13,
normal and optimized) executes **27,167 conversions, five expected overflow
rejections, 20,474 finite encodings, zero mismatches**. Total: **108,668
conversions and 81,896 encoding checks**, with no skipped numerical cells.

Four negative controls detect corrupted exponent rounding, unsigned-zero
normalization, wrong ties-to-even and wrong special String output. The generated
decoder is tested against frozen independently computed expected bits. Every
encoder token is independently parsed by the integer/rational oracle. The
harness suppresses only the historical oracle's import-time global digit-limit
setter in its private execution namespace; it does not edit the historical
source or mutate the actual process limit. Fast runs 6,693 focused conversion
cases and 162 distributed encoding patterns plus controls; Deep runs the full
corpus, using both interpreters normally and under `-O`.

## Generated execution, parity and transport

The production Rust CLI generates synthetic models/API/codecs for both worlds.
The Float64 fixture executes **11 unittest methods × two worlds × two import
styles × normal/-O = 88 methods per interpreter**, 176 across both. Checked
scalar/range tests, negative schemas, deterministic regeneration, exact metadata
and model/API codec/no-codec parity execute in the Rust test targets too.
Wrong-family primitives, invalid facets, Choice and abstract values remain
atomic failures. `unittest` expectations stay active under optimization.

Non-Float64 byte parity against the verified parent passes 12 selected generations
and 30 model/API/codec source comparisons across codec/no-codec and both worlds.
Existing Ada/Rust/C++ output/report parity also passes: six full models, twelve
selected model/API generations, twelve readiness reports and two coverage reports,
plus eight legacy Python selections. Canonical digest remains
`b0749e258b6484b568c07b60f81b855353565488a5b26fc3ed224b160cfab235`.
The PY-001 golden model is unchanged.

Sleet uses the existing single clean checkout at
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`; no second broker build is introduced.
The reused broker executable SHA-256 is
`4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
The new shared-harness test passes on both Python versions normal and optimized:
XSD/contract → generated models/API/codec → LacalRuntime → OWP/WebSocket →
pinned Sleet → actual generated typed decoder → typed handler. It independently
compares delivered bits for signed zero, minimum subnormal, adjacent-to-one and
constrained values. The broker rejects the exclusive-upper-bound violation as
`Invalid-Message`; that invalid wire input does **not** reach the handler.
Subsequent valid delivery succeeds. Corrupted outbound bounded models fail before
publication; subsequent valid delivery succeeds. Clean unsubscribe/close leaves
no runtime tasks. Required marker: `PY008 PINNED SLEET FLOAT64: PASSED`.
PY-005–007 broker tests are preserved and executed against the same binary;
their historical evidence is not rewritten.

## Actual pinned-UCI measurements and historical CI

Both external roots were SHA-256 verified before use. UCI 2.5 revision
`093610b7753944059360d3236770ab446d039556`, root
`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`;
UCI 2.6 revision `78eb61b6112c8bffa40820c33124b57787fc5bd9`, root
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
All **24 selections** retain exact message/world/codec and contract identities,
selected closure, stdout/stderr/exit and stage verdicts in task-owned BEFORE and
AFTER files. Both model and API remain NOT READY, and codecs remain NOT MEASURED
when requested. No complete service newly reaches READY.

| Release | Actual message | BEFORE first blocker | AFTER first blocker |
|---|---|---|---|
| 2.5 | PositionReport | primitive Float64 | primitive DateTime |
| 2.5 | RDMA_InitializeSetup | primitive DateTime | primitive DateTime |
| 2.5 | SubsystemStream | primitive DateTime | primitive DateTime |
| 2.6 | PositionReport | primitive Float64 | unsupported constraints |
| 2.6 | RDMA_InitializeSetup | unsupported constraints | unsupported constraints |
| 2.6 | SubsystemStream | primitive Binary | primitive Binary |

These are the actual diagnostics in both worlds and codec modes, not inferred
feature inventories. In particular, UCI 2.6's generic `unsupported constraints`
diagnostic is not relabeled as a guessed primitive. The dedicated synthetic
Float64 selection changes from NOT READY to READY separately; that is not real
PositionReport or complete-UCI interoperability.

`scripts/check-py008-uci.py` reuses the existing strict driver and inherited
Record/profile executions. PY-006 and PY-007 historical fixtures retain immutable
hashes and original assertions. PY-007 defaults remain historical; its optional
versioned service baseline allows the successor gate without deleting assertions.
The new gate requires all 24 identities, exact pins/contracts, stage verdicts,
first blockers, completion, and 96 inherited/32 isolated-profile executions.
Edited READY text cannot qualify a service. Five adversarial controls exercise
missing/duplicate rows, source/contract identity, process/blocker corruption and
edited-oracle false readiness. The existing Fast matrix and Deep infrastructure
carry the gates; the shared broker build/download is not duplicated. CI split
guards and adversarial self-tests require the new gates while retaining old ones.

### Measurement incident, not accepted evidence

A first AFTER capture accidentally used a stale baseline release executable
because baseline/candidate builds reused the task-owned Cargo target. It was
stopped and discarded, the candidate rebuilt and verified against synthetic
Float64 READY, then all 24 selections were freshly recaptured. The preserved
verified BEFORE executable is a separate copy; accepted evidence comes from
`before-uci-release` and `after-uci-measured`, not that discarded capture.
The superseded slow debug BEFORE run was stopped after the complete release
BEFORE passed. Neither incident changes or overwrites historical evidence.

An initial CI split self-test overlapped edits to its guard script and failed
against the transient file contents. It is not accepted as a passing run; the
complete stable-file rerun and its exact count are recorded with final results.

## Reproduction and disposition

Use Rust 1.95.0, both explicit Python interpreters, task-owned Cargo targets and
the existing pinned roots/broker. Core commands:

```bash
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --workspace --all-targets --locked
cargo +1.95.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.95.0 test --locked -p ams-gra-oms-backend-python
cargo +1.95.0 test --locked -p ams-gra-codegen-oms --test python_cli \
  --test python_service --test python_codec --test python_header_profiles \
  --test python_inheritance --test python_float64
AMS_GRA_REQUIRE_GNAT=1 cargo +1.95.0 test --workspace --locked
python3.11 scripts/test-py007-uci.py
python3.11 scripts/test-py008-uci.py
python3.11 scripts/check-py008-uci.py --generator /absolute/generator --output /fresh/evidence
python3.11 tests/ada-float64-probe/oracle.py /fresh/corpus
python3.11 scripts/check-py008-numeric.py --full --generated /generated \
  --corpus /fresh/corpus --output /evidence/numeric.json
bash scripts/check-ci-split.sh
bash scripts/test-check-ci-split.sh
git diff --check
```

Repeat Python execution under 3.13 and `-O`; numerical totals, local regression
completion and exact-head hosted results must be checked rather than inferred.
Final qualification records are task-owned, not edits to PY-006/PY-007 or Task
096 results. Stop at independent review; do not merge, self-approve or enable
auto-merge.

### Proposed-source local completion checkpoint

Production implementation commit `8c2bbe148c4557dd2fea51bb86f49068b2f620bf`,
tree `c733c9d1597064a0c8c236a3fe64ab222b5fd99b`, was pushed normally and
opened as the single non-draft stacked PR
[#119](https://github.com/zackboll/ams-gra-codegen-oms/pull/119). Subsequent
qualification/evidence commits do not change the generated Float64 sources.
The final separate-target executable SHA-256 is
`f1a0fd113c155f232b625500d6141496b68d8bc06d0bb234757c55a5586c14f9`.
Its generated model/API/codec bytes match all four independent numeric cells.

Both explicit Python interpreters pass **19 backend Rust tests and 23 CLI
Rust tests each**, with zero failures/ignores; these execute normal/optimized
generated probes, including the 176 Float64 unittest methods. Both final-source
UCI campaigns pass all 24 service selections plus 96 inherited and 32 isolated
profile executions each. Final-source Float64 Sleet tests pass all four cells.
Rust 1.95 formatting, locked workspace/all-target check and warnings-denied
Clippy pass. Actionlint and diff whitespace checks pass. CI split guard passes;
its stable-file adversarial self-test passes **502 checks**.

The first GNAT-required locked workspace run completed **1,488 passed, zero
failed, eight pre-existing ignored**, in 189 target/doc summaries. That run
predates the last additive domain/collision tests, so the complete proposed-source
GNAT-required rerun is tracked separately and is **not yet claimed complete** at
this checkpoint. The inherited Rust real-UCI/Sleet build was restarted as a
task-owned release campaign after the debug build spent over 18 minutes in its
build script without a verdict; it too is not yet claimed passed here. No failed
or unexecuted campaign is counted as a pass. Both exact implementation-head hosted
Fast and Deep runs were still **queued** at the last checkpoint. This is
**HOSTED_CI_PENDING**, not READY_FOR_INDEPENDENT_REVIEW.

Remaining boundaries: Float32, Decimal, DateTime/Time/Duration, Binary, general
numeric/String patterns, Choice, abstract value polymorphism, `$type`, general
named ancestry, field-local constrained direct Float64, unbounded/nillable
cardinalities and additional transport capabilities remain unsupported. No
resource-exhaustion recovery or NaN payload preservation is claimed.