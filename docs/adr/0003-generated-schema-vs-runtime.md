# ADR-0003: Separate Generated Schema Bindings from Handwritten LA-CAL Runtimes

- Status: Accepted for bootstrap
- Date: 2026-09-16

## Context

Two kinds of logic change at very different rates:

- UCI schema-specific types and constraints change with schema revisions.
- WebSocket/OWP connection machinery changes with runtime/protocol behavior.

Generating both together would produce large unstable outputs and duplicate complex runtime code across thousands of message types.

## Decision

Generate schema-specific source that depends on a small stable runtime API for each language.

```text
generated Ada UCI bindings  -> oms_ada runtime  -> Sleet
generated Rust UCI bindings -> oms_rust runtime -> Sleet
generated C++ UCI bindings  -> oms_cpp runtime  -> Sleet
```

The exact runtime repository/package names may evolve; the architectural boundary is the important decision.

## Consequences

- schema regeneration does not rewrite WebSocket/OWP implementations;
- runtimes can be fuzzed/tested independently;
- generated code can remain deterministic and mostly declarative;
- each language still requires one high-quality runtime, but not one client implementation per message/service.
