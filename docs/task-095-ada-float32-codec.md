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