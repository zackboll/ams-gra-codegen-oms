# Contributing

## Architectural rules

1. UCI/OMS XSD remains authoritative; do not invent a parallel public IDL.
2. Raw XSD/XML concepts stop at the frontend/IR boundary.
3. Language backends consume semantic IR only.
4. Unsupported schema constructs fail explicitly; never silently discard semantics.
5. Generated clients must preserve the existing LA-CAL WebSocket/OWP/JSON behavior unless an explicit ADR changes that decision.
6. Keep handwritten protocol runtimes separate from schema-generated source.
7. Every IR feature needs cross-language fixture coverage.
8. Prefer deterministic output and stable ordering.
9. Preserve source provenance in diagnostics and generated artifacts.
10. Do not claim OMS/AMS-GRA compliance solely from successful generation.

## Pull-request expectations

A change to the XSD frontend or IR should normally include:

- a focused XSD fixture;
- expected normalized IR behavior;
- negative/error coverage;
- at least one affected backend test or an explicit reason no backend change is needed.
