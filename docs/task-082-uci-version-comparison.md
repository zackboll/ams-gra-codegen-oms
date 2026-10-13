# Task 082 — Static direct semantic comparison

## Baseline and isolation

Starting origin/main: `051a634fa7a69f87a9c4d6330b6edee1bcc5c0bf`.
Task 081 head `c20547f` is an ancestor, with its merged implementation intact.
No prior Task 082 branch/PR was found. Branch:
`feature/082-uci-version-comparison`; worktree:
`/home/zboll/git/ams-gra-codegen-oms-task082`. Sibling worktrees are preserved.
Builds, inputs, browser evidence and rendered artifacts are task-owned under
`/tmp/task082`; no complete UCI release is committed.

## Authority and transport

Validated SchemaIr → existing `compare_schemas()` → unchanged Task 069 TSV is
the only semantic authority. No Rust source, comparator contract, golden file,
explorer implementation, runtime, backend, dependency, workflow or CI guard changes.

`site/compare/package.py` decodes only Task 069's TSV escaping and packages its
formatted values without interpreting references, constraints, patterns or bounds.
JSON null means Task 069's missing `absent` sentinel. Literal `"absent"`, `""`
and all other formatted strings remain strings. Ordered property labels and
change-kind labels are retained exactly. Inventories provide presence and exact
destinations, not semantic property comparison. Summaries and totals must reconcile.
Full `[namespace_uri, local_name]` tuples distinguish categories and equal local
names. Added/removed declarations have only the engine's single presence record.

The external classic data wrapper is `window.schemaCompareData = <JSON>;\n`.
Model version 1 includes scope, release pins and formatted schema versions,
Type/Message summaries, and declaration records with ordered changes and two
nullable destination objects. The four published comparison files are exactly
`compare/index.html`, `compare/assets/compare.css`, `compare/assets/compare.js`,
and `compare/assets/compare-data.js`. Python helpers are never published.

Share state is `#compare=<percent-encoded JSON>` with exactly
`{v:1,category:"Type"|"Message",qname:[namespace,local]}`. Validate size, keys,
version, category and QName before lookup; unknown/stale state clears details and
reports an error, never substitutes another identity. Explorer links retain the
existing version-1 `#uci=` state with root QName and empty path. URLs come from
each version's inventory, never reused ordinal page numbers.

The UI renders at most 50 list buttons per page, uses literal substring search,
native labeled keyboard controls and visible focus. Every property is displayed
with exact before/after text in bounded, focusable scroll regions. All schema text
uses textContent; no fetch, modules, CDN, worker, telemetry or service is needed.
The report is prominently **Direct normalized SchemaIr changes — not a
compatibility verdict.** Unchanged direct message payloads do not assert unchanged
transitive dependencies; Task 078 remains that separate authority.

## Production pipeline

One fetched temporary repository verifies both pinned tag commits. A detached
temporary 2.5 worktree preserves its entire dependency closure when the acquisition
checkout switches to 2.6. Existing archive safety checks remain; both root SHA256
values are explicitly verified. The executable is built once. Existing release
generation and provenance paths are unchanged. One semantic comparison per build
is followed by packaging, asset assembly and complete fail-closed validation.
No source checkout, ZIP or XSD is copied into publication.

The artifact validator retains the exact root/release inventory, size limit,
symlink, provenance, HTML anchor and explorer-state checks. It additionally checks
exact comparison assets, wrapper/version/scope/pins/schema metadata, complete
unique QName inventories, status/count/presence consistency, property value and
label shape, duplicates, and each reference and explorer destination against the
specified release identity. Adversarial fixtures have all required assets before
introducing each intended defect.

## Reproduction and local qualification

Use Rust 1.95.0 with locked Cargo dependencies. Existing Task 069 engine tests (7),
schema-docs integration tests (8 browser-export and 2 renderer tests), CLI diff (4)
and CLI docs (2) passed. Formatting, shell/JS/Python syntax and whitespace checks
passed. Python discovery executes 5 test methods, including transport checks and
positive/negative Pages cases. Existing explorer search Node suite: 6 passed.

Generate the synthetic qualification using:

```sh
python3 site/compare/qualify.py target/debug/ams-gra-codegen-oms /tmp/task082/synthetic-new
```

Hand-authored expected property/kind lists and exact representative values cover
16 records: type/message additions/removals, unchanged common payload, field
addition/removal, cardinality/type reference, abstract/base, enumeration addition
and ordering, minLength/pattern, namespace replacement, multiple changes on one
type, documentation-only exclusion and equal local names in two namespaces.
Quotes, backslashes, tabs, newlines, Unicode and script-like enum/pattern text cross
the actual production TSV → JSON → browser boundary. Synthetic comparison data
also passes the artifact comparison validator.

Browser runner: `tests/task080-browser/compare.cjs`, with `TASK082_PAGES` pointing
at the generated artifact. Reuses the existing locked Task 080 Playwright install
through NODE_PATH, without modifying the sibling installation. Synthetic matrix:
Chromium 149.0.7827.55 and Firefox 151.0, project-prefix HTTP and ordinary file,
4/4 passed. Status/category filters, full-QName search, keyboard selection,
grouped exact values, version-specific destinations, added/removed navigation,
reload, Back/Forward, invalid/oversized/unknown/hostile fragments, inert hostile
schema text, 375px layout, and no external requests or uncaught errors passed.

Both real generated release directories are byte-identical to qualified Task 081
outputs: 5,566 files for 2.5, 5,579 for 2.6, including graphs and all legacy pages.
Existing Task 080A navigation matrix: 8/8 passed. Task 081 field search/reveal
matrix: 8/8 passed on these same real-release bytes. Local browser coverage is
not hosted CI coverage; no workflow dispatch or deployment is performed.

Pinned build timing, final inventory/determinism, exact-head parity and hosted
status are recorded after completion, not assumed from historical evidence.

## Measured pinned qualification

First complete production build: **922 seconds (15m22s)**, including fresh source
fetch, compilation, both release browsers, comparison, packaging and validation.
The added semantic comparison took **450 seconds (7m30s)** in the debug build.
This fits the unchanged 30-minute budget on the local machine; not a hosted
runner performance guarantee. No validation or release was skipped.

Recomputed types: before 5,557, after 5,570; 39 Added, 26 Removed, 162 Changed,
5,369 Unchanged. Messages: before 722, after 725; 3 Added, 0 Removed, 0 Changed,
722 Unchanged. Exactly **309** property/presence records. The complete forward
TSV SHA256 is `c58fbca0f0958d063ec156dfca065f6826f5a1c0088cd2f688947644a433d09c`,
identical to Task 069's frozen historical report; frozen metadata/summary/totals
also compare exactly. No expected semantic count is hardcoded in the viewer.

Actual generated comparison matrix: **4/4 passed**, Chromium 149.0.7827.55 and
Firefox 151.0 over project-prefix HTTP and ordinary file. The inspected removed
type is `AMTI_MessageOutputsEnum` (2.5-only `types/000039.html`); added message
`SystemSchedule` is 2.6-only `index.html#message-000681`. The two
`ADS_B_KinematicsContributionType` Boolean → named type records and
`AMTI_CapabilityType` base-reference and member-removal records display exact
Task 069 values and release-specific destinations.

Final synthetic browser matrix repeated 4/4 successfully. During qualification
the Task 080/081 sibling directories disappeared due to activity outside this
task; this agent issued no worktree removal or pruning. The already completed
baseline byte comparisons and 8+8 explorer regressions remain recorded. The
unchanged Task 080 lockfile was installed task-locally with `npm ci` (Playwright
1.61.1) for the remaining final comparison runs; no lockfile edits or production
dependencies. Existing sibling worktree registrations were not modified.

Second complete fresh production build: **919 seconds (15m19s)**; semantic
comparison **449 seconds**. Both builds exited 0 and passed complete validation.
All paths and bytes match, **without exclusions**. Exact inventory:

| Directory | Files | Bytes |
| --- | ---: | ---: |
| 2.5 | 5,566 | 49,011,702 |
| 2.6 | 5,579 | 47,846,574 |
| compare | 4 | 4,245,083 |
| root index.html | 1 | 1,510 |
| Total | **11,150** | **101,104,869** |

Sorted complete SHA256 manifest digest:
`4f5ba42aa9440ba66addb524c4c4a214086686622c3d61e81a6441eb28e94001`.
Current validator independently rechecked the full artifact successfully.
The approximately 4.25 MB comparison loads and searches statically; real browser
tests confirm the result DOM is capped at 50 rather than rendering the inventory.

All required focused local qualification passed: **READY_FOR_INDEPENDENT_REVIEW**.
Hosted exact-head CI status is separate, not assumed successful. Existing Fast CI
covers Rust; Pages tests/build run post-merge on main; Playwright and repeated
pinned artifact qualification above are local. No hosted workflow is dispatched.
The PR must remain open, non-draft, unmerged, with auto-merge disabled. No deployment
or Task 083 work.