# Task 097-C5 — resource-safe lossless SAX ingress

## Scope and local decision

The lossless Float64 decoder now selects a separate immutable SAX-backed ingress.
It does not construct a GNATCOLL JSON DOM, provider occurrence tree, JSON_Array
value, or XString. Original input and numeric spans remain authoritative. Parent
model APIs, public codec signatures, legacy portable/Float32 ingress, C3 visitors,
C4 output validation, and width-specific floating conversion are unchanged.

**This checkpoint addresses ingress ownership, not all Decode resource safety.**
Typed Binary/optional/Choice/Record/sequence construction and result transfer still
include demonstrated allocating controlled adjustments. No readiness expansion,
provider patch, publication, push, PR, merge, rebase, or other-worktree edit is
authorized or performed.

**Disposition: INGRESS_RESOURCE_BOUNDARY_CLOSED_LOCAL.** All focused mandatory
gates have definitive completion records. Downstream model copies remain red.

Evidence: `/home/zboll/task097-scratch/c5/`. Final disposition and commit identity
are recorded by the completed `decision.json` and post-commit `git-closure.json`,
not inferred from historical passes or a partially completed worker.

## 1. Starting identities and prior preservation

| Item | Verified identity |
|---|---|
| Worktree | `/home/zboll/git/ams-gra-codegen-oms-task097` |
| Branch | `feature/097-ada-float64-codec` |
| Starting C4 HEAD | `2b230b3944f7f02a8b783496ef8da69001c93b9d` |
| Starting tree | `a0995cf5f97e54d8d2e4f4a3eb30cdfdec63ffcf` |
| Freshly fetched main/FETCH_HEAD | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| Initial worktree/index | Clean |

C1 `276fa90f1b7dd8aaad7af66bd2810e009c7c457a`, C2
`6252830f38df9c43cc715861640615974344b769`, C3
`a8aee97d9028a4cbebdc736d12df6d9c439ef81a`, and C4 remain ordinary ancestors.
`ownership.json` records initial state. C1–C4 reports and actual sources were
reviewed before design. `c4-verification.json` verifies every **1,205 source** and
**195,568 evidence** entry, without rewriting old manifests:

```text
8c6b49f45b4ed8510182e98b4c290321a2f33db8404ab5c1be7fc59357201738  C4 source
93393fae52ecad9566801a8e8dd8b26df5a12a0862dcfaa0c1b100b9a7623725  C4 evidence
```

Historical C1/C3/C4 failed attempts, Program_Error outcomes and SIGABRT remain
separate adverse evidence. New passes qualify only the frozen C5 sources and
recorded isolated dependencies.

## 2. Original ownership call graph and independent reds

```text
public Decode_* -> Require_Platform
 -> Occurrences.Read
    -> original SAX pass, numeric-token shadow substitution
    -> Guard.Read(shadow) -> provider DOM construction -> Validate_Tree
       -> Get(array) -> JSON_ArrayDA -> Vect_Pkg.Adjust
    -> second original-input SAX pass
       -> Decode_As_String -> Un_Escape_String -> XString transition
       -> recursive metadata JSON objects/maps/arrays
       -> Create(array) / value-returning array copies
 -> From_JSON_* -> scalar/width-specific conversion
 -> Model.Create / optional / Choice / Record / sequence construction
 -> public typed result / caller assignment
```

The old occurrence node owns a provider JSON_Value metadata object with kind,
first/last and data fields. Numeric data is a STRING token, not a rounded numeric
shadow value. Object insertion uses decoded keys and replaces whole child nodes;
arrays contain provider JSON_Value nodes. JSON_Value Adjust increments provider
references, but JSON_Array contains an Ada vector whose Adjust allocates. XString
construction has its independent unsafe representation transition.

Before production edits, fresh old-source GNAT14/16 builds reproduce separately:

* **Unescape:** original committed C1 provider main, actual 28-byte failed
  `Convert_To_Big_String` allocation, invalid `is_big=true` representation with
  inline Unicode bytes interpreted as pointer `0x8fb8efb09b9ff020`, cleanup fault,
  Program_Error, retained sentinel and same-process recovery.
* **DOM inside actual lossless decode:** source-boundary failure in shadow
  Guard.Validate_Tree's Get(array), controlled vector adjustment, Program_Error
  and subsequent recovery. No standalone-Guard result is substituted for the
  actual generated decode witness.
* **Typed carrier:** generated decode reaches Digest adjustment/Binary_Vectors
  allocation, Program_Error and recovery. GNAT14's first observed copy is
  Model.Create; GNAT16 may first expose a later typed repeated-element copy.
  They are recorded by actual stack, not asserted identical from an ordinal.

`old-red-result.json`, per-compiler `old-*/` traces, and
`unescape-representation/` preserve first allocation, representation, exception
transformation, disarming and cleanup. The original provider remains pristine.

## 3. Source-backed new ownership inventory

New support units:

* `/home/zboll/git/ams-gra-codegen-oms-task097/ada/oms-json-support/lossless/ams_gra_oms_json_lossless_ingress.ads`
* `/home/zboll/git/ams-gra-codegen-oms-task097/ada/oms-json-support/lossless/ams_gra_oms_json_lossless_ingress.adb`

| Operation | Storage/ownership | Fallible controlled Adjust? |
|---|---|---|
| C4 Validate_Encoded_Document | scoped limited reader/parser, bounded storage | No |
| Document construction | initialized limited ordinary record with atomic refs and null pointers | No |
| Original input | separately allocated canonical-bounds String | No |
| Decoded key/string storage | separately allocated document-length byte arena | No |
| Node storage | preallocated ordinary array of scalar records | No |
| Array indices | preallocated ordinary integer array | No |
| SAX events | scalar records with original byte spans | No |
| Node/array handles | document pointer + stable ID, nonallocating atomic ref operations | No allocation in Adjust |
| Get string/numeric token | returned ordinary String slice | May allocate ordinary result storage; not controlled Adjust |
| Float32/64 conversion | unchanged width-specific qualified helpers | Existing numerical resource contract |
| Typed model constructors/results | original model vectors/aggregates | **Yes, downstream blockers remain** |

The generated backend chooses the new import/Ingress alias semantically when it
emits Float64. The body differs only by that import/alias, not a textual search
of generated code. Float64-free and legacy profiles retain their original
source bytes. Existing lossless support GPR Source_Dirs includes the new units;
no shared provider project is edited.

### Arena lifetime and exception safety

Each Value retains a private Document pointer and stable integer ID. Value_Array
contains such a Value, not an owned JSON array. A document has no references back
to handles, hence no cycles. Returning children or array views retains the same
owner, so they may outlive parent handles without dangling storage. No raw
pointer, access-returning borrow, unchecked lifetime conversion, or mutable node
is public.

Read validates first, then constructs an extended return Value. Its fully
initialized owner is installed before allocating substorage. Each access field
starts null and is changed only by a successful ordinary allocator. Failure at
any later allocation finalizes that return owner; uninitialized byte contents
are never interpreted as ownership metadata. Last-owner cleanup frees ordinary
input/byte/node/index arrays and then the document, without tree recursion.

Actual Adjust consists only of a null check and
`System.Atomic_Counters.Increment`. Finalize first clears its handle, decrements
the counter and, if last, performs ordinary deallocations. Both qualified x86_64
runtime implementations use atomic integer intrinsics; no allocation, callback,
element Adjust, or resource exception is introduced. Runtime sources and
execution evidence are retained. Independent immutable handles support shared
reads; assignment to the same handle concurrently requires caller synchronization.

The fixed 32-bit reference counter follows the qualified runtime convention;
counter overflow from billions of simultaneously live caller-created handles is
not a new arbitrary-consumer guarantee. Generated decoder live handles are
bounded by the capped traversal/model construction, far below that limit.

### Bounded representation/work

Node/index capacity is `min (100_000, Text'Length)`, sufficient because each
value occurrence consumes at least one source byte. Capacity is allocated once;
there is no incremental node growth. Input and decoded bytes each reserve at
most 1 MiB; decoded length cannot exceed original string-token contents summed
across disjoint key/value occurrences. Pinned Reader owns another bounded input
copy. SAX state grows only with depth. Local JSON stack is 128 frames.

Objects use an AVL index over canonical member slots, maintaining logarithmic
lookup/insertion in distinct names and bounded AVL-height recursion (under 24
levels at 100,000 members), not uncontrolled JSON/member recursion. Array elements
are finalized into contiguous integer indices, so repeated indexed access is
constant time rather than linked-list rescanning. All nonselected duplicate
nodes remain ordinary arena entries until document cleanup; memory remains
bounded by **all original** admitted occurrences, not only survivors.

## 4. String decoding, original spans and JSON semantics

The narrow decoder runs only after C4 validates the exact original document.
GNATCOLL SAX still supplies grammar and original token boundaries. It does not
call Decode_As_String/Un_Escape_String or parse JSON grammar independently.

The decoder copies raw valid UTF-8, maps the eight JSON short escape spellings,
reads validated four-hex units and combines validated surrogate pairs into
Unicode scalars, then writes canonical UTF-8 into allocated byte storage. It
checks destination capacity before each write and advances used length only
after writing. Empty strings, escaped controls/NUL, quotes/backslashes and
inline/heap threshold lengths have no distinct representation state transition.
The backing byte arena is allocated before any decoded data is published.

Object names compare exact decoded bytes without normalization. Literal and
escaped-equivalent names use the same key. Duplicate insertion changes the
canonical slot's **Selected node ID**, replacing kind, original numeric token,
first/last and nested children together. It does not merge old children or match
numbers by global ordinal/bare field name. Both old and new subtrees already
passed complete grammar, Unicode and all-occurrence limits.

Map_JSON_Object enumerates only canonical members and the last selected subtree.
Object enumeration order is not a published JSON semantic; generated unknown-key
checking has no ordering dependence. Array order/indices are exact. Missing
fields remain distinct from present null. JSON integer, fraction/exponent,
string, Boolean, null, array and object kinds remain distinct.

Number nodes refer to original owned input byte ranges. Numeric_Lexeme checks
numeric kind and returns that exact slice; it never narrows through signed-64
or Long_Float. The signed-64 scalar Get remains a checked integer-only operation
for typed integral fields. Float32 still uses its qualified Number/Special
helpers; Float64 uses unchanged qualified strtod_l/Special conversion. Locale
ownership and floating-environment checks are not changed.

## 5. Differential and generated-code qualification

Checked-in runner:
`/home/zboll/git/ams-gra-codegen-oms-task097/scripts/task097/c5_ingress.py`.
Consumer:
`/home/zboll/git/ams-gra-codegen-oms-task097/tests/ada-float64-codec/c5_ingress.adb`.

Both full ingress matrices complete **487 cases in each of 12** GNAT14/16 ×
Normal/Check/Ignore × O0/O2 cells. The 487 comprise unchanged C4 **339** validator
cases plus **148** occurrence/string/association cases. They include:

* exact byte/depth/node limits, malformed JSON, all C4 UTF-8/surrogate controls;
* every BMP UTF-16 code unit outside the surrogate range, independent expected
  decoding, and paired non-BMP boundary scalars;
* raw non-BMP UTF-8, short escapes, empty/long strings, escaped-equivalent keys;
* complete duplicate subtree/kind/span replacement, nested equal bare names;
* shuffled high-member-count objects, repeated arrays and original numeric slices.

For admitted documents the old nonfaulted occurrence reader and new reader are
compared by path, kind, First/Last and payload, independent of member enumeration
order. Python strict JSON supplies a separate decoded structure/type oracle;
every surviving source span independently parses and every numeric payload
matches its exact original source slice. Invalid originals remain rejected.

An important distinction: **old Occurrences.Read already preserved large
integer-form Float64 tokens by shadowing them before Guard.Read**. Therefore
large-token success is parity with old occurrences, even though direct old
Guard.Read could not store them numerically. No false expected-difference claim
is used to excuse a mismatch. Duplicate node limits count all source values.

All **24** actual production-generated composition cells complete:

```text
GNAT 14.2.0 / 16.1.0
 × OAM / qualified urn:copyproof
 × Normal / Check / Ignore
 × O0 / O2
```

The fixture contains Float64/Float32, direct/constrained Binary, nested/inherited
Records, named Choices, optional values, repeated Binary-containing alternatives,
bounded/unbounded sequences, temporal fields and Unicode strings. Tests verify
ordinary decode/encode structure, exact values, cardinality and atomic rejection.
Additional generated semantics reject unknown members at root, Record, nested
Leaf, Choice and repeated-record levels; null/wrong-type members; quoted finite
floats. Escaped keys, arbitrary member order and complete duplicate Record
replacement succeed with independently checked output key uniqueness.

**312** numerical input cases (13 × 24 cells) traverse the complete generated
Decode/Encode path. Independent unchanged Task096 oracle checks exact binary64
bits for integer tokens beyond signed-64/unsigned-64, negative zero, minimum
subnormal, max finite ±values, normal/subnormal boundary and neighbors of 1.
Finite outputs must be actual JSON numbers; specials exactly "Infinity",
"-Infinity", "NaN". Another 24 escaped-equivalent duplicate numerical leaves
verify the last token/kind is used. No successful roundtrip alone is the oracle.

## 6. Allocation, ownership and cleanup evidence

Shallow Read has **nine** actual allocations: C4 reader+initial parser state;
owner; input; decoded bytes; nodes; array indices; second reader+parser state.
Depth32 has eleven; depth128 fifteen because **both** SAX passes grow. Keys,
strings, AVL insertion, duplicate replacement and element association do not
allocate separately. There is no arena growth mechanism to qualify.

Standalone ingress traces exercise **528** reached real-NULL failures across
12 cells on duplicate/escaped documents and depths32/128, with precise call
stacks, disarmed injector, Storage_Error, no partial Value and successful
same-process recovery. No controlled exception-transformation breakpoint fires.
Additional per-case budget sweeps and malformed/valid operations require their
explicit markers. Missing targets are not counted as failures successfully tested.

Handle/lifetime tests arm a zero-allocation budget while copying/assigning node
and array views, release the original root, then access retained children.
All 12 cells pass. Separate scoped owner tests observe **nine allocations and
nine frees** after root/array/100 child-handle copies leave scope. They pass both
compilers/all policies/O0/O2. Callback Program_Error propagates unchanged and
later reading succeeds.

Failure-cleanup traces track actual successful ingress allocation addresses and
frees across all nine failure positions in 12 cells (**108** cases). No ingress
owned allocations remain at unwinding completion. Runtime exception-occurrence
storage is deliberately distinct from document ownership; retained Exception_
Occurrence test diagnostics are not misreported as an ingress leak. This is
focused ownership proof, not a universal allocator/leak-sanitizer campaign.

Actual generated Decode is separately traced in all 24 cells. Armed counters
prove zero shadow Guard.Read, provider JSON_Array adjustment, Decode_As_String,
Un_Escape_String and old Occurrences.Read. Each new ingress group is injected
and must propagate Storage_Error/recover. Typed-model groups are classified and
their adverse outcomes preserved, not counted as ingress failures or passes.
`decode-audit-result.json` and `downstream-inventory.json` supply exact final
totals and per-stack attribution.

The completed actual generated-code audit has **30,288** baseline allocation
observations and **1,808** representative mechanism injections across all 24
cells. All **144** ingress mechanism-group injections propagate Storage_Error
with recovery. The separate downstream groups have **728 Storage_Error** and
**936 Program_Error** outcomes. Of those downstream Program_Error outcomes,
**834** reach recovery and **102 do not**: the retained-model integrity check
exits unsuccessfully after a typed composite/result-transfer failure. All 728
downstream Storage_Error outcomes recover. Those 102 adverse outcomes remain
explicit downstream blockers; a zero GDB command exit is not an inferior pass.
No SIGABRT is observed in corrected ingress/decode witnesses. These
are representative groups, not exhaustive per-ordinal proof. All full baseline
caller/type variants remain in the inventory; 97 attributed adverse downstream
families have concrete reproducer commands and smallest observed sizes.

## 7. Performance and bounded memory

`performance-result.json` measures per-child runtime/peak RSS without depending
on an unavailable external time command. `peak-storage-result.json` independently
tracks actual live requested malloc bytes under debugger execution.

For sorted unique objects at 1,000/10,000/50,000 members, GNAT14 observed roughly
0.007/0.016/0.051 seconds; GNAT16 0.004/0.012/0.042 seconds. These are local
observations, not hard realtime promises. Arrays, repeated duplicate subtrees
and long strings are measured separately. 50,000-member object input is 888,891
bytes, with **9,866,844** peak live requested bytes, **8,977,904** retained document
bytes and nine allocation calls, in both compilers. RSS includes process/runtime
baseline and allocator overhead and is labeled separately. No quadratic
member-scan or array-index scan is introduced.

## 8. Preservation and regressions

Fresh exact-C4 generator comparisons pass **105** cases: **86 byte-identical
generations**, **19 matching atomic rejections**. Parent Ada model/service APIs,
portable/qualified Float32/Binary/temporal/repeated codecs and Rust/C++ remain
unchanged. Fourteen additional actual lossless parent/API/view units across both
namespaces match C4 bytes; codec specs match; lossless bodies differ only in
the intended semantic import/Ingress alias.

Completed gates include formatting, workspace/all-target check, warnings-denied
Clippy, **785 component tests**, **26 CLI codec tests**, **37 CLI library tests**,
CI split/layout and **210 split mutation checks**. C3 visitor regressions and
96 expected negative compilations run on C5 sources. C4 339-case validator
matrices run fresh under both compilers/all policies/O0/O2; actual encode
final-validation failure/recovery is rechecked in 24 cells. Legacy 199-case
Guard Unicode corpus and support-boundary probes pass both compilers/all policies.

## 9. Remaining typed-model construction inventory

The ingress no longer owns provider-controlled arrays/strings, but model APIs
retain owned controlled vectors and composite values. Distinct remaining stacks
are inventoried in `downstream-inventory.json`, including:

* constrained Binary `Model.Create -> Digest/ExactDigestDA -> Binary_Vectors.Adjust`;
* optional Digest/Record assignment and aggregate/result adjustment;
* repeated constrained Binary indefinite-vector Insert/Append element copies;
* Choice alternatives and nested Leaf/Reading aggregate copies;
* bounded sequence slot/To_Sequence and Required-array composite adjustment;
* unbounded UB0/UB1 Additional vectors, vector header/element allocation during
  Adjust, sequence aggregate and enclosing result adjustment;
* decoded ordinary String/temporal construction, Binary temporary reserve and
  final public/caller result transfer, classified separately from ingress.

Validated scalar Float64 carriers have no allocating controlled storage. Their
checked conversion/platform behavior is unchanged; this does not make an
enclosing Binary-containing aggregate copy safe.

Minimal already-known carrier witness remains Model.Create on two Binary octets
with an 8-byte vector copy. Runtime/compiler optimization may expose ordinary
Storage_Error at another construction boundary or Program_Error at a composite
Adjust. Source-line breakpoints at an inlined function may land before the
intended allocation: only actual allocator stacks are used for attribution.
Full per-mechanism reproducer commands, smallest observed sizes, original/current
stacks and categories accompany the inventory. Legacy Guard/Float32 occurrence
DOM/XString hazards also remain outside the new lossless ingress.

**Recommended follow-on:** separately review in-place/limited construction or
model-owned builders for carriers, optional/Choice/Record aggregates and sequence
elements/results. Preserve checks and atomic result publication. Do not broaden
C5 into a model rewrite or translate Program_Error from text/site/injector state.

Complete final numerical, all-ordinal resource, real-service and hosted gates
remain publication prerequisites. No complete 27,167/20,474 campaign was rerun
or claimed necessary for this unchanged numerical boundary checkpoint.

## 10. Source and attempt integrity

`source-attempt01/` and its manifest freeze consumed production/test sources
before full workers. Production ingress/backend sources remain unchanged after
that freeze. Later edits are support README/decision documentation only. Builds
own private provider copies and task-owned TMPDIR; shared provider sources are
not patched. Original C4 evidence is reverified at closure.

Failed attempts remain explicit: Ada test identifier `Digits` was reserved and
corrected before freeze; performance attempted an unavailable `/usr/bin/time`
then used wait4; GDB `free` alias on GNAT16 required `__libc_free`; a cleanup probe
initially included separately owned runtime exception-occurrence storage and was
corrected using stack attribution. Downstream audit attempts using over-specific
caller grouping or imprecise inlined source targeting retain their failed/
interrupted records. A resumed immutable audit reuses only complete same-binary
trace records and retains all full stack variants, never changed production.

Final deterministic source/evidence inventories, completed exits, decision and
read-back verification are under C5. Manifest hashes are supplied in the handoff;
self-files/post-commit closure are excluded from recursive self-hashing.
The final normal commit and clean branch status are recorded in git-closure.json.

**Final disposition: INGRESS_RESOURCE_BOUNDARY_CLOSED_LOCAL.** No interpretation
of this checkpoint may imply complete Task097 Float64 readiness/publication.