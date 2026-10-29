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

During final qualification, main advanced through PR #113 to
`d603077c6d38ccc0658391776e3af9eecb009255`, tree
`5ea3873e3bf6a8e2960e8d1faa57049d867a2456`. Ordinary merge
`6c467b266d672f549c7139422d90bc70d569d670`, tree
`e998e81d4294912185ed69f51f5158a1dc260900`, incorporated it into PY-008
without editing PY-007 or retargeting the stacked PR. The twelve incoming
trusted-dispatch security/workflow/document files are byte-identical to main;
production backends, tests, runtime and existing Fast/Deep workflows are
byte-identical to the qualified PY-008 source. The incoming hash-pinned PyYAML
requirements were installed in a task-owned venv: all **30 CI002 security tests**
pass, including mutation/provenance gates. The authenticated historical workflow
probe returns exactly `4c79fff41970a8ba7109df9bd185a6b166c0ffb0`. Actionlint
and the existing CI split guard pass with these files present. No activation
variables, credentials, runner settings or another active PR were modified.

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
GNAT-required rerun was tracked separately: it now completed **1,491 passed,
zero failed, eight pre-existing ignored**, again 189 target/doc summaries.
The integrated-main release executable is byte-identical to the qualified
final generator. Integrated-main focused suites, workspace check and Clippy
also pass; incoming trusted-dispatch files do not change Cargo or Python inputs.
The inherited Rust real-UCI/Sleet build was restarted as a
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

## R1 — reconciliation with merged PY-007 and current CI architecture

The earlier sections are implementation history, not a statement that PR #117
is still open or that the old stacked target/independent Deep PR trigger applies.
R1 verified PR #117 **MERGED**, PR #119 **OPEN**, non-draft, targeting `main`.
No replacement branch/PR, rebase, amendment, cherry-pick or forced update is used.

### Provenance and conflict-resolution manifest

| Checkpoint | Commit | Tree |
|---|---|---|
| Original qualified PY-008 PR head | `b9d9fcf74ace0a3063ec28d823979adf41fc166d` | `0f89dbf953dcbded5d38c24957aff88eaffcfd78` |
| Fetched starting main / merged PR #117 | `b6c6c9c5b1fabcd0716c1fd1183cc411d8841872` | `9dc99ef6531ab61b248f83b5f613e981e4002bc0` |
| Merged PY-007 feature parent | `71da44f199b135100697e2837732e54a9aa26370` | `9dc99ef6531ab61b248f83b5f613e981e4002bc0` |
| Historical qualified PY-007 dependency | `d76a36d2cb9d18bcc8b825b5134a65686f361deb` | `bb28f2e70e0c2b1c75ed19ff390f8c1df40773c3` |

The existing worktree was clean and had no observed competing task process or
Git lock. Before integration, all 36 requested production/test/script/fixture
files, plus the Task 096 oracle and manifest, were copied read-only and recorded
with Git blob IDs and SHA-256 in
`/home/zboll/py008-r1-evidence/original-manifest.json`. Its SHA-256 is
`13f873bfb3d6e06b81c0971f414ae1278d408cf26f9df3c8d189bf79e1510ea9`.
Original qualification files were not regenerated or rewritten.

`git fetch origin main` followed by `git merge --no-ff origin/main` produced
one textual conflict, in `.github/workflows/deep-ci.yml`: the old PY-008
`pull_request.paths` block conflicted with main's reusable entrypoint. The
obsolete direct PR trigger was removed; `workflow_call`, nightly/manual entry,
and the ordinary CI selector/caller remain authoritative. No whole conflicted
file was selected from either parent.

The automatically merged surfaces were reviewed and reconciled explicitly:

- `.github/workflows/ci.yml`: retain main's `changes`, five shared lanes,
  `queue: max`, 20-minute `baseline`, and mandatory `pr-ci-required`; add
  Float64 and PY-008 UCI controls to both existing Python cells without dropping
  any historical Python target. Keep focused numerical execution normal/`-O`
  against actual CLI-generated code and pin its Rust toolchain to 1.95.0.
- `.github/workflows/deep-ci.yml`: retain all three PERF-007 primary shards and
  existing source verification/artifacts; integrate the versioned PY-008 UCI
  gate in the existing PY-007 current-service step, retaining both sets of
  controls and the fail-closed `py006-uci` artifact. Own the four full numerical
  cells once in `real-sleet`, with mandatory `py008-numeric` evidence.
- `scripts/check-ci-split.sh` and `scripts/test-check-ci-split.sh`: replace only
  obsolete PY-008 path-trigger expectations with selector/owning-payload and
  shared-Sleet checks; retain inherited PY-007/PERF/security coverage and
  require the intended diagnostic for every new mutation.
- `scripts/ci-qualification-inventory.json`: reconcile exactly the three owning
  job payloads and the new numeric artifact; add PY-008 evidence paths and
  include Python/Deep in the existing shared Task 096 oracle/corpus ownership.
- `scripts/ci_policy.py` and `scripts/test_ci_select_py008.py`: require both
  Python versions and Deep for Float64 surfaces; reject removed, masked,
  renamed, filtered, version-reduced, optimization-reduced or source/artifact
  altered qualification. Pin the already-qualified shared Sleet harness,
  including its loop and build, against early exits/outer false conditions.

All other inventory records, orchestration payloads, inherited-source IDs,
trusted-dispatch source hashes/security headers, aggregate gate, selector
implementation and PERF-007 evidence reconciliation remain identical to main.
No trusted-dispatch hash was recalculated to authorize workflow changes.

### Production and historical parity

All **36 frozen files match byte-for-byte** after integration. The complete
Python backend, runtime tests and historical fixtures have no changes relative
to original PY-008. Main's production/fixture trees have no changes relative to
the historical PY-007 dependency, so no baseline drift requires an exception.
The final diff against main introduces only incremental Float64 production,
tests, evidence, documentation and necessary CI integration, not PY-007-only
files. Shared `effective_record_fields`, frozen concrete Records, ancestry-only
abstract elision, demanded abstract-value rejection, both worlds and inherited
QName/order behavior are preserved and re-executed by the inherited tests.

The preserved historical PY-007 executable and current release executable
match the unchanged frozen `nonfloat-parity.json`: **12 selections / 30 Python
model/API/codec files**. All **40 legacy generation/report selections** also
match `legacy-parity.json`, including six Ada/Rust/C++ full model generations,
twelve selected generations, twelve readiness reports, two coverage reports
and eight Python non-codec selections. Canonical legacy evidence SHA-256 stays
`b0749e258b6484b568c07b60f81b855353565488a5b26fc3ed224b160cfab235`.
The integrated release generator SHA-256 remains
`f1a0fd113c155f232b625500d6141496b68d8bc06d0bb234757c55a5586c14f9`.
Historical dependency/source identities in PY-008 evidence remain unchanged.

### R1 local qualification

Rust 1.95.0 formatting, locked workspace/all-target check and warnings-denied
Clippy pass. Focused backend suites pass **19 Rust tests** and the six CLI
targets pass **23 Rust tests**, separately with Python 3.11.17 and 3.13.16;
zero failures, ignores or filters in those suites. The Float64 CLI test executes
**88 unittest methods per interpreter**, covering both worlds, standalone and
package imports, normal and optimized execution (176 methods across versions).
`AMS_GRA_REQUIRE_GNAT=1` locked workspace execution completes **1,491 passed,
zero failed, eight pre-existing ignored, zero filtered**, in 189 summaries.
Ignored external campaigns are not counted as successful execution.

| Numerical cell | Decimal conversions | Finite encodings | Mismatches | Corruption controls |
|---|---:|---:|---:|---:|
| Python 3.11 normal | 27,167 | 20,474 | 0 | 4/4 |
| Python 3.11 optimized | 27,167 | 20,474 | 0 | 4/4 |
| Python 3.13 normal | 27,167 | 20,474 | 0 | 4/4 |
| Python 3.13 optimized | 27,167 | 20,474 | 0 | 4/4 |

The unchanged oracle and verified corpus hashes above are reused, not adjusted.
All four smaller Fast samples also pass **6,693 conversions / 162 encodings**
each, with all controls and zero mismatches. Both current-UCI campaigns pass
**24 actual selections, 96 inherited tests and 32 profile executions each**;
zero complete services READY and the exact six first blockers in the original
table remain unchanged. PY-006/PY-007 history and source/contract/process/stage
identities are checked by the versioned gates, not relabeled as new evidence.

Offline policy checks pass **61 selector tests**, **18 real-UCI evidence and
performance tests**, **30 CI002 tests**, and both PY-007/PY-008 **five-test**
UCI-control suites. The split self-test passes **556 checks**, including the
new seven-test attributed Float64 policy suite and inherited layout tests.
Task 060/061 wrappers pass **96/4 controls**, Task 093 placement passes **12
tests**. Pinned actionlint 1.7.12 and documentation/whitespace policies pass.
Authenticated historical CI002 provenance remains exactly
`4c79fff41970a8ba7109df9bd185a6b166c0ffb0`. Large logs and generated code are
outside the repository in `/home/zboll/py008-r1-evidence`.

The complete shared pinned-Sleet campaign passes: **nine Rust tests**, zero
failures/ignores, two explicitly filtered counterparts, plus **14 Python
executions** (PY-005 normal/optimized and PY-006/007/008 in both interpreter
versions/policies). A separate PY-005 3.13 run adds its two executions. All four
PY-008 real broker delivery/rejection/recovery cells pass. The existing clean
Sleet checkout/build at `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` is reused;
its binary SHA-256 remains
`4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
The external local harness uses the unchanged shared script's campaigns with
explicit Rust 1.95/release/locked commands, the existing pinned checkout and
task-owned target, not a second broker. All four existing real-UCI Rust codec
tests also pass (two deliberately filtered Sleet counterparts are not passes).
The historical PY-007 executable separately requalifies all 24 historical
services, 96 inherited tests and 32 PY-006 profile executions against its
unchanged default baseline. Both Python mock campaigns pass **38 methods per
version**, normal and optimized, with no skips.

Exact ordinary-merge identity, final push identity and exact-head hosted
outcomes are recorded at the completion checkpoint below;
none is inferred from the old PR head's green workflow runs. No merge,
self-approval, release, publication or auto-merge is authorized.

### R1 integration completion checkpoint

The ordinary integration commit is
`84f7d5f171d2254c84ba9c10bd21cbd5ce16b2c0`, tree
`0abd08c302374780f7e4601e75c8766803f17c8e`. Its ordered parents are original
PY-008 `b9d9fcf74ace0a3063ec28d823979adf41fc166d` and incorporated main
`b6c6c9c5b1fabcd0716c1fd1183cc411d8841872`. Both historical and merged PY-007
ancestries are retained. The stable-source adversarial rerun passes all **556
checks**; all 42 recorded local gates and the 61 selector tests pass. The
follow-up evidence-only commit changes no qualified executable or CI input.

Final-source gate SHA-256 identities:

| Surface | SHA-256 |
|---|---|
| `.github/workflows/ci.yml` | `102fc1366bcdca1ba04b21221b4a7c9da1559eb6f50e5e949119f38102fe6b70` |
| `.github/workflows/deep-ci.yml` | `c2b6f0cc973d7956234328a12d1422035706ed858f0bd64c40b04378f241ed9a` |
| `scripts/ci_selection.py` | `6b61d83d500527c7f4a91bf83248f210273c6ea4569f813a5bc9b1650563f638` |
| `scripts/ci_policy.py` | `c836ae41846a4bc72fc9f902c9202ab75026d1b3a790f111a0ae81566e54195f` |
| `scripts/ci-required-gate.py` | `118b2792dd7eaac9a45d5b617b5f2b79146a56f504785ac48963193e91e70434` |
| `scripts/ci-qualification-inventory.json` | `03e817dfe8ee912798d9211ca0993c0f0e201214d13ee6d854430f79155c6c4a` |
| `scripts/check-ci-split.sh` | `2d07ff4fc9792bd2eb659f2d32555f2b750ce0354d16be36801a2e1c3edff8eb` |
| `scripts/test-check-ci-split.sh` | `780719c932e295e774f405a2cbfd783cb568fdc8115c27c0531153b49ded60fe` |
| `scripts/test_ci_select_r1.py` | `e7a82f5c59688dd4ab5e553413bcaac733a6510e83c3f7f844b8bac4251f9f16` |
| `scripts/test_ci_select_py008.py` | `e8e832cbfb6d68a2d1c6faeeb4796d473ea7305965f035465a1e495203a500ab` |

The final commit/tree, normal-push/PR-head equality, complete PR diff and
exact-head workflow IDs/outcomes are recorded in the external delivery handoff
and existing PR #119. At this local checkpoint the disposition is
**HOSTED_CI_PENDING**, not independent-review readiness: the final proposed
integration must still satisfy the selected ordinary/reusable Deep jobs,
CI002 security and source-bound `pr-ci-required`, without a failed, cancelled,
timed-out or unexpectedly skipped mandatory cell. PR #119 remains unmerged.

## PY-008-R2 — Task100 and RUST-CI-001 integration

### Verified provenance and preservation

R2 starts from qualified PR #119 HEAD
`af467b4789758cab544829a15bc0ae5d2d96cbba`, tree
`bb030913d5f7f04d208bd2575eba2f0836643c5a`, with a clean worktree on the
existing `feature/py-008-python-float64` branch. Remote refs and independent
GitHub branch metadata agree on main
`7958c4e0d8f78768fd4c7d4c1bd34877c31d893a`, tree
`a94fc3bb844349780772566eafd50db7a7328efe`. The merge base is
`b6c6c9c5b1fabcd0716c1fd1183cc411d8841872`. No competing worktree process or
Git lock was observed. The ordinary merge's first parent must be the starting
PR HEAD and its second parent the independently fetched main above.

The R1 pending checkpoint above is historical: R2 independently confirms R1
CI run **40794646389 succeeded with all 31 jobs**, including `pr-ci-required`,
and CI002 run **40794646242 succeeded** for the exact R1 HEAD. Those results
do not qualify R2. Main run **40800583532** was still pending when inspected;
its merge is not evidence that main qualification completed.

Before merging, R2 froze **122 files**, with Git blob IDs, SHA-256 hashes and
read-only external copies. **All 36 original frozen PY-008 files remain
byte-identical**, as do all backend/runtime/CLI Float64 implementation files,
historical PY-006/PY-007 fixtures, numerical scripts and the Task096 oracle
and corpus manifest. Of the broader 122-file capture, 121 are unchanged; the
sole intentional extension is `scripts/test_ci_select_py008.py`, explicitly
required to qualify the new 40-cell policy. No historical evidence is
regenerated or revised. The final executable SHA-256 remains
`f1a0fd113c155f232b625500d6141496b68d8bc06d0bb234757c55a5586c14f9`.

Task100's Ada runtime/transport, GPR companions, fixtures, local peer,
qualification driver, evidence verifier and original documentation are
byte-identical to fetched main. RUST-CI-001's entire facade-test source tree,
worker-join helper, fairness/retry/lifecycle regressions and qualification
documentation/evidence are also byte-identical to main. Their merged
ancestries are preserved; no duplicate or reverted implementation is added.

### Complete conflict and semantic reconciliation manifest

There is **one textual conflict**, in
`scripts/ci-qualification-inventory.json`: the Task096 Float64 rule has
Python ownership on R1 and Ada WebSocket transport ownership on main. The
resolution takes the union, retaining Rust, Ada JSON/Float64/TimeZulu/runtime,
Task100 GNAT14/GNAT16 transport and both Python versions, plus Deep.

Automatic merges require the following semantic review/corrections:

| Surface | Resolution |
|---|---|
| Ordinary CI | Retain both Task100 transport compiler cells and aggregate dependency; preserve exact R1 Python model/Float64 commands and Rust 1.95 pin. |
| Reusable Deep | Retain independent Task100 GNAT14/GNAT16 Sleet campaigns, all PERF-007 shards, and the R1 Python full numerical/UCI/shared-broker campaign. |
| Qualification inventory | Restore original main Python/Deep payloads as historical authority; separately retain reviewed R1 additive payloads and parsed steps. Preserve new-main selection rules and all 40 expanded cells (39 main-eligible). |
| `ci_policy.py` | Keep every original `INHERITED_PAYLOADS`, Task100 original/integration distinction and seven CI002 hashes. Separately pin R1 integration YAML and parsed step arrays; project the narrow additive delta back to the original mandatory steps. Preserve the optional third shared-Sleet argument and all Task100 checks. |
| `ci-required-gate.py` | Require the integrated Python named steps, not just the historical subset. Existing four-cell Task100 and three-shard PERF-007 evidence reconciliation remains intact. |
| Selector tests | Healthy aggregate fixtures contain the additive named steps. All inherited assertions and tests remain; PY-008 adds attributed negative controls, step-array drift rejection and 40-cell coverage. |
| Rust qualification guard | Bind the unchanged main worker-join source identities and original Rust job payload so removal/exclusion cannot silently avoid the newly inherited tests. |

`PY008_INTEGRATED_PAYLOADS` is derived from **qualified R1**, not from merged
candidate workflows. Original payload hashes are verified separately against
the actual fetched-main Git objects. Parsed original and integrated step
contracts are separately pinned because the final gate consumes those arrays.
The additive projection permits only the reviewed Float64 commands/steps,
Rust pin, versioned UCI baseline argument and combined step name. Removed
inherited checks replaced with unrelated successful commands are rejected.
Comment-only differences do not remove executable qualification.

All five lanes and lane assignments, `queue: max`, baseline **20-minute**
timeout, mandatory aggregate, trusted workflow protections and seven CI002
source hashes are preserved. Task100's four artifacts remain:
`task100-ada-websocket-transport-14`, `task100-ada-websocket-transport-16`,
`task100-ada-websocket-sleet-14`, `task100-ada-websocket-sleet-16`.

### R2 numerical, generated-code and interoperability execution

| Interpreter/policy | Conversions | Finite encodings | Mismatches | Corruption controls |
|---|---:|---:|---:|---:|
| Python 3.11 normal | 27,167 | 20,474 | 0 | 4/4 |
| Python 3.11 optimized | 27,167 | 20,474 | 0 | 4/4 |
| Python 3.13 normal | 27,167 | 20,474 | 0 | 4/4 |
| Python 3.13 optimized | 27,167 | 20,474 | 0 | 4/4 |

All four Fast samples separately pass **6,693 conversions / 162 encodings**.
The frozen corpus hashes and all three CLI-generated Python source hashes
match R1. Actual probes cover binary64 rounding/ties, signed zero, subnormals,
NaN/bounds, infinities, special strings, huge integers, finite numeric
serialization and duplicate-member rejection without acquiring unsupported
Float32/Decimal/DateTime/Binary/Choice behavior.

Each explicit interpreter passes **19 backend Rust tests + 23 focused CLI
Rust tests**, zero failed/ignored/filtered. Their generated Python probes
execute 24 backend unittest methods and 440 focused methods per interpreter;
the Float64 standalone/package normal/optimized subset is **88 methods**.
The network mocks execute **38 methods per interpreter**, without skips.

The current generator reproduces all **12 non-Float64 selections / 30 Python
files** and **40 legacy generation/report selections**, with the complete
result objects equal to frozen `nonfloat-parity.json` and `legacy-parity.json`.
Both pinned UCI release commits and root hashes are verified unchanged.
Each interpreter executes **24 current selections, 96 inherited-Record tests
and 32 isolated-profile executions**, with **zero complete services READY**.
The blockers remain DateTime for all UCI 2.5 messages; UCI 2.6 PositionReport
and RDMA_InitializeSetup have unsupported constraints, and SubsystemStream
has Binary. Historical profiles/expected verdicts are not rewritten.

The complete Python shared pinned-Sleet campaign passes **nine Rust tests**
and **14 Python executions**, including all four Float64
delivery/rejection/recovery cells. Both additional PY-005 interpreter
normal/optimized campaigns pass. Four inherited real-UCI Rust codec tests
also pass. The shared campaign and codec campaign each explicitly filter two
counterpart tests; filtered tests are **not** counted as executed passes.
The original clean pinned broker/build is reused; the Ada Task100 campaigns
remain independent executions, not substitutions for Python qualification.

### R2 local toolchain and policy gates

Rust **1.95.0** formatting, locked workspace/all-target check and
warnings-denied Clippy pass. The full **GNAT-required locked workspace**
passes **1,494 tests, zero failed, eight pre-existing ignored, zero filtered**,
across 189 summaries. All three new worker-join close/drop/panic tests execute
and pass; the increase over historical 1,491 is not hidden or backdated.

Selector/policy tests pass **91 tests** (including Task100's separately
executed **21-test** policy/evidence-verifier entrypoint). UCI
evidence/performance tests pass **18**, CI002 **30**, PY-007/PY-008 controls
**five each**. The split adversarial harness passes **556 checks**; inherited
Task060/061 wrapper controls pass **96/4**, and Task093 placement passes
**12 tests**. Pinned actionlint **1.7.12**, documentation policy and whitespace
checks pass. Initial policy-projection development failures are retained
externally, not counted as passes. An extra codec invocation named nonexistent
CLI targets, and an actionlint invocation used a nonexistent path; corrected
actual entrypoints pass without source/test-expectation changes.

Task100 is executed through unchanged `scripts/task100/qualify.py` with the
original Alire **2.1.1**, GNAT **14.2.0/16.1.0**, GPRbuild **26.0.1**,
GNATCOLL/minimal **26.0.0**, Linux x86-64 glibc **2.41**, pinned AHT and
websocket-ada sources/licenses, and pinned Sleet. Original setup input hashes
are verified before reuse; an immutable exact-source generator is supplied.
Both local transport compiler campaigns pass **24 cells / 1,044 scenarios /
12 Float64 framing probes each**. Both separate Ada pinned-Sleet compiler
campaigns also pass **24 cells / 1,044 scenarios / 12 Float64 probes each**,
with **six real pinned-Sleet executions per compiler**. Clean-source artifact
verification is recorded at the R2 completion checkpoint, not inferred from
these staged-source local runs or earlier Task100 runs.

Precommit campaigns honestly record their staged worktree state and R1 HEAD;
they are local execution evidence, **not clean hosted artifacts**. Source
identities/status files must never be rewritten to make them pass the clean
artifact verifier. Final exact-source Task100 evidence and hosted acceptance
remain separate gates. Full manifests, command logs, failure histories,
generated output and count summaries are external under
`/home/zboll/py008-r2-evidence`, not committed generated artifacts.

### R2 completion and hosted acceptance

The exact ordinary-merge SHA/tree, ordered parents, normal-push equality,
final-head hosted run IDs, accepted artifact identities and disposition are
recorded in the completion handoff and existing PR #119. No earlier green
head qualifies this new integration. R2 is not independent-review-ready
until every selected ordinary/reusable Deep cell, both Task100 transport and
both Task100 real-Sleet compilers, all PERF-007 primary shards, numerical/UCI/
Python Sleet evidence, CI002 and `pr-ci-required` pass for the final source.
No merge, approval, auto-merge, release, publication or tag is authorized.