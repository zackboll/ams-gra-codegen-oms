# Task 097-C3 — production copy-safe views; encoder closure remains blocked

## Decision and scope

**ADDITIONAL_ENCODER_COPY_BOUNDARIES.** Both known generated model-read copies
are corrected and compiler-qualified in actual production-generated lossless
codecs. A separate allocating provider-array result copy remains **inside the
encoder's final output validation**. Its real allocation failure becomes
`Program_Error`. The unchanged provider string-transition defect is also reached
there, including an observed allocator abort. Therefore neither encoder resource
safety nor complete Float64 resource qualification is claimed.

This is a local corrective checkpoint, not a publication. No push, PR, workflow
dispatch, merge, rebase, force-push, old-branch change, upstream dependency patch,
or model-storage rewrite was performed. C1/C2 evidence and their failed attempts
remain unchanged. Only the existing Task 097 worktree was edited. New evidence
is `/home/zboll/task097-scratch/c3/`.

## 1. Identity, evidence ownership and frozen inputs

| Item | Identity |
|---|---|
| Worktree | `/home/zboll/git/ams-gra-codegen-oms-task097` |
| Branch | `feature/097-ada-float64-codec` |
| Starting C2 HEAD | `6252830f38df9c43cc715861640615974344b769` |
| C1 | `276fa90f1b7dd8aaad7af66bd2810e009c7c457a` |
| Implementation checkpoint | `b1e5d2d4545c2ce809456316f54ca0b04ef50dc6` |
| Fetched main / FETCH_HEAD | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| Initial index/worktree | Clean |

All three supplied checkpoints were verified ordinary ancestors before editing.
`ownership.json` records the starting state; `git-closure.json` records the final
normal commit and clean local worktree without recursively embedding that commit
in its own document.

Before consuming C2 sources, `prior-verification.json` reread and verified all
**4,618 source entries**, **31,524 evidence entries** and **462 completed exits**,
including the expected nonzero negative compilations. Consumed manifest hashes:

```text
dd85c514916fe9eaab5389585526075675134e8e3ad8792f4722d20422bcbb59  C2 source manifest
4f018475a02e26e8571613b0689da3d0f5b3d9c0d40c38e58e2499d754e7ce48  C2 evidence manifest
f9b32087de91dc049eb0f1598312dca1506ac0d387958e69556a5e01d03a2e84  C2 completed exits
```

The Task 097/C1/C2 documents, C1 model-copy reproducer and traces, selected C2
child, rejected access-reference experiment and external consumer results were
reviewed. C2's rejected access-exposing `Constant_Reference` API is **not** promoted.

Production sources were frozen in `source-final01/` before the compiler campaigns;
`source-final01-manifest.json` records the exact whole snapshot. Workers consume
this immutable snapshot or separately recorded test inputs. The only subsequent
test-source correction was the standalone tracing `Allocation_Faults` body:
imported C counter functions must have different Ada names with proper public
completion bodies. `post-freeze-test-correction.json` records that distinction;
all 24 allocation builds and later traces consumed the corrected body in
`trace-source02/`, byte-equal to the final checked-in test source. No production
generator input changed after the compiler source freeze.

Long campaigns use the established detached `scripts/task097/job.py` supervisor,
atomic completion records, independent streams and isolated writable build trees.
Fresh main and C2 baseline trees were extracted from exact commits and built
independently; they are not mutable historical evidence directories.

## 2. Production implementation

### Shared semantic view plan

`/home/zboll/git/ams-gra-codegen-oms-task097/crates/codegen-core/src/backend_names.rs`
adds:

* `AdaCodecViews`, `AdaCodecSequenceView`, and `ada_codec_views`;
* shared child/callback identifiers `Codec_Views`, `Visit_Element`, `Visit_Binary`;
* `ada_codec_views_files`, using conventional lowercase GNAT child filenames;
* `validate_ada_codec_views_name`, reusing the existing case-insensitive parent
  declaration, private helper and enumeration-literal registration rules.

Demand comes from **actual `TypeEmission`s**, effective inherited fields and
field-storage semantics, never completed source searches, guessed suffixes, or
declarations excluded by projection. The required public prefix array of an
unbounded positive-minimum sequence is deliberately absent from the opaque
accessor plan. Constrained Binary declarations use the shared effective Binary
length-domain classifier and retain their original constructor checks.

Shared codec preflight in
`/home/zboll/git/ams-gra-codegen-oms-task097/crates/codegen-core/src/service_codec.rs`
checks the child identifier and complete output path set **before rendering**.
The CLI's existing combined generated-file validation still rejects duplicate
paths before creating output as defense in depth. This does not broaden model-only
name policy or reserve child callback names in the parent.

### New codec-only artifact pair

`/home/zboll/git/ams-gra-codegen-oms-task097/crates/backend-ada/src/codec_views.rs`
renders the additional child spec/body. Examples:

```text
Programs.Oam.Codec_Views -> programs-oam-codec_views.ads / .adb
Urn.Copyproof.Codec_Views -> urn-copyproof-codec_views.ads / .adb
```

Parent types in callback profiles are fully qualified; `Standard.Positive` and
`Standard.Constraint_Error` avoid accidental hiding by parent names. The public
child spec names only public parent types. Its body has Ada child-unit visibility
of parent private vector instances, occupancy and constrained-carrier components.
Both generated files are in the returned codec file set, only when effective
lossless encoding needs a view. Model-only, portable and Float32-only generation
do not gain a child/import. A Float64 schema without repeated/carrier view demand
also gains none.

### Sequence read

```ada
procedure Visit_Element
  (Container : Programs.Oam.LosslessEnvelope_Picks_Sequence;
   Index : Standard.Positive;
   Process : not null access procedure
     (Element : in Programs.Oam.LosslessEnvelope_Picks_Item));
```

Unbounded private storage calls its parent's `Indefinite_Vectors.Query_Element`.
Bounded storage checks logical occupancy then calls `Process` directly on
`Container.Items (Index).Value`. Positive-minimum `Required (Index)` remains a
direct array component argument in the codec. Existing public `Model.Element`
and all other model operations remain unchanged.

### Constrained Binary read

```ada
procedure Visit_Binary
  (Item : Programs.Oam.Digest;
   Process : not null access procedure
     (Octets : in Programs.Oam.Binary_Vectors.Vector));
```

Its entire body operation is `Process (Item.Data)`. No returned vector,
intermediate `Model.Value`, access result or unchecked conversion is introduced.
The generated named primitive's `To_JSON_Digest` declares `Encode_Octets`, invokes
the child synchronously, and serializes the actual tagged vector through the
unchanged `Encode_Hex_Binary` helper. Its emitter `Result` is independently owned
output, not a borrowed model component.

All named uses already route through `To_JSON_<Name>`, so this covers required and
optional Record fields, nested Records, Choice-contained Records, repeated named
carriers and repeated Records/Choices containing carriers. Binary length checks,
empty/exact/min/max value domains, uppercase hex spelling and XML whitespace
behavior are unchanged. Neither `Create` nor the existing public `Value` is
removed or modified.

### Narrow repeated-carrier admission

The old codec explicitly rejected **all** named repeated Binary, so the requested
coverage could not compile without a narrowly accounted correction. Only effective
lossless-profile **constrained named Binary** is now admitted by the existing
named-repeated classifier. It still passes its original provenance/domain codec
checks. Direct repeated Binary, unconstrained named repeated Binary, invalid
facets, unsupported lexical profiles and legacy profiles retain their rejection.
No other profile is newly marked READY. This is the task-requested carrier path,
not a blanket Binary capability expansion.

## 3. Language, lifetime and mutation contract

Ada RM 6.2 makes tagged types by-reference and composites containing a by-reference
subcomponent by-reference, including private types through their full view.
`Binary_Vectors.Vector` is tagged. Consequently constrained Binary carriers,
Binary-containing Records/Choices, their slots/arrays and vector-containing
sequence containers are by-reference. Passing the stored component to an `in`
callback does not create the old controlled function-result object. Elementary
float/integer/boolean/enumeration values can copy without controlled payloads.
Temporal/string full views are inspected against both actual GNAT runtimes.

`Query_Element` passes the actual element, prohibits vector tampering during the
callback and propagates callback exceptions; local lock finalization releases
the restriction on both normal and exceptional exits. These rules supplement,
not replace, compiler-backed allocation observations. A callback API is not
automatically a no-copy proof for arbitrary unknown private element categories.

Callbacks are synchronous and never retained. No access-to-element API is exposed.
Both compilers reject attempted `Element'Access`/`Octets'Access` escape and mutation
through the `in` parameter with the required specific diagnostics and exit 4.
Nested visits and subsequent same-process visits after `Storage_Error`, unrelated
`Program_Error`, and controlled callback allocation failure succeed. Input octets
and sequence counts remain unchanged. Invalid constructor lengths still fail.

Source mutation through another alias, assignment, finalization or concurrent
mutation during a visit is **prohibited**, not supported by this interface.
The isolated unbounded `Clear` negative control raises `Program_Error` and leaves
the vector unchanged. Bounded slots have no runtime borrow lock; privacy plus
an `in` callback is not a general-purpose alias/mutation checker. No thread-safe
simultaneous mutation promise is made. C2's GNAT14-retainable access-reference
alternative remains rejected and unexposed.

No production code classifies errors by exception text, source location,
allocation ordinal, last-failure state or injector flags. No `Program_Error`
translation is introduced.

## 4. Original and corrected model-copy traces

### A. Repeated Choice/Record result

The unchanged C1 model reproducer remains intentional red:

```text
Model.Element (Picks, 1)
 -> ArraySample adjustment -> PathLength adjustment
 -> Binary_Vectors.Adjust -> real malloc(8) fails
 -> compiler controlled adjustment -> Program_Error
```

`known-copies/` contains fresh stacks under both namespaces, compilers, policies
and O0/O2. The original consumer remains compiled without changing the parent.
Same-process recovery completes after the red failure.

Corrected production children execute all five forms without a read allocation:

| Form | Production read mechanism | Required evidence |
|---|---|---|
| Zero-min unbounded | private vector `Query_Element` | empty/single/multi, Binary/Choice ordering |
| Positive-min Required | public array component | direct read, callback failure/re-read |
| Positive-min Additional | private vector `Query_Element` | empty/single/multi |
| Zero-min bounded | occupied slot after bounds check | empty/single/multi, failure recovery |
| Positive-min bounded | occupied slot after bounds check | single/multi, failure recovery |

Empty complete positive-minimum sequences are invalid, not fabricated test cases.
C2's original and shape consumers, supplemental old Binary-copy probe and isolated
tamper/negative tests were rebuilt with **production-generated** children in all
24 compiler/namespace/policy/optimization cells. Armed read visits require zero
attempts. Callback-local buffer allocation is a separate 64-byte failure.

### B. Constrained Binary result

Old API intentionally remains:

```text
Model.Value (Digest) -> Binary_Vectors.Adjust -> real malloc(8) fails
```

It propagates `Storage_Error` in the focused old-vector-result reproducer; it is
still an unnecessary fallible controlled copy inside the old encoder. The new
visitor reads the original owned `Data` with zero armed attempts. `c3_views.adb`
checks exact two octets, repeated nested visitors, empty/exact carriers, invalid
constructor lengths, deliberate callback `Storage_Error` and `Program_Error`,
real callback allocation failure, unchanged carrier and subsequent success.

`known-copies-result.json` has **72 fresh debugger executions**: old element,
corrected element and old Digest copy for each of 24 cells. All required markers
and inferior exits complete; red is never relabeled resource-safe.

Actual generated codec boundary injection appears in the encoder audit:
arming at `Visit_Binary` reaches serialization `Encode_Octets -> Encode_Hex_Binary
-> Output.Create`, not `Binary_Vectors.Adjust`. Element callback injection likewise
reaches serialization after entry. All **48** actual callback-boundary cases
propagate `Storage_Error`, preserve output sentinel/model state and recover.

## 5. Production-generated composition and consumer matrix

Fixture:
`/home/zboll/git/ams-gra-codegen-oms-task097/tests/fixtures/service-generate/codec-ada-copy-views.xsd`.
Consumer/visitor and trace test units are in
`/home/zboll/git/ams-gra-codegen-oms-task097/tests/ada-float64-codec/`.
The durable production-matrix runner is
`/home/zboll/git/ams-gra-codegen-oms-task097/scripts/task097/c3_matrix.py`.

All **24** cells complete: GNAT **14.2.0 / 16.1.0**, OAM / qualified `urn:copyproof`,
Normal / Check / Ignore, O0 / O2. Actual emitted model/codec/child units compile,
not handwritten prototypes. Payload coverage includes:

* direct Binary; min/max, exact-two and exact-empty named Binary;
* required, optional and repeated carriers inside nested Records;
* inherited Float64, finite nextafter-style binary64, subnormal `5e-324`, negative
  zero and Float32;
* named Choice and repeated Binary-containing Choice;
* bounded and unbounded repeated Records, Required prefix order;
* repeated temporal fields; short and heap-crossing Unicode strings.

Each cell checks populated, minimum and independently authored duplicate-key
documents, typed decode/encode/decode roundtrip, exact binary64 bits (including
negative zero), actual JSON number type, Binary octets and array order. Nine
malformed documents exercise unknown members, syntax, Binary spelling/length,
repeated carrier failure, invalid finite spelling and missing required sequence.
Rejection leaves retained typed state unchanged. Duplicate keys retain last-value
wins. The matrix also checks no-fault encode, first reached resource failure,
output sentinel preservation and subsequent encode success.

Each compiler/namespace campaign records **98 exit-0 commands**, **392 total**.
No-fault composition is not used as evidence that final validation is resource-safe.

Both child units are listed explicitly in the consumer GPR's `Source_Files`,
proving external explicit-file-list integration and GNAT dependency ordering.
Independent Alire consumers compile/run production-generated Binary visitors with
pinned actual GNATLS 14.2.0/16.1.0 and GPRbuild 26.0.1, all six commands exit 0.
Their settings/config/toolchain deployment is isolated under C3. Parent units
precede the child; the parent does not depend on the child, so no new elaboration
cycle is introduced.

Both compiler naming consumers compile constrained carriers named `Visit_Binary`
and `Visit_Element` because all parent profile types are qualified. Exact/mixed-case
`Codec_Views` declaration and literal collisions fail shared preflight atomically.
Existing `Element` and helper-name collision tests remain in the passing shared
and CLI suites. An ancestry-only abstract `Codec_Views` Record does not falsely
reserve a child conflict. No demand means no artifact/import.

## 6. Complete supported encoder copy/ownership inventory

The supported renderer call graph is:

```text
Encode_* -> Require_Platform -> To_JSON_<payload>
  -> To_JSON_Record / To_JSON_Choice
     -> direct field / optional present field / nested named helper
     -> Required array read / Codec_Views.Visit_Element -> Add_Item
        -> named constrained Binary helper -> Codec_Views.Visit_Binary
           -> Encode_Hex_Binary (actual tagged vector)
        -> Float32 / Float64 / integer / boolean / enum / string / temporal output
     -> Output.Set_Field / Append / Create(array), return owned Output.Value
  -> Output.Write -> String extraction
     -> Occurrences.Read -> Guard.Read -> provider parser -> Validate_Tree
     -> occurrence shadow parser, provider strings/objects/arrays
  -> return String -> optional separate caller assignment
```

`encoder-copy-inventory.json` records effective type, access/result mechanism,
allocating Adjust possibility, actual reachability, alternatives and evidence for
every supported encode-operation family. It includes **990 representative
stack-group injections** after **88,716** successful-baseline allocation stacks
were enumerated across 24 cells. These groups deliberately retain compiler-
dependent call-chain differences; they are not claimed 990 independent language
mechanisms or an exhaustive ordinal sweep.

| Expression/mechanism | Ownership/copy classification | Status |
|---|---|---|
| opaque `Model.Element` result | allocating controlled Record/Choice/Binary copy | removed from lossless encode only; public model API retained |
| constrained `Model.Value` vector result | allocating controlled vector copy | removed from lossless encode only; public carrier API retained |
| `Required(Index)`, occupied slot, Query callback, carrier `Data` callback | reference-preserving by-reference arguments | no model-result copy; armed zero-attempt reads |
| direct Binary argument/octet iteration | tagged vector by-reference; octets scalar | unchanged, no vector extraction copy |
| checked float `Value`, derived float/integer/boolean/enum operations | elementary results | no controlled-copy allocation |
| temporal/string `Value`, `To_String` | String results, controlled input read | ordinary result/serialization allocation, not vector Adjust |
| `To_JSON_* return Result`; new callback `Result := Encode_Hex_Binary` | emitter Value containing controlled Unbounded_String | GNAT14/16 Adjust only increments shared-string refcount; no allocating Adjust observed; owned construction still fallible |
| emitter `Temporary := Container.Text`, mutation, assignment back | nonallocating refcount copy, then copy-on-write Delete/Append | serialization allocations separately injected; not automatically classified as unsafe Adjust |
| `GNATCOLL.JSON.Create(text)` / JSON_Value returns | refcount Adjust; ordinary owned provider construction | ordinary failures plus unchanged XString transition hazard |
| `Output.Create (JSON_Items)` | emitter wrapper/value result, controlled strings | no deep controlled Adjust on tested runtimes; output ownership transfers inventoried |
| `Output.Write` `Checked := Occurrences.Read(Text)` | controlled occurrence wrapper, internal validation allocations | reaches unresolved provider-array result copy before Encode returns |
| guard `Items : constant JSON_Array := Get(Item)` | provider JSON_Array containing allocating controlled vector | **remaining unsafe controlled result copy**, red reproduced |
| caller `To_Unbounded_String(Encode_*)` | separate caller allocation after successful String return | classified separately, never used to exclude internal encode validation |

Runtime `Unbounded_String.Adjust` and provider `JSON_Value.Adjust` are inspected,
not assumed: they perform reference count operations, not deep heap copies.
Serialization mutation may allocate after a refcounted copy; that is a separate
ordinary failure site with its own publication and cleanup obligations. No
absence-of-`Program_Error` inference substitutes for allocation reachability.

### Executable audit outcomes

`c3_malloc_trace.c` logs libc malloc/realloc in fixed thread-local storage with no
dlsym/backtrace/formatting/allocation in the armed path. It returns a one-shot
real NULL with ENOMEM before unwinding. Independent C controls under GCC14/16,
O0/O2 verify thread isolation, exact counts, disarm, one-shot failure and original
pointer ownership after failed realloc. The old C1 wrapper remains a separate
validated instrument for known-copy and view tests.

For each successful generated encode baseline there are **3,537** OAM or **3,856**
qualified libc events, including runtime/provider allocations not all visible
through the older linker wrapper. GDB captures every armed allocation stack;
the event count and stack count agree. Realloc is separately identified. First
instances of every actual normalized call-chain group are injected and traced,
plus both actual model-view callback boundaries in every cell. No inherited
ordinal is assumed stable: selected ordinals come from the same executable's
fresh baseline stack inventory.

Definitive observations in `audit-result.json`:

| Observation | Count |
|---|---:|
| Representative real-NULL mechanism injections | 990 |
| `Storage_Error`, state preserved and same-process recovery | 911 |
| `Program_Error`, state preserved and same-process recovery | 78 |
| Qualified GNAT16 Check/O0 provider cleanup allocator abort | 1 |
| Additional actual visitor callback injection/recovery | 48 |

The 78 Program_Error observations comprise 36 provider-array result-copy sites,
36 unescape transition sites and 6 provider key `To_XString` transition sites.
The single SIGABRT is **not** a recovery or resource-contract pass. It is a
definitive adverse provider result captured at real failure stack and allocator
diagnostic, not missing evidence. All selected observations are complete; the
resource contract is explicitly false.

## 7. Remaining encoder blocker and smallest reproducer

Actual generated encode trace (GNAT14 OAM Normal/O0 example):

```text
Encode_LosslessEnvelope -> Output.Write
 -> Occurrences.Read -> Guard.Read -> Validate_Tree.Visit
 -> GNATCOLL.JSON.Get (array) -> JSON_ArrayDA
 -> Vect_Pkg.Adjust -> real malloc(72) fails
 -> Program_Error
```

This occurs inside public Encode output validation, not decode requested by the
caller. It cannot be excluded from the C1 resource contract as caller copying.
Both known model copies are already absent when it fails.

Smallest separately executed reproduction: `Guard.Read ("[1]")`. Arm specifically
at the array traversal boundary, then let the real **48-byte** `Vect_Pkg.Adjust`
allocation fail. `JSON_ArrayDA` transforms the exception to Program_Error; a
subsequent same-process parse succeeds. All **12** compiler/policy/O0/O2 cells
confirm this with explicit red/recovery markers and normal inferior exits.
The source and per-executable symbol/address-selected debugger commands are in
`guard-copy/` and `guard-traces/`. The address is discovered from each built
executable's actual symbol, not a hardcoded stable address or ordinal.

A generated model child cannot view or borrow provider-private array storage.
The provider public API offers by-value `Get(JSON_Value) return JSON_Array` and
object mapping, but no demonstrated safe read-only array-query operation.
Correcting this requires a separately reviewed **validation ownership** design:
for example a SAX guard that retains all existing UTF-8/grammar/depth/node/duplicate
semantics without extracting controlled arrays, or a qualified provider query
API. Removing final validation or modifying provider internals opportunistically
would not follow the proven generated-model child architecture. No such broad
guard/provider rewrite is performed here. Unrelated model representation/API
changes cannot repair this provider-owned result boundary.

The qualified key transition `To_XString -> Set -> Reserve -> Convert_To_Big_String`
is separately recorded as the same unsafe provider publication-order mechanism,
not misidentified as another model controlled copy.

## 8. Decode and provider remain separate blockers

Decode still constructs owned values. Its relevant graph includes
`Decode_Hex_Binary return Vector`, `Model.Create(Octets)`, carrier adjustments,
`Decode_Item` returns, bounded `Values` arrays/slot assignments, required-prefix
arrays, Additional `Append`, nested/Choice/optional aggregates and the final
returned payload. Their allocating construction/copy lifetimes are not solved
by a read-only encoder visitor.

`decode-boundary-result.json` records **24** source-aware real-NULL reproductions:

```text
Decode -> From_JSON_LosslessEnvelope -> From_JSON_Reading -> From_JSON_Leaf
 -> From_JSON_Digest -> Model.Create(Octets)
 -> DigestDA -> Binary_Vectors.Adjust -> malloc(8) fails
 -> Program_Error
```

Retained input/output state and same-process recovery are checked. No broad
decoder rewrite or decode resource pass is claimed. A separate construction/
ownership design is needed, including invalid-default avoidance and constructor
invariants; borrowing a source view does not construct a new owned carrier.

The C1 provider defect remains pristine:
`Un_Escape_String -> Append -> Reserve -> Convert_To_Big_String` mutates the
representation before allocation. C3 also demonstrates its reachability during
Encode's final validation and provider key creation. The C1 transition patch
and evidence are unchanged, unintegrated candidates. Provider source hashes in
every C3 isolated tree retain the pristine identity. No provider success is
inferred from fixing model access, and no resource failure is translated by text.

## 9. Preservation, local gates and failed attempts

Fresh exact-main and exact-C2 generator builds each pass **105** comparisons:
**86 byte-identical generations**, **19 matching atomic rejections**. Total:
210 comparisons, with success/rejection kept distinct. They cover Ada model/API,
portable/Float32/Binary combinations and Rust/C++. The new two-namespace fixture's
eight model/API files compare byte-identically between C2 model-only generation,
corrected model-only generation and corrected codec generation. New child files
are correctly counted as additions, not identical to nonexistent baseline files.

All required local source gates pass on frozen generator inputs:

* `cargo fmt --all -- --check`;
* `cargo check --workspace --all-targets`;
* warnings-denied `cargo clippy --workspace --all-targets -- -D warnings`;
* complete codegen-core and Ada/Rust/C++ backend test suites, including live
  compiler probes and new semantic-plan/name/view tests;
* all 26 CLI codec tests and 37 CLI library tests;
* 24 actual generated composition/Binary visitor cells;
* 24 C2 original/shape regression cells, 72 known-copy GDB runs;
* 48 element and 48 Binary compile-time-negative builds, expected exit 4 and
  specific diagnostics; vector tamper/recovery tests;
* explicit-source-list GPR and both actual Alire compiler consumers;
* complete allocation stack inventory and representative mechanisms with
  separately classified adverse resource results.

This is not the 27,167/20,474 numerical campaign, real-UCI qualification, leak/
all-ordinal proof or hosted CI. Those final gates remain outstanding.

All failed setup attempts are preserved. Examples: test fixture used Ada reserved
`Entry` before being corrected to `Reading`; imported C counters needed Ada
completion bodies; first debugger launch ran before some build outputs existed;
initial minimum-reproducer commands compiled without linking or lacked dependency
debug symbols. An early guard job was explicitly stopped after detecting GPR
dependency rebuilds aimed at shared C3 scratch object directories; later guard
builds own isolated dependencies. No C1/C2 or other worktree was involved. The
final minimum traces discover per-binary symbols and complete successfully.

The earlier broad debugger worker stops on an adverse SIGABRT rather than making
its optimistic recovery-only result record. `audit-completion-job` verifies all
already completed observations, completes remaining planned traces, and records
that SIGABRT as adverse, not as a recovery pass. `audit-result.json` is the final
observation ledger; failed/superseded worker records remain visible. Likewise,
`guard-copy-result.json` retains the unsuccessful source-breakpoint attempt;
`guard-traces-result.json` is the independently completed minimum reproduction.

## 10. Evidence identities and next task

Key result hashes under `/home/zboll/task097-scratch/c3/`:

| File | SHA-256 |
|---|---|
| `prior-verification.json` | `b11c69d87b31a7926cefd66b3e708367435cd570c213377a51379e665f17b93e` |
| `audit-result.json` | `2bde414d3a81031ecdc7881cbbaec74b24d81f6a9b4214b3162fb3c81009d3e8` |
| `encoder-copy-inventory.json` | `2b3fa6d85abd55dedd3b0708f88f0d75537f469296b92e417a514adac6b95b61` |
| `known-copies-result.json` | `14965a092121d88045735ad702a0c1699ec8fb8b1f930988cb8a09a0b86253e4` |
| `guard-traces-result.json` | `7ac0fa93b44e874fdd0c8d5215aaa9f9c89b3fbeba1483ef5a6fc719129775ab` |
| `decode-boundary-result.json` | `65eb2c3f463539b2b402ac4dde0634f3486f12b99647fc26ce38c421ea3ad2f6` |

`final-source-manifest.json`, `final-evidence-manifest.json`,
`manifest-hashes.json`, `completed-exits.json`, `inventory-verification.json` and
`decision.json` provide deterministic sorted inventories and final completion
accounting. Source inventories distinguish consumed frozen sources from final
test-only corrections. Compiler/provider/input/generated-artifact/source/GPR/
Alire identities, allocation sizes, call stacks and command exits are retained.
Failed and interrupted attempts are included, not silently removed. Inventory
self-files and post-commit closure are excluded from recursive evidence hashing;
their own hashes are reported separately.

Next work must independently design and qualify final-output validation ownership,
resolve provider transitions without weakening validation, then address decode
construction/ownership. Only afterward can the complete final-source numerical,
resource, real-service, preservation and hosted gates support any publication
decision. These views remain a local production correction for independent
review; no automatic publication is authorized.

**Final disposition: ADDITIONAL_ENCODER_COPY_BOUNDARIES.**
