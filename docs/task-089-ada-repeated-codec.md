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
