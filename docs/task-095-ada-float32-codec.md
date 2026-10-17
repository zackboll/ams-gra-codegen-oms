# Task 095 — platform-qualified Ada Float32 codecs

This change requires an explicit **deployment-target** opt-in:

```
--language ada --with-codec --ada-codec-runtime linux-x86_64-glibc-2.41
```

The portable analyzer and renderer APIs retain Float32 rejection. The new
`AdaCodecRuntime` and `*_with_runtime` APIs carry the same contract as the CLI.
Running the generator on Linux is not target qualification. On the target,
both public codec operations validate Linux/x86-64, glibc 2.41, binary32/C/Ada
ABI, binary64 writer storage and FE_TONEAREST. Unsupported targets/environments
fail with the support library's explicit `Unsupported_Platform`, not JSON null
or silent approximate values. The caller's rounding mode is never changed.

## Runtime dependency

Consumers of generated Float32 codecs must import the separate source project
`ada/oms-json-support/ams_gra_oms_json_float32.gpr`. It imports the existing guard
project and compiles the isolated C bridge. Float32-free generated codecs and
the guard-only GPR do not acquire this C dependency. The Ada model, service API
and public `Encode_*`, `Decode_*`, `Codec_Error` surface are unchanged.

The bridge uses glibc `strtof_l`, an explicit C numeric locale, checked buffer
length/capacity/termination, full token consumption and finite-result checks.
It distinguishes unsupported environment, invalid numeric input and locale
resource failure. Valid rounded subnormal/signed-zero underflow is not rejected
merely because libc reports ERANGE. The Ada boundary accepts only an exact
GNATCOLL-certified JSON number before calling C. OMS specials enter only via
the exact strings `NaN`, `Infinity`, `-Infinity`.

## Authoritative ingress, not token/DOM correlation

The occurrence API owns each original numeric lexeme in its structural node.
It never retrieves floating DOM numbers. Provider reference-counted objects
are used only as private tagged-node storage: numeric payloads are **strings**,
with kind and source positions recorded separately. Effective object members
are keyed by decoded JSON keys; replacing a duplicate replaces its entire
child subtree and token. Arrays preserve position. There are no global token
tables, name-only correlations or numeric-ordinal matching.

Original SAX parsing validates original grammar. Only SAX-certified numeric
spans are replaced in a same-length validation shadow by `0` plus spaces.
All other bytes and structural positions remain unchanged. The unchanged guard
validates that shadow for Unicode, size, depth and node limits. Shadow values
never supply typed data. The original is parsed into bounded occurrences and
typed model construction then uses only those occurrences. Limits also count
original duplicate occurrences, conservatively rejecting duplicate-heavy
resource abuse earlier than a collapsed DOM would.

Nested Records, existing Choice-of-Record composition, required/optional and
existing bounded/unbounded sequence machinery are reused. Named ranges use
the shared `floating_domain` classifier and checked `Model.Create`; extraction
uses `Model.Value`. Only expected predicate construction failure is translated.
Float64, Decimal, unsupported facets, direct/named Float32 Choice scalar
alternatives and the existing unsupported structural boundaries stay rejected.

## Platform and research provenance

Original stage-zero evidence demonstrated irrecoverable double rounding and
integer `-0` sign loss through GNATCOLL's DOM. That failure is not accepted as
new behavior. R1 showed exact SAX recovery but also disproved using either
Ada Float32 `'Value` candidate: 1,972 failures in the expanded 15,710-row corpus.
The production bridge avoids both conversion paths.

The qualified container base is the actually inspected Debian 13 amd64 image:
`debian@sha256:bcf83fd8af414eda0c0a99d25a4e61dedd88e7aac7f7bc519d04555ff0c40ec5`.
It reports libc `2.41-12+deb13u4`. The dedicated Fast job verifies architecture,
libc version, compiler versions, Alire 2.1.1 archive integrity, GNATCOLL 26.0.0
archive and source hashes, ABI controls and executable completion markers.
Ubuntu's host glibc is not used as Float32 evidence.

GNATCOLL source/license remains unchanged. The bridge additionally depends on
the target glibc API (LGPL-2.1-or-later); platform pinning and deployment review
are required. Other libc versions, musl, Windows, macOS or ARM64 are not admitted.
No universal ISO C rounding guarantee is inferred from this qualified host.

## Qualification sources

`tests/ada-float32-probe` preserves the exact R1 conversion/encoding corpora,
exact-rational oracle methodology, production bridge failure controls and
occurrence controls. `scripts/check-task095.py` drives actual generated Ada
using Task 095-owned fixtures, validates model/API parity and requires exact
completion counts, policy-specific locks and evidence hashes. It supports
OAM/non-OAM and both GNAT compilers under Normal/Check/Ignore. Standalone
support tests do not substitute for generated-code execution.

The numerical contract preserves IEEE value and signed zero, not arbitrary
source decimal spelling or NaN payload bits. Finite output uses the existing
GNATCOLL writer after exact widening; specials are strings, never null.

Full local publication gates and hosted exact-head checks must finish before
claiming production support. Research success alone does not authorize a PR
or readiness claim for real deployed Service Contracts.

## C1 local qualification checkpoint

Baseline: `d6d5b01747be562e7c86a21d8df8deeaaff8bbc9`, independent of open
hexBinary PR #94. Original stage-zero and R1 manifests remain unchanged.
Qualification evidence root: `/home/zboll/task095-scratch/c1/`.

- Both GNAT 14.2.0 and 16.1.0 × OAM/non-OAM × Normal/Check/Ignore:
  **12 generated executable cells**, each 96 checks and 14 controlled
  rejections; all pass. Named checked predicates still reject under Ignore.
- Every generated cell also executes the complete preserved **15,710 numeric
  conversion** and **5,552 finite encoding** corpora, with zero production
  mismatches; 27 explicit occurrence controls pass in each cell. The 1,972
  mismatches of each rejected Ada `'Value` candidate remain expected evidence,
  not acceptable production rounding. Exact rational oracle rechecks all
  conversion expectations and serialized values.
- C interoperability controls: 20 checks, including ABI sentinels, changed
  locale/rounding, unsupported libc, failed locale allocation, buffer capacity,
  embedded NUL, overflow and signed underflow. All pass.
- **28 inherited synthetic campaigns** and **18 inherited real-UCI Task 092
  executable cells** pass with both compilers/all policies. Task 092/094 frozen
  fixtures and completion markers are unchanged. Historical Float32-negative
  controls remain valid under the unchanged portable default; none was deleted
  or weakened to admit the qualified profile.
- Rust 1.95 fmt/check/Clippy/all-target workspace tests pass: 162 test summaries,
  **1,417 passed, 0 failed, 8 ignored**. Exact matching-main baseline:
  **1,414 passed, 0 failed, 8 ignored**. Ignored tests are not claimed executed.
- Cross-language existing-fixture preservation: 130 comparisons (120
  byte-identical, 10 matching atomic rejections), plus 25 byte-identical
  repeated-Choice/temporal/composition comparisons. New Float32 model/API
  parity is checked independently in every executable campaign. Guard sources,
  existing model renderer, Rust implementation and Deep CI remain unchanged.
- CI split/layout integrity passes, including **210 split controls** and
  layout mutation controls. The dedicated pinned-container job is additive.

### Pinned real-UCI impact

Both root hashes are verified before normalized-IR analysis. Each release has
four named constrained Float32 declarations, all supported by `floating_domain`:
`IFF_BarometricPressureType`, `SpoilFactorType`, `UnitBallFloatType`, and
`UnitIntervalFloatType`. Neither has a named unconstrained Float32 declaration.
Each has 56 direct Float32 members (48 Record, 8 direct Choice), 9 named Float32
members and no repeated Float32 fields. No unsupported named Float32 facet
profile occurs in these roots; direct Choice scalar alternatives remain out
of scope.

Static semantic reachability reaches 162 messages in 2.5 and 163 in 2.6.
Each schema is loaded once per analyzer run; per-message projections and
codec checks are checkpointed without repeated whole-schema coverage builds.
The matching-main/current comparison measures 325 closures per variant:
277 gain codec-renderable declarations, 311 remain codec-blocked and 14 retain
the exact same projection errors. **No newly codec READY closure or real
service is claimed.** Remaining exact first blockers are recorded per message;
they include Float64, Binary and polymorphic Ada values. The smallest beneficial
real closure, DLZ, is model READY in both releases and improves from 46/53 to
47/53 codec-renderable declarations, retaining
`AnglePositiveType: unsupported Ada codec primitive or facets: Float64`.

These analysis-only single-message contracts are not proof of deployed Service
Contract selection. Semantic closure, projection-generated support and codec
blockers are separately recorded. No ignored projection error is counted as
readiness, and no new real-service executable claim is made.

Hosted exact-head Fast/Deep results, mergeability and PR identity are recorded
separately at publication; local completion is not a hosted success claim.

## C2 corrective review

Reviewed source: `8783fd73912851b6bd0f4bd455226d83d9d51f79`.
Fresh evidence: `/home/zboll/task095-scratch/c2/`; original stage-zero/R1/C1
evidence remains immutable. The optional review archive was not locally found
and was not consumed. Its reported glibc -u3 C reproduction is not presented
as C1/C2 evidence. C2 reproduced against the inspected Debian -u4 environment.

### Identifier corruption

The reviewed post-render `occurrence_ingress` replaced unrestricted substrings
inside complete Ada source. `PathLength`, `TargetGet` and `ArraySample` passed
model generation, qualified service-check and codec generation, but actual
GNAT 14.2.0 compilation rejected the generated dotted decoder names with
"child unit allowed only at library level". The intentional red preserves
declarations, calls, bodies and end identifiers; it is not merely a Python
replacement demonstration.

The correction removes post-render rewriting. An explicit `Decoder` context
selects value/array types, extraction/index/length/traversal functions, reader,
error translation and public-operation environment checks at each emission
site. Encoder JSON types, schema names, model calls and strings never undergo
rewriting. No schema names are newly forbidden. DOM/portable rendering remains
the default and Float32-free output has no new runtime dependency.

### Floating environment

Reviewed bridge platform/ABI checks accepted all four DAZ/FTZ combinations.
The original generated Ada encoder lost subnormal values under DAZ; FTZ-only
preserved the observed widening values and is not claimed to exhibit identical
loss. The C probe initially allowed optimizer reuse of an earlier widening;
that observation is retained, and a separate runtime cross-translation-unit
widening demonstrates the DAZ loss without assuming constant folding behavior.

The runtime now requires **FE_TONEAREST, DAZ off and FTZ off**. Guarded x86
intrinsic `_mm_getcsr` inspects named SDM-documented bits on every bridge call,
before data-dependent floating operations. No approval is cached, no controls
are cleared and no exception masks are changed. Existing status ABI and Ada
unsupported/validation/resource exception separation remain unchanged.

Tests run each DAZ/FTZ combination in its own process, record control state
before logging, and restore the harness's saved MXCSR on success/failure.
Supported conversion may change ordinary arithmetic sticky status flags;
control-state preservation is tested separately rather than asserting a new
sticky-flag contract. Rejected bridge outputs retain sentinel bits, Ada support
raises Unsupported_Platform, and generated encode/decode publishes no result.
Restoration and a subsequent successful operation prevent startup-only checks.

Task 095-owned identifier and environment consumers supplement the unchanged
96-check/14-rejection generated corpus, 15,710 conversion vectors, 5,552
encoding vectors and 27 occurrence controls. Both compilers, both namespaces
and all assertion policies are mandatory, with an additional optimized
environment campaign. Exact final results, corrective SHA, per-command exits,
source manifests and preservation records are recorded in C2 evidence and the
existing PR; no historical UCI inventory is relabeled as corrective execution.
Analyzer, schema, projection, eligibility and UCI-selection rules are unchanged.

### C2 final local results

The final frozen-source audit passed: 12 existing compiler/profile/policy
cells (96 checks/14 rejections unchanged), 12 adversarial-identifier cells
(12 typed/string checks each), and 6 additional optimized environment cells.
Each environment cell executes all four independent DAZ/FTZ modes: supported
mode has 49 checks; each unsupported mode has 43 checks/16 encode/decode
rejections. Every existing cell also passes all 15,710 conversions, 5,552
encodings and 27 occurrence controls. C environment modes pass 32 supported
or 35 unsupported checks, alongside the original 20 bridge controls.

All 28 inherited synthetic campaigns and 18 Task 092 real-UCI executable cells
pass. Rust 1.95 fmt/check/warnings-denied Clippy/workspace passes with 162 test
summaries: **1,418 passed, 0 failed, 8 ignored**. Ignored tests are not claimed
executed. Three-way reviewed/main/corrected generation preservation completes
150 comparisons: 139 byte-identical, 11 matching atomic rejections; the existing
qualified Float32 fixture is also byte-identical. The malformed identifier
fixture is intentionally corrected, not subjected to malformed-byte parity.
CI split/layout/mutation checks pass, including 210 split controls. Existing
Fast/Deep jobs are retained; the Float32 job now runs identifier/environment
and optimized regressions in addition to the original full corpus.

Old/new renderer SHA-256:
`8783fd7:crates/backend-ada/src/service_codec.rs` is retained in
`/home/zboll/task095-scratch/c2/reviewed-identities.sha256`; corrected renderer
is `59733955b6c1f8de417794249020fc1a37b6081f1fcb1ada63f2b8962f70018c`.
Corrected C bridge is
`12dab58c5914ebcd0a108b9ab10f35667ea70d5bef71140f3781089b01f14efe`.
Final per-command records, compiler/runtime identities, locks, source hashes
and atomic results are in `final-matrix-02`, `local-qualification.json` and
the C2 final evidence manifest. The initial marker-variable runner failure and
missing-HOME launcher failure are preserved as failed development attempts,
not passing qualification. No production sources changed during final runs.

## C3 independent rounding-control correction

Reviewed head: `9c07eddc86658a84f3dce44645c5d447c4be6f9a`.
Fresh evidence is `/home/zboll/task095-scratch/c3/`. The optional
`pr96_c2_rounding_review.zip` was not locally available and was not consumed.
Original stage-zero/R1/C1/C2 records remain immutable.

The reviewed bridge accepted three independently mismatched environments:
x87 nearest with MXCSR downward/upward/toward-zero. On the inspected local
glibc `2.41-12+deb13u4` runtime, `fegetround()` reports x87 nearest while
platform/ABI/parse/special return success. O0 and O2 observations preserve
both raw controls and midpoint/signed-zero results. These sampled conversions
did **not** demonstrate numerical corruption. The defect is acceptance outside
the declared nearest-only contract, not a manufactured numerical expectation.
The supplied review's -u3 runtime is not conflated with this -u4 binary.

The minimal production change reads MXCSR once per platform invocation and
checks `_MM_ROUND_MASK` against `_MM_ROUND_NEAREST`, in addition to DAZ/FTZ
rejection and the existing `fegetround() == FE_TONEAREST` requirement. Intrinsic
constants describe MXCSR bits and are not compared to FE_* encodings. Both
**x87 and MXCSR must be nearest**, DAZ/FTZ must be disabled. Existing width,
ABI, libc and compiler checks remain; production never changes controls or
caches approval. Conversion and serialization algorithms are unchanged.

Test-only helpers use full `fegetenv`/`fesetenv` save/restore plus explicit
MXCSR restoration, verified even from an initially mismatched pair. x87 RC
and MXCSR RC are selected independently through explicit encoding maps;
unrelated precision, flushing and mask bits are preserved. Each pair runs in
an isolated process. Controls are measured before logging; successful numeric
operations may set normal sticky exception flags, which are not asserted to
be preserved. Unsupported outputs use byte/integer sentinel checks and no
typed or serialized value is published. The harness restores both environments
on controlled failure/exception and proves subsequent supported calls succeed.

The full pair policy is the same in C O0/O2 and generated Ada:

| x87 / MXCSR | nearest | downward | upward | toward zero |
| --- | --- | --- | --- | --- |
| nearest | accept | reject | reject | reject |
| downward | reject | reject | reject | reject |
| upward | reject | reject | reject | reject |
| toward zero | reject | reject | reject | reject |

Separate C3 markers retain C2's existing 96/14, 27-occurrence, identifier and
DAZ/FTZ assertions unchanged. The runner requires all 16 distinct pairs for
each compiler/profile/policy cell, plus optimized OAM cells for both compilers.
The C runner requires both optimization levels and all 32 pair executions,
then reruns C2's bridge and independent flushing controls. Final counts,
commands, source hashes and atomic results are recorded in C3 evidence and
the existing PR after qualification; pending execution is not a pass.

The renderer, occurrence ownership, shadow validation, model/APIs, numerical
oracle, eligibility, schema/projection and UCI-selection sources are unchanged.
Historical C1 UCI findings and C2 unrelated inherited campaigns remain reused
historical results with that source-identity basis, not fresh C3 measurements.

### C3 completed local qualification

The frozen-source audit passed. C O0/O2 each execute all 16 pairs (32 isolated
processes), with 57 counted checks per process. Generated Ada executes all 16
pairs in 12 compiler/profile/policy cells plus 6 optimized OAM cells: 288
isolated processes. Nearest/nearest has 218 assertions and no rejections;
each other pair has 290 assertions and 45 expected unsupported-operation
rejections. All cells match the table above and pass control/output atomicity
and supported post-restoration execution. No numerical corruption is claimed
for the original sampled red conversions.

All existing 12 generated cells retain the exact 96/14 marker and pass all
15,710 conversions, 5,552 encodings and 27 occurrence controls; C2 identifiers,
DAZ/FTZ, locale, libc, ABI, buffer and allocation-failure controls pass unchanged.
Rust 1.95 fmt/check/warnings-denied Clippy/workspace passes: 162 summaries,
1,418 passed, 0 failed, 8 ignored (ignored is not executed qualification).
Fresh generation preservation against C2 completes 150 comparisons with the
same 139 byte-identical/11 atomic-rejection outcomes, and the original qualified
Float32 output is unchanged. CI split/layout/mutation checks pass, including
210 split controls. C2's 28 unrelated inherited synthetic campaigns, 18 real-UCI
cells and C1 UCI inventory are explicitly reused historical evidence, not rerun
or relabeled; their unchanged-input basis is recorded in reuse-accounting.json.

Reviewed C bridge SHA-256:
`12dab58c5914ebcd0a108b9ab10f35667ea70d5bef71140f3781089b01f14efe`.
Corrected C bridge SHA-256:
`ec7217befcf053dfd133d836c983c0153f82099caaa273fffd34553b4c8b5320`.
Runtime remains actual glibc 2.41-12+deb13u4, libc binary
`9792e3cbb541c8f44c7acf5f14f4022ea62998ecc787d326bed4d8b6547dfd92`.
Exact command/source/compiler/lock/pair records, final source/evidence manifests
and atomic local result reside under `/home/zboll/task095-scratch/c3/`.