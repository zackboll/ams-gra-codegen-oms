# Task 071 — Contract-derived OMS route inventory and Sleet preview

Read-only developer/deployment tooling, not backend generation, readiness,
portable contract validation replacement, or a schema/contract/runtime change.

```sh
ams-gra-codegen-oms service-routes --schema ROOT.xsd --contract SERVICE.yaml \
  --extension ID=PATH --format tsv
ams-gra-codegen-oms service-routes --schema ROOT.xsd --contract SERVICE.yaml \
  --format sleet-toml --service-id sensor-service \
  --service-uuid 550e8400-e29b-41d4-a716-446655440071 > service.toml
```

Extensions are repeatable and optional when none are declared. TSV is default.
Service identity options are forbidden for TSV and required for Sleet TOML.
Identity is caller-supplied: never inferred from human `service.name`, and no
random UUID. The existing UUID library parses UUIDs; output is canonical
hyphenated UUID text. Unsupported: `--world`, `--language`, `--with-codec`,
`--output`. Output is stdout only, never an edit to existing Sleet files.
Exit codes: 0 success (including empty), 1 resolution/render failure,
2 invalid usage (including identity and extension mappings).
All validation finishes before stdout is written.

## Authority and model

The independent `ams-gra-oms-service-routes` crate exposes
`build_route_manifest`, `render_route_tsv`, `render_sleet_toml`.
`RouteManifest::occurrences()` retains each authored OMS occurrence;
`topics()` derives unique exact topic/QName sets in first-occurrence order;
`excluded_non_oms_occurrences()` counts non-OMS omissions. The CLI reuses
`load_service_inputs` for portable contract parsing, exact extension mappings,
contract-order overlays, IR validation and `ServicePlan` resolution.
No new resolver, fuzzy lookup, overlay policy, Capability inference or grouping.

Functions/exchanges are traversed in contract order, **not** the deduplicated
`selected_messages()` closure list. Shared
`ServiceApiOmsOperation::for_direction` maps Input to Subscribe and Output to
Publish; mandate/timing do not participate. Repeated pairs remain multiple
occurrences but one pair per topic in aggregation. One message on two topics
belongs on both. No case or punctuation normalization.
The four non-OMS kinds lack UCI identity: count and omit them, never fabricate
topics/messages. This is OMS-only, not the complete service interface.

## Stable TSV

Header columns, tab-separated:

```text
function_id exchange_id direction operation mandate topic message_namespace message_local_name has_subscription_group subscription_group
```

Namespace/local name preserve the exact QName as separate components. Each
exchange occupies one physical line. Escape backslash, tab, newline and CR as
`\\`, `\t`, `\n`, `\r`; other controls as `\u{lowercase hex code point}`.
`has_subscription_group` is true/false, distinguishing absent from present-empty
text. Zero routes emit only the header. No filesystem paths or timestamps.

## Pinned Sleet surface and limitations

Authority inspected: `open-arsenal/ams-gra-hello-world-sk-infra-sleet` at
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`:
`config/services.d/service.example.toml`, `ir-search-and-track.toml`,
`docs/configuration.md`, `sleet/src/config.rs`, `sleet/src/server.rs` and
`sleet-types/src/owp.rs`.

Emit only `service_id`, `service_uuid`, `allowed_topics`, and
`[[topic_bindings]]` with `topic` and `allowed_messages`. Every distinct allowed
topic gets exactly one nonempty binding. This prevents the pinned server's
**default-permissive missing-binding fallback** allowing any schema-valid
message. No duplicate bindings or empty message lists. Empty routes emit
`allowed_topics = []`, no bindings; pinned `load_valid_empty_allowed_topics`
explicitly tests acceptance of that shape.

The exact identifier class is nonempty ASCII `[A-Za-z0-9_.-]+`, including any
of those characters in first position. Validate service ID, every topic and
message. Invalid topics identify function/exchange/topic. No replacements of
spaces, punctuation, Unicode, quotes, backslashes, or controls. Such inputs
fail, rather than being escaped into transport identifiers. Validated TOML
strings need only double-quote delimiters.

Only messages in the shared codegen-core exact OAM namespace
`https://www.vdl.afrl.af.mil/programs/oam` are accepted for TOML. Non-OAM TSV
is supported; non-OAM TOML fails closed with an unsupported-namespace error.
Never strip Clark names to local parts. This respects the pinned Task051
namespace limitation, not a claim of general Sleet compatibility.

**Bindings authorize both PUB and SUB.** Output-only routes also authorize
subscription; input-only routes also authorize publication. Contract direction
is retained in the manifest and not changed. TOML is **not** a complete
direction-specific least-privilege policy. There is no subscription-group
policy in the pinned config: groups remain in TSV, with no claim of validating
all group runtime behavior. No fabricated direction/group/capability fields.

## Executable evidence gates

Fast: `bash scripts/check-task071-fast.sh` verifies exact test registration and
one-test result counts for typed projection/counts, exact TSV/TOML, escaping,
empty routes, first-occurrence ordering, identifiers/namespaces, resolver errors,
private overlays and CLI negative controls. Zero matches fail, command status
is preserved and diagnostics printed. No real UCI or Sleet fetch/build.
Marker: `TASK071 ROUTE MANIFEST FAST: PASSED`.

Deep real-uci reuses its already fetched/hash-verified UCI 2.5 root and existing
PositionReport loop contract: one unique QName, two authored input/output
occurrences, exact topic, one topic/pair, stable TSV and exact OAM TOML.
No backend generation or 722-message campaign is added.
Marker: `TASK071 REAL UCI ROUTES: PASSED`.

Deep real-sleet reuses the existing single pinned checkout/build in
`scripts/run-real-sleet-test.sh`. Production CLI-generated TOML is passed to
the actual server, including an empty service. The integration requires INIT,
MessageA SUB/PUB delivery, schema-valid MessageB SUB/PUB rejection, unauthorized
topic rejection, and valid MessageA delivery afterwards. The synthetic OAM
fixture contains both globals. Non-verbose -ERR events are observed
asynchronously with finite waits. Runtime close joins workers; server RAII
kills/waits for the child. Inherited Task049–053 tests remain unchanged.
Marker: `TASK071 REAL SLEET POLICY: PASSED`.

These are executable gates, not assertions that a skipped/unrun integration
passed. Publication certification must use actual final-head job logs.