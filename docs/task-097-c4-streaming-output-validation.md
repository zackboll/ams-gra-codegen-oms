# Task 097-C4 — streaming final-output validation

## Decision

**OUTPUT_VALIDATION_BOUNDARY_CLOSED_LOCAL.** The lossless emitter validates its
completed JSON text without constructing a provider DOM, copying provider JSON
arrays, decoding strings, or constructing an occurrence tree. The replacement
preserves lexical Unicode checks, strict grammar, complete consumption, byte,
depth and value-node limits. All focused compiler, differential, allocation,
composition, preservation and regression gates have completed.

This is **not** publication or complete Float64 resource qualification. Ordinary
Guard.Read/Write and decoder/occurrence construction retain separate resource
blockers. No C1 provider patch is integrated. No push, PR, workflow dispatch,
merge, rebase, force-push, upstream edit, or other-worktree modification occurred.

Evidence root: `/home/zboll/task097-scratch/c4/`.

## 1. Identity and preserved evidence

| Item | Verified value |
|---|---|
| Worktree | `/home/zboll/git/ams-gra-codegen-oms-task097` |
| Branch | `feature/097-ada-float64-codec` |
| Starting C3 HEAD | `a8aee97d9028a4cbebdc736d12df6d9c439ef81a` |
| Starting tree | `daae3481a4a818c0e3f46c8e63f6e8b09393e716` |
| Fetched main / FETCH_HEAD | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| Initial index/worktree | Clean |

C1 `276fa90f1b7dd8aaad7af66bd2810e009c7c457a`, C2
`6252830f38df9c43cc715861640615974344b769`, and C3 remain ordinary ancestors.
`ownership.json` records starting identity; post-commit `git-closure.json` records
the final normal local commit and clean status without circular self-identification.

The four prerequisite Task 097/C1/C2/C3 records and full C3 encoder-copy inventory
were reviewed. `c3-verification.json` verifies **1,911 source entries**, **135,297
evidence entries**, and **2,428 completed exits**, including expected negative and
atomic-rejection outcomes. Supplied SHA-256 values match:

```text
1e8a690490483caa83ae91204e5fdaf2193b84c8d3947be83f53e9ab70fd2b0d  C3 source
0614e3f24cf8feb6a15f0a0719d4460f7b0271715eb0aa0dd0ae7a8488ddc613  C3 evidence
e795849e993cad3ffd79adff0c4d2fa71c7e0780cd38b5e4f0729516a123f51b  C3 completed exits
```

All **88,716 C3 baseline stack observations**, **990 representative mechanism
injections**, **78 Program_Error results**, and **one SIGABRT** remain original
adverse evidence. They are not overwritten, relabeled, or mistaken for corrected
source results. The 911 C3 Storage_Error/recovery observations also retain their
actual ownership and source identities.

## 2. Original causal chains and fresh red reproductions

C3's generated encoder called:

```text
Encode_* -> typed structural emitter -> Emitter.Write
 -> Occurrences.Read -> Guard.Read -> GNATCOLL.JSON.Read
 -> Guard.Validate_Tree -> GNATCOLL.JSON.Get(array)
 -> JSON_Array controlled adjustment -> Vect_Pkg.Adjust
 -> failed real allocation -> Program_Error
```

`Guard.Read ("[1]")` separately demonstrates the controlled-array copy. Fresh C4
traces preserve the **48-byte GNAT14** minimum request; the fresh GNAT16 build
requests **64 bytes**, reflecting compiler/runtime controlled-pool overhead.
The mechanism is established by stack, not an assumed portable size.

The actual C3-generated populated encoder is rebuilt against exact C3 support in
isolated C4 trees. `old-array-result.json` contains **12** fresh compiler/policy/
optimization source-boundary injections, with actual allocation size, controlled
exception transformation and same-process recovery. GNAT14 reproduces the
**72-byte** request; GNAT16's observed size is separately recorded. The old
function-result object owns the failed vector copy; it is not a SAX parser-state
allocation and not a caller assignment after Encode returns.

Fresh provider traces keep two ordinary-construction failures separate:

* `Un_Escape_String -> Append -> Reserve -> Convert_To_Big_String` still mutates
  XString representation before allocation. Both compiler old-encode witnesses
  reproduce Program_Error during cleanup and recover in-process.
* Qualified-key `Set_Field -> To_XString -> Set -> Reserve -> Convert_To_Big_String`
  reaches the distinct occurrence-tree key construction boundary. Exact scoped
  allocation targeting in `provider-key-final05/` reproduces Program_Error on
  GNAT14 and **SIGABRT / double-free-or-corruption diagnostic on GNAT16 Check/O0**.
  That abort is recorded as a definitive adverse result, not recovery.

The original committed C1 provider-only reproducer is also copied unchanged into
fresh C4 consumers, compiled with both compilers, and injected through the validated
C1 one-shot real-NULL interposer. Both reach its actual **28-byte** unescape
allocation, controlled cleanup transformation, original Program_Error, unchanged
sentinel and subsequent provider success (`c1-exact-result.json`). No old
allocation ordinal is imported as a site identity.

## 3. Replacement architecture

Additive API in
`/home/zboll/git/ams-gra-codegen-oms-task097/ada/oms-json-support/src/ams_gra_oms_json_guard.ads`:

```ada
procedure Validate_Encoded_Document (Text : String);
```

Its implementation in the sibling `.adb`:

1. Rejects document length over 1,048,576 **before reader allocation**.
2. Invokes the **unchanged existing** `Validate (Text, Document => True)` scanner
   for UTF-8, Unicode escapes/surrogate pairs, controls, and lexical nesting.
3. Constructs a scoped `GNATCOLL.Buffer.Reader` and limited `JSON_Parser`.
4. Iteratively consumes pinned `GNATCOLL.JSON.Parse_Next` events until DOC_END.
5. Uses a fixed, initialized **128-frame** array of two Booleans to distinguish
   object keys from values, count actual value occurrences, and track containers.
6. Returns no value/tree and performs no number or string token conversion.

This bookkeeping is not a second grammar parser. The pinned SAX implementation
checks number/string/literal syntax, separators, trailing commas, delimiters,
root value and trailing data. Keys and values share STRING_VALUE events; the
frame's `Expect_Key` state toggles only for an object's key and complete value
start. Collection-end events only pop a frame. Object names and collection ends
are **not** JSON value nodes. Root values and collection starts each count once.
No token text, key table, numeric shadow, recursive traversal or unbounded local
stack is created. Unexpected public separator events remain Program_Error rather
than being normalized into a resource error.

Only `/home/zboll/git/ams-gra-codegen-oms-task097/ada/oms-json-support/lossless/ams_gra_oms_json_emitter.adb`
switches its final `Write`: after `To_String (Item.Text)`, it calls the new guard
procedure, then returns Text. Its old occurrence-tree local and context clause
are removed. Guard validation errors retain the emitter's Validation_Error
translation. **Storage_Error and unrelated internal exceptions are not caught or
rewritten.** There is no partial returned String if validation fails.

Typed fragment construction, expansion bounds, per-operation node/depth checks,
provider escaping of scalar strings, numerical formatter, and C3 Codec_Views
visitors are unchanged. There is still no public arbitrary raw-JSON fragment
constructor. No `Encode_*`/`Decode_*` signature or generated child artifact changes.

## 4. Before/after validation-contract matrix

| Protection | C3 final validation | C4 final validation |
|---|---|---|
| Document bytes | Guard.Read precheck, plus emitter construction checks | same 1,048,576 check before reader; emitter checks retained |
| UTF-8 lead/continuation/range/overlong rules | existing lexical scanner, decoded DOM string checks | **same scanner**, no replacement Unicode algorithm |
| Unicode hex escapes and surrogate pairing | scanner, provider decoding, tree checks | same scanner; token strings are never decoded |
| Raw string controls/NUL | scanner/provider rejection | same scanner/SAX rejection; escaped NUL remains legal |
| Complete JSON grammar | provider DOM parse and occurrence SAX parse | pinned Parse_Next grammar only; complete DOC_END required |
| Delimiters/separators/trailing values | provider parse | same pinned SAX grammar state machine |
| Max simultaneous collections | scanner limit 128 | identical scanner, explicit fixed-frame cap 128 |
| Value nodes | final DOM traversal plus occurrence parse | at most 100,000 **lexical value occurrences**, never keys/end events |
| Duplicates | decoded last-value-wins tree; occurrence pass counts occurrences | syntactically legal, no rewriting; lexical values count; generated output keys verified unique |
| Integer/Float64 token | old Guard.Read's numeric DOM could reject integer overflow | syntax only; **no signed-64, Long_Float, Float32 or other numeric conversion** |
| Number versus string | DOM/occurrence kind | SAX event kind preserved; quoted numeric text is still STRING_VALUE |
| Output publication | after constructing/discarding validation tree | only after complete streaming validation succeeds |
| Parser resources | DOM and occurrence trees/array copies, unsafe provider transitions | bounded reader copy + scalar parser-state allocations, fixed local frames |

Duplicate objects require explicit accounting: a DOM may collapse many repeated
keys before node traversal, but C3's subsequent occurrence pass already counted
their value occurrences. The new validator preserves that occurrence count for
the lossless output boundary. It is intentionally not claimed byte-for-byte
behavioral identity with standalone Guard.Read on all duplicate-node-limit inputs.
Guard.Read/Write behavior itself is unchanged. Generated encoders do not emit
duplicate keys; tests decode keys independently to detect escaped-equivalent
duplicates rather than comparing only raw spellings.

## 5. SAX ownership and bounded allocation proof

Pinned GNATCOLL 26.0.0 source is inspected and inventoried, not assumed safe:

* `Buffer.Open_String` creates a limited Reader result in place and allocates a
  **document-length copy** (`new String (1 .. Str'Length)`). This is explicit
  O(document bytes), **not hidden or described as allocation-free**. It is bounded
  by 1 MiB. It normalizes source bounds, including non-one-based Strings.
* Reader ownership starts with null storage. A failed allocation leaves null;
  successful storage is freed by its finalizer. There is no allocating Adjust.
* Parser starts with null State and lazily allocates 32 scalar states. At growth,
  it allocates a doubled scalar array **before freeing/replacing the old pointer**.
  On failed growth `State_Current` was incremented, but finalization only frees
  the still-owned pointer and never indexes with that counter. No controlled
  element/result copy is involved.
* With a root state plus at most 128 containers, capacity grows 32→64→128→256;
  state work is O(capped depth), not O(nodes). Both parser and reader are limited
  controlled objects with strictly local lifetimes and no exposed borrow.
* `Parse_Next` does not call Decode_As_String or convert numeric tokens; its
  returned event is an elementary-field record. It does allocate exceptions for
  invalid input as usual; this does not require constructing a DOM.

The lexical scan uses offsets with length-checked lookahead, not `Text'Last+1`.
Differential tests include String bounds 1, 17 and ending at Integer'Last.
All lengths are checked before the parser's offset work. Fixed-frame/node
counters are bounded, checked before increment, and explicitly initialized.

Actual valid-document allocations at depths 1/31/32/33/64/128 are respectively
**2/2/3/3/4/5**: reader copy, initial parser state, and necessary growth arrays.
`validation-traces-result.json` has **12** compiler/policy/O0/O2 cells, **72**
baseline depth traces and **228** reached one-shot real-NULL failures. Every
failure propagates Storage_Error, disarms, cleans up safely and succeeds in the
same process. No controlled-adjust/finalize transformation breakpoint hits.
Unrelated Program_Error controls remain Program_Error.

`emitter-growth-result.json` additionally injects parser growth **inside actual
Emitter.Write** at depths 32, 33, 64 and 128: **48** reached growth failures across
both compilers/all policies/O0/O2. It requires unchanged unpublished output and
same-process Emitter.Write success after failure. Boundary injection is at the
actual growth allocation, not a shifted budget.

The C1/C3 single-shot interposers are copied unchanged. C3 fixed thread-local
logging neither formats nor allocates while armed; real malloc/realloc returns
NULL, sets ENOMEM and disarms before runtime exception propagation. Independent
thread isolation/count/failed-realloc ownership controls pass again. Debugger
diagnostics run outside program-owned logging. No production error classification
uses text, ordinals, locations or injector metadata.

## 6. Independent differential and correctness evidence

`scripts/task097/c4_differential.py` and `tests/ada-float64-codec/c4_validate.adb`
are task-owned production-boundary regressions. The test oracle uses Python's
strict JSON grammar, strict UTF-8 decoding, independent surrogate-scalar check,
and an independent bounded value walk. `ObjectPairs` preserves duplicate members
for occurrence counting; numeric lexemes are retained rather than narrowed.

All **12** GNAT14/16 × Normal/Check/Ignore × O0/O2 cells pass **339** independent
cases each. The corpus covers:

* empty/whitespace roots, all scalar roots, objects/arrays and nesting;
* exact/over byte, depth and node limits, arrays and objects, 50,001 object keys
  that would fail if keys counted as nodes, repeated-key occurrence node limits;
* depth 30–33, 63–65, 127–129; exact 1,048,576 bytes and adjacent lengths;
* truncated input, trailing values, wrong separators/delimiters, trailing commas;
* leading-zero, malformed fraction/exponent/literal spellings;
* delimiter-looking string content, quotes, backslashes, valid UTF-8/non-BMP;
* illegal UTF-8 leads/continuations, overlong forms, UTF-8 surrogate ranges,
  out-of-range scalars, lone/reversed/mispaired escaped surrogates;
* raw controls/NUL versus escaped controls/NUL;
* a deterministic independent randomized JSON corpus and malformed trailing roots.

For **329 overlapping cases per compiler**, old Guard.Read and new validation
accept/reject outcomes agree (`old-differences.json` is empty for both). Large
numeric tokens outside signed-64 or old DOM conversion range are **explicitly
excluded from overlap**, not disguised as parity. New accepts valid JSON number
syntax including `9223372036854775808`, `18446744073709551616`, `1e9999`, while
quoted numbers remain syntactic strings. Validation alone does not make a string
a supported typed Float64 value, nor promise every syntax-valid number is finite.

Actual production-generated codec execution covers **24** cells: GNAT14.2.0 /
16.1.0 × OAM / `urn:copyproof` × all three policies × O0/O2. C3's full fixture
retains Float64/Float32, direct and constrained Binary, optional/nested/inherited
Records, named/repeated Choices, bounded/unbounded Records, temporal and string
fields. All populated/minimum/duplicate documents and malformed inputs complete
their expected explicit markers and atomicity checks. Independent output parsing
checks decoded key uniqueness, exact member counts, structure, kinds and order.

`numerical-result.json` adds **336** representative numerical outputs (14 patterns
× 24 generated-code cells), passing through actual generated Encode and final
validation. The unchanged independent Task096 integer-rational oracle recovers
exact binary64 bits for negative zero, minimum subnormal, maximum finite ±values,
neighbors of 1, normal/subnormal boundary, and values beyond signed-64 range.
Finite leaves must be actual JSON number tokens; special-value leaves must be
exact OMS strings **"Infinity", "-Infinity", "NaN"**. No oracle or formatter is
modified, and no decoder roundtrip is the sole numerical proof.

`duplicate-output-result.json` checks escaped-equivalent input keys across all
24 cells and independently rejects duplicate decoded **output** keys. The last
input spelling wins, then the encoder emits exactly one field. A schema with
XML-entity-equivalent duplicate member names is rejected atomically by existing
model/name preflight. It is not silently accepted because raw spellings differ.

`output-contract-result.json` separately proves malformed typed UTF-8 and
oversized typed output never publish a result, and subsequent valid output
succeeds, in all 24 cells. C3 visitors are rebuilt/reexecuted with production
children, including nested/zero-allocation/lifetime/alias-tamper controls and
96 required compile-time negative diagnostics. No access-returning workaround
or reintroduced model Element/Value copy is used.

## 7. Reconciliation of all C3 adverse observations

`c3-reconciliation.json` contains **all 79 original adverse records**, their
stack, actual size, category, recovery/abort outcome, original trace hash,
mechanism and corrected source ownership. Grouping is by actual stack:

| C3 mechanism | Original observations | Corrected final validation |
|---|---:|---|
| JSON_Array controlled result/aggregate copy | 36 Program_Error | No DOM/occurrence array construction or Get(array) |
| Provider unescape XString transition | 36 Program_Error | No Decode_As_String/Un_Escape_String |
| Qualified-key To_XString transition | 6 Program_Error + 1 SIGABRT | No occurrence-tree Set_Field/key materialization |

The old 911 resource successes remain old observations, not new qualification.
Targets removed from corrected execution are **not** counted as reached
injection passes. Their absence is proved by armed-execution counters/stacks;
replacement operations receive their own reached real-allocation failures.

`new-audit-result.json` contains all **24 corrected encode cells**, **35,340**
baseline allocation stacks and **886 representative call-chain group injections**.
Every injected result is Storage_Error with unchanged model/output sentinel and
same-process recovery. Armed counters are **zero DOM occurrence reads, zero
provider array adjustments, zero unescape calls**, with positive streaming SAX
event counts. Every actual final-validation entry is independently injected and
attributed to `Buffer.Open_String`, not to a guessed old ordinal.

No Program_Error/SIGABRT remains in these active corrected encode witnesses.
The old-source SIGABRT is still observed and preserved. This closes the focused
output-validation boundary, not a universal allocator proof over all supported
inputs or the whole provider/decoder.

## 8. Preservation and final local gates

Fresh exact-main and exact-C3 generator builds each pass **105** comparisons:
**86 byte-identical generations**, **19 matching atomic rejections**. Total 210
comparisons preserve Ada models/APIs, portable/qualified-Float32/Binary profiles,
Rust and C++. **16** additional actual C3 lossless generated source units across
both namespaces are byte-identical, including parent models, codec interfaces,
codec bodies and Codec_Views. The change is in separately supplied lossless
support, not generator source or generated source rewriting.

Ordinary Guard.Read, Guard.Write and Validate_String code remain unchanged. The
existing **199-case** guard Unicode corpus and support-boundary probes pass both
compilers/all policies. The guard's new public procedure is intentional additive
support API; legacy applications are not globally substituted to it.

Completed local gates on frozen sources:

* formatting; workspace/all-target check; warnings-denied workspace/all-target
  Clippy;
* **785 component tests**, **26 CLI codec tests**, **37 CLI library tests**;
* both 339-case differential matrices, Unicode/malformed/limits and allocation
  recovery traces;
* 24 generated composition, 24 numerical, 24 output-atomicity cells;
* C3 view regressions and negative controls; legacy guard corpus;
* CI split guard, **210 split mutation checks**, layout guard and its mutation
  tests, with actual workflow path supplied;
* original array-copy/C1/provider reds separately from corrected results.

The full 27,167/20,474 numerical corpus, real-UCI campaign, all-ordinal resource
sweep, leak campaign and hosted CI were not rerun or claimed complete.

### Attempt/source integrity

All long work uses the established detached supervisor, unique attempt roots,
flushed progress, isolated writable providers, and atomic result records.
`source-final01-manifest.json` freezes consumed production sources. One later
**test-only** correction adds `with Allocation_Faults` to `c4_numbers.adb` so its
standalone linked test elaborates the allocator wrapper. The corrected test unit
is frozen under `numeric-test-source02/` and consumed by the actual allocation
builds. No production validation/emitter source changed after freeze.

Failed setup attempts remain distinct: an initial numeric main omitted the
allocator-wrapper elaboration; old-trace size assumptions were replaced by
actual stacks; an early old-source worker started before all executables existed;
a key-target attempt selected a different provider allocation and is not counted
as the exact key-transition proof. The final scoped target includes Set_Field
entry/exit and captures the actual To_XString chain.

The host's shared `/tmp` exhausted temporary resources during several builds.
These commands remain failed attempts; affected old-source/numeric/legacy builds
were retried using task-owned TMPDIR with final successful results. No unrelated
temporary directory or worktree was cleaned. The layout-checker initially lacked
its required workflow argument; a wrong-path retry also failed. The final
`.github/workflows/deep-ci.yml` invocation passes on the same frozen sources,
with original attempts preserved in `local-checks-final.json` accounting.

## 9. Remaining resource mechanisms and next work

### Encoder

Final validation now has only bounded reader storage, scalar parser-state
allocation/growth and fixed bookkeeping; no in-scope output-validation failure
remains in the completed focused proofs. Typed serialization is still fallible:
Unbounded_String construction/copy-on-write growth, provider string escaping,
scalar JSON string creation, numerical formatter/locale resources, result String
construction and separate caller assignment. Tested representatives propagate
Storage_Error, but this checkpoint is not an exhaustive failure/cleanup proof of
every remaining encoder operation. Internal invariant errors remain visible.

C3 model visitor contracts remain: synchronous `in` views, no escaped pointers,
no source mutation through other aliases, no concurrent mutation promise.

### Decoder and ordinary provider APIs

Unchanged blockers include:

* `Guard.Read ("[1]") -> Get(array) -> JSON_ArrayDA -> Vect_Pkg.Adjust` red;
* `Occurrences.Read` owned shadow trees, key unescape/To_XString and array
  construction/assignment copies;
* `From_JSON_Digest -> Model.Create(Octets) -> DigestDA -> Binary_Vectors.Adjust`
  real 8-byte failure becomes Program_Error (C3 minimum traces remain valid);
* bounded decode arrays/slot assignments, Required arrays, Additional Append,
  nested/Choice/optional aggregate returns and final owned result composition;
* unintegrated C1 provider XString transition candidate and other provider cleanup
  paths. The exact C1 28-byte and qualified-key abort reproductions remain red.

Read-only encoder views cannot construct new owned decoder carriers. Follow-on
work needs a separately reviewed ownership/construction design and provider
transition qualification, not exception-text rewriting or a broad incidental
decoder rewrite. Complete final numerical/resource/real-service/hosted gates
remain mandatory before any Float64 publication decision.

## 10. Final evidence records

`final-source-manifest.json`, `final-evidence-manifest.json`,
`manifest-hashes.json`, `completed-exits.json`, `inventory-verification.json`,
`decision.json` and post-commit `git-closure.json` under C4 preserve deterministic
SHA-256 inventories and completed exits. Inventories distinguish immutable C3
evidence, consumed C4 snapshots, corrected test inputs, provider/compiler/runtime,
generated units, original adverse traces and corrected-source traces. Failed and
interrupted attempts are included, not relabeled as completed successes.

Their final hashes are supplied in the handoff; inventory self-files and
post-commit closure are excluded from recursive self-hashing. A final read-back
verifies every inventory entry. The normal local commit retains C1/C2/C3 history
and leaves the feature branch clean and unpushed.

**Final disposition: OUTPUT_VALIDATION_BOUNDARY_CLOSED_LOCAL.** This does not
authorize publication or complete-feature READY status.
