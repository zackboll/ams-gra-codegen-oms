# Task 079 — baseline service-contract schema impact

Immutable starting `origin/main`: `6612046a92928ac89e0f641a323d0b33137ed36d`
(includes Task 078 head `3e4f4e81fbe1fbaf307a76e7467ace6303be3b38`).
Branch `feature/079-service-schema-impact`; isolated worktree
`/home/zboll/git/ams-gra-codegen-oms-task079`; build/evidence `/tmp/task079`.
Task 077 is neither imported nor required. No core/resolver semantic change,
new dependency, manifest, lockfile, workflow or production guard change.

## Command and typed entry points

```sh
ams-gra-codegen-oms service-impact --contract SERVICE.yaml \
  --before OLD_ROOT.xsd --after NEW_ROOT.xsd
ams-gra-codegen-oms service-impact --contract SERVICE.yaml \
  --before OLD_ROOT.xsd --after NEW_ROOT.xsd \
  --before-extension private=OLD_EXTENSION.xsd \
  --after-extension private=NEW_EXTENSION.xsd --format tsv
```

Text is default; output goes through the existing error-aware stdout writer.
No generation, directory creation or configuration writes. Completed findings,
including removal, exit 0; loading/validation/resolution/output failures exit 1;
usage/extension-set mistakes exit 2. Singular flags cannot repeat. Unknown,
generation/world/output, shared extension and raw overlay flags are rejected.
The command-specific `--help` describes selection, removals and limitations.
Existing command help/output bytes are not rewritten.

`service_impact::ServiceImpact::analyze(&Contract, &SchemaIr, &SchemaIr)` returns
`Result<ServiceImpact, String>`. Contracts must come from the existing validated
portable parser, the same precondition as the existing resolver. Both semantic
schemas are validated. The entry point derives selection and comparison from
those snapshots; it accepts no separately supplied unchecked plan/report.
`analyze_many(&[&Contract], before, after)` is the bounded batch equivalent:
one Task 078 comparison shared across cases, one BEFORE resolution per contract.
It deliberately exposes no generated-support impact/readiness result.

The CLI parses one contract and validates both mapping sets independently with
the unchanged extension helper. Overlay order comes from contract declarations,
not flags. No missing AFTER path falls back to BEFORE. Each side loads once
through the production frontend, retaining same-namespace composition and
duplicate-QName failures. Invalid schema closures cannot masquerade as removals.

## Selection, identity and evidence

Scope is `STATIC_NAMED_REFERENCES`; basis is `BEFORE_CONTRACT`. Resolution is
exactly the existing `resolve_service_plan`, including its binding bookkeeping.
Unknown/ambiguous baseline selections fail with function/exchange context.
Function/exchange order, optional mandates, directions, topics and groups remain
authored, not filtered by a new applicability policy. Every OMS occurrence is
retained; selected messages are deduplicated by full QName in first-use order.

AFTER uses direct full-QName lookup, never local-name re-resolution or binding
verification. Status is `SAME_PAYLOAD`, `CHANGED_PAYLOAD`, or `MISSING`. A namespace
replacement does not replace the baseline identity. Another same-local message
does not redirect selection; the report explicitly does **not** certify that
fresh AFTER planning would be unambiguous. Newly added unselected messages are
not service additions. This compares schemas for one contract, not two contracts.

Task 078 runs once on complete schemas. Its message changes and each side's
exposures are filtered by the baseline set without inventory rewriting. Witnesses
are copied unchanged: message/payload-to-seed direction, snapshot side, full
QNames, shortest paths and existing lexicographic tie-breaking. No per-exchange
traversal and no snapshot-edge union. Optional/repeated references remain edges;
no reverse inheritance, abstract-descendant expansion, backend/storage or unknown
runtime analysis. A generated closed-sum change can have no new static exposure.

Counts distinguish total, OMS and excluded non-OMS occurrences; unique selected,
present/missing AFTER, directly changed selected and exposed selected messages;
and exposure pairs per side. Repeated exchange use never multiplies message or
pair counts. Non-OMS exclusions were not assessed against UCI and carry no
compatibility implication. Zero OMS is successful only after input validation.
Empty findings say “No service-selected static dependency impact found; this is
not a compatibility or readiness verdict.”

Logical standards versions, service name/version/kind, portable contract version
and both literal root versions are retained separately. No invented version
normalization or logical-to-root matching rule.

## Independent service-impact-v1 output contract

Fixed 21-column TSV header, also emitted for empty selections:

```text
record side function_id exchange_id direction mandate topic has_subscription_group subscription_group message_namespace message_local seed_namespace seed_local step type_namespace type_local key has_value value has_after_value after_value
```

Columns above are tab separated. `META` identifies format, scope, basis, service,
standards, extension IDs and root versions, plus explicit limitations. `COUNT`
uses distinct `whole_schema_*` versus service-selected keys. `OCCURRENCE` contains
every authored OMS binding. `SELECTED_STATUS` and `SELECTED_PAYLOAD` give fixed
identity and direct reference on each side (named QName in separate type columns,
or primitive tag/value; binary provenance uses after-value presence/value).
`DIRECT` contains the unchanged Task 069 change label and debug-rendered typed
old/new values with explicit presence flags. `EXPOSURE` identifies seed change
ADDED/REMOVED/CHANGED and payload-to-seed direction; `WITNESS` has one row per
step. `EMPTY_SELECTION` and `EMPTY_IMPACT` retain qualified wording.

Function and exchange IDs and QName components are separate fields, never a
concatenated identity. Presence booleans distinguish None from authored empty
values/groups. TSV escapes backslash, tab, LF and CR as `\\`, `\t`, `\n`, `\r`,
and other controls as `\u{hex}`. Every row has 21 physical columns. Text uses
labeled fields in the same record order with the same control escaping; explicit
present-empty metadata renders `value=`. No timestamps or filesystem paths.
Task 069 and Task 078 rendering code and frozen outputs remain unchanged.

## Ordinary and pinned qualification

Eight registered ordinary CLI integration tests need no UCI, Python, GNAT or
Sleet. Small hand-reviewed fixtures test nested selected/unselected changes,
occurrence order/deduplication, optional/repeated edges, valid removals, namespace
replacement/additional ambiguity, unknown/ambiguous baseline failures, direct
payload changes versus nested changes, separate snapshot paths, abstract
descendant exclusion, mixed/empty selections, independent extension sets and
contract-order composition, invalid inputs/duplicate declarations, logical/root
version separation, exact text/TSV fixtures, hostile controls, presence flags,
repeat determinism, broken writer and actual command argument/error paths.
Independent Task 078 reports are filtered by a hand-specified QName and compared
to selected witnesses/counts. Existing core dependency tests additionally retain
all edge families, cycles, shortest paths and full-QName tie-breaking.

Pinned contracts are explicitly synthetic qualification selections, **not**
authoritative deployment contracts. Both cached roots and SecurityMarkings files
are SHA256 verified against existing Task 078 pins before loading. Preparation
is a separately named ignored test that prints evidence and no qualification
pass marker. The strict test always compares the inspected frozen summary; no
environment bypass exists. Both tests are ignored in ordinary offline runs.

```sh
AMS_GRA_UCI_2_5_ROOT=/absolute/cache/UCI_MessageDefinitions_v2_5_0.xsd \
AMS_GRA_UCI_2_6_ROOT=/absolute/cache/UCI_MessageDefinitions_v2_6_0.xsd \
CARGO_TARGET_DIR=/tmp/task079/target cargo +1.95.0 test --release --locked \
  -p ams-gra-codegen-oms --test service_impact_pinned -- \
  --ignored --exact task079_pinned_service_impact --nocapture
```

Measured UCI 2.5 → 2.6, frozen in `uci-summary.tsv`:

| Case | OMS occurrences | Unique selected | BEFORE exposed/pairs | AFTER exposed/pairs | Direct unique |
|---|---:|---:|---:|---:|---:|
| PositionReport | 1 | 1 | 1 / 3 | 1 / 3 | 0 |
| PositionReport twice + OrderOfBattle | 3 | 2 | 2 / 7 | 2 / 7 | 0 |

No non-OMS exclusions in these real-schema cases. Selected QNames match baseline
planning and selected rows equal the exact Task 078 subset. Every witness edge is
checked against its own snapshot. Representative paths on both sides:
`PositionReportMT -> MessageType -> SecurityInformationType` and
`OrderOfBattleMT -> MessageType -> SecurityInformationType`, all in the OAM
namespace. Repeated PositionReport use has identical message evidence. Logical
`2.5` and root `002.5.0`/`002.6.0` remain visible. Preparation took 14.73 seconds;
initial strict qualification took 13.69 seconds excluding compilation.

Local Rust 1.95 locked qualification includes ordinary feature tests, existing
Task 069/078/core and service-plan/routes regressions, affected all-target check,
warnings-denied Clippy, formatting, unchanged CI split guard and 150 adversarial
checks, and whitespace/lockfile checks. The strict pinned qualification is local
evidence; no hosted pinned campaign or full readiness/backend/Sleet campaign is
claimed. Existing workflow registration runs new ordinary Cargo targets without
workflow edits. Auto-merge stays disabled; independent review is required.