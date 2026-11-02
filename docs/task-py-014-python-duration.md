# PY-014 — Python XML Schema Duration models and OMS JSON codecs

## Source isolation and implementation

Dedicated worktree: `/home/zboll/git/ams-gra-codegen-oms-py-014`; branch:
`feature/py-014-duration`. Fetched origin/main before starting, checked branches,
worktrees and PRs; no previous PY-014 work existed. Starting main is
`c2dcca5027ef294103346a59d46a1eb08a79a0d4`, tree
`efb054162833a1254f06fa476932a75ad8c0ca58`. PY-013 #165 was already merged.
No sibling worktree, historical evidence or CI policy was modified.

Production implementation: commit `56a9fb2309b73ff56e9d5777ec8fc1ef2df71759`,
tree `e24c7dc89053d40392b0f99a8f493a448cd807da`. Subsequent evidence additions
do not change production bytes. The historical Python production inventory is
authenticated by a fixed digest in `scripts/check-py014-uci.py`, not compared
against future live files or dependent on availability of historical Git objects.

* Shared `TemporalProfile::Duration` / `DirectTemporalProfile::Duration` prove
  admission. Names have no semantic role. Every named restriction edge is checked;
  incompatible/missing/cyclic ancestry, aliases and unsupported facets fail closed.
* Named carriers are exact-string, frozen, slotted dataclasses. Direct fields
  are checked strings. Required, optional, bounded/unbounded tuples, effective
  inherited fields and nested Records reuse existing storage mechanisms.
* A single private deterministic scanner collapses only XML SPACE/TAB/LF/CR,
  scans ASCII digits, and enforces component order, fraction placement and actual
  components after P/T. It never accumulates integers, converts seconds, performs
  arithmetic or canonicalizes lexical forms. Negative zero, leading zeros and
  fractional precision are preserved. No ordering is defined.
* Codec Duration is a JSON string. The codec calls the model validator through a
  narrow TypeError/ValueError-to-CodecError boundary. Egress checks exact carrier
  types and revalidates forged/missing storage. Existing strict JSON, Unicode,
  duplicate/unknown member and `$type` rules remain unchanged. Paths retain
  schema member names, Clark qualification and repeated indexes.
* Helpers are demand-gated, including inherited direct fields but excluding
  unused abstract ancestry. Duration-free generated source bytes are unchanged.
* No frontend, IR, shared classifier, external dependency, DateTime parser,
  transport implementation, workflow, lane or PR-smoke selector changes.

## Qualification and evidence

Versioned evidence: `tests/fixtures/python-duration/evidence/v1`.
Task-owned raw attempts, binaries and generated artifacts:
`/home/zboll/py014-evidence/v1`. Primary builds:
`/home/zboll/py014-build`; isolated corrected release AFTER build:
`/home/zboll/py014-build/after`.

`failed-attempts.txt` explicitly distinguishes failed/superseded attempts,
including Cargo target reuse that initially selected the baseline release binary
as AFTER. That attempt was stopped, not treated as qualification. The corrected
AFTER executable has SHA-256
`95d6f46448ff83c174c58c51ca97c1bac344f96d3e626edd748636057f813203`;
an actual Duration service-check READY probe precedes measurement.

CPython 3.11.17 and 3.13.16 are used, normal and optimized. The focused CLI
suite executes model-only and codec-enabled generation in closed/open worlds,
standalone/package imports, deterministic regeneration, all 100 immutable Task
057 vectors (33 valid, 67 invalid), an independent test-only regex oracle and
combinatorial controls, exact types including string subclasses, malformed Unicode,
8,192-digit components/fractions, forged storage, inherited/nested/repeated fields,
strict JSON, indexed and non-OAM Clark paths, and atomic unsupported generation
with missing and pre-existing output.

The requested fmt, locked workspace all-target check, focused warning-denied
Clippy, Python backend and CLI regression targets (including merged Binary),
local Python PR smoke and diff checks passed. Duration-free parity passed all
44 selections / 110 generated Python files per interpreter/policy. This includes
Binary, DateTime, Float64, VisibleAscii, NATO, header and unbounded repetition
fixtures. Source-bound Fast/UCI/Sleet receipts passed all 12 combinations with
10/6/6 registered tests respectively and zero skips. These receipts identify
their actual tested source commit/tree, not the later evidence-only commit.

## Authentic UCI service results

The source-bound campaign uses unchanged authenticated UCI roots/includes at
2.5 `093610b7753944059360d3236770ab446d039556` and
2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`.
It measures five services × two releases × two worlds × model/API with or without
codec selection: **40 BEFORE and 40 AFTER**. Each captures exact selected QNames,
closure/contract hashes, source inventories and executable hashes, exit/stdout/
stderr, generation outcome and artifact hashes. Unsupported codecs are explicitly
not measured when model/API is NOT READY. Separate pinned pre-existing-output
checks preserve sentinel bytes. No source schema is simplified for these results.

| Release | Service | BEFORE first blocker | AFTER first blocker |
|---|---|---|---|
| 2.5 | Log | `LogMDT.ServiceUpTime`: direct Duration | `FGI_SourceOpenChoiceType`: Choice/List |
| 2.6 | Log | `DurationType`: named Duration | `FGI_SourceOpenChoiceType`: Choice/List |
| both | PositionReport | `FGI_SourceOpenChoiceType`: Choice/List | unchanged |
| both | RDMA_InitializeSetup | `IPv4_AddressType`: constraints | unchanged |
| both | SubsystemStream | `FGI_SourceOpenChoiceType`: Choice/List | unchanged |
| both | SurvivabilityRiskLevel | `DurationType`: named Duration | `FGI_SourceOpenChoiceType`: Choice/List |

All complete services remain model/API NOT READY; there is no full-service codec
or transport claim. Isolated unchanged `LogMDT.ServiceUpTime` and `DurationType`
declarations from both releases execute separately through generated code under
both worlds/import styles/interpreters/policies; this is declaration qualification,
not full-schema Log interoperability.

## Pinned Sleet

Unchanged clean Sleet checkout at
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`; existing dependencies including
websockets 17.2 unchanged. Synthetic OAM DurationNotice exercises generated
models/API/codecs through the existing Python LA-CAL runtime.

Both interpreters/policies pass named/direct, required/optional, bounded/unbounded
repetition, inherited/nested storage, XML whitespace, fractional trailing zeros,
negative zero, typed publish/subscribe, recovery, unsubscribe, clean teardown and
no leaked asynchronous tasks. Each run observes six malformed Duration payloads
reaching the instrumented generated decoder and being rejected there; the
unknown-member payload is rejected server-side without a decoder call. Broker
logs, decoder inputs and generated artifact hashes are retained and versioned.

Since complete authentic Log is NOT READY, conditional full-schema Log golden
transport qualification does not apply. Choice/List, Time, polymorphic/abstract
values, constrained Duration and unsupported aliases remain unsupported.

## Delivery

Final exact-head hosted smoke and source identity are reported in the PR handoff;
this document does not claim a future or self-referential final commit test.
Auto-merge remains disabled; no approval or merge is performed.