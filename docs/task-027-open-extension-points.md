# Task 027 — UCI Open Extension-Point Semantics

Evidence-first audit of what a zero-known-descendant abstract UCI type means,
and an audit of the world-model assumption Task 026 relies on.

No production code changed in this task.

## Question

Task 026 left one authoritative blocker unimplemented: a `0..unbounded`
occurrence of `SourceCommandEXT`, an abstract type with zero concrete
structural descendants in the pinned schema set.

Two incompatible readings were possible:

- **A — closed-world uninhabited value.** The supplied schema set is complete
  for this target, so no legal value can exist and the collection is always
  empty.
- **B — open-world extension point.** Concrete descendants legitimately exist
  outside the supplied public schema set and are supplied by another
  schema/runtime/domain.

Task 027 answers this from measured evidence before any lowering is attempted.

## Authoritative inputs

| Role | Path |
|---|---|
| UCI 2.5 root | `/tmp/ams-gra-uci-probe-2.5/UCI_MessageDefinitions_v2_5_0.xsd` (147,419 lines) |
| UCI 2.6 root | `/tmp/ams-gra-uci-probe-2.6/UCI_MessageDefinitions_v2_6_0.xsd` (147,748 lines) |

Both roots declare `targetNamespace="https://www.vdl.afrl.af.mil/programs/oam"`,
`elementFormDefault="qualified"`, `attributeFormDefault="unqualified"`, and each
pulls in exactly one sibling via `<xs:include
schemaLocation="UCI_SecurityMarkings_v2_X_0.xsd"/>`. The root annotation carries
*Distribution Statement A. Approved for public release: distribution is
unlimited.*

Baseline binary used for all probes:

- `git rev-parse HEAD` = `2d81918402531435b3e215b62e58ff061466debd`
- `sha256sum target/release/ams-gra-codegen-oms` =
  `170dd1682dbf0847849658d3bbb6e85283230e562458fb25ad8c76213c9325a4`

All six fresh generation probes (2.5/2.6 × Ada/Rust/C++) fail identically with:

```text
error: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants
```

## Secondary compatibility evidence

Public generated UCI CAL C++ interface for **UCI 2.3.2**:

- repository `Santiago010/Uci-Cal-api`
- commit `87409b91e163931b6ac0905134367e5fb48729c9`

This is **secondary**: it is an older release than the pinned 2.5/2.6
authoritative roots, and 2.3.2 behavior is *not* automatically normative for
2.5/2.6. It is used only as a corroborating compatibility signal.


## SourceCommandEXT current-schema evidence

### Declaration

| Property | UCI 2.5 | UCI 2.6 |
|---|---|---|
| source file | `UCI_MessageDefinitions_v2_5_0.xsd` | `UCI_MessageDefinitions_v2_6_0.xsd` |
| declaration line | 95638 | 95712 |
| `abstract` | `true` | `true` |
| `uci:version` | `000.000.000.000` | `000.000.000.000` |
| base type | none (no `xs:complexContent`, no `xs:extension`) | same |
| local compositor/content | none — the type body is *only* an `xs:annotation` | same |
| concrete descendants | 0 | 0 |
| abstract descendants | 0 | 0 |

The declaration is textually identical between releases:

```xml
<xs:complexType name="SourceCommandEXT" abstract="true" uci:version="000.000.000.000">
    <xs:annotation>
        <xs:documentation>This is a type that represents a point of abstract extension to create SourceCommands that can't be documented in the open, unclassified UCI schema.</xs:documentation>
    </xs:annotation>
</xs:complexType>
```

That documentation sentence is the decisive authoritative statement. It does not
say "unused", "reserved", or "not yet defined". It says the type *is* a point of
abstract extension for SourceCommands that **cannot be documented in the open,
unclassified UCI schema** — i.e. the descendants exist but are deliberately not
published here. No inference from the `EXT` name suffix is needed or used.

### The exact blocking value use

| Property | UCI 2.5 | UCI 2.6 |
|---|---|---|
| owner declaration | `DisseminationSubplanType` (line 30909) | `DisseminationSubplanType` (line 30899) |
| owner abstract? | no | no |
| owner `uci:version` | `002.002.001.000` | `002.002.001.000` |
| owner documentation | "This type represents the information necessary to disseminate a product." | identical |
| member name | `ExtensionCommand` | `ExtensionCommand` |
| member line | 30929 | 30919 |
| compositor | `xs:sequence` (Record, not Choice) | same |
| local vs inherited | local | local |
| `minOccurs` | `0` | `0` |
| `maxOccurs` | `unbounded` | `unbounded` |
| `nillable` | absent (not nillable) | absent |
| local constraints | none | none |

```xml
<xs:element name="ExtensionCommand" type="uci:SourceCommandEXT" minOccurs="0" maxOccurs="unbounded">
    <xs:annotation>
        <xs:documentation>This element represents a point of abstract extension to create SourceCommands that can't be documented in the open, unclassified UCI schema.</xs:documentation>
    </xs:annotation>
</xs:element>
```

The normalized shape is confirmed as exactly `minOccurs = 0`,
`maxOccurs = unbounded` in both releases. The member documentation restates the

## All 13 zero-descendant targets

Recomputed independently from both authoritative roots. Each release declares
70 abstract complex types; exactly **13** of them are never named by any
`base="uci:…"`, so the topology count is unchanged from Task 026.

| # | Target | Value-use owner (2.5 line / 2.6 line) | Member | Shape |
|---|---|---|---|---|
| 1 | `CommSupportCapabilityEXT` | `CommSupportCapabilityType` 20504 / 20493 | `ExtensionData` | `0..1` |
| 2 | `CommSupportCapabilityStatusEXT` | `CommSupportCapabilityStatusType` 20458 / 20447 | `ExtensionData` | `0..1` |
| 3 | `CommSupportCommandEXT` | `CommSupportCapabilityCommandType` 20338 / 20332 | `ExtensionData` | `0..1` |
| 4 | `CommSupportCommandStatusEXT` | `CommSupportCommandStatusMDT` 20598 / 20587 | `ExtensionData` | `0..1` |
| 5 | `CommSupportPlanningStatusEXT` | `CommSupportPlanningStatusMDT` 21178 / 21167 | `ExtensionData` | `0..1` |
| 6 | `CommSupportPointingActivityEXT` | `CommPointingActivityType` 19659 / 19653 | `ExtensionData` | `0..1` |
| 7 | `CommSupportPointingEXT` | `CommPointingMDT` 19721 / 19715 | `ExtensionData` | `0..1` |
| 8 | `CommSupportStatusEXT` | `CommSupportStatusMDT` 21601 / 21617 | `ExtensionData` | `0..1` |
| 9 | `CommSupportTaskEXT` | `CommSupportTaskBaseType` 21638 / 21654 | `ExtensionData` | `0..1` |
| 10 | `CommSupportWindowEXT` | `CommWindowType` 23036 / 23052 | `ExtensionData` | `0..1` |
| 11 | `ConstraintEXT` | `NavigationConstraintsBaseType` 56802 / 56843 | `Other` | `0..1` |
| 12 | `OpNotificationEXT` | `OpNotificationMDT` 59923 / 59986 | `Extension` | `0..1` |
| 13 | `SourceCommandEXT` | `DisseminationSubplanType` 30909 / 30899 | `ExtensionCommand` | **`0..unbounded`** |

Shared structural facts for all 13, in both releases:

- `abstract="true"`;
- `uci:version="000.000.000.000"`;
- no base type, no `xs:complexContent`, no `xs:extension`, no `xs:restriction`;
- **no local fields at all** — the complex-type body contains only an
  `xs:annotation`;
- zero concrete descendants and zero abstract descendants.

## Value-use/cardinality inventory

Per-target reference counts across each whole authoritative root:

| Use kind | Count per target (2.5) | Count per target (2.6) |
|---|---:|---:|
| Record field (`<xs:element … type="uci:T">`) | 1 | 1 |
| Choice alternative | 0 | 0 |
| message payload | 0 | 0 |
| `base_type` (`base="uci:T"`) | 0 | 0 |
| Alias target | 0 | 0 |
| List target (`itemType`) | 0 | 0 |

Occurrence cross-tab over all 13 targets, identical in 2.5 and 2.6:

| Shape | Record fields | Choice alternatives |
|---|---:|---:|
| `0..1` | 12 | 0 |
| `1..1` | 0 | 0 |
| `0..N` (positive finite) | 0 | 0 |
| `1..N` (positive finite) | 0 | 0 |
| `0..*` (unbounded) | **1** (`SourceCommandEXT`) | 0 |
| `1..*` (unbounded) | 0 | 0 |

So the current first blocker is the *only* use of `SourceCommandEXT`, and the
`0..unbounded` shape is unique among the 13 targets. Every other target is used
in exactly one non-nillable, unconstrained, `0..1` Record position — precisely
the shape Task 026 elides.


## Documentation classification

Classification of each target's authoritative declaration documentation. **This
classifier exists for this report only.** No production behavior keys on
documentation text, and no string matching against descriptions is proposed.

| Target | Class | Basis (authoritative documentation) |
|---|---|---|
| `CommSupportCapabilityEXT` | **A — explicitly open** | "Defines the generic extension point for CommSupportCapability details. For adopting programs or communication systems with truly unique details not defined in the UCI standard, this type can be used as an extension point to define them." |
| `CommSupportCapabilityStatusEXT` | **A** | same wording, "CommSupportCapabilityStatus details" |
| `CommSupportCommandEXT` | **A** | same wording, "CommSupportCommand details" |
| `CommSupportCommandStatusEXT` | **A** | same wording, "CommSupportCommandStatus details" |
| `CommSupportPlanningStatusEXT` | **A** | same wording, "CommSupportPlanningStatus details" |
| `CommSupportPointingActivityEXT` | **A** | same wording, "CommSupportActivity pointing details" |
| `CommSupportPointingEXT` | **A** | same wording, "CommPointing details" |
| `CommSupportStatusEXT` | **A** | same wording, "CommSupportStatus details" |
| `CommSupportTaskEXT` | **A** | same wording, "CommSupport Task details" |
| `CommSupportWindowEXT` | **A** | same wording, "CommSupport window details" |
| `ConstraintEXT` | **C — generic abstract description** | "Indicates a navigation or navigational task constraint otherwise not listed." Open-ended, but names no external supplier. |
| `OpNotificationEXT` | **A** | "This is a type that represents a point of abstract extension to create addition details for an OpNotification message." |
| `SourceCommandEXT` | **A** | "…a point of abstract extension to create SourceCommands that can't be documented in the open, unclassified UCI schema." |

Counts: **12 of 13 class A**, 1 class C, **0 class B**, 0 class D. Not one of
the 13 is documented as reserved, unused, deprecated, or uninhabited.

The ten `CommSupport*EXT` types carry the strongest phrasing for model B: they
tell *adopting programs* to define their own details as extensions, and add
"Details that are applicable to multiple programs or communications systems
should be proposed for addition to the UCI standard" — which only makes sense if
program-local derived types already exist outside the standard.

## Documentation term search

Occurrence counts in `<xs:documentation>` text of the pinned roots (2.5 shown;
2.6 matches):

| Term | Hits | Notes |
|---|---:|---|
| `extension point` | 38 | declaration + member text across the EXT families |
| `classified` | 32 | mostly NITF/STANAG classification fields, unrelated |
| `adopting programs` | 20 | all in the `CommSupport*EXT` family |
| `unclassified` | 5 | 2 are the `SourceCommandEXT` declaration + member |
| `point of abstract extension` | 3 | `SourceCommandEXT` ×2, `OpNotificationEXT` ×1 |
| `vendor` | 3 | all unrelated (modulation/status enums, not EXT types) |
| `mission-specific` | 0 | — |
| `implementation-specific` | 0 | — |
| `open schema` | 0 | — |
| `derived type` | 0 | — |
| `xsi:type` | 0 | — |
| `accessor type` | 0 | — |

The three non-EXT `unclassified` hits are NITF/STANAG classification-field
descriptions. No pinned supporting document in the probe directories
(`VDD.txt`, `README_DISCLAIMER.txt`) adds extension-point language beyond the
schema itself.

## Machine-readable discriminator search

Searched both authoritative roots for any structural marker distinguishing an
open extension point beyond `abstract=true`, zero known descendants, the name,
and the documentation prose:

| Candidate discriminator | 2.5 | 2.6 | Finding |
|---|---:|---:|---|
| `substitutionGroup` | 0 | 0 | never used anywhere in the schema |
| `block=` / `final=` | 0 | 0 | never used |
| `xs:appinfo` | 0 | 0 | never used |
| `xs:import` | 0 | 0 | no cross-namespace imports; only one `xs:include` |
| namespace differences | — | — | single `targetNamespace`, no second namespace |
| `xs:any` / `xs:anyAttribute` | 0 | 0 | no wildcard extension slot |
| custom UCI/OAM attributes | — | — | the *only* custom attribute in the whole schema is `uci:version` (6,256 uses) |
| special base class | — | — | none: the 13 targets have no base at all |
| version metadata | — | — | all 13 carry `uci:version="000.000.000.000"` |

**Result: no reliable machine-readable discriminator exists.**

The `uci:version="000.000.000.000"` value is suggestive but **not** a valid
discriminator: 1,098 complex types in 2.5 carry that same version string,
including ordinary inhabited concrete types such as `AccessAssessmentID_Type`
and `TimeWindowType`. Using it to mean "open extension point" would misclassify
over a thousand types. Nothing in the schema is therefore machine-usable today
to separate model A from model B, which is why any future behavior must come
from an explicit external semantic source rather than from the schema alone.


## Public CAL 2.3.2 behavior (secondary)

### `cppInterface/2.3.2/include/uci/type/SourceCommandEXT.h`

- Documentation is the **same sentence** as the 2.5/2.6 schema: "a point of
  abstract extension to create SourceCommands that can't be documented in the
  open, unclassified UCI schema."
- `class SourceCommandEXT : public virtual uci::base::Accessor` — an abstract
  accessor base.
- Accessor type identity: `getAccessorType()` returns
  `uci::type::accessorType::sourceCommandEXT`; `getUCITypeVersion()` returns
  `"000.000.000.000"`, matching the schema's `uci:version`.
- `virtual void copy(const SourceCommandEXT& accessor) = 0;` — pure virtual, so
  the base cannot be instantiated.
- Constructor, destructor, copy constructor, and `operator=` are all
  **`protected`**, documented "[only available to derived classes]". The
  generated header is written *for* derived classes the open schema lacks.

### `starterKit/…/asb_uci/type/SourceCommandEXT.h` and `.cpp`

- Runtime creation API:
  `static std::unique_ptr<SourceCommandEXT> create(uci::base::accessorType::AccessorType type);`
  — creation is parameterized by a **runtime accessor type**.
- Its implementation resolves `null` to `sourceCommandEXT` and returns a
  non-null instance for that type, `nullptr` otherwise. The ASB reference
  implementation therefore *does* produce `SourceCommandEXT` values at runtime;
  it is not a hard "never constructible" path.
- `copyImpl(const uci::type::SourceCommandEXT&, bool checkIfDerivation)` carries
  an explicit **derivation check** parameter.
- `serialize(…, bool addTypeAttribute = false, bool checkIfDerivation = true, …)`
  likewise. The signatures are shaped around derived-type dispatch.

### CAL list semantics — the direct test of "always empty"

`cppInterface/2.3.2/include/uci/type/DisseminationSubplanType.h`:

```cpp
/** This element represents a point of abstract extension to create SourceCommands that can't be documented in the open,
  * unclassified UCI schema. [Occurrences: Minimum: 0; Maximum: MAX_LENGTH]
  */
typedef uci::base::BoundedList<uci::type::SourceCommandEXT, uci::type::accessorType::sourceCommandEXT> ExtensionCommand;
```

with full mutable accessors: `getExtensionCommand()` (const and non-const) and
`setExtensionCommand(const ExtensionCommand&)`.

`uci/base/BoundedList.h` is decisive:

```cpp
virtual void resize(size_type newSize, uci::base::accessorType::AccessorType type = V) = 0;
```

whose documentation states the `type` parameter "provides support for
inheritable types… this argument must either be a type ID associated with the
BoundedList's base type or **a accessor derived from the BoundedList's base
type**." `push_back` and `at()` are likewise exposed.

So the generated CAL API **explicitly permits a nonempty `ExtensionCommand`
collection populated with runtime-derived `SourceCommandEXT` values**. A public
generated interface does not expose `resize(n, derivedType)`, `push_back`, and
`setExtensionCommand` for a collection its own generator believed could only
ever be empty. This directly contradicts the "always empty" reading.

### Derived-type serialization evidence

`externJSON/src/extjson_uci/type/DisseminationSubplanType.cpp` and the
corresponding `externXML` file both:

- **iterate every entry** on serialize:
  `for (… i = 0, end = boundedList.size(); i < end; ++i) SourceCommandEXT::serialize(boundedList.at(i), node, ExtensionCommand_Name);`
- **grow the list on deserialize**: `boundedList.resize(boundedListSize + 1);`
  then `SourceCommandEXT::deserialize(valueType.second, boundedList.at(…), …)`
  — entries are read back into `SourceCommandEXT` slots;
- **dispatch on runtime derived type** at the owner level:
  `if (!checkIfDerivation || (accessor.getAccessorType() == uci::type::accessorType::disseminationSubplanType)) { … } else { DerivedTypesSerializer::serialize(accessor, propTree, nodeName, createNode); }`
- **emit type identity** via
  `SerializationHelpers::addTypeAttribute(node, Extern_Type_Name)` when
  `addTypeAttribute` is set, and both translation units include
  `util/DerivedTypesSerializer.h` and `util/DerivedTypesDeserializer.h`.

The whole codec path is built for derived extension instances the open schema
does not declare. None of this is implemented in this task.

## Current-vs-2.3.2 comparison

| Semantic statement | UCI 2.5 schema | UCI 2.6 schema | Public CAL 2.3.2 | Confidence |
|---|---|---|---|---|
| `SourceCommandEXT` is an extension point | **Yes** — declaration + member documentation both say "point of abstract extension" | **Yes** — identical text | Yes — same sentence in the generated header | **High** (authoritative, both releases) |
| Descendants may exist outside the open schema | **Yes** — "can't be documented in the open, unclassified UCI schema" | **Yes** | Yes — protected ctor "only available to derived classes"; `create(AccessorType)` | **High** |
| Repeated values are meaningful | Implied — `minOccurs=0 maxOccurs=unbounded` with extension documentation | Implied — identical | **Yes, demonstrated** — `BoundedList` with `resize(n, derivedType)`, `push_back`, `setExtensionCommand` | **Medium-High** (authoritative shape + secondary API) |
| Runtime subtype identity matters | Not expressible in XSD alone (no `substitutionGroup`/`appinfo`) | same | **Yes** — `getAccessorType()`, `checkIfDerivation`, `addTypeAttribute`, `DerivedTypesSerializer` | **Medium** (secondary only) |
| Empty-only interpretation is justified | **No** — contradicted by declaration documentation | **No** | **No** — contradicted by mutable list + per-entry codec | **High** |


## Closed schema set vs open type universe

Three distinct questions, deliberately not conflated:

**1. Is the loaded XSD dependency graph closed as a FILE SET?**
**Yes.** Each root has exactly one `xs:include` (`UCI_SecurityMarkings_v2_X_0.xsd`),
zero `xs:import`, a single `targetNamespace`, and no wildcard (`xs:any`)
indirection. The generator resolves a finite, deterministic file closure per
invocation. This is a real and useful property.

**2. Does that imply the UCI TYPE UNIVERSE is closed?**
**No.** File-set closure only bounds what *this invocation can see*. XSD
derivation by extension is an open mechanism: any schema may derive from
`uci:SourceCommandEXT`, and an instance document may select that derived type
via `xsi:type` at a `SourceCommandEXT`-typed element. Nothing in the pinned
schema prevents it — `block`/`final` are never used anywhere in either root,
which would be the standard XSD way to forbid derivation or substitution. The
schema authors left derivation deliberately unblocked.

**3. Can CAL or external schemas legally supply derived extension types not
present in the public schema files?**
**Yes.** The authoritative documentation says so directly ("can't be documented
in the open, unclassified UCI schema"; "For adopting programs … this type can be
used as an extension point to define them"), and the public CAL 2.3.2 generated
interface is built to carry them.

**Conclusion: schema-set closure holds; type-universe closure does not.**

## Task 026 assumption audit

Task 026 elides storage for an optional (`0..1`, non-nillable, unconstrained)
Record field whose target has zero concrete descendants, on the stated ground
that its "only legal state is absence."

For every one of the 13 targets, ask: *could a valid external/private/runtime
subtype make this optional value present?*

| Target | Could an external subtype make it present? | Basis |
|---|---|---|
| all ten `CommSupport*EXT` | **Yes** | documentation directs adopting programs to define extensions here |
| `ConstraintEXT` | **Probably** | "otherwise not listed" is open-ended, but names no external supplier — class C |
| `OpNotificationEXT` | **Yes** | "point of abstract extension to create addition details" |
| `SourceCommandEXT` | **Yes** | "can't be documented in the open, unclassified UCI schema" |

So **12 of 13 elided-or-blocked targets are documented open extension points.**
Task 026's elision is therefore a **deliberate closed-world limitation, not
complete UCI extension semantics.** Stated explicitly, without pretending
otherwise:

> Under a closed-schema world model — only types declared in the supplied schema
> set may ever appear in a value — Task 026's optional elision is correct.
> Under general UCI/CAL interoperability — where an adopting program supplies a
> private derived type and a peer sends it — an elided `ExtensionData` field
> would **silently drop data that a conforming peer legitimately sent.**

That data-loss risk is real but currently unexercised: this generator produces
no codec and no runtime, so nothing today can receive such a value. The
limitation is a correctness boundary for future runtime work, not a present bug.

### Is Task 026 unsound?

Distinguishing the two levels, and never using "sound" without naming the world
model:

- **A. Sound for a declared CLOSED-SCHEMA generation mode** (only types in the
  supplied schema set can ever appear): **Yes, Task 026 is sound.** Under that
  assumption an optional occurrence of a target with no declared descendant
  genuinely has one legal state. Task 026 also correctly fails closed on every
  other occurrence shape, and correctly verifies whole-schema usage before
  skipping a wrapper.
- **B. Sound for GENERAL UCI/CAL interoperability** (externally supplied private
  derived extension types may appear): **No, not established.** Under model B an
  elided field cannot represent a present extension value.

**Task 026 is valid under A, and is not claimed valid under B.** Notably,
Task 026 *refused* to extend the same reasoning to the repeated
`SourceCommandEXT` occurrence — under model B, an always-empty collection would
have been wrong, so that caution is vindicated by this task's evidence.

## Decision gate

**DECISION B — OPEN WORLD CONFIRMED.**

Authoritative current UCI 2.5/2.6 evidence establishes that externally
defined/private derived values are valid at these extension points:

1. The `SourceCommandEXT` declaration documentation, identical in 2.5 and 2.6,
   states the type exists to create SourceCommands that *cannot be documented in
   the open, unclassified UCI schema* — an explicit statement that the concrete
   forms live outside this schema set.
2. The blocking member's own documentation repeats that at the use site.
3. Ten of the thirteen zero-descendant targets instruct *adopting programs* to
   define their own details as extensions here.
4. Zero of the thirteen are documented as reserved, unused, or uninhabited.
5. Neither root ever uses `block` or `final`, so derivation and substitution are
   deliberately left unrestricted.

Secondary public CAL 2.3.2 evidence corroborates independently: mutable
`BoundedList<SourceCommandEXT>` with `resize(n, derivedAccessorType)`, protected
constructors "only available to derived classes", a runtime
`create(AccessorType)` factory, and derived-type serialize/deserialize paths.

Consequences, per the decision gate:

- **Always-empty repeated lowering is NOT justified and is not implemented.**
  `SourceCommandEXT`'s `0..unbounded` occurrence remains fail-closed with the
  unchanged Task 024 diagnostic.
- **Task 026's optional elision is identified as closed-world-only behavior**
  requiring an explicit policy or future correction for general UCI
  interoperability. It is not modified here.
- No backend semantic behavior, coverage semantic behavior, or Task 026
  implementation is altered by this task.


## Representation options

Analyzed, **none implemented**.

### Option 1 — fail closed on externally extensible values (status quo)

| Axis | Assessment |
|---|---|
| semantic fidelity | Highest available: refuses to misrepresent rather than guessing |
| Ada/SPARK impact | None |
| Rust ownership impact | None |
| C++ ownership impact | None |
| codec dependency | None |
| CAL-runtime dependency | None |
| deterministic generation | Fully deterministic |
| private-schema compatibility | N/A (nothing generated) |
| suitability before Phase 3 | **Best** — zero commitment, zero rework |

Cost: `DisseminationSubplanType` and its dependents stay ungenerated.

### Option 2 — explicit generator `closed-world` mode

| Axis | Assessment |
|---|---|
| semantic fidelity | Honest: the unsafe assumption becomes opt-in and reviewable |
| Ada/SPARK impact | None beyond Task 026's existing elision |
| Rust ownership impact | None new |
| C++ ownership impact | None new |
| codec dependency | None |
| CAL-runtime dependency | None |
| deterministic generation | Deterministic per mode; output becomes mode-dependent |
| private-schema compatibility | Explicitly out of scope in that mode — which is the point |
| suitability before Phase 3 | **Good** — smallest step that serves closed-world users without asserting general UCI semantics |

Under this mode the repeated occurrence could lower as an always-empty
collection *because the user declared the world closed*, not because the schema
says so. It also retrofits Task 026 behavior behind an explicit flag.

### Option 3 — runtime-polymorphic extension interface with future CAL runtime

| Axis | Assessment |
|---|---|
| semantic fidelity | Highest possible: preserves subtype identity and payload |
| Ada/SPARK impact | **Severe** — classwide/dispatching values conflict with the current SPARK-friendly value model |
| Rust ownership impact | Requires a heap/`dyn` representation the non-goals forbid today |
| C++ ownership impact | Requires owning-pointer semantics, likewise forbidden today |
| codec dependency | **Hard** — needs type-identity-aware codecs |
| CAL-runtime dependency | **Hard** — needs an accessor-type registry |
| deterministic generation | Achievable, but the registry becomes a generation input |
| private-schema compatibility | Full |
| suitability before Phase 3 | **Premature** — depends on runtime work not yet started |

### Option 4 — externally supplied extension registry known at generation time

| Axis | Assessment |
|---|---|
| semantic fidelity | High: derived types become ordinary closed-sum variants again |
| Ada/SPARK impact | Low — reuses Task 024's existing closed-sum lowering |
| Rust ownership impact | Low — ordinary enum variants, no heap indirection |
| C++ ownership impact | Low — ordinary variant members |
| codec dependency | None beyond existing |
| CAL-runtime dependency | None |
| deterministic generation | Deterministic if the registry is pinned like any schema input |
| private-schema compatibility | **Full for known extensions**; unknown ones still fail closed |
| suitability before Phase 3 | **Strong candidate** — it is really "supply the missing schema files", which the frontend already handles |

The cleanest framing: an adopting program that *has* its private extension
schema can simply add it to the schema set, at which point the target has
concrete descendants and every existing Task 024 mechanism works with no new
code. That makes this less a new representation than a documentation and
configuration story.

### Option 5 — opaque serialized extension payload

| Axis | Assessment |
|---|---|
| semantic fidelity | Low — preserves bytes but erases structure; also on this task's non-goal list |
| Ada/SPARK impact | Needs unbounded byte storage at every extension position |
| Rust ownership impact | `Vec<u8>` wherever an extension may appear |
| C++ ownership impact | `std::vector<std::uint8_t>` wherever an extension may appear |
| codec dependency | **Hard** — meaningless without a defined encoding |
| CAL-runtime dependency | Medium |
| deterministic generation | Deterministic |
| private-schema compatibility | Round-trips unknown data but gives no typed access |
| suitability before Phase 3 | **Poor** — commits to a wire encoding before any codec layer exists |


## Why fake placeholder representations are rejected

An open extension point carries two things: an **unknown payload** and a
**runtime subtype identity**. Each placeholder loses at least one:

- **empty struct** (`struct SourceCommandExt;`) — claims the value has no
  fields. A derived extension has fields by definition; every one is erased.
- **Ada null record** — same erasure, and additionally implies to a reader that
  the schema declared an empty type, which it did not.
- **unit enum variant** (`SourceCommandExt`) — records only *that* something was
  present, not which subtype, nor with what data.
- **`Unknown` marker with no payload** — the most misleading: it looks like
  deliberate handling of unknown extensions while discarding both identity and
  bytes, so a round trip silently loses a conforming peer's data.

None can round-trip an instance produced by a valid private extension schema, so
none is sufficient for a documented open extension point.

## Closed-world generation mode assessment

Task 024 and Task 026 already embed closed-world semantics implicitly. The
cleanest eventual architecture is to make the assumption **explicit** rather
than ambient, conceptually:

```text
GenerationWorld::ClosedSchemaSet   // today's implicit assumption, made explicit
GenerationWorld::OpenExtensions    // possible future
```

No config or API is introduced in this task. On placement, if adopted later:

| Candidate location | Assessment |
|---|---|
| CLI | Needed eventually as the *surface*, but must not be the definition |
| Backend options | **Wrong** — would let Ada/Rust/C++ disagree about what values exist; this is a language-neutral semantic question |
| `codegen-core` semantic policy | **Preferred** — where `field_storage_semantics` and `abstract_value` already live, so all backends and coverage consult one decision point, as they already must |
| schema profile | Reasonable alternative if the world model should travel with a pinned schema set rather than with an invocation |

Recommendation if pursued: define it in `codegen-core` (language-neutral),
surface it through the CLI, and never let a backend choose it.

## No `EXT` name heuristics

No production behavior should key on `ends_with("EXT")` or on exact UCI type
names, regardless of how strong the evidence is. The evidence above shows why
this is not merely a style preference: the naming convention and the
documentation prose are the *only* signals present, and neither is
machine-reliable — `uci:version="000.000.000.000"` is shared with 1,098 ordinary
types, and `ConstraintEXT` carries class-C prose despite the same suffix. Any
future special handling requires an explicit semantic source: schema metadata,
generation configuration, an extension registry, a runtime contract, or another
evidence-backed mechanism.

## Recommended next implementation boundary

1. Keep `SourceCommandEXT`'s repeated occurrence **fail-closed**. Do not lower it
   as an always-empty collection under any current evidence.
2. Make the world model **explicit** (Option 2) in `codegen-core`, so Task 026's
   elision is attributable to a declared assumption rather than being ambient.
3. Prefer **Option 4** for real open-extension support: an adopting program adds
   its private extension schema to the schema set and every existing Task 024
   closed-sum mechanism applies unchanged.
4. Defer Option 3 until Phase 3 runtime/codec work exists; it cannot be
   evaluated properly before then.
5. Revisit if authoritative evidence changes — specifically, a pinned UCI
   specification document or a pinned CAL 2.5/2.6 interface would raise the
   runtime-subtype-identity row of the comparison table from Medium to High.

## Performance note

This task adds no production code and therefore no schema-wide nested scans.
Production generation behavior and output are byte-for-byte unchanged; the
baseline binary hash recorded above is the same binary used for all six probes.

extension semantics at the *use site*, not only at the declaration.
