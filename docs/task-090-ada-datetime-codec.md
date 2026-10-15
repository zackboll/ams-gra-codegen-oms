# Task 090 — checked Ada OMS JSON DateTime codecs

## Isolation

Starting main: `dd1353a93f4e1e62fa544f7fb7349a6eba41fc03`, the merged
PR #88 commit. Branch: `feature/090-ada-datetime-codec`. Worktree:
`/home/zboll/git/ams-gra-codegen-oms-task090`. Task-owned, disk-backed evidence:
`/home/zboll/task090-scratch`. Task 088/089 branches and qualification artifacts
were not modified. Main remained at the starting SHA on the subsequent fetch.

## Architecture and domains

Ada readiness calls the existing `temporal_profile` for named DateTime and
requires exactly `TemporalProfile::DateTimeZulu`. For direct members it calls
`direct_temporal_profile` and requires `DirectTemporalProfile::DateTime`.
No type-name recognition, new parser, representation, or timezone conversion
was introduced. Named unconstrained DateTime and unsupported constraints fail
closed; named TimeZulu, Duration, Float, Decimal and Binary remain unsupported
by the Ada codec. Choice and repetition are deliberately outside this branch.

The generated model API is unchanged:

```ada
function Create (Value : String) return Instant;
function Value (Item : Instant) return String;
function Create (Value : String) return Deadline;
function Value (Item : Deadline) return String;
function Create (Value : String) return XML_Schema_Date_Time;
function Value (Item : XML_Schema_Date_Time) return String;
```

Both profiles encode as JSON strings using `Model.Value`. Named decoding uses
the expected return type to resolve the overloaded checked `Model.Create`.
Direct decoding uses a private codec helper, `Scalar_Date_Time`, reserved in
codec naming preflight and emitted only when direct DateTime is stored.
Only the constructor call is in the `Constraint_Error` translation scope;
String extraction precedes that scope. Record decoders do not catch
`Constraint_Error`. Invalid expected input raises `Service_Codec.Codec_Error`.
Optional wrappers and Clark-notation `{urn:test}` wire keys are preserved.

Public payload operations are `Encode_TemporalPayload` / `Decode_TemporalPayload`
and `Encode_DirectPayload` / `Decode_DirectPayload`; their signatures follow the
existing `Item : <model payload> return String` and
`Text : String return <model payload>` codec API.

Named values require literal `Z`. Direct values allow omitted timezone, `Z`,
and valid signed offsets. Whitespace collapse, fractional trailing zeros,
leap seconds, leap days, hour 24, long years and fractions retain the model's
existing XML Schema 1.0 lexical behavior. Lexical equality is not value-space
equality; no timestamp comparison or canonicalization is performed.

## Fixture-first and reproducibility

The original `temporal` and `direct-datetime` service fixtures are used unchanged.
Models and service APIs were generated without codecs first and compiled using
GNAT 14.2.0 before production codec edits. Baseline model hashes are in
`model-baseline.sha256`. Independently authored JSON goldens use qualified wire
keys and reversed member order. Unsupported neighboring temporal declarations
remain outside the projected selected closures.

`scripts/check-task090.py` builds and executes both actual production-generated
codecs with the unchanged JSON guard. It requires Alire 2.1.1, GNAT 14.2.0 or
16.1.0, GNATCOLL 26.0.0 (including the qualified provider archive hash) and
GPRBUILD 26.0.1. Dependency builds are sandboxed in the task-owned workspace.
It records command arrays, exits, compiler identities, dependency locks and
source hashes, and verifies exact nonzero completion counts.

Frozen independent oracles:

| Oracle | SHA-256 |
|---|---|
| `datetime-zulu.txt` | `3230a3270dc2239c53435235578c343248809c08d8d30af9b85a2392ae3c86c2` |
| `datetime-direct.txt` | `a0417fee4fbd1820618f60282200502fce543836e76dd1f952ac0612f33756a5` |

The named consumer uses the entire Zulu corpus. The direct consumer additionally
uses the direct corpus and explicitly assigns positive direct expectations to
the six Zulu corpus inputs rejected solely for omitted/numeric timezone.
Negative tests catch only `Codec_Error`; unexpected exceptions fail execution.

Initial executable campaigns completed successfully on GNAT 14 and 16 under
normal, Check and Ignore policies: **161 named and 205 direct cases per run**.
Task-local sandboxed repetitions also passed for both compilers. Early
concurrent 087A/087B shared-cache builds failed with missing archive objects;
those failures were retained, not certified. Unchanged scripts were subsequently
rerun serially using isolated task-owned Alire settings.

The unchanged Task 087A and Task 087B scripts subsequently completed exit zero
on **both GNAT 14.2.0 and 16.1.0, all three policies**. Task 087A retains its
199-case/four-boundary frozen output hashes; Task 087B retains its 41-case
consumer and seven-case subset markers. Wrapper adversarial controls also pass.

## Regression and delivery gates

Rust 1.95 formatting, locked workspace all-target check, warnings-denied Clippy,
the focused core codec suite and CLI codec suite (14 tests) pass. Recursive byte
comparisons with an archived baseline generator pass for the original Task 087B
Ada codec, Rust model/API/codec and plain output, C++ output, and both temporal
Ada model/API selections. The protected model backend, shared temporal
classifier, Rust codec and C++ backend have no production edits.

The separate Fast CI job `ada-datetime-codec` depends on `ada-json-guard`, checks
out the exact PR head, runs both synthetic fixtures under all GNAT 16 policies,
and uploads logs, hashes, exits, commands and dependency locks. No real UCI
fetch or Sleet build is part of that job. Hosted execution is not implied by
registration.

Real UCI measurements use the existing pinned roots (2.5 SHA-256
`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`, 2.6
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`).
Before/after RDMA reports and exact exit records are retained in the evidence
directory. No full RDMA READY claim is inferred from support for DateTimeType.

| Release | Before | After | Newly codec-renderable | First remaining blocker |
|---|---|---|---|---|
| UCI 2.5 | 41/49 | 42/49 | `DateTimeType` | `NetworkEndpointType` Choice |
| UCI 2.6 | 41/49 | 42/49 | `DateTimeType` | `NetworkEndpointType` Choice |

Both selected RDMA services remain codec **NOT READY** (expected command exit
1). The 2.5 first blocker changes from DateTimeType to NetworkEndpointType;
the 2.6 first blocker is unchanged. This bounded measurement is not an inventory
of every UCI service and establishes no entirely codec-ready real service.

The complete affected Rust/Ada/C++/CLI/core campaign finished exit zero:
**1,079 passed, zero failed, six pre-existing ignored** across 112 summaries.
The independent Rust/C++ backend campaign also finished exit zero. Existing
zero-OMS and unsupported-output atomicity controls pass. No ignored synthetic
DateTime case was introduced. Final Python/JSON/workflow syntax, formatting and
`git diff --check` pass.

Publication and exact tested/local/remote/PR head parity are recorded in the
final handoff. Hosted CI status is reported separately and is not claimed here.

## Task 090R — corrective Choice/repeated integration

### Starting state and history

The verified original Task 090 head is
`701919c14e9f00673ca80212315670c7a78d54d0`. Actual fetched main is
`5f1100e20ec6dbb3bd39d2e6a23de979c3783306`, the accepted PR #90 merge,
including PR #89's Choice implementation. PR #88, #89 and #90 were verified
merged. PR #91 was open, non-draft, based on main, with auto-merge disabled;
PR #92 was inspected read-only and not modified. The original worktree was
clean. Integration uses a normal, two-parent merge, not a rebase or squash.

Evidence is isolated under `/home/zboll/task090r-scratch`, including compiler
campaign directories, command logs, exit files, generated-source hashes,
dependency locks, isolated Alire settings/cache/indexes, Rust target directories
and temporary storage. Failed development attempts are retained separately
from completed qualification campaigns.

### Source reconciliation and safety

The four overlapping files were reviewed. Only the core codec test section
produced textual conflict markers; both sides' tests were retained. The workflow,
Ada renderer and CLI tests auto-merged and were reviewed semantically.

`AdaCodecAnalysis` remains the main-branch memoized implementation: emitted
lookup map, capability and Record-value caches, in-progress cycle sentinels,
shared dependency traversal and first-blocker ordering are preserved. DateTime
scalar classification uses the original `temporal_profile` and
`direct_temporal_profile`, not declaration names or a new validator.

Named DateTime requires exactly `DateTimeZulu` (`.+Z`); ordinary direct DateTime
requires the unconstrained direct profile. Encoding uses `Model.Value` and
decoding uses checked `Model.Create`. The existing JSON guard remains the sole
document boundary. No model/API representation or lexical/timezone conversion
was changed. `Scalar_Date_Time` is emitted using the existing emission plan;
its case-insensitive fixed-name reservation has a focused unit test.

Direct DateTime Choice alternatives fail closed with
`direct DateTime Choice alternative is unsupported`. Bounded and unbounded
direct repeated DateTime fail closed with
`direct DateTime repeated element is unsupported`. Neither can reach a numeric
renderer fallback. Named DateTime used directly as a Choice value or repeated
scalar element remains conservatively NOT READY with `is not qualified`
diagnostics. These restrictions do not disable non-DateTime Choice/repetition.

Concrete Records with ordinary named/direct DateTime fields are exercised in
the production-generated combined fixture, including nested Records, inherited
members, a required-single Choice selecting a Record, bounded repeated Records,
zero-minimum unbounded Records, and positive-minimum unbounded Records using
the public Required/Additional construction. Named/direct optional values are
tested present and absent. No private sequence storage is accessed.

The independent JSON golden and Ada consumer check exact structural equality,
lexical preservation, legal offsets, Zulu restrictions, Choice discriminants,
element ordering, unknown members, malformed nested/Choice DateTimes and bad
later repeated elements. Failure controls prove no typed value is published.
`scripts/check-task090r.py` requires both service commands to reject unsupported
schemas atomically; it also checks compiler identities and qualified locks under
Normal, Check and Ignore. Anonymous direct field restrictions are rejected at
the existing XSD ingress boundary; nonempty direct constraints are additionally
tested against the shared scalar classifier in core.

### Negative-control rationale

The Task 089R invalid repeated-Record DAG leaf and Task 089 invalid nested child
previously used DateTime merely as an unsupported primitive. They now use
Float64, preserving the DAG shape, reference counts, at-most-once visitation
assertions and invalid-dependency purpose. The CLI combined invalid nested-value
control and the 089R executable wrapper use `xs:double` for the same reason.
The original repeated-temporal atomicity control still uses direct DateTime and
now requires the specific repeated-DateTime diagnostic. No golden for previously
supported repeated values was changed; no new ignore or skip was introduced.

### Compatibility and qualification evidence

The accepted main Ada model renderer is byte-identical, including the corrected
direct String vector equality association. The original 087A/087B/088/089/090
qualification scripts and frozen temporal corpora are unchanged. The only 089R
wrapper change is its unsupported primitive substitution described above.

Thirty-five generator-output comparisons against the accepted 089 generator
and original 090 generator completed with byte identity (`byte-comparisons.json`;
`logs/parity-final.log`). They cover Choice, repeated, combined, minimal and OAM
fixtures across Ada/Rust/C++, plus named/direct DateTime output against 090.
Rejected historical combinations are compared as rejected, not as byte evidence
for an artifact that was never generated. Successful comparisons include model,
service API and applicable codec files. No protected golden output was edited.

Final publication, complete local gate results, real-UCI declaration inventory
and corrective-head hosted certification are recorded below only after their
commands complete. Until then, this section is progress evidence, not merge
approval.

Completed local gates (exit-zero logs in the evidence directory): Rust 1.95.0
formatting, locked workspace all-target check, warnings-denied workspace Clippy,
complete workspace tests (**1,409 passed, zero failed, eight pre-existing
ignored**, 176 summaries), final core rerun (**356 passed**), and the independent
Rust/C++/CLI/runtime campaign (**612 passed**, overlapping the workspace run).
The complete workspace run includes Ada backend and generated model tests;
generated Rust codec/runtime regressions and CLI readiness/generation tests are
also included. No additional ignore was introduced. Eight guard adversarial
wrapper controls, CI split controls, Python/JSON/workflow parsing, four distinct
Ada job structural controls, whitespace and source conflict-marker checks pass.

| Campaign | GNAT 14.2.0 Normal/Check/Ignore | GNAT 16.1.0 Normal/Check/Ignore |
|---|---|---|
| 087A guard | PASS: 199 cases + four boundaries | PASS: 199 cases + four boundaries |
| 087B original codec | PASS: 41 + seven subset checks | PASS: 41 + seven subset checks |
| 088 Choice | PASS: 174 checks per policy | PASS: 174 checks per policy |
| 089 repeated | PASS: 121 checks per policy | PASS: 121 checks per policy |
| 089R combined | PASS | PASS |
| 090 named/direct | PASS: 161 / 205 checks per policy | PASS: 161 / 205 checks per policy |
| 090R combined DateTime | PASS: seven negative schema controls | PASS: seven negative schema controls |

Original campaign logs are `logs/campaign-<task>-<compiler>.log` with matching
exit files. Final combined evidence is `certified-combined14` and
`certified-combined16`; every assertion-policy run records actual compiler and
GPRBUILD identities, locks and generated-source hashes. Early new-consumer
syntax/overload errors were corrected; the final complete reruns pass. The
intentionally unsupported anonymous direct facets fail at the original XSD
boundary before model publication. Unsupported named facets report model NOT
READY before codec analysis. Classifier unit tests independently cover direct
nonempty constraints.

The accepted UCI roots were SHA-256 verified against the original hashes above.
The integrated production CLI's UCI 2.5 RDMA result is **47/49** codec-renderable
emitted declarations (50/50 selected model types, one renderable selected
message). `DateTimeType` is newly codec-renderable relative to accepted Task 089.
The service codec is **NOT READY**. The complete 2.5 unsupported declaration
inventory is `RDMA_InitializeSetupMDT` (`Endpoint`, repeated Choice) and
`SecurityInformationType` (`OwnerProducer`, repeated Choice). The first blocker
is `RDMA_InitializeSetupMDT.Endpoint: Choice/alias/list repeated element is
unsupported`. No real-message typed round-trip claim is made for a NOT READY
service. Per-declaration diagnostics use a scratch-only copy of the integrated
analysis, with a public-name adapter solely for diagnostic access; the
production CLI remains the readiness authority.

The integrated production CLI also completed UCI 2.6 RDMA at **47/49**, with
the same 50/50 selected model readiness and the same first repeated-Choice
`Endpoint` blocker. Both production CLI NOT READY measurements completed with
the expected exit 1; neither is a failed qualification execution. No fully
codec-ready real UCI message was demonstrated by these measurements.

All mandatory local source/execution gates above have completed. Publication
must retain the original 090 and accepted 088/089 ancestors. Hosted results from
`701919c` do not certify this integration: fresh Fast and Deep CI are required
on the published corrective head. Auto-merge remains disabled and PR #91 must
remain open for independent review.

### Corrective publication and independent-review handoff

The qualified normal merge commit is
`c0ff9cdc07807a78f23c389abf53ee4051327708`, with parents `701919c14e9f00673ca80212315670c7a78d54d0`
and `5f1100e20ec6dbb3bd39d2e6a23de979c3783306`. Original 090, accepted 089 and
accepted 088 ancestry checks pass. The merge was committed only after all local
source/execution gates completed, then pushed normally to the existing branch.
Committed-head core and CLI codec reruns passed. The accompanying documentation
handoff commit changes no qualified source or executable fixture.

Both release inventories completed successfully: **49 emitted declarations,
47 codec-renderable**, `DateTimeType` newly supported, and exactly two remaining
unsupported declarations in each release: `RDMA_InitializeSetupMDT` at repeated
Choice-valued `Endpoint`, and `SecurityInformationType` at repeated Choice-valued
`OwnerProducer`. Selected model readiness is 50/50 in each release; full generated
codec support is NOT READY. No full real-UCI codec READY claim is made.

Fresh Fast run `38671778140` and Deep run `38671778163` were started on the
corrective merge SHA, not on historical 090. At inspection, Fast guard/Rust and
all four Deep jobs were in progress. GitHub's synthetic merge
`aaf45b9cf1dab5f975939eccb26e517820b50b36` has parents accepted main and the
corrective merge; its tree `82a469bf44c1a0ea32e1622c50424558747a6a09` is identical
to the corrective merge tree. Completed per-job checkout certification is still
pending. A documentation-only handoff publication requires fresh hosted runs
for its exact SHA as well; earlier runs cannot substitute for those results.

PR #91 remains open, non-draft, targets main, is mergeable, and has no auto-merge
request. PR #92 remains untouched at `ead6e7e905e434b9c29e7517bba991768d13006d`.
Final documentation-head SHA, exact local/remote/PR parity, fresh run IDs and
status are recorded in the external handoff evidence and final response (a
commit cannot contain its own SHA). No PR was created or merged.

Disposition: **READY_FOR_INDEPENDENT_REVIEW — HOSTED_CI_PENDING**.
This is not merge approval. Stop for independent review; do not enable
auto-merge or proceed to another capability task.