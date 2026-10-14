# Task 091 — checked Ada OMS JSON Duration codecs

Status: mandatory local qualification completed; publication/head parity must
be verified before **READY_FOR_INDEPENDENT_REVIEW**. Hosted CI is reported
separately, never inferred from job registration.

## Isolation and source contract

- Starting `origin/main`: `dd1353a93f4e1e62fa544f7fb7349a6eba41fc03`.
- Branch: `feature/091-ada-duration-codec`.
- Worktree: `/home/zboll/git/ams-gra-codegen-oms-task091`.
- Task-owned scratch/evidence: `/home/zboll/task091-scratch`.
- PRs #89 (Choice), #90 (repeated), and #91 (DateTime) were open at inspection.
  No sibling feature was cherry-picked or modified.
- The original working tree was clean. Main remained at the verified SHA on
  the subsequent fetch.

The authority is Task 057 section 7: OMSC-SPC-013 Rev B §6.1.4's otherwise
String case and §6.1.5.4's lexical characters. Duration is not Ada `Duration`,
not seconds, and not a numerical comparison domain.

The fixture-first production model/API generation, without `--with-codec`,
compiled and ran before any production source change on GNAT 14.2.0. Its
exact marker is `TASK091_MODEL_COMPLETE cases= 10`. The unchanged baseline
generator's model also compiled and ran on GNAT 16.1.0.

Actual generated interfaces in `Programs.Oam`:

```ada
function Create (Value : String) return SpanType;
function Value (Item : SpanType) return String;
function Create (Value : String) return IntervalType;
function Value (Item : IntervalType) return String;
function Create (Value : String) return XML_Schema_Duration;
function Value (Item : XML_Schema_Duration) return String;
```

Present optionals use the generated discriminated `Is_Present`/`Value`
wrappers; absent variants never initialize a checked carrier. The model,
temporal classifier, Rust/C++ backends, frontend, and JSON guard are unchanged.

## Production implementation

Only the Ada capability path accepts Duration. Named declarations must pass
`temporal_profile` with `TemporalProfile::Duration`; direct members must pass
`direct_temporal_profile` with `DirectTemporalProfile::Duration`. Unsupported
facets remain rejected, including patterns, numeric bounds, lengths, and an
explicit whitespace facet. Type names have no semantic significance.

Named `To_JSON_*` uses `Create (Model.Value (Item))`. Named `From_JSON_*`
extracts a JSON String before entering a nested block whose only operation is
`return Model.Create (Text)`. The expected function return type resolves the
overloaded constructor. Only constructor `Constraint_Error` is converted to
`Service_Codec.Codec_Error`; extraction and Record decoder errors are not
blanket-caught.

Direct fields use a codec-local `Scalar_Duration`, returning
`Model.XML_Schema_Duration`, with the same narrow boundary. It is emitted only
when `emissions_emit_direct_duration` over the existing emission plan says the
projected model stores direct Duration. No second emission plan is computed
for helper presence. The name is reserved in codec naming preflight.

The original projected-emission, generated-support, inheritance/storage,
wire-QName, deterministic blocker, and payload deduplication decisions remain.
Choice, repeated fields, Time/TimeZulu, DateTime, binary, floating-point,
nillable, abstract/polymorphic, unsupported alias/list, and unqualified wire
members remain outside this sibling baseline's Ada codec capability.

## Executable qualification

`scripts/check-task091.py` always invokes the real production generator. It
generates model-only, OAM codec, qualified non-OAM codec, and named-only codec
selections. The qualified fixture and golden are transformations of the
independently authored synthetic source/goldens, never codec-produced JSON.
The two exchanges select the same payload and yield exactly one public
Encode/Decode pair.

Each compiler runs normal, `Assertion_Policy(Check)`, and
`Assertion_Policy(Ignore)`. Required versions: Alire 2.1.1, GNAT 14.2.0 or
16.1.0, GNATCOLL 26.0.0, GPRBUILD 26.0.1 release (executable reports 26.0.0).
Dependencies are built in isolated workspaces. Logs, explicit commands/exits,
compiler identities, locks, and generated Ada source hashes are preserved.
Missing executables, nonzero exits, incorrect markers, or dependency identity
changes fail qualification.

Per compiler/policy: model **10** checks; each of OAM, qualified and named-only
codec consumers **650** checks. The frozen corpus's **100** entries execute
through six actual codec field paths. Its immutable SHA-256 is
`1f723a55b4c79bc9e879c6d8ee1d7fa1b21d1e1d9b2dd725f763a8b9ab4222d7`.

Coverage includes two differently named declarations; required/present/absent
optional values; inherited and nested direct/named Duration; independent
structural equality; reversed object order; whitespace collapse; fractional
trailing zeros; negative zero; `P12M` distinct from `P1Y`; long component
digits including an 8,192-digit component; missing/unknown members;
non-string/null values; invalid nested Duration; malformed Unicode; oversized
documents; and excessive nesting. Negative tests catch only `Codec_Error`.
The provider's existing last-value-wins duplicate-key policy is unchanged.

Completed intermediate compiler-backed campaigns are preserved under
`certified14` and `certified16`. The final command/exit-recording and explicitly
workspace-isolated campaigns completed exit zero under `final-isolated14` and
`final-isolated16` (61 successful command records each). Both exact compiler
identities, all policies, all four selections and all completion counts passed.

## Regression evidence

The unchanged Task 087A guarded corpus and boundaries passed with both
compilers under all three policies, preserving the frozen output hash
`67a60d687e5fa1ad660a9b0322a4f9588f0107925e6abd4fe15353f8777c3036`.
Clean consumer and eight wrapper negative controls passed. The unchanged
Task 087B **41**-check consumer and **7**-check subset passed on both compilers
under all three policies, including unsupported-output atomicity.

The baseline/current generator comparison records **25** selections, with
identical exit statuses and SHA-256 maps in `byte-preservation.json`: Ada
models/APIs, original minimal Ada codec bytes, Rust models/APIs/codecs, and
C++ models/APIs. No-Duration codec bytes remain unchanged. The new model/API
is also compared byte-for-byte with/without codecs. Named-only selection
executes successfully without any direct helper/carrier.

Core readiness **11/11** and CLI codec **14/14** focused tests passed, including
zero-OMS and unsupported/collision atomicity. Rust 1.95 formatting, locked
workspace all-target check, warnings-denied Clippy, Python/workflow/JSON
syntax, and `git diff --check` passed. CI split controls passed **209** checks.

The initial affected suite campaign failed because an existing CLI test uses
hardcoded `/tmp/task085`, while `/tmp` is full. Exact failure evidence is
preserved. The retry uses a private mount namespace binding task-owned
disk-backed scratch at `/tmp`, without changing tests or removing existing
evidence. That retry completed exit zero: **1,080 passed, zero failed, six
pre-existing ignored**, across 112 test summaries. No newly added test is
ignored. Final focused tests, locked all-target check and warnings-denied
Clippy were rerun after the pattern-rejection control was added.

## Real UCI and hosted CI

The cached authoritative roots were hash-verified:

- UCI 2.5: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`.
- UCI 2.6: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

Actual baseline/after measurements use the same immutable roots and baseline
generator. All commands return the expected NOT READY exit 1.

| Selection | Selected declarations | Before → after codec declarations | First blocker |
|---|---:|---|---|
| RDMA_InitializeSetup 2.5 | 50 | 41/49 → 41/49 | DateTimeType |
| RDMA_InitializeSetup 2.6 | 50 | 41/49 → 41/49 | NetworkEndpointType Choice |
| Log 2.5 | 42 | 34/41 → 34/41 | DateTimeType |
| SurvivabilityRiskLevel 2.5 | 45 | 35/44 → 36/44 | DateTimeType |

An independent pinned-XSD dependency walk confirms neither RDMA selection
reaches Duration, so unchanged readiness is expected. Log's direct optional
`LogMDT.ServiceUpTime` is newly supported, but its Record still has other
unsupported members, so this partial improvement does not increase the
whole-declaration count. No real selected service above becomes codec READY.

The smaller named-Duration `SurvivabilityRiskLevel` 2.5 reaches `DurationType`
(established independently from pinned XSD), selects 45 normalized declarations,
and improves from 35/44 to 36/44 as `DurationType` becomes codec-renderable.
The first blocker remains DateTimeType, and the service remains NOT READY.
No real service round trip is claimed or required for a non-READY selection.

The isolated Fast CI job `ada-duration-codec` depends on the untouched
`ada-json-guard`, checks out exact PR head, builds the production generator,
runs GNAT 16 qualification under all policies, and uploads logs, source
hashes/sources, commands/exits, and dependency locks. It downloads no real
UCI or Sleet data. Hosted execution is not yet claimed.

Publication and final tested/local/remote/PR head parity remain gates.