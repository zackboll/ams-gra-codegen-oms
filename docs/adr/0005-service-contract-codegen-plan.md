# ADR 0005: Service Contract as a first-class codegen input

## Status

Accepted (Task 030).

## Context

Until now the only authoritative input to this generator was the UCI/OMS XSD
set. That set describes **everything UCI can express**: in UCI 2.5, 5,557
types and 722 messages. A given mission service uses a small fraction of it.

Separately, `zackboll/ams-gra-service-contract` publishes a portable,
versioned, machine-readable Service Contract format describing **what a
particular service actually uses**: its functions, its Capability inventory,
and each function's exchanges with their direction, mandate, topic, and
timing. That project also owns the OMS profile engine and the
completion-assistant tooling that help an author produce such a contract.

Those two inputs answer different questions, and neither can answer the
other's. The XSD knows that `PositionReport` exists, what its payload type is,
and what that payload transitively depends on. It does not know that a given
service subscribes to it on topic `mission.position-report` at 1 Hz. The
contract knows the latter and deliberately does not restate the former.

## Decision

Make a portable Service Contract an actual first-class input, joined to the
normalized schema IR in a language-neutral Resolved Service Plan.

1. **A dedicated crate owns the portable format.**
   `ams-gra-oms-service-contract` owns parsing, the typed Contract IR, and
   portable semantic validation. It depends on nothing else in the workspace
   -- not `xsd-frontend`, not `codegen-core`, not a backend, not the CLI -- so
   the portable format cannot acquire a hidden XSD dependency, and so it can
   stand alone as the first independent non-Python consumer of the format.

2. **The join lives in `codegen-core`, not in a backend or the CLI.** The
   Resolved Service Plan is language-neutral because Ada, Rust, and C++
   service generation must all consume the same plan. Putting resolution in a
   backend would force the other two to reimplement it; putting it in the CLI
   would make it unavailable to library consumers.

3. **The join adds no semantics of its own.** The contract owns service
   interface semantics; the XSD and Schema IR own message and type identity.
   The plan owns only the join. Contract semantics are never pushed into
   `SchemaIr`, and XSD semantics are never pushed into the contract parser.

4. **No profile engine is copied.** `profiles/oms/2.5/profile.yaml` and the
   completion tooling stay upstream. This repository consumes an
   already-authored contract. Validation here only rejects; it never completes,
   infers, or regenerates. In particular there is no Capability inference, no
   function grouping, no topic generation, and no Section 3.3 regeneration.

5. **Resolution is exact or it fails.** Contract message names resolve by
   exact local-name equality. Zero matches fail; more than one fails as
   ambiguous with candidates listed. No fuzzy, case-insensitive, affix, or
   first-match-wins behavior exists, because a wrong silent match would bind a
   service to the wrong wire message.

6. **Extension identifiers are not paths.** A contract's
   `uci_extension_schemas` entries are logical IDs. The CLI requires an
   explicit `--extension ID=PATH` pair per declared ID, with exact set
   matching in both directions, and composes Task 029 overlays in **contract**
   order rather than command-line order.

7. **The plan is world-independent.** `service-plan` takes no `--world`.
   Which messages and types a contract selects is a fact; whether a backend
   can render them under a closed or open world is policy, and belongs to a
   later task.

## Consequences

Positive:

* useful contract-driven generation no longer waits on full-UCI renderability.
  A generator can target only the closure a contract selects;
* the portable format gains an independent implementation, which is itself a
  compatibility test of the format;
* one shared dependency model (`direct_named_dependencies`) now backs
  declaration ordering, coverage closures, and the contract-selected closure,
  so those three cannot drift apart.

Costs and risks:

* the workspace gains `serde`, `yaml_serde`, and `serde_json`. This is
  accepted over shelling out to Python, which would make the generator depend
  on another project's runtime tooling. No Python parser or runtime dependency
  is introduced;
* YAML parsing uses `yaml_serde` 0.10, the maintained fork published by The
  YAML Organization, rather than the deprecated and unmaintained original
  `serde_yaml`. Cargo's package renaming aliases it back to `serde_yaml` in
  this crate's manifest, so the source surface is unchanged. That surface is
  deliberately one call, `serde_yaml::from_str`, on read-only,
  schema-constrained input, which keeps the parsing boundary narrow enough to
  swap again should the need arise;
* the contract's logical UCI version and the XSD root's release string are
  both retained but not compared, so a genuine version mismatch is not yet
  caught here. That is deliberate: see the evidence section of
  `docs/service-contract-integration.md`. Guessing a mapping would be worse
  than deferring one.

## Follow-up: Task 031 backend readiness

This decision is unchanged and remains **Accepted**. Task 031 builds on it
rather than revising it.

Task 030 deliberately stopped at the join, leaving "can a backend render this
selection?" open. Task 031 answers that in a separate `service_readiness` layer
in `codegen-core`, taking `ServicePlan`, `SchemaIr`, `BackendLanguage`, and
`GenerationWorld` and producing a `ServiceBackendReadiness`.

Two boundaries this ADR established were load-bearing there and were kept:

* **The plan stays world- and language-independent.** No `GenerationWorld` or
  `BackendLanguage` field was added to `ServicePlan` or to the Contract IR.
  Readiness is a separate analysis over the plan, and `service-plan` still
  refuses `--world`, `--language`, and `--output`.
* **The contract crate stays independent.** `ams-gra-oms-service-contract`
  acquired no new dependency; readiness lives entirely in `codegen-core`.

Task 031 also declines to introduce a second capability model: it reuses
`CoverageAnalysis`, whose per-declaration renderability computation was
extracted into one snapshot shared by full-schema coverage and selected-service
readiness. Full-schema coverage semantics are unchanged.

The deferred version comparison stays deferred. Task 031 introduces no
normalization, stripping, or comparison of the contract's logical UCI version
against the XSD root's release string.

## Task 032 follow-up

The resolved plan now drives **contract-selected UCI type generation**, through
a language-neutral schema projection:

```text
project_service_generation_schema(&ServicePlan, &SchemaIr, GenerationWorld)
    -> Result<ServiceGenerationProjection, ServiceGenerationError>
```

The projection narrows a full `SchemaIr` to the model one contract selects and
returns it as an ordinary owned `SchemaIr`, which the existing Ada, Rust, and
C++ backends generate from unchanged.

The boundaries this ADR established held again, and were kept:

* **Backend crates still do not parse or understand the contract.** They
  receive a `SchemaIr`, as they always have. No `generate_service()` entry
  point was added anywhere, and no backend crate changed.
* **The plan stays world- and language-independent.** No generation
  information — no `BackendLanguage`, `GenerationWorld`, output path, backend
  name, or generated file name — was added to `ServicePlan`. `GenerationWorld`
  is a projection *argument*, and is not stored in the projected `SchemaIr`
  either.
* **The contract crate stays independent.** `ams-gra-oms-service-contract`
  acquired no new dependency.

One new boundary was drawn. `ServicePlan::selected_type_closure` remains a
*semantic* closure and was deliberately **not** extended to cover generated
representation support. Task 024's closed-sum concrete descendants are tracked
in a separate fixed-point support closure and reported separately, because the
contract selects neither them nor the abstract intermediates retained for their
ancestry. Presenting them as contract selections would make the plan claim
something the contract never said.

Readiness remains the authoritative capability gate: `service-generate` refuses
to invoke a backend, create an output directory, or write a file for a NOT
READY selection, and support expansion is never a route around that verdict.

## Alternatives considered

* **Put contract parsing in `codegen-core`.** Rejected: it would tie the
  portable format to the crate that also knows about XSD-derived IR, blurring
  the authority boundary this ADR exists to protect.
* **Reuse `--overlay` with inferred identifiers.** Rejected: it would require
  guessing a logical ID from a filename, and would silently accept an overlay
  set that does not match what the contract declared.
* **Normalize `002.5.0` to `2.5` and compare.** Rejected: no documented
  mapping exists. See above.
* **Normalize omitted Capabilities to an empty list.** Rejected: it converts
  "the author said nothing" into "the author asserted none", which are
  different claims with different downstream consequences.
