# ADR-0001: Use a Language-Neutral Semantic Schema IR

- Status: Accepted for bootstrap
- Date: 2026-09-16

## Context

The project intends to generate equivalent UCI/OMS APIs for Ada/SPARK, Rust, C++, and potentially Python.

A naive architecture would let each backend parse and interpret XSD independently. That duplicates the hardest semantic work and creates a high risk that languages disagree about namespace resolution, inheritance, cardinality, restrictions, `choice`, and anonymous types.

## Decision

Introduce a versioned, language-neutral **semantic schema IR** between the XSD frontend and all language backends.

The frontend owns:

- XSD syntax;
- import/include resolution;
- QName resolution;
- normalization;
- OMS/UCI-specific classification;
- invariant checking.

Backends own only language representation and emission decisions.

## Consequences

### Positive

- one interpretation of the UCI schema;
- easier cross-language equivalence testing;
- simpler language backends;
- easier future schema diff/documentation tools;
- better isolation for SPARK-oriented generation;
- explicit unsupported-feature diagnostics.

### Negative

- an additional internal model must be designed and versioned;
- the IR must be rich enough not to lose semantics needed by future backends;
- frontend work happens earlier instead of being hidden inside individual generators.

## Rejected alternatives

### Backend-specific XSD parsers

Rejected due to duplication and semantic drift.

### DDS IDL as the intermediate model

Rejected because it would introduce another external type system with semantics not identical to UCI XSD. The IR is an internal compiler model, not a new application-level IDL.

### Raw XML DOM as the shared model

Rejected because it leaves XSD semantic interpretation inside every backend.
