# Task 093 — Ada OMS JSON hexBinary codecs

## C3 — Integrate merged Float32 without restricting portable Binary

Starting feature `31bdccf635008415434a5112cd443570abe6fb47`; exact integrated
main `0e4d25718915496a7e01de4c4a689a8f739cd7a7`, containing merged PR #96 and
reviewed Float32 C2/C3 head `5aff250117d1bf219480e8ef0bc4a74f411f3957`.
Ownership verification identified the clean original Task 093 worktree at
`/home/zboll/git/ams-gra-codegen-oms-task093`; local, fetched remote and PR #94
heads agreed. PR remained OPEN, non-draft, unmerged, base main, auto-merge off.
No sibling worktree was changed. Main/feature baselines are immutable source
archives with independent build areas, not source-worktree switches.

### Semantic reconciliation

The controlled `git merge --no-ff --no-commit origin/main` conflicted only in
the core analyzer and Ada codec renderer. Resolution keeps Binary provenance
and length-domain classification before generic runtime-aware scalar dispatch,
retains runtime context and memoized deterministic analysis, and retains both
Binary and Float32 Choice-scalar rejection. Repeated Choice of a concrete
Record containing single Binary stays supported, not repeated scalar Binary.
Main's occurrence ownership, decoded duplicate keys, original numeric lexemes,
guarded validation, helper reservations, and explicit Decoder rendering remain.
No production floating-environment source was modified.

`hex_binary_helpers(Decoder)` explicitly renders only the decoder parameter
type: `JSON_Value` for portable DOM, `Ingress.Value` for occurrence ingress.
Both call the correctly typed `Scalar_String`. The encoder remains
`Encode_Hex_Binary (...) return JSON_Value` in both modes. No completed Ada,
schema-derived identifier, or whole-body template rewriting is used. Existing
uppercase encoding, mixed-case input, XML whitespace, octet bounds, checked
constructors, imports, atomicity and public interfaces remain unchanged.
Portable output matches the original feature bytes; Float32-only output
matches current main. Binary data never enters numeric token replacement.

| Selection | Portable | Explicit qualified runtime |
|---|---|---|
| Admitted hexBinary, no Float32 | READY; ordinary guard GPR | Same portable bytes; no platform restriction |
| Admitted Float32, no Binary | Rejected atomically | READY for established supported shapes |
| Admitted Binary + Float32 | Rejected atomically | READY with occurrence ingress and Float32 GPR |
| Unsupported Binary provenance/shapes/facets | Rejected | Rejected |

Unknown/Base64 provenance, named/field-local unsupported facets, direct/named
repeated Binary, Binary scalar Choice alternatives, Float32 scalar Choice and
unsupported facets/targets, Float64/Decimal, invalid cardinalities,
nillable/unqualified values, missing emissions and cycles remain fail-closed.
Core module: 30 tests. Complete CLI service-codec suite: 21 tests. Ada backend:
99 passed in 17 summaries (64 library). Portable actual generated-Ada smoke
runs outside the Float32 container with GNAT 16.1.0; host runtime identity is
recorded, not inferred from the container.

### New C3-owned composition

`codec-ada-hexbinary-float32.{xsd,yaml}`, `task093-mixed*.json`,
`hexbinary_float32_{consumer,environment}.adb`, and `check-task093-mixed.py`
are new Task 093 fixtures; frozen Task 092–095 fixtures/corpora are untouched.
The schema composes repeated named Choice, required-single Record alternative,
direct/optional/unconstrained named/checked named Binary, direct/checked/repeated
Float32, and repeated DateTime/Duration. PathLength, TargetGet and ArraySample
exercise identifier safety. Namespace transformation is structural/reversible,
retains version metadata in OAM, and changes only JSON keys without reparsing
numeric input lexemes.

All 12 compiler/profile/policy cells generate, compile and execute: GNAT
14.2.0/16.1.0 × OAM/non-OAM × Normal/Check/Ignore. Each reports
`TASK093_C3_MIXED_COMPLETE checks= 95 rejections= 26`, checking independent
canonical JSON, exact octets including zero/high-bit, empty/optional absence,
checked length boundaries, exact midpoint/signed-zero/subnormal bits, checked
Float32 ranges, ordering/nested access, invalid later values, escaped decoded
key and whole-subtree duplicates, malformed overwritten JSON, Unicode/depth/
document limits, 100,000-octet unconstrained Binary and failure atomicity.

Each cell also reports `TASK093_C3_ENVIRONMENT_COMPLETE checks= 770
rejections= 126`: all 64 independent DAZ/FTZ/x87/MXCSR combinations, generated
encode/decode operations, unchanged typed and serialized outputs on rejection,
unchanged control state, restoration and subsequent successful operations.
These consumers reuse merged test-only C harnesses; no new production floating
environment implementation or exception suppression was introduced.

Qualified container base:
`debian@sha256:bcf83fd8af414eda0c0a99d25a4e61dedd88e7aac7f7bc519d04555ff0c40ec5`.
Actual provisioned image ID:
`3be2d67b84bc116c44fa08d25714359dfd5aff95fddc3c8a270dd359633e03ab`.
Actual libc package `2.41-12+deb13u4`, binary SHA-256
`9792e3cbb541c8f44c7acf5f14f4022ea62998ecc787d326bed4d8b6547dfd92`;
GNAT external 14.2.0 (`gnat-14 14.2.0-19`) and native 16.1.0; Alire 2.1.1;
gprbuild release 26.0.1 (banner 26.0.0); GNATCOLL/minimal 26.0.0,
`gnatcoll-json.adb` SHA-256
`7e07277731a44ea08716fd61a6462ebe7a7ce0722adc6dc787cbbaafd3099063`.
C/Ada ABI, C locale and allocation/capacity controls execute in the inherited
Float32 bridge/Require_Platform corpus; nearest x87/MXCSR, DAZ off and FTZ off
remain mandatory. Digest alone is not asserted to identify installed packages.

### Integrated qualification and preservation

The manifest enumerates 162 primary compiler/profile/policy cells plus 18
inherited campaigns and separate C bridge/rounding controls; counts are derived
from runners, not assumed. All synthetic lanes and the 18 real Task 092 cells
have definitive successful completion. Task 093 original, Binary/Choice/temporal
and extracted real Binary, Task 092 synthetic, Task 094 expanded/composition/
Choice-temporal/model-only, and inherited 087A/B–091R were reexecuted against
integrated source. The original completion markers remain unchanged.

Task 095 executes 12 original generated/numeric cells (96 checks/14 rejections,
15,710 conversions, 5,552 encodings, 27 occurrence controls each), 12 identifier
cells, six optimized environment/rounding cells, all DAZ/FTZ modes and 16
independent x87/MXCSR pairs. Separate C execution covers 32 O0/O2 rounding
pairs, four environment modes and 20 bridge controls. No production numeric
or writer mismatches. Known rejected Ada Value candidates retain their 1,972
conversion mismatches and are not the production parser. Corpus hashes remain
`a1b0a2a6f0a9c8f11371391930b716e04ca64c5072ef8b0fb12ce273f4d6c140`
and `a2b9a9c043e218c2e1cadf5118bb698c7e413ea66ef14fc0036ade8f3f682ccc`.
No Task 095 325-message impact campaign was rerun or relabeled.

Three-way preservation uses A original PR head, B exact merged main, C pending
integrated source. 156 successful byte-identical generations are separated
from nine matching atomic rejections. Binary outputs compare to A, Float32 and
existing non-Binary output to B; Rust/C++, models/APIs, Choice/temporal retained.
Full Log model/API parity and default/explicit runtime-option byte parity are
separately measured. Mixed output is newly qualified composition, not an old
baseline. All full Log codecs use the default portable profile and have no
occurrence imports, C bridge or Require_Platform dependency.

Rust 1.95 locked all-target workspace fmt/check/Clippy/test commands exit 0:
1,425 passed, zero failed, eight pre-existing ignored, 162 summaries. Ignored
is not passed. CI split guard and 210 mutation checks pass; Task 077B layout
check and both 54-case profiles pass. Existing Binary Fast/Deep, Float32,
Choice, temporal, guard, Rust and Deep jobs remain. The mixed matrix is an
additional step inside the existing digest-pinned Float32 job; portable Binary
jobs remain outside. Main's C2/C3 Float32 regressions are unchanged.

### Fresh complete real Log

Unchanged contracts and pinned UCI roots:
2.5 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`;
2.6 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

| Selection | Selected/model | Main before | Integrated after | First blocker after |
|---|---|---|---|---|
| Log 2.5 | 42/42 | 40/41 NOT READY, LogMDT.Data Binary | 41/41 READY | none |
| Log 2.6 | 44/44 | 42/43 NOT READY, HexBinaryType Binary | 43/43 READY | none |

All 12 full Log compiler/release/policy cells generate, compile and execute with
`TASK093_LOG_COMPLETE checks= 29 rejections= 9`, unchanged from C2. Both
compiler/release qualification records and source manifests pass.
No deployed-service, transport-interoperability or broader UCI readiness claim.

### Evidence and publication

Fresh evidence is `/home/zboll/task093-scratch/c3/attempt-01`, including
ownership/merge records, qualification manifest, expected cells, immutable
baseline identities, qualified-input hashes, detached worker final records,
pre-fix correction classifications and final audit. Historical C1/C2 sections
below certify their recorded baselines only. Pre-fix test/golden/manifest
expectation errors are preserved and followed by fresh completed reruns.
No old green CI is represented as certification of this integration.

Final local audit: `TASK093_C3_LOCAL_AUDIT_PASS 1425 passed 8 ignored`.
All mandatory local lanes completed with exit zero; no active workers or
unresolved merge entries remain. The final source manifest binds 1,016 files;
SHA-256 `d826c400e4a07dc23f7bbede2066f3f489480237e8b8fd0c9a23c7352e777439`.
Merge identity, publication parity and hosted disposition are recorded in the
C3 publication evidence and existing PR #94. Leave open/unmerged; hosted
qualification must complete before independent merge-readiness claims.

## C2 — Integrate merged Choice/temporal codecs and requalify

### Repository identity and semantic merge

Starting feature HEAD: `99f8b65273f4e7b46aeb2c0bba839adfe1c31941`.
Exact current-main baseline: `d6d5b01747be562e7c86a21d8df8deeaaff8bbc9`,
including PR #93 (Task 092) and PR #95 (Task 094). `git worktree list`
identified the clean original Task 093 checkout at
`/home/zboll/git/ams-gra-codegen-oms-task093`; no sibling worktree was edited.
PR #94 was freshly verified OPEN, non-draft, targeting main, at the expected
head, with auto-merge disabled. Historical C1 reports/manifests were reviewed
before integration and remain untouched.

Ordinary two-parent merge: `983147e2fd9e3efac73683a82d2de14679f49407`.
No rebase, reset, clean, stash, force-push, replacement PR, or Task 095 work.
The only textual conflicts were:

- Core `service_codec.rs`: retained the Binary-specific required/optional
  branch and repeated direct/named Binary guards, removed obsolete repeated
  DateTime/Duration rejection branches, and retained main's repeated Choice
  allowlist, named scalar profile checks and cached dependency analysis.
- Fast CI: reconstructed the two independently owned jobs side by side,
  preserving both `ada-hexbinary-codec` and `ada-repeated-choice-codec` plus
  every existing job, including `ada-repeated-temporal-codec`.

The Ada backend auto-merged Task 094's explicit `Scalar_Date_Time` and
`Scalar_Duration` repeated conversion branches with Task 093's helpers. Binary
does not use integer conversion; temporal values do not use numeric fallback.
The Deep CI and mutation-script merges retain the executable duplicate-fetch
mutation inside `real-uci` and Task 092's job-local Task 077B hardening.
Task 089R/090R/091R, CLI service-codec tests, and frozen Task 092/094 fixtures
and runners are byte-identical to the new main.

Qualified production/fixture/runner source commit:
`9dc4e15e0596bdc9dbe52ff1f82dfc6c6f1c00ad` (the authoritative exact value is
also in `c2/qualified-source-commit.txt`). New evidence is isolated under
`/home/zboll/task093-scratch/c2`; no C1 result is relabeled against new main.

### Focused analyzer and actual combined executable

The updated core codec suite passes **27 tests**, including bounded/unbounded
repeated Choice containing a concrete Record with single Binary, repeated
direct DateTime and named Duration. Separate mutations reject repeated Binary
and Binary Choice alternatives. Existing tests retain temporal Choice
restrictions, named/direct temporal profile checks, invalid facets/cardinality,
unknown/Base64 provenance, nillable/unqualified, missing/cyclic dependencies,
case-insensitive helper reservation, cached closure and first-blocker checks.
Complete Ada backend tests pass **99 tests across 17 summaries** (64 library
tests); CLI service-codec tests pass **18/18**. GNAT 16 focused capability
campaigns 089/090/091/092/093/094 all pass before full campaigns launch.

Task 093 owns `codec-ada-hexbinary-combined.{xsd,yaml}`,
`task093-combined.json`, `hexbinary_combined_consumer.adb`, and
`check-task093-combined.py`; frozen sibling fixtures were not repurposed.
The schema contains repeated named Choice, a concrete Record alternative,
repeated direct DateTime, repeated named Duration, direct required Binary, and
optional named length-two-octet Binary. Binary itself is never repeated or
used as a Choice alternative. The non-OAM transformation reuses Task 094's
structural/reversible checks, preserving message version metadata in OAM.

All **12 combined cells** pass: GNAT 14.2.0/16.1.0 × OAM/qualified ×
Normal/Check/Ignore. Every cell compiles production-generated Ada and reports
`TASK093_COMBINED_COMPLETE checks= 49 rejections= 9`. Independent golden JSON
checks outer discriminants/length, nested temporal lengths/order/exact lexical
values, exact Binary octets including 0x80/0xFF, lowercase and XML whitespace,
canonical uppercase, empty unconstrained Binary, optional absent/present,
named length validation, and decode/encode/decode. Later temporal and Binary
errors reject without publishing a typed result and report `Picks item 3`.
Runner gates enforce model/API parity and one copy of each required Binary
helper, one direct DateTime helper and no unnecessary direct Duration helper.
Four independent generator controls (both CLI actions in both namespaces)
retain repeated direct/named Binary and direct/named Binary-alternative
rejection. Fast CI executes this additional corpus with separate evidence
and exact completion-marker checks, not instead of the original corpus.

### Preserved unsuccessful development evidence

`production-core.log` records an initial **test-only** incorrect expectation:
the established deterministic blocker includes `Selection.Nested:` before
`Details.Data`. The new test was corrected to assert that existing full path;
`production-corrected-core.log` passes 27/27. No production behavior changed.
`combined-smoke/` records an over-specific nested-index expectation failure:
the existing outer repeated decoder intentionally replaces inner diagnostics
with its own item index. An attempted direct nested decoder check then failed
to compile because only selected payload decoders are public
(`combined-smoke-corrected/`). The final independent corpus checks the supported
outer index without changing historical codec bytes; `combined-smoke-final/`
passes both namespaces, and `matrix-{14,16}-binary-combined/` passes all cells.
The new corpus's 49/9 marker reflects its actual authored assertions, not any
alteration to historical 093/092/094 markers. Scratch inventory harness compile
errors (`&Path` and a private method) are preserved separately; corrected
public-API harnesses both build and complete with atomic exit zero.
A first foreground debug readiness invocation exceeded the tool timeout;
fresh detached release measurements completed all eight required selections.
The redundant detached debug-only repeat was subsequently stopped at its
child command, after the release/inventory/full-service gates completed;
`redundant-debug-cancellation.json` and its atomic nonzero worker record retain
the reason and actual exit. It is not used as mandatory qualification evidence.
Neither the incomplete foreground log nor old hosted results certify C2.

### Fresh real UCI Log impact, against exact Task 094 main

Unchanged real root SHA-256 values:

- 2.5: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`
- 2.6: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`

The actual unchanged `real-duration-log.yaml` contract is used throughout.
`readiness-release/` preserves all eight baseline/integrated model/codec
commands, exits, reports and generator hashes. The independently built
`inventory-{baseline,current}.json` records every selected/emitted identity,
unsupported declaration and first blocker, plus temporal owner controls.

| Log | Selected / model renderable | Main codec | Integrated codec | Main first blocker |
|---|---:|---:|---:|---|
| 2.5 | 42/42 (both) | 40/41 NOT READY | 41/41 READY | `LogMDT.Data: unsupported Ada codec primitive or facets: Binary` |
| 2.6 | 44/44 (both) | 42/43 NOT READY | 43/43 READY | `HexBinaryType: unsupported Ada codec primitive or facets: Binary` |

Selected and emitted declaration lists are identical before/after. The sole
newly qualified declaration is `LogMDT` (2.5) or `HexBinaryType` (2.6); the
integrated codec has no unsupported declaration and no blocker. This delta is
against merged Choice/temporal main, not Task 091 or historical Task 093 alone.
Both full real Log services now require actual typed execution, separately
from the unchanged extracted Binary qualification.

`check-task093-log.py` generates the complete real model/API/codec, compares
model/API bytes with codec disabled, compiles actual generated sources, and
executes independently authored `task093-real-log{25,26}.json`. Checks cover
required/additional OwnerProducer Choice discriminants and value, timestamp,
Duration spelling, exact Binary bytes, absent/present/empty optional data,
lowercase/whitespace to uppercase canonicalization and typed round trips.
Nine invalid Binary lexical/kind/odd-length cases verify failure atomicity.
Real `LogMDT.Data`/`HexBinaryType` has **no octet length facet**: empty and
three-octet inputs are valid, odd lexical lengths reject; no invented maximum
or modified contract is imposed. Named octet length rejection is independently
qualified by original/extracted/combined Task 093 consumers, not falsely
attributed to full Log. Deep CI retains extracted evidence and adds full Log
execution/evidence with explicit markers.

### Qualification status

All mandatory local gates completed with atomic exit zero. Final audit:
`c2/local-qualification.json`, produced by `c2/audit.py`, checks the actual
completion markers, commands/exits, source hashes and locks, without relying
on historical hosted results. No detached C2 workers remain running.

| Campaign | Final C2 result |
|---|---|
| Original Task 093 OAM/qualified | 12/12 cells; 33 codec + 6 model checks per cell; original markers unchanged |
| New Binary/Choice/temporal | 12/12; 49 checks including 9 rejection cases per cell |
| Task 092 OAM/qualified | 12/12; 63 checks including 17 rejection cases per cell; missing/cyclic controls retained |
| Task 094 expanded temporal | 12/12; 54 checks including 17 rejections per cell |
| Task 094 composition | 12/12; 19 checks including 3 rejection cases per cell |
| Task 094 Choice/temporal | 12/12; 26 checks including 2 rejection cases per cell |
| Task 094 model-only | 6/6; 24 checks per cell |
| Task 093 extracted UCI 2.5/2.6 Binary | 12/12; `TASK093_REAL_COMPLETE cases= 14` |
| Newly READY full UCI 2.5/2.6 Log | 12/12; `TASK093_LOG_COMPLETE checks= 29 rejections= 9` |
| Task 092 real RDMA 2.5/2.6 and Survivability 2.5 | 18/18; original 12/12/8-check markers |
| Inherited 087A/B, 088, 089/089R, 090/090R, 091/091R | All 18 campaigns pass on both compilers, Normal/Check/Ignore |
| Rust 1.95 fmt / locked workspace all-target check / warnings-denied Clippy | All exit zero |
| Rust 1.95 locked workspace all-target tests | **1,419 passed, 0 failed, 8 pre-existing ignored**, 162 summaries; ignored tests not counted as passed |
| Task 087A guard wrapper | 8/8 pass |
| CI split / mutation / Task 077B layout | All pass; **210** checks, both **54-case** profiles; no expected count changes |

The primary new/regression matrices total **120 compiler/policy cells**, in
addition to the 18 inherited campaigns. Each two-profile codec corpus uses
GNAT 14.2.0/16.1.0 × OAM/qualified × Normal/Check/Ignore. Real services and
extracted real Binary use both releases × both compilers × all three policies.
Model-only uses both compilers × all three policies. Alire 2.1.1, GNAT
14.2.0/16.1.0, GPRBUILD package 26.0.1 (executable 26.0.0), and the established
GNATCOLL/minimal 26.0.0 hash-pinned solution are checked by the runners.

### Exact-main output preservation and final identities

`preservation/comparison.json` records **58 byte-identical successful
generations**, **two permitted hexBinary codec additions**, and **nine matching
atomic rejections**. Ada model/API, Binary-free codecs, Task 092 Choice codecs,
Task 094 repeated-temporal codecs, Rust model/API/codecs and C++ model/API
bytes match exact new main. Five helper controls retain absence of Binary
helpers from Binary-free output and unchanged temporal helper cardinalities.
New combined consumers separately verify required Binary helpers occur once.
The full real Log model/API additionally matches exact main for both releases
(`real-preservation/comparison.json`). Before/after manifests preserve every
generated file hash and exact command; no unexpected rename/removal is allowed.

`qualified-input-hashes.json` binds **897** production, support, runner,
fixture, test and workflow files; SHA-256:
`b136eff39af76be38153ade0060c34a662dfb985810fe33bb83e3deaa0cb415e`.
Qualified generator SHA-256 values:

- Debug: `f0a31e00021a8fcbf3f6cff13e8c7f77135c65722556f06c82e4807efc1d02b9`
- Release: `5704638d503f6c6cc69a4433cb8a5865b08f0792ff88c7d5a2405b752eacfb84`
- Exact-main Git archive:
  `b44188112e1d4bfb14cd912514801def97dbb601184b337aea7a83f4a1054a46`

Generated Ada source hashes, independent goldens, actual roots, generators,
command results, tool banners and dependency locks are retained beside each
campaign. All final input hashes match qualification. Focused checks and
preservation measurements preceded only workflow additions (not production,
fixture or runner changes); the final CI lane independently binds the final
workflow bytes. Those earlier source manifests are retained, not rewritten.
Fresh real temporal owner controls remain identical: PRI_DataBaseType 6/6
READY; ScanDataBaseType 10/13 with its existing Float64 blocker, both releases.
All non-Task093 worktree identities match initial discovery.

Final pre-publication fetch still reports main
`d6d5b01747be562e7c86a21d8df8deeaaff8bbc9`. The PR diff contains only original
Task 093 and its owned integration qualification/CI/documentation additions.
Publication targets existing PR #94 only, via normal push, leaving it open,
non-draft and unmerged with auto-merge disabled. Fresh exact-head Fast and Deep
CI must be checked after publication; local success is not hosted certification.
Final commit/head parity, mergeability and hosted status are recorded in the
publication evidence and final report, not predicted here.

## C1 correction and qualification (current work)

C1 evidence is isolated under `/home/zboll/task093-scratch/c1`; all original
failed schemas and logs above that directory remain untouched. Starting HEAD
and both sibling PR HEADs were freshly verified unchanged. No sibling work
was integrated, reset, rebased, or modified.

### Stage-specific corrections

`parse_local_element` rejects child declarations before resolving `type`.
The original inline restriction therefore tests **frontend** rejection, not
Binary model/codec readiness. The runner now requires exit **1**, the exact
relevant `unsupported XSD construct: xs:anonymous element type` diagnostic,
and no output directory from both `service-check` and `service-generate`.
It records stage, command array, cwd, exit, expected diagnostic and actual log.

The separate backend IR test
`task093_field_local_binary_facets_fail_at_model_preflight` constructs
required and optional direct HexBinary `Data` members with local `length=4`
and requires the exact error
`unsupported Ada IR construct: field constraints on Data`. A preliminary
test-only `u32`/`u64` assignment compile error is retained in
`ir-field-facets.log`; the corrected test passes in
`ir-field-facets-corrected.log`. No production change was needed.

Both original malformed Choice schemas are retained as frontend tests.
Additional syntactically valid named Choice declarations now reach codec
readiness and specifically reject direct and named Binary alternatives.
A named Binary pattern restriction rejects at the existing frontend pattern
boundary; a separate core IR test verifies `Binary pattern constraints`.
Unknown lexical provenance and manually supplied Base64 provenance remain
independent core controls; frontend Base64 rejection is not substituted for
them. Invalid cardinality, repeated direct/named Binary and field-local
constraints remain negative controls. No frontend behavior was changed.

The synthetic runner has ten generator controls, each executed through both
CLI actions (20 negative invocations per run). Executable decoder negatives
remain inside the original 33 codec checks; model coverage remains six checks.
Qualified non-OAM names run as a separate full fixture variant.

### Compiler and real-source evidence

`complete-{14,16}{,-qualified}/` passed all Normal/Check/Ignore cells,
each executing the 33-check codec and six-check model consumers. Final
manifest-producing reruns use `certified-*` evidence directories.

`real-2.{5,6}-{14,16}/` passed all three policies with 14 checks per cell.
The real runner verifies the unchanged root SHA-256, extracts the actual
`LogMDT.Data` member, `FileMetadataMDT.SHA_2_Hash` member, and the complete
`SHA_2_256_HashType` restriction (32 octets). UCI 2.6 additionally retains
the complete `HexBinaryType` declaration. A synthetic `BinaryProbe` envelope
allows independent qualification without weakening the full Log selection.
`provenance.json` preserves exact fragments and fragment hashes; source
definitions, facet values, restriction ancestry and optionality are unchanged.
It also inventories `SubsystemStreamMDT.SubsystemStreamBinary` and
`ProductMetadataMDT.SHA_2_Hash`. Typed execution checks exact octets, high-bit
bytes, empty/present/absent data, canonical uppercase, lowercase and XML
whitespace input, malformed hex, and 31/33-octet hash rejection. Only expected
`Codec_Error` is caught. This is **extracted real-schema Binary qualification,
not full Log service qualification or real-provider acceptance**.

Fresh exact Task 091 baseline/current Log measurements:

| Selection | Selected types | Emitted declarations | Codec baseline → Task 093 | First blocker baseline → Task 093 |
|---|---:|---:|---|---|
| Log 2.5 | 42 | 41 | 39/41 → 40/41 | LogMDT.Data Binary → OwnerProducer repeated Choice |
| Log 2.6 | 44 | 43 | 41/43 → 42/43 | HexBinaryType Binary → OwnerProducer repeated Choice |

Both remain NOT READY. Exact logs are `log25-{baseline,current}.log` and
`log26-{baseline,current}.log`. Task 092 was not used to remove the blocker.

### Pins and preservation

Fresh OMS source checkout at `726272bd0390982a759c91a9cf4e13b81c2b510b`
reproduces DOCX SHA-256
`b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
Fresh W3C Second Edition HTML reproduces
`980de872aa2c50013d5202176eb85eef3f183a7df635fdcb6b4aa34c1c12eb72`.
Both UCI roots reproduce the previously recorded hashes.

`byte-preservation.json` records **44 equal outcomes: 43 successful
generations and one matching unsupported rejection**. Successful outputs
cover Rust models/APIs/codecs, C++ models/APIs, existing Ada model/API output,
Binary-free Ada codecs, direct and combined DateTime, Duration and combined
temporal codecs. Successful Binary-free Ada codecs explicitly contain no
`Encode_Hex_Binary` helper. Synthetic and extracted real Binary model/API
bytes also match with and without `--with-codec`. Final source hashes are
recorded in `final-source-hashes.json`; generated hashes and locks accompany
each compiler campaign.

### CI integrity localization

The new Deep job is appended after all inherited jobs. The inherited mutation
test originally appends a purported extra fetch at EOF. With artifact metadata
at EOF, that text is not an executable run command, so the guard correctly
does not count it. `integrity-final-mutations.log` preserves the failure.
The narrow correction duplicates the real fetch line **inside its owning
`real-uci` job**, asserting the mutation changes that block. No guard was
weakened, check count reduced, inherited job reordered or Task 092 fix copied.
This may overlap future sibling CI-test changes and requires reconciliation
if Task 092 subsequently lands. Corrected full self-test evidence is separate.

### Publication gate status

During C1, PR #92 merged as
`624364c4f8f6212edee3afc04e331c4064202659`. Fresh fetch confirmed the original
Task 091 HEAD is an ancestor and its tree is byte-identical to final main.
Task 093 integrated this history-preservingly with `git merge --ff-only
origin/main`, preserving every uncommitted change. No source or fixture bytes
changed, so the ongoing qualification remains applicable to the integrated
tree. Publication, if qualified, must target **main** rather than the former
feature parent. PR #93 was still open and has not been integrated.

Full inherited campaigns, final workspace completion, final integrity reruns,
documentation disposition and publication are still being evaluated. Until
their results are recorded, the historical `QUALIFICATION_FAILED` disposition
below is not replaced by a ready-for-review claim. No hosted CI result or PR
identity is implied by local executable success.

### C1 mandatory local completion

All **18 inherited campaigns** completed exit zero: Tasks 087A/B, 088,
089/089R, 090/090R and 091/091R under GNAT 14.2.0 and 16.1.0, each campaign
executing Normal/Check/Ignore. `inherited-results.json` records exact driver
commands/exits and each campaign retains compiler/build/consumer logs.
All eight final Task 093 campaigns in `compiler-matrix.json` passed: two
compilers times the OAM/non-OAM synthetic variants and two real UCI releases.

Rust 1.95 locked workspace all-target fmt/check/Clippy/tests all exited zero.
Workspace tests report **1,415 passed, zero failed, eight pre-existing ignored
external-environment tests**, across 162 summaries; ignored tests are not
claimed as executed qualification. The Ada backend and core readiness suites
are included. Separate CLI codec suite: 18/18; guard wrapper: 8/8.
Final warnings-denied Clippy also passed after adding all focused IR tests.
Python/JSON/XML/workflow syntax and `git diff --check` pass. CI split guard,
layout checker, and corrected mutation suite (209 checks) pass.

The production generator was rebuilt after the final test additions; its
binary remains identical to the qualified generator, SHA-256
`54cbcb6dbf177087b4a116a38b0c07fcb4f3b4eec10c7e05acb8726d2c9a7866`.
Final source-manifest SHA-256:
`3c6e36e12512f69f89457838fea5a6c1fee912d1dedc33a8f2fb0d71c6ee6b4b`.
Each compiler campaign preserves generated source hashes, exact command
arrays/exits, GNAT/GPRBUILD identities and dependency locks. Alire is 2.1.1,
GPRBUILD package 26.0.1 (executable 26.0.0), GNATCOLL/minimal 26.0.0.

Semantic review confirms only resolved HexBinary enables codecs; no model,
Rust, C++, IR or frontend behavior changed. Length construction remains inside
the existing checked constructor; only its `Constraint_Error` is translated.
The encoding bound precedes multiplication; decoding trims trailing XML
whitespace before incrementing the leading index, then uses bounded pair
offsets and byte arithmetic (0..255). The JSON string length bounds the decoded
vector capacity. No nibble array or additional emission plan is introduced.
Helper names are reserved case-insensitively. Full Log remains unsupported
because repeated Choice is outside this branch. Sleet lexical-character length
behavior is not emulated; no transport, hardware or provider acceptance is
claimed.

Mandatory local qualification is complete. Publication targets main because
PR #92 merged; exact-head hosted CI remains required. No automatic merge is
authorized. Hosted identities and HEAD parity will be recorded in the PR and
final report rather than predicted here.

## Original attempt disposition: QUALIFICATION_FAILED (historical)

The following sections preserve the original stopped attempt. C1 results and
current publication status are recorded above; historical failures are not
silently replaced by later passing evidence.

## Verified baseline and isolation

- Main: `13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`.
- Starting/dependency HEAD: `3eb92115e2466a151c2bad9b9ff87b2ab5ba2c72`.
- PR #92 remains open at that HEAD.
- PR #93 remains open at `09a1b2e630acfda7e971bbe2524f3763a54e6b50`.
- Branch: `feature/093-ada-hexbinary-codec`, created directly from PR #92.
- Worktree: `/home/zboll/git/ams-gra-codegen-oms-task093`.
- Evidence: `/home/zboll/task093-scratch`.
- No sibling branch, worktree, evidence, or PR was modified.

## Contract and implementation

The reviewed Task 052/053 research maps semantic octets to JSON strings:
empty is `""`, `00 A1 FF` is `"00A1FF"`. XML Schema Part 2 Second Edition
3.2.15.1 admits both hexadecimal cases; 3.2.15.2 canonicalizes uppercase.
Sections 4.3.1–4.3.3 count octets. Section 4.3.6 collapses XML whitespace:
surrounding XML whitespace is accepted, internal whitespace is invalid.
OMSC-SPC-013 Rev B 6.1.3/6.1.4 supplies the String representation.
Fresh normative document hash reverification remains outstanding.

The draft changes preserve the model: direct Binary uses
`Binary_Vectors.Vector` of `Interfaces.Unsigned_8`; unconstrained named
Binary uses its public record `Value` component; constrained named Binary
uses checked `Create`/`Value`. Only constructor `Constraint_Error` is mapped
to `Codec_Error`, after lexical extraction. Codec length checks are not
duplicated. The decoder trims XML whitespace and decodes pairs directly,
without an intermediate nibble array. Encoding checks the doubled String
length before allocation. New helper names are reserved in collision preflight.
Helpers are inserted only when projected operations reference Binary.

Readiness requires resolved HexBinary provenance and supported named length
facets. Unknown provenance, Base64, direct field facets, repeated Binary,
direct/named Binary Choice alternatives, unsupported aliases/lists,
abstract values and unsupported facets remain excluded. Named declarations
are checked in their own emission context. No model API changes were made.

## Executed evidence

The initial synthetic codec runner completed all six cells:

| Compiler | Normal | Check | Ignore |
|---|---|---|---|
| GNAT 14.2.0 | 33 checks pass | 33 checks pass | 33 checks pass |
| GNAT 16.1.0 | 33 checks pass | 33 checks pass | 33 checks pass |

Evidence: `gnat14/` and `gnat16-v2/`. These runs precede the added model
consumer and generator negative controls; they do **not** certify the final
runner. The later `gnat14-v3/` and `gnat16-v3/` runs completed Normal and Check
with codec count 33 and model count 6, but were interrupted during Ignore by
the command execution timeout. Do not count those as completed campaigns.

Typed tests cover vector lengths, `00`, `80`, `FF`, empty values, optional
presence/absence, unconstrained/constrained named values, restriction ancestry,
nested and inherited Records, mixed-case input and uppercase re-encoding.
Negative tests catch only `Codec_Error`, checking no successfully returned
typed payload is published on failure. Cases include invalid digits, odd
counts, internal whitespace, non-string kinds, named lengths, unknown/missing
members and failures in both earlier and later nested Records.

The final runner attempt `gnat16-v4/` failed at `field-facets`: the anonymous
field-type fixture fails atomically in the frontend with
`unsupported XSD construct: xs:anonymous element type`, but the runner
requires a diagnostic containing `Binary`. Exact command, exit 1, and actual
diagnostic are preserved in `field-facets.command.json` and `field-facets.log`.
This is a runner expectation failure, not evidence of accepting the schema.
Hand-built IR direct-field facet rejection passes the focused core test.
The final runner has not passed and must be corrected and fully rerun.

Other completed gates:

- Rust 1.95 locked workspace all-target check.
- Warnings-denied workspace all-target Clippy (before the final added tests).
- Core service codec tests and two focused Task 093 readiness tests.
- CLI service codec suite: 18 passed, including zero-OMS/atomic-output controls.
- Formatting and `git diff --check`.
- Python/JSON/XML/YAML syntax.
- CI split guard, layout check, and mutation self-test (209 checks).
- Task 087A/B on GNAT 14 and 16, Task 088 on GNAT 14.
- Task 091R on GNAT 16, all policies, 14 controls.

Workspace tests, Task 088 GNAT 16 and remaining inherited campaigns were
interrupted at the stop disposition; no full-suite pass is claimed.
`stopped-processes.json` identifies only Task 093-owned process trees stopped.

## Byte preservation

`byte-comparison.json` records 30 equal baseline/current command outcomes and
generated-file hash maps across Rust model/API/codec, C++ model/API, Ada
model/API and Binary-free Ada codecs, including Duration and combined temporal
output. Some unsupported outcomes generate no files; this is not 30 successful
code-generation selections. Full DateTime selection coverage and explicit
Binary-free helper absence controls remain outstanding. Synthetic Binary
model/API bytes match with and without `--with-codec`.

## Real UCI measurements (partial)

Both unchanged UCI roots were fetched through the existing pinned scripts:
2.5 root hash `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`,
2.6 root hash `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

| Log selection | Selected types | Codec declarations | First blocker | Status |
|---|---:|---:|---|---|
| 2.5 current | 42 | 40/41 | SecurityInformationType.OwnerProducer repeated Choice | NOT READY |
| 2.6 current | 44 | 42/43 | SecurityInformationType.OwnerProducer repeated Choice | NOT READY |

Current logs: `log25-current-v2.log`, `log26-current-v2.log`.
Exact baseline measurements were interrupted and remain outstanding. Historical
Task 091R 2.5 measurement is 39/41 with `LogMDT.Data` as blocker; it is not
substituted for a fresh measurement. Additional real Binary provenance
inspection and extracted real-schema executable qualification are incomplete.
No complete real-UCI service execution or provider acceptance is claimed.
Pinned Sleet's known lexical-character-versus-octet length defect must not
change the Ada contract. No hardware/transport interoperability was executed.

## Remaining mandatory work

Correct and rerun the mandatory runner; qualify qualified non-OAM names;
complete all core context/facet controls, isolated model-only execution,
normative pins, fresh before/after UCI measurements, extracted real Binary
execution, all inherited campaigns and workspace tests, and comprehensive
byte/helper controls. The Fast CI job is drafted but has not run hosted.
No Deep CI job was added without established real-source qualification.
No commit, push, hosted run identity, PR, final Task 093 SHA or publication
milestone is available. HEAD remains the starting SHA with local changes.