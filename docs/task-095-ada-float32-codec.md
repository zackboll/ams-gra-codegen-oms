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