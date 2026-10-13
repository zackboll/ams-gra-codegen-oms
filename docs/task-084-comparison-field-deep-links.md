# Task 084 — Exact comparison field links

## Isolation and authority

Immutable starting `origin/main`:
`a9b94e3b90349055db130f8028c5b057c88813b2`. Fresh fetch matched the supplied
baseline; Task 082 and its LF-only Unicode TSV correction (`a348567`) are merged.
Branch: `feature/084-comparison-field-deep-links`. Isolated worktree:
`/home/zboll/git/ams-gra-codegen-oms-task084`. Scratch and evidence:
`/tmp/task084`. Sibling worktrees, public inputs, Playwright installation and
earlier qualification evidence are untouched. PR #83 was inspected read-only;
its current file list has zero overlap. No Task 083 functionality is imported.

Task 069's unchanged comparator/TSV remains the semantic authority. Each release's
exported explorer graph remains the navigation authority. No Rust, workflow,
manifest, lockfile, runtime, backend or explorer production assets change.

## Optional comparison-data version-1 extension

A member change may contain `memberDestinations: [beforeURL, afterURL]`.
Positions retain release order 2.5, 2.6. A missing or unsupported side is null;
if neither side has an exact association the property is omitted. Historical
records without this property remain valid. Semantic fields, record ordering,
status classification, summaries, and comparison share state are unchanged.

Association is conservative candidate matching, not parsing arbitrary Rust Debug
text. Only `MEMBER_ADDED`/`MEMBER_REMOVED` with the complete `/presence` suffix,
and `MEMBER_CHANGED` with `/type_ref`, `/cardinality`, `/wire_namespace` or
`/nillable` qualify. Printable ASCII member names have an exact supported Debug
representation (quotes/backslashes included); control and non-ASCII names are
deliberately unsupported. Their semantic evidence and type navigation remain.
Declaration constraints, enum changes, whole-member order, base/abstractness
changes, and whole-type presence changes do not acquire field links.

The full declaration QName is looked up independently in each graph. Only its
local Record/Choice members are candidates. Require a unique exact label match,
exactly one own declaring-owner/compositor segment, and an integer local index
equal to that member's array position. Inherited same-named fields are never
substitutes. Before and after indices are read independently, not copied.

Links use the existing state `{v:1,kind:"type",root:QName,path:[
{owner:QName,compositor:"Record"|"Choice"},{member:index}]}`. There are precisely
two path steps; the root is the changed type, not a message or instance path.
The independent Pages validator repeats the candidate proof using its own exact
name formatter, verifies release/path safety, invokes `explorer_state()`, and
requires exact state equality. Generic structural validity alone is insufficient.
Duplicate semantic records are detected independently of optional navigation.
All existing publication inventories, provenance, symlink and external-link
restrictions remain intact.

Each change article renders native keyboard-reachable actions labeled “Reveal
field in UCI 2.5” and/or “Reveal field in UCI 2.6”. Type reference/explorer actions
remain. Text rendering uses textContent; no innerHTML, fetch, framework, CDN or
new production dependency. The page explains schema definitions versus runtime
instances. Existing wrapping handles narrow widths without a CSS change.

## Independently authored regression evidence

Python packager: **7 test methods**; validator: **5 test methods**. Positive
subcases cover Record and Choice, added-after/removed-before, index differences,
same local names in different namespaces, inherited own versus base owner,
several properties, optional/repeated fields, escaped hostile ASCII names and
unchanged declaration/root actions. Negative cases omit or reject unsupported
names, duplicate matching members, missing declarations/members, wrong namespace,
wrong compositor, out-of-range/wrong/stale indices, inherited same-named
substitutes, malformed suffixes, extra path/state fields, wrong release and
absolute/external URLs. Separate integration tests exercise optional-property
length/type/null handling and historical property-free records.

Task 082A transport cases still include actual U+2028/U+2029 code points and exact
before/after preservation. Unchanged Task 069 tests: **7 semantic tests** and
**4 CLI tests**. Unchanged Task 081 pure search tests: **6 passed**. JavaScript
syntax, Pages builder shell syntax and `git diff --check` pass.

Production-CLI synthetic qualification preserves the original **16** independently
expected Task 082 records. Supplemental task-owned inputs produce an inherited
Record and a Choice whose changed `Field` moves **0 → 1**, with a same-named base
field at index zero. Their order records receive only type-root navigation.
Repeat synthetic generation matches exact comparison-data bytes:

* Original fixture SHA256:
  `f22c4f3f83c1659077ddead6a3a0763f08cfab99a36967721218b03cafcaedab`.
* Supplemental fixture SHA256:
  `02fac3682b3d4fede390ae660dc68d8e030257d62a19af62cfd32d3863c4ed6e`.

## Measured pinned release evidence

| Category | Added | Removed | Changed | Unchanged |
| --- | ---: | ---: | ---: | ---: |
| Types | 39 | 26 | 162 | 5,369 |
| Messages | 3 | 0 | 0 | 722 |

Exactly **309** Task 069 records. Removing only `memberDestinations` from the
new model reproduces every original semantic record and ordered value. The
cached pinned TSV SHA256 remains
`c58fbca0f0958d063ec156dfca065f6826f5a1c0088cd2f688947644a433d09c`.
No production count is hardcoded.

| Member-change kind | Linked records | Unlinked records | Destinations |
| --- | ---: | ---: | ---: |
| MEMBER_CHANGED | 102 | 0 | 204 |
| MEMBER_ADDED | 30 | 0 | 30 |
| MEMBER_REMOVED | 56 | 0 | 56 |
| Total | **188** | **0** | **290** |

No unsupported real-release member names or unexplained omissions.
Changed members: 99 Record records and 3 Choice records. Actual Choice examples:
`AtomicValueType.HexBinaryValue` (16 → 16),
`EntitySourceIdentifierType.Operator` (6 → 6), and
`PO_ComponentSettingsFocusSweepSettingsStepTimeType.CollectionTime` (1 → 1).
All have namespace `https://www.vdl.afrl.af.mil/programs/oam`.

Measured `ADS_B_KinematicsContributionType.AirborneSurfaceFormat` selects local
Record index **21 in each release**, preserving
`primitive:Boolean;binary=None` →
`named:{https://www.vdl.afrl.af.mil/programs/oam}EmptyType;binary=None`.
Its `SV_QualitySIL_Supplement` change is also exercised. Actual added
`ADS_B_ModeStatusReportType.UnmannedOperation` has only the 2.6 index **9** link;
removed `MannedUnmannedOperation` has only the 2.5 index **9** link. These are
distinct definitions, not a rename inference.

Repeated pinned packaging on the verified cached TSV/graphs is byte-identical:
SHA256 `97cb5c90d3847f3d6d242602a997f0e333fb73fd36836e3685ea19f8d9a4a60c`.
Full Pages validation on a task-owned copy of the qualified publication passes:
**11,150 files, 101,220,161 bytes**. Every non-null destination is independently
validated. This cached-artifact run is separate from the fresh production build.

## Browser qualification

Reused Playwright **1.61.1**, Chromium **149.0.7827.55**, Firefox **151.0**, with
normal security, project-prefix HTTP and ordinary file URLs. Comparison matrices
pass **4/4 original synthetic**, **4/4 supplemental**, **4/4 cached pinned**.
Each combination exercises 6 original, 4 supplemental, or 10 pinned field actions,
plus click-through and keyboard Enter, correct root/segment/index/member label,
declaring owner, exact local cardinality and correct reference target, reload and
normal Back navigation. Malformed structural links select no substitute. Added
and removed sides, hostile schema text, 375px layouts, existing filters/share
state/history, and zero uncaught errors/external requests are checked.

Back and reload are independently exercised: Firefox's persisted reload history
behavior made a combined reload-then-Back test unreliable; no production code
was changed to accommodate the harness. Re-enter the actual link after normal
Back, then reload and verify identical details.

Unchanged Task 080A fixture navigation: **4/4**; actual pinned navigation:
**8/8**. Task 081 fixture field search: **4/4**; actual pinned field search:
**8/8**, including 24 roots and nested reveals. Evidence is retained under
`/tmp/task084/evidence`; historical evidence is not overwritten.

## Fresh build and handoff gate

The unmodified production builder completed with fresh pinned acquisition in
`/tmp/task084/scratch` and publication in `/tmp/task084/pages`. Its freshly
generated release directories already compare byte-for-byte against Task 082:
**5,566 files in 2.5**, **5,579 files in 2.6**, zero differences. Complete build
exited **0**, including tag/root digest checks, safe archive extraction, semantic
comparison (**449 seconds**) and full publication validation. Final inventory:
**11,150 files, 101,220,161 bytes**. Comparison data is byte-identical to repeated
packaging on the previously verified TSV/graphs (SHA256 above). File inventory
is unchanged; the only publication byte differences are
`compare/index.html`, `compare/assets/compare.js`, and
`compare/assets/compare-data.js`. No asset added or workflow budget changed.

Fresh publication comparison browser rerun: **4/4 passed**, exit **0**, with
10 version-specific field actions per combination, both engines/transports and
the exact final production bytes. All required local implementation, identity,
reproducibility and qualification gates are complete:
**READY_FOR_INDEPENDENT_REVIEW**.

Final normal commit, PR URL, local/remote/PR head parity and the once-queried
exact-head hosted status are supplied in the PR handoff/final report rather than
invented here before commit. Leave the PR open, non-draft, unmerged, with
auto-merge disabled. No manual workflow dispatch, merge, deployment or Task 085.