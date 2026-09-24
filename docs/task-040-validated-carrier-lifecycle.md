# Task 040 — validated lexical-carrier lifecycle

Corrective task. Two lifecycle gaps in the **existing** generated validated
lexical carriers are closed. No schema-support surface changes: the classifier,
the effective XSD facets, the accepted lexical spaces, and the current
normalization semantics are all untouched, and the Task 039 observed-bound
domain is not widened.

## 1. The two original failure paths

### 1.1 Ada — default initialization produced an unchecked, invalid carrier

Each carrier's private completion was a plain record whose
`Unbounded_String` component carried the predefined null-string default:

```ada
type Uuid is record
   Text : Standard.Ada.Strings.Unbounded.Unbounded_String;
end record;
```

An ordinary default declaration therefore produced a **usable, fully
unchecked** instance:

```ada
Item : Test.Identity.Uuid;               --  Create is never called
Put_Line (Test.Identity.Value (Item));   --  printed the empty string
```

Every one of these profiles rejects the empty string, so the carrier's own
advertised invariant did not hold for a value any client could obtain without
writing anything unusual. Privacy did **not** prevent this: privacy stops a
client from *naming* the representation, but it does not stop the language from
default-initializing it.

### 1.2 C++ — implicit destructive move left an invalid, still-live source

The generated classes declared no special members, so the compiler implicitly
declared a move constructor and a move assignment operator. Both transferred
`value_`'s buffer and left the **still-live source** holding an unspecified
`std::string`:

```cpp
auto a = *Uuid::create("123e4567-e89b-12d3-a456-426614174000");
Uuid b = std::move(a);
a.value();          // observed empty — rejected by Uuid::create
Uuid c = a;         // propagated that invalid representation
```

The observed emptiness is an implementation detail of `std::string`, but the
*defect* is not: a validated carrier that can be observed holding a
representation its own validator rejects, and that can then be copied, has lost
the invariant it exists to hold.

## 2. Scope of affected carriers

Both defects affected **all four** existing validated lexical-carrier families
in both backends. Coverage was confirmed per family rather than inferred from
one declaration name:

| Family | Ada default | C++ implicit move |
| --- | --- | --- |
| UCI schema-version String profile | affected | affected |
| UUID String profile | affected | affected |
| Visible-ASCII String profiles (all bounds) | affected | affected |
| Named Zulu DateTime profile | affected | affected |

Rust was **not** affected by either defect, and its generated output is
byte-identical before and after this task.

Deliberately out of scope and unchanged: ordinary unconstrained `String`
representations, and numeric/other wrapper lifecycle semantics.

## 3. The selected Ada initialization policy

The private component's default is now an explicitly failing **raise
expression**:

```ada
type Uuid is record
   Text : Standard.Ada.Strings.Unbounded.Unbounded_String :=
     raise Standard.Program_Error
       with "Uuid requires initialization from Create";
end record;
```

A carrier must therefore be initialized from `Create` or from an already valid
carrier; an unchecked default declaration fails explicitly. No default UUID,
date, version, or visible string is invented, and the empty representation is
never treated as valid.

Why a component default rather than the alternatives:

- it is **assertion-independent**. Enforcement is part of Ada's initialization
  semantics, so it does not rely on the client compiling with `-gnata`, on any
  `Assertion_Policy`, on a caller-side precondition, or on a type invariant an
  ordinary client setting could silently disable. The regressions build the
  probes **without** `-gnata` and additionally under
  `pragma Assertion_Policy (Ignore)` to demonstrate exactly that;
- it introduces **no new package-scope name**, so the shared generated-name
  model owes nothing new and no collision or optional-storage regression can be
  reintroduced;
- it is not a public "possibly invalid" carrier API.

`Create` already built its result with an explicit named aggregate after
validation succeeded, so **legitimate construction never evaluates the
rejecting default**. The public `Create` / `Value` signatures are unchanged; no
compiler or composition constraint forced an API change.

### Client-visible behavior change

Ada code that default-declared one of these carriers and then relied on reading
empty text now raises `Program_Error` at that declaration. Such code was
already holding a value its own profile rejects, so this converts a silent
correctness defect into an explicit failure. All legitimate construction paths
are unaffected.

One consequence worth stating precisely: a *constrained present* optional
wrapper, `Payload_Label_Optional (Is_Present => True)`, elaborates its payload
component and therefore raises. That is the intended result — it is exactly the
"default-created present payload silently contains invalid text" case. An
**absent** wrapper still default-creates normally, because its absent variant
has no payload component.

A practical testing note recorded so it is not rediscovered: an exception
raised while elaborating a *declarative part* propagates **past** that block's
own handler, so the negative-control probes declare the offending object inside
a nested procedure.

## 4. The selected C++ special-member policy

Each affected class now declares its copy operations explicitly:

```cpp
Uuid(const Uuid&) = default;
Uuid& operator=(const Uuid&) = default;
```

Declaring the copy operations suppresses the *implicit declaration* of the move
constructor and move assignment operator (C++17 `[class.copy.ctor]/8` and
`[class.copy.assign]/4`). The type stays copy-constructible and
copy-assignable, and overload resolution on an rvalue selects the copy
operations, so the source keeps its exact original spelling while the
destination receives the correct value.

Nothing is `= delete`d. Deleted move overloads would still participate in
overload resolution and would break rvalue construction, `std::optional`, and
generated container composition; *suppression* is what makes the fallback to
copying work.

Note also that `std::is_move_constructible` remains **true** for these types —
it is satisfied by the copy constructor. That trait is therefore *not* evidence
that a destructive move constructor exists or is being selected, which is why
the regressions assert observed values rather than traits.

No moved-from carrier is "repaired" with a fabricated value: there is no
arbitrary nil UUID and no fabricated default for any other family.

Preserved unchanged: private unchecked construction, private storage, factory
validation and rejection behavior, exact lexical spelling for the preserving
profiles, the existing DateTime `collapse` normalization, and the read-only
accessor interface. The policy is applied consistently to all four current
lexical templates, with no new public base class and no broad template rewrite.

### Performance and exception implications

- operations written with `std::move` **copy** the carrier's string;
- they may therefore **allocate**;
- they are correctly **not** marked `noexcept`, because a copy can throw
  `std::bad_alloc`. Declaring them `noexcept` while copying would be incorrect;
- this task deliberately prioritizes the validated-value invariant over
  destructive string-move optimization.

## 5. Reproductions and controls

Every test exercises code produced by the repository's actual generator, not a
hand-written imitation of a wrapper.

### Baseline reproduction (before the correction)

With the generator changes stashed, the new regressions fail as follows.

Ada (`crates/backend-ada/tests/carrier_lifecycle.rs`), all three tests FAILED:

```
Callsign must default to an explicit failure
UNCHECKED: []
UNCHECKED present payload: []
UNCHECKED present payload did not raise
```

`UNCHECKED: []` is the defect: a default-declared carrier yielded empty text
through the generated `Value` accessor.

C++ (`crates/backend-cpp/tests/carrier_lifecycle.rs`), three of four FAILED:

```
Callsign must declare its copy constructor
invalid representation after Instant rvalue-construct src
invalid representation after Instant rvalue-assign dst
invalid representation after Instant rvalue-assign src
invalid representation after Instant copy-after-rvalue
invalid representation after Instant self-rvalue-assign
invalid representation after Instant optional src
... the same six for Deadline
```

`copy-after-rvalue` is the propagation path, and `rvalue-assign dst` shows the
destination itself could end up invalid through self-move.

The permanent assertions are the lifecycle **contract** — "this carrier's own
`create` still accepts what it now holds", plus exact spelling under the
copy-preserving policy — not "some standard library emptied a moved-from
string". The observed emptiness was the reproduction; the contract is the
requirement, and it holds regardless of moved-from string behavior.

### Positive controls

Ada, all executed under GNAT **without** `-gnata`:

- explicit initialization from `Create` succeeds and stores exactly;
- copy initialization from a valid carrier succeeds;
- assignment between valid carriers preserves the exact stored text;
- an explicitly initialized **array** aggregate remains usable;
- an explicitly initialized **record** aggregate (the generated `Payload`,
  including its Task 034 optional wrappers) remains usable;
- an **absent** optional wrapper still default-creates without trying to
  construct its absent payload;
- a **present** optional wrapper initialized with `Create` succeeds;
- a default-created **present** payload cannot silently contain invalid text.

C++ runtime controls, per carrier, for all four families:

- copy construction and copy assignment;
- construction and assignment from rvalues;
- source **and** destination inspected after each operation;
- copying the source after an rvalue operation;
- self-copy assignment and self-rvalue assignment;
- `std::swap`;
- `std::optional` composition including an rvalue transfer;
- `std::vector` composition including reallocating growth;
- representative generated record composition, with both the destination and
  the still-live source record inspected member by member.

Compile-fail probes are kept and are additionally required to fail **for
privacy**, so an unrelated syntax error cannot be mistaken for enforcement:
direct construction, a raw `value_` read, and a raw read on a carrier obtained
through an rvalue operation.

### Integration coverage

`crates/cli/tests/service_generate.rs` gains two Task 040 regressions that go
through the ordinary `service-generate` path a consumer actually uses, rather
than a directly invoked backend:

- `task040_selected_cpp_output_preserves_the_validated_value`;
- `task040_selected_ada_output_requires_explicit_initialization`.

The corrective pass adds a third, which actually **constructs and manipulates**
a repeated field rather than merely compiling the generated spec:

- `task040_selected_ada_repeated_field_is_constructible`.

### Bounds coverage

The visible-ASCII family is exercised at several bounds, including the **2..4**
`CountryCode` profile, whose minimum is above one character. That ensures no
accidental valid-default assumption can hide behind a 1-character minimum.

The shared lexical corpora under `tests/fixtures/string/` and
`tests/fixtures/temporal/` are used **unchanged**; this task alters no accepted
or rejected case.

## 6. Compiler evidence

| Tool | Version |
| --- | --- |
| GNAT (`gnatmake`) | 14.2.0 |
| C++ (`g++`) | Debian 14.2.0-19, 14.2.0 |
| rustc | 1.98.1 (48a229cea 2026-09-01) |
| cargo | 1.98.1 (797e8a9bc 2026-08-05) |

C++ probes are built with `-std=c++17 -Wall -Wextra -pedantic-errors`. Ada
probes are built without `-gnata`, and the negative control is repeated under
`pragma Assertion_Policy (Ignore)`.

Test counts, all measured the same way -- summing the `test result:` passed
counts of `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`:

| Head | Passing |
| --- | ---: |
| original `main` (`2d97e46`) | **778** |
| first reviewed head (`aa60ba7`) | **787** |
| second reviewed head (`c726b9e`) | **790** |
| corrected head (this pass) | **797** |

The first reviewed head's commit message stated a `785 -> 787` transition. The
`787` is correct; the `785` baseline is not, and is corrected here. Measured
against original `main`, `aa60ba7` added **nine** tests -- seven backend
lifecycle tests plus two `service-generate` integration tests -- and `c726b9e`
added **three** more.

The second corrective pass adds **seven**, all real regressions rather than
count padding:

| Added test | Location |
| --- | --- |
| `emitted_sequence_operations_are_exactly_the_shared_definition` | `backend-ada` lib |
| `a_non_emitted_abstract_record_reserves_no_sequence_callable` | `codegen-core::backend_names` |
| `an_inherited_unbounded_member_attributes_callables_to_the_descendant` | `codegen-core::backend_names` |
| `bounded_shapes_reserve_exactly_the_operations_they_emit` | `codegen-core::backend_names` |
| `an_emitted_sequence_still_collides_with_a_conflicting_type_name` | `codegen-core::backend_names` |
| `callable_deferral_for_one_field_does_not_disable_checking` | `codegen-core::backend_names` |
| `multiple_sequences_with_valid_overloads_coexist` | `codegen-core::backend_names` |

The bounded-occupancy controls are *not* new test functions: they were added to
the existing `repeated_carrier_storage` probe and to the existing
`task040_selected_ada_repeated_field_is_constructible` integration test, which
is why the count rises by seven rather than by the number of new assertions.
No existing test was deleted or weakened to obtain this total. Two existing
tests changed premise for a substantive reason, documented in §10.5: bounded
storage now needs a package body, so `track.xsd` moved from the "emits no
`.adb`" list to a positive control asserting what its body contains.

The GNAT-backed regressions execute rather than skip: `AMS_GRA_REQUIRE_GNAT`
turns the developer-machine skip into a hard failure, and CI names the
repeated-storage probes explicitly so zero matched tests is a failure.

## 7. Capability and output results

This task changes generated **lifecycle behavior**, not which schema
declarations are supported.

### Measured coverage — UCI 2.6

Pinned revision `78eb61b6112c8bffa40820c33124b57787fc5bd9`, declarations
renderable out of 5570:

| Cell | Task 039 baseline | Task 040 measured | Delta |
| --- | ---: | ---: | ---: |
| 2.6 closed, Ada / Rust / C++ | 5335 / 5413 / 5417 | 5335 / 5413 / 5417 | **0 / 0 / 0** |
| 2.6 open, Ada / Rust / C++ | 5250 / 5325 / 5329 | 5250 / 5325 / 5329 | **0 / 0 / 0** |

Kinds (5457/5570), field-types (13198/13198), field-occurrences
(13198/13198), and message-closures (0/725) are likewise unchanged in every
cell. Repeated generation is deterministic: two consecutive coverage runs are
byte-identical, and the corrected binary's report is byte-identical to the
baseline binary's.

### Other checks

- **Rust**: generated output is byte-identical before and after, verified by a
  directory diff of the visible-ASCII fixture generation. No unnecessary output
  and no capability change.
- **No new profile or `FeatureFamily`** is introduced.
- **Unsupported-profile diagnostics** are unchanged: no classifier code was
  touched.
- **Generated-name preflight** behavior is preserved. The original Ada change
  was a component default inside an existing private record and introduced no
  package-scope name. The corrective pass *does* introduce new generated
  spellings, and they are registered in the shared model -- see §10.5.
- **Unrelated generated outputs** are unchanged.

### Verification limits — resolved in the corrective pass

The earlier revision of this document recorded the pinned **UCI 2.5** set
(`093610b7753944059360d3236770ab446d039556`) as unobtainable, and therefore
recorded six of the twelve comparison cells and the `PositionReport` recheck as
**missing measurement**.

Both were retried through the established source workflow during the corrective
pass and **were** obtainable this time. They are now freshly measured, and the
limitation is withdrawn rather than carried forward. See §10.

Full-schema first blockers are unchanged and unrelated to this task: Ada
`AltitudeRangePairType / Range`, Rust `ConfigurationParameterType / Type`, C++
`ApprovalResponseType / Operator`.

## 8. Precision about what privacy proves

Earlier task evidence stated, in effect, that privacy alone proves there is no
unchecked construction path. That statement is **too broad**, and this task is
the counterexample. It is preserved as historical evidence of what was true at
the time, and corrected here.

Stated precisely, for the Ada carriers:

- privacy prevents a client from **naming** the representation, so no client
  aggregate, component read, or type conversion can construct or inspect one.
  That remains true and is still tested;
- privacy did **not** prevent the language from **default-initializing** the
  representation. Task 040 closes that with an explicitly failing component
  default.

And for the C++ carriers:

- a private constructor and private storage prevent client construction and raw
  reads. That remains true and is still tested;
- they did **not** prevent the compiler from implicitly declaring destructive
  move operations. Task 040 closes that by declaring the copy operations.

The tested guarantee is therefore: *within the generated API, and through the
special members the language supplies, a carrier of these four families cannot
be observed holding a representation its own validator rejects.* Neither
backend claims protection against arbitrary unchecked or foreign-memory
manipulation; that is explicitly out of scope.

## 9. Out of scope

`WhitespaceVisibleString1024Type` / `WhitespaceVisibleString4096Type` support,
`NATO_SpecialWordsType` support, enum identifier remapping, general regex
support, new temporal formats, codec/serialization/runtime integration,
CAL/OWP/WebSocket changes, changes to XSD normalization or the semantic IR, and
broad cleanup of numeric-wrapper lifecycle semantics.

The previously proposed whitespace-visible-string feature is deliberately
**not** implemented here. Neither gap is addressed by documentation alone: both
corrections are in the generator.

## 10. Corrective pass — repeated storage of validated carriers

### 10.1 The over-broad claim being corrected

The first revision of this document stated, in effect, that the rejecting
component default left **all** legitimate Ada construction paths unaffected,
because `Create` builds its result with an explicit named aggregate and a
legitimate path therefore never evaluates the default.

That is **too broad**, and it is corrected here. It is true of the paths that
were actually tested -- direct declaration, copy, assignment, array aggregates,
record aggregates, and the optional wrapper. It is *not* true of **repeated
storage**, which was not exercised at all: the existing capability analysis
counts renderable declarations and never executes a container operation, so an
unchanged coverage number could not have detected this.

### 10.2 Logical elements versus spare storage

The distinction the original change missed:

* a **logical element** is an occurrence the schema says is present. It is
  always constructed from a supplied, validated value;
* **spare storage** is physical capacity a representation holds so that it
  *can* accept future elements. It corresponds to no occurrence at all.

Ada initializes storage, not occurrences. A representation that gives spare
storage the element type therefore default-initializes carriers for occurrences
that do not exist -- and after Task 040 those defaults raise.

The invariant the corrected representations hold is:

> **Unused capacity is not a live validated value.**

### 10.3 The reproduced failures

Both were reproduced on the reviewed head (`aa60ba7`) with GNAT 14.2.0, using
types produced by this repository's own generator, before any production code
was changed.

**A. Unbounded.** `render_unbounded_helper()` instantiated
`Ada.Containers.Vectors` -- the *definite* vector -- over the generated element
type. GNAT's definite vector allocates a **default-initialized** replacement
array when it grows, before copying the existing elements and `New_Item`. A
client-side probe over an existing generated carrier isolated it exactly:

```text
empty vector created
append 1 ok, length = 1
FAILED: Program_Error from container storage
raised PROGRAM_ERROR : probe.adb:5 finalize/adjust raised exception
```

The first insertion into an unallocated vector initializes directly from
`New_Item` and survived; the second, which required capacity growth, did not.

**B. Bounded.** The representation was a logical `Length` initialized to the
schema minimum plus an `Items` array sized to the schema maximum. For a `0..N`
field, `Length = 0` did **not** prevent default initialization of all `N`
physical carrier slots, so a valid *empty* sequence raised. This was confirmed
in a containing record with no unrelated required field, so the failure is
attributable to unused repeated storage and to nothing else.

Both the Record-field and the Choice-alternative emission paths were affected,
and both are corrected. Fixing the unbounded vector did **not** fix the bounded
array; they are separate representations and were separately corrected.

**C. A separate, pre-existing compile-time defect.** While reproducing A, the
unbounded helper was found not to *compile at all* whenever the element type is
a generated `private` type: a generic instantiation may not use a private type
before its full declaration.

```text
test-carriers.ads:53:47: error: premature use of private type
```

This is reported separately and explicitly **not** mislabelled as the
vector-growth failure. It is not new to Task 040 -- it reproduces on original
`main` (`2d97e46`) for a Task 033 constrained-float element -- but it is fixed
by the same storage correction, so it is recorded rather than deferred.

**D. Bounded occupancy (second corrective pass).** The first corrective pass's
bounded shape was itself defective, and the defect was reproduced by execution
on the first corrective head (`c726b9e`) before any further production code was
changed. A probe built from this repository's own generator printed:

```text
A: length is 1 with slot 1 used = FALSE
B: default length is 2 slot1 used = FALSE slot2 used = FALSE
C: fixed length is 2 slot1 used = FALSE
D: length is 2 but slot 1 used = FALSE
```

* **A** — a zero-minimum `0 .. 3` sequence was default-created, its `Length`
  set to 1 with no element supplied, and it then claimed one element whose slot
  had no payload.
* **B** — a positive-minimum `2 .. 3` field's *default* construction produced
  `Length = 2` while both required logical positions were unused slots.
* **C** — a fixed `2 .. 2` field behaved identically: required occupancy was
  entirely replaceable by unused slots.
* **D** — an ordinary public mutation removed a payload from a slot still
  counted by `Length`, creating a hole in the logical prefix.

The cause is structural rather than incidental. A numeric range on `Length` is
**not** sufficient once a slot may contain no payload: the count and the
payloads were two independently writable facts with no enforced relationship
between them. The earlier claim in this document that a range-constrained
`Length` alone preserved actual occurrence semantics is therefore **withdrawn**
and corrected in §10.4.

### 10.4 The corrected representations

**Unbounded** storage moves to `Ada.Containers.Indefinite_Vectors`, and the
instantiation moves into the **private part** behind an opaque sequence type:

```ada
   type Payload_Any_Sequence is private;

   function Length (Container : Payload_Any_Sequence) return Natural;
   procedure Append
     (Container : in out Payload_Any_Sequence; New_Item : Payload_Any_Item);
   function Element
     (Container : Payload_Any_Sequence; Index : Positive)
      return Payload_Any_Item;
   procedure Clear (Container : in out Payload_Any_Sequence);
   procedure Reserve_Capacity
     (Container : in out Payload_Any_Sequence; Capacity : Natural);
```

An indefinite vector holds access values for spare capacity and constructs each
live element from the supplied value, so no carrier is ever default-created.
Moving the instantiation into the private part also resolves 10.3 C.

**Bounded** storage keeps finite backing storage tied to `maxOccurs`, and its
slots still carry no payload component when unused. The second corrective pass
changes *who may write them*. The required invariant is:

> Every logical element has a live, valid payload. Unused capacity is not a
> logical element.

The type is now **private**, with a checked constructor as the only way to
obtain occupancy:

```ada
   subtype EmptyBounded_Slots_Item is Callsign;
   subtype EmptyBounded_Slots_Index is Positive range 1 .. 3;
   type EmptyBounded_Slots_Values is
     array (Positive range <>) of EmptyBounded_Slots_Item;

   type EmptyBounded_Slots_Sequence is private;

   function To_Sequence
     (Values : EmptyBounded_Slots_Values) return EmptyBounded_Slots_Sequence;
   function Length
     (Container : EmptyBounded_Slots_Sequence) return Natural;
   function Element
     (Container : EmptyBounded_Slots_Sequence; Index : Positive)
      return EmptyBounded_Slots_Item;
   procedure Append
     (Container : in out EmptyBounded_Slots_Sequence;
      New_Item  : EmptyBounded_Slots_Item);
   procedure Clear (Container : in out EmptyBounded_Slots_Sequence);
private
   type EmptyBounded_Slots_Slot (Is_Used : Boolean := False) is record
      case Is_Used is
         when False => null;
         when True  => Value : EmptyBounded_Slots_Item;
      end case;
   end record;

   type EmptyBounded_Slots_Array is
     array (EmptyBounded_Slots_Index) of EmptyBounded_Slots_Slot;

   type EmptyBounded_Slots_Sequence is record
      Count : Natural range 0 .. 3 := 0;
      Items : EmptyBounded_Slots_Array;
   end record;
```

The contract this establishes, in the terms of the four reproduced failures:

* **a zero-minimum sequence starts empty** — count zero over entirely unused
  slots, which is a legal value;
* **a positive-minimum sequence cannot claim required elements that were never
  supplied** — its private completion gives `Count` a `raise` expression
  default instead, exactly as Task 040 does for a validated carrier, so a
  default declaration fails with a diagnostic naming the type. A positive count
  over unused slots is not a valid default, and none is emitted;
* **explicit construction from enough valid elements succeeds**, and
  `To_Sequence` raises `Constraint_Error` for too few or too many values.
  Nothing is silently truncated and no position is fabricated;
* **the maximum and minimum apply to actual logical elements** — `Append`
  raises at capacity, and every counted position was built from a supplied
  value;
* **no exposed operation can create a hole** — there is no occupancy setter and
  no public component, so the disagreement in failure D is unrepresentable
  rather than merely discouraged;
* **spare slots do not default-create validated carriers** — the unused variant
  still has no payload component;
* **access outside the logical length is rejected** — `Element` raises for any
  index past `Length`, so spare capacity is unreachable;
* **copying and assignment preserve a coherent sequence**, since the count and
  the slots are copied together as one value.

`Clear` is emitted **only** when `minOccurs = 0`. Where the schema requires at
least `minOccurs` occurrences, an empty sequence is not a legal value of the
type and no operation produces one.

Failure ordering in mutation is explicit: `Append` establishes the new slot
*before* publishing the larger logical length, so a failed element copy cannot
leave a claimed position empty.

Every check is an ordinary `raise` statement, not a predicate or an assertion,
so none of this depends on `-gnata` or on the client's `Assertion_Policy`.

The bounded representation is deliberately **not** replaced by an unconstrained
vector, and no container heap allocation is introduced to represent spare
bounded capacity. Cardinality is still enforced by the type, the array still
cannot exceed `maxOccurs`, and the bounded shape remains allocation-free beyond
whatever the element type itself owns.

The correction is applied uniformly to Record fields and Choice alternatives,
including elements that transitively contain validated carriers.

A **positive-minimum** unbounded field keeps its two-part shape. Its required
prefix is `min` live elements *by definition* -- the schema says at least that
many occur -- so it stays an array of the element type and needs no slot
wrapper. Only its additional portion becomes opaque storage, starting empty.

Nothing is fabricated. No UUID, date, version, or string is invented to fill
unused capacity; no initialization or validation check is suppressed; no
unchecked public constructor is added; no value is discarded or truncated; no
supported occurrence was marked unsupported; and the shared lexical classifier
and its accepted bounds are untouched.

### 10.5 Affected generated APIs, and the shared policy

This is an explicit, documented, compiler-backed API change:

* a zero-minimum unbounded field's `{Owner}_{Member}_Sequence` is now an opaque
  private type manipulated through the five package-level operations above,
  rather than a public `Ada.Containers.Vectors` subtype whose primitive
  operations were directly visible;
* a positive-minimum unbounded field's `Additional` component is now
  `{Owner}_{Member}_Additional`, an opaque storage type, rather than a raw
  `.Vector`;
* a bounded field's `{Owner}_{Member}_Sequence` is now an opaque private type.
  The first corrective pass's `Items (I).Value` read and
  `(Is_Used => True, Value => ...)` write are **gone from the public surface**,
  along with the writable `Length`; a bounded sequence is built with
  `To_Sequence`, extended with `Append`, read with `Element`, and measured with
  `Length`. This is the change that makes the four failures in §10.3 D
  compile errors rather than silent corruption, and it is a breaking API
  change stated as such.

The operation names are owned by **one** shared definition in
`codegen-core::backend_names`, which the renderer, the generated-name
preflight, and readiness analysis all consult. There is no separate backend,
readiness, and generated-name interpretation of the policy. Like the existing
wrapper callables they are *checked* rather than inserted, because Ada
overloads them on the container parameter's type.

Each storage shape publishes only what it really emits, and the shared model
reserves exactly that:

| Shape | Operations |
| --- | --- |
| unbounded | `Length` `Append` `Element` `Clear` `Reserve_Capacity` |
| bounded, `minOccurs = 0` | `Length` `Append` `Element` `Clear` `To_Sequence` |
| bounded, `minOccurs > 0` | `Length` `Append` `Element` `To_Sequence` |

`Reserve_Capacity` is absent from both bounded shapes because bounded capacity
is fixed by `maxOccurs` and there is nothing to reserve; `Clear` is absent from
the positive-minimum shape for the reason given in §10.4. Reserving an
operation a shape never writes would block a legal user declaration, so the
distinction is load-bearing rather than cosmetic.

**The coupling between the shared definition and the emitted text is now
mechanical.** The previous arrangement was a `ADA_SEQUENCE_CALLABLES.len() == 5`
assertion sitting beside hard-coded spellings in the renderer's format strings.
A length assertion cannot detect a rename, a reorder, or a substitution: the
renderer could have published `Size` while preflight reserved `Length` and the
assertion would still have held. Every emitted spelling is now formatted from
the shared per-operation constant itself, and
`emitted_sequence_operations_are_exactly_the_shared_definition` renders real
bounded and unbounded storage and checks set equality of the declared
subprogram names against each shape's shared list, in both directions.

User-derived spellings registered in the shared name model, so a user
declaration can no longer silently collide with one: `{stem}_Additional` for a
positive-minimum unbounded member, and `{stem}_Item`, `{stem}_Index`,
`{stem}_Values`, `{stem}_Slot`, and `{stem}_Array` for a bounded member. The
last two are private-part spellings, but Ada's package is flat, so they occupy
the same single declarative region and are still reserved.

**Callable ownership is derived from actual emissions.** The previous
`ada_sequence_callable_owners()` scanned raw `schema.types` and
`declared_members()`, which disagreed with rendering in four ways. It now walks
the same name-preflight emission plan the rest of the model uses, applying the
effective-member and storage logic, and accounts for:

* **non-emitted abstract Records** — `render_declaration` returns early for an
  abstract Record, so an unused abstract base with an unbounded field emits no
  sequence and therefore no `Append`. A schema legitimately declaring a type
  named `Append` was previously rejected for a callable that would never exist;
* **effective inherited Record fields** — a concrete descendant that inherits
  an unbounded field without declaring one has an empty `declared_members`
  list, so the raw scan attributed the callables to nobody, while Ada really
  emits them under the descendant;
* **effective Choice alternatives**, projected the same way;
* **closed-sum wrappers versus ordinary declarations** — a Task 024 wrapper is
  a discriminated record over descendant *types* and emits no sequence storage
  of its own, so it contributes nothing and each descendant speaks for itself;
* **elided or semantically unrepresentable fields** — a Task 026 absent-only
  field and a field whose abstract value reference cannot be projected generate
  no storage, so they manufacture no callable collision;
* **the requested generation world**, which is threaded through rather than
  assumed.

An `is_abstract` filter alone would have fixed only the first of these. The
existing field- and entity-scoped diagnostic deferral is preserved in both
directions: a field that cannot render manufactures no callable collision, and
unrelated real collisions in the same schema are still reported and still
implicate both sides. Generation preflight, collected collision attribution,
coverage, and service readiness all consult this one function, so they agree by
construction.

Conditional imports are tracked: a schema with an unbounded member now imports
`Ada.Containers.Indefinite_Vectors`. `Binary_Vectors` is unrelated and remains
*definite* -- its element is `Interfaces.Unsigned_8`, an unvalidated scalar with
no rejecting default -- so the two imports are independent and a schema needing
only one does not acquire the other.

Because the storage operations have real bodies, the package-body predicate is
extended: a schema emits an `.adb` when it has a temporal carrier, a
string-profile carrier, an unbounded sequence, **or** a bounded sequence. A
schema with none of the four still emits no `.adb` at all.

The bounded clause is new in the second corrective pass and changes which
fixtures qualify. `track.xsd` — whose `Sensor_Ids` field is `0 .. 8` — now
legitimately acquires a body. That is recorded as a positive control asserting
the body contains exactly the sequence operations and no carrier validator,
rather than quietly dropped from the negative list;
`backend-constrained-floating.xsd` has no repeated member at all and still
emits no `.adb`, which keeps the negative control real.

### 10.6 Transitive composition

Storage is selected from **semantic** information only -- the member's
occurrence shape, and the element's structure -- never from a declaration
spelling such as `Uuid` or `Instant`. The policy is uniform: every repeated
member of a given occurrence shape gets the same representation, whether its
element is a direct lexical carrier, a record that transitively contains one,
or an ordinary scalar.

A uniform policy is chosen deliberately over a "does this element reach a
validated carrier?" analysis. Such an analysis would have to traverse inherited
fields and recursive structures to stay correct, and would make the generated
API depend on a whole-schema reachability property that is invisible at the use
site. The fixture covers the composed case directly: `Tag` is a record whose
required field is a validated carrier, and it is repeated in both a Record
field and a Choice alternative.

### 10.7 Allocation and performance implications

An indefinite vector heap-allocates **per element**. That is a real cost and it
is accepted deliberately: it is the only representation found that constructs
live elements solely from supplied values, and the validated-value invariant is
prioritized over avoiding a per-element allocation. Bounded fields are
unaffected -- the discriminated slot adds a discriminant per slot and no
allocation whatsoever.

The second corrective pass does not change that. Making bounded storage opaque
adds no allocation: the representation is still a fixed-size array of
discriminated slots plus a `Natural` count, sized by `maxOccurs`. No container
heap allocation is introduced merely to represent spare bounded capacity.
Element-owned allocation is a separate matter and is unchanged — a validated
carrier owning an `Unbounded_String` still owns it.

`To_Sequence` copies each supplied value into its slot, which is the same
per-element copy any construction would perform; it performs no allocation of
its own.


### 10.8 Compiler-tested versus inferred

**Compiler-and-runtime tested**, GNAT 14.2.0, built without `-gnata` and again
under `pragma Assertion_Policy (Ignore)`:

* the original scalar/optional controls still hold -- a default-declared
  carrier fails explicitly, `Create`/copy/assignment succeed, an absent
  optional wrapper defaults successfully, and an unchecked *present* payload
  still fails;
* valid empty zero-minimum sequences, bounded and unbounded;
* appending valid values, and growth through many capacity boundaries (64
  appends, well past several reallocations);
* `Reserve_Capacity` on both empty and populated containers;
* copy construction, assignment, `Clear`, and reuse of populated sequences;
* positive-minimum required values plus additional elements;
* empty, partial, and full bounded sequences, with the cardinality limit
  enforced by the **generated sequence itself**. The previous probe declared a
  local `Over : Natural range 0 .. 3` and observed *its* `Constraint_Error`,
  which proved nothing about the generated API; that substitute has been
  replaced by a real `Append` at capacity;
* bounded `2 .. 3` and fixed `2 .. 2` shapes: valid explicit construction
  succeeds (the positive control), construction with too few or too many values
  raises, `Append` past `maxOccurs` raises, reading past the logical length
  raises, and a default declaration of a positive-minimum sequence raises
  rather than claiming unsupplied elements;
* bounded copy, assignment, `Clear`, and the fact that a cleared sequence
  yields nothing on read;
* the four §10.3 D disagreements as **compile errors**: with the type private
  there is no `.Length` selector and no `.Items` component, so
  length/slot disagreement is unrepresentable rather than merely rejected at
  run time. Positive controls proving valid construction still compiles and
  executes are retained alongside;
* composed element types containing validated carriers;
* repeated Choice-alternative composition, in both storage shapes;
* the DateTime carrier compared against its expected **normalized**
  representation;
* the same through `service-generate`, constructing and manipulating a real
  repeated field rather than merely compiling the generated spec.

Every live value is inspected for its exact expected text after the operation.

**Inferred, not tested.** That *any* conforming Ada implementation allocates
identically. The probes deliberately assert the required **usable API
behavior** -- a client can build and read back these sequences without ever
supplying a value to populate unused capacity -- rather than any particular
allocation strategy, because allocation behavior is implementation-defined.

### 10.9 Fresh measurements

The pinned UCI 2.5 inputs were retried through the established source workflow
and were obtainable this time, so the previously missing cells are now measured
rather than restated.

Full twelve-cell comparison, re-measured on the corrected head of the second
pass, same pinned inputs and same binary invocation:

| Cell | Ada | Rust | C++ | Delta |
| --- | ---: | ---: | ---: | ---: |
| 2.5 closed (of 5557) | 5314 | 5391 | 5394 | **0 / 0 / 0** |
| 2.5 open (of 5557) | 5229 | 5303 | 5306 | **0 / 0 / 0** |
| 2.6 closed (of 5570) | 5335 | 5413 | 5417 | **0 / 0 / 0** |
| 2.6 open (of 5570) | 5250 | 5325 | 5329 | **0 / 0 / 0** |

All twelve cells are unchanged from the first corrective head and from the Task
039 baseline. Kinds (5444/5557 for 2.5, 5457/5570 for 2.6), field-types,
field-occurrences, and message-closures are likewise unchanged in every cell.

**The zero delta was verified rather than assumed, and it is the expected
result.** The emission-aware callable correction is a *name-analysis*
correction, and a legitimate one can move attribution, so this was checked
rather than asserted. It does not move any UCI cell because the correction only
ever *removes* phantom reservations and *adds* effective-inheritance ones, and
pinned UCI contains no declaration spelled `Length`, `Append`, `Element`,
`Clear`, `Reserve_Capacity`, or `To_Sequence` for either direction to act on.
The bounded-storage change is a representation change, not a capability change:
no occurrence shape moved between supported and unsupported, and no validator
or name check was weakened to hold the totals.

UCI 2.5 `PositionReport` selection recheck, closed world, now freshly measured:

| Backend | Renderable selected | First blocker |
| --- | ---: | --- |
| Ada | 51/60 | `SecurityInformationType` |
| Rust | 55/60 | `SecurityInformationType` |
| C++ | 55/60 | `SecurityInformationType` |

Unchanged, and no lexical support or occurrence limit was altered to obtain
that.

Three distinct properties are kept separate:

1. **Deterministic generated output across repeated runs** -- two consecutive
   generations of the same fixture are byte-identical;
2. **Unchanged coverage-report output before/after** -- the reviewed-head and
   corrected-head reports diff clean;
3. **Intentionally changed Ada generated source** -- the storage
   representations above genuinely changed, and the golden `track.ads` and the
   backend text assertions are updated to match. Rust and C++ generated output
   is byte-identical before and after, verified by directory diff.

### 10.10 What the corrective did not touch

The C++ lifecycle correction and its tests are unchanged, as are the shared
lexical corpora under `tests/fixtures/string/` and `tests/fixtures/temporal/`.
Rust generated output is unchanged. No classifier, no accepted lexical space,
no normalization rule, and no occurrence limit was modified.
