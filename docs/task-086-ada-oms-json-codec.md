# Task 086 — JSON dependency evidence stop

**Status: NOT_READY — JSON_LIBRARY_GATE_NOT_ESTABLISHED.**

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
