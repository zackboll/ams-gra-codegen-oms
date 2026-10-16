# Task 091R — incomplete reconciliation, qualification failed

Disposition: **QUALIFICATION_FAILED**. No commit or push was made. PR #92 was
not merged, approved, replaced, or configured for auto-merge.

## Verified baseline and isolation

- Original and still-current published PR HEAD:
  `ead6e7e905e434b9c29e7517bba991768d13006d`.
- Fetched integration main: `13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`.
- Original Task 091 base: `dd1353a93f4e1e62fa544f7fb7349a6eba41fc03`.
- Dedicated worktree: `/home/zboll/git/ams-gra-codegen-oms-task091`, initially
  clean, on `feature/091-ada-duration-codec`.
- Fresh evidence: `/home/zboll/task091r-scratch`. Historical evidence in
  `/home/zboll/task091-scratch` and the original Task 091 document is unchanged.
- Remote PR inspection: OPEN, non-draft, auto-merge null, CONFLICTING/DIRTY.

An ordinary `git merge --no-commit --no-ff origin/main` was started. Textual
conflicts occurred in the core readiness analyzer and Ada codec generator.
The CLI tests and CI workflow merged automatically. The merge remains
uncommitted and its conflicted index entries remain unresolved; do not treat
the working files as a completed or certified merge.

## Provisional implementation

The working files combine main's Choice/repeated traversal and memoization
with the Duration temporal-profile branches and DateTime scalar handling.
Both scalar helper names are reserved and their emission remains conditional
on the emission plan. Duration remains a String constructor/value API, with
constructor-scoped Constraint_Error conversion. Direct/named Duration scalar
Choice alternatives and repeated Duration scalars are explicitly rejected as
not qualified; Record-contained Duration is the proposed qualification scope.

One obsolete DateTime test expectation that Duration is unsupported was
removed. A reciprocal obsolete expectation remains in the Duration test and
caused the affected test run to fail (see below). Semantic review is incomplete.

Additive draft fixture files are `codec-ada-duration-combined.xsd/.yaml` and
`task091r-combined.json` under `tests/fixtures/service-generate`. The draft
`duration_combined_consumer.adb` extends the DateTime combined consumer with
Duration lexical expectations, inherited/nested fields, optional presence,
Choice payloads, repeated Record order, and rejection of invalid Duration in
later repeated elements. The draft `scripts/check-task091r.py` adds negative
controls, model/API byte parity, helper checks, compiler/lock identities,
three assertion policies, and source hashing. These are **not qualified**.

## Executed checks and stop evidence

- Rust 1.95.0 locked production-generator build: PASS, including after the
  provisional integration; `build.log` and `build-integrated.log`.
- Rust formatting applied; initial formatting check requested changes.
- Affected core test command: `cargo +1.95.0 test --locked
  -p ams-gra-oms-codegen-core service_codec`, exit 101: 20 passed, 1 failed,
  336 filtered. Failure: `task091_duration_uses_only_zero_facet_profiles`
  still expects direct DateTime to be rejected. Evidence: `core-tests.log`.
  Filtered tests are not counted as qualification.
- Draft Task 091R GNAT-16 runner, exit 1: readiness, codec generation, and
  model-only generation passed. Model/API byte comparison passed and is
  recorded in `integration16/model-byte-parity.json`. The constrained named
  Duration control expected `unsupported selected types:` but frontend
  rejected `xs:pattern` earlier. Evidence:
  `integration16/duration-named-facets-service-check.log` and corresponding
  command/exit records. This is a diagnostic-expectation failure, not evidence
  of relaxed constraints. The runner never reached Ada compilation/execution.
- Frozen Duration oracle SHA-256 remains
  `1f723a55b4c79bc9e879c6d8ee1d7fa1b21d1e1d9b2dd725f763a8b9ab4222d7`.
- `git diff --check`: PASS at stop inspection.

## Explicitly unexecuted gates

No GNAT 14.2.0 or GNAT 16.1.0 executable qualification is claimed under normal,
Check, or Ignore assertion policy. Task 087A/B, 088, 089, 089R, 090, 090R, and
091 regression campaigns were not executed for this reconciliation. The new
091R attempt failed before compilation. Workspace check, warnings-denied
Clippy, full workspace tests, guard negative-control campaign, non-Ada byte
preservation, current-main UCI baseline/after measurements, and final remote
main freshness verification remain unexecuted.

The automatically merged Duration CI job has not yet been extended to execute
091R. No final corrected hosted revision or CI runs exist. Historical successful
runs 38514184826 and 38514184843 certify only the original PR HEAD and are not
combined qualification evidence. No new UCI counts or READY service claims are
made. No final corrective SHA exists; local HEAD and fetched feature/PR HEAD
remain the original SHA, with a pending merge and working-tree changes.

Resume only with an explicit decision to continue correction after this failed
qualification. All mandatory gates remain required before publication.

## Task 091R-C1 continuation — Phase A failed

The explicit continuation request was received. Recovery inspection confirmed
the pending merge, both unresolved index entries, and all six untracked draft
files. Fetch left origin/main and PR HEAD unchanged at the identities above.
No merge abort, reset, branch recreation, or history rewrite was performed.

Two known expectation failures were corrected:

1. Removed DateTime from the Duration test's blanket unsupported primitive
   loop. Duration's zero-facet acceptance and numeric-bound, length, pattern,
   and explicit-whitespace rejection assertions remain for both named and
   direct profiles. Existing Task 090 tests verify the distinct DateTime
   supported and unsupported profile contracts.
2. The new runner expects `unsupported XSD construct: xs:pattern` for its
   constrained-Duration schema. Negative-control records now identify the
   frontend versus capability rejection stage; exact exit 1, narrow diagnostic,
   and no published generation output remain required. No frontend admission
   rule or Duration production profile was relaxed.

The historical CLI fixture was inspected: it contains a direct Duration
Choice alternative and repeated direct Duration scalars, not just Records
containing Duration. It remains intentionally NOT READY. Its obsolete generic
Choice/repetition diagnostic expectation was replaced by the exact first
blocker `IntervalChoice.Every: direct Duration Choice alternative is not
qualified`. Historical fixtures and the frozen oracle remain unchanged.

Fresh evidence is in `/home/zboll/task091r-scratch/c1` and
`/home/zboll/task091r-scratch/c1-integration16`, with the runner transcript in
`/home/zboll/task091r-scratch/c1-integration16.log`.

| Phase A check | Outcome |
|---|---|
| Requested Duration core test | PASS: 1 executed |
| Complete core codec test module | PASS: 21 executed |
| Complete CLI service_codec integration target | PASS: 18 executed, none filtered |
| Ada backend library suite | Launched; no final test summary/exit captured; not certified |
| Combined readiness and generation | PASS |
| Combined model/API byte parity | PASS; model-byte-parity.json |
| End-to-end negative controls | PASS: 14 controls, 28 commands, all exit 1, no output publication |
| Combined GNAT 16.1.0 normal build and compiler/lock identity | PASS |
| Combined GNAT 16.1.0 normal execution | **FAIL**, exit 1, PROGRAM_ERROR: all-four integration regression |
| Combined GNAT 16 Check / Ignore | Not executed after normal execution failure |
| GNAT 14 combined execution | Not executed |
| Python/JSON/XML syntax, Rust formatting, git diff whitespace | PASS |
| Workflow syntax | Not independently validated in C1 |

The executable failure's individual assertion has not been localized. Batch
debugger attempts did not obtain a stack; those transcripts are preserved in
the C1 directory. No production-code defect or oracle defect is inferred from
the generic check message, and no expectation was weakened to bypass it.

Per the Phase A failure stop condition, Phase B, UCI remeasurement, CI updates,
final semantic certification/staging, merge completion, commits, and publication
were not attempted. The merge remains pending with unresolved index entries.
No final corrective SHA or corrected-head hosted runs exist. PR remains OPEN,
non-draft, CONFLICTING/DIRTY, auto-merge null; local/fetched-feature/PR commit
identities still match the original SHA, not a qualified corrective revision.
Disposition remains **QUALIFICATION_FAILED**.

## Task 091R-C2 — localized runtime failure and repair

C2 evidence is isolated in `/home/zboll/task091r-scratch/c2`. Recovery saved
status, unmerged index entries, working and staged diffs before modifications.
The ordinary merge and all C1/historical evidence were preserved.

### Root cause: category A, incorrect integration test setup

The original C1 executable reproduced exit 1 with the generic Program_Error.
A fresh production-generated GNAT 16 build reproduced it; all generated Ada
source bytes matched C1. Source/input/executable hashes and commands are in
`original-hashes.json`, `original-run.command.json`, and `fresh-original16`.
Compiler switches were `-gnat2022 -gnato -O0`, normal assertion policy.

Persistent assertion labels localized the failure to `Validate/check-28`,
now named `Decode/absent-duration`:

```
expected presence FALSE/FALSE; actual TRUE/TRUE
```

The absent-case setup cloned the independent golden, which contains
`OptionalSpan: P1DT02H` and `OptionalDirectSpan: PT01.2300S`, but removed only
the two DateTime optional keys. The assertion incorrectly expected Duration
absence without removing the Duration keys. The generated decoder correctly
uses JSON `Has_Field` to set each optional's `Is_Present`; the schema declares
both fields `minOccurs="0"`. This contract and the observed values are recorded
in `contract-evidence.json` and `observed16/normal-run.log`.

The minimal repair removes both Duration keys before the existing absence
assertion. Additional setup checks verify present keys before removal and
absent keys afterward. No predicate was weakened and no production codec code,
golden JSON, frozen Duration oracle, or historical fixture was changed for this
runtime repair. The labeled pre-repair run proves failure; the repaired run
proves success. Assertions retain stable context and short actual lexical
values. Unexpected exceptions still fail; only Codec_Error is expected by
negative consumer cases.

### Qualification results obtained in C2

| Campaign | GNAT 14.2.0 normal / Check / Ignore | GNAT 16.1.0 normal / Check / Ignore |
|---|---|---|
| 087A guard and boundaries | PASS / PASS / PASS | PASS / PASS / PASS |
| 087B original 41 checks and 7-check subset | PASS / PASS / PASS | PASS / PASS / PASS |
| 088 Choice, 172 + 2 checks | PASS / PASS / PASS | PASS / PASS / PASS |
| 089 repeated Records | PASS / PASS / PASS | PASS / PASS / PASS |
| 089R Choice/repetition | PASS / PASS / PASS | PASS / PASS / PASS |
| 090 named/direct DateTime | PASS / PASS / PASS | PASS / PASS / PASS |
| 090R DateTime composition | PASS / PASS / PASS | PASS / PASS / PASS |
| 091 Duration, frozen corpus 100, codec 650, model 10 | PASS / PASS / PASS | PASS / PASS / PASS |
| 091R all four, 52 checks and 20 rejections | PASS / PASS / PASS | PASS / PASS / PASS |

All 18 campaign exits are zero; `campaign-matrix.json` contains actual commands
and exits. Each compiler uses separate fresh scratch trees and verified lock,
GNAT, GPRbuild, and dependency identities. No inherited runner was weakened.
The 091R runner records commands/cwd/exits, sources and input hashes, checks
both scalar helpers occur exactly once, preserves model/API bytes, and executes
14 rejection controls (28 commands), distinguishing frontend and capability
rejections and checking atomic output failure.

Focused reruns: Duration core 1 passed; complete codec module 21 passed; CLI
codec target 18 passed; Ada backend library **63 passed, zero failed, exit 0**,
with the complete 325-second run captured in `backend.log` and `backend.exit`.
Eight guard wrapper negative controls passed. Python/JSON/XML/YAML syntax,
Rust formatting, whitespace checks, locked workspace all-target check and
warnings-denied Clippy passed. Full workspace execution remains to be recorded
below; launching it is not counted as a pass.

### Semantic review and preservation

The core diff against main preserves effective-field/Choice traversal,
memoization, projected emissions, generated support decisions, and deterministic
first-blocker iteration. Duration uses the named/direct temporal classifiers.
Only direct/named Duration scalar Choice and repeated-scalar contexts are
excluded; Record-contained Duration passes the combined executable campaigns.
Both helper names are reserved and emitted from the same emission plan. Ada
Duration remains the checked lexical String Create/Value API, with narrow
constructor Constraint_Error translation. DateTime branches remain unchanged.
Optional presence, inherited fields, repeated order, Choice discriminants,
wire keys, operation deduplication, and atomic nested failure are exercised.

The original Task 091 negative DateTime assumption and reciprocal Task 090
negative Duration assumption were obsolete; profile-specific tests replace
neither with unconditional temporal acceptance. The historical mixed Duration
fixture still rejects its unqualified direct Duration Choice alternative.

`byte-preservation.json` records **28** baseline/current selections: Ada model
and API bytes plus Rust/C++ model/API and Rust codecs, with identical hashes
and statuses. Combined model/API bytes with/without codecs are independently
checked by 091R. Baseline generator was built from a git archive of verified
main, not the old Task 091 base. Frozen oracle hash remains unchanged.
Inherited backend model changes equal main exactly; inherited runners equal
main (091 equals original HEAD). The four inherited Ada CI jobs remain exactly
equal to main. Duration CI now additionally runs 091R and uploads its evidence.

### Current-main UCI remeasurement

The pinned UCI root hashes were reverified unchanged:
2.5 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`;
2.6 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
Exact selections, commands, root hashes, reports, and exit 1 records are in
`parallel-uci-comparison.json`. Main baseline is
`13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`.

| Selection | Selected types | Codec declarations main → corrected | First blocker main → corrected |
|---|---:|---|---|
| RDMA_InitializeSetup 2.5 | 50 | 47/49 → 47/49 | RDMA_InitializeSetupMDT.Endpoint repeated Choice → same |
| RDMA_InitializeSetup 2.6 | 50 | 47/49 → 47/49 | RDMA_InitializeSetupMDT.Endpoint repeated Choice → same |
| Log 2.5 | 42 | 39/41 → 39/41 | LogMDT.ServiceUpTime Duration → LogMDT.Data Binary |
| SurvivabilityRiskLevel 2.5 | 45 | 40/44 → 43/44 | DurationType Duration → SecurityInformationType.OwnerProducer repeated Choice |

All four selections remain **codec NOT READY**. Log's direct Duration support
does not make the whole Record renderable because Binary remains unsupported.
Survivability's increased declaration count does not imply service readiness.
No new real-service round trip or READY claim is made.

### Publication gates

No commit/push has yet occurred in C2. Full workspace test completion, final
merge staging/commit, remote freshness, clean-tree verification, hosted CI, and
head parity on a corrected revision remain mandatory gates. Historical hosted
success is not substituted for corrected-head CI. PR remains open, non-draft,
unmerged, auto-merge disabled at the last inspection.

### Final local gate completion

The full locked Rust 1.95 all-target workspace test run completed exit zero:
**1,412 passed, zero failed, zero filtered**, 162 summaries. Eight pre-existing
ignored external-data/broker tests are explicitly not counted as qualification;
their names/reasons remain in `workspace-test.log`. No new test is ignored.
Workspace check and warnings-denied Clippy both exited zero. Phase A and all
mandatory local codec compiler campaigns are complete. Original Task 087A
negative-control wrapper tests passed 8/8. Main was fetched again and remains
`13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`. Both reviewed merge-conflict paths
were staged, and `git ls-files -u` is empty. Final format/whitespace checks pass.

Local results qualify publication, not independent-review readiness: hosted
Fast/Deep CI and corrected-head parity remain required after push.