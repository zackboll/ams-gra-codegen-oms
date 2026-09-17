# ADR-0002: Preserve the Existing LA-CAL WebSocket/OWP/JSON Boundary

- Status: Accepted for bootstrap
- Date: 2026-09-16

## Context

Code generation could be used as an opportunity to replace the runtime transport or introduce another middleware such as DDS. Doing so would mix two separate decisions: improving application-language ergonomics and changing the mission-system communication architecture.

The public AMS GRA Starter Kit already demonstrates language-agnostic CAL access through Sleet using WebSockets, OWP framing, and UCI JSON.

## Decision

The initial generator will target the existing LA-CAL interface.

Generated clients ultimately emit the same externally visible OWP operations and UCI JSON as handwritten clients.

## Consequences

### Positive

- no Sleet changes required;
- generated and manual clients interoperate;
- existing LA-CAL conformance behavior remains relevant;
- the CAL/ASB abstraction is preserved;
- codegen can be adopted incrementally.

### Negative

- JSON/WebSocket overhead remains;
- codegen does not by itself add DDS-style QoS or binary serialization performance;
- each supported language still needs a small runtime implementation.

## Future option

A different ASB implementation, including a possible DDS-backed transport below the CAL, can be explored separately without forcing application code to change.
