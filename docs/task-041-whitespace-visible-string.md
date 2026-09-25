# Task 041 — Validated whitespace-visible String profiles

Adds generated, validated carriers for the UCI whitespace-visible bounded-String
family in Ada, Rust, and C++, for **both** pinned releases. This supersedes the
"still open" whitespace-visible entry that Task 039 and Task 040 recorded; those
documents' historical results stand unchanged.

## 1. Authoritative evidence

Obtained through the established workflow and verified at the exact pinned
revisions. Neither tree was modified; `git status` was clean in both.

| Release | Revision | Schema source |
| --- | --- | --- |
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` | `03_OAC-STD-002_RevE_UCI_Schema_v2_5/` |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `03_UCI-STD-002_Rev6_UCI_Schema_v2_6-CDRL.zip` |

Both target declarations live in the **security-markings** document, not the
message-definitions root — they are reached through the root's `xs:include`.

### 1.1 The effective profiles

Every declaration is a **direct** restriction of `xs:string`: immediate base and
ultimate primitive are both `xs:string`, and the restriction chain depth is 1, so
the effective constraint set is the single restriction step itself. Each carries
exactly one `<xs:pattern>` facet holding exactly one XML-Schema-dialect
alternative, and no other facets beyond those listed.

| Release | Declaration | File:line | `whiteSpace` | `minLength` | `maxLength` | Pattern |
| --- | --- | --- | --- | ---: | ---: | --- |
| 2.5 | `WhitespaceVisibleString1024Type` | `UCI_SecurityMarkings_v2_5_0.xsd:8565` | `collapse` | 0 | 1024 | `[ -~\n\r]{0,1024}` |
| 2.5 | `WhitespaceVisibleString4096Type` | `UCI_SecurityMarkings_v2_5_0.xsd:8576` | `collapse` | 0 | 4096 | `[ -~\n\r]{0,4096}` |
| 2.5 | `QueryString4096Type` | `UCI_MessageDefinitions_v2_5_0.xsd:138224` | *absent* | 0 | 4096 | `[ -~\n\r]{0,4096}` |
| 2.6 | `WhitespaceVisibleString1024Type` | `UCI_SecurityMarkings_v2_6_0.xsd:8578` | *absent* | 1 | 1024 | `[ -~\n\r]{1,1024}` |
| 2.6 | `WhitespaceVisibleString4096Type` | `UCI_SecurityMarkings_v2_6_0.xsd:8588` | *absent* | 1 | 4096 | `[ -~\n\r]{1,4096}` |
| 2.6 | `QueryString4096Type` | `UCI_MessageDefinitions_v2_6_0.xsd:138589` | *absent* | 1 | 4096 | `[ -~\n\r]{1,4096}` |

There is no `length` facet, no numeric facet, no second pattern group, and no
second alternative anywhere in the family. No declaration in either release
*derives* from one of these six; the family has no named-restriction members.

The prior repository evidence is **reconfirmed**:

- UCI 2.5: direct `xs:string` restrictions, `whiteSpace = collapse`,
  `minLength = 0`, `maxLength` 1024 or 4096, one expression shaped
  `[ -~\n\r]{0,N}` — all confirmed for the whitespace-visible pair;
- UCI 2.6: no explicit `whiteSpace` facet, pattern lower bounds of one —
  confirmed.

The `minLength` facet and the pattern quantifier's lower bound were read
**independently**. They agree in both releases, and that agreement is recorded as
an observation, never inferred from the quantifier. The classifier checks both,
and a declaration whose two minima disagree fails closed.

### 1.2 `QueryString4096Type` is a genuine semantic neighbour

Inspected without pre-deciding from its name, and it belongs to the family — but
not identically in the two releases:

- in **UCI 2.6** it is byte-for-byte the same effective profile as
  `WhitespaceVisibleString4096Type`. It is an exact **semantic alias**;
- in **UCI 2.5** it is a *different* shape from both its own 2.6 self and the 2.5
  whitespace-visible pair: it carries **no** `whiteSpace` facet where they carry
  `collapse`.

So the two releases are emphatically **not** equivalent, and one release's value
space cannot be inherited by the other on the strength of a matching name.
Task 039 had listed 2.5's `QueryString4096Type` shape among the shapes that must
fail closed; that expectation was correct then and is superseded here. §5.2
records how those tests were redirected rather than weakened.

### 1.3 Three distinct layers of escaping

The pinned facet is written
`<xs:pattern value="[&#x20;-&#x7E;\n\r]{0,1024}"/>`, and exactly one of the three
layers involved is the XML parser's:

1. **XML character references.** `&#x20;` and `&#x7E;` are expanded *by the XML
   parser*. The normalized IR expression therefore contains a literal SPACE and a
   literal TILDE, confirmed by running the frontend over both pinned roots.
2. **XSD regex backslash escapes.** `\n` and `\r` are **not** XML escapes. The
   parser passes them through as two characters each; they are XML Schema regular
   expression `SingleCharEsc` sequences denoting LF and CR. The IR expression
   holds them **literally**, as a backslash followed by `n` or `r`.
3. **Decoded constructor input.** Generated `Create` / `new` / `create` receive
   already-decoded text: an actual U+000A, never a backslash and an `n`. The
   validators compare code points and never inspect escape spellings.

A test pins all three: the expression contains `\n` as two characters and no
actual LF, and a declaration whose class is spelled with real control characters
is a different shape that fails closed.

### 1.4 The exact supported tuples

Six declarations collapse onto **five** distinct `(policy, min, max)` triples,
because UCI 2.6's `QueryString4096Type` and `WhitespaceVisibleString4096Type`
share a shape:

| Policy | `minLength` | `maxLength` | Declarations |
| --- | ---: | ---: | --- |
| collapse | 0 | 1024 | 2.5 `WhitespaceVisibleString1024Type` |
| collapse | 0 | 4096 | 2.5 `WhitespaceVisibleString4096Type` |
| preserve | 0 | 4096 | 2.5 `QueryString4096Type` |
| preserve | 1 | 1024 | 2.6 `WhitespaceVisibleString1024Type` |
| preserve | 1 | 4096 | 2.6 `WhitespaceVisibleString4096Type`, 2.6 `QueryString4096Type` |

Those are **whole observed triples**, not three independent axes. Treating the
axes separately — policies `{collapse, preserve}` × minima `{0, 1}` × maxima
`{1024, 4096}` — would admit eight profiles, and **three** appear in no pinned
release. A test enumerates all eight and asserts precisely those three fail
closed, so no Cartesian product of separately observed components is accepted.

## 2. Shared classification

`crates/codegen-core/src/string_profile.rs` gains one variant,
`StringProfile::WhitespaceVisible { white_space, min_length, max_length }`, plus a
`WhitespaceVisiblePolicy` enum with `Collapse` and `Preserve`.

### 2.1 Why a separate variant, and why a policy field

A **separate variant** rather than a flag on `VisibleAscii`, because the class is
strictly larger (`[ -~]` plus LF and CR) and, for the collapse half, the
constructor *changes the stored value*. Folding a policy flag into `VisibleAscii`
would have made every existing visible-ASCII match site responsible for a
normalization decision that cannot arise there, and would have let a `collapse`
facet reach the visible-ASCII validators, which reject it today and must keep
doing so.

The **policy is part of the profile**, not a rendering detail, because it decides
which of two different values is stored for the same input. It is therefore
carried in the classification and consumed identically by all four readers.

### 2.2 Admitting only evidenced tuples

Membership is decided in this order, and nothing reads a declaration name, file
name, schema version, or restriction depth:

1. reject a `length` facet;
2. read both bound facets; a half-bounded restriction is not a member;
3. map the `whiteSpace` facet to a policy: *absent* → `Preserve`, explicit
   `collapse` → `Collapse`. An explicit `preserve` and a `replace` are
   **unobserved** shapes and are rejected, so "absent" stays distinguishable
   from "explicitly restated";
4. require the single expression to equal `whitespace_visible_pattern(min, max)`
   — derived *from the facets*, which makes facet/quantifier disagreement
   unrepresentable rather than merely unchecked;
5. only then require the whole triple to be in `UCI_WHITESPACE_VISIBLE_PROFILES`.

Step 5 is what keeps a parameterized profile from degenerating. Shape agreement is
necessary but not sufficient: `[ -~\n\r]{1,18446744073709551615}` with perfectly
agreeing facets fails closed, as do ordinary unobserved pairs such as `0..512` and
`1..2048`.

### 2.3 The old `has_only_pattern()` guard is not relaxed

`has_only_pattern()` rejected any explicit `whiteSpace` facet. It was **not**
removed. It now delegates to `has_only_pattern_under_white_space(..., Preserve)`,
which is exactly the original behaviour, and only the new family passes a
different policy — the one whose generated code it has been proven to implement.

A dedicated test asserts that all three older profiles still reject **every**
explicit `whiteSpace` value (`collapse`, `preserve`, `replace`), so the
schema-version, UUID, and visible-ASCII accepted/rejected sets are unchanged.

### 2.4 Consumers

The shared classifier is read, unchanged in shape, by all three backend
validation gates and renderers, `CoverageAnalysis`, readiness, the Ada
carrier/body predicates, and generated callable/name analysis. No backend infers
a profile for itself, and no backend parses raw XSD.

## 3. Generated value semantics

### 3.1 Collapse

Implements XML Schema Part 2 §4.3.6 `collapse`, applied to the decoded
constructor input **before** the pattern and length facets:

- only XML's four whitespace characters participate: SPACE, TAB, LF, CR;
- runs of them collapse to a single SPACE; leading and trailing runs are removed;
- the **normalized** result is validated and stored, never the original input;
- consequently a raw input longer than `maxLength` is **accepted** when its
  normalized form fits. Tested directly with a 4002-character input that
  normalizes to `"a b"` and is accepted by the 0..1024 carrier;
- U+00A0 and every other Unicode space, plus U+000B, U+000C, and NUL, are **not**
  normalized. They pass through untouched and are then *rejected* by the class
  test. Nothing invalid is silently deleted.

`char::is_whitespace`, `std::isspace`, `Ada.Characters.Handling`, and
`split_ascii_whitespace` are all deliberately avoided: each would either treat
non-XML characters as whitespace or introduce locale dependence. Generated-text
tests forbid them as calls.

### 3.2 Preserve

Derived from the **actual** evidence for the preserve-side declarations, not
inherited from 2.5 because the names match:

- no trimming and no collapsing; the caller's decoded text is stored unchanged;
- LF and CR are ordinary class members and stay significant;
- **TAB is rejected.** It is admitted by neither the printable interval nor the
  two escapes, and no normalization runs to convert it. This is the sharpest
  observable difference between the halves: `"a\tb"` is *invalid* under preserve
  and stores as `"a b"` under collapse.

### 3.3 Why a direct deterministic validator suffices

The authoritative expression is `[ -~\n\r]{min,max}`: **one** character class
under **one** bounded quantifier, with no alternation, grouping, backreference, or
unbounded repetition. Membership is therefore exactly a length test plus an
independent per-character set test, decided in a single linear pass with no
backtracking. XML Schema patterns are implicitly anchored, which testing *every*
character enforces directly. A regex dependency would add a second dialect to
reason about — `GNAT.Regpat` is Perl-derived, `<regex>` is ECMAScript/POSIX,
neither is XML Schema — and would buy nothing. No new dependency was added.

### 3.4 Facets, length units, and redundancy

The declared length facets **and** the pattern restriction are both enforced, even
though the quantifier repeats the facets, so no facet is silently lost. Their
minima are not assumed to agree; both are read and their agreement asserted.

Length units are explicit: every accepted character is at most U+007E, hence
single-byte in UTF-8, so the byte count **equals** the XSD character count for
this alphabet. Any multi-byte character contains bytes outside the class and fails
the class test, so no such value reaches a length comparison as an accepted one.
Ada's `String` is an array of `Character`, so `'Length` is already a character
count. C++ casts to `unsigned char` before every comparison, so a high-bit byte
such as `0xC2` is classified correctly where plain `char` is signed.

### 3.5 Linearity and the Ada buffer

Validation is linear in input size in all three languages. For the Ada collapse
family the normalization buffer is `String (1 .. Max_Length)` — sized by the
declaration's own `maxLength`, at most 4096 — and deliberately **not**
`String (1 .. Value'Length)`, which would make `Create`'s stack demand a function
of untrusted client input. Because a normalized value longer than `maxLength` is
invalid anyway, the buffer never needs to hold one: an `Overflowed` flag trips the
moment the output would exceed the bound, and the value is rejected on the
`maxLength` facet it actually violates. There is therefore **no undocumented
lexical input-length cap** — an over-long argument is rejected by the schema's own
rule — and normalization still visits every input character in one pass. A test
asserts `String (1 .. Value'Length)` does not appear in generated bodies.

### 3.6 Scope, API conventions, and equality

Constructors operate on **decoded** strings. No XML entity parsing, no document
line-ending normalization, and no serialization behaviour is introduced here.

Existing public carrier conventions are preserved: Rust checked `new` returning
`Option<Self>` with read-only `as_str`; C++ checked static `create` returning
`std::optional` with read-only `value()`; Ada `Create`/`Value` with
`Constraint_Error` on invalid input.

Generated documentation distinguishes the halves. Collapse carriers say the stored
text is "the collapse-normalized form of the input, not the input itself" (Ada:
"which is the collapse-normalized form of Create's argument"); preserve carriers
keep "exactly as supplied" / "preserved unchanged". The phrase "exactly as
supplied" is never reused for a collapse carrier, and a test asserts both
spellings are present and the old over-broad wording is gone.

Equality is defined on the **stored** value. For `xs:string` the value space is
the set of lexical forms, so stored-text equality is genuine XML Schema value
equality. Consequently normalized spellings that become identical **compare
equal** — `"a b"` and `"  a   b  "` are the same collapse value — while preserved
whitespace remains significant. Rust derives `PartialEq`/`Eq`; Ada's predefined
`"="` compares the stored representation. No ordering is claimed (XML Schema
defines none on `string`), and no new C++ comparison API is added.

## 4. Task 040 lifecycle and storage guarantees

No merged Task 040 correction is regressed. See
`docs/task-040-validated-carrier-lifecycle.md` for that task's own reasoning; only
the Task 041-specific consequences are recorded here.

### 4.1 Ada — the policy/validity distinction

The explicit-initialization policy is kept: every new carrier's private record
carries the failing component default, so a default-initialized object cannot
exist, enforced by the language's initialization semantics rather than by `-gnata`.

Task 041 forced one **documentation correction**. The Task 040 comment said the
default was needed because default initialization "would otherwise produce an
empty string, which this profile's own validator rejects". That is now false for
the collapse profiles, whose `minLength` is 0 and which **accept** the empty
string. The generated comment now states the distinction explicitly:

- **API policy:** a validated carrier is constructed by `Create` or not at all;
  default construction stays prohibited for every profile.
- **Value fact:** `Create ("")` *succeeds* for a profile whose schema permits it.

Both hold simultaneously. The tests exercise them **separately**: a positive test
asserts `Create ("")` returns a carrier whose `Value` is `""` for the 0..1024
collapse profile and the 0..4096 query shape, and the prohibition on default
construction is asserted independently. No claim is made that empty text is
invalid for a profile whose schema permits it.

Absent versus present optional semantics are unchanged, the existing opaque
bounded and unbounded sequence machinery is reused with no new container code, and
`Clear` remains **absent** for the positive-minimum bounded member — asserted
directly against the generated spec.

### 4.2 C++

The explicit copy-special-member policy is carried by all five new carriers: the
copy constructor and copy assignment are declared `= default`, which suppresses
the implicit move operations, so an rvalue selects copy. A runtime probe asserts
that after `auto dest = source`, `auto moved = std::move(source)`, and
`dest = std::move(moved)`, **both** source and destination still hold `"keep me"`.
No destructive implicit string move is reintroduced, and a test asserts no move
operation is declared.

### 4.3 Rust

Storage stays private and construction stays checked. No `Default` implementation
is added — asserted explicitly — and privacy bypasses (struct literal, field read)
are proven not to compile.

### 4.4 Containers

The new profiles are exercised through required fields, optional fields, bounded
sequences (including a positive-minimum one), and unbounded sequences, in both the
backend fixture and the contract-selected fixture. There is **no**
profile-specific container path: composition is entirely the existing
Task 023/034/040 machinery.

## 5. Corpus and test changes

### 5.1 The shared loader's empty-field encoding

`load_corpus()` calls `line.trim_end()` before splitting a `VALID` case on
`" => "`. That hygiene is kept, but it made an empty field on either side
unwritable: `VALID  => ` loses its trailing space and the delimiter becomes
incomplete, and padding with trailing spaces is exactly what text hygiene removes.

Task 041 needs empty fields to be load-bearing rather than hypothetical, because
`minLength = 0` accepts the empty string and whitespace-only input *normalizes to*
it. The fix is an explicit whole-field token, `<empty>`:

- it is **additive**: no existing corpus line contains it, so every existing case
  parses to exactly what it parsed to before;
- it is recognized **only as an entire field**, so `<empty>` inside longer text is
  ordinary text;
- it is deliberately not `""`, which is itself a valid two-character input here;
- the payload-free `INVALID` line keeps its existing meaning and is not replaced.

The semantics are unchanged: `Some("")` is successful construction of an empty
`String` value, `None` is rejection. An empty stored value is neither a NUL
character nor missing data, and a test asserts accepted-empty and rejected-empty
share the same written input with opposite outcomes.

Six loader tests cover valid empty input, valid empty stored output,
whitespace-only input normalizing to empty, invalid empty input for a different
profile, the whole-field scope rule, the entire pre-existing escape vocabulary, and
that every shipped corpus still loads. Malformed cases still **panic** rather than
being skipped — including a `VALID` line whose single field is the token — so the
token created no new way to slip a bad case past the loader. All three backends'
loaders received the identical change.

### 5.2 Conformance data, and two redirected Task 039 expectations

Two new shared corpora, read by all three backends' probes:

- `tests/fixtures/string/whitespace-visible-collapse.txt`
- `tests/fixtures/string/whitespace-visible-preserve.txt`

They are **separate files on purpose**: the same input has different expected
stored values under the two policies, and merging them would invite copying one
policy's expectations onto the other. Between them they cover the empty input;
SPACE-, TAB-, LF-, CR-only and mixed whitespace; leading and trailing runs;
repeated interior spaces and mixed interior whitespace; printable ASCII boundaries
and punctuation; idempotence; LF/CR preservation versus normalization; TAB
rejection where the preserved pattern excludes it; NUL, VT, FF, DEL and U+001F;
nonbreaking space, U+2028 and U+3000; and non-ASCII letters and multibyte
characters. Cases needing generated lengths — normalized length exactly N and
N+1, raw length beyond N normalizing within N, and lengths between 1024 and 4096 —
are emitted programmatically. Every valid case asserts the **exact returned
text**, not merely acceptance, and is re-fed to the constructor to prove
idempotence.

Two Task 039 expectations were **redirected, not weakened**. Both listed
`[ -~\n\r]{0,4096}` — UCI 2.5's `QueryString4096Type` — among shapes that must
fail closed, which was correct when no profile implemented a class containing LF
and CR. Asserting it now would be asserting a bug. In their place:

- the classifier test becomes a **stronger** claim: that shape must never be the
  `VisibleAscii` profile (whose emitted code rejects LF and CR) *and* must be
  exactly the evidenced `WhitespaceVisible` profile. It also confirms `0..4096` is
  still not a visible-ASCII bound pair;
- the coverage control's role moves to a positive control in
  `whitespace_visible_coverage.rs`, plus a test asserting the visible-ASCII
  accepted/rejected sets are unchanged — specifically that `0..4096` is baseline
  for the new family yet still **not** baseline for visible ASCII.

No other test was deleted, skipped, or loosened.

### 5.3 Classification and coverage controls

Asserted for the new family: every evidenced tuple is supported; the whole family
renders in one schema; semantic aliases under any name receive identical support;
a UCI-looking name with wrong facets stays unsupported; a missing or changed
`whiteSpace` facet is never ignored; mismatched pattern/facet bounds — including
mismatched *minima* specifically — are not approximated; extra groups, extra
alternatives, a `length` facet, a numeric facet, and a TAB-bearing class all stay
rejected; unobserved ordinary bounds (`0..512`, `1..2048`, `0..1023`, `1..4097`)
and huge bounds (`1..u64::MAX`, `0..2^40`) fail closed; and the older profile
controls are unchanged.

## 6. Compiler and runtime evidence

Generated output is **compiled and executed**, not only snapshotted.

| Toolchain | Local version | Invocation |
| --- | --- | --- |
| GNAT | `GNATMAKE 14.2.0` | `gnatmake -q probe.adb`, run twice: ordinary, and under `pragma Assertion_Policy (Ignore)` |
| rustc | `1.98.1 (48a229cea 2026-09-01)` | `rustc --edition 2021` |
| C++ | `g++ (Debian 14.2.0-19) 14.2.0` | `-std=c++17 -Wall -Wextra -pedantic-errors` |

CI toolchain versions are pinned by the workflow rather than by this document and
are reported separately; the versions above are the **local** ones.

Both maxima (1024 and 4096) and both normalization modes are exercised in every
language. The named runtime probes are:

- `generated_whitespace_visible_validators_match_the_shared_corpora` (Rust, C++);
- `generated_whitespace_visible_validators_match_the_shared_corpora_under_both_policies`
  (Ada — the `Assertion_Policy (Ignore)` run proves enforcement does not depend on
  assertions being enabled, because `Constraint_Error` is raised from explicit code
  rather than from a `pragma Assert`);
- `the_private_whitespace_visible_storage_cannot_be_reached_from_outside_the_module`
  (Rust);
- `task041_selected_rust_client_constructs_and_reads_the_new_values`;
- `task041_selected_cpp_client_constructs_and_reads_the_new_values`;
- `task041_selected_ada_client_exercises_the_sequences_under_gnat`.

Ada additionally exercises null and non-1-based input `String` ranges (a
`String (7 .. 12)` slice and its null `(7 .. 6)` sub-slice), accepted-empty
construction separately from prohibited default construction, mixed
`Create`/`Value` overloads alongside the Task 037 and Task 039 profiles in one
declarative part, bounded partial/full/overflow plus `Clear` and reuse, the
positive-minimum shape's copy/assignment/too-short rejection, and unbounded growth,
copying, clearing and reuse. C++ builds every control-character input with an
**explicit length** so embedded-NUL cases are not truncated.

## 7. Service-contract integration

A new synthetic fixture pair,
`tests/fixtures/service-generate/whitespace-visible.{xsd,yaml}`, selects one
message whose closure carries both policies, both maxima, and required, optional,
bounded, positive-minimum-bounded and unbounded occurrences.

- **Selection becomes ready.** All three backends report `status: READY`. Measured
  on the reviewed main (`72d12a3`) with the same fixture, all three reported
  `NOT READY` with `blocker: {urn:test}CollapsedRemarks`, so the READY result is
  attributable to this change rather than to an accident.
- **Unselected unsupported declarations do not contaminate it.** The schema also
  declares `UnobservedCollapsedText` — collapse paired with UCI 2.6's minimum of
  one, a triple every component of which is evidenced somewhere but whose
  *combination* appears in no release. It is reachable in the schema, absent from
  the selection, and does not appear in the projected closure.
- **Unsupported selected neighbours still block, with no partial output.**
  Selecting that declaration's own message yields `NOT READY` naming it as the
  blocker, generation fails, and the output directory is asserted empty.
- **Generated clients construct and read the new values**, in all three languages,
  through every occurrence shape.

No change was made to `service_plan.rs`, `service_readiness.rs`, or
`service_generation.rs`: the capability propagates through the single shared
coverage snapshot and the ordinary backends.

## 8. Measured results

Measured before and after with identical pinned inputs and commands. The "before"
column was produced from a clean worktree at the reviewed main
`72d12a32e902a198d20450554a93bfa3d140d15b`; it reproduces the recorded Task 040
baseline in all twelve cells exactly.

### 8.1 Workspace tests

`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`, counting summed `test result`
passes the same way in both runs:

| | Passing |
| --- | ---: |
| Baseline (`72d12a3`), measured | 797 |
| Task 041 | **832** |

The recorded merged baseline of 797 was **measured**, not assumed, and matched.
The +35 are all new: 5 classifier tests, 5 coverage controls, 1 redirected-case
test, 3 Rust / 2 C++ / 2 Ada generated-code probes, 6 corpus-loader tests, and 6
CLI integration tests, plus the enumerating inventory tests. No existing test was
deleted, skipped, or weakened.

### 8.2 Coverage — all twelve cells

Declarations fully renderable, out of 5557 (2.5) and 5570 (2.6):

| Cell | Before | After | Delta |
| --- | ---: | ---: | ---: |
| 2.5 closed, Ada / Rust / C++ | 5314 / 5391 / 5394 | 5317 / 5394 / 5397 | **+3 / +3 / +3** |
| 2.5 open, Ada / Rust / C++ | 5229 / 5303 / 5306 | 5232 / 5306 / 5309 | **+3 / +3 / +3** |
| 2.6 closed, Ada / Rust / C++ | 5335 / 5413 / 5417 | 5338 / 5416 / 5420 | **+3 / +3 / +3** |
| 2.6 open, Ada / Rust / C++ | 5250 / 5325 / 5329 | 5253 / 5328 / 5332 | **+3 / +3 / +3** |

`kinds` rises by the same 3 in every cell (2.5: 5444 → 5447; 2.6: 5457 → 5460).
Field-types, field-occurrences, and message-closures are unchanged in all twelve
cells.

### 8.3 Explaining the movement

The delta is **+3, not +2**, and every unit is accounted for. Per release, the
three newly supported declarations are:

| Declaration | Attribution |
| --- | --- |
| `WhitespaceVisibleString1024Type` | direct match of a new profile |
| `WhitespaceVisibleString4096Type` | direct match of a new profile |
| `QueryString4096Type` | **semantic alias / equivalent declaration** — in 2.6 an exact alias of the 4096 profile; in 2.5 its own distinct evidenced triple |

There are **no transitive dependency gains** in the coverage figures: no other
declaration in either release became renderable, because the remaining consumers
of these types — chiefly `SecurityInformationType` — are still blocked by
independent, out-of-scope constructs (§9). The `+3` is therefore exactly the three
directly classified declarations, and `QueryString4096Type` is the third one
precisely because the classifier reads facets rather than names.

### 8.4 Selected `PositionReport` recheck

The established UCI 2.5 selection, closed world, re-run against the pinned bytes:

| Backend | Baseline | Task 041 | First blocker |
| --- | ---: | ---: | --- |
| Ada | 51/60 | **53/60** | `SecurityInformationType` (unchanged) |
| Rust | 55/60 | **57/60** | `SecurityInformationType` (unchanged) |
| C++ | 55/60 | **57/60** | `SecurityInformationType` (unchanged) |

`PositionReport` is **still NOT READY in every backend.** Two of its dependencies
improving does not make it ready, and it is not claimed to be.
`SecurityInformationType` remains the first blocker because it depends on
`WhitespaceVisibleString1024Type` and `WhitespaceVisibleString4096Type` — now
supported — **and** on `NATO_SpecialWordsType` plus `DeclassExceptionEnum`,
`FGI_SourceOpenEnum`, `FGI_SourceProtectedEnum`, `OwnerProducerEnum` and
`ReleasableToEnum`. The measured Ada diagnostic is explicit:

```text
backend boundary: Ada cannot form a legal identifier from "25X1"
in the members of DeclassExceptionEnum
```

Enum identifier remapping and `NATO_SpecialWordsType` are both explicitly out of
scope for this task, so those blockers are expected to remain.

### 8.5 Determinism and full-schema first failures

Repeated generation is deterministic: two consecutive UCI 2.5 closed-world
coverage runs are byte-identical, and both match the earlier measurement run
byte-for-byte.

Full-schema `generate` first failures were re-measured rather than assumed, and
are **unchanged** — all are reserved-word/identifier boundaries unrelated to this
task:

| Release | Ada | Rust | C++ |
| --- | --- | --- | --- |
| 2.5 | `AltitudeRangePairType / Range` | `ConfigurationParameterType / Type` | `ApprovalResponseType / Operator` |
| 2.6 | `AltitudeRangePairType / Range` | `ConfigurationParameterType / Type` | `COMINT_ChangeDwellType / Delete` |

Older full-schema blockers did **not** disappear, and none was expected to.

## 9. Files changed

| Area | Files |
| --- | --- |
| shared classifier | `crates/codegen-core/src/string_profile.rs`, `crates/codegen-core/src/lib.rs` |
| backends | `crates/backend-{ada,rust,cpp}/src/lib.rs` |
| corpus loaders | `crates/backend-{ada,rust,cpp}/tests/common/mod.rs` |
| new corpora | `tests/fixtures/string/whitespace-visible-{collapse,preserve}.txt` |
| new fixtures | `crates/xsd-frontend/tests/fixtures/backend-string-whitespace-visible.xsd`, `tests/fixtures/service-generate/whitespace-visible.{xsd,yaml}` |
| new tests | `crates/backend-{ada,rust,cpp}/tests/whitespace_visible.rs`, `crates/backend-rust/tests/corpus_loader.rs`, `crates/codegen-core/tests/whitespace_visible_coverage.rs` |
| amended tests | `crates/codegen-core/tests/string_profile_coverage.rs`, `crates/cli/tests/service_generate.rs` |
| CI | `.github/workflows/ci.yml` |
| docs | this file, `docs/roadmap.md`, `docs/backend-compatibility.md` |

### Generated API and dependency implications

- **Additive only.** Five new carrier types appear where a supported
  whitespace-visible declaration exists. No existing generated spelling changed,
  and no existing carrier's behaviour changed.
- **One generated-comment correction** in Ada, described in §4.1. It is a comment,
  not an API change.
- **No new dependency** in the generator or in generated code: no regex crate, no
  `GNAT.Regpat`, no `<regex>`, no `<cctype>`.
- Generated Ada gains a bounded stack buffer of at most 4096 `Character` inside
  collapse `Create` calls; generated Rust and C++ allocate one normalized
  `String` per collapse construction. Preserve carriers allocate exactly as the
  existing visible-ASCII carriers do.

## 10. Out of scope

`NATO_SpecialWordsType`, generic XML Schema regex translation or a new regex
engine, fixed-length visible-string families, enum identifier remapping, new
temporal formats, codec/transport/CAL/runtime integration, and any further
redesign of Task 040 repeated storage. No declaration-name-based exception was
added, and neither `SecurityInformationType` nor `PositionReport` is special-cased
anywhere — their partial improvement follows from ordinary dependency analysis.
