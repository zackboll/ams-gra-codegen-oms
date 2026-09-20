# ADR-0004: Treat UCI Zero-Descendant Abstract Types as Open Extension Points

- Status: Accepted
- Date: 2026-09-19
- Accepted: 2026-09-20 (Task 028)

## Context

Thirteen abstract complex types in the pinned UCI 2.5 and 2.6 schemas have zero
concrete structural descendants: `SourceCommandEXT`, `ConstraintEXT`,
`OpNotificationEXT`, and ten `CommSupport*EXT` types. Task 024 fails closed on
them; Task 026 elides storage for the twelve used in optional `0..1` Record
positions, on an explicit closed-schema assumption.

The thirteenth, `SourceCommandEXT`, is used at
`DisseminationSubplanType.ExtensionCommand` with `minOccurs=0`
`maxOccurs=unbounded`. It is the current generation blocker for all six
UCI × language probes. Lowering it as an always-empty collection would have been
the natural extension of Task 026's reasoning.

## Decision needed

Does "zero known descendants" mean the value is uninhabited (closed world), or
that its concrete forms are supplied outside this schema set (open world)?

The two readings imply opposite lowerings, so this must be settled from evidence
before any representation is chosen.

## Evidence

Authoritative, from the pinned roots (full detail in
`docs/task-027-open-extension-points.md`):

- `SourceCommandEXT` (2.5 line 95638, 2.6 line 95712) is documented as "a point
  of abstract extension to create SourceCommands that can't be documented in the
  open, unclassified UCI schema". The blocking member repeats this at the use
  site.
- The ten `CommSupport*EXT` types tell *adopting programs* with details "not
  defined in the UCI standard" to define them here, and to propose broadly
  applicable ones for addition to the standard.
- Twelve of thirteen are explicit open extension points; one
  (`ConstraintEXT`) is an open-ended generic description. **None** is documented
  as reserved, unused, or uninhabited.
- Neither root uses `block` or `final` anywhere, so derivation and substitution
  are deliberately unrestricted.
- No machine-readable discriminator exists: `substitutionGroup`, `xs:appinfo`,
  `xs:import`, and `xs:any` are used zero times; the only custom attribute is
  `uci:version`, and `000.000.000.000` is shared with 1,098 ordinary inhabited
  types in 2.5.

Secondary, public UCI CAL 2.3.2 (`Santiago010/Uci-Cal-api` @
`87409b91e163931b6ac0905134367e5fb48729c9`) — an older release, therefore not
normative for 2.5/2.6:

- the generated `SourceCommandEXT` accessor has protected constructors "only
  available to derived classes" and a pure-virtual `copy`;
- the ASB layer exposes `create(uci::base::accessorType::AccessorType)`;
- `ExtensionCommand` is a mutable `BoundedList<SourceCommandEXT>` whose
  `resize(size, type)` documents that `type` may be "a accessor derived from the
  BoundedList's base type";
- JSON and XML serializers iterate every list entry, grow the list on
  deserialize, emit type attributes, and delegate to derived-type serializers.

## Decision

Treat these as **open extension points** (open world). Schema-set closure holds
for a generation invocation; type-universe closure does not.

Consequently:

- `SourceCommandEXT`'s repeated occurrence stays **fail-closed**. An
  always-empty collection is not justified and is not implemented.
- Task 026's optional elision is **correct under a closed-schema world model and
  is recorded as closed-world-only behavior**, not as general UCI semantics.
- No behavior may key on the `EXT` name suffix or on exact UCI type names.

## Options considered

1. **Fail closed** (status quo) — highest fidelity, no ownership or codec
   commitments, leaves `DisseminationSubplanType` ungenerated.
2. **Explicit closed-world generation mode** — makes today's implicit assumption
   opt-in and reviewable; would belong in `codegen-core` as a language-neutral
   policy, surfaced via the CLI, never chosen per backend.
3. **Runtime-polymorphic extension interface** — full fidelity, but requires
   dispatching/heap representations that conflict with the current SPARK-friendly
   Ada model and with Rust/C++ value semantics, plus codec and CAL-runtime
   support that do not yet exist.
4. **Externally supplied extension registry at generation time** — in practice
   "add the private extension schema to the schema set", after which existing
   Task 024 closed-sum lowering applies with no new code.
5. **Opaque serialized payload** — erases structure and presumes a wire encoding
   before any codec layer exists.

Placeholder representations (empty struct, Ada null record, unit enum variant,
payload-free `Unknown` marker) are rejected outright: each erases the extension
payload, the runtime subtype identity, or both, so none can round-trip an
instance from a valid private extension schema.

## Consequences

- Six generation probes remain blocked at `SourceCommandEXT` with the unchanged
  Task 024 diagnostic; this is now a documented, evidence-backed position rather
  than an open question.
- The generator's world model is documented as an assumption rather than a
  proven property, so future runtime work inherits an explicit correctness
  boundary instead of a hidden one.
- Task 026 requires no change today, because the generator emits no codec and no
  runtime that could receive an external extension value.

## Recommended direction

Adopt option 2 to make the world model explicit in `codegen-core`, and option 4
as the preferred route to real open-extension support. Defer option 3 until
Phase 3 runtime and codec work exists. Revisit if a pinned UCI specification
document or a pinned CAL 2.5/2.6 interface becomes available.

## Status: Accepted (Task 028)

Task 028 puts this decision into force, so the status moves from **Proposed** to
**Accepted** under the repository convention that a status is only raised once
the decision is actually in effect in the codebase.

What "Accepted" means here:

- open extensions are recognised as a **semantic possibility**, not a
  hypothesis: the generator now carries a language-neutral
  `GenerationWorld` policy with `ClosedSchemaSet` and `OpenExtensions`
  variants (recommended option 2);
- closed-schema assumptions are **explicit**: `generate` and `coverage` require
  `--world`, with no implicit default, so Task 024 closed sums and Task 026
  optional elision now only apply where a caller has actually asserted
  type-universe completeness;
- open mode **fails closed**: every abstract structural value position is
  rejected with a diagnostic that names the target and the policy, and no
  placeholder value is invented;
- option 4 is **validated**: a same-target-namespace private-extension overlay
  fixture shows that supplying the private derived-type schema in the
  generation schema set makes ordinary Task 024 lowering apply with no new code.

What "Accepted" does **not** mean:

- no runtime open-extension representation exists. Option 3
  (runtime-polymorphic extension interface) remains deferred to Phase 3 runtime
  and codec work;
- `SourceCommandEXT`'s repeated occurrence is still fail-closed under both
  worlds; no always-empty collection lowering was added;
- there is still no extension registry runtime, no `xsi:type` handling, no
  codec, and no multi-namespace private-extension support.

Selecting `--world open-extensions` therefore does not make more schemas
generate; it makes the generator *stop pretending* that the supplied descendant
set is exhaustive. It is the honest mode, and it is deliberately more
restrictive than `--world closed-schema` until a real runtime representation
exists.

## Follow-up: option 4 becomes operational (Task 029)

The status remains **Accepted**; this records how the recommended direction
advanced.

Task 028 validated option 4 only through a fixture whose root `xs:include`d the
private document. That is not usable against a *pinned* authoritative root:
adding the private schema would have required editing the root or manufacturing
a wrapper document of new `xs:include` directives. Task 029 removes that
obstacle by making the composition an explicit **input** instead of a schema
edit — a repeatable additive `--overlay PATH` on `validate`, `coverage`, and
`generate`, backed by one frontend API,
`load_schema_set_with_overlays(root, overlays)`.

Option 4 is therefore now operational rather than merely demonstrated:

- a private same-namespace derived type can be supplied against an unmodified
  pinned root, and existing Task 024 closed-sum lowering then applies with no
  new code;
- measured against the pinned UCI roots, a synthetic private descendant of
  `SourceCommandEXT` moves all six `closed-schema` probes past the long-standing
  zero-descendant blocker. The next blocker is unrelated to extension points
  (`Primitive(Duration)` in 2.5, `constraints on AA_CodeType` in 2.6) and was
  deliberately not implemented;
- the same overlay removes `SourceCommandEXT` from the zero-known-descendant
  inventory (13 → 12 in both releases) through ordinary Task 024 topology
  analysis rather than any special registry path — which is the concrete
  evidence that option 4 needs no new lowering machinery;
- open-world behaviour is measurably unchanged: all six `open-extensions` probes
  still stop at `CapabilityCommandBaseType` with and without the overlay.

Full measurements are in `docs/backend-compatibility.md`, "Task 029 — additive
schema overlays". The UCI probe overlay is synthetic test data and is not
committed.

Clarifications and unchanged boundaries:

- **same target namespace only.** A top-level overlay must share the root's
  `targetNamespace`. The backends remain single-namespace, so a private derived
  type in a *different* namespace is still unsupported.
- **additive only.** Overlays never override, shadow, or remove declarations; a
  duplicate qualified name is still an error.
- **no world inference.** Supplying an overlay does not select
  `ClosedSchemaSet`; under `OpenExtensions` known descendants remain
  non-exhaustive and abstract values still fail closed.
- **option 3 remains deferred.** There is still no runtime-open polymorphism,
  extension registry, `xsi:type` dispatch, or codec. Overlays are build-time
  schema composition and nothing more.
