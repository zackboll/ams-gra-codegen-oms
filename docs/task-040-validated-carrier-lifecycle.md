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

Test counts: **785** passing before this task, **787** after, with
`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`. The GNAT-backed regressions
execute rather than skip: `AMS_GRA_REQUIRE_GNAT` turns the developer-machine
skip into a hard failure, and the existing CI gate is unchanged.

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
- **Generated-name preflight** behavior is preserved. The Ada change is a
  component default inside an existing private record and introduces no
  package-scope name, so nothing new is owed to the shared generated-name
  model and no earlier collision or optional-storage regression is
  reintroduced.
- **Unrelated generated outputs** are unchanged.

### Verification limits

The pinned **UCI 2.6** set was available locally and was measured, as above.
The pinned **UCI 2.5** set (`093610b7753944059360d3236770ab446d039556`) was
**not** obtainable in this environment. Consequently:

- the six UCI 2.5 cells of the twelve-cell comparison are **not** freshly
  measured here;
- the UCI 2.5 `PositionReport` selection recheck (Ada 51/60, Rust 55/60,
  C++ 55/60, first blocker `SecurityInformationType`) is **not** freshly
  measured here.

That is recorded as missing evidence rather than restated from existing
documentation as though it were a fresh authoritative measurement. Readiness is
*expected* to be unchanged, because nothing in this task alters which
declarations are renderable and the measured 2.6 cells moved by exactly zero;
but expectation is not measurement, and this limitation is stated explicitly.

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
