# Task 080 — Interactive UCI structure explorer

## Isolation and authority

Started directly from `origin/main` at
`a9378b31608a38dd5eb0601e57a8cfd50ace76b0` in the isolated worktree
`/home/zboll/git/ams-gra-codegen-oms-task080`, branch
`feature/080-uci-structure-explorer`. No Task 080 predecessor existed.
Task 077 and its Deep CI correction were inspected read-only; no runtime,
workflow, guard, README, roadmap, Cargo.lock or Sleet harness changes are needed.

The existing frontend → validated normalized `SchemaIr` → schema-docs pipeline
is the only schema authority. Documentation does not require backend readiness.
Structural export uses `lossless_structural_with_index`, preserving ancestry,
declaring owners, duplicate inherited member names and Record/Choice boundaries.
It does not flatten effective fields or interpret XSD in the browser.

## Navigation and semantics

Every release index links to its explorer. Each type and message has a
root-specific exploration link. The assembled Pages selector has Reference and
Explorer choices for each known release; generic CLI output has no assumed
release parent directory. Existing ordinal type URLs, search and message anchors
are preserved. Explorer assets are not loaded by reference pages.

Choose Messages or Types and refine the root search. Select a name to inspect it;
use the separate `+`/`−` disclosure button to expand/collapse. Tab reaches native
buttons and links; Enter/Space activates them. There are no ARIA tree roles or
custom arrow-key model. Collapsing an ancestor of the selected node selects that
ancestor and focuses its disclosure. Collapse exploration resets the root and
materialized-row budget. Explore as root starts a distinct history entry;
Back/Forward restores root and selected path. Copy supplies the structured path
and URL, with a selectable textarea when clipboard access is unavailable.

Cardinalities are exact **local** `min..max` ranges; `*` means unbounded.
Optional means min=0; Repeated independently means finite max>1 or unbounded.
`0..0` also says Not occurring. Nillability is separate. Required local members
under optional ancestors are conditionally present; the details panel explains
this without multiplying ranges. Choice/Alternatives sections distinguish
alternative selection from local cardinality. No missing compositor occurrence
information is invented. Repeated branches show the structure of one item, never
instance entries, defaults or fabricated values. This is not runtime validation
or payload-construction advice.

Inheritance segments are base-to-derived and independently collapsible. Alias
identity and target remain distinct. Lists have explicit item steps. Abstractness
is visible; immediate known derived types are listed separately as “Known derived
types in this schema”, not inserted as fields. This does not claim exhaustive
runtime extension knowledge.

## Data and link contracts

Four additive files: `explorer.html`, `assets/explorer.css`,
`assets/explorer.js`, `assets/explorer-data.js`. The classic external data script
is exactly `window.schemaExplorerData = <JSON>;\n`, parsed as JSON (never executed)
by the artifact validator. Graph version 1 contains schema metadata, namespaces,
type and message record arrays. A QName is `[namespace_uri, local_name]`, not a
display prefix or concatenated lookup name. Browser lookup uses `Map` keyed by
JSON-encoded QName tuples. Each type is declared once, with local member arrays;
effective segments reference their declaring owner and compositor. Ancestry
retains empty owners. References are `{named: QName}` or `{primitive: kind}`.
Absent wire namespace is JSON null, distinct from a present empty string.
Occurrence min/max are decimal strings; max null means unbounded. Numeric and
length constraint display values are strings, retaining large integers exactly.
Pattern groups preserve AND between restriction groups, OR within each group,
and dialect. Enumeration wire values and docs are retained separately.

Reference URLs come from the existing deterministic page registry. Message
anchors retain schema order. Schema metadata includes normalized schema version,
namespace/prefix metadata and the deterministic set of message source documents
as root provenance; it does not invent release names from directory layout.
No timestamps, random IDs or transient explorer checkout paths are generated.

State is `#uci=` followed by percent-encoded JSON:

```json
{"v":1,"kind":"message","root":["urn:example","Report"],"path":[{"owner":["urn:example","Root"],"compositor":"Record"},{"member":0}]}
```

`kind` is message/type. Structural steps select an exact owner/compositor,
then a local member occurrence index. Alias/list edges use `{step:"alias"}` or
`{step:"item"}`. Owner and occurrence index disambiguate inherited duplicate
names. The release is the explorer page URL. Paths do not contain executable
URLs or the expanded DOM. Restoration validates the complete path before
changing the display; malformed, unknown, stale, terminal and recursive paths
show errors rather than selecting a similarly named member. Reordering members
can make occurrence-index links stale; this format is release-specific, not a
cross-version semantic link contract.

## Limits, safety and compatibility

Expansion state belongs to a structural path. Recursion is detected only on that
ancestor path; sibling occurrences expand independently. Recursive edges expose
Explore as root, not unlimited expansion. Maximum path/expansion depth is 24
steps, materialized rows 2,000, rendered root search results 50, fragment length
16,384 characters. Search reports full counts and asks for refinement. Details
lists initially render 50 entries per list, with explicit next-50 controls;
pattern sublists use the same bound. Expansion limits show feedback; no Expand
everything operation is offered. Collapse exploration resets the row budget.

Schema and URL display text uses text nodes/textContent only. No HTML recovery,
innerHTML, eval, inline event handlers, prototype-derived lookup or arbitrary
fragment navigation URLs are used. Registry-generated reference URLs are checked
before UI initialization. Hostile markup, script-closing text, Unicode,
`constructor` and `__proto__` are fixture-tested.

There is no fetch, XHR, module, service worker, CDN, telemetry or runtime package
download. External classic local scripts support both file opening and project
prefix hosting. A persistent reference link and noscript explanation remain
usable without JS; missing/invalid data is an explicit error. The explorer
requires modern browsers supporting Map, replaceChildren, optional modern DOM
APIs and Array.at; Chromium/Firefox are qualified below, not historical engines.
Legacy reference HTML remains available as fallback.

Responsive panels stack below 1,000px. Wide structures intentionally scroll
inside their pane; body text wraps at narrow widths and increased text sizing.
Badges use words and ranges as well as color; forced-colors selection has a border.

## Local qualification

Rust 1.95.0: schema-docs integration/data tests (10), CLI docs tests (2), enum
remapping regressions (4), affected all-target check and Clippy `-D warnings`,
fmt, shell syntax and diff checks passed during implementation.
Python Pages tests pass positive artifact controls and isolated negative controls
for wrapper/version, inventory/reference, metadata and fragment failures, while
retaining ordinary anchor, provenance, archive and inventory protections.
The material-purity check now examines paths relative to the artifact root,
not the checkout's absolute parents: an artifact may be staged beneath `target`
without allowing a `target` directory inside the published artifact. The exact
file-set check is strengthened to set equality, not merely a file count.

Opt-in browser package: `tests/task080-browser`, exact Playwright 1.61.1 with npm
lockfile. `npm ci`, `npx --no-install playwright install chromium firefox`, then
`npm test` against `target/task080-fixture`. Generate it with the task worktree's
CLI using `crates/schema-docs/tests/explorer.xsd`. `node real.cjs` uses
`TASK080_PAGES` for a complete Pages artifact. Browser/server resources are closed
in finally handlers; the HTTP server binds an ephemeral loopback port and mounts
the artifact under `/ams-gra-codegen-oms/`. Unexpected external requests are
blocked and asserted absent. Screenshots are ignored local evidence, never part
of the publication artifact or commit.

Fixture tests passed Chromium **149.0.7827.55**, Firefox **151.0**, Node **20.19.2**,
Playwright **1.61.1**, HTTP project prefix and ordinary file:// with no disabled
browser security. Coverage includes root message/type search, multilevel and
independent sibling expansion, optional/repeated/zero/large ranges, owner labels,
choices, recursion, constraints/enums/docs and reference-link targets, copy
fallback/deep-link reload, Back/Forward, malformed/oversized state, native keyboard
disclosures, hostile text, 390px width at 200% text sizing, JS-disabled reference
navigation, and missing/invalid data. Desktop/narrow screenshots were visually
inspected: no panel overlap or page-level clipping after the wrapping fix;
structure-pane horizontal scrolling at narrow widths is intentional.

### Production artifact evidence

Two independent production builder executions fetched and verified the existing
public pinned tags, built the task worktree's `target/debug` CLI with Rust 1.95.0,
and generated into empty task-owned destinations. They took **495 s** and
**494 s**, including acquisition and full validation. Both passed the strengthened
validator. Complete paths and bytes compare identically: **11,144 files**,
**96,826,254 bytes**; independent Python byte comparison took **0.604 s**, and
the full `diff -qr` also passed without exclusions.

| Release | Types | Messages | Files | Graph bytes | Release bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2.5 | 5,557 | 722 | 5,565 | 11,115,091 | 48,994,968 |
| 2.6 | 5,570 | 725 | 5,578 | 11,142,756 | 47,829,840 |

Pinned inputs remain `093610b7753944059360d3236770ab446d039556` (2.5) and
`78eb61b6112c8bffa40820c33124b57787fc5bd9` (2.6). Loaded normalized versions are
`002.5.0` and `002.6.0`. No source trees, output, browser binaries or screenshots
are committed. Preliminary artifacts made before final asset changes are not
used for final-source reproducibility evidence.

Real-release browser results are recorded in the PR qualification report.
The bounded script inspects PositionReport, OrderOfBattle and EntityPositionType,
compares selected field names/ranges/docs to graph data and independently rendered
reference pages, and opens reference navigation. Initial structure has **0 rows**
before root selection. Representative exploration after 14 disclosures has
**53 rows** (PositionReport/OrderOfBattle) and **32 rows** (EntityPositionType),
not the whole graph. Chromium initial readiness observed around 242–260 ms in
the completed-artifact run; timings are observations, not performance guarantees.
Actual EntityPositionType fields FixedPositionType, RelativePosition and Zone
are each 0..1 in both releases. PositionReportMT inherits MessageType and locally
declares MessageData 1..1; OrderOfBattleMT additionally has ObjectState 0..1.
Inherited SecurityInformation is 1..1. Expanded security content visibly includes
OwnerProducer 1..*, SCI_Controls 0..20 and explicit OwnerProducer choice alternatives.

## Hosted coverage and scope

Existing Fast CI runs Rust workspace tests/check/Clippy/fmt (including the new
renderer/data and CLI tests). Pages runs the Python validator tests and pinned
production build on applicable main pushes, then publishes post-merge. Browser
tests are opt-in/local; green Rust CI is **not** browser evidence. No workflow or
CI guard changes, manual production dispatch or feature-branch deployment.

Deferred: cross-version visual diff, service filtering, exhaustive all-path
search, editable instances, samples, whole-schema graphs and production schema
validation. No server service, runtime codec/model change or compatibility verdict.