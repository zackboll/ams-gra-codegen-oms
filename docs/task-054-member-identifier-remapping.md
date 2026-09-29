# Task 054 — Remap reserved Record and Choice member identifiers

Branch `feature/054-member-identifier-remapping`, from `main` at
`d04ca9ca67a2aa5c27cc41512de73bc1e59e997c` (Task 053 / PR #54).

Since Task 044 the full-schema first failures on both pinned UCI releases were
host-language identifier failures on **structural members**:

| Release | Ada | Rust | C++ |
| --- | --- | --- | --- |
| 2.5 | `AltitudeRangePairType.Range` | `ConfigurationParameterType.Type` | `ApprovalResponseType.Operator` |
| 2.6 | `AltitudeRangePairType.Range` | `ConfigurationParameterType.Type` | `COMINT_ChangeDwellType.Delete` |

Task 054 fixes exactly that boundary: an emitted Record field or Choice
alternative whose ORDINARY generated identifier is a target reserved word gets
one fixed escape category. It is not a sanitizer, and it does not rename
top-level declarations, message names, service API IDs, enumeration values or
wire names.

## 1. Three identities, never conflated

| Concept | Source of truth | Example (`Type`) |
| --- | --- | --- |
| source member identity | `FieldDecl.name` (XSD local name, Schema IR) | `Type` |
| wire member identity | `FieldDecl::wire_name()` -> `member_key` | `"Type"` (OAM) / `"{urn:test}Type"` |
| backend API identifier | `generated_record_field_name(language, name)` / `generated_choice_alternative_name(language, name)` | Ada `Field_Type`, Rust `field_type`, C++ `type` |

Schema IR is never modified: `FieldDecl.name` stays `"Type"` and
`wire_name().local_name` stays `"Type"` (Task 051 semantics unchanged). No
escaped spelling is stored in IR, docs, or on the wire.

## 2. Fresh pinned-UCI member inventory (recorded BEFORE production changes)

Roots fetched with `scripts/fetch-pinned-uci-2.5.sh` (v2.5 @ `093610b7...`,
root SHA-256 `ac943049...`) and `scripts/fetch-pinned-uci-2.6.sh` (v2.6 @
`78eb61b6...`, root SHA-256 `af54ce72...`), outside the repository.

The inventory is `codegen-core::structural_member_name_inventory`: it walks the
same shared `name_preflight_plan`, storage elision (`field_storage_semantics`)
and abstract-value check that generated-name preflight uses, so every counted
member is one a renderer actually writes. It records, per member: owner QName,
source name, Record vs Choice, inherited vs local, ordinary candidate, syntax
legality, reserved-ness, final candidate, escape legality and final-region
collision. Before any backend change the *final* column was computed but not
yet consumed; the table below is that pre-change run (raw log kept outside the
repository as `inventory-pre-change.log`), and is asserted exactly by
`crates/cli/tests/uci_member_names.rs`.

| Release | World | Backend | Record-field occ. | Choice-alt. occ. | inherited | syntax-invalid | reserved | escaped | escape illegal | post-remap collisions |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2.5 | closed | Ada | 17614 | 1468 | 6037 | 0 | 58 | 58 | 0 | 0 |
| 2.5 | closed | Rust | 17614 | 1468 | 6037 | 0 | 10 | 10 | 0 | 0 |
| 2.5 | closed | C++ | 17614 | 1468 | 6037 | 0 | 7 | 7 | 0 | 0 |
| 2.5 | open | Ada | 17551 | 1468 | 6025 | 0 | 58 | 58 | 0 | 0 |
| 2.5 | open | Rust | 17551 | 1468 | 6025 | 0 | 10 | 10 | 0 | 0 |
| 2.5 | open | C++ | 17551 | 1468 | 6025 | 0 | 7 | 7 | 0 | 0 |
| 2.6 | closed | Ada | 17778 | 1482 | 6178 | 0 | 58 | 58 | 0 | 0 |
| 2.6 | closed | Rust | 17778 | 1482 | 6178 | 0 | 10 | 10 | 0 | 0 |
| 2.6 | closed | C++ | 17778 | 1482 | 6178 | 0 | 6 | 6 | 0 | 0 |
| 2.6 | open | Ada | 17715 | 1482 | 6166 | 0 | 58 | 58 | 0 | 0 |
| 2.6 | open | Rust | 17715 | 1482 | 6166 | 0 | 10 | 10 | 0 | 0 |
| 2.6 | open | C++ | 17715 | 1482 | 6166 | 0 | 6 | 6 | 0 | 0 |

The two worlds differ only by 63 Record-field occurrences that
`open-extensions` elides (Task 026 absent-only storage / unrepresentable open
abstract values); the unsafe spellings are identical.

### Distinct unsafe source spellings (identical in 2.5 and 2.6, both worlds)

| Backend | Kind | Source -> ordinary => final |
| --- | --- | --- |
| Ada | Record field | `All`, `Begin`, `End`, `Function`, `Range`, `Task`, `Type`, `When` -> same => `Field_<Source>` |
| Ada | Choice alternative | `All`, `And`, `Body`, `Not`, `Or`, `Range`, `Task` -> same => `Alternative_<Source>` |
| Rust | Record field | `Type` -> `type` => `field_type`; `Yield` -> `yield` => `field_yield` |
| Rust | Choice alternative | none (no UCI alternative upper-camels onto `Self`) |
| C++ | Record field | `Auto` -> `auto` => `field_auto`; `Delete` -> `delete` => `field_delete`; `Friend` -> `friend` => `field_friend`; `Operator` -> `operator` => `field_operator` |
| C++ | Choice alternative | none (C++ keywords are lowercase; every upper-camel alternative is legal) |

(2.6 C++ has one fewer reserved *occurrence* than 2.5 - 6 vs 7 - with the same
four spellings.)

### Evidence gate

Every pinned failure is "ordinary candidate syntactically valid BUT target
reserved". **Zero** members in any cell have a syntactically invalid ordinary
candidate (no punctuation, leading-digit, or malformed member names exist in
pinned UCI), every escaped candidate is legal, and there are **zero**
post-remap collisions. The gate is satisfied, so the task proceeds without any
sanitizer widening.

## 3. The shared naming rule (`codegen-core::backend_names`)

```rust
pub fn generated_record_field_name(language, source_name) -> Option<String>;
pub fn generated_choice_alternative_name(language, source_name) -> Option<String>;
```

Both delegate to one private `generated_member_name(language, kind, source)`:

1. Form the ORDINARY candidate with the target's existing rule - Ada
   verbatim identifier; Rust/C++ `snake_case` for Record fields and
   `UpperCamel` for Choice alternatives. These are the same helpers
   preflight already used, so safe spellings are byte-identical.
2. Syntactically invalid candidate -> `None` (fail closed, exactly as before).
3. Legal and not reserved -> returned unchanged.
4. Legal and reserved -> prefix the fixed category to the **source** spelling
   (`Field_` for Record fields, `Alternative_` for Choice alternatives) and
   re-apply the SAME ordinary rule, so the casing comes from one transform:
   Rust `Field_Type` -> `field_type`, `Alternative_Self` -> `AlternativeSelf`;
   Ada `Field_Range`; C++ `Field_Operator` -> `field_operator`.
5. The escaped candidate is validated (legal and not reserved), else `None`.

No numeric suffix, counter, hash, raw Rust identifier, leading/trailing
underscore scheme, or backend-specific replacement exists. Collisions are
never resolved; they are reported.

| Language | Record `Range` | Record `Type` | Record `Operator` | Record `Delete` | Record `Self` | Choice `Range` | Choice `Self` | Choice `Operator` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Ada | `Field_Range` | `Field_Type` | `Operator` | `Delete` | `Self` | `Alternative_Range` | `Self` | `Operator` |
| Rust | `range` | `field_type` | `operator` | `delete` | `field_self` | `Range` | `AlternativeSelf` | `Operator` |
| C++ | `range` | `type` | `field_operator` | `field_delete` | `self` | `Range` | `Self` | `Operator` |

Only the ACTUAL generated candidate decides: C++ Choice `Operator` is a legal
nested type and is unchanged; Rust `Range` is not a keyword.

**Task 044 is separate.** Enumeration variants keep
`generated_enum_variant_name` (the `Value_` category, leading-digit repair);
the Task 054 helpers are never used for enumerations and never repair leading
digits.

### Authoritative consumers

The helpers are the ONLY source of Record-field / Choice-alternative
identifiers:

* generated-name preflight (`register_declaration_members`) registers the
  FINAL spelling in the member region; Ada helper stems (`validate_ada_helpers`,
  `validate_ada_optional_helper`) and Ada Choice literals (`{Final}_Kind`) are
  derived from the final spelling, while `NameSource::Member` keeps the SOURCE
  name for attribution;
* `backend-ada` (`record_field_name` / `choice_alternative_name`): Record
  components, `{Owner}_{Final}_Optional`, bounded/unbounded helper stems,
  Choice discriminant literals, default discriminant, case arms and Choice
  sequence storage type;
* `backend-rust` (`record_field_name` / `choice_alternative_name`): struct
  fields and enum variants;
* `backend-rust::service_codec`: host member access (`value.field_type`,
  `KeywordChoice::AlternativeSelf`), from the SAME two `pub(crate)` functions
  the model renderer uses; the wire key is always `member_key(field)`;
* `backend-cpp` (`record_field_name` / `choice_alternative_name`): data members,
  nested alternative structs and the `std::variant<...>` list.

No direct `snake_case(&field.name)`, `upper_camel(&alternative.name)` or
`ada_identifier(&field.name)` remains at any member rendering site.

### Preflight is final-name based

For Record/Choice regions preflight now registers the final generated
spelling, not the source name or the unsafe ordinary candidate. Consequences:

* a reserved candidate becomes renderable;
* a **post-remap collision** is detected against the final spelling and fails
  closed, e.g. Rust `Type` + `Field_Type` both `field_type`; Ada `Range` +
  `Field_Range` (or `field_range`, case-insensitive) both `Field_Range`; C++
  `Operator` + `Field_Operator` both `field_operator`. The diagnostic keeps the
  backend language, owning declaration, both SOURCE member names and the final
  spelling:
  `Rust names "Type" and "Field_Type" both generate "field_type" in the members of LocalClash`;
* collision analysis runs over EFFECTIVE members, so an escaped inherited
  member colliding with a local one condemns the emitted descendant (and never
  the non-emitted abstract base);
* generated fixed names stay independent: the Ada discriminant `Kind` is not a
  reserved word, so an alternative named `Kind` still collides; escaped helper
  stems (`Owner_Field_Range_Sequence`, `Owner_Field_Type_Optional`) and
  literals (`Alternative_Range_Kind`) collide with a same-spelled user
  declaration exactly like any other helper.

## 4. Generated lowering (synthetic `member-keywords.xsd`)

`tests/fixtures/service-generate/member-keywords.xsd` (OAM, not UCI):
`KeywordRecord` extends abstract `KeywordBase` (inherited `Type`) with
`Range` (0..2), `Operator`, `Delete` (optional), `Self`, `Ordinary`, and
`Pick : KeywordChoice`; `KeywordChoice` has `Range` (1..3), `Self`, `Type`,
`Delete`, `Ordinary`.

Ada:

```ada
type KeywordChoice_Kind is
   (Alternative_Range_Kind, Self_Kind, Alternative_Type_Kind, Delete_Kind, Ordinary_Kind);
type KeywordChoice (Kind : KeywordChoice_Kind := Alternative_Range_Kind) is record
   case Kind is
      when Alternative_Range_Kind => Alternative_Range : KeywordChoice_Alternative_Range_Sequence;
      ...
type KeywordRecord is record
   Field_Type  : Unbounded_String;                    --  inherited
   Field_Range : KeywordRecord_Field_Range_Sequence;  --  repeated, helper stem escaped
   Operator    : Unbounded_String;
   Delete      : KeywordRecord_Delete_Optional;       --  Delete is not an Ada keyword
   Self        : ...;
```

Rust:

```rust
pub enum KeywordChoice { Range(..), AlternativeSelf(..), Type(String), Delete(bool), Ordinary(..) }
pub struct KeywordRecord { pub field_type: String, pub range: ..., pub operator: String,
    pub delete: Option<bool>, pub field_self: ..., pub ordinary: ..., pub pick: KeywordChoice }
```

C++:

```cpp
struct KeywordChoice { struct Range {..}; struct Self {..}; struct Type {..}; struct Delete {..}; struct Ordinary {..};
                       std::variant<Range, Self, Type, Delete, Ordinary> value; };
struct KeywordRecord { std::string type; ... range; std::string field_operator;
                       std::optional<bool> field_delete; ... self; ... ordinary; KeywordChoice pick; };
```

## 5. Evidence

### Compilers (`crates/cli/tests/member_remapping.rs`)

Every expected spelling in these probes is computed by the shared helpers, so
no test asserts an escape the reserved-word table does not justify.

* **Ada** (`task054_ada_escaped_members_compile_and_run`): assigns and reads
  every escaped component (inherited `Field_Type`, repeated `Field_Range`
  through `To_Sequence`/`Length`/`Element`, optional `Delete` wrapper),
  constructs every Choice alternative by discriminant, `-gnat2012 -gnatwa
  -gnata`. Passed under local GNAT 14.2.0 and GNAT 13.2.0 (Alire
  `gnat_native_13.2.1`, via `AMS_GRA_GNAT13_GNATMAKE`).
* **Rust** (`task054_rust_escaped_members_compile_and_run`): no `r#`
  anywhere; builds every escaped field/variant and runs, under
  `rustc --edition=2024 -D warnings`.
* **C++** (`task054_cpp_escaped_members_compile_strictly_and_run`):
  `field_operator`, `field_delete`, inherited `type`, every legal upper-camel
  alternative, `-std=c++17 -Wall -Wextra -Werror -pedantic-errors`, run.

### Readiness parity

| Case | preflight | coverage | service-check | generation | compiler |
| --- | --- | --- | --- | --- | --- |
| reserved member (`member-keywords`) | ok | renderable | READY (x3, + Rust codec READY) | succeeds | compiles, runs |
| post-remap collision, local (`member-collision` / `LocalClash`) | rejects | unsafe owner | NOT READY (Rust, Ada) | fails, no output | n/a |
| post-remap collision, inherited (`InheritedClash`) | rejects | unsafe descendant | NOT READY (C++, Ada) | fails, no output | n/a |
| malformed (`member-malformed`: `has-dash`, `has.dot`) | rejects | unsafe | NOT READY (x3) | fails, no output | n/a |

`task054_collisions_are_decided_by_the_actual_candidate` is the control: the
same collision schema is READY where the candidate is not reserved (C++ has no
`type` keyword; Rust has no `operator`/`range` keyword).

### Malformed names stay unsupported

`1Bad`, `has-dash`, `space name`, `café`, `_`, empty, and Ada
`Double__Underscore` / `Trailing_` return `None` from both helpers and are
`InvalidIdentifier` in preflight and coverage, even though a prefix would make
some compile. Task 054 does not broaden beyond its evidence.

### Rust codec: API name versus wire name

`crates/runtime-rust-facade-tests/tests/generated_codec_member_remapping.rs`
uses the real `service-generate --with-codec` output (`build.rs`):

* OAM `member-keywords`: a value built through `field_type`, `field_self` and
  `KeywordChoice::AlternativeSelf` encodes to exactly
  `{"Type":..,"Range":..,"Operator":..,"Delete":..,"Self":..,"Ordinary":..,"Pick":{"Self":3}}`
  and decodes back to an equal value; no escaped spelling appears in the JSON.
  A document keyed `field_type`, or a Choice keyed `AlternativeSelf`, is
  rejected (`unknown member`): host names are not wire aliases.
* Qualified non-OAM `member-keywords-qualified` (`urn:test`, Task 051):
  `field_type` encodes as `"{urn:test}Type"` (never
  `"{urn:test}field_type"`), `AlternativeSelf` as `"{urn:test}Self"`, and
  round-trips; the same service also round-trips through the generated
  facade, runtime-rust, pinned sleet-client and a mock OWP peer (pinned Sleet
  has the already-known non-OAM limitation, so mock OWP is the evidence).

## 6. Pinned UCI evidence after Task 054

Inventory (`crates/cli/tests/uci_member_names.rs`, exact expectations, all
cells: zero syntax-invalid members, every reserved candidate escaped to a
legal spelling, zero post-remap collisions):

| Release | World | Record fields | Choice alts | Inherited | Ada reserved | Rust reserved | C++ reserved |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2.5 | closed-schema | 17614 | 1468 | 6037 | 58 | 10 | 7 |
| 2.5 | open-extensions | 17551 | 1468 | 6025 | 58 | 10 | 7 |
| 2.6 | closed-schema | 17778 | 1482 | 6178 | 58 | 10 | 6 |
| 2.6 | open-extensions | 17715 | 1482 | 6166 | 58 | 10 | 6 |

### 12-cell coverage (declarations / message closures), before -> after

| Release / world | Ada | Rust | C++ |
| --- | --- | --- | --- |
| 2.5 closed | 5368 -> 5419; 324 -> 355 / 722 | 5410 -> 5420; 354 -> 355 | 5413 -> 5420; 351 -> 355 |
| 2.5 open | 5283 -> 5332; 317 -> 347 | 5322 -> 5332; 347 -> 347 | 5325 -> 5332; 344 -> 347 |
| 2.6 closed | 5389 -> 5440; 323 -> 355 / 725 | 5431 -> 5441; 354 -> 355 | 5435 -> 5441; 352 -> 355 |
| 2.6 open | 5304 -> 5353; 316 -> 347 | 5343 -> 5353; 347 -> 347 | 5347 -> 5353; 345 -> 347 |

After Task 054 the three backends agree in every cell (declaration counts
differ only by Ada's one pre-existing extra blocker in closed-schema); no cell
regressed. Field-occurrence coverage is unchanged.

### Whole-schema first blockers (`generate`), before -> after

| Cell | Before | After (pre-existing, previously masked) |
| --- | --- | --- |
| 2.5/2.6 closed Ada | `AltitudeRangePairType.Range` | `QueryType companion` / `QueryType` both generate `QueryType_Kind` (top-level scope) |
| 2.5/2.6 closed Rust | `ConfigurationParameterType.Type` | abstract value target `SourceCommandEXT` has no concrete structural descendants |
| 2.5/2.6 closed C++ | 2.5 `ApprovalResponseType.Operator`, 2.6 `COMINT_ChangeDwellType.Delete` | `SourceCommandEXT` (as Rust) |
| 2.5/2.6 open (all) | as closed | abstract value `CapabilityCommandBaseType` is not closed under open-extensions |

None of the after blockers is a structural-member name; Task 054 does not fix
them. They are asserted exactly by `task054_real_uci_whole_schema_first_blockers`.

### Message impact (per-message service-check, closed-schema)

Messages whose pre-Task-054 blocker set included a reserved-member owner:

| Release | Backend | A: now READY | B: still blocked by another boundary | C: topology error (unchanged) |
| --- | --- | --- | --- | --- |
| 2.5 | Ada | 31 | 230 | 46 |
| 2.5 | Rust | 1 (`OrderOfBattle`) | 66 | 46 |
| 2.5 | C++ | 4 (`ApprovalRequestStatus`, `EntityNotification`, `OpNotification`, `SystemNotification`) | 33 | 46 |
| 2.6 | Ada | 32 | 234 | 46 |
| 2.6 | Rust | 1 (`OrderOfBattle`) | 66 | 46 |
| 2.6 | C++ | 3 (`EntityNotification`, `OpNotification`, `SystemNotification`) | 32 | 46 |

Dominant B next blockers: `DurationType` (xs:duration), `AircraftIdentifierType`
(constrained string facet profile), `EphemerisOrbitalModelType`,
`AlphanumericDashSpaceUnderscoreStringLength15Type`, `EmptyType`. Topology
errors are identical before and after. After Task 054 the READY message sets
are equal across backends (355 in each cell of 2.5/2.6 closed).

### Category-A generation (single-message contracts, 2.5 closed)

Each of the 35 category-A messages was generated before/after in all three
backends (210 runs): Ada 4 -> 34 generate, Rust 34 -> 34, C++ 30 -> 34; zero
regressions. The single exception is `OrderOfBattle`:

> **Open item (pre-existing, not Task 054):** `OrderOfBattle` is READY in
> service-check but `service-generate` fails on `AircraftIdentifierType`
> (Rust/C++) and `xs:duration` (Ada). Those declarations enter only through
> generated-support (abstract-value descendant) expansion, which readiness
> does not measure against the same rules. C++ already shows the identical
> failure on the pre-Task-054 binary. Not fixed here.

> **Follow-up (Task 056).** The table and counts above are preserved as
> Task 054's historical evidence. Task 056 corrected the readiness false
> positive this item describes: readiness now also measures the projection's
> generated-support declarations, so `OrderOfBattle` is no longer category-A /
> READY. It is NOT READY in all three backends (pinned 2.5 and 2.6,
> closed-schema) before any backend call, with its 55 (2.5) / 56 (2.6) selected
> types still all renderable and 42 / 40 of its 442 generated-support types
> reported under `unsupported generated support types:`. The other 34
> category-A messages remain READY and still generate. The exposed blockers
> (`AircraftIdentifierType`, `xs:duration`, ...) are not implemented. See
> [Task 056](task-056-generated-support-readiness.md).

`real-member-remapping.yaml` therefore selects `SystemOrbitalElementSetRequest`
(Ada `DateTimeRangeType.Begin`/`End` -> `Field_Begin`/`Field_End`, helper
stems `DateTimeRangeType_Field_Begin_Optional`) and `ApprovalRequestStatus`
(C++ `ApprovalResponseType.Operator` -> `field_operator`). With the pinned 2.5
root it is READY and generates in all three backends (and the Rust codec
generates); `task054_real_uci_category_a_selection_generates_and_compiles`
checks it with GNAT `-gnatc`, `rustc -D warnings`, and strict C++17. No pinned
category-A closure reaches a Rust-reserved member.

## 7. Regressions

* PositionReport (real 2.5): generated files and service-check output
  byte-identical before/after in Ada, Rust and C++.
* Every pre-existing synthetic `service-generate` fixture schema (177
  schema x backend combinations): generated output byte-identical.
* Task 044 enum remapping (`Value_`) unchanged
  (`task054_enum_remapping_is_independent` plus the Task 044 suite).
* Task 053: `constrained_binary` (Ada, 2 passed),
  `generated_constrained_binary` (Rust codec, 4 passed), and the real-UCI
  constrained-Binary inventories (2.5, 2.6) pass unchanged.
  `task053_real_uci_constrained_binary_message_impact` needed one expected
  update. Its Ada first blockers were the reserved-member owners
  `AltitudeRangePairType` (IFF_Activity, IFF_Command) and `DateTimeRangeType`
  (ProductMetadata). Ada now reports the same next blocker as Rust/C++
  (`AircraftIdentifierType`, `AlphanumericStringLength4Type`). The A/B/C
  classes are unchanged (0 / 4 / 1 in both releases), and no constrained
  Binary is a blocker.

## 8. Versions

GNAT 14.2.0 (and 13.2.0 via Alire `gnat_native_13.2.1`), rustc 1.98.1 plus the
1.95.0 MSRV gate, g++ (Debian) 14.2.0 `-std=c++17`. Pinned UCI 2.5 `093610b7` (root
`ac943049...`) and 2.6 `78eb61b6` (root `af54ce72...`).

## 9. Validation

Baseline on `d04ca9ca`: 1104 workspace tests passed. After Task 054:
`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` passes 1128 (0 failed, 0
ignored; +24 = 9 core, 7 CLI, 4 codec, 4 pinned-UCI env-gated which skip
without the roots). `cargo fmt --check`, `cargo check --workspace
--all-targets`, `cargo clippy --workspace --all-targets -D warnings`, and
`git diff --check` are clean. The pinned-UCI tests were run with both roots
(`uci_member_names`: the inventory and first-blocker tests, 3 passed; the
category-A test, 1 passed).
