# Task 097 — production-generated lossless Ada Float64 codecs

## Status and ownership

**Disposition: QUALIFICATION_FAILED — local checkpoint only, no PR.** Passing numerical sampling
below is not completion of the mandatory composition, preservation, integration,
resource, real-service, and hosted gates. No exhaustive binary64 proof or
additional-platform portability claim is made.

Starting fetched main: `5f6800d505c6501638c322d20b23cfc36b09d7d6`.
Starting tree: `fde60d31fe7d539a7f5c2d2deafa7e12e8f574d2`.
Both reviewed heads (`81c56a434000096f7c378d03b93b354b776e15f2` and
`2083c30fd5ef68098b739b6c4b5cfa3f5a309d91`) were verified ancestors.
No Task 097 owner existed in local/remote branches, worktrees, or PRs.
Branch: `feature/097-ada-float64-codec`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task097`.
Evidence: `/home/zboll/task097-scratch/`.
Original checkout and former task branches/worktrees/evidence are not modified.

### Unresolved mandatory gate

`conformance16-oam-03/Normal-allocation` fails with:

```
raised PROGRAM_ERROR : gnatcoll-json-utility.adb:152 finalize/adjust raised exception
```

The isolated `allocation-diagnostic-01-job` reproduces it at **decode allocation
budget 33**, after encode budgets 0–33 and decode budgets 0–32. The single-shot
test allocator wraps `__gnat_malloc`, disarms before raising `Storage_Error`,
and lets exception cleanup allocate. GNATCOLL's controlled string unescape
return path converts an exception during adjustment/finalization to
`Program_Error`. This is neither successful resource handling nor invalid user
input. No blanket `Program_Error` catch or message-based relabeling is added.
The required distinct resource contract remains unresolved; publication is
prohibited. No broad fault campaign, final source qualification, or complete
real-service qualification is claimed.

The diagnostic reused the completed failed attempt's build tree only after its
worker exited. The original consumer was preserved as
`allocation-diagnostic-01/prior-consumer.adb`; added stderr budget tracing is
recorded separately. Original command/stdout/stderr/exit files are not replaced.
Remaining real-service/inherited supervisors were explicitly interrupted on
this disposition. Their missing final successes and interruption records must
not be counted as passes. Earlier finished cells remain individually identified.

Final refetch still found main at the exact starting SHA. There is no remote
Task 097 branch, PR, hosted head, auto-merge, or merge. Local feature checkpoint
identity is recorded in the final response/evidence after commit.

## Explicit deployment profile

```
--language ada --with-codec \
--ada-codec-runtime linux-x86_64-glibc-2.41-lossless-v1
```

The deployment target, not generator host, is Linux/x86-64/glibc 2.41,
x87 nearest, MXCSR nearest, DAZ disabled, FTZ disabled. The measured local
libc is Debian `2.41-12+deb13u4`, SHA-256
`9792e3cbb541c8f44c7acf5f14f4022ea62998ecc787d326bed4d8b6547dfd92`.
No approval is cached or caller control state modified. Ordinary sticky
arithmetic flags may change during supported operations.

Portable Float32/Float64 rejection is retained. The existing
`linux-x86_64-glibc-2.41` profile retains Float32 and Binary/Float32, rejecting
Float64. For Float64-free selections the new option normalizes to the existing
portable or Float32 rendering path. Semantic emitted operations and effective
Record fields determine demand, never declaration-name substrings or original
unselected XSD types. CLI, shared readiness, and backend carry explicit runtime
selection. Wrong-language options and unsupported selections fail before output.

## Production support architecture

`ada/oms-json-support/ams_gra_oms_json_lossless.gpr` explicitly imports the
existing Float32 project, which imports the ordinary guard project. It compiles
only new `lossless/` sources. No research source is linked and existing C bridge
symbols/objects are not duplicated.

The additive `Occurrences.Numeric_Lexeme` API returns original SAX-certified
numeric text only for numeric nodes. Existing occurrence ownership preserves
decoded-key last-value-wins and complete subtree replacement; original grammar
is validated before effective duplicate values are consumed.

The Float64 Ada boundary validates exact numeric syntax and the document-size
token limit, then calls C `strtod_l` with an explicit C numeric locale and a
bounded terminated buffer. C checks full consumption, errno and finite result;
ERANGE finite subnormal/signed-zero results remain valid, infinite overflow
rejects. There is no numeric DOM, Float32, or Ada `'Value` conversion.
Every operation checks target/control state before numerical work. A C/Ada
ABI exchange verifies binary64 and negative zero. Exact `NaN`, `Infinity`, and
`-Infinity` strings use an explicit-status C special boundary; no zero division
or arithmetic overflow manufactures them. NaN payload/signaling preservation
is not promised.

Finite output uses a fixed `%.17g` formatter in a 32-byte local buffer.
Length, capacity, termination and embedded NUL are checked before publication.
`uselocale` restores the exact previous handle, including `LC_GLOBAL_LOCALE`.
No process-global `setlocale`, mutable locale side table or approval cache is
used. Allocation/installation/formatting failures return distinct resource
status without output. **Restoration failure is fail-stop (`abort`), not an
ordinary recoverable error:** returning could leave a private locale installed.
The process does not free a potentially installed locale or publish output.
An isolated fault-test child must terminate with SIGABRT, with core files off.

Invalid values translate to `Codec_Error` only at the codec boundary.
Unsupported platform and resource exceptions propagate distinctly; Ada
`Storage_Error` is not relabeled as invalid payload. Unknown C statuses remain
`Program_Error`. There is no blanket exception translation.

## Structural emission and generator integration

The explicit `Encoder` context selects the legacy DOM or a bounded typed
fragment builder. New Float64-containing output uses typed object/array/scalar
fragments; no arbitrary raw JSON constructor is public. Finite Float64 material
comes only from the validated private numeric formatter and is inserted into
its structural position. Output is occurrence-validated, not reserialized.
Legacy Float32 leaves retain the existing formatter. Strings use existing
guard/provider escaping, with UTF-8 validation and exact expansion budgeting
before allocation. Binary expansion has a lossless-path document bound before
allocating its hex string. Nodes, depth and bytes are bounded at composition;
fragment mutation uses owned temporary state before assignment/publication.

Object members follow effective schema field order (inherited fields first),
Choice emits exactly its selected alternative, and arrays retain element order.
Unknown-member allowlists remain active at every Record and Choice level.
Named constrained floats use only `Model.Create`/`Model.Value`; predicates and
facets are not duplicated in codec code. Float64 scalar Choice alternatives,
Decimal, unsupported facets/temporal/Binary/polymorphic shapes stay rejected.

## Fresh early measurements (not final qualification)

* `checkpoint-05`: actual production-generated mixed Binary/Float32/Float64
  compiled and executed with fresh GNAT 16.1.0 provisioning. Root/nested/Choice
  finite number-kind and exact-bit checks, signed zero/subnormal, six invalid
  cases, old-profile/portable atomic rejection, model/API byte parity passed.
* `generated{14,16}-{oam,qualified}-01`: all 12 numerical cells passed
  Normal/Check/Ignore; each consumes 27,167 conversion cases and 20,474 finite
  encoding patterns. Fifteen independently identified output positions per
  encoding document use an explicit JSON-number subtype tag plus independent
  exact-rational rounding. Quoted numerals cannot pass as JSON numbers.
* `generated{14,16}-oam-O2-01`: targeted optimized generated-Ada campaigns passed
  all three policies, using the same complete corpora and output path.
* `boundary-02`: new production C boundary at O0/O2 × C/German thread locale
  passed frozen corpus hashes, conversions and exact-rational encoding checks;
  six lifecycle fault modes, 8,000 concurrent calls per optimization,
  SIGABRT restoration-failure contract, and 128 isolated environment processes.
* `environment{14,16}-{oam,qualified}-01`: all 768 generated-Ada processes passed
  all 16 independent rounding pairs × four DAZ/FTZ states × policies.
  Only nearest/nearest without flushing succeeds. Rejection is before JSON work,
  output sentinels and caller controls remain unchanged, TEST harness restores
  full state, and subsequent supported calls succeed without cached approval.
* `workspace-static-01`: Rust 1.95 fmt, locked all-target workspace check and
  warnings-denied Clippy passed. `workspace-tests-01`: 162 summaries,
  **1,429 passed, 0 failed, 8 ignored**. Ignored tests are not execution evidence.
  These precede the final semantic-demand refactor; `semantic-check-01` then
  passes the affected core codec and complete CLI codec suites. They are not
  a final-source full workspace rerun.
* `preservation-02`: 105 exact-main comparisons, **86 byte-identical generations,
  19 matching atomic rejections** across Ada model/API/legacy codecs, Rust, C++.
* `ci-integrity-01`: split/layout guards and all 210 split mutations passed.

Frozen research corpus identities were verified before use:

```
14c86c5fa674d279d198947dad03f71f517a40b583729df1184bbf6596a6451b  conversions.tsv
b1ad323c7d5d88c465bf966dc2edc1249c68982ad817044fa5579ce65e9abc82  encoding.tsv
```

The corpora/oracle are reused frozen research inputs. The executions above are
new production executions; unchanged historical research results are not
relabeled. Failed checkpoint/test setup attempts and interrupted debug full-root
CLI attempts remain separately preserved, never relabeled as completed passes.
The broad composition/allocation runner failed as described above. Inherited
campaigns were interrupted; no unresolved attempt is a pass.

## Focused real-UCI analysis

`uci-{baseline,current}-01` verifies pinned root/security-schema hashes and
parses each root once per variant, reusing production normalized IR.
Both releases contain 40 named Float64 declarations; effective fields number
1,393 (2.5) and 1,395 (2.6), including inherited occurrences. Static affected
messages number 393/395, respectively. These are not deployed contract counts.

The six smallest affected unchanged messages per release become model/codec
ready under the lossless profile in this measurement. UCI 2.5 selects EntityLost,
EntityConfirmation, EarthOrientationParameters, SpectralDensityReport,
SubsystemCalibrationStatus and LAR_Report; 2.6 replaces SpectralDensityReport
with RF_Profile. Each has a recorded analysis-only valid contract.
Complete generated typed valid/invalid execution is mandatory before service
qualification. Individually finished real-service cells are retained in
`real{14,16}-02/`; both overall campaigns were interrupted, so complete selected
real-service qualification is not claimed here.
DLZ remains blocked by TimeType: portable 46/53, Float32 47/53, lossless 52/53.
ScanDataBaseType owner-only projection improves from 10/13 to 13/13; separately
recorded static witnesses reach 40 actual messages in each release. This
synthetic owner projection is not a selected message/service readiness result.
Log remains portable READY (41/41, 43/43), RDMA_InitializeSetup 49/49 and
SurvivabilityRiskLevel 44/44; fresh inherited typed execution is still required.

## Hosted execution

A separate `ada-float64-codec` matrix job is drafted in Fast CI, independently
selecting GNAT 14/16, leaving existing jobs unchanged. It verifies external
GNAT 14.2.0 before Alire selection and uses the pinned Debian image with actual
installed libc package/binary identities logged. Fresh setup, exact-head
checkout, production corpora, generated numerical/conformance/environment
execution and complete textual artifacts are required. The 90-minute timeout
is provisional until measured fresh end-to-end runs justify it with margin.
**No hosted execution, qualified feature head, remote parity or PR is claimed.**
A local unqualified checkpoint commit preserves the implementation and failure.

Any resumed qualification must first resolve the allocation failure, then
recheck source manifests after semantic demand changes,
complete all required tests/real-service execution, review task-owned changes,
fetch/reconcile current main, and only then publish one open non-draft PR.