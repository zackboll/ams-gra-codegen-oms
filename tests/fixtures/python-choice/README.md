# PY-015 Choice qualification

This is a synthetic OAM service, not a complete UCI contract.

## Public model API

Each admitted named Choice is a frozen, slotted dataclass with required
`alternative: str` and `value` constructor arguments and stable
`__schema_name__ = (namespace, local_name)` metadata. It has no default selection
and stores no inactive payloads. Explicit static factories use Python's existing
case-preserving schema naming policy:

```python
owner = Owner.GovernmentIdentifier(Country.USA)
special = Owner.NATO_SpecialWord(Words("NATO:EXAMPLE"))
text = Selection.Text("same payload type")
other = Selection.OtherText("same payload type")
keyword = Selection.schema_class("wire key remains class")
```

The equivalent direct constructor `Selection("Text", "value")` is checked;
`Selection("Number", "value")` is rejected. Tags are emitted factory identifiers,
never inferred from payload types. Factories return the exact generated class,
including when accessed through a subclass. Constructors, nested Records and
immutable repeated tuples reject subclasses and revalidate checked carriers.
Validation does not use Python assertions.

The OMS mapping is explicitly generated per declaration. Only the selected
alternative's exact `FieldDecl::wire_name()` is encoded. OAM keys are bare;
other qualified keys use Clark notation. Tags are not JSON discriminators.
Record cardinality wraps whole Choice occurrences: repetitions are tuples in
models and arrays of single-member objects on the wire.

Only concrete named Choices admitted by the shared effective-alternative
projection are supported. Alternatives must be singular, required, non-nillable
and supported by the existing Python reference/domain machinery. Lists,
polymorphism, recursive/unresolved dependencies, unsupported ancestry/facets,
name collisions and unqualified codec member identities fail closed.

## Executable qualification

The CLI target `python_choice` executes `probe.py` in both worlds, both import
styles, normal/optimized Python, and model-only/codec-enabled generation.
The probe contains independently authored JSON literals, exact-type,
malformed-input and mutation controls.

The source-discovered Fast, UCI and Sleet tests use the existing runner lanes.
`uci_probe.py` tests original declarations extracted from hash-verified pinned
UCI roots/includes; its surrounding Record is deliberately synthetic. Passing
that probe is not complete-service readiness or authentic transport proof.

The task's measurement driver records 40 actual selected-service checks and
generations per source, including fresh/pre-existing output atomicity on
failure. It does not assume readiness. The task's parity driver compares
Choice-free artifacts against the independently built verified-main generator,
including the temporal and Binary regressions.