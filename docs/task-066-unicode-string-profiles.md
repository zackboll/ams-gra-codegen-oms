# Task 066 — Exact Unicode 3.1 String profiles

## Hosted corrective — optional vertical environment (current checkpoint)

The previous feature head was `921ea176d0d91e05941220f96fa86d31ea25db09`,
on immutable BEFORE/original base `52e22b67b94b431d96925c41ff730749c33b3c9e`.
Actions actually checked out synthetic merge
`83cbee6478604851f321c7ed3e1fa37efc25af24`.
Fast run **37256984008 succeeded**; Deep run **37256984003 failed**:
`msrv-real-uci` job **111596138833 succeeded**, `real-sleet` job
**111596138928 succeeded**, and `real-uci` job **111596138962 failed**.

The downloaded real-uci log confirms both exact authority inventory markers
(`UCI 2.5 TASK066 AUTHORITY INVENTORY: PASSED` and
`UCI 2.6 TASK066 AUTHORITY INVENTORY: PASSED`) and
`test task066_pinned_unicode_authority_inventory ... ok` **before** the failure.
The traceback was `KeyError: 'TMPDIR'` at
`scripts/check-task066-vertical.py` **line 11**. This was CI vertical scratch-path
environment handling; production Unicode semantics were not implicated.

The same correction removes a second latent `CARGO_TARGET_DIR` indexing defect.
Scratch-root precedence is non-empty `TMPDIR`, non-empty `RUNNER_TEMP`, then
portable `tempfile.gettempdir()`. Cargo storage uses a non-empty configured
target (relative paths resolved at the repository), otherwise repository
`target`; the nested build uses its `task066-vertical` child, never the outer
target itself. Required `AMS_GRA_UCI_2_5_ROOT` and `AMS_GRA_UCI_2_6_ROOT` remain
hard requirements. Importing the vertical now executes no service loop.

Fast's existing Task066 wrapper step runs seven wrapper adversarial cases plus
nine real-helper environment controls, including empty settings and a generated
service cwd. A direct `env -u TMPDIR -u CARGO_TARGET_DIR` import/path-selection
smoke also passes without inventory, service generation or compilers.
Historical BEFORE, isolated AFTER, readiness and 1,134-cell output evidence below
remain immutable-parent evidence, not current-main measurements.

### Current-main reconciliation (Tasks063/064/066)

Normal corrective commit `6e30a43d6cf66d3fdda8b324879e6ad037b18412` was followed
by normal merge `ff0846c9860200912e580b1f7042049c29f32041`, whose second parent
is current integration base `5d284a346433d461f1520e599f969e82e00f3e82`.
Task065 remains independent and was not incorporated. No rebase, amend,
cherry-pick or force-push was used.

The merged tree retains Task063's named patterned-integral exports/classifier,
Task064's `GeneratedSupportChange` and plan binding, and Task066's Unicode
exports/classifier. Unicode production implementation and authority tables are
unchanged. All merged Fast and Deep gates remain, including Task064's filtered
Task056 invocation, its separate binding test, and both release markers.
Current-main's two-support-step Task060 wrapper and split adversarial controls
remain intact. Task066's wrapper/environment step is now independently protected
against removal; split adversarial tests pass **143 checks**, Task060 **57**,
and Task063 **5 wrapper + 3 temp-root** checks.

Task064 correctly rejects a stale plan after the synthetic Unicode support
declaration is mutated. The Task066 neighbor test now checks that typed binding
failure first, then resolves a fresh plan and retains the unsupported-profile
readiness/codec assertions. This composes binding and classification rather than
weakening either. Task063 Fast passes; Task064 compact Fast prints all four
required markers. Task066 Fast passes including corpus/compiler/codec/mock-OWP.

Live production CLI coverage (not inferred assertions) was measured first and
recorded separately in `tests/fixtures/string/task066-integrated-current.tsv`:

|Release|World|Backend|Kinds/total|Full declarations|Field refs/occurrences|Closures/messages|
|---|---|---|---|---:|---|---|
|2.5|closed|Ada|5557/5557|5536|13160/13160|643/722|
|2.5|closed|Rust|5557/5557|5537|13160/13160|676/722|
|2.5|closed|C++|5557/5557|5537|13160/13160|676/722|
|2.5|open|Ada|5557/5557|5449|13160/13160|571/722|
|2.5|open|Rust|5557/5557|5449|13160/13160|571/722|
|2.5|open|C++|5557/5557|5449|13160/13160|571/722|
|2.6|closed|Ada|5570/5570|5549|13198/13198|646/725|
|2.6|closed|Rust|5570/5570|5550|13198/13198|679/725|
|2.6|closed|C++|5570/5570|5550|13198/13198|679/725|
|2.6|open|Ada|5570/5570|5462|13198/13198|574/725|
|2.6|open|Rust|5570/5570|5462|13198/13198|574/725|
|2.6|open|C++|5570/5570|5462|13198/13198|574/725|

Fields total and both renderability metrics are 13160 (2.5) / 13198 (2.6).
The current oracle composes the frozen Task062 + Task063 campaigns with only
Task066's +3 kinds/full-declaration delta and verifies it against this new ledger.
It does not rewrite any isolated historical fixture.

The six integrated AO_CapabilityStatus service-check/generate/compiler verticals
pass, still **64 selected + 68 support = 132** per release. Strict GNAT and C++17
pass, and both actual generated Rust message codecs print
`TASK066 REAL UNICODE JSON ROUNDTRIP: PASSED` with invalid lexical rejection.
The direct full vertical also passed with TMPDIR absent (portable system temp).

A NEW integrated-current output comparison uses a binary built from an archive
of `5d284a34...` in Task066-only source/target storage, not a sibling's target.
Across **1,146 cells** (191 fixtures × three backends × two worlds):
Ada 191 identical successes / 2 new / 189 shared failures;
Rust 199 / 2 / 181; C++ 200 / 2 / 180.
There are **zero changed successful outputs and zero lost successful outputs**;
new successes are only the Unicode synthetic fixture. Task063 outputs survive,
and Task064's own zero-capability/source/codec-delta control passes.
The original 1,134-cell result below remains historical isolated evidence.

Rust 1.95 and stable fmt/check/all-target Clippy pass, as do isolated runtime
MSRV checks and both GNAT-required full workspace runs. The final integrated pinned wrapper
passes: both inventories, all six compiler verticals, both Unicode JSON markers
and `TASK066 PINNED GATES: PASSED`. It recomputes the exact same eleven services,
33 closed world tuples/release, **66 total world tuples and 66 unique
release/backend/message tuples**, with AO_CapabilityStatus still smallest.
An independently built current-main binary confirms all 66 tuples remain
Unicode-blocked before Task066. Task063's exact live inventory/impact test also
passes on both releases with its current UCI2.5 integral gain retained.
Task060's live projected impact control passes with 97 READY / six topology /
zero Unicode or integral capability blockers in each release. Fresh hosted
certification is mandatory and no hosted success is implied at this local checkpoint.
Logs/manifests are retained under `/tmp/task066-corrective`.

## Authorized resume — CURRENT, supersedes the historical STOP below

The user explicitly authorized **Unicode Character Database 3.1.0,
General_Category=Nd** as the repository's deterministic XML Schema 1.0 category
compatibility baseline. The original Recommendation (2 May 2001), Appendix F
and normative Unicode Database reference point to Unicode 3.1.0. Later
databases may change/extend category membership; this implementation deliberately
does not inherit the host runtime's current Unicode semantics. No material
contradiction with the original Recommendation was found. The historical stop
and comparison below remain preserved as provenance, not current delivery status.

The existing worktree and all five pre-existing Task066 source/evidence files
were inventoried before edits, with no active Task066 inventory process.
The completed 233.69-second, exit-0 BEFORE inventory was reused, not relaunched.
Its historical fixture SHA256 is
`8f7a3613e5b34f566af9285a651ed2107c6063c41ec793d6aa64b591a277f09a`.
Additional raw inventory/witness exploration from scratch was also retained.

### Frozen authority and exact model

The retained `UnicodeData-3.1.0.txt` was independently rehashed and re-derived:
SHA256 `8e57884da0da3a66782b8a6332a601fccbcae9ed0060e12501aca02fa56ffecd`,
248 Nd points, 21 contiguous ranges. The compact production ranges, independent
range fixture, provenance JSON and offline derivation checker are committed;
the full source stays outside the repository. Normal builds/tests need no
Unicode network access. Unicode 3.1 assignments, including U+1369..U+1371,
are retained exactly, not reclassified through modern libraries.

One shared name-free classifier compares the entire effective ConstraintSet
against exactly three rows. Semantic tokens are only authored ASCII sets,
Unicode31DecimalDigit, fixed sequences and finite alternatives. No regex parser,
date/calendar/coordinate interpretation or normalization exists. Explicit
whiteSpace, changed lengths, extra facets/groups/alternatives and near-matches
fail closed. Renamed exact declarations classify identically.

### UTF-8 contract and generated size

Rust String/&str is valid UTF-8; generated code iterates scalars. Ada
String/Unbounded_String and C++ string/string_view are UTF-8 bytes for these
carriers. Acceptance preserves every byte. XML length is scalar count, not byte
count: `2١000101` is 8 scalars / 9 bytes. The Nd positions do not normalize digits.

Ada/C++ decode ASCII and valid 2/3/4-byte sequences; reject lone continuation,
truncation, overlong spelling, bad continuation, surrogate and >U+10FFFF.
Bounded storage holds at most 21 scalars. Ada forms absolute indices only for
proven valid zero-based offsets; no one-past-Positive'Last index is formed.
C++ casts bytes to unsigned char before arithmetic. No locale or host Unicode
predicate is used. One shared helper/table per generated unit is emitted;
private/nested APIs do not introduce a public runtime interface. Helper names
are registered through normal preflight.

Synthetic all-three-profile model measured: Rust 11,705 bytes / 110 lines;
C++ 18,464 / 161;
Ada spec 7,359 / 170 and body 14,045 / 115. Final source-size results are printed
by the exact compiler test; no per-declaration decoder/table duplication.

### Independent conformance and lifecycle

Frozen corpus: **560 cases**, independent expected results derived from raw
authored positions and retained Unicode source, never production matching.
Every Nd range has first/interior/last and adjacent-point witnesses, including
supplementary mathematical digits. Numeric-but-not-Nd, newer Unicode digits,
ASCII-class substitutions, branches/spaces, length, punctuation/control/NUL and
malformed UTF-8 are covered. A mistaken draft expectation at a genuine
coordinate Nd position was corrected by raw-pattern positional review before
freeze; logs retain the failed draft run.

Positive witnesses: `2١000101`, `2١0001010000`, `+1١.123456+012.123456`.
U+0661 occupies the second date character / third coordinate character, each
authored `\d`; all other positions remain authored ASCII. Fixed independent
lengths are 8/12/21 scalars respectively. ASCII year `[12]` and coordinate
leading `[0-8]` do not accept that substitution.

All three production-rendered carriers pass the same corpus with strict Rust
edition2024/-Dwarnings, C++17/-Wall/-Wextra/-Werror/-pedantic-errors and Ada
-gnat2022/-gnatwe/-gnato. Each backend's deliberately wrong expectation fails
execution, then restored expectations pass. Ada Check/Ignore policies both
pass, including shifted strings, null, truncated UTF-8 and a valid final byte
at Positive'Last. Carriers retain checked construction/private storage; no
invalid Default or aggregate bypass. Copy/accessor preserves validated text.

Generated Rust JSON uses existing checked construction, no second validator.
Synthetic required/optional/repeated/Choice JSON and mock OWP prove non-ASCII
text reaches typed handlers unchanged, while lexical/type errors emit codec
events and never reach handlers. Optional JSON null remains ordinary absence,
not an unchecked invalid carrier.

### Measured impact (independent sibling base, without Task065 naming repair)

All twelve AFTER cells are frozen separately in `task066-after.tsv`. Each cell
gains three renderable kinds and three fully renderable declarations; field
metrics and full-schema message closures remain unchanged:

|Release|World|Ada messages|Rust/C++ messages|Ada full decls BEFORE -> AFTER|Peer full decls BEFORE -> AFTER|
|---|---|---:|---:|---|---|
|2.5|closed|641|674|5532 -> 5535|5533 -> 5536|
|2.5|open|570|570|5445 -> 5448|5445 -> 5448|
|2.6|closed|646|679|5546 -> 5549|5547 -> 5550|
|2.6|open|574|574|5459 -> 5462|5459 -> 5462|

Selected reach is zero for all three profiles; closed support reach is the same
eleven messages listed in the historical inventory. All eleven become READY
for all three backends in each release: **66 world tuples**, separately **66
unique release/backend/message tuples**, all closed-world. BEFORE first blocker
is NITF_DateAndTimeType, with all three profile declarations unsupported; AFTER
selected/support blockers are empty. Open projections that require unknown
abstract descendants remain topology/extension blockers, not capability gains.

The smallest newly READY service by selected+support then lexical name is
**AO_CapabilityStatus**, 64 selected + 68 support = 132 in both releases.
Both releases pass production service-check and service-generate for all
backends, strict Ada model/body/API and C++ model/API compilation, and Rust
compiled actual-message JSON round trips containing non-ASCII Date and
DateAndTime in the NITF packing-plan alternative, with invalid lexical rejection.
No standalone support-type codec is mistaken for the actual message codec.

Real generated model/API/body/codec source totals (bytes / lines):

|Release|Ada|Rust|C++|
|---|---|---|---|
|2.5|408652 / 10731|544089 / 12361|209412 / 6415|
|2.6|405772 / 10673|549915 / 12405|205866 / 6358|

Repeated vertical runs exclude the locally authored `probe.rs` from generated
source totals. The final audit corrected that evidence-only counting defect;
compiler and JSON results were unaffected. Fresh hosted head-keyed CI is required.

Remaining full-schema Ada Query companion collision is Task065 scope and is
unchanged on this independent branch. Required zero-descendant abstract values,
open-world extension points and unrelated topology/cycle/selected blockers are
not widened. Full message-closure gains are **zero**, not the 66 readiness gains.

### Output comparison and historical discipline

189 repository XSD paths x 3 backends x 2 worlds = 1134 cells. Ada: 188 identical
success / 2 new / 188 shared failure; Rust: 196 / 2 / 180; C++: 197 / 2 / 179.
Zero changed or lost successful outputs. The new successes are precisely the
Task066 synthetic profile fixture in both worlds. The immutable parent binary
was built in Task066-only storage, not borrowed from a sibling target.
Original Task060 and Task062 tables remain untouched; inherited current tests
retain their owned ASCII/naming invariants and explicitly supersede only the
now-supported Unicode exclusions. Task066 owns new integrated totals.

### Gates and publication status

Focused Fast wrapper, Unicode authority check, compiler/corpus, codec/mock OWP,
12-cell AFTER and real vertical pass locally. The final exact pinned wrapper
completed exit 0, both release markers, all six real compiler vertical markers
and the overall completion marker. Its one inventory test took 271.55 seconds.
Inherited Task060 admission and projected impact tests pass on both roots;
Task060 retains its historical 15-ASCII-member/three-Unicode-exclusion boundary.
fmt/check/all-target Clippy pass.
Workspace and MSRV initial attempts ran out of tmpfs space; failure logs are
retained. Only Task066 build directories were moved to Task066 disk storage,
with logical isolated `/tmp/task066/target` and `/tmp/task066/msrv-target` paths
preserved via symlinks. Rust 1.95 final retry and the GNAT-required full workspace
retry both passed, exit 0. The first workspace semantic failure was a stale
historical Task060 non-admission assertion; its owned ASCII membership and
historical tables were preserved, and the current Unicode capability assertion
was explicitly superseded. Final fmt/check/all-target Clippy/Fast, CI split
(129 adversarial checks), seven Task066 wrapper mutations, diff check and the
1134-cell output comparison all passed. Final integration base remains
`52e22b67b94b431d96925c41ff730749c33b3c9e`; no sibling has merged.

Publication uses a normal commit/push and exactly one open non-draft PR to main;
no rebase/force-push/merge. Hosted final-head/base certification is pending and
must be collected before claiming delivery certified. Source manifest including
untracked files is retained under `/tmp/task066/manifests`.

## Historical authority STOP (preserved from the previous session)

## Provenance and scope

Immutable BEFORE and selected integration base:
`52e22b67b94b431d96925c41ff730749c33b3c9e`.
Branch `feature/066-unicode-string-profiles`, isolated worktree
`/home/zboll/git/ams-gra-codegen-oms-task066`; target and temporary state are
`/tmp/task066/target` and `/tmp/task066/tmp`. Task063/064 remain untouched;
Task065 is an independent sibling. No production change, Unicode runtime,
membership-policy selection, push, PR, or merge is claimed.

**STOP at the requested mandatory Unicode authority gate.** XML Schema 1.0
does not bind these documents to one immutable Unicode category database.
Choosing a membership version requires an explicit project policy decision.
The existing three unsupported profiles remain fail-closed. Task060's
historical `DEFERRED_UNICODE=3` evidence is preserved without alteration.

## Standards evidence

XML Schema Part 2, Second Edition (28 October 2004), Appendix F, category
escapes, states:

> [Unicode Database] is subject to future revision. For example, the mapping
> from code points to character properties might be updated. All minimally
> conforming processors must support the character properties defined in the
> version of [Unicode Database] that is current at the time this specification
> became a W3C Recommendation. However, implementors are encouraged to support
> the character properties defined in any future version.

The 2 May 2001 edition contains the same provision. Both references point to
the Unicode 3.1 database. The multi-character escape table defines `\d` as
`\p{Nd}`; the category table distinguishes `Nd` decimal digit from `Nl` letter
and `No` other numeric characters. Neither Rust `is_numeric`, locale `isdigit`,
nor ASCII `[0-9]` is an equivalent category oracle.

Appendix F defines a regular expression as a set of strings `L(R)` and says
only strings in that set are valid literals. This is full-string matching,
not substring search. Section 4.3.1 specifies String length in characters,
not UTF-8 bytes. These facts do not resolve the membership-version ambiguity.

Authorities:

- <https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/#regexs>
- <https://www.w3.org/TR/2001/REC-xmlschema-2-20010502/#regexs>
- <https://www.unicode.org/Public/3.1-Update/UnicodeData-3.1.0.txt>
- <https://www.unicode.org/Public/16.0.0/ucd/UnicodeData.txt>

Independent downloaded category comparison (evidence only, **not** an adopted
production membership model): Unicode 3.1 has 248 Nd code points in 21 ranges;
Unicode 16.0 has 760 in 71 ranges. U+1E950 ADLAM DIGIT ZERO is absent from the
former and present in the latter. U+1D7CE mathematical bold digit zero is in
both. Even existing assignments differ: the 3.1 category table labels
U+1369..U+1371 Nd, while later tables do not. Thus the difference is not merely
adding previously unassigned scalars.

Downloaded UnicodeData SHA-256:

- 3.1: `8e57884da0da3a66782b8a6332a601fccbcae9ed0060e12501aca02fa56ffecd`
- 16.0: `ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f`

Neither pinned root declares a Unicode membership version. Its schema
`version` attribute identifies the UCI release, not a Unicode database.
An exact cross-language table is mechanically feasible **after** an authorized
version policy, but the pinned bytes alone do not uniquely select that table.

## Pinned inventory

Source: `https://gitlab.com/open-arsenal/uci/standard.git`.

| Release | Pinned commit | Root SHA-256 |
|---|---|---|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

All three QNames use namespace `https://www.vdl.afrl.af.mil/programs/oam`.
They are `Primitive(String)`, directly restricted from `xs:string`, depth 1,
with no numeric facets, minLength, maxLength, or explicit whiteSpace.
Effective whiteSpace is Preserve. Each has one XML-Schema pattern group with
two alternatives in authored order and no direct named dependencies.

| Declaration | 2.5 line | 2.6 line | length |
|---|---:|---:|---:|
| NITF_DateAndTimeType | 131253 | 131742 | 12 |
| NITF_DateType | 131263 | 131752 | 8 |
| NITF_MSTGTA_TargetLocationType | 131363 | 131852 | 21 |

Exact normalized expressions, identical across releases:

```text
DateAndTime:
(([12]\d\d\d)((0[1-9])|(1[012]))(0[1-9]|[12][0-9]|3[01])([01][0-9]|[2][0-3])([0-5][0-9]))
 {12}
Date:
([12]\d\d\d((0[1-9])|(1[012]))(0[1-9]|[12][0-9]|3[01]))
 {8}
TargetLocation:
([0-8]\d([0-5]\d){2}\.\d{2}(N|S)(0\d{2}|1[0-7]\d)([0-5]\d){2}\.\d{2}(E|W))
([\+\-]{1}[0-8]\d\.\d{6}[\+\-]{1}(0\d{2}|1[0-7]\d)\.\d{6})
```

The raw XSD uses `&#x20;` for space; the frontend decodes it without changing
alternative order. The evidence test compares normalized expressions against
the historical pinned facet rows and prints the actual raw declarations.

All three have zero selected-message reach. In each release, closed-schema
generated-support reach is the same eleven messages:
`AO_CapabilityStatus`, `AO_SettingsCommand`, `PO_Activity`, `PO_Capability`,
`PO_CapabilityStatus`, `PO_Command`, `PO_SettingsCommand`,
`SAR_CapabilityStatus`, `SAR_Command`, `SAR_SettingsCommand`, `Task`.
Open-world generated-support reach is zero; projection errors are retained
separately, not interpreted as successful unsupported projections.
Authored and effective inherited references are printed in the frozen evidence.

## Measured BEFORE (no AFTER implementation)

All 12 cells are frozen in `tests/fixtures/string/task066-before-authority.tsv`.

| Release | World | Ada closed/open messages | Rust | C++ |
|---|---|---:|---:|---:|
| 2.5 | ClosedSchemaSet | 641/722 | 674/722 | 674/722 |
| 2.5 | OpenExtensions | 570/722 | 570/722 | 570/722 |
| 2.6 | ClosedSchemaSet | 646/725 | 679/725 | 679/725 |
| 2.6 | OpenExtensions | 574/725 | 574/725 | 574/725 |

No newly READY tuples, compiler conformance, UTF-8 decoder, codec/mock-OWP,
source-size, AFTER coverage, or hosted CI results are claimed. Those stages
must not run as implementation evidence before the mandatory authority gate.

## Validation and retained evidence

`unicode_string_authority::task066_pinned_unicode_authority_inventory` was
listed as exactly one registered test, then executed with `--exact` and both
verified pinned roots. Exit 0, exactly one test passed, both release completion
markers, 12 measured coverage cells. Focused warnings-denied Clippy and fmt
passed. Raw logs, downloaded authorities, comparison-only tables and normalized
evidence remain under `/tmp/task066/evidence` and `/tmp/task066/logs`.

This is an evidence-gate stop, not a completed feature or publication request.