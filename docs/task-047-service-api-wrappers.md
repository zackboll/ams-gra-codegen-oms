# Task 047 — generated typed Service API wrappers

Task 047 implements the Phase 2.5 roadmap item **service-specific generated
wrapper APIs**. For a READY Service Contract, `service-generate` now emits:

1. the existing contract-selected UCI type model files, **unchanged**; and
2. one generated **service API wrapper entrypoint** for the requested
   language: `service_api.rs`, `service_api.hpp`, or `service_api.ads`.

> Task 047 generates typed endpoint descriptors. It does not send, receive,
> encode, decode, subscribe, publish, dispatch, or connect to CAL.

There are no sockets, WebSockets, CAL or Sleet calls, JSON codecs, threads,
callbacks, publishers, subscribers, or dispatchers. The wrapper is a
compile-time API description. A later task can add runtime operations against
this stable generated surface instead of reinterpreting the Service Contract.

> Task 047 generates the typed endpoint surface, not a serialized mirror of the
> entire portable contract.

## Authority boundary

```text
portable Contract
      |
      v
  ServicePlan                (Task 030: contract x schema join)
      |
      v
  ServiceApiModel            (Task 047: codegen-core, lowered ONCE)
      |
      +----------+----------+
      |          |          |
     Ada        Rust       C++   (backends render syntax only)
   wrapper     wrapper    wrapper
```

* The raw portable `Contract` is never passed to a backend. Backends do not
  parse YAML or know Service Contract validation rules.
* `codegen-core::build_service_api_model(plan, projected_schema, world)` is the
  single place the plan is lowered. Backends receive the resulting
  `ServiceApiModel` plus the projected `SchemaIr` they already generate, and
  nothing else: no YAML, no `Contract`, no filesystem path, no profile engine.
* Backends never reinterpret function applicability, direction, mandate,
  exchange kind, OMS message identity, topic, or contract ordering. They spell
  already-decided facts in their own syntax.
* The CLI orchestrates. It contains no language-specific source generation.

`Backend` gained one method:

```rust
fn generate_service_api(
    &self,
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<Vec<GeneratedFile>, CodegenError>;
```

`schema` is used only to spell the model's module/namespace/package and each
payload's generated type name with each backend's **existing** rules
(`model_file_name`/`rust_type`, `namespace_name`/`cpp_type`,
`package_name`/`ada_type`), so the wrapper can never drift from what type
generation emitted.

## `ServiceApiModel`

The model preserves **contract order** and keeps **every exchange occurrence**.

| Level | Retained |
| --- | --- |
| service | name, version, kind, and whether a type model is emitted |
| function | `id`, human `name`, exchanges in contract order |
| exchange | `id`, kind, direction, mandate |
| OMS exchange only | topic, resolved message `QualifiedName`, resolved payload `TypeRef` |

All five exchange variants are represented: `oms_message`, `data_transfer`,
`special_signal`, `security_exchange`, `non_oms_message`. Only `oms_message`
carries a payload binding; none is fabricated for the other four.

Exchanges are **not deduplicated**. Two exchanges that select the same UCI
message but differ in function, direction, topic, or mandate remain two
endpoints. This is deliberately distinct from
`ServicePlan::selected_messages()`, which still deduplicates messages for the
type closure.

Lowering copies the plan's resolved message and payload identities verbatim.
It never re-resolves a message name, searches `SchemaIr` by local name, infers
a topic or direction, or regroups exchanges. The only new check is that each
OMS payload really is a named declaration the **projected** model emits as its
own generated top-level type. Failures are typed
(`ServiceApiError::UnboundPayload`): a primitive payload (no generated type
exists to alias, and none is invented), a message the projected schema does
not carry unchanged, or a payload that is not a generated type.

### What is intentionally not exposed yet

These remain in `ServicePlan`, unmodified, for a later façade/runtime task:
traceability, `operational_attribute`, `subscription_group`,
`appendix_c_mapping`, timing parameters, Capability ownership,
`standard_role`, descriptions, Data Transfer protocol/format details, and
Special Signal / Security / Non-OMS `details`/`reference`. The four non-OMS
scopes emit only the common `ID`/`KIND`/`DIRECTION`/`MANDATE` metadata; their
kind-specific metadata is **not** generated in this task.

## Generated identifier policy

Scope names come from portable **IDs only**:

```text
function.id  -> function scope
exchange.id  -> exchange scope (nested inside its function's scope)
```

Never from `function.name`, `service.name`, exchange names, topics, or message
local names. Human names are arbitrary display text and appear only as
escaped string constants. The raw authored ID is always available as the
generated `ID` constant.

One shared, deterministic rule in `codegen-core`
(`service_api_function_scope_name`, `service_api_exchange_scope_name`) splits
the ID on `-` and `_` and prefixes a fixed word:

| Language | Function scope | Exchange scope |
| --- | --- | --- |
| Rust / C++ | `function_<words joined by _>` | `exchange_<words joined by _>` |
| Ada | `Function_<Title_Words>` | `Exchange_<Title_Words>` |

```text
position-input-example -> function_position_input_example / Function_Position_Input_Example
position-report-input  -> exchange_position_report_input  / Exchange_Position_Report_Input
```

Portable IDs match `^[a-z][a-z0-9]*(?:[-_][a-z0-9]+)*$`, so every word is
non-empty lowercase alphanumeric and the prefixed result is always a legal
identifier. The fixed prefix means a valid ID such as `type`, `match`,
`range`, `operator`, or `delete` never becomes a bare host identifier. Task
044's enum `Value_` escape is not used.

This analysis is **separate** from the Schema IR `backend_names` preflight:
schema names come from XSD and wrapper names from contract IDs, which are
different authorities occupying different scopes. It does not use full-UCI
member-name sanitization.

## Collision preflight

`foo-bar` and `foo_bar` are distinct portable IDs that normalize to the same
scope name. They are **never** silently merged, and no numeric suffix is
invented: the wrapper fails closed.

The shared preflight (`validate_service_api_names`, and
`validate_service_plan_api_names` over a plan) models the generated regions
exactly:

| Region | Checked |
| --- | --- |
| file | wrapper root; Rust also the `model` module |
| service root | `SERVICE_NAME`/`SERVICE_VERSION`/`SERVICE_KIND` + every function scope |
| each function | `ID`/`NAME` + every exchange scope |
| each exchange | `ID`/`KIND`/`DIRECTION`/`MANDATE`, plus `TOPIC`/`Payload` for OMS |

Identity is case-insensitive for Ada, exact for Rust and C++. Errors are typed
(`ServiceApiNameError::{InvalidIdentifier, ReservedWord, Collision}`) and name
the language, the logical function/exchange identity, and the generated name.
Every backend re-runs the same preflight before rendering, so a library caller
that skips readiness still fails closed.

The **same exchange ID in different functions is legal** (`function-a` and
`function-b` may each have `status-output`), because exchange scopes nest
beneath different function scopes.

A contract that fails this preflight is still a **valid portable contract**;
`service-plan` accepts it. No naming rule was added to the contract crate.

## Readiness includes the wrapper

After Task 047, `service-check` READY means `service-generate` can produce
**both** the selected type model **and** its wrapper.

`ServiceBackendReadiness` gained
`service_api_blocker: Option<ServiceApiError>`, and `is_ready()` requires it to
be `None`. Wrapper names are always checked (they depend only on IDs and
kinds); payload binding is checked when the type selection is otherwise
ready, so one cause is never reported twice. Counts are **never** changed and
no fake unsupported UCI declaration is invented. The report adds one distinct
line:

```text
status: NOT READY

service api boundary: Ada service API names for function id 'foo-bar' and function id 'foo_bar' both generate 'Function_Foo_Bar' in the service API root scope; normalization collisions fail closed
```

A safe service's `service-check` report is byte-identical to before Task 047.

Since the corrective review, READY also covers the **artifact boundary**:
the wrapper must be emittable beside the model, with no shared output path
and no model/wrapper name conflict in a shared host scope. See
[Corrective review](#corrective-review-reviewed-head-10b3040).

## Atomic generation

```text
load/resolve
-> readiness (now including service API names/bindings)
-> project selected schema
-> build ServiceApiModel
-> generate UCI model files
-> generate service API files
-> validate the COMPLETE combined file set (incl. duplicate paths)
-> only then touch the output directory
```

Any wrapper failure leaves no directory, no model file, and no wrapper file.
The success summary now distinguishes the artifacts:

```text
generated model files: 1
generated service api files: 1
generated 2 file(s)
```

## Generated shape

Verbatim output (header comments omitted) for the upstream
`upstream-minimal.yaml` against real UCI 2.5. The payload of `PositionReport`
is `PositionReportMT`, so message and payload local names differ and the
wrapper binds the **resolved payload**, not a name guessed from the message.

### Rust — `service_api.rs`

```rust
#[path = "oam.rs"]
pub mod model;

pub mod service_api {
    pub const SERVICE_NAME: &str = "example-service";
    pub const SERVICE_VERSION: &str = "0.1.0";
    pub const SERVICE_KIND: &str = "service";

    pub mod function_position_input_example {
        pub const ID: &str = "position-input-example";
        pub const NAME: &str = "Position Input Example";

        pub mod exchange_position_report_input {
            pub const ID: &str = "position-report-input";
            pub const KIND: &str = "oms_message";
            pub const DIRECTION: &str = "input";
            pub const MANDATE: &str = "mandatory";
            pub const TOPIC: &str = "PositionReport";

            pub type Payload = super::super::super::model::PositionReportMT;
        }
    }
}
```

Compile it as the crate root of the generated service. `Payload` uses a
`super`-relative path, so the tree resolves identically wherever it is mounted.
With zero OMS exchanges there is no model, and no `#[path]` module is emitted.

### C++ — `service_api.hpp`

```cpp
#pragma once

#include "oam.hpp"
#include <string_view>

namespace service_api {

inline constexpr std::string_view service_name = "example-service";
inline constexpr std::string_view service_version = "0.1.0";
inline constexpr std::string_view service_kind = "service";

namespace function_position_input_example {

inline constexpr std::string_view id = "position-input-example";
inline constexpr std::string_view name = "Position Input Example";

namespace exchange_position_report_input {

inline constexpr std::string_view id = "position-report-input";
inline constexpr std::string_view kind = "oms_message";
inline constexpr std::string_view direction = "input";
inline constexpr std::string_view mandate = "mandatory";
inline constexpr std::string_view topic = "PositionReport";

using Payload = ::programs::oam::PositionReportMT;

}  // namespace exchange_position_report_input

}  // namespace function_position_input_example

}  // namespace service_api
```

`inline constexpr` keeps the header ODR-safe across translation units. The
model namespace comes from the C++ backend's own `namespace_name`, not from
the CLI.

### Ada — `service_api.ads`

```ada
with Programs.Oam;

package Service_API is

   Service_Name    : constant String := "example-service";
   Service_Version : constant String := "0.1.0";
   Service_Kind    : constant String := "service";

   package Function_Position_Input_Example is

      Id   : constant String := "position-input-example";
      Name : constant String := "Position Input Example";

      package Exchange_Position_Report_Input is

         Id        : constant String := "position-report-input";
         Kind      : constant String := "oms_message";
         Direction : constant String := "input";
         Mandate   : constant String := "mandatory";
         Topic     : constant String := "PositionReport";

         subtype Payload is Programs.Oam.PositionReportMT;

      end Exchange_Position_Report_Input;

   end Function_Position_Input_Example;

end Service_API;
```

No package body is required or emitted. The model package name comes from the
Ada backend's own `package_name`.

### Non-OMS scopes

A non-OMS exchange gets the common metadata only, never `TOPIC` or `Payload`
(Rust shown; C++ and Ada are analogous):

```rust
        pub mod exchange_bulk_imagery {
            pub const ID: &str = "bulk-imagery";
            pub const KIND: &str = "data_transfer";
            pub const DIRECTION: &str = "output";
            pub const MANDATE: &str = "optional";
        }
```

### String constants

Constants hold exactly the contract text. Printable ASCII is written
literally; anything else is escaped (`\u{..}` in Rust, three-digit octal byte
escapes in C++, `Character'Val` concatenation in Ada), so generated source is
always pure ASCII.

## Behavior changes, stated explicitly

### Zero-OMS services now produce one file

Task 032 documented that a service with zero OMS Message exchanges writes zero
files, because generation was **types only**. That remains historically
correct for type-only generation, and it remains true for the **model**: still
zero model files, and still no fabricated `SchemaIr` type. But such a service
has a real interface, so Task 047 intentionally emits its first meaningful
file: the wrapper, with structural descriptors, no model import, and no
`Payload`. It compiles standalone in all three languages.

### Contract order is now observable output

Task 032 proved that reordering exchange occurrences while selecting the same
messages leaves TYPE output byte-identical. That assertion is **kept at full
strength for the model files**. The wrapper is a different artifact whose
meaning *is* contract order, so it is judged separately:

```text
same selected messages + reordered contract
  -> model type files:  byte-identical
  -> service API file:  reflects the new contract order
```

### Ordinary `generate` is unchanged

`generate` has no contract and never emits `service_api.*`. `service-plan` and
`service-check` remain read-only.

## Evidence

### Real UCI 2.5 `PositionReport`

Inputs: the pinned UCI 2.5 `UCI_MessageDefinitions_v2_5_0.xsd`
(sha256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`, a
local copy outside the repository), the verbatim
`crates/service-contract/tests/fixtures/upstream-minimal.yaml`, closed world.
`origin/main` (`06bc62c`) and this branch were each built in release mode and
run on the same inputs.

| | Ada | Rust | C++ |
| --- | --- | --- | --- |
| readiness | 60/60 READY | 60/60 READY | 60/60 READY |
| `service-check` report vs main | byte-identical | byte-identical | byte-identical |
| model files vs main | 3 files, byte-identical | `oam.rs` byte-identical | `oam.hpp` byte-identical |
| new file | `service_api.ads` | `service_api.rs` | `service_api.hpp` |
| wrapper compile | GNAT `-gnatc -gnatwa -gnatwe` | `rustc -D warnings`, as crate root | `-std=c++17 -Wall -Wextra -Werror -pedantic-errors` |
| consumer probe | compiled and ran | compiled and ran | compiled and ran |

Model file hashes, identical on main and on this branch:

```text
0d207cd84193fdb66f5743fe95c899bba166507724a8b0e7a7245dc555ce18ad  oam.rs
988348a490ba448ac357dd89cece24e225cde64884ed00c78e5cd84dd5936de4  oam.hpp
a5d92d26a18c03999588818e68c5a8b3426ff1c780a7582bf21121727b4a0d16  programs.ads
4ab4ecfd228e6417c48d4805afbe98ddc17e4c0e1e1b760b19a89cd26b243b5b  programs-oam.adb
1e02efdd8817ea3b22318dd8836bf6d45b9b7d87f012d51dcd7eaa39a92d1eb7  programs-oam.ads
```

The consumer probes follow the task's shape. Rust imports
`service_api::service_api::function_position_input_example::exchange_position_report_input`,
asserts `TOPIC` and `DIRECTION`, and type-checks `Payload` as
`service_api::model::PositionReportMT`. C++ `static_assert`s
`topic == "PositionReport"` and
`std::is_same_v<endpoint::Payload, ::programs::oam::PositionReportMT>`. Ada
passes a `Position.Payload` value to a subprogram taking
`Programs.Oam.PositionReportMT` and checks `Topic` and `Direction`.

Local toolchain for this evidence: `rustc 1.98.1`, `c++ (Debian 14.2.0-19)
14.2.0`, `GNATMAKE 14.2.0`. CI uses floating selections
(`dtolnay/rust-toolchain@stable` and the runner's apt `gnat`/`g++`), so CI
versions are whatever those resolve to on the day and are not pinned.

The pinned UCI release is not in CI. CI instead runs the **verbatim** upstream
contract against `tests/fixtures/service-generate/position-report.xsd`, a
synthetic schema with the same namespace and the same `PositionReport ->
PositionReportMT` naming, and compiles a consumer in all three languages.

### Synthetic controls in CI

| Control | Result |
| --- | --- |
| all five exchange kinds, in contract order | one scope per occurrence, in order; TOPIC/Payload on the 3 OMS scopes only |
| repeated `MessageA` in two functions | `PayloadA` once in the model; two `Payload` aliases naming it |
| same exchange ID in two functions | legal; two nested scopes |
| reordered contract, same messages | model byte-identical; wrapper differs and follows the new order |
| `type`/`match`/`range`/`operator`/`delete`/`package` IDs | generate and compile in all three languages |
| human names with spaces, quotes, punctuation | escaped string constants only |
| `foo-bar` + `foo_bar` functions | NOT READY, `service api boundary:`, counts unchanged, nothing written |
| `foo-bar` + `foo_bar` exchanges in one function | same |
| primitive-payload OMS message | typed `UnboundPayload`, NOT READY, never a fabricated Payload |
| zero-OMS service | 0 model files, 1 wrapper, compiles standalone |
| model files vs library `Backend::generate` | byte-identical in all three languages |
| safe `service-check` report | byte-identical to the pre-Task-047 format |
| ordinary `generate` | no `service_api.*` |

`task047_ada_wrapper_compiles_under_gnat_and_is_consumable` is gated in CI
with `require_one_test`, so it cannot pass by skipping. Every existing GNAT
gate is unchanged.

### Not rerun

Full twelve-cell UCI backend coverage was **not** rerun as an acceptance
gate: this task changes no type lowering, and the byte-identity evidence above
shows type output is unchanged. The existing full-schema first blockers
(`AltitudeRangePairType.Range` in Ada, `ConfigurationParameterType.Type` in
Rust, `ApprovalResponseType.Operator` / `COMINT_ChangeDwellType.Delete` in
C++) are untouched and out of scope.

## Corrective review (reviewed head `10b3040`)

PR #48 was reviewed at `10b304061b9988eff0847876ac8d557834ecae34`. The review
asked whether `service-check` READY really implied that the **complete**
artifact set -- model files plus wrapper -- could be emitted. It could not:
the Task 047 preflight checked wrapper names against each other and payload
bindings against the model, but never checked the wrapper against the model's
*own* namespace-derived file and unit names. Readiness could not even ask:
those names were private to each backend (`model_file_name`,
`model_header_name`, `namespace_name`, `package_name`), and a second copy in
`service_api.rs` would have recreated exactly the drift readiness exists to
prevent.

### What the reproductions actually showed

The review predicted two specific failures. Both were reproduced as written
and **neither occurs**, because of one fact about the existing naming rule:
the namespace URI is split on *every* non-alphanumeric character and each
component is lowercased, so no model file stem and no C++/Ada unit component
can ever contain `_`, while every wrapper file and root does
(`service_api.rs`, `service_api.hpp`, `service_api.ads`, `service_api`,
`Service_API`).

Evidence on the reviewed head (debug build, `artifact-boundary.yaml`, closed
world):

| Schema namespace | Language | check | generate | files written | strict compile |
| --- | --- | --- | --- | --- | --- |
| `urn:test:serviceApi` | Rust | READY | exit 0 | `service_api.rs`, `serviceapi.rs` | ok |
| `urn:test:serviceApi` | C++ | READY | exit 0 | `service_api.hpp`, `serviceapi.hpp` | ok |
| `urn:serviceApi:serviceName` | C++ | READY | exit 0 | `service_api.hpp`, `servicename.hpp` | ok; model is `namespace serviceapi::servicename` |

So the predicted `service_api.rs`/`service_api.hpp` duplicate path and the
predicted `service_api::service_name` namespace/constant redeclaration are
**unreachable** from any schema. No "duplicate generated path" diagnostic
could be produced, and none was fabricated for this document. Both fixtures
are kept as positive controls
(`task047c_wrapper_look_alike_namespaces_are_ready_and_compile`).

The general question still had a real "no" answer, **in Ada**. The Ada
wrapper names its model as `Outer.Inner.Type` from inside
`Service_API.Function_*.Exchange_*`. Ada resolves `Outer` through direct
visibility, innermost first, so a wrapper declaration already visible there
and spelled like the model's parent package (case-insensitively) hides it:

| Schema namespace | Language | check (reviewed) | generate (reviewed) | GNAT `-gnatc -gnatwa -gnatwe` |
| --- | --- | --- | --- | --- |
| `urn:name:model` | Ada | READY | exit 0, 3 files | `service_api.ads:28:29: error: invalid prefix in selected component "Name"` |
| `urn:payload:model` | Ada | READY | exit 0, 3 files | `service_api.ads:28:29: error: type "Payload" cannot be used before end of its declaration` |

The same failure was reproduced for parents `Id`, `Kind`, `Topic`,
`Direction`, and `Mandate`. Rust and C++ built from those same schemas compile,
because their wrappers never name the model through that identifier. This is
the corrective's actual defect: READY, successful generation, and source that
does not compile.

Separately, parents `String`, `Character`, and `Standard` failed GNAT on the
**model alone**, before any wrapper is involved (`"String" conflicts with
declaration in package Standard`). That is a pre-existing Schema IR
backend-name gap in the model unit rule, not a model/wrapper boundary. It is
recorded here and left out of scope, like the full-UCI blockers.

### Shared artifact layout

`codegen-core` gained `backend_layout`, the **single** home of each backend's
model artifact rules:

```text
BackendModelLayout::for_schema(schema, language)
  artifacts: every model path, in emission order, optional ones flagged
  unit:      RustFile | CppNamespace([outer, inner]) | AdaPackage([Outer, Inner])
```

| Consumer | Uses |
| --- | --- |
| `backend-rust` `generate`, wrapper `#[path]` | `rust_model_file_name` |
| `backend-cpp` `generate`, wrapper `#include`/`Payload` | `cpp_model_header_name`, `cpp_model_namespace` |
| `backend-ada` `generate` (parent, spec, body files), wrapper `with` | `ada_model_package`, `ada_model_file_names` |
| `backend_names` namespace-unit preflight | `namespace_uri_components` |
| service API artifact preflight | `BackendModelLayout` |

The backends' private copies were deleted, not wrapped. Error texts are
preserved exactly (`Rust generation requires one namespace`,
`unsupported C++ IR construct: namespace URI ...`, and so on), and every
generated byte is unchanged; see the byte-identity evidence below. The
wrapper file name moved into `ServiceApiFixedNames::file`, and each backend's
`SERVICE_API_FILE` constant now reads it, so the preflight checks the path
that is actually written.

The C++ model's top-level names come from
`backend_names::top_level_generated_names`, which runs the same support-type
and declaration registration the Schema IR name preflight runs. Nothing
re-derives them.

### Artifact preflight

`validate_service_api_artifacts(model, projected_schema, language)` is called
by `service_api_preflight`, i.e. by readiness and by `service-generate`, **and**
by every backend's `generate_service_api` before rendering. A direct library
caller that skips readiness therefore gets the same typed diagnostic, not
source that conflicts with its model
(`task047c_direct_backend_call_fails_closed_on_the_artifact_boundary`). A
service with no type model has no model artifacts and passes trivially.

New typed errors, reported through `service_api_blocker` and never as an
unsupported UCI declaration:

| Variant | Carries |
| --- | --- |
| `ServiceApiError::ArtifactPathCollision` | language, path, whether the model side is optional |
| `ServiceApiError::ModelWrapperNameCollision` | language, host scope, identifier, model entity, wrapper owner, contract function/exchange region, and `Redeclaration` or `Hides { referenced }` |
| `ServiceApiError::ModelLayout` | integrity only: backend preflight already mapped the same namespace with the same rules, so readiness propagates it as `ServiceReadinessError::ServiceApi`, never as NOT READY |

**Paths, all three languages.** Every model path is built from URI
components and lowercase, so it never contains `_`. Every wrapper file does:

| Language | Possible model files | Wrapper |
| --- | --- | --- |
| Rust | `{c}.rs` | `service_api.rs` |
| C++ | `{c}.hpp` | `service_api.hpp` |
| Ada | `{o}.ads`, `{o}-{i}.ads`, optional `{o}-{i}.adb` | `service_api.ads` |

The collision is therefore impossible by construction, including the Ada
parent spec and the optional body. That is proved by
`model_paths_cannot_contain_the_wrapper_file_by_construction`, run over
hostile URIs (`serviceApi`, `service_api`, `service-api`, `service.api`,
`SERVICE_API`, `service_api.rs`, `service:api:ads`, real UCI, and more). The
check is still live: `a_layout_claiming_the_wrapper_path_is_a_typed_collision`
feeds it a layout that does claim the wrapper path (required and optional,
different case) and gets the typed error. Comparison is case-insensitive, so
a case-insensitive filesystem can never be the one to discover a clash.

**C++ combined scopes.** Only regions both artifacts actually reach are
compared:

```text
::                        model: namespace outer      wrapper: namespace service_api
::service_api             model: namespace inner      wrapper: 3 constants, function namespaces
::service_api::function_f model: top-level types      wrapper: id, name, exchange namespaces
```

The walk stops at `::` unless `outer == service_api`, and it descends only
through namespace-reopens-namespace, which is legal. Classification was
checked against g++ 14 `-std=c++17 -pedantic-errors`, with each model shape
prepended to a real generated wrapper:

| Model shape | g++ | Preflight |
| --- | --- | --- |
| `namespace service_api::service_name/_version/_kind` | redeclared as different kind of entity | `Redeclaration` |
| `namespace service_api::std` | `string_view` does not name a type | `Hides ::std` |
| `service_api::function_f` type `exchange_e` | redeclared as different kind of entity | `Redeclaration` |
| `service_api::function_f` alias/template `id` | redeclared as different kind of entity | `Hides` |
| `service_api::function_f` struct/enum `id` | compiles, constant hides the type | `Hides` |
| `service_api::function_f` entity `std` | `string_view` does not name a type | `Hides ::std` |
| `service_api::function_f` type `PayloadA` | compiles | ok |
| `service_api::other` type `id` | compiles (different region) | ok |

The preflight is intentionally conservative on struct/enum-vs-constant, which
compiles but makes the model type unreachable by its own name. None of the
C++ rows is reachable from a real schema today (`outer` never contains `_`,
and C++ model top-level names are UpperCamel, proved by
`cpp_model_top_level_names_are_upper_camel`). The walk exists so the verdict
never silently depends on those spellings. Positive controls
(`cpp_scope_positive_controls`) cover `programs::oam`, harmless prefixes
(`serviceapi`, `service`, `service_api_x`), a reopened `::service_api` with an
unrelated inner namespace, and a legally reopened function namespace.

**Ada visibility.** At each OMS `Payload`, the preflight walks the wrapper
declarations visible there, innermost first: the exchange's own constants
and `Payload`, the function's `Id`/`Name` and *preceding* exchange scopes,
the root constants and *preceding* function scopes, and `Service_API`. Any
case-insensitive match with the model parent package is `Hides`. Later
siblings are not yet visible and are not checked
(`ada_payload_visibility`).

### Readiness after the corrective

```text
$ service-check --schema artifact-ada-name-parent.xsd --contract artifact-boundary.yaml --language ada --world closed-schema
...
selected oms messages: 1
renderable selected oms messages: 1
selected type closure: 1
renderable selected types: 1
status: NOT READY

service api boundary: Ada service API fixed name 'Name' declares 'Name' (in the scope of function 'mission-data'), which hides 'Name' of the model package 'Name.Model' where the wrapper names it in Service_API.Function_Mission_Data.Exchange_A_Input
```

`artifact-ada-payload-parent.xsd` reports `fixed name 'Payload'` in the scope
of exchange `a-input`. For both schemas `service-generate` prints the same
report, exits 1, invokes no backend generation, and creates no directory.
Rust and C++ stay READY and compile. `validate_generated_files` is unchanged
and still runs on the combined file list as defence in depth.

### Regression evidence after the corrective

The real UCI 2.5 `PositionReport` check was re-run with the pinned
`UCI_MessageDefinitions_v2_5_0.xsd` (sha256 `ac94304...bf27`) and the verbatim
`upstream-minimal.yaml`, comparing a release build of `10b3040` with a
release build of the corrective:

| | Ada | Rust | C++ |
| --- | --- | --- | --- |
| readiness | 60/60 READY | 60/60 READY | 60/60 READY |
| `service-check` report | byte-identical | byte-identical | byte-identical |
| model files | byte-identical | byte-identical | byte-identical |
| wrapper file | byte-identical | byte-identical | byte-identical |
| consumer probe | compiled and ran | compiled and ran | compiled and ran |

Wrapper hashes (unchanged): `service_api.ads` `87d23f15...c3e`,
`service_api.rs` `3be11b3a...39a7`, `service_api.hpp` `71046ff7...f155`. Model
hashes match the table above.

Every original Task 047 control still passes unchanged. Metadata escaping is
untouched, and its literal-helper unit tests are kept. No public wrapper API
changed, no identifier sanitization was added, and the full-UCI member-name
blockers are untouched.

## Still open

- generated service publish/subscribe façade;
- typed LA-CAL integration;
- codec and runtime integration;
- kind-specific metadata for the four non-OMS exchange kinds;
- full-UCI generation (Phase 5).
