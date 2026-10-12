# Task 078 — static message dependency impact

Immutable starting `origin/main`: `65a8f6c9a7d330ce6c915efcb6cf7ef45bc61658`.
Branch: `feature/078-schema-message-impact`. Isolated worktree:
`/home/zboll/git/ams-gra-codegen-oms-task078`; scratch/build output:
`/tmp/task078`. Task 077 is not imported or required.

## Command and scope

```sh
ams-gra-codegen-oms schema-impact --before OLD_ROOT.xsd --after NEW_ROOT.xsd
ams-gra-codegen-oms schema-impact --before OLD_ROOT.xsd --after NEW_ROOT.xsd \
  --before-overlay OLD_EXTENSION.xsd --after-overlay NEW_EXTENSION.xsd --format tsv
```

Format defaults to text. Each overlay option is repeatable and preserves caller
order independently. Singular options cannot repeat. Unknown options, missing
values, unsupported formats and generation/service options are usage errors (2).
Each side is loaded once through the production frontend. Validation, analysis,
loading and output failures exit 1; completed analysis exits 0 even with findings.
Only stdout is written through the existing error-aware writer. No files or
directories are created, inputs are not altered and source is not generated.

Every report identifies `STATIC_NAMED_REFERENCES`. This is not backward, forward,
wire, runtime or safe-upgrade compatibility. Empty output says **no static
dependency impact found**, not that messages are unaffected or safe.

`compare_schemas()` alone defines changed declarations. Type-category property
changes are deduplicated by full QName. BEFORE uses changed common and removed
types present there; AFTER uses changed common and added types present there.
Snapshots never share edges. Message presence and direct payload changes remain
distinct evidence, even when a message also has dependency exposure. Namespace
identity changes are removal/addition, not inferred renames.

The named payload itself is reachable. Type edges come exclusively from
`direct_named_dependencies()`: immediate named bases, aliases, record fields,
choice alternatives and list items. Optional/repeated members are not suppressed.
Primitive payloads have no named roots. Primitive payload changes are supported
at the typed IR layer; the current UCI frontend requires named global payloads.

No reverse inheritance edges expand abstract values into descendants. Adding an
otherwise unreferenced concrete descendant therefore produces no static message
path to it, even if a closed-sum generated representation gains support. This
does not assert generated support is unchanged. Unknown runtime descendants,
backend storage/renderability, generated-support closure and contract-selected
service readiness are explicitly outside this command.

## Typed API and algorithm

`codegen_core::static_dependency_exposures(&SchemaIr, &BTreeSet<QualifiedName>)`
returns `Result<Vec<DependencyExposure>, DependencyImpactError>`. Each exposure
contains structured `message`, `seed`, and `witness: Vec<QualifiedName>`.
Invalid schemas and undeclared seeds fail, rather than returning empty closure.
The query enforces its validated-model precondition. The CLI's
`schema_impact::SchemaImpact::analyze` validates before invoking the typed diff.

Declaration, deduplicated forward/reverse adjacency and message-root indexes are
built once per query. Reverse BFS per unique seed computes shortest distances
with explicit visited state. Witness reconstruction chooses the smallest full
QName next hop with distance one smaller. This gives lexicographically smallest
shortest paths, independently of source/declaration/member order, preferred
prefixes and hash iteration. Traversal is iterative and cycle-safe; accepted
record-reference cycles and internal graph cycles have separate tests.

For V types, E unique edges, M messages and S unique seeds: index construction
is O((V+E+M) log V); traversal is O(S(V+E) log V), plus message-root scans and
witness/output size. Working storage is O(V+E+M), plus returned witness storage.
No constant-memory or all-path enumeration claim is made.

## Independent v1 output contract

The existing `schema-diff` engine and its v1 report bytes are unchanged.
Text begins with `schema-impact v1`, explicit scope and a compatibility disclaimer.
Both sides include version metadata and declaration counts. Direct changes show
typed old/new values. Exposure lines identify side, message, seed change category
and witness, explaining the separate message-to-payload edge.

TSV always has this fixed eleven-column header, including empty results:

```text
record side message_namespace message_local seed_namespace seed_local change step type_namespace type_local value
```

The displayed spaces above represent tabs. Rows:

* FORMAT: `value=schema-impact-v1`.
* SCOPE: `value=STATIC_NAMED_REFERENCES`.
* SCHEMA: side, `change=version`, value (empty when absent).
* COUNT: side when applicable, metric in change, decimal value. Metrics are
  `types`, `messages`, `exposed_unique_messages`, `exposure_pairs` per side,
  and `direct_unique_messages` across direct changes. Side counts are not
  summed to imply distinct cross-snapshot messages.
* DIRECT: message QName, semantic change label, typed old/new values in value.
* EXPOSURE: side, message/seed QNames, `ADDED|REMOVED|CHANGED` in change,
  and `message payload -> witness step 0` in value.
* WITNESS: side, message/seed QNames, zero-based step and structured type QName.
  Step zero is the named payload; final step is the seed. Direct exposure has
  exactly one step. This avoids ambiguous internal/path-component delimiters.

Metadata/counts precede direct records, then BEFORE and AFTER exposures.
Exposures sort by message and seed full QName; witness steps retain path order.
Every TSV cell escapes backslash, tab, newline and carriage return as `\\`,
`\t`, `\n`, `\r`. Blank unused cells never omit columns. Text QName display
also escapes these characters. Exact changed/empty text and TSV fixtures are
under `tests/fixtures/schema-impact`.

## Qualification

Ordinary offline tests require no UCI, Sleet, Python, GNAT or added dependencies.
Core tests cover all edge families, shortest/tied/diamond paths, duplicate edges,
cycles, optional/repeated references, full QNames, primitive roots, invalid input
and the abstract-descendant exclusion. CLI tests cover direct primitive changes
in IR, additions/removals, multiple changed properties, a false union path,
metadata-only changes, namespace replacement, hostile TSV cells, declaration
permutations, exact reports, overlays, usage, malformed input and broken output.

The opt-in test is ignored in ordinary runs; explicitly requesting it fails for
missing or mismatched roots or SecurityMarkings dependencies. It loads each
hash-verified cached release once, then compares both directions and renders
repeatedly without backend/service/Sleet campaigns:

```sh
AMS_GRA_UCI_2_5_ROOT=/absolute/cache/UCI_MessageDefinitions_v2_5_0.xsd \
AMS_GRA_UCI_2_6_ROOT=/absolute/cache/UCI_MessageDefinitions_v2_6_0.xsd \
CARGO_TARGET_DIR=/tmp/task078/target cargo +1.95.0 test --release --locked \
  -p ams-gra-codegen-oms --test schema_impact_pinned -- \
  --ignored --exact task078_pinned_static_impact --nocapture
```

Measured UCI 2.5 → 2.6 (not prefilled):

| Evidence | BEFORE | AFTER |
|---|---:|---:|
| Exposed unique messages | 722 | 725 |
| Message/type exposure pairs | 3,000 | 3,475 |

There are **3 directly changed unique messages**, all additions; no common
message payload changed. Broad exposure is explained by shared nested/base
dependencies: the unchanged-payload `OrderOfBattle` witness on both sides is
`OrderOfBattleMT -> MessageType -> SecurityInformationType`. Thus all messages
have some static exposure, which is not a compatibility verdict.

Every witness is checked against its own snapshot, and an independent forward
per-message stack closure checks exact membership against reverse BFS results.
Reverse BEFORE equals forward AFTER and vice versa. Added seeds never appear
on BEFORE, removed seeds never on AFTER. Task 069's frozen semantic summary is
checked unchanged. The measured summary and representative witnesses are frozen
in `tests/fixtures/schema-impact/uci-summary.tsv` after inspection. The initial
optimized campaign executed one test in 12.05 seconds (excluding compilation).
No existing Deep wrapper or historical fixture is modified.

Final local qualification used Rust 1.95.0 and locked dependencies: 497 core/
schema-diff tests (including 7 actual semantic-engine tests), 48 CLI library and
focused integration tests (7 impact integration tests and 4 existing diff CLI
controls). New feature tests total 15 ordinary tests: 6 core integration, one
internal cycle, 7 CLI integration and one escaping unit test. The pinned test
executes separately as one ignored/explicitly requested test. Affected all-target
check, warnings-denied Clippy, formatting, diff whitespace checks and the unchanged
CI split guard plus 150 adversarial checks passed. Existing core closure and
generated-support/service-plan binding targets ran in the 497-test total.

Hosted Fast CI will run the normal registered targets; this task does not add
the opt-in pinned campaign to a workflow. Real-input qualification above is local
evidence, not a claim of hosted execution. No workflow is dispatched or modified.