# Task 097-C2 — copy-safe traversal proof checkpoint

## Decision

**ADDITIONAL_COPY_BOUNDARIES_FOUND.** The selected callback visitor avoids the
demonstrated sequence element-result copy, including when the former allocation
boundary is armed to fail. It works with unchanged parent model bytes and normal
external GPR/Alire consumers. Other controlled value-return operations remain in
the actual encoder. This is not full resource qualification, Float64 readiness,
publication authorization, or a production implementation.

Only this decision record is added to the feature worktree. Experimental sources,
generated units, runners, logs, provider copies, executables and build products
are under `/home/zboll/task097-scratch/c2/`. No provider change, model representation
change, exception translation, readiness change, push, PR, hosted dispatch, merge
or rebase was performed. The C1 provider candidate and all original evidence were
left untouched.

## 1. Ownership and source identities

| Item | Verified identity |
|---|---|
| Worktree | `/home/zboll/git/ams-gra-codegen-oms-task097` |
| Branch | `feature/097-ada-float64-codec` |
| Starting C1 HEAD | `276fa90f1b7dd8aaad7af66bd2810e009c7c457a` |
| Preserved implementation | `b1e5d2d4545c2ce809456316f54ca0b04ef50dc6` |
| Fetched main / FETCH_HEAD | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| Initial worktree status | Clean |

Both supplied commits are ordinary ancestors of starting HEAD (each ancestry
command exit 0). `ownership.json` records these observations. Fetch was performed
from the task worktree; no other worktree was edited or checked out.

`evidence-verification.json` verifies **14,666/14,666** original evidence entries,
**36,209/36,209** C1 evidence entries, and **15/15** C1 source entries. Comparing
the earlier 42-entry checkpoint source manifest to current C1 finds exactly the
expected later C1 change to `docs/task-097-ada-float64-codec.md`.
`checkpoint-source-verification.json` independently verifies **42/42** against
commit `b1e5d2d...`; this mismatch is not treated as evidence corruption.

The prior task document, C1 resource contract, model/provider reproducers,
allocator controls and ownership/exception traces were reviewed before testing.
The original C1 source and evidence manifests are reverified at closure. No C1
campaign is relabeled as a C2 production qualification.

## 2. Exact old unsafe call graph

The encoder emitted by `repeated_member_render` in
`/home/zboll/git/ams-gra-codegen-oms-task097/crates/backend-ada/src/service_codec.rs`
uses:

```ada
Add_Item (Model.Element (Item.Picks, Index));
```

For C1's actual bounded `LosslessEnvelope_Picks_Sequence`:

```text
Encode_LosslessEnvelope
  -> To_JSON_LosslessEnvelope
     -> Model.Element (Picks, Index)
        -> return Container.Items (Index).Value
           -> compiler-generated ArraySample adjustment
              -> compiler-generated PathLength adjustment
                 -> Binary_Vectors.Adjust
                    -> new Elements_Type'(L, EA)  [8 bytes for these two octets]
                       -> real System.Memory.Alloc -> malloc returns NULL
```

The failed object belongs to the **function result**, not the retained slot and
not callback parameter passing. `Adjust` first detaches the result's element
pointer and resets its last index. The generated enclosing composite adjustments
convert the raised exception to `Program_Error`; debugger breakpoints capture
`__gnat_rcheck_PE_Finalize_Raised_Exception`. This is not the C1 provider's
invalid inline-string representation defect. The callback has not yet been
entered at the old failure.

`original/c1_model_probe.adb` is copied byte-for-byte from the committed C1
reproducer and invokes no parser/provider. Its unchanged parent model is also
copied from C1. All **12** compiler/policy/optimization combinations reproduce
`TASK097_C1_MODEL_EXCEPTION PROGRAM_ERROR`, then
`TASK097_C1_MODEL_RECOVERY_COMPLETE`, exit 0. The exit means the **intentional red
reproduction completed**, not that the old API is safe. All 12 corresponding
debugger runs attribute the failure to the real 8-byte Binary `Adjust` allocation.
No optimized cell eliminated this old copy in this experiment; no portability
claim is inferred from optimization.

The index lookup itself is not the deep copy. The bounded return expression
constructs a value result. The unbounded wrapper additionally calls the private
`Indefinite_Vectors.Element`, also a value-returning function. Either result
boundary is unsafe to assume noncopying. In contrast, passing a stored by-reference
component to an `in` callback has no element-result object.

## 3. Chosen child package and language basis

Actual library unit and filenames:

```text
Programs.Oam.Codec_Views
programs-oam-codec_views.ads
programs-oam-codec_views.adb
```

Representative public declaration:

```ada
procedure Visit_Element
  (Container : LosslessEnvelope_Picks_Sequence;
   Index     : Positive;
   Process   : not null access procedure
     (Element : in LosslessEnvelope_Picks_Item));
```

The visible child specification names only public parent types. Its **body** can
see the parent's private completions and vector instances. The compiled bounded
body is:

```ada
if Index > Container.Count then
   raise Constraint_Error;
end if;
Process (Container.Items (Index).Value);
```

The compiled unbounded body is:

```ada
LosslessEnvelope_UB0_Vectors.Query_Element
  (Container.Items, Index, Process);
```

There are no returned access values, reference objects, unchecked access
conversions, per-element heap objects or model-copy temporaries in this API.
Callbacks are invoked synchronously and are never stored. Only relevant opaque
sequence overloads are exposed; the already public `Required` array needs none.

### Why this callback does not copy the controlled elements

Ada RM 6.2 paragraphs 4–10 makes tagged types by-reference, composites containing
a by-reference subcomponent by-reference, and private types with such a full view
by-reference. `Binary_Vectors.Vector` is tagged; hence `PathLength`, `ArraySample`,
their slot/array compositions and controlled-element sequence containers are
by-reference. This is true of the type, not conditional on which Choice variant
is currently active. The indefinite vector is itself tagged. Neither the visitor
container argument nor its `in ArraySample` callback parameter requires a copy.
The callback receives the stored object's view, not a copied function result.

`Query_Element` alone is **not** a proof for arbitrary types. Here the actual
types were examined. The generated checked float carriers are scalar full types,
so copying their values cannot deep-copy a vector. Temporal/string carriers have
controlled `Unbounded_String` full-view subcomponents in both tested GNAT
runtimes. The Binary-containing Record/Choice case has the direct language-defined
tagged-vector basis above and does not depend on that string implementation.
Future element categories must receive the same classification before reuse of
this design; this is not a universal arbitrary-private-type theorem.

RM A.18.2 `Query_Element` (136–137/3), applied to indefinite vectors through
A.18.11, passes the indexed element to `Process`, prohibits element tampering
during the callback and propagates callback exceptions. Both compiler runtime
sources show `Process (V.Elements.EA (Index).all)` under a local `With_Lock`.
Its finalization unlocks on ordinary and exceptional exits; no element result is
constructed. RM 10.1.1/10.1.2 provides the child-unit/ancestor visibility model;
the external compiler tests establish the actual body visibility used here.

The Ada 2012 RM text for these established rules is archived in
`language/` with URL, HTTP status and hashes. An initial Ada 2022-site fetch failed;
the successful mirror fetch is not represented as a successful 2022-site fetch.
All tested units are compiled in Ada 2022 mode. Runtime `a-coinve`, `a-convec`,
`a-conhel`, `a-strunb` source identities are included in the source inventory.

### Rejected alternate: exposing a Constant_Reference pointer

`controls{14,16}/programs-oam-reference_trial.*` investigates an explicitly
aliased container formal, local `Constant_Reference_Type`, and an anonymous
access-to-constant callback argument. Both compilers compile the child, perform
an allocation-free scalar visit, reject `Clear` while the reference is active,
and allow mutation after callback failure, demonstrating reference cleanup.
This investigation is separate from the selected `Codec_Views` artifact.

The escape negative control is decisive: converting that callback argument to a
named access type declared in the enclosing consumer **succeeds on GNAT 14** in
the tested program, whereas GNAT 16 raises `Program_Error`. The test discards the
retained pointer before clearing the vector and never dereferences it after
lifetime termination. `final-controls{14,16}` confirms this difference across
all policies and O0/O2. No conclusion about a compiler bug is required here;
the access-exposing API is simply **not selected** as the cross-compiler
non-retention contract. A reference object's own scoped finalization does not
prove a pointer handed out by it cannot be retained.

## 4. Lifetime and mutation restrictions

* The selected callback sees only an `in` object; it may read nested discriminants,
  record fields, Binary octets and other read-only fields. It may serialize them
  into independently owned output.
* No element pointer is supplied or stored in codec state. A direct attempt to
  save `Element'Access` in a longer-lived named access value fails compilation
  with both compilers (exit 4, `non-local pointer cannot point to local object`).
* Assignment to an `in` element also fails compilation (exit 4). Those negative
  controls are separate from valid consumer builds.
* A callback must not mutate the source container or its elements through another
  alias, clear/append/assign/finalize it, or concurrently change it. No unchecked
  access/address conversion is supported. The design is not synchronization.
* Vector `Query_Element` detects the isolated attempted `Clear` as
  `Program_Error`, leaves the vector unchanged, and releases its lock afterward.
  Checks are not disabled with `-gnatp`, `Suppress (Container_Checks)` or
  `Suppress (Tampering_Check)` in this proof.
* The bounded representation has **no runtime borrowing lock**. Mutation through
  a separately captured alias during a visit is prohibited, not made safe or
  supported. The generated codec callbacks do not contain such mutations.
  Privacy and an `in` view do not constitute a general-purpose borrow checker.
* Callback exceptions propagate unchanged. The visitor does not catch or classify
  `Program_Error`, `Storage_Error` or arbitrary internal exceptions. Nothing
  translates an exception using its text, location, prior allocation or injector
  state.

The selected callback cannot silently retain an access value using its normal
interface. It can deliberately make its own owned copy, which is a **new** copy
boundary belonging to that consumer, not a guarantee supplied by the visitor.

## 5. Five sequence shapes and controlled-copy proof

| Shape | Exact experimental storage/access | Result in all 12 cells |
|---|---|---|
| Zero-min unbounded | `LosslessEnvelope_UB0_Sequence.Items`, `Query_Element` | Empty, single, multi, ordered Binary/Choice reads; no armed read allocation |
| Positive-min unbounded Required | `LosslessEnvelope_UB1_Required_Array (1)` directly passed to `in` reader | No opaque accessor, no armed read allocation; callback failure/re-read |
| Positive-min unbounded Additional | `LosslessEnvelope_UB1_Additional.Items`, `Query_Element` | Empty, single, multi, ordered Binary/Choice reads; no armed read allocation |
| Zero-min bounded | `LosslessEnvelope_B0_Sequence.Items (Index).Value` after logical bounds check | Empty, single, multi, ordered Binary/Choice reads; no armed read allocation |
| Positive-min bounded | Original C1 `LosslessEnvelope_Picks_Sequence` occupied slot | Single/multi Binary/Choice reads; no armed read allocation; original parent bytes preserved |

Empty positive-min complete sequences are invalid and are not claimed valid test
cases. A positive-min sequence's `Additional` portion may legitimately be empty.
Index 0 is rejected by the `Positive` subtype; past-length and empty lookup tests
raise `Constraint_Error`. First/last positions and nested same-vector read visits
succeed. Repeated visits observe the same element address without modifying it.

`matrix-result.json`: 12 cells, each with separate original-parent and five-shape
consumer builds. The original consumer completes **17 checks, 1 deliberate
callback allocation failure**. The shape consumer completes **89 checks, 4
deliberate callback allocation failures**. All build/run exits are 0. The visitor
is armed with `Arm (0)` and must report **zero allocation attempts** while reading
controlled Binary/Choice elements. Thus the formerly failing read copy does not
execute; this is not merely a no-fault traversal pass.

`supplement-result.json`: 12 further cells cover single-element forms of all five
shapes, exact two-octet contents, direct Required callback failure/recovery, and
the independently classified constrained Binary `Value` copy. All exits 0 with
`C2_SINGLE_REQUIRED_DIGEST_BOUNDARY_COMPLETE`.

`traces-result.json`: **36** completed GDB runs (old red, original visitor,
five-shape visitor for every cell). Old red reaches real 8-byte Binary Adjust.
Visitor read does not. Its deliberate callback allocates a separate 64-byte
buffer, raises `Storage_Error`, leaves the destination null, preserves Binary
source octets/count, and permits another visit in the same process. These visitor
traces contain no controlled-exception transformation breakpoint hit.

Instrumentation is the validated C1 thread-local real-NULL interposer plus its Ada
allocator wrapper, copied unchanged. It disarms before real allocator failure;
no Ada exception is raised across a C allocator frame. Logging/check counters
are preallocated/nonallocating while armed; formatted reporting occurs after
disarm. The wrapper observes linked Ada allocation sites relevant to the target
copy, not every internal allocation of every shared runtime. Semantic by-reference
reasoning and the inspected runtime implementation supplement that measurement;
zero observed calls alone is not a universal no-allocation theorem.

The debugger stops at the actual C failure-arming function and records callers,
size, owning function and exception transformation. The original model-only
case's first allocation is demonstrated to be the target copy rather than assumed
from an inherited ordinal. Addresses and allocation ordering are observations,
not production error-classification mechanisms.

## 6. Actual generated codec experiment

`base-generation.*` uses the existing Task 097 generator and original schema to
regenerate the exact C1 model; the three model source files compare equal.
`shapes-generation.*` uses a scratch schema extending the same payload with
controlled Choice fields covering all storage forms plus repeated DateTime and
Duration. The actual generated models/codecs retain Float32, Float64, concrete
Record with Binary, constrained Binary `Digest`, named Choice and nested Records.

Only the experimental repeated-member encode traversal is changed: **10**
`Add_Item (Model.Element (...))` sites become child visitor calls. A child context
clause is added. Direct Required access and every other codec operation remain
unchanged. Child units are separately added. The parent model is never rewritten
to assemble the prototype. `prototype-changes.json` records generator identity
and replacement count. Scratch-only generation is not modification of the
committed renderer or its capability/readiness rules.

`integration{14,16}` uses fresh independent provider/support sources and independent
consumer object directories. The provider is **pristine**, with string source
SHA-256 `e9c95ad6b0899821852a787c586bb772017fabee1a2a852d5edf21f026687e3b`;
the C1 candidate patch is not applied. Both compilers complete all six
policy/optimization cells each:

* 2 independently authored valid documents: populated and minimum/empty optional
  forms. Expected typed values and array order compare independently in Python.
* 10 independently authored invalid documents: Binary spelling, Choice shape,
  DateTime, Duration, missing minimum, bounded overflow, constrained Binary length,
  Float32, Float64 and malformed JSON. Each requires explicit atomic-rejection
  marker and unchanged retained typed value.
* Invalid typed output preserves the caller's sentinel.
* First observed encode allocation failure propagates `Storage_Error`, preserves
  retained input/output state and completes same-process decode/encode recovery.

There are **180** successful integration build/run commands: 12 builds and 168
executions. `roundtrip-result.json` adds **24** typed re-decode/re-encode executions
of the emitted populated/minimum documents. `exact-values-result.json` compares
all 24 initial outputs recursively with IEEE binary64 bit comparisons for numeric
values, including negative zero; the Float32 witness is exactly representable
1.25. These are composition tests, not the full numerical corpus or real-UCI
service qualification.

`codec-callback-traces-result.json` adds **12** successful GDB executions. Each
arms failure at the **actual Picks visitor callback boundary** using debugger
test state, not an assumed allocation budget:

* GNAT 14 first reaches an **8-byte Digest.Value / Binary_Vectors.Adjust** inside
  `Add_Item -> To_JSON_ArraySample -> To_JSON_PathLength -> To_JSON_Digest`.
* GNAT 16 first reaches a **32-byte emitter Create_Object string construction**
  inside the callback. Some visitor frames are inlined at O2.

All propagate `Storage_Error`, preserve the input/output publication state, and
complete the same-process recovery marker with inferior exit 0. The GNAT 14
8-byte request is **not** the old element-result copy: the call stack explicitly
contains `Digest.Value` after entry to `Add_Item`. Allocation size alone cannot
identify ownership. `codec-callback-classification.json` records that distinction.

## 7. Parent preservation and external-consumer viability

Original C1 parent hashes:

| File | SHA-256 |
|---|---|
| `programs-oam.ads` | `5c137a617f91a78987137b0148dac5bf620f15fe52d14e301e24804db52833ed` |
| `programs-oam.adb` | `e6a46dce8733d4aee15051bfa4dcdf56ec2c633415475f4e2015f2148a796ba3` |
| Original-model child spec | `c585e227b0da3ba3d69b162451a8cce34cf4439a6db3faa82b3f5a1ca3d7aac0` |
| Original-model child body | `7f3e84a50e1998d98d3587b9eb69588573f2414ea48b9b780ccaf881ec3a8624` |

`parent-comparison.json` confirms exact original regeneration. The 36 observations
in `preservation-result.json` cover 32 original parent file comparisons across
the regenerated baseline, 12 compiler cells and two Alire consumers, plus four
pristine provider-source comparisons. Existing `Element`, `Length`, `Append`,
`Clear`, `Reserve_Capacity` and `To_Sequence` remain unchanged, including which
operations exist for each shape. The original C1 consumer compiles unchanged.

The five-shape model intentionally has added schema members, so it is **not**
byte-compared to the original model and is never represented as the same schema.
For that schema, experimental and untouched generated parent files remain equal.

External custom GPR projects with normal `Source_Dirs (".")` find and compile the
child using GNAT's conventional filenames, without exposing private storage or
requiring a custom naming map. Both compiler versions compile the child's access
to every required private representation. Parent units elaborate before their
child; the model never withs its child, so no new parent/child elaboration cycle
is introduced. The codec withs the child in its body.

`alire-result.json` records successful independent `alr build`, compiler identity
inspection and execution with **GNATLS 14.2.0 and 16.1.0**, three exit-0 commands
per consumer. Manifests pin the actual compiler and GPRbuild 26.0.1. Alire settings,
mutable cache and generated config belong exclusively to C2. Initial toolchain
selection also downloaded native tools inside C2; the recorded actual GNATLS
output, not the directory/cell name, establishes the compiler consumed. No
historical cache is used as mutable build storage.

This is **additive API surface**, not overall generated-API identity. A production
corrective task must reserve `Codec_Views` against schema declarations and other
children, check case-insensitive identifiers and filename collisions, and include
both new files in artifact lists/manifests and package layouts. Consumers with
explicit `Source_Files` lists must add the files; source-directory consumers need
no naming customization. No claim is made that all custom consumer file lists
magically discover an unlisted child. The experiment proves a viable normal
consumer path, not production artifact plumbing or a completed collision audit
implementation. No additive operation in the parent is needed for this proof.

## 8. Static copy-boundary inventory and remaining blockers

The source audit covers the model storage renderers (`render_sequence_storage_*`,
`render_bounded_*`), `render_unbounded_helper`, `repeated_member_render`, primitive
carrier rendering and the complete actual generated codec. Shared callable/name
policy is in
`/home/zboll/git/ams-gra-codegen-oms-task097/crates/codegen-core/src/backend_names.rs`.
Task 040 occupancy and initialization rules are preserved, not bypassed.

| Exact generated operation | Controlled involvement / value return | Can copy allocate? | Read-only alternative / current Task 097 path |
|---|---|---|---|
| `Add_Item (Model.Element (...))` | Record/Choice with Binary; returns element | Yes, old 8-byte Adjust red | Selected child visitor; actual encode path, fixed only in scratch |
| `Add_Item (Item.UB1.Required (Index))` | Same by-reference element; no result | No element-result copy | Already direct read; actual encode path unchanged |
| `To_JSON_Digest`: `Encode_Hex_Binary (Model.Value (Item))` | Returns controlled `Binary_Vectors.Vector` from private carrier's `Data` | **Yes**, independent 8-byte Adjust demonstrated in all 12 supplemental/debugger cells | A child scalar/carrier query could read private `Data`; not implemented/qualified; actual encode path **remaining boundary** |
| `Encode_Hex_Binary (Item.Data)` and octet iteration | Tagged vector argument, scalar octets; returns emitter value, not Binary | No extraction copy of Binary; output construction can allocate | Already direct read of public Binary; actual encode path |
| `To_JSON_ArraySample (Element_Item)` -> `To_JSON_PathLength (Item.Nested)` | By-reference input composites; emitter value result | No required input copy; emitter result needs separate accounting | Already direct field/callback input; actual encode path |
| `Model.Value` of checked Float32/Float64; derived float conversions | Elementary result, no controlled component | No controlled copy | Existing scalar operation; actual encode path |
| `Model.Value` of DateTime/Duration/checked String and `To_String (Item.Text)` | Returns `String`, not controlled model object | Result construction/string allocation remains fallible | No controlled-vector extraction; existing API; temporal/direct String paths exercised |
| `To_JSON_*` local `Result`, return `Output.Value`; `Output.Create (JSON_Items)` | Tagged emitter composite contains controlled Unbounded_String; value returns | Controlled adjustments/construction are not removed or fully qualified by visitor proof | Owned-output builder design requires separate audit; all actual encode paths |
| `Output.Append` / `Set_Field`, emitter `Temporary : Unbounded_String := Container.Text`, assignment back | Controlled string temporaries/assignment; not model return | Refcount/clone/growth behavior provider-dependent; construction can allocate | Independent output ownership/atomicity qualification remains; actual encode path |
| `Sequence_B0` / `Sequence_Picks` bounded `Values := (for ... => Decode_Item ...)`, `Construct -> To_Sequence` | Arrays/slots of controlled composites, value-return functions and assignments | Yes, construction/copy boundaries remain | Decode, **not encode**; actual round-trip path exercised, no exhaustive failure sweep |
| Positive-min `Required := (for ... => Decode_Item ...)`, `Append (Additional, ...)`, returned pair aggregate | Controlled arrays/vector and composite returns | Yes | Decode, not encode; no blanket safety claim |
| `Decode_Hex_Binary return Result`; `Model.Create (Octets)` | Controlled vector/carrier results; aggregate copies | Yes | Decode, not encode; unchanged |
| `From_JSON_PathLength`, `From_JSON_ArraySample`, `From_JSON_LosslessEnvelope` returned aggregates | Nested controlled Record/Choice/sequence composites | Yes, regardless of possible compiler copy elision | Decode result construction/publication qualification remains |

`digest-traces-result.json` has 12 call-site-attributed real-NULL reproductions of
the separate `Digest.Value` copy: `Value -> Binary_Vectors.Adjust -> 8 bytes`.
These localized cases propagate `Storage_Error` and recover, unlike the old
Record/Choice result-copy red. They are not evidence that every surrounding
aggregate/result transfer is safe. In actual codec callback tracing the GNAT 14
Digest copy is reached; GNAT 16's earlier emitter allocation is a different site,
not proof the Digest copy is absent. Remaining controlled-result ownership must
be independently qualified or redesigned before promotion.

No automatic refactoring of these sites was performed. A successful sequence
visitor removes exactly one class of unsafe read boundary; it does not certify
all helpers returning controlled values.

## 9. Provider versus model boundary

The unchanged C1 finding remains separate: pristine GNATCOLL mutates the XString
small/big tag before failed ordinary string growth; invalid finalization can
replace `Storage_Error`. The C1 transition candidate repairs that localized
publication order and demonstrated subsequent provider success. It is neither
edited nor promoted here. Provider-only and ownership traces remain original C1
evidence, with their verified hashes.

C2 proves that model **read traversal** can avoid constructing an allocating
element result while keeping retained source ownership intact. Its callback can
fail independently, with unchanged source and subsequent success. Neither fact
repairs the pristine provider, closes other controlled-copy boundaries, or permits
internal errors to be normalized to a resource error. Integration uses simple
independent JSON and pristine provider sources, not a new provider allocation
sweep or a combined full-resource claim.

## 10. Completed matrix, evidence and follow-on

All selected proof campaigns have definitive completion results:

| Result manifest under `/home/zboll/task097-scratch/c2/` | Completed coverage |
|---|---|
| `matrix-result.json` | 12 cells; old red plus original and five-shape visitor |
| `supplement-result.json` | 12 cells; all single forms, Required recovery, Digest boundary |
| `traces-result.json` | 36 debugger runs, all GDB and inferior exits 0 |
| `digest-traces-result.json` | 12 independent Digest-copy debugger runs, exits 0 |
| `codec-callback-traces-result.json` | 12 actual codec boundary injection/debugger runs, exits 0 |
| `integration-result.json` | Both compilers, all policies/O0/O2; 180 exit-0 commands |
| `roundtrip-result.json` | 24 explicit typed re-decode/re-encode executions, exits 0 |
| `exact-values-result.json` | 24 exact numeric/negative-zero/order comparisons |
| `final-controls-result.json` | 12 cells × 3 isolated controls; 72 exit-0 commands, plus 24 required exit-4 negative compilations |
| `alire-result.json` | Both pinned compilers; 6 exit-0 commands |
| `preservation-result.json` | 36 exact parent/provider observations |

Negative compilation commands intentionally exit **4** and require the specific
read-only/accessibility diagnostics. `controls-result.json` is retained as an
**unsuccessful earlier alternate-reference assumption**, superseded for the
selected visitor proof by `final-controls-result.json`; it is not relabeled a
pass. Initial matrix/controls setup failures (missing `Interfaces.C`, missing
allocator wrapper elaboration in standalone control mains) are preserved under
`*-attempt01/02*`. No missing result or missing exception output is counted as a
pass. Successful executions require their explicit completion markers and exits.

Deterministic sorted SHA-256 inventories and completion records are in
`final-source-manifest.json`, `final-evidence-manifest.json`, `manifest-hashes.json`,
`completed-exits.json` and `decision.json` in C2 scratch. Source inventory includes
the decision record, actual scratch source files, scripts, schemas, GPR/Alire
configuration, tool identities and compiler runtime sources. Evidence inventory
includes failed attempts rather than hiding them. Its own inventory/check files
are excluded to avoid recursive self-hashing. Top-level matrix-result hashes are
recorded below; identical summaries can intentionally have the same digest, so
per-command logs and the full evidence inventory provide attribution.

| Result | SHA-256 |
|---|---|
| Visitor matrix | `bcc7647c2084608d566108e3901f1f91fc256d01cd3f23bfd134d851c75f26ce` |
| 36 visitor/red traces | `c89187557c3e7ee85e7f010d60fcb50920ec9769cdb078ed8666b7d207d754e9` |
| Codec callback traces | `23e0f864782c87b4025188d2873dc75deeaf871dc8fff094aec9d7281778768e` |
| Supplemental cells | `b258a1a5f1949f5d4d9cd1dad1740279a01a1225504c09f02bbf248d3b4e3d9b` |
| Typed round trips | `51af0c4cb52306a75d4ae1f159c54b0e90aa2eba03b3433c311da62cb6830b08` |
| Exact values | `838c005a3637c8c4f424943d251f2d7d0aae8ac4eb73c3919bd274ddd4bb46d7` |
| Preservation | `60ae125b118c0983b19dca405768da9935f9889a717d84ef87baa63ab298712e` |

### Production corrective task, not performed here

1. Independently review this visitor's type classification, mutation contract,
   public-child naming, and the rejected access-exposure experiment.
2. Add narrowly scoped child artifact rendering using model storage metadata near
   `render_sequence_storage_*` and `render_bounded_*` in
   `/home/zboll/git/ams-gra-codegen-oms-task097/crates/backend-ada/src/lib.rs`,
   **without changing their parent output bytes or storage representation**.
3. Update `generate_service_codec` artifact/context composition and
   `repeated_member_render` encode calls in
   `/home/zboll/git/ams-gra-codegen-oms-task097/crates/backend-ada/src/service_codec.rs`.
   Preserve direct Required access. Add the two child source artifacts only where
   needed; maintain artifact atomicity and no-codec output preservation.
4. Update shared generated-name/file collision checks, artifact manifests,
   packaging and explicit-file-list consumer tests. Account for the **additional**
   API rather than declaring total generated API identity.
5. Resolve/audit `Digest.Value` and the remaining controlled-result/aggregate
   inventory independently. Do not catch `Program_Error` as a substitute.
6. Qualify the provider transition separately, then rerun the complete end-to-end
   resource, numerical, malformed-input, assertion-policy, compiler, consumer,
   source-preservation and selected real-service gates on final production sources.

**Final disposition: ADDITIONAL_COPY_BOUNDARIES_FOUND.** Copy-safe repeated-element
traversal is proven for the tested generated categories and integration path.
Production Float64 publication remains unauthorized.
