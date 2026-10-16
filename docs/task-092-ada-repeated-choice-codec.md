# Task 092 — Ada OMS JSON repeated Choice codecs

Disposition: **LOCAL_QUALIFICATION_PASSED** (C1); stacked publication/hosted status recorded below. Earlier failure and stop sections are historical.

## Baseline, isolation, and publication boundary

Initial main: `13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`.
Verified unmerged Task 091 dependency / Task 092 starting commit:
`3eb92115e2466a151c2bad9b9ff87b2ab5ba2c72`.
Dependency: <https://github.com/zackboll/ams-gra-codegen-oms/pull/92>.
Task branch: `feature/092-ada-repeated-choice-codec`; dedicated worktree:
`/home/zboll/git/ams-gra-codegen-oms-task092`.
Evidence: `/home/zboll/task092-scratch`. Task 091's worktree, branch, and evidence
were not modified. No final-main integration, publication, or hosted success is
claimed by the provisional results below. Initial stopped-attempt documentation
and failing logs are preserved separately in the evidence root.

## Independently verified OMS contract

Normative source: <https://gitlab.com/open-arsenal/oms/standard>, immutable commit
`726272bd0390982a759c91a9cf4e13b81c2b510b`, official document
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`.
Downloaded again to `contract/cal.docx`; independently checked SHA-256:
`b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
Extracted original `word/document.xml` paragraphs are in `contract/paragraphs.txt`.
Paragraph numbers count all `w:p` descendants, including table cells, one-based.

- Section 6.1.2, P573: for choice, text prescribed for **one** particle.
- P575–578: an element particle is a member; exact OAM namespace uses the local
  name, otherwise the name is `{target namespace}name`.
- P580: maxOccurs greater than one or unbounded is an array, each value prescribed
  by the following clauses; P583–589 apply the complex-type rules per value.
- Section 6.1.3, P591–595: a complex value is an object whose members come from its
  content-model particle.
- Table 6.1-2, P840–841: object members map to content in valid particle order;
  an array maps each value to an element information item.
- P608–623 explicitly illustrate optional repeated content as both empty array
  and absent member. Canonical **encoding omission** for empty zero-minimum
  sequences is the existing repository policy, not a unique normative encoding.

Composition therefore uses arrays of existing single-Choice objects, each with
one active alternative. It introduces no discriminant wire key, `$type`, or
alternate representation. Existing provider duplicate-key handling is unchanged.

## Narrow implementation

Only production readiness changes: admit `TypeKind::Choice` in the repeated named
value allowlist and narrow the residual alias/list diagnostic. Existing emitted
lookup, shared `support`/`record_support` closure checks, effective alternatives,
source-order blockers, and cycle sentinels remain in use. No independent traversal
or second plan is added. The existing repeated renderer already calls named
`To_JSON_*`/`From_JSON_*`; no Ada codec renderer modification was needed.

Supported: bounded/unbounded repeated Record fields containing emitted concrete
Choices, zero/positive minima, required-single supported alternatives, concrete
Records (including qualified DateTime/Duration contents), nested supported Choices,
and exact qualified OAM/non-OAM names. Enumeration alternatives are executable
fixture coverage; nested Choice traversal has dedicated shared-dependency tests.

Unsupported: optional/repeated/nillable Choice alternatives, abstract/polymorphic
values, direct/named unqualified temporal scalar alternatives, repeated temporal
scalars, aliases/lists, unqualified members, unsupported facets/cardinalities,
unresolved dependencies and cycles. This is not generalized Choice support.

No generated model or public API changed. The model-first production-generated
GNAT 16 probe passed 14 explicit checks before the initial readiness modification.
It constructs bounded sequences through To_Sequence, unbounded through Append,
and positive-minimum sequences through Required/Additional; spare slots expose no
payload. The final codec matrix additionally proves explicit construction under
all policies without invalid observable default Choice payloads.

## Executable synthetic qualification

Run `scripts/check-task092.py --compiler 14|16 --profile oam|qualified --scratch
/absolute/new/directory --generator /absolute/production/generator`.
Each profile uses a task-owned XSD and independently authored expected JSON;
the qualified schema retains OAM's version-attribute namespace while using
`urn:task092` wire member names. The same consumer is mechanically adapted only
for package/wire names; expected JSON is not produced by the codec.

The named Choice selects Boolean, concrete Record with DateTime/Duration, or Enum.
Containing Record covers 0..3, 1..3, 0..*, 2..*, and required-single Choice.

| Compiler | Normal | Assertion Check | Assertion Ignore |
|---|---|---|---|
| GNAT 14.2.0 | PASS, both profiles | PASS, both profiles | PASS, both profiles |
| GNAT 16.1.0 | PASS, both profiles | PASS, both profiles | PASS, both profiles |

Final evidence: `final-{oam,qualified}-{14,16}`, `final-matrix.json`.
Every one of 12 executable cells produces exactly:
`TASK092_CODEC_PASS checks= 63 rejections= 17`.
Checks include typed discriminants/contents, mixed order, nested temporal values,
bounded empty/one/two/maximum, absent zero-minimum fields, positive-unbounded
required-only/additional values, single Choice, structural golden equality, and
decode/encode/decode semantics. Check uses explicit raises, not assertions.

17 malformed-input cases per run cover scalar/object/null arrays, null elements,
empty/multiple/unknown alternatives, wrong alternative types, invalid nested
Records, invalid DateTime/Duration and wrong temporal type in the second element,
missing required arrays, too few bounded/unbounded items, and too many bounded
items. Only Codec_Error is caught; each rejection checks that no typed result was
published. Nine later-item failures additionally check `Bounded item 2` diagnostics.

Each profile runs eight schema controls through service-check and service-generate
(16 rejected commands): repeated, optional, nillable, unqualified alternatives,
abstract Choice, direct Duration alternative, missing dependency, and cycle.
Every failure must exit 1 with no published output directory.

Runner checks Alire 2.1.1, exact GNAT banners, GPRBUILD Alire 26.0.1/banner 26.0.0,
GNATCOLL 26.0.0 and inherited provider archive hash through the unchanged guard
validator. It isolates dependency workspaces, records command arrays/cwd/exits,
locks, logs, generated sources, source hashes, and exact completion markers.
Model/API output with/without codec is compared byte-for-byte for every profile.

## Core and historical controls

Full core package: PASS, zero failures/ignored/filtered. Main unit suite has 359
checks; integration/doc summaries are retained in `core-final.log`.
New controls cover shared valid/invalid repeated Choice closures, deterministic
first blocker, missing emitted Choice, and cyclic references. Capability and
Record-value visit counters remain at most one per declaration even across two
scans of a 25-declaration shared graph.

CLI codec suite: 18/18 PASS (`cli-attempt3.log`). Task 087A wrapper safety controls:
8/8 PASS (`guard-wrapper.log`). Historical failed attempts remain separate.

Expectation updates are narrowly justified by compiler-backed new support:
Task 089 core now positively checks repeated Choice then rejects repeated Choice
alternatives; Task 089R runner/CLI replaces rejected repeated Choice with nillable
repeated Choice; the other CLI repeated-Choice control uses an optional alternative.
The nillable case is rejected by model preflight, so its CLI expectation explicitly
checks unsupported selected types rather than a codec-only diagnostic.
No frozen corpus or historical JSON golden was weakened or replaced.

## Preservation and remaining gates

`byte-preservation.json`: 30 baseline/current selections with identical exit
statuses and generated file hashes. Baseline generator built from git archive of
exact dependency SHA, not Task 091's mutable worktree. Fixtures include minimal,
OAM, Choice, repeated, DateTime-combined, and Duration-combined; Ada model/API and
existing codecs, Rust model/API/codecs, and C++ model/API are compared. Unsupported
selections compare atomic failure statuses and absence of artifacts, not success.

Inherited compiler campaigns, full workspace-test completion, exact UCI reports,
final documentation, publication/head parity, hosted Fast/Deep CI and dependency
integration remain gates until their final evidence is recorded below. Workspace
all-target check and warnings-denied Clippy passed; formatting/whitespace and
Python/JSON/XML/YAML syntax checks passed. Full workspace tests are not yet claimed
complete. No real UCI service execution or unconditional READY claim is made.

Fast CI adds independent `ada-repeated-choice-codec`, depending on ada-json-guard,
checking out exact PR HEAD, building production generator, executing GNAT 16 all
policies in both profiles, requiring artifacts and uploading evidence. Existing
jobs remain intact. Deep CI's existing `.github/workflows/**` PR path includes this
workflow change; a triggered hosted run is still required, not assumed.

## Completed broad and inherited local gates

All 18 inherited compiler campaigns passed: Tasks 087A/B, 088, 089/089R,
090/090R, 091/091R on GNAT 14 and 16, all inherited policies. Exact command arrays
and exits are in `inherited-results.json`; detailed per-task logs and evidence are
in `inherited-*`. No filtered/omitted campaign is counted as success.

Locked Rust 1.95 workspace all-target check and warnings-denied Clippy passed.
Full locked all-target workspace tests exited 0: **1,413 passed, 0 failed,
0 filtered**, 162 summaries. Eight pre-existing ignored external-data/broker or
preparation tests remain explicitly unqualified, with names/reasons preserved in
`workspace-test.log`; they are not claimed as executed. Full core was rerun after
the additional dependency tests and passed separately. Historical ignored tests
were not altered. See `workspace-summary.json`, `core-final.log`, `clippy-final.log`.

## Actual pinned UCI readiness measurements

Root hashes reverified without schema modifications:
2.5 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`;
2.6 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
Before is the exact Task 091 dependency archive build; after is the production
Task 092 generator. `parallel-uci-results.json` records exact commands, exits,
root hashes and full reports. These are analyzer results, not execution claims.

| Selection | Selected types | Emitted declarations | Codec before → after | First blocker before → after |
|---|---:|---:|---|---|
| RDMA_InitializeSetup 2.5 | 50 | 49 | 47/49 NOT READY → 49/49 READY | Endpoint repeated Choice → none |
| RDMA_InitializeSetup 2.6 | 50 | 49 | 47/49 NOT READY → 49/49 READY | Endpoint repeated Choice → none |
| SurvivabilityRiskLevel 2.5 | 45 | 44 | 43/44 NOT READY → 44/44 READY | OwnerProducer repeated Choice → none |

Newly codec-renderable declarations are RDMA_InitializeSetupMDT and
SecurityInformationType for RDMA, and SecurityInformationType for Survivability.
All other emitted declarations were already codec-renderable in the measured
baseline. No remaining codec blocker is reported for these exact selections.
This does not imply arbitrary UCI service support.

`uci-field-shapes.json` independently inventories both source versions:
Endpoint is 0..* NetworkEndpointType selecting concrete IPv4_EndpointType or
IPv6_EndpointType; OwnerProducer is 1..* OwnerProducerChoiceType selecting
OwnerProducerEnum or NATO_SpecialWordsType. DisplayOnlyTo/ReleasableTo select
ReleasableToChoiceType; FGI_SourceOpen/Protected select their corresponding named
Choices; all four are 0..* with required-single enum/qualified String alternatives.
Complete closures pass the production analyzer, not merely the immediate Choice
kind check. The generated real-service compilation/execution gate is recorded
separately, and remains mandatory before real-service qualification is claimed.

## Mandatory real-UCI execution failure and stop

All three exact selections generated Ada successfully, with no schema edits:
`real-generate-*.json` and `.log`. RDMA 2.5 then compiled with GNAT 14.2.0 under
normal policy, including production-generated model and service codec.
The typed consumer executed but exited **1** while decoding the independently
authored input, reporting exactly:

```text
raised SERVICE_CODEC.CODEC_ERROR : invalid checked String
```

Evidence: `real-qualification-25-RDMA_InitializeSetup-14/normal-build.log`,
`normal-run.log`, and `normal-run.command.json`; consumer, dependency locks,
generated sources and the original input are preserved. The input is in
`real-expectations/RDMA_InitializeSetup.json`; qualification driver is
`qualify-real.py`. No expected Codec_Error handler suppresses this positive-case
failure. Whether the checked-String failure originates in the independently
authored input or generated validation is not established. No input was changed
and no codec validation was relaxed to turn this attempt green.

Per the task's stop condition, work stopped at this mandatory failed gate.
Remaining real-service compiler/policy cells were not run and are not claimed.
The synthetic 12-cell matrix remains passing, but real-service READY is only the
analyzer result, **not qualified real-UCI typed execution**.

Final disposition: **QUALIFICATION_FAILED**. Neither stacked nor final milestone
is satisfied. No commit, push, PR creation, approval, merge, retarget, or hosted
Task 092 CI run occurred. Task 092 HEAD remains the initial dependency commit;
all incremental work is uncommitted. PR #92 remained OPEN at last refreshed check,
with unchanged dependency HEAD; main remained the recorded initial SHA.
Hosted Fast/Deep status and PR head parity are not applicable before publication.
The new CI job is implemented locally but unexecuted on GitHub. Existing workflow
jobs remain present. Post-merge integration against final main is outstanding.

## C1 localization and authoritative fixture correction

Original generated sources and JSON inputs were SHA-256 recorded before diagnosis
in `c1/original-sha256.json`; later revalidation confirms every original byte is
unchanged. Original inputs are also copied to `c1/original-expectations`.
Diagnostic-only modified Ada is isolated under `c1/diagnostic`; none of its
instrumented source is used as qualification output.

Diagnostic build and execution identified exactly
`From_JSON_UniversallyUniqueIdentifierType` (original service_codec.adb:326–335),
calling Model.Create with `12345678-1234-1234-1234-123456789abc`.
Member extraction trace proves the first failing JSON path is
`MessageHeader.SystemID.UUID`. No Choice alternative or repeated index applies
to this failing call: the preceding repeated OwnerProducer choices decoded
successfully, then aggregate evaluation entered MessageHeader/SystemID/UUID.
The same invalid lexical value also appears at
`MessageData.RDMA_InitializeSetupID.UUID`. Trace evidence: `c1/trace-run.log`.

Authoritative pinned 2.5 declaration is UniversallyUniqueIdentifierType in
UCI_MessageDefinitions_v2_5_0.xsd, textual line 145719 (lxml's parsed sourceline
145720). It restricts xs:string (whiteSpace preserve), length 36, and pattern:

```text
(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})
```

Generated programs-oam.adb:634–703 implements that unchanged contract.
The UUID is non-nil; zero-based offset 19 (one-based character 20, first nibble
of the fourth group) is `1`, outside `[89abAB]`. Matches_Pattern returns False
at lines 678–680; Create's condition at line 697 raises Constraint_Error with
`invalid UUID` (lines 698–699). The codec correctly translates that constructor
failure into Codec_Error. No JSON extraction/dispatch defect is demonstrated.

Independent lxml XMLSchema validation of the copied authoritative declaration
rejects the original value and accepts `12345678-1234-1234-8234-123456789abc`.
A direct generated Model.Create probe independently confirms both decisions.
See `c1/independent-xsd.json`, `c1/constructor-run.log`; exact marker
`TASK092_C1_CONSTRUCTOR_PASS checks=4` includes the version controls below.

Pre-correction inspection also identified UCI_SchemaVersionStringType at
MessageHeader.SchemaVersion: `002.5` has length 5, below minLength 7, and lacks
the mandatory third numeric component. The authoritative declaration restricts
xs:string with minLength 7, maxLength 57 and pattern
`[0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?`.
Generated constructor programs-oam.adb:512–627 rejects through Matches_Pattern
or the length condition at 618–623 (`invalid schema version`). Independent XSD
and direct Create checks reject `002.5` and accept `002.5.0`; 2.6 uses `002.6.0`.
Exact source declarations/locations for both roots are recorded in
`c1/authoritative-declarations.json`.

Only independently authored positive fixtures changed: UUID variant nibble to
8 and schema version to 002.5.0/002.6.0. No production String constructor,
constraint, extraction, model API, or codec implementation changed for C1.
The original invalid RDMA input is retained as task092-real-rdma25-invalid-original.json.
Real consumers separately inject its invalid UUID and invalid version into a
valid input, catch only Codec_Error, and assert no typed result is published.
Thus neither invalid value is converted into an expected positive success.

The corrected actual production-generated Ada passed three selected real services
on both compilers under all three policies (18 executions), with typed repeated
Endpoint IPv4/IPv6 contents, OwnerProducer required/additional variants, and
structural expected JSON comparisons. RDMA markers: TASK092_REAL_PASS checks= 12;
Survivability: TASK092_REAL_PASS checks= 8. Each includes two negative regressions.
Evidence: `c1/real-qualification-*`; initial corrected driver marker is
`TASK092_REAL_QUALIFICATION_PASS selections=3 compilers=2 policies=3`.

Reusable repository runner scripts/check-task092-real.py regenerates via the
actual production generator, checks unchanged pinned root AND security-schema
hashes, isolates consumers, checks compiler/provider locks and banners, and runs
all policies with exact completion counts. Its end-to-end validation and all
inherited/Rust reruns remain publication gates until completion below.
Deep CI adds an independent pinned-source real-UCI job using the existing fetch
scripts/provenance guarantees; no existing Deep CI job is altered or bypassed.

### C1 reusable runner and preservation completion

scripts/check-task092-real.py passed end-to-end with release-built production
generator on GNAT 14.2.0 and 16.1.0, all three policies, all three selections:
**18 cells**, recorded under `c1/runner-release14` and `c1/runner-release16`.
Every RDMA run has 12 checks (including two separate original-invalid-value
rejections); every Survivability run has 8 checks (including those two rejections).
Generated source bytes equal the debug-generator output for all three selections
(`c1/release-debug-parity.json`). The initial debug-regeneration runner attempts
remain separate evidence; no tool or compiler substitution occurred.

C1 synthetic rerun: both profiles/both compilers/all policies again pass exactly
63 checks and 17 rejections per run (`c1/final-matrix.json`).
C1 baseline/current preservation rerun: 30/30 comparisons pass with identical
statuses/hashes (`c1/byte-preservation.json`). Workspace check, warnings-denied
Clippy, format, syntax, whitespace, and 8/8 guard-wrapper controls pass.

Full inherited and workspace test rerun completion and publication identities are
recorded below when available. The original failed execution and initial stopped
report remain historical evidence; C1 proves the input—not generated String
validation—caused the original failure.

### C1 final local gates and stacked publication baseline

All **18/18** inherited campaigns reran successfully in C1, both GNAT versions,
all assertion policies (`c1/inherited-results.json`). Full locked Rust 1.95
all-target workspace tests exited 0: **1,414 passed, 0 failed, 0 filtered**,
162 summaries; eight pre-existing ignored external/broker/preparation tests are
explicitly not counted as execution (`c1/workspace-summary.json`). The increase
from the earlier 1,413 is the added missing/cyclic dependency regression already
present when C1 began; it was not in the earlier in-flight full workspace run.
No new ignored test was added.

C1 workspace check, warnings-denied Clippy and release generator build exited 0.
All 30 byte-preservation selections passed again. Synthetic counters stayed
63/17 in all twelve cells. All 18 real-service runner cells passed. Formatting,
whitespace, Python/JSON/XML/workflow syntax, original-byte preservation, and
Task 087A wrapper controls passed. Evidence is separate from initial failures.

Immediately before publication, origin/main remains
`13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`; PR #92 is OPEN at unchanged
`3eb92115e2466a151c2bad9b9ff87b2ab5ba2c72`. Task 091 worktree remains clean and
its branch identity unchanged. Task 092 targets `feature/091-ada-duration-codec`
until dependency merge; final main integration and retarget remain a second gate.
Hosted exact-head checks must be reported separately, never inferred from local
success. The qualification commit is identified by the PR HEAD, not a
self-referential SHA embedded in that commit.
