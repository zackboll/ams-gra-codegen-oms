# Task 089 — blocked model-first baseline

Status: **NOT_READY**. No production codec/readiness changes have been made.

## Dependency and isolation

Live inspection confirmed PR #87 merged, PR #88 open at
`5f06d7a121b059f7eb91311021a85ceeb68c4fef`, and sibling PR #89 open at
`722a99e06bcf059a52cf491450c587c482963163`.

Task starting SHA: `5f06d7a121b059f7eb91311021a85ceeb68c4fef`.
Branch: `feature/089-ada-repeated-codec`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task089`.
The worktree was created from `origin/feature/087b-ada-oms-codec`, not Task 088.
Existing worktrees and unrelated changes were preserved.

## Fixture and model consumer

The task-owned qualified OAM fixture selects one concrete payload through two
exchanges. The production dependency generator successfully emitted three model
files and one service API file without `--with-codec`, selecting five types.
The fixture contains bounded 0..3 and 2..3 checked Strings, fixed 2..2 checked
Records, zero-minimum unbounded direct Strings, positive-minimum unbounded checked
Strings, repeated enumerations, direct signed bytes and unsigned bytes, Booleans,
unbounded checked Records, and an inherited repeated member. Wire name `begin`
is emitted as `Field_begin`. Populated and zero-occurrence JSON documents were
authored independently and both pass Python JSON validation.

The model-only consumer uses directly initialized Ada 2022 iterated aggregates
for bounded value arrays and the complete positive-minimum `Required` prefix.
It uses only public `To_Sequence`, `Append`, `Length`, and `Element` operations,
apart from the publicly defined required-prefix components. No checked carrier
placeholder is constructed. Empty storage is inspected for phantom elements;
the unbounded containers grow to eight elements and preserve order.

## Blocking defect: direct String vector equality

The **unmodified generated model** fails compilation with both GNAT 14.2.0 and
16.1.0 under normal, Check, and Ignore assertion policies (six build failures,
each exit 4). At `programs-oam.ads:423`, it emits:

```ada
package RepeatedPayload_Words_Vectors is new
  Standard.Ada.Containers.Indefinite_Vectors
    (Index_Type => Positive, Element_Type => RepeatedPayload_Words_Item);
```

The item subtype is `Ada.Strings.Unbounded.Unbounded_String`. GNAT reports:

```text
instantiation error at a-coinve.ads:45
no visible subprogram matches the specification for "="
default "=" on "Unbounded_String" is not directly visible
```

`render_unbounded_helper` in `crates/backend-ada/src/lib.rs` supplies explicit
equality for unsigned/floating/Binary elements but not direct String. This is a
model-generation defect, not a checked-carrier initialization defect or a codec
failure. Task 089 explicitly forbids modifying this model renderer and requires
the direct String storage shape, so implementation is blocked pending a scoped
upstream correction or explicit authorization for a model-renderer correction.
The fixture has not been weakened to conceal the defect.

For diagnosis only, six disk-backed scratch copies of the generated spec were
given the explicit generic actual:

```ada
"=" => Standard.Ada.Strings.Unbounded."="
```

All six diagnostic builds and executions then passed (exit 0) with exact marker
`TASK089_MODEL_BASELINE_PASS`. These are **not production qualification passes**:
the generated spec was modified in scratch. They isolate the missing equality
actual and demonstrate that the proposed checked-carrier aggregate construction
works on both compilers under all three policies.

## Evidence and remaining gates

Evidence root: `/home/zboll/task089-scratch`. The production generated model is
under `model`; twelve isolated final probes are under `disk-probe-*`, each with
compiler output, build log, and build exit status. Diagnostic successes also
retain run logs and exit statuses. Summary: `model-blocker-evidence.json`.
Initial generation log: `/tmp/task089/model-generate.log`.
An initial diagnostic attempt exhausted `/tmp`; it is not counted as successful
qualification. Final probes use `TMPDIR=/home/zboll/task089-scratch/tmp`.

Dependency generator executable SHA-256:
`5370420d3ab641098593f4f2e15cd99c7052bf3f69b399b1a40e61d101e45814`.

Production codec/readiness implementation, codec execution consumer/wrapper,
Alire codec builds/locks, hosted gate, real UCI measurement, full regression and
byte-parity campaigns, final certification, push and PR publication are not
complete. No codec READY claim, real-UCI improvement claim, or hosted success
claim is made. Task 087A sources, prior fixtures, model representation, Rust/C++
renderers, and Task 088 implementation remain untouched. Stop for review of the
model-first blocker; do not begin Task 090.
## Authorized correction continuation

The user authorized a narrowly scoped direct String equality correction in the
Ada model renderer. Equality selection now uses a match: direct String supplies
`Standard.Ada.Strings.Unbounded."="`; Binary and Interfaces associations are
unchanged and other types retain default equality. Public storage is unchanged.
The original failure evidence remains preserved.

The rebuilt production CLI generated the unmodified Task 089 fixture under
`/home/zboll/task089-scratch/corrected-model`. All six production model builds
and executions pass on GNAT 14.2.0/16.1.0 under normal/Check/Ignore, with exact
`TASK089_MODEL_BASELINE_PASS` markers. Evidence is in `corrected-model-*` and
`corrected-model-evidence.json`. No scratch source correction was used.
The new focused backend regression tests both zero- and positive-minimum direct
String sequences under all three policies on both compilers, including empty
storage, complete required prefix, append, length, element, assignment, copy
independence, and value preservation; all six executions pass.

Production CLI byte comparisons against the exact upstream generator pass for
`codec-ada-minimal` and `codec-oam` model/API outputs in Ada, Rust, and C++.
Evidence directories are `equality-parity-*`. Codec implementation and remaining
certification/publication gates are still pending; status remains NOT_READY.

## Repeated codec implementation and qualification continuation

Corrective commit: `6ccda2fd06771bd47913fe405bc2c3ff860b4bb0`.
The preserved fixture-first commit remains an ancestor.

Readiness supports bounded repeated Record members within the portable index
limit and unbounded members with portable required prefixes. It preserves
single/optional behavior and checks every effective inherited member. A visited
worklist checks the complete named repeated-element closure without exponential
recursive traversal. Choices, abstract values, temporal/Binary/float values,
nillability, unqualified members, unsupported facets and recursive planning
remain rejected. Rust/C++ readiness is unchanged. Task 088 is not incorporated.

Encoding uses the shared sequence operation constants and published model API,
never private Count/Items slots. Zero-length members are omitted. Positive-minimum
unbounded encoding emits Required then Additional and checks combined length.
Decoding requires arrays, validates count before construction, and uses existing
scalar and named decoders. Bounded Values and unbounded Required use Ada 2022
iterated aggregates, constructing each live checked element directly. Bounded
construction calls To_Sequence; its Constraint_Error translation covers only
that call, not aggregate initialization. Zero-minimum vectors and Additional
use Append in input order. A failing later element rejects the whole Record.
Member/index errors catch only Codec_Error; unexpected defects propagate.

The committed execution wrapper builds actual CLI-generated sources through a
clean Alire consumer with unchanged Task 087A support, validates Alire 2.1.1,
GNATCOLL 26.0.0 provider hash, GPRBUILD 26.0.1 release and compiler versions,
records exit statuses, generated hashes and policy-specific locks, and requires
exact `TASK089_CODEC_COMPLETE cases= 121` and `TASK089_MODEL_BASELINE_PASS`.
GNAT 14.2.0 and 16.1.0 each pass all three policies: 726 codec checks total.
Evidence: `/home/zboll/task089-scratch/qualify14-121` and `qualify16-121`.
The 121 checks include typed comparisons for every field, independent populated
and zero-count goldens, round trips, each bounded minimum/maximum and adjacent
invalid counts, required unbounded 0/1/2/3/4/12 counts, missing/null/scalar members,
null and wrong-kind items, numeric ranges, enum wire spelling, checked String
rejection, malformed later nested Records and Unicode, and unknown members.
Expected-invalid tests catch only Codec_Error. An initial structural comparison
failure was in the test oracle: provider programmatic String creation differs
from parsed String representation; comparing both through guarded write/read
fixed the oracle. Initial failure logs remain preserved, not certified.

The focused readiness suite passes 12/12; CLI codec tests pass 15/15, including
zero-OMS, payload deduplication, model/API parity and repeated unsupported-output
atomicity. Focused repeated-storage backend tests pass 3/3. Original Task 087B
41 main plus 7 subset cases pass with both compilers/all policies; original
Task 087A 199 guarded cases plus four boundaries retain frozen hashes and pass
both compilers/all policies. Evidence directories: `regression087{a,b}{14,16}`.
Eight original wrapper adversarial controls and Task 089 nonzero-exit control pass.

Byte comparisons against the upstream production generator pass for checked
String repeated storage, bounded storage, unsigned/floating/Binary unbounded
storage and unchanged naming. Ada/Rust/C++ representative model/API output,
Rust codec-oam model/API/codec and Ada codec-ada-minimal complete outputs are
byte-identical. Task 089 model/API is byte-identical with/without codec. Evidence:
`byte-parity-evidence.json`, `equality-parity-*`, `model-parity-*`, `codec-parity-*`
and `vector-parity-*`. Only direct String unbounded equality output changes.

A separate mandatory `ada-repeated-codec` hosted job depends on ada-json-guard,
checks out exact PR source head, builds the generator and runs the full corpus
with GNAT 16 under all policies. It uploads logs, exit statuses, source hashes
and Alire locks. Existing Task 087A/087B jobs remain intact. Hosted execution is
not claimed merely from registration. Fast/Deep CI split guard passes.

Rust 1.95 formatting, locked workspace all-target check, warnings-denied Clippy,
Python syntax, workflow YAML parsing and whitespace checks pass. Runtime
regression completed exit 0 with 119 passes and two existing real-input ignores.
The broad affected backend/core/CLI campaign is still running at this checkpoint;
its completion must be recorded before READY. Real UCI roots match the original
2.5/2.6 SHA-256 hashes. 2.5 currently measures 41/49 with DateTimeType first
blocker and remains NOT READY. Endpoint is a repeated NetworkEndpointType Choice,
so it is deliberately not newly supported on this sibling branch. The 2.6
measurement and publication/final-HEAD certification are pending at checkpoint.

## Completed real UCI measurement

Both hash-verified roots completed production service-check. On this PR #88
sibling baseline, both releases remain **41/49 codec-renderable declarations**
and NOT READY (expected CLI exit 1). First remaining blocker is DateTimeType
for 2.5 and NetworkEndpointType Choice for 2.6. Task 088's 46/49 measurement
includes independently developed Choice support; it is not the baseline here.
Endpoint remains a repeated Choice-valued Record member and is intentionally
rejected, even though the repeated storage shape itself is supported.
No complete real message becomes READY and no real compiled round-trip claim
is made. Logs: `rdma2_5.log`, `rdma2_6.log`, `rdma26-parallel.log`.

The exact newly supported non-Choice repeated members in this closure, on
SecurityInformationType in both releases, are SCI_Controls (0..20),
SAR_Identifier (0..unbounded checked String), AtomicEnergyMarkings (0..14),
DisseminationControls (0..22), NonIC_Markings (0..9), NonUS_Controls (0..3),
DeclassException (0..25), HighWaterNATO (0..5), CUI_Basic (0..93),
CUI_Specified (0..57), and SecondBannerLine (0..6). The finite members are
enumerations. All eleven per release passed isolated production readiness using
actual copied declarations and their complete named dependency closures.
Evidence: `uci-new-repeated-members.json` and `uci-member-*`. Isolated readiness
does not qualify the full SecurityInformationType containing unsupported Choices
and temporal members; hence whole-declaration renderability does not increase.

## Local completion checkpoint

The broad affected backend/core/CLI campaign completed with actual exit 0.
Runtime regression also completed exit 0. The final committed implementation
`ed9daecc2251c5dfa72326f879bdd27d86158c6d` was re-executed through clean consumers
on both compilers/all policies under `final14` and `final16`, each with exact
121-check marker, verified generated-source hashes and model baseline marker.
Production source is unchanged by this evidence-only documentation update.
All mandatory local categories are complete; publication and final published
HEAD parity still determine independent-review readiness. Hosted status remains
separate and must be observed on the published PR.

## Task 089R — integration with merged Task 088

Original reviewed Task 089 head: `8cac92d65e5a22a19672be68b918d44178771e90`.
Actual starting and accepted main: `37d3dc4f77e73d8218f2f53b6032d23c34573a9d`.
PR #88 and PR #89 were verified merged; main contains Choice commit
`722a99e06bcf059a52cf491450c587c482963163`. PR #90 was open, non-draft,
targeting main, with auto-merge disabled. The existing Task 089 worktree and
the primary worktree were clean. No reset, rebase, amend, squash or force-push
was used. This section supplements, rather than replaces, the historical
failure evidence and sibling-branch measurements above.

### Source reconciliation

The normal `git merge --no-ff --no-commit origin/main` produced a textual
conflict only in `crates/codegen-core/src/service_codec.rs`. The overlapping
Ada renderer and Fast workflow merged automatically and were inspected.
`AdaCodecAnalysis`, its emitted lookup, capability and Record-value caches,
cycle sentinels, and original dependency-visitation tests are retained.
Record fields admit the original portable bounded/unbounded cardinalities,
with unchanged local facets, qualification, nillability and abstract-value
checks. Repeated named values use the existing emitted lookup, `support` and
`record_support`; the independent repeated-element worklist is removed.
Choice remains forbidden as the immediate element of repetition, while a
concrete repeated Record may contain a supported required-single Choice.
Choice alternatives remain required-single.

The original Choice renderer and repeated Record renderer are both retained.
No private sequence storage is accessed. The public model and API remain
unchanged, including the authorized direct String equality association;
Binary and Interfaces equality mappings are unchanged. Rust/C++ renderers
are untouched. Fast CI retains Task 088 then Task 087B in
`ada-generated-codec`, including exact-head checkout and Choice artifacts,
and the separate `ada-repeated-codec` gate depending on `ada-json-guard`.
The latter also executes the focused combined qualification.

### Local completed qualification

Evidence root: `/home/zboll/task089r-scratch` (full logs, exit files, compiler
identities, dependency locks and generated-source hashes). Rust target,
temporary files, Alire settings, cache, toolchains and sandboxed dependencies
are task-owned. The initial symlinked toolchain cache was rejected by Alire
before compilation; copying the toolchains corrected environment setup.
Failed attempts remain preserved. Original qualification scripts and frozen
Task 087A hashes were not changed.

All ten Ada campaign/compiler combinations completed exit zero:

| Campaign | GNAT 14.2.0 | GNAT 16.1.0 |
| --- | --- | --- |
| Task 087A: 199 cases + four boundaries, frozen evidence | normal/Check/Ignore pass | normal/Check/Ignore pass |
| Task 087B: 41 main + seven subset checks | all policies pass | all policies pass |
| Task 088: 172 OAM + two qualified Choice checks | all policies pass | all policies pass |
| Task 089: 121 repeated checks + model baseline | all policies pass | all policies pass |
| Task 089R: generated combined-feature consumer | all policies pass | all policies pass |

Original campaigns: `isolated-{087a,087b,088,089}-{14,16}`. Combined execution:
`combined-retry-14` and `isolated-089r-16`. The first combined consumer build
exposed a handwritten test import typo (`Programs_OAM` versus `Programs.OAM`);
only the consumer import was corrected. No generator capability was broadened.
Both direct String unbounded shapes remain in the original model/codec corpus.

The combined fixture covers bounded scalar values, positive-minimum unbounded
checked Strings, a Choice containing the repeated Record, a Record containing
Choice alongside repeated values, and repeated named Records. Executed round
trips verify discriminants, order and semantic JSON equality. Negative controls
cover too few/many elements, invalid later scalar/String elements, missing and
unknown alternatives and multiple Choice members. CLI checks reject repeated
Choice members, repeated alternatives and unsupported nested values before
output publication. The new CLI codec suite passes 16/16.

The original Task 088 shared-DAG tests pass unchanged. The new 18-level,
two-edges-per-level repeated-Record DAG checks both successful and malformed
nested values, re-queries cached errors, and asserts each capability and
Record-value cache visit occurs at most once per declaration. Focused readiness
tests pass 17/17; the complete core campaign passes (353 library tests plus all
integration/doc tests).

Rust 1.95 formatting, locked workspace all-target check, warnings-denied
Clippy, complete workspace tests, and the separate complete Rust/C++/CLI/runtime
campaign all completed exit zero. The workspace includes the complete Ada
backend and generated model/codec regressions. Existing explicitly ignored
external-input tests retain their original configuration; no ignore was added.
Wrapper negative controls pass 8/8; Fast/Deep structural controls pass 209/209;
`git diff --check`, conflict-marker checks and Python syntax checks pass.

### Output compatibility

`byte-comparisons.json` records 25 production-generator comparisons against an
actual build of accepted main and the original Task 089 generator. They match
exit status and every generated file hash. These cover Ada/Rust/C++ model/API
and supported codec outputs for the Choice, qualified Choice, Task 087B,
codec-oam and repeated fixtures. Unsupported combinations remain unsupported.
All eleven generated Task 088 files from each compiler campaign independently
match the accepted-main generator byte for byte. Task 089 generated output is
unchanged; no integration output exception or frozen expectation update was
needed.

### Combined real UCI measurements

Both roots match the pinned SHA-256 values in the unchanged fetch scripts:
2.5 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`,
2.6 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
Production `service-check --with-codec` completed with expected exit 1 for
both releases: **46/49** renderable emitted declarations, **NOT READY**.
First blocker: 2.5 `DateTimeType`; 2.6
`RDMA_InitializeSetupMDT.Endpoint` (repeated Choice). These measurements are
recomputed, not sums of the sibling results. Neither complete message closure
is codec-ready, so no real compiled round-trip success is claimed.

The 2.5 declaration-level diagnostic reports exactly `DateTimeType`,
`RDMA_InitializeSetupMDT`, and `SecurityInformationType` unsupported; the latter
first fails at repeated `OwnerProducer`. The diagnostic uses a scratch copy of
the integrated production analysis without changing its checks. The 2.6
declaration-level diagnostic is recorded separately when completed.
Inspection of all repeated complex members in this selected closure finds only
immediate Choice elements (Endpoint, OwnerProducer, DisplayOnlyTo,
FGI_SourceOpen, FGI_SourceProtected, ReleasableTo), in both releases. Thus no
repeated Record element in this real closure becomes newly supported because
its value closure contains a Choice. Existing repeated scalar/enumeration
members remain supported; no schema was modified to remove blockers.

### Publication tracking

The corrective merge commit is identified by the normal two-parent commit
with subject `Merge Task 088 Choice codecs into Task 089 repeated codecs`;
its exact SHA and publication/hosted status are appended after commit creation
(a commit cannot contain its own hash). Fresh Fast and Deep results are
required on the published corrective head. Historical CI on `8cac92d` is not
certification of this integration. Auto-merge remains disabled, and PR #90
must remain open for independent review.
