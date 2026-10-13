# Task 085 — structured service-contract schema impact

Starting fetched main: `e5ff70ce20867bbe7a4a3f67a3f440802c06746f`.
Branch: `feature/085-service-impact-json`. Isolated worktree:
`/home/zboll/git/ams-gra-codegen-oms-task085`. Builds and evidence: `/tmp/task085`.
Task 083 is merged. No unmerged PR #84 changes are incorporated.

This JSON is intended for external tools and future service-impact visualization.
It is not published to GitHub Pages in this task.

## CLI

```sh
ams-gra-codegen-oms service-impact --contract SERVICE.yaml \
  --before OLD.xsd --after NEW.xsd --format json
ams-gra-codegen-oms service-impact --contract SERVICE.yaml \
  --before OLD.xsd --after NEW.xsd \
  --before-extension private=OLD_EXTENSION.xsd \
  --after-extension private=NEW_EXTENSION.xsd --format json
```

Default remains text; explicit text/TSV retain Task 079 bytes. JSON is accepted
only by service-impact and schema-impact. Duplicate format, unsupported formats,
missing required options, and invalid extension mapping sets exit 2. Each side
requires its own complete mappings, composed in authored contract order, never
with AFTER fallback to BEFORE. Loading, validation, resolution, serialization,
and writer failures exit 1. Completed findings (including MISSING) exit 0.
The complete report is serialized before stdout is written. Invalid input emits
no JSON; a physical writer failure can leave a prefix at its destination.

## Frozen service-impact-json-v1 structure

One compact UTF-8 JSON object followed by exactly one LF. All fields are required;
key order is the order listed here, enforced by serde structs. No timestamps,
checkout/input paths, random identifiers, descriptions or source documents are
included. Strings retain authored values without version normalization.
Every QName is exactly `[namespace_uri, local_name]`, two strings.

Top-level keys, in order:

1. `format`: string `service-impact-json-v1`.
2. `scope`: string `STATIC_NAMED_REFERENCES`.
3. `selectionBasis`: string `BEFORE_CONTRACT`.
4. `compatibility`: string `NOT_ASSESSED`.
5. `limitations`: string, exactly the existing Task 079 limitation statement.
6. `contract`: metadata object below.
7. `before`: snapshot object below.
8. `after`: snapshot object below.
9. `counts`: count object below.
10. `occurrences`: ordered OMS occurrence object array.
11. `selectedMessages`: first-use unique-message object array.
12. `directMessageChanges`: existing direct-change object array.
13. `emptySelection`: boolean, no baseline-selected OMS messages.
14. `emptyImpact`: boolean, no direct selected changes and neither snapshot has
    selected static exposures. Independent of emptySelection; not compatibility.

Contract keys: `contractVersion`, `serviceName`, `serviceVersion`, `serviceKind`,
`omsVersion`, `uciSchemaVersion` (strings), `amsGraVersion` (string or null),
`uciExtensionIds` (ordered string array). Kind uses portable service/subsystem/
isolator spelling. Logical UCI `2.5` does not imply root XSD `002.5.0`.
Absent optional metadata is null, authored empty is `""`, no extensions is `[]`.

Snapshot keys: `schemaVersion` (string or null), `wholeSchemaTypes`,
`wholeSchemaMessages`, `exposedSelectedUniqueMessages`, `selectedMessageTypePairs`
(nonnegative integers), `exposures` (array). Inventories are whole-schema, not
selected inventories. Exposures retain Task 078 order by full message QName,
seed QName and witness. Each object has `message` (QName), `seed` (QName),
`seedChange` (ADDED/REMOVED/CHANGED string), `witness` (ordered QName array).
Witnesses start at the named payload and end at the changed seed; the
message-to-payload edge is separate. Each uses only its own snapshot's edges,
retaining shortest paths and lexicographic full-QName ties. No mixed path or
reverse inheritance/closed-sum descendant expansion is introduced.

Count keys (all nonnegative integers): `totalExchangeOccurrences`,
`omsExchangeOccurrences`, `excludedNonOmsOccurrences`, `selectedUniqueMessages`,
`selectedPresentInAfter`, `selectedMissingInAfter`,
`directChangedSelectedUniqueMessages`. Repeated exchanges count repeatedly;
selected identities and exposure pairs do not. Non-OMS exchanges are excluded,
not declared unaffected or compatible.

Occurrence keys: `functionId`, `exchangeId`, `direction`, `mandate`, `topic`
(strings), `subscriptionGroup` (string or null), `message` (QName). Order is
baseline function/exchange author order, only resolved OMS exchanges. Direction
and mandate use existing portable labels. Missing, empty and nonempty groups
remain distinct; no group is inferred.

Selected-message keys: `message` (QName), `status` (SAME_PAYLOAD/CHANGED_PAYLOAD/
MISSING string), `beforePayload` (reference), `afterPayload` (reference or null).
Selection resolves only against BEFORE. AFTER lookup requires the complete same
QName; a different-namespace same-local-name message is not a replacement.
New unselected messages are not service additions. SAME_PAYLOAD means only equal
direct references, not equal nested definitions.

Direct-change keys: `message` (QName), `change` (existing Task 069 label),
`before`, `after` (reference or null). Existing typed Change order is preserved.
Unexpected manually supplied non-reference payload values fail explicitly.

References reuse Task 083's representation and key order:
named: `kind` = named, `qname` (QName), `binaryEncoding` = null;
primitive: `kind` = primitive, `primitive` (existing PrimitiveKind label),
`binaryEncoding` (null/HexBinary/Base64Binary). Variant-inapplicable keys are
omitted. Binary lexical provenance is retained, not guessed. Missing references
are null, never empty objects. Primitive labels: Boolean, SignedInteger,
UnsignedInteger, Decimal, Float32, Float64, String, Binary, DateTime, Time,
Duration.

## Determinism, scope and limitations

Analysis reuses ServiceImpact::analyze/analyze_many, ServicePlan, SelectedStatus,
SchemaImpact, Change and DependencyExposure. No second resolver, comparator,
traversal or compatibility algorithm exists. Serialization repeats identically;
schema declaration permutations do not change bytes. Authored contract order
does remain meaningful. JSON escaping retains controls, Unicode and hostile
strings without interpreting script-like text. Input paths are not report fields.

No backward/forward/wire compatibility, runtime interoperability, safe deployment
or upgrade, generated support completeness, backend renderability, service
readiness or open-extension completeness is claimed. There is no generated
support or closed-sum descendant analysis. Existing Task 079 limitations remain.
Task 083's helpers are exposed crate-wide only; its format, key order and frozen
JSON reports are unchanged. Its historical service-impact JSON rejection test
must be narrowed to schema-diff because service-impact now accepts JSON.

## Qualification

Independent synthetic expectations and CLI JSON goldens cover inventories,
selection, occurrences, exclusions, statuses, references and complete witnesses.
Existing Task 079 text/TSV and Task 083 JSON goldens remain frozen. An independent
TSV oracle checks occurrence and exposure/witness records against parsed JSON.

The original opt-in `task079_pinned_service_impact` is extended, loading each
verified closure once. It retains both root SHA256 pins and SecurityMarkings pins
and compares unchanged `uci-summary.tsv`. PositionReport has 1 occurrence,
1 unique selected and 3 pairs per side; repeated PositionReport + OrderOfBattle
has 3 occurrences, 2 unique and 7 pairs per side. Complete witnesses and the
representative payload -> MessageType -> SecurityInformationType paths are
checked separately for BEFORE/AFTER. Qualification results are recorded below
after execution, not inferred from cache availability.

### Executed local results

Rust 1.95.0 with locked dependencies:

```sh
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
  -p ams-gra-oms-schema-diff -p ams-gra-oms-codegen-core
cargo +1.95.0 check --workspace --all-targets --locked
cargo +1.95.0 clippy --workspace --all-targets --locked -- -D warnings
git diff --check
```

All pass. Package campaign: **815 passed, 0 failed, 6 existing opt-in tests
ignored**. Service-impact integration: 14 passed; schema-impact integration:
12 passed. Both new service JSON goldens are tested through the actual CLI.
All four existing service text/TSV fixtures, Task 083 JSON goldens and historical
UCI evidence remain byte-identical to starting main. Task 083 helper changes
are visibility-only; exact golden assertions prove its output byte parity.

The original release-mode `task079_pinned_service_impact` was explicitly
executed against cached UCI 2.5/2.6 closures: **1 passed, 0 failed**, with the
preparation entry point filtered out. Root and SecurityMarkings hash checks
passed. Execution excluding compilation took 12.34 seconds. It verified exact
selected QNames, statuses, occurrence order, inventories, 3/7 separate snapshot
pairs, every complete JSON witness against independent TSV records and validated
snapshot edges, representative SecurityInformationType witnesses, repeat
deduplication, version distinctions, repeated JSON bytes and unchanged frozen
historical evidence. No full real-UCI Deep/backend/Sleet campaign is claimed.

JSON CLI regressions verify usage 2, execution 1 with no stdout for malformed
schema/ambiguous BEFORE selection, missing extensions, findings 0, independent
extension mappings and original identifier order, and broken-writer execution 1.
Schema-diff still rejects JSON; existing schema-impact JSON succeeds.
Synthetic tests cover primitive/named payloads and all three Binary provenance
states, cycles/shortest ties, optional/repeated dependencies, abstract descendant
exclusion, namespace replacement, unselected additions, repeated/mixed exchanges,
null/empty/nonempty groups, hostile values, empty valid selection and determinism.

No dependency, manifest, lockfile, workflow, core/resolver, backend/runtime,
generated output or Pages file changes. Actual PR #84 changed-file overlap is
zero. The sole additional test file beyond the proposed boundary is
`crates/cli/tests/schema_impact.rs`: its obsolete service-impact rejection is
removed while schema-diff rejection and all Task 083 qualification remain.
Hosted CI and final head parity are reported separately in the PR handoff.