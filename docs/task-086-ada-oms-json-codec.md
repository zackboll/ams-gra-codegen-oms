# Task 086 — JSON dependency evidence stop

**Current research status (Task 086A): JSON_LIBRARY_GATE_ESTABLISHED for the
guarded Unicode boundary and signed-integer subset.** Production codec support
remains NOT_READY and unchanged. The original Task 086 stop evidence below is
preserved; the follow-up qualification at the end supersedes only that
dependency stop, not any unfinished generator gate.

This is dependency research, not generated Ada codec support or complete CAL
conformance. No production generator, readiness, model API, runtime, CLI, or CI
configuration was modified. Ada retains its existing codec rejection.

## Baseline and isolation

Freshly fetched `origin/main` and immutable starting commit:
`b852b054e96202ca7b0ac83591b70338d98ed332`.
Branch: `feature/086-ada-oms-json-codec`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task086`.
Scratch/build evidence: `/tmp/task086`.
No existing worktree was removed or modified. No open PR or existing Task 086
branch/PR was found at initial inspection. Fast CI currently installs system
GNAT; it does not provision GNATCOLL. No CI provisioning changes were made.

## Independently compiled first candidate

Alire 2.1.1 resolved `gnatcoll=26.0.0` and `gnatcoll_minimal=26.0.0`.
The archive is `https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v26.0.0.zip`,
with Alire index SHA-512:
`cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b`.
License: `GPL-3.0-or-later WITH GCC-exception-3.1`. This was evaluated, not adopted
as a production dependency. A separate consumer can `with "gnatcoll_core"`;
the model need not acquire a JSON dependency.

The standalone probe compiled and executed with GNAT 16.1.0, then with system
GNAT 14.2.0 after adding an explicit `gnat_external=14.2.0` dependency and
forcing a complete rebuild. GPRBUILD reports 26.0.0 (2026-04-15).
Local toolchain selection alone initially retained GNAT 16 in the solution;
that run is **not** treated as GNAT 14 evidence.

The preserved probe is in `tests/ada-json-probe`. It uses explicit checks,
not assertion-policy-dependent assertions. It intentionally prints parser
observations for adverse cases rather than presenting them as passing tests.
Reproduce outside the repository by copying that directory into task-owned
scratch, then running `alr -n build` and `alr -n run`. The consumer pins both
GNATCOLL crates and the external compiler; GPRBUILD must be available through
Alire. Build products must not be committed.

### Observed results (both compilers)

* Construct/serialize/parse an object with string, boolean and integer: passed.
* Quotes, backslashes and UTF-8 contents: exact round trip.
* `\u00e9\uD83D\uDE00`: decoded to exact UTF-8 for é and U+1F600.
* Missing member, JSON null and authored empty string: observably distinct.
* Arrays, objects, booleans, strings and floating numbers: supported.
* Signed 64-bit minimum/maximum and `9007199254740993`: preserved exactly.
* `9223372036854775808` and `18446744073709551615`: rejected as outside
  `Long_Long_Integer`; not silently converted to floating point.
* `1.234567890123456789012345`: serialized as `1.23456789012346E+00`.
  This is not evidence for Decimal precision. Full unsigned-64 readiness
  would be unsafe with this DOM representation.
* Duplicate `{"k":1,"k":2}`: accepted and collapsed to `{"k":2}`.
  Duplicate rejection cannot be claimed using the DOM reader alone.
* Objects serialize in sorted key order in the observed probe. The provider
  API documents object equality as independent of field order; further
  deterministic encoding qualification was not completed.
* Truncated object/string, missing value, trailing junk, `\q` and `\uZZZZ`:
  rejected through `Read_Result.Success = False`.
* **Lone `\uD800` and `\uDC00`: accepted.**
* **`\uD800\u0041`: accepted, then serialized as `\uD800\uDC41`.**
  The second escape is not a low surrogate. This is a concrete invalid-input
  and content-preservation blocker for selecting the unmodified provider.

Source inspection corroborates the final result: `gnatcoll-json-utility.adb`
combines a high surrogate followed by another Unicode escape without requiring
the second code unit to lie in the low-surrogate range.

Scratch logs: `probe-build.log`, `probe-run.log`,
`probe-build-gnat14-real.log`, `probe-run-gnat14-real.log`, `gnat14-version.log`.
SHA-256 of the GNAT 16 run log:
`ca75b6231e583ab6d0ba79003e235a69dfd51474d00feaef15e57721a1924ce3`.
SHA-256 of the verified GNAT 14 run log:
`ebb142437a76faa4e1ddc325cdf4172e9e77ab7c33579dc9caaa38b155d6012a`.

## Second candidate inspection

Alire `json=6.0.0`, Apache-2.0, pins upstream commit
`511489d4cf3bba05ca1376b36664ebe619004221` of
`https://github.com/onox/json-ada.git`.
The pinned source was cloned into `/tmp/task086/json-ada-source`.
`json/src/json-tokenizers.adb` explicitly handles `when 'u'` by raising
`Program_Error` with `Escaped unicode not supported yet`.
It cannot satisfy the mandatory Unicode escape requirement. This candidate
was source-inspected, **not compiler-qualified**.

These two candidates do not establish a viable dependency. This is not an
exhaustive assertion that every other Ada JSON provider is unsuitable. No
general-purpose parser or ad-hoc replacement was written.

## Work deliberately not performed after the stop

The proposed API remains a design only: separate `Service_Codec` spec/body,
`Codec_Error`, and one typed `Encode_<Payload>` / `Decode_<Payload>` pair per
unique payload. There are **no newly supported Ada codec features**.
All requested Choice, polymorphism, repetition, temporal, floating, Decimal,
binary, nillable, unqualified-member and other unsupported cases remain
unimplemented; even the proposed scalar/record subset is not promoted.

OMS mapping authority remains OMSC-SPC-013 Rev B §6.1 and the existing Task
050/051 evidence: payload body only, actual element wire QNames, bare OAM
members and Clark names otherwise, exact enum wire values, no null as absence.
No optional wrappers or checked constructors were bypassed or changed.

Both cached real-UCI root hashes were independently checked and matched the
requested 2.5/2.6 pins. SecurityMarkings verification and normalized RDMA
closure inventory were **not completed**. No real-UCI codec success or new
first unsupported declaration is claimed. Existing reports are not replaced
by an XSD-only approximation.

The baseline production CLI built successfully with Rust 1.95.0 and the
committed Cargo.lock using `/tmp/task086/target`. Full source review, generated
codec execution corpus, byte-preservation matrix, full workspace qualification,
RDMA readiness research, hosted CI, push, PR and SHA parity gates were **not
completed**. No production-output preservation claim is inferred from this
probe. No implementation PR is opened at this dependency stop.

## Next work

Establish a versioned provider (or independently qualified provider API usage)
that rejects malformed Unicode without changing string contents. A narrow
provider-based validation layer would require its own executable evidence;
it has not been implemented here. Then complete the normalized RDMA inventory
and detailed source review **before** production changes, implement the shared
Ada classifier/renderer, and run every requested compiler and compatibility
gate. Task 087 has not begun.

## Task 086A — independently qualified guarded provider boundary

### Scope, pins, and upstream inspection

The original local evidence commit `09c6a3d48914db737386f886c0ceb27c312bd33b`
is preserved without amendment. All new work is confined to the test probe and
this document. Scratch, source checkout, compiler solutions, and logs reside
under `/tmp/task086a`. No production code, CI, Cargo dependency, generated model,
UCI cache, or sibling worktree was changed.

The exact upstream release tag `v26.0.0` resolves through annotated tag
`bd164704e64e4a64411cae31a16374bb9d6a6d77` to source commit
`8e270f83719d4284782a965edd80246be15b949b`. Its JSON implementation and utility
body were byte-compared with the previously downloaded Alire source. The
archive hash and GPL/GCC-exception license recorded above remain unchanged.
No upstream patch or alternate parser is used.

Inspection covered `gnatcoll-json.ads/.adb` and
`gnatcoll-json-utility.ads/.adb`:

* `Parse_Next` owns object/array/number grammar. String scanning checks four
  hexadecimal digits after lowercase `u`, but not UTF-16 pairing or raw UTF-8.
* `Decode_As_String` calls the private utility `Un_Escape_String`. Both DOM
  values and object keys use the same utility during `Read`.
* `Un_Escape_String` emits lone surrogate code units through GNAT's encoder.
  Its high-surrogate branch tests for another Unicode escape, but masks its
  low ten bits without requiring a low-surrogate code unit. Thus U+0041 is
  incorrectly treated as a trail surrogate.
* `Read (String) return Read_Result` delegates through `Buffer.Open_String`.
  Integer token conversion uses `Long_Long_Integer'Value` and converts range
  failure into a parsing error; it does not fall back to floating point.
  The JSON_Value-returning overload prints and raises on parsing failure;
  the guard deliberately uses the nonprinting result overload.
* `Write` recursively serializes the tree; both string values and keys call
  `Escape_String`. This utility catches UTF-8 decoding `Constraint_Error`,
  skips an invalid byte and substitutes NUL, then emits `\u0000`.
  Output validation alone therefore cannot establish losslessness.

### Compiler-backed unguarded defect evidence

Before introducing the guard, the executable reproduced both defects with
explicit checks and hexadecimal byte dumps (`defects14-run.log`). The complete
probe repeats those checks in every subsequent compiler/policy run:

| Stage | Exact bytes |
| --- | --- |
| malformed pair input `"\uD800\u0041"` | `22 5C 75 44 38 30 30 5C 75 30 30 34 31 22` |
| provider decoded value | `F0 90 81 81` (U+10041) |
| provider reencoded text `"\uD800\uDC41"` | `22 5C 75 44 38 30 30 5C 75 44 43 34 31 22` |
| invalid writer input | `FF` |
| provider substituted output `"\u0000"` | `22 5C 75 30 30 30 30 22` |

### Public prototype and invariants

`JSON_Guard` exposes only:

```ada
Validation_Error : exception;
function Read (Text : String) return GNATCOLL.JSON.JSON_Value;
function Write (Value : GNATCOLL.JSON.JSON_Value) return String;
```

There is one serialization entry point, and no unchecked writer exported by
this package. `Write` accepts even an unchecked provider tree but validates it
before invoking GNATCOLL. A caller deliberately using GNATCOLL directly is
outside this API; Ada cannot revoke that upstream public API. Intended codec
integration must call this boundary exclusively.

The incoming scanner is iterative, read-only, with constant bounded lookahead
per byte and no object/array grammar or normalization. It tracks quoted-string
state and consumes escaped quotes/backslashes as pairs. Every Unicode escape
has four hexadecimal digits; a high surrogate requires an immediately adjacent
actual `\u` low-surrogate escape. An escaped literal backslash cannot satisfy
that requirement. UTF-8 lead and second-byte bounds implement RFC 3629 scalar
encoding: no overlong forms, surrogates, values above U+10FFFF, invalid leads,
or invalid/truncated continuations. Unescaped string controls fail. Decoded
string controls, including NUL, are valid and preserved.

GNATCOLL alone then parses structural grammar. Parser failures are translated
to `Validation_Error`, not defaults. The guard does not rely on assertions.
Tree validation uses only `Map_JSON_Object`, `Get`, `Length`, and array access;
it validates every key/string recursively before writing and validates output
lexical encoding afterwards. Traversal rejects depth greater than 128 or more
than 100,000 visited nodes, providing a finite rejection for cyclic/excessive
trees. These are prototype resource limits, not XSD or CAL rules. They do not
bound provider allocations before DOM construction or total document bytes;
production ingress resource policy remains separate. Concurrent mutation of
the supplied tree is not supported.

The package can be copied into a future codec consumer without introducing
GNATCOLL into model types. This is an integration design, not authorized
production integration or generated codec qualification.

### Independently authored corpus and compiler results

All expected decoded UTF-8 bytes are literal hexadecimal sequences, not
computed by a second library decoder. The probe compares them directly, then
checks guarded write/read round trips. Every positive also runs as both an
object key and value, before and after serialization.

**16 positive vectors:** ASCII, empty, embedded quote/backslash, all short
escapes, raw multibyte UTF-8, escaped BMP, supplementary surrogate pair, NUL,
U+D7FF, U+E000, U+10000, U+10FFFF, combined raw boundary encodings, combining
character, U+2028/U+2029, and escaped literal Unicode-looking text.

**Adverse corpus:** lone high/low surrogate; high plus non-low/high surrogate;
high plus escaped literal backslash; truncated/nonhex/unknown/uppercase escape;
14 invalid UTF-8 byte vectors (continuation, FF, wrong second/third/fourth byte,
overlong 2/3/4-byte forms, raw surrogate, above maximum, 5-byte lead, truncated
2/3/4-byte forms). These run in keys and values; every invalid UTF-8 vector also
runs through the writer as scalar, key, and nested object/array value.
Every unescaped control U+0000..U+001F is rejected in keys and values.
Additional cases cover truncation, invalid bytes outside strings, malformed
grammar, trailing documents, empty document, all 13 incomplete prefixes of a
surrogate-pair document, non-one-based input, and writer depth/node limits.
Expected rejection handlers catch **only** `Validation_Error`; unexpected
bounds failures or other exceptions fail the executable.

Alire **2.1.1**, GNATCOLL and GNATCOLL minimal **26.0.0**, GPRBUILD **26.0.0**
(2026-04-15), GNAT **14.2.0** and **16.1.0** were used. GPR flags remain
`-gnat2022 -gnato -O0`; no runtime range/index checks were disabled.
Each compiler completed the original provider probe plus the **199-case**
guarded corpus under normal, explicit Check, and explicit Ignore assertion
policies. All six runs passed. Separate-policy executable output is identical
across both compilers and both policies; its SHA-256 is
`ba61ad1bf1cd12253655cb6495104874f5a61bf6f7f4b06fe8a84f154cfcf64e`.
Logs include all expected byte dumps and controlled rejection results.

Reproduction (copy the probe into new task-owned directories first):

```sh
# System compiler copy uses committed gnat_external=14.2.0.
alr -n build -- -f
alr -n run
# In a separate compiler-16 copy only:
alr -n with --del gnat_external
alr -n with gnat_native=16.1.0
alr -n build -- -f
alr -n run
# For each compiler, repeat using an absolute task-owned .adc file containing
# pragma Assertion_Policy (Check); or pragma Assertion_Policy (Ignore);
alr -n build -- -f -cargs:Ada -gnatec=/absolute/task-owned/policy.adc
./bin/json_probe
```

The Alire solutions and compiler-version logs in `/tmp/task086a/guard14` and
`guard16` verify the actual selected compilers, rather than inferring them
from the local toolchain preference. The committed manifest pins GNAT 14;
the GNAT 16 copy changes only its scratch compiler dependency.

### Numbers and duplicate-key limitation

Signed minimum, maximum, `9007199254740993`, zero and integer negative zero
round-trip exactly. Integer `-0` becomes `0`: the integer value contract does
not preserve a negative-zero spelling. Both adjacent signed overflows and
`18446744073709551615` fail with `Validation_Error`. Initial codec domains must
be contained in **[-9223372036854775808, 9223372036854775807]**; an unsigned domain
must additionally have nonnegative values and maximum no greater than signed
64-bit maximum. Model-domain checks are still required in the future renderer.
Fraction/exponent tokens use floating storage and are not evidence of integer
or Decimal exactness. No float/Decimal codec capability is qualified here.

Duplicate keys remain **last-value-wins**, including literal/escaped equivalent
names. The executable SAX investigation shows `Parse_Next` exposes both key
events in `{"a":1,"\u0061":2}`, and `Decode_As_String` yields byte-identical
`a` for each. The guarded DOM retains `a = 2`. This indicates a potential
provider-driven duplicate check without re-parsing JSON grammar, but a robust
implementation needs per-object decoded-key sets, nested scope tracking,
key-versus-string-value state, resource limits and further tests. That is a
**separate unresolved strict-ingress-policy gate**, not implemented here.
No strict duplicate rejection is claimed or authorized by this qualification.

### Decision and review boundary

**JSON_LIBRARY_GATE_ESTABLISHED** for this guarded Unicode/signed-integer
boundary with the explicit duplicate-key limitation above. Both alteration
defects are reproduced; guarded ingress rejects them before provider decoding,
guarded output rejects invalid bytes before substitution, valid Unicode bytes
round-trip exactly, and bad inputs fail controllably on both compilers.

This is research-only. Ada service codec readiness, generation, model output,
Rust codecs and CAL/runtime conformance remain unchanged. No generated Ada
codec implementation begins before independent review of this boundary.
Configured hosted CI does not execute this standalone Alire probe; the explicit
local compiler evidence is distinct from hosted regression status. No CI
workflow is modified or manually dispatched in this qualification.
