# Task 094 — Ada repeated temporal OMS JSON codecs

## C7 — original-baseline certification

The preserved original-baseline qualification is certified against starting main
`624364c4f8f6212edee3afc04e331c4064202659`, **not** against Task 092 main.
Manifest: `/home/zboll/task094-scratch/c7/original-baseline-certification.json`
(SHA-256 `e5e165bcaf5e1220c808206db8881c801114c25ce9107758b2bba10c0eed2410`).
The audit script and all new evidence reside in `/home/zboll/task094-scratch/c7`.

Qualified production SHA-256:

- Core `service_codec.rs`:
  `b6f1fd511800a35779dcdae8fc91d7b59a5055d60204050c4bb74f49eaa9b361`.
- Ada backend `service_codec.rs`:
  `8cdb1ab5b3b4a3413ade1bb0d384f5543388167eeec8334fa669dd1aa207a0fe`.

All production, fixture, consumer and runner hashes match the C5 certified
snapshot; only this documentation subsequently changed. Relative to the C4
090R/091R executions, each runner's sole later change suppresses Python import
bytecode writes before importing the shared control module; the old hashes were
reconstructed exactly to establish that no assertion or invocation changed.
CI changes were separately qualified by C5. No lengthy campaign was repeated.

Verified: expanded 12 cells (54 checks/17 rejections); composition 12 cells
(19 checks/3 rejections); model 6 cells (24 checks); CLI 18/18; core codec 21/21;
inherited 14/14; workspace 1,412 passed/8 ignored; 25 byte-identical generations;
CI mutation 209 checks/exit zero; all 303 C6R-C4 evidence hashes; all 80 complete
UCI comparisons. No Task 094 worker remains active. The manifest records exact
campaign commands, result identities, production and task-owned source hashes,
UCI attribution, and limitations. This section is a documentation-only update
after certification; earlier failed/interrupted attempts remain below.

The original UCI result is unchanged: 36 NOT_READY and four known projection
errors per release/variant, no newly READY real service. PRI's owner projection
improves 5/6 to 6/6; Scan remains Float64-blocked. Task 092 integration and all
integrated qualification remain required before publication. The composition
runner's inaccurate aggregate count must be corrected before that qualification;
its original per-cell 19/3 evidence is authoritative.

## C6R-C4 — UCI_REACHABILITY_PASSED

Fresh evidence: `/home/zboll/task094-scratch/c6r-c4`. Production source remains
unchanged, HEAD `624364c4f8f6212edee3afc04e331c4064202659`. No commit,
integration, push, PR or Task 095. Earlier failures remain recorded below.

### Correct generated-value proof

The scratch validator now separates semantic named reachability from generated
containment. The exact typed `Projection(ProjectedEmissionPlan(...))` boundary,
full OAM QNames, closed four-position path and directed rotation checks remain.
`QueryPET -> QueryType` requires a demanded abstract target and a concrete
descendant returned by `project_abstract_value`. Other edges require effective
Record/Choice members with stored-value semantics; inheritance alone is not
accepted. `classify_abstract_value_topology` independently corroborates the
same recursive graph modulo rotation, never reversal. Original diagnostics
are retained and projection errors have null codec counts.

Real pinned 2.5 and 2.6 proof executions passed all six rotations and missing-
member controls. Unit tests retain reversed/missing/extra/open/misqualified/
wrong-variant/malformed and false-wrapper/ancestry-only negatives. Scratch
formatting, full unit tests, synthetic reachability and final unit rerun passed.
Proof details: `generated-value-cycle-proofs.json`, `cycle-classifier-tests.json`.

### Complete candidate accounting

Preserved C6R-C2 Phase A artifacts were consumed, not regenerated. Each release
has two named zero-facet repeated Duration fields, both 0..unbounded:
`PRI_DataBaseType.PRI_PatternInterval` and `ScanDataBaseType.ScanPeriod`.
Each owner reaches 40 messages: 80 exposure pairs, 40 distinct messages.
The original 722/725 all-message closure checks remain authority. All six
recorded RDMA/Log/Survivability selections reach neither owner semantically
nor as generated-support-only.

Validated/reused **29 checkpoints per variant (58 total)**, including renewed
generated-value validation of saved cycle diagnostics. Original saved files
were not modified. Fresh **51 evaluations per variant (102 total)** resume
2.5 at ordinal 30, then process all 2.6 candidates. Every fresh candidate has
an ATTEMPT record followed by atomic result publication. Separate baseline
and current manifests link the archived baseline and Task 094 libraries.

| Release | Variant | NOT_READY | PROJECTION_ERROR | READY | Unexpected |
|---|---|---:|---:|---:|---:|
| 2.5 | baseline | 36 | 4 | 0 | 0 |
| 2.5 | Task 094 | 36 | 4 | 0 | 0 |
| 2.6 | baseline | 36 | 4 | 0 | 0 |
| 2.6 | Task 094 | 36 | 4 | 0 | 0 |

All 80 baseline/current message pairs are present exactly once; both temporal
owners remain linked to each message. Projection errors affect `ActivityPlan`,
`OrbitActivityPlan`, `Response`, and `RouteActivityPlan` in each release.
Their exact before/after diagnostics match and canonical cycle identities
agree. They are completed error outcomes, not ordinary NOT_READY verdicts.

### Measured declaration benefit and limits

All 72 ordinary readiness pairs (36 per release) retain model/codec total
counts and exact first blocker, while gaining exactly **one codec-renderable
declaration**. The newly qualified identity is full-OAM `PRI_DataBaseType`.
An additional attribution-only production API projection of each owner
confirms PRI's emitted closure improves **5/6 -> 6/6** in both releases,
removing `PRI_PatternInterval: named Duration repeated element is not qualified`.
This artificial owner projection is explicitly not real-message qualification.

The `ScanPeriod` field is admitted, but `ScanDataBaseType`'s owner projection
remains **10/13**, first blocker `AnglePositiveType` Float64. It does not add
another completely renderable declaration. Real candidate blockers include
existing Float64 limitations (exact per-message blockers are preserved).
No whole real message becomes codec READY; no generated real-UCI Ada typed
execution or deployed-service claim is made.

### Definitive evidence and next milestone

Atomic job results all exit zero: `tests`, `synthetic`, `pinned-proofs`,
`current`, `baseline`, `impact-current`, `impact-baseline`, and `final-tests`.
Per-candidate source records, attempt/result artifacts and manifests remain.
Summary artifacts: `reused-checkpoints.json`, `resumed-candidates.json`,
`candidate-readiness-before-after.json`, `projection-error-cases.json`,
`declaration-impact.json`, `qualification-summary.json`,
`input-source-hashes.json`, `evidence.sha256`. Distinct executable hashes and
dependency locks are retained. Original twelve CLI controls were not rerun.

This closes the focused UCI reachability/candidate gate. Original-baseline
certification may now advance to the existing C6 local-commit/Task 092
integration milestone, subject to its remaining final-source gates. This
checkpoint does not itself integrate or authorize PR publication.

## C6R-C3 — directed rotations tested; semantic-edge checkpoint blocked

Disposition: **QUALIFICATION_FAILED**. Fresh scratch:
`/home/zboll/task094-scratch/c6r-c3`. Production files and previous artifacts
are unchanged. No commit, integration, push or PR was made.

The scratch classifier parses full QNames, requires a closed four-position
path, accepts only rotations of the exact directed QueryPET/QueryType/
QueryMatchType order, records a stable canonical cycle, and requires each
directed edge through `direct_named_dependencies`. It retains the typed error
and original diagnostic, and produces no readiness counts for topology errors.

Classifier unit tests passed all three rotations and strict negative cases:
reversal, omitted/extra nodes, open paths, missing edges, other namespace,
unrelated cycle identities, other typed error variants, and malformed paths.
Synthetic reachability execution passed. Initial resume-path compilation
failed E0282 because removed Phase A work left untyped unused vectors; that
scratch defect was fixed and the full suite passed again (3 tests, exit zero).
All attempts remain separately recorded.

The resume harness consumes preserved C6R-C2 Phase A artifacts rather than
recomputing exposure/closure cross-checks. Before reusing candidates it checks
ordinal/message/source variant and revalidates saved projection cycles. Both
baseline and Task 094 resume jobs completed exit **101** during this reuse
validation, before fresh production readiness calls. Exact assertion:
`reused cycle edges must validate` (`src/main.rs:105`).

The pinned normalized graph explains the failure: `QueryPET` is an empty
abstract extension point and has no named reference to `QueryType`.
`QueryType` declares base `QueryPET`, giving the semantic edge in the reverse
direction. The reported generated-value cycle introduces `QueryPET -> QueryType`
through closed-sum concrete-descendant storage. This is a production generated
representation edge, not a `direct_named_dependencies` edge. The C6R-C2
classifier explicitly allowed that abstract-wrapper edge, whereas C6R-C3's
strict requirement does not. The two graphs must not be misrepresented as
identical to satisfy evidence.

Consequently zero complete checkpoint sets were certified for reuse and zero
fresh readiness evaluations ran. The historical first 29 results per variant
remain intact but are not promoted to C6R-C3 certification. Phase A still
establishes 2 fields, 40 messages, 80 exposures per release and 722/725
all-message closure checks; no final candidate counts or declaration-impact
claims are made. No newly READY service or typed execution is claimed.

Future continuation needs an explicitly justified distinction between semantic
named-dependency witnesses and generated-value cycle-edge validation using
the existing production abstract-value topology APIs. Do not weaken production
cycle rejection or silently add reverse inheritance to semantic reachability.
Atomic job exits, sources and hashes are in `qualification-summary.json` and
`evidence.sha256`. No workers remain active. Task 094 cannot yet proceed to
original-baseline certification or Task 092 integration.

## C6R-C2 — ownership repaired; rotated known-cycle classification failed

Disposition: **QUALIFICATION_FAILED**. No production changes, commit,
integration, push, or PR. Original-baseline HEAD remains
`624364c4f8f6212edee3afc04e331c4064202659`. Earlier C6R/C6R-C1 evidence is intact.

Fresh scratch: `/home/zboll/task094-scratch/c6r-c2`. E0382 was repaired narrowly:
`codec_ready = codec.is_ready()` and
`codec_blocker = codec.blocker.as_ref().map(ToString::to_string)` are recorded
without consuming `codec.blocker`. Initial formatting requested a short-form
conditional; formatting was applied and the final fmt check passed. Full
scratch unit suite passed **2 tests**, exit 0, including READY/NOT_READY,
known-cycle classification and unexpected integrity errors. The standalone
synthetic direct/indirect/negative/shared/cycle probe also completed exit 0.

Distinct optimized current and baseline executables were built against the
Task 094 worktree libraries and archived original baseline libraries,
respectively. Neither executable reran the original twelve CLI measurements.
Each loaded each pinned root once and finished both-release Phase A before
candidate readiness. Atomic attempt and result records identify every
evaluated candidate before and after readiness calls.

### Completed Phase A

Both releases contain exactly two repeated named Duration fields, zero local
and effective facets, OAM namespace, `0..unbounded`, referring to full-QName
`DurationType`:

| Owner/member | 2.5 source line | 2.6 source line |
|---|---:|---:|
| PRI_DataBaseType.PRI_PatternInterval | 75113 | 75185 |
| ScanDataBaseType.ScanPeriod | 91748 | 91832 |

Both releases have **40 distinct reaching messages**, **40 exposures per
owner**, **80 message/owner pairs**. Full deterministic witnesses and separate
member edges are preserved per variant in `uci25-message-reachability.json`
and `uci26-message-reachability.json`. Witness adjacency checks passed.
Independent service-plan membership cross-checks covered all **722** UCI 2.5
and **725** UCI 2.6 messages, including absent exposures, with no mismatch.
These aggregates were persisted before Phase B in
`reachability-cross-check.json`.

The six actual recorded RDMA/Log/Survivability contracts select neither owner
in either release. Neither owner occurs as generated-support-only in those
projections. Exact messages, selected closures, contract hashes, and negative
findings are in each variant's `selected-service-reachability.json`.

### Exact Phase B failure

Both `jobs/current/result.json` and `jobs/baseline/result.json` record completed
exit **101**. Each has 30 UCI 2.5 candidate results: **27 NOT_READY**,
**2 PROJECTION_ERROR**, **1 UNEXPECTED_ERROR**. The last candidate is full-QName
OAM `Response`, ordinal 30/40. Its actual production topology error is:

```text
QueryMatchType -> QueryPET -> QueryType -> QueryMatchType
```

The full names and exact typed
`Projection(ProjectedEmissionPlan(CodegenError { ... }))` are preserved in
`25-Response-current/result.json` and `25-Response-baseline/result.json`.
The scratch classifier accepted only the ordered path beginning `QueryPET`;
it rejected this rotation of the same previously documented closed cycle.
This is a scratch cycle-classification defect, not evidence of a new codec
regression. A future correction must recognize rotations of the exact known
directed cycle while still validating every edge and rejecting unknown
topologies; matching any diagnostic containing `cyclic` is not acceptable.

No retry occurred after this mandatory candidate failure. The remaining ten
2.5 candidates and all forty 2.6 candidates lack Phase B outcomes. Complete
before/after comparisons, declaration impact, representative CLI parity,
and new-READY analysis are therefore not qualified. No newly READY service
or real-UCI typed execution is claimed. `qualification-summary.json` and
`evidence.sha256` preserve current status/hashes. No qualification worker
remains running; original-baseline certification cannot yet advance.

## C6R — semantic reachability partial evidence, candidate gate failed

Disposition: **QUALIFICATION_FAILED**. HEAD remains
`624364c4f8f6212edee3afc04e331c4064202659`; no commit, integration, push or PR.
The original C4 campaign completed 12/12 measurements with atomic exit zero.
Its individual exit-one results are expected codec NOT READY outcomes; those
measurements were verified and not rerun. C4 stopped-source hashes matched
at the beginning of C6R. No original worker remained active.

Scratch-only Rust harness and locks:
`/home/zboll/task094-scratch/c6r/harness`. Production source and frozen oracles
were not modified. The harness uses `load_schema_set`,
`static_dependency_exposures`, `direct_named_dependencies`,
`resolve_service_plan`, `selected_type_closure`, and production projection/
readiness APIs. It loads each version once, rather than invoking CLI per
message. Input source hashes and Rust 1.95 identity are recorded in
`c6r/input-source-hashes.json`.

Synthetic validation completed exit **0** in `c6r/jobs/synthetic/result.json`:
`SYNTHETIC_REACHABILITY_PASS direct=1 indirect=1 negative=1 cycle=1 shared=1`.
It checks direct/indirect/negative message exposure, named-edge witnesses,
shared references, valid Record-reference cycles, and selected closure.

For UCI 2.5, the pinned root hash was verified before its only parse. Normalized
IR inventory finds exactly two relevant repeated fields, both named
`{https://www.vdl.afrl.af.mil/programs/oam}DurationType`, zero effective/local
facets, `0..unbounded`, OAM wire namespace:

- `PRI_DataBaseType.PRI_PatternInterval`, source line 75113.
- `ScanDataBaseType.ScanPeriod`, source line 91748.

Each owner has **40** static message exposures: **80 message/owner pairs over
40 distinct messages**. Full-QName lists, deterministic payload-to-owner
witnesses and separate final member edges are preserved in
`c6r/results/uci25-message-reachability.json`; inventory/provenance are in
`uci25-repeated-temporal-inventory.json`. Every witness adjacency was checked
against `direct_named_dependencies`; duplicate pairs were rejected. The
independent all-message `selected_type_closure` membership cross-check completed
without mismatch before candidate analysis. Its final aggregate artifact was
not written because the harness failed later; do not count a missing artifact
as completed final evidence.

Mandatory detached command:

```sh
CARGO_TARGET_DIR=/home/zboll/task094-scratch/c6r/target \
  cargo +1.95.0 run \
  --manifest-path /home/zboll/task094-scratch/c6r/harness/Cargo.toml \
  -- /home/zboll/task094-scratch/c6r/results
```

Working directory: `/home/zboll/git/ams-gra-codegen-oms-task094`.
Atomic result `c6r/jobs/reachability/result.json`: completed exit **101**.
At scratch `src/main.rs:57:113`, the candidate-readiness `unwrap()` encountered:

```text
Projection(ProjectedEmissionPlan(CodegenError { message: "cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET" }))
```

This is a production projection boundary mishandled by the evidence harness,
not a failed temporal decoder. Candidate identity was not logged before the
call and cannot be attributed from this panic alone. A future continuation
must record the selected message and this actual projection result instead
of assuming readiness always returns an ordinary report; do not weaken the
cycle guard. No retry was launched after the failure.

UCI 2.6 was not reached. Selected-service aggregate evidence, complete
candidate baseline/current comparisons, exact declaration-level effects, and
final cross-check/qualification summaries are missing. No newly READY service
or typed real-UCI execution is claimed. Original-baseline certification and
Task 092 integration remain blocked pending complete evidence.

## Latest disposition — C4 EXECUTION_INTERRUPTED

No commit, push, or PR. HEAD remains
`624364c4f8f6212edee3afc04e331c4064202659`. All C1/C2/C3 evidence remains.
C4 evidence: `/home/zboll/task094-scratch/c4`.

### Corrected rejection-stage mapping

Production CLI proof (`c4/stage-proof`) established repeated `xs:double`
is model READY: model check/generation exit 0 with four actual Ada files.
Codec check/generation exit 1 at
`AdaPayload.Note: unsupported Ada codec primitive or facets: Float64`,
without codec output. Direct repeated `xs:time` instead fails model preflight:
`unsupported selected types:`, model `status: NOT READY`, and
`codec: not measured (selected model is NOT READY)`.

CLI now keeps distinct `repeated-time-model-preflight` and
`repeated-float64-codec-readiness` cases. The latter explicitly checks and
generates the model before asserting codec failure. Existing floating,
unsigned, unqualified and package-collision negatives remain.

090R/091R use `scripts/task094_stage_controls.py` to record explicit stages,
argv/cwd/exit, exact diagnostic, input SHA-256, and output existence for each
invocation. Codec negatives must first pass model-only check/generation.
Model negatives require the model-NOT-READY/codec-not-measured report, not
merely any failure. Frontend controls require their construct diagnostic.

| Former identity | Replacement/current stage | Diagnostic |
|---|---|---|
| direct-bounded/direct-unbounded repeated DateTime | model-direct-bounded-time/model-direct-unbounded-time, model-preflight | codec not measured, selected model NOT READY |
| named-repeated DateTime | named-repeated-unsupported-profile, model-preflight (`.*Z`) | unsupported selected types, codec not measured |
| repeated direct/named Duration, bounded | invalid minimum 32768, frontend | minOccurs exceeds maxOccurs |
| repeated direct/named Duration, unbounded | nonportable minimum, model-preflight | unsupported selected types, codec not measured |
| additional repeated Float64 | codec-readiness | CombinedPayload.Values: unsupported Ada codec primitive or facets: Float64 |

Direct/named DateTime/Duration Choice alternatives still reject at codec
readiness with their original exact diagnostics. Unsupported named facets,
Duration pattern, and anonymous inline type controls remain. C3's 12-cell
positive executions justify the supported repeated temporal shapes, not
deletion of neighboring negatives. Missing/cyclic and unqualified/nillable
controls elsewhere remain unchanged; no frontend validator changed.

### Completed C4 focused gates

Atomic result records under `c4/jobs` establish:

- CLI focused test: 1 passed, exit 0.
- CLI full codec suite: 18 passed, exit 0.
- Original Task 089 core test: 1 passed, exit 0.
- Core codec module: 21 passed, exit 0 (filtered tests excluded).
- 090R GNAT 14/16: all policies PASS, 8 negative controls each.
- 091R GNAT 14/16: all policies PASS, 15 negative controls each.
- Rust 1.95 formatting, locked workspace all-target check, and
  warnings-denied Clippy: completed exit 0.

### Expanded executable qualification

Added independently authored minimum JSON and 12 checks covering all four
empty zero-minimum collections, bounded positive minima, required prefixes,
empty Additional storage, and canonical omission of explicit empty arrays.
The main codec corpus now reports **54 total checks, 17 rejections** (37
non-rejection checks), not 54 positives plus 17.

Added separate composition schema/golden/consumer covering inherited named
DateTime arrays, nested direct DateTime and positive-minimum named Duration,
Choice selecting a concrete temporal Record, mixed temporal values and
single-valued DateTime/Duration preservation. Later-element invalid values
in inherited/nested/Choice paths assert both no publication and `item 2`
diagnostic. This reports **19 total checks, 3 rejections** (13 positive checks
and 6 rejection-related checks).

Both expanded and composition campaigns passed all **12 cells each**:
GNAT 14/16, OAM/qualified, Normal/Check/Ignore. Atomic exit-zero records:
`expanded14`, `expanded16`, `composition14`, `composition16`.
Generated source hashes, pinned locks, identities, model/API parity and
helper uniqueness passed. The composition runner's final summary line still
prints main-corpus counts; actual per-cell composition markers are 19/3 and
must be used, not that inaccurate summary. This needs correction before final
qualification/publication. No production generator changes were made in C4.
The C3 six-cell 24-check model checkpoint remains historical, not rerun in C4.

### Byte preservation, inherited work, CI, and interruption

Built exact baseline from a git archive into independent baseline-target.
`c4/bytes/comparison.json` establishes **25 successful byte-identical
generations**, zero matching rejections in that sample: five existing fixtures
across Ada model/API and codecs, Rust model/API and codecs, and C++ model/API.
Atomic rejection comparisons and explicit unnecessary-helper controls remain
outstanding.

Inherited serial batch completed GNAT 14 087A/B, 088, 089/089R, 090 and 091
with exit 0. At the final inspection it was continuing GNAT 16 087A.
Full workspace tests were still running compiler probes, with no observed
failure; no aggregate pass count is claimed. Pinned UCI root hashes were
verified and inventory/readiness job started; baseline first selection was
still executing, so no UCI result/improvement is claimed. These detached jobs
remain preserved and must be inspected to definitive completion before any
future qualification claim. They were not killed or silently counted passed.

Added Fast job `ada-repeated-temporal-codec`, depending on ada-json-guard,
exact PR-head checkout, production build, both expanded/composition GNAT 16
campaigns, six exact markers each, hashes and full evidence upload. Existing
jobs and deep-ci.yml are unchanged. Initial CI gates passed, including 209
mutation checks; the new job's check-ci-split gate also passed.

**C4 interruption:** the final foreground
`bash scripts/test-check-ci-split.sh` invocation exceeded the tool's 30-second
limit, leaving `c4/ci-final-mutations.log` without an exit record. No remaining
mutation process was found. This is incomplete, not passing evidence.
The planned final 077B layout/whitespace commands in that shell were not
reached. No further qualification was launched after observing interruption.
Use the detached wrapper for the final mutation retry in a fresh attempt.

Disposition is **EXECUTION_INTERRUPTED**, not a demonstrated assertion failure.
Remaining requirements include definitive running-job results, final-source
reruns after runner fixes, full inherited/workspace completion, UCI results,
remaining byte/atomicity controls, final CI integrity, remote freshness and
publication. There is no final Task 094 commit/PR or hosted Fast/Deep run.
Task 095 was not started. Sibling PRs/worktrees were not modified.

Disposition: **QUALIFICATION_FAILED**. Changes are uncommitted and unpublished.
No PR was opened, no branch was pushed, and no sibling worktree was modified.

## Verified baseline and isolation

- Fetched starting main: `624364c4f8f6212edee3afc04e331c4064202659`.
- Repository remote: `git@github.com:zackboll/ams-gra-codegen-oms.git`.
- Branch: `feature/094-ada-repeated-temporal-codec`.
- Worktree: `/home/zboll/git/ams-gra-codegen-oms-task094`.
- Evidence: `/home/zboll/task094-scratch`.
- PR #93 was OPEN, unmerged, at
  `09a1b2e630acfda7e971bbe2524f3763a54e6b50`.
- PR #94 was OPEN, unmerged, at
  `99f8b65273f4e7b46aeb2c0bba839adfe1c31941`.
- No existing Task 094 branch, worktree, or PR was found before creation.
- Main contains merged Task 091 and its 091R runner/reconciliation evidence.

## Model-first checkpoint

The actual Rust 1.95 locked production generator emitted the model without
`--with-codec`. The task-owned XSD covers direct DateTime 0..3/0..*, named
Zulu DateTime 1..3/2..*, direct Duration 1..3/2..*, and named zero-facet
Duration 0..3/0..*.

The consumer constructs through `Create`, inserts through `To_Sequence` and
`Append`, reads ordered lexical values through `Element`/`Value`, inspects
the positive-minimum `Required`/`Additional` storage, and rejects four invalid
constructor inputs. It uses explicit checks, not policy-dependent assertions.

All six model build/execution combinations returned zero and produced
`TASK094_MODEL_COMPLETE checks= 24`: GNAT 14.2.0 and 16.1.0, each with normal,
Check, and Ignore policies. Alire is 2.1.1; GPRBUILD release is 26.0.1 with
executable banner 26.0.0. The model-only checkpoint does not require GNATCOLL.
No generated source was patched. No model renderer/API change was made.

Generation command and output, dependency locks, per-command exit records,
generated Ada, and generated model/API SHA-256 values are preserved under
`/home/zboll/task094-scratch/model-first`.

Evidence limitation: the ad-hoc checkpoint script used `Path.with_suffix`
on compiler-version-prefixed log stems, overwriting individual build/run
logs with the final identity log for each compiler. Full command/cwd/exit
JSON records remain separate and the execution tool transcript records each
successful invocation. The initial normal execution log is also preserved.
This is not the required final qualification runner and must be replaced
with collision-free logging before claiming final provenance qualification.

## Provisional codec changes (not qualified)

The core repeated-member path now calls the existing direct scalar temporal
classifier, and named repeated values must resolve to emitted declarations
and pass memoized declaration support. Existing cardinality, field-local
facet, closure, and Choice-alternative restrictions remain.

The Ada repeated generator explicitly encodes direct DateTime/Duration using
`Create (Model.Value (Element_Item))` and decodes with `Scalar_Date_Time` or
`Scalar_Duration`, before numeric fallback. Named dispatch is unchanged.
No parser, constructor exception scope, or emission plan was added.

The intended mapping remains ordered JSON arrays of lexical JSON Strings,
with the established zero-minimum empty omission policy. Codec execution,
canonical omission, helper emission, and failure atomicity are NOT qualified.

## Mandatory local failure and stop

Command (working directory `/home/zboll/git/ams-gra-codegen-oms-task094`):

```sh
CARGO_TARGET_DIR=/home/zboll/task094-scratch/target \
  cargo +1.95.0 test --locked -p ams-gra-oms-codegen-core service_codec
```

Exit **101**, **20 passed, 1 failed**, 336 filtered. Exact evidence:
`/home/zboll/task094-scratch/core-tests.log`.

Failure: `task089_repeated_cardinalities_and_element_boundaries`, at the
assertion that every primitive in its unsupported repeated-kind list must
fail readiness. The list still contains DateTime and Duration. These obsolete
expectations must be replaced with unsupported neighboring temporal profiles,
not simply deleted. The existing 090R core test was provisionally changed to
accept repeated direct DateTime and reject a local length facet, retaining
its direct DateTime Choice-alternative rejection.

Additional obsolete expectations identified but NOT yet reconciled:

- `/home/zboll/git/ams-gra-codegen-oms-task094/scripts/check-task090r.py`:
  direct-bounded, direct-unbounded, named-repeated controls.
- `/home/zboll/git/ams-gra-codegen-oms-task094/scripts/check-task091r.py`:
  those same DateTime controls plus direct/named Duration bounded/unbounded
  controls.

Stopped on mandatory local qualification failure as requested. The current
generator builds successfully, but a successful build does not qualify codecs.
An earlier baseline build invocation used a nonexistent package name; its
failure is preserved separately in `baseline-build.log`, and the corrected
baseline build succeeded in `baseline-build-corrected.log`.

## Other gates and outstanding work

Initial unchanged CI integrity gates passed:

- `/home/zboll/git/ams-gra-codegen-oms-task094/scripts/check-ci-split.sh`.
- `/home/zboll/git/ams-gra-codegen-oms-task094/scripts/test-check-ci-split.sh`
  (209 checks).
- `/home/zboll/git/ams-gra-codegen-oms-task094/scripts/check-task077b-ci-layout.py`
  against `.github/workflows/deep-ci.yml`.

Whitespace verification passed before this report was added.

No final Task 094 qualification runner, JSON goldens, executable codec
consumer, hosted job, real UCI inventory/readiness comparison, baseline/current
Ada/Rust/C++ byte comparison, or inherited compiler regression campaigns
were completed. No full workspace check/test/Clippy qualification is claimed.
No final commit SHA, PR URL/base/head, or hosted run identity exists.
No real UCI readiness improvement or typed execution is claimed.
The fixture is the minimal model checkpoint, not the final composition corpus.
Deep CI was not modified. Task 095 was not started.

## C1 continuation — Phase A corrected, executable campaign stopped

Disposition remains **QUALIFICATION_FAILED**; no commit or push was made.
Original evidence above remains unchanged. New evidence is isolated under
`/home/zboll/task094-scratch/c1`.

The exact Task 089 blanket unsupported list no longer contains DateTime or
Duration. All other unsupported primitive assertions remain. Both temporal
kinds are positively tested at 0..3, 2..3, 2..2, 0..*, and 2..*. Each of those
profiles also rejects a field-local length constraint. Existing invalid
cardinalities (inverted range and nonportable bounded/unbounded limits) now
reject both temporal kinds as well as the original String case.

Immediate Rust 1.95 locked Phase A results:

- Exact Task 089 test: **1 passed**, exit 0 (`phase-a-focused.log`).
- Full core codec module: **21 passed**, exit 0 (`phase-a-module.log`).
- Filtered tests are not counted as qualification.

The repository diagnostic audit is preserved in `stale-expectations.txt`.
Besides the known 090R/091R runner controls, it identified an obsolete CLI
expectation in `crates/cli/tests/service_codec.rs`. These runner/CLI controls
have NOT yet been modified. Historical documentation remains unchanged.
Direct and named temporal Choice restrictions remain intact.

### C1 executable fixture and runner

Added an independently authored structural JSON golden, a production-codec
Ada consumer, and `scripts/check-task094.py`. The consumer checks actual typed
temporal values, order, Required/Additional elements, supported timezone forms,
structural output equality, encode/decode/encode stability, and 17 malformed
input rejections without publishing typed results. Only `Codec_Error` is
accepted for malformed codec input. Model/API byte parity and repeated-only
direct helper uniqueness are checked by the runner.

Evidence naming now appends `.log` and `.command.json` to the complete stem,
not `Path.with_suffix`. Stems identify profile, compiler, policy, consumer,
and operation. Existing evidence paths are refused rather than overwritten.
The original checkpoint files are untouched.

The GNAT 14 OAM profile passed normal/Check/Ignore compilation and execution,
each with **42 checks, 17 rejections** and exact marker:

```text
TASK094_CODEC_COMPLETE checks= 42 rejections= 17
```

Pinned GNATCOLL/Alire solution validation and GNAT/GPRBUILD identity checks
passed for these three executions. Model/API parity passed. Both direct
helpers occur exactly once in this repeated-only fixture.

### Exact C1 mandatory failure

Command (worktree `/home/zboll/git/ams-gra-codegen-oms-task094`):

```sh
python3 scripts/check-task094.py --compiler 14 \
  --scratch /home/zboll/task094-scratch/c1/codec14-attempt1 \
  --generator /home/zboll/task094-scratch/target/debug/ams-gra-codegen-oms
```

Runner exit **1**. The qualified non-OAM model-only generation subprocess
exited **1**, before compilation or codec readiness:

```text
error: unsupported XSD construct: xs:element @version at /home/zboll/task094-scratch/c1/codec14-attempt1/qualified.xsd:19:2
```

The runner replaces the OAM namespace everywhere in the schema, inadvertently
moving the recognized `uci:version` extension annotation into the new namespace.
This is a frontend-stage runner-fixture failure, not a codec decoder failure
and not a model storage blocker. No expected-count assertion failed.
The qualified fixture transformation must preserve the recognized annotation
namespace independently of the model target namespace on a future continuation.

Precise subprocess command/cwd/exit and diagnostic:

- `c1/codec14-attempt1/qualified-gnat14.2.0-plain-generate.command.json`.
- `c1/codec14-attempt1/qualified-gnat14.2.0-plain-generate.log`.
- Campaign output: `c1/codec14-attempt1.log`; exit: `.exit`.

Stopped at this mandatory local failure without retrying or publishing.
GNAT 16 codec execution and qualified non-OAM executions were not run.
The current corpus still lacks the required inherited/nested/Choice composition,
minimum-length/canonical-omission and normalization coverage; it is not a final
Task 094 qualification claim. Inherited campaigns, final CI registration,
real-UCI measurements, baseline/current non-temporal byte comparisons, and
hosted qualification remain outstanding. The original model-first six-way
success is historical model evidence, not GNAT 16 codec evidence.

## C2 — namespace correction passed; matrix attempts interrupted

Disposition: **QUALIFICATION_FAILED** (mandatory matrix incomplete after
execution-tool timeouts). No commit, push, or PR was made. All original/C1
evidence is preserved. New evidence: `/home/zboll/task094-scratch/c2`.

Only the runner was changed in C2; production frontend/model/codec code was
not changed. The qualified transformation now separately replaces exactly one
target namespace, one `xmlns:uci` binding, one schema attribute insertion
point, and one version-attribute prefix. It introduces `xmlns:oam` and retains
the version expanded QName
`{https://www.vdl.afrl.af.mil/programs/oam}version`, with its original value
`001.000.000.000`. Local `uci:` type references resolve to Task 094 types.

Before generation the runner verifies prefix bindings, global-message
version presence/value/QName, local type-reference resolution, unchanged
node counts, parsed-tree equality after reversing the target namespace,
and exact original source bytes after reversing all intentional substitutions.
This preserves restrictions, cardinalities, declaration order, and text.
The namespace manifest records both fixture SHA-256 values, replacement
counts, element/type counts, references, and versions. Evidence:
`c2/qualified14-retry/qualified-gnat14.2.0-namespace-conversion.json`.

The isolated qualified retry passed model-only generation, codec generation,
model/API byte parity, both repeated-only scalar helper uniqueness checks,
pinned dependency validation, compiler/build-tool identities, GNAT 14.2.0
normal-policy compilation, and actual execution. Exact marker remained
`TASK094_CODEC_COMPLETE checks= 42 rejections= 17`.
The golden and consumer use qualified member keys, not version metadata.
The exact structural-golden equality assertion passed.

### Frontend namespace controls

Four expected frontend rejections passed, each exit 1 with no generated output:

| Control | Commands | Required diagnostic |
|---|---|---|
| Version moved into Task 094 namespace | service-check, service-generate | `unsupported XSD construct: xs:element @version` |
| Required version removed | service-check, service-generate | `missing required UCI version attribute` |

Exact commands, cwd, exits, and diagnostics are separate files under
`c2/namespace-controls`. The frontend's annotation policy was not relaxed.

### C2 matrix interruption and stop

Commands from `/home/zboll/git/ams-gra-codegen-oms-task094`:

```sh
python3 scripts/check-task094.py --compiler 14 \
  --scratch /home/zboll/task094-scratch/c2/matrix14 \
  --generator /home/zboll/task094-scratch/target/debug/ams-gra-codegen-oms
python3 scripts/check-task094.py --compiler 16 \
  --scratch /home/zboll/task094-scratch/c2/matrix16 \
  --generator /home/zboll/task094-scratch/target/debug/ams-gra-codegen-oms
```

Both foreground tool commands returned `Command timed out after 30000ms`.
No runner processes remained at inspection. The shell did not reach its
campaign exit-record write, and the interrupted subprocess has no completed
command/log pair. These attempts are incomplete, not successful qualification
and not observed codec assertion failures. Preserve these directories; do not
resume by overwriting evidence. Future full campaigns must run in background
with redirected output and a captured completion status.

Completed C2 execution cells (42 checks / 17 rejections each):

| Compiler/profile | Normal | Check | Ignore |
|---|---|---|---|
| GNAT 14.2.0 OAM | PASS | PASS | PASS |
| GNAT 14.2.0 qualified | PASS (isolated retry) | not run | not run |
| GNAT 16.1.0 OAM | PASS | PASS | interrupted |
| GNAT 16.1.0 qualified | not run | not run | not run |

The established 42 checks include 17 rejection-publication assertions; they
are not 42 separate positive-value assertions plus 17 additional assertions.
The 24-check model-only six-way checkpoint was not rerun in C2.

The full 12-cell codec matrix, composition/minimum/normalization corpus,
model-only rerun, historical 090R/091R/CLI reconciliation, inherited compiler
campaigns, Rust workspace gates, real UCI measurements, baseline/current
generated-byte comparisons, CI job, remote freshness/integration, and hosted
qualification remain outstanding. No final Task 094 commit, PR head/base/URL,
or hosted Fast/Deep run exists. The branch HEAD remains
`624364c4f8f6212edee3afc04e331c4064202659`.

## C3 — detached matrices passed; reconciliation diagnostic failed

Disposition: **QUALIFICATION_FAILED**. No commit, push, or PR. Original/C1/C2
failures and timeout directories remain untouched. New evidence is under
`/home/zboll/task094-scratch/c3`.

### Detached execution correction

The scratch-only launcher/worker is `c3/runner/detach.py`. The launcher starts
a persistent Python worker with `start_new_session=True`, DEVNULL stdin,
separate redirected stdout/stderr, and closed inherited descriptors. The
worker waits for the subprocess and atomically renames its final JSON record
only after termination. Each new job directory is exclusively created,
preventing duplicate launch/overwrite. Its manifest records argv/cwd,
environment, Python identity, HEAD, repository input hashes, launcher/worker
PIDs, sessions, timestamps, output paths, actual exit, and final source hashes.
Missing results are not interpreted as success.

Self-tests in `c3/jobs/self-success` and `self-failure` completed with recorded
exits **0** and **7**. Duplicate launch was refused (`duplicate-control.log`).
Distinct worker/launcher session IDs and a worker reparented after launcher
exit demonstrate independence. Both long compiler campaigns outlived the
30-second foreground limit and completed normally.

### Definitively completed C3 matrices

`c3/jobs/matrix14/result.json` and `matrix16/result.json` both report completed
exit **0**. Every compiler/profile/policy cell compiled and executed:

| Compiler | OAM normal/Check/Ignore | Qualified normal/Check/Ignore |
|---|---|---|
| GNAT 14.2.0 | 3/3 PASS | 3/3 PASS |
| GNAT 16.1.0 | 3/3 PASS | 3/3 PASS |

Each of 12 execution logs contains exactly
`TASK094_CODEC_COMPLETE checks= 42 rejections= 17`. This means **25 non-rejection
checks plus 17 rejection/publication checks**, not 42 positives plus 17 more.
The source/hash/lock/helper/parity checks in the runner passed. Evidence lives
in fresh `c3/matrix14` and `c3/matrix16`, not reused timeout directories.

`c3/jobs/model/result.json` reports completed exit **0**. The separate scratch
model runner generated actual models, compared all model/API Ada bytes with
the original model-first output, compiled/executed GNAT 14/16 under all three
policies, and verified all six exact 24-check markers. Completion:
`TASK094_MODEL_MATRIX_COMPLETE cells=6 checks=24`.

### Provisional historical reconciliation and exact mandatory failures

After all 12 codec cells passed, obsolete repeated direct DateTime negatives
in 090R/091R were replaced provisionally by repeated Time neighbors. The named
DateTime control uses a repeated named declaration with unsupported `.*Z`
profile. Duration bounded/unbounded controls now use invalid minimum 32768:
bounded inverted cardinality should reject in frontend, unbounded nonportable
storage should reject in model preflight. All temporal Choice and anonymous
type controls remain. CLI's obsolete repeated DateTime case was likewise
replaced provisionally with repeated Time. No historical report/oracle changed.

These modifications are NOT qualified. Repeated Time is rejected earlier than
the expected codec diagnostic: the selected model is NOT READY, so codec
analysis is not measured. The next continuation must preserve the negative
case but correct its stage-specific expected diagnostic based on this evidence.

Detached focused command:

```sh
cargo +1.95.0 test --locked -p ams-gra-oms-codegen-core \
  -p ams-gra-oms-backend-ada -p ams-gra-codegen-oms --test service_codec
```

`CARGO_TARGET_DIR=/home/zboll/task094-scratch/target`, worktree cwd.
Atomic job result exit **101**. CLI results: **17 passed, 1 failed**.
Failing test `task087b_ada_codec_unsupported_and_collision_preflight_is_atomic`,
assertion at `crates/cli/tests/service_codec.rs:835`, label
`repeated-unsupported-time`. Actual report includes `unsupported selected types:`
and `codec: not measured (selected model is NOT READY)`, not the expected
`unsupported Ada codec primitive or facets: Time`.
Exact argv/logs/source hashes: `c3/jobs/focused-tests/{manifest,result}.json`,
`stdout.log`, `stderr.log`. This targeted integration command does not count
the full core/backend unit suites as run.

A 090R GNAT 14 job launched before observing that failure also completed exit
**1**, without compilation, at the same stage mismatch:
`direct-bounded-time: service-check did not reject atomically with unsupported
Ada codec primitive or facets: Time`.
Evidence: `c3/jobs/reconcile090r14/result.json`, `stderr.log`, and
`c3/reconcile090r14/direct-bounded-time-service-check.*`.

Both jobs are definitively completed, not interrupted. No further qualification
was launched after observing failure. No task qualification worker remains
running. No failed expectation was relaxed after the mandatory stop.

### Remaining qualification

The 12-cell corpus covers all eight storage shapes but still does NOT cover
the full requested minimum/empty/normalization/inherited/nested/Choice and
single-valued composition matrix. No expanded count is claimed. Historical
runner reconciliation and full inherited campaigns remain incomplete. Full
Rust workspace formatting/check/test/Clippy, UCI inventory/readiness comparison,
baseline/current Ada/Rust/C++ preservation, new CI job, remote freshness,
publication and hosted CI remain outstanding. Only model/API checkpoint parity
and per-profile with/without-codec parity are established here.

Exact input and final repository source SHA-256 maps are preserved in each
detached result record; these qualify only that job's inputs, not a future
revision. There is no final Task 094 commit or PR URL/base/head. HEAD remains
`624364c4f8f6212edee3afc04e331c4064202659`. Task 095 was not started.