# Task 081 — Root-scoped nested-field search

## Isolation and contracts

Immutable starting SHA: `255f7a04f70bc75f8754c32dd0c02fa1e1a9c286`.
Branch: `feature/081-explorer-field-search`; isolated worktree:
`/home/zboll/git/ams-gra-codegen-oms-task081`. Fetched `origin/main` and
verified corrective Task 080 head `6e01ad2f147ef9f750d068385d55b63475955c65`
is an ancestor. The only intervening commit is the Task 080 merge. No prior
Task 081 effort was found. Task 077 PR #77 (`07313a9`) was inspected read-only;
its changed-file set has no intersection with this task. No runtime, manifest,
lockfile, workflow, CI guard, README or roadmap changes.

Authority remains validated `SchemaIr` → existing exported graph. Graph version
and exploration-state version remain **1**. No exporter-semantic changes, XSD
parsing, reference-page scraping, reverse inheritance or inferred descendants.
The additional local classic `assets/explorer-search.js` is embedded with
`include_str!`; HTML loads data, search, then explorer. The exact Pages inventory
adds only that asset, retaining missing-file, extra-file, provenance and link
rejection. No production dependency, fetch, dynamic import, worker or CDN.

## Search semantics

The current message/type root is the scope, independent of selected path,
rendered rows, disclosure state or history of expansion. Traverse Record/Choice
segments using their full declaring-owner QNames, every declared member,
immediate named references, alias targets and list items. Include optional and
zero-occurrence declarations. Do not insert known abstract descendants.

Definition identity is `(full owner QName, compositor, local occurrence index)`.
Names and namespace prefixes are not lookup keys. Shared/inherited definitions
appear once, with one shortest representative path measured in existing
structural path steps. BFS consumes sorted segment edges: namespace URI, local
name, compositor using explicit UTF-16 lexical comparisons; member indices use
numeric order; reference tags order alias before item. BFS level order supplies
the lexicographically earliest equal-length witness. Every path retains owner,
compositor, member, alias and list steps. Cycles are declaration-deduplicated for
indexing; manual expansion still uses its unchanged path-local recursion guard.
No all-instance-location count or recursive-path enumeration is claimed.

Modes: field name, immediate referenced type (full QName/local name or displayed
primitive), field documentation only, or all three. Trim query boundary
whitespace and lowercase search text for literal substring matching. Do not
alter stored identities, displayed values or link state. User text is never a
regular expression or markup. Unrelated type documentation is not field prose.

Optional means exact local min string `0`; Repeated means unbounded max or exact
decimal max > 1 without Number conversion. Both filters require intersection.
`0..0` remains Optional and Not occurring, not Repeated. Exact bounds and
nillability are separate. Locally min=1 is not labeled globally mandatory.
Existing details explain optional ancestors and choice context.

Results show member, full reference/owner identity, exact local cardinality,
badges, matching modes and readable representative breadcrumb. Status is a
polite live region; native controls and reveal buttons are keyboard-operable.
Search/query/filter/page actions do not write history or the URL contract.

## Work, cache and limits

One shared QName lookup and deterministically sorted segment lookup is built
per page; no per-message eager indexes. Keep only the current root index and
current filtered result array. Same-root selected-path changes reuse it.
Root changes and fragmentless reset discard index/query state. Index jobs use a
root generation; queries/results use a search generation. Obsolete jobs cannot
publish. Pagehide invalidates work; persisted pageshow restarts as needed.

Finite limits: query **256 UTF-16 code units**, at most **100,000 queued work
nodes**, **250 nodes/edges per timer-yield batch**, and **50 result rows** per
page. Pages replace, rather than accumulate, DOM rows; results scroll within a
bounded panel. Index and query yields use actual `setTimeout` tasks, not only
resolved-promise microtasks. Large member/segment fanout also yields. A work cap
is explicitly Limited, with an incomplete lower-bound match count, never a
complete/no-match claim. Invalid/cancelled work has no complete count.

Let D be declarations, S exported segments and W bounded reached work nodes.
Shared setup: O(D + sum s log s) time and O(D+S) lookup memory. Index: O(W)
node/edge traversal, plus identity/string costs, with O(W) queue, visited,
entry and parent-witness memory. Parent links avoid cloning whole paths/subtrees.
Queries are O(F+searched text) per reached field set F and O(F) match references.
Materializing one page's paths costs their total path lengths; long shared
witness tails can therefore cost more than 50 operations, although rows remain
bounded. Retained witnesses and concurrent cancelling jobs are bounded by the
work cap per job; this is not a hard byte-memory sandbox or performance guarantee.

Search coverage is separate from view depth **24**, materialized rows **2,000**,
and fragment **16,384**. Long-path results remain searchable and explicitly
explain that reveal exceeds the explorer limit, with declaring reference and
explicit Explore declaring type actions.

Reveal checks generation/current fragment root, normalizes structured state,
then validates every step with the existing specs/navigation model. Preflight
counts all siblings materialized by every expanded ancestor group, not merely
the selected path. Failure leaves rows, selection, details, link and history
intact. Success rebuilds the minimal root/path view, writes at most one normal
history entry, selects the intended field, updates details/share and focuses
and scrolls to its selection button. Unrelated expansions are not preserved.

## Qualification commands and initial evidence

Rust **1.95.0**: fmt, schema-docs tests (8 renderer + 2 exporter), CLI docs tests
(2), affected all-target checks and warnings-denied Clippy passed. Node
**20.19.2** built-in tests: **6** passing hand-reviewed tests, including an
independent exhaustive simple-path oracle, shorter alternate witnesses,
declaration permutations, owner/namespace/index distinctions, recursive graphs,
alias/list/Choice, independent modes, exact bounds, limits and cancellation.
Python **3.13.5**: **3** validator tests passed. JS syntax, shell syntax and
`git diff --check` passed.

Browser tooling reused read-only from Task 080's locked installation:
Playwright **1.61.1**, Chromium **149.0.7827.55**, Firefox **151.0**. No browser
installation or sibling output was modified. The Task 081 fixture suite forces
batch size 1 and real timer tasks; wide/deep fixtures use 25. It checks collapsed
search, all modes, filters, deduplication/owners, reveal identity/reference,
keyboard focus, share/reload/history, hostile text, pagination, explicit query
generation replacement and pending reset. Wide-root reveal (>2,000 sibling
rows) fails atomically; a 30-step definition is searchable but not revealable.
All four Chromium/Firefox × prefix-HTTP/file fixture cases passed before the
final additional root-switch assertion; final-head evidence is recorded below.

The unchanged Task 080A fixture suite passed all **4** combinations, including
**11 accepted + 31 rejected** state cases per combination, property-order
equivalence, reload/copy and history A–H/empty reset. Tests remain opt-in/local;
hosted Rust checks do not execute them.

A pinned production build using this worktree's binary and verified public
tags completed and validated **11,146 files**, **96,857,502 bytes** (intermediate
assets). Public inputs were copied into task-owned cache before the acquisition
script removed its own temporary checkout. Current-asset regeneration uses
these verified inputs, identical relative schema roots and production CLI.
Intermediate publication comparison to Task 080A: two added search assets;
only explorer HTML/JS/CSS differ for both releases; every graph and legacy
reference file is byte-identical. No generated publications or evidence committed.

Independent real-graph closure and step-by-step path validation measured:

| Release/root | Reachable named types | Definitions | Work nodes | Max steps | Optional ∩ Repeated |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2.5 PositionReport | 56 | 103 | 127 | 10 | 15 |
| 2.6 PositionReport | 56 | 103 | 127 | 10 | 15 |
| 2.5 OrderOfBattle | 49 | 71 | 95 | 6 | 17 |
| 2.6 OrderOfBattle | 50 | 71 | 96 | 6 | 17 |
| 2.5 EntityPositionType | 98 | 205 | 282 | 18 | 1 |
| 2.6 EntityPositionType | 98 | 205 | 282 | 18 | 1 |

Graph sizes unchanged: 2.5 **11,115,091 bytes**, 5,557 types, 722 messages,
13,160 declared members; 2.6 **11,142,756 bytes**, 5,570 types, 725 messages,
13,198 members. No measured witness crosses depth/row limits for these roots.
Node initial index times were 9–23 ms (timer yielding); these are single-machine
observations, not guarantees. Repeated indexing produced identical identities
and witnesses. Four mode/filter queries measured 13–33 ms including the repeat
index in that timer interval; not presented as isolated query latency.
Browser forced-batch index+query measurements and visible counts are in task-owned
logs. All broad queries show at most 50 rows. Real name `Position`, type `Type`,
documentation `the` counts were independently exercised, not prefilled.

## Review gate

Final-byte qualification is in progress. Do not interpret intermediate successes
as READY_FOR_INDEPENDENT_REVIEW. Final exact-head parity, full current-artifact
matrix, publication comparison and outstanding gates must be recorded before
readiness. No merge, deploy, manual workflow dispatch or Task 082.

The final fixture search suite passed all **4** cases twice, including explicit
pending-root switching, latest-query generation, pending fragmentless reset,
same-name namespace and inherited/local owner distinctions, and wide/deep atomic
reveal rejection. The existing fixture navigation suite was repeated on those
same current bytes: **4** cases passed with the unchanged 11/31 corpus and
history checks. Two current synthetic productions compare byte-identically.

An intermediate-current production search matrix passed **8** cases (**24** root
exercises), with independently chosen nested Classification under
SecurityInformationType and FixedPoint under FixedPositionType. Inherited
SecurityInformation reveals and their actual owner/reference/share identities
were checked for both message roots. Existing release navigation passed **8**
cases. These runs preceded only a narrow replacement of the duplicated view
Repeated predicate with the already-tested pure classifier; final bytes are being
regenerated and the complete matrix rerun rather than treating that as equivalent
head evidence.

Desktop/narrow fixture and release screenshots were inspected. Explicit desktop
grid placement retains the navigator/structure/details columns; the separate
full-width search panel is below them. Narrow controls, full identities,
breadcrumbs, count and action links wrap within the viewport. Results have a
bounded scroll area; existing structural horizontal scrolling remains intact.
No panel overlap or external requests were observed in passed matrices.

Latest fetched main remains the starting SHA; PR #77 remains `07313a9`, with
**no changed-file overlap**. Original, Task 077 and Task 080 worktrees remain
clean. All outputs live under this worktree's `target/task081-*`; only the task's
own Python bytecode was removed. Shared caches, browser installs, worktree
registrations and previous qualification evidence were not cleaned or changed.

## Final publication bytes

Both release outputs were generated again with the current binary and preserved
verified inputs. All four explorer assets per release are source-identical.
Full validator: **11,146 files, 96,859,722 bytes**, all links pass; per-release
file counts **5,566 / 5,579**. Compared to corrected Task 080A output:

- Add `2.5/assets/explorer-search.js` and `2.6/assets/explorer-search.js`
  (**6,679 bytes** each).
- Each explorer HTML: 1,689 → **2,907 bytes** (local controls and script order).
- Each explorer JS: 17,170 → **24,174 bytes** (search UI and atomic preflight).
- Each explorer CSS: 1,991 → **2,465 bytes** (panel/grid/narrow layout).
- No removed files; all other publication bytes, including both graphs and
  every legacy reference page, unchanged.

Current pure module SHA-256:
`dfa294b4e53caa05863cb712809ba85ace023fb73665bf09f1ea857f48c1434d`.
Current explorer JS SHA-256:
`cc8c343c9ff43c83fc6ad3408ccab94745cda6a244806c43af5539c0dd057b6d`.
The corrected timing harness measures four queries separately from repeat
indexing: **4–5 ms** total across all six measured roots; indexing **9–22 ms**.
The independent closure, navigation path checks, and deterministic repeat all
passed again. Browser tests deliberately lower batch size to 25 on real graphs,
so their wall times include additional timer yields and UI automation overhead.

Final search mode counts (`Position` name / `Type` immediate type / `the` field
documentation), 2.5 then 2.6:
PositionReport **3/76/76**, **3/78/76**; OrderOfBattle **0/41/48**, **0/43/48**;
EntityPositionType **2/142/159** in both. Broad query visible rows **50**;
synthetic fixture **17**, with Optional ∩ Repeated **1**. Neither count nor timing
is extrapolated to all possible roots or machines.

Final exact-asset production search matrix: **8/8 cases passed**, **24 root
exercises and 24 measured nested reveals**, Chromium/Firefox × both releases ×
project-prefix HTTP/ordinary file with normal security. Current root/index and
query-generation completion conditions pass; no hostile markup, external
requests or uncaught browser errors. Test annotations wrap the pure API only to
lower batch sizes and count timer scheduling; traversal/query implementation is
the same shipped asset. Fixture search and Task 080A suites also pass on current
bytes, repeated generation is identical, and independent real measurements
were repeated successfully.

Changed files (all task-local implementation/test/report boundaries):
`crates/cli/tests/docs.rs`; `crates/schema-docs/src/explorer.css`,
`explorer.html`, `explorer.js`, `explorer-search.js`, `lib.rs`;
`crates/schema-docs/tests/browser.rs`; `scripts/test_validate_uci_pages.py`,
`scripts/validate-uci-pages.py`; `tests/task080-browser/field-search.cjs`,
`measure-search.cjs`, `search.test.cjs`; this report. Historical Task 080 reports,
navigation corpus and harness sources are unchanged.

Reproduce from repository root: `node --test
tests/task080-browser/search.test.cjs`; generate the IR fixture with
`python3 tests/task080-browser/generate-fixture.py target/task081-fixture`;
run `node tests/task080-browser/field-search.cjs` with `TASK081_ARTIFACT` and
`TASK081_EVIDENCE` absolute paths. Set `TASK081_RELEASES=1` for a two-release
artifact. `node tests/task080-browser/measure-search.cjs <absolute-pages-path>`
executes closure/path/determinism checks without a browser. Use the existing
locked package installation (`npm ci` in the browser-test directory) and normal
Playwright Chromium/Firefox installs; this task reused the sibling install via
`NODE_PATH` without altering it. Existing suites retain `TASK080_*` variables.

Final Task 080A real-release navigation suite: **8/8 cases passed** on those
same source-identical bytes, including inherited-owner property-order
equivalence, share/reload/restore, Back/Forward and fragmentless reset. Combined
final release qualification exited **0**. Final focused Rust/Node/Python,
format/syntax and diff checks passed; no outstanding local implementation or
qualification blocker. **READY_FOR_INDEPENDENT_REVIEW**. Hosted status remains
separate and is queried once after opening the normal non-draft PR, with
auto-merge disabled. Commit/head parity and PR URL are supplied in the final
handoff, not fabricated into this pre-commit report. Stop for independent review.