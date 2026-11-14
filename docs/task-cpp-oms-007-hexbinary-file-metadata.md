# CPP-OMS-007R1 follow-up

Current corrective-dependency review and refreshed qualification are recorded in
[the R1 acceptance ledger](task-cpp-oms-007r1-acceptance.md). Historical
original-pin failures below remain valid; candidate-broker success is conditional
and does not change the production pin or authorize merging.

# CPP-OMS-007 — constrained hexBinary codecs and authentic FileMetadata

## Status: codec complete; hash-present pinned-Sleet acceptance BLOCKED

The authentic C++17 FileMetadata codec is READY and behaviorally tested in
both releases. **The full task's acceptance criteria are not met.** The
unmodified pinned Sleet broker rejects a valid 32-octet hash because it applies
`xs:length=32` to the **64 hexadecimal characters**, not the decoded octets.
No schema, broker, wire representation, runtime, dependency pin, or validation
gate is changed to conceal this. Hash-absent transport success is not evidence
of hash-present interoperability.

## Identities and provenance

Freshly fetched `origin/main` base:

* commit `a6a0f575ccdb6a850205d3e93e6fbd21823dfeaa`;
* tree `a31c2e4305464701d01610209eb00766999ad3e2`.

Both prerequisite merge commits are verified ancestors of that base:
PR #178 `9ddcfa9514fbaff9e819525421e014bf7a737aec` and
PR #182 `1af9565f881778d2aad8163524cdd41d3fbe2c21`.
Work is isolated in `/home/zboll/git/ams-gra-codegen-oms-cpp-oms-007` on
`feature/cpp-oms-007-hexbinary-file-metadata`; the unrelated CI worktree is untouched.

Implementation head (including qualification-driver flag correction):

* commit `6eca1ba543afe77f50c6485820ce4881d85eedf2`;
* tree `b73bca0367bae7870181f2fe97789e1584390861`.

The final documentation/delivery commit and tree are recorded in the PR
receipt (a document cannot contain its own Git object identity). Ordinary
commits only; no amend, force push, merge, approval or auto-merge.

Authenticated public UCI source: `https://gitlab.com/open-arsenal/uci/standard.git`,
using the repository's existing release pins and admission SHA-256 values.

| Release | Tag commit | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 |
| --- | --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

Existing local extracted roots were authenticated against those hashes before
inspection. The checked qualification driver repeats all four checks. The
source `SHA_2_256_HashType` declaration is at lines 141247–141254 in 2.5 and
141523–141530 in 2.6: `xs:restriction base="xs:hexBinary"`, `xs:length value="32"`.
`FileMetadataMDT.SHA_2_Hash` is optional in both. The test accounts for the
additional optional `FileSize` in 2.6. No authentic schema or generated UCI
header is committed.

Sleet source: `https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet`,
commit `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`. Its clean checkout is built with
its own Rust toolchain and `cargo build --release --locked -p sleet`.

## Admission and generated behavior

The existing shared `binary_length_domain` classifier already chooses the
checked C++ model carrier. C++ codec admission now accepts **named, length-only
Binary declarations** accepted by that classifier, provided resolved lexical
provenance is `HexBinary`. This includes exact, minimum, maximum and intersected
length domains. It is not a schema-name special case. Unsupported facets,
unknown provenance, base64Binary and constrained direct members remain rejected.
The renderer still runs the production readiness analyzer before writing files;
there is no independent permissive renderer policy.

The checked binary carrier does **not** parse hexadecimal strings. Its public
API is `static std::optional<T> create(std::vector<std::uint8_t>)` and
`const std::vector<std::uint8_t>& value() const`; the storing constructor is
private, no default constructor exists, and copy-only lifecycle preserves a
moved-from value's invariants. The existing codec runtime already implements
hexadecimal parsing and canonicalization and is unchanged:

* JSON string input, case-insensitive ASCII hex digits;
* XML Schema whitespace collapse (leading/trailing XML whitespace accepted,
  surviving internal whitespace rejected);
* malformed digits and odd length rejected by the existing lexical parser;
* parsed octets passed to **`T::create`**, with no codec-side length checks;
* factory refusal reports the actual member path;
* encode reads the validated `value()` octets and uses the existing uppercase
  two-digits-per-octet JSON string representation.

There is no new parser, regex engine, third-party library, runtime subsystem or
model-generator change. Previously admitted binary output takes the original
rendering branch unchanged. C++17 is retained.

## Measured readiness and complete-release impact

Baseline CLI was built from the exact fresh base and saved separately before
editing. Both measurements use production `service-check --language cpp
--world closed-schema --with-codec`, not a compile-only approximation.

| Release | Selected type closure | Before | After |
| --- | ---: | --- | --- |
| 2.5 | 52 | 49/50 NOT READY, SHA_2_256_HashType | **50/50 READY** |
| 2.6 | 54 | 51/52 NOT READY, SHA_2_256_HashType | **52/52 READY** |

The existing pinned sweep evaluates every global element with production
`service-check-batch`, verifies batch completeness/order/exit codes and compares
the exact baseline CLI with the candidate:

| Release | Global elements | Valid contracts/model READY | Codec READY before → after | Newly READY | Lost |
| --- | ---: | ---: | ---: | --- | ---: |
| 2.5 | 722 | 676 | 400 → 401 | FileMetadata only | 0 |
| 2.6 | 725 | 679 | 400 → 401 | FileMetadata only | 0 |

Four messages' renderable-declaration counts improve per release, none fall.
Remaining first codec blockers among model-READY messages:

| Kind | 2.5 | 2.6 |
| --- | ---: | ---: |
| alternating ASCII | 93 | 94 |
| Float32 | 70 | 78 |
| optional/repeated Choice alternative | 69 | 59 |
| named Time | 29 | 32 |
| abstract `$type` | 8 | 9 |
| uninhabited abstract member | 6 | 6 |

These are first-blocker counts, not exhaustive unsupported-type counts.

## Synthetic qualification

The explicitly synthetic `constrained-hex` fixture selects a named 32-octet
carrier in required/optional, nested Record, direct Choice and nested Choice
positions. Handwritten independent wire expectations cover:

* 32 zero octets, 32 `0xAA` octets, 32 maximum `0xFF` octets;
* upper/lowercase and escaped leading/trailing XML whitespace, with exact
  uppercase canonical output;
* empty, 31- and 33-octet strings (verdict agreement with `Digest::create`);
* odd lengths, invalid digits, internal whitespace, `0x` prefix;
* number, Boolean, array, object and null where a string is required;
* root null, nested null, absent optional, present optional and missing required;
* malformed values in all nested/Choice positions;
* duplicate keys at root/nested/Choice levels (parser path `$`);
* semantic member paths and containing-record paths for missing members;
* static construction/accessor constraints and moved-from carrier validity.

Exact consumer summary: **272 checks, 67 rejections**. Independent mutation
controls compile each weakened artifact and require a runtime failure:
carrier length removal, codec factory-input bypass, lowercase output, wrong
error path, odd-length parser bypass: **5/5 detected**. Compilation failure is
not counted as mutation detection. The admission unit test independently gates
length domains, provenance, unsupported facets and direct constrained members.

## Authentic FileMetadata codec and Sleet results

The independent C++ consumer constructs the FileMetadata aggregate through
checked model APIs, uses a checked decoded security seed for the large shared
SecurityInformation aggregate, and compares exact handwritten expected JSON.
It checks every populated field via exact wire comparison, then checks the
filename and hash octets explicitly. Both distinct payloads are exercised with
and without the optional hash. Uppercase output, lowercase input, 0/31/32/33
octet carrier agreement, malformed lexical values, JSON kinds, null, and exact
`FileMetadataMT.MessageData.SHA_2_Hash` error paths are covered.

For **each release**: authentic consumer **2 tests, 42 checks** passed under
strict g++, strict clang++, g++ ASan+UBSan and clang++ ASan+UBSan. The independent
clang run is recorded in `clang-authentic.log`. Additional transport
configurations are reported below as they finish.

Transport is the existing C++ LA-CAL runtime and generated facade/codec, talking
to the real pinned Sleet with that release's full unmodified schema. No mock is
used. Hash-absent test: **1 test, 25 checks per release passed**, with two
distinct payloads, callback-owned retained copies, exact populated fields,
delivery/decode/handler statistics, acknowledged unsubscribe, no callback
after unsubscribe, close and worker-thread cleanup. This passed with strict
g++ and g++ ASan+UBSan for each release.

Hash-present test: **FAILED independently for 2.5 and 2.6**. Each attempts two
distinct valid hashes (all zero and all `FF`), receives broker rejection, times
out waiting for those two callbacks, then successfully routes an absent-hash
recovery payload. **5 of 23 checks fail per release** (two publish failures,
two absent deliveries, delivered count 1 rather than 3). The test remains a
failing acceptance test; the qualification driver aggregates failures across
releases/configurations and exits nonzero, never relabeling them as success.

Exact broker evidence:

```text
-ERR Invalid-Message schema validation failed: FacetViolation {
  path: "FileMetadata.MessageData.SHA_2_Hash",
  facet: "length", detail: "length 64 != 32"
}
```

Pinned `sleet/src/facets.rs` counts `lexical.chars().count()` (line 232) for
length facets. This same upstream boundary is already documented in
`docs/rust-oms-010-real-file-metadata.md`. Sending a 16-octet hash, removing the
facet, disabling broker validation, or changing the pin would not satisfy this
task and is deliberately not done. An independently reviewed Sleet fix and
approved pin update are needed before hash-present acceptance can pass.

## Regression results and limitations of this run

Evidence logs are outside the checkout at `/home/zboll/cpp-oms-007-evidence`.
Toolchains: g++ 14.2.0, clang++ 19.1.7, Rust 1.95.0; the existing Boost 1.83
headers are supplied through `CPLUS_INCLUDE_PATH`. System Boost was absent;
noninteractive sudo installation was unavailable. No dependency was vendored.

| Check | Result |
| --- | --- |
| CLI C++ codec suite, CPP-OMS-001/002/004/005/006 plus 007 | **26 passed**, 0 failed/ignored; strict g++/clang++, ASan+UBSan, oracle and mutations |
| generated model/API/facade/member-remapping regressions | **105 passed** (7 + 17 + 13 + 68), 0 failed/ignored |
| standalone CPP-OMS-001 | PASS: 6313 checks under g++, clang++, g++ ASan+UBSan |
| CPP-OMS-002 authentic Log | PASS both releases, four compiler configurations; 307 checks, 189/192 rejections |
| CPP-OMS-003 offline LA-CAL | PASS all six configurations (both compilers, strict/ASan+UBSan/TSan): 49 tests, 695 checks per configuration |
| CPP-OMS-003 real Log through pinned Sleet | PASS both releases, g++ strict, clang++ strict, g++ ASan+UBSan: 2 tests, 407 checks per release/configuration |
| baseline parity without codec + Ada/Rust codec readiness | **59 pairs, 472 comparisons, 0 mismatches** |
| previously supported synthetic C++ generated output | **32 pairs, 96 files byte-identical**, 0 mismatches |
| Python generation with and without codec | **118 comparisons**, 0 mismatches |
| whole-release pinned readiness | PASS both releases, no losses or decreased counts |
| registered pinned C++ readiness regression | **1 passed**, both authenticated roots, all six CPP-OMS-006 newly READY messages plus full FileMetadata readiness |
| PR smoke router/executor unit tests | **38 passed** |
| production C++ PR smoke | PASS, including offline CPP-OMS-007 consumer |
| Rust PR smoke | PASS |
| Python PR smoke | Initial system Python failed version gate; rerun under required Python 3.11 PASS (9 consumer tests) |
| workspace clippy all targets / fmt / diff whitespace | PASS |
| complete workspace library tests | **650 passed**, 0 failed/ignored, 15 suite summaries; offline retry completed. Initial pinned-root build interrupted after expensive authentic generation and is not counted as a pass |
| CPP-OMS-004 PositionReport deep script | **FAIL**, UCI 2.6 clang TSan: post-close `check::threads_named("ams-lacal") == 0`, 1 of 247 checks failed; other release/configuration executions passed. Gate and runtime unchanged |
| CPP-OMS-005 authentic AMTI_SettingsCommand deep script | PASS both releases in all six compiler/sanitizer configurations; 12 Sleet executions, each 2 tests/206 checks; synthetic 289/148, mutations 13/13, authentic consumer 68/47 (2.5), 70/46 (2.6) |
| CPP-OMS-006 structured ASCII deep script | Stable-file rerun PASS: all six configurations, consumer 323/166, 27-row consumer 828/259, mutations 21/21; both-release model probe 20 checks and six newly READY message compile smokes per release |
| 007 full compiler/sanitizer authentic qualification | In progress; hash-present Sleet failures above already make acceptance FAIL |
| hosted CI at `f0a5a68f2b23ce48aaa191ab393694b5269af77f` | PR smoke **SUCCESS**, run `42535159399`; trusted signal **SUCCESS**. This is not claimed for a later documentation head; final PR receipt records that exact head separately |
| hosted CI at `4ad061d42f3289b84b3bf42e6983635753b0d1e4` | PR smoke **SUCCESS**, run `42535445329`; trusted signal **SUCCESS**. Later documentation-only head status is recorded separately in the final PR receipt |

An earlier local C++ smoke process had loaded the old 264-check summary before
the final consumer gained four rejection cases; it failed the exact marker
gate. A clean final run passed with 272/67. Early acceptance-driver attempts
exposed a clang linker-only `-no-pie` compile flag and the established GCC
Boost.Asio `-Wno-tsan` requirement; ordinary follow-up commits corrected the
driver. Those aborted runs are not full compiler-configuration passes. The
exact-head full driver is rerun separately and still cannot satisfy the
hash-present broker acceptance criterion.

The first CPP-OMS-006 deep process had its script edited while it was running;
it completed the substantive per-release checks but failed later while reading
the changed file (`syntax error near unexpected token ||`). It is **not**
reported as a passing script. The final stable file passes `bash -n` and a
fresh run (`final-006.log`) is tracked separately. This is a harness execution
failure, not evidence of a generated-code or schema failure. An interrupted
redundant CPP-OMS-004 chain is likewise not counted as a pass.

No Ada, Rust or Python production source, workflow YAML, C++ runtime, Sleet or
LA-CAL production behavior is changed. The shared production change is confined
to the C++ admission branch. The existing CPP-OMS-006 regression is advanced
from its historical FileMetadata NOT READY boundary to require full readiness,
generation, and unchanged model/API bytes; its structured-ASCII checks remain.

## CI integration and reproduction

The existing `PR smoke` C++ executor gets one focused **offline** generation,
strict g++ build and exact-summary execution. Router/executor tests check the
new source and fail closed on wrong summaries, missing generated artifacts,
compile failures and execution failures. No hosted runner, matrix, workflow
repair, documentation campaign, UCI fetch or Sleet process is added to PR smoke.
The expensive acceptance driver remains local/deep and is deliberately red
when authentic hash-present transport cannot work.

```sh
cargo test --locked -p ams-gra-codegen-oms --test cpp_codec
cargo test --locked -p ams-gra-oms-codegen-core cpp_oms_007
python3 -m unittest discover -s scripts -p test_pr_smoke.py
python3 scripts/pr_smoke.py run cpp

# Set both authenticated roots, and supply the existing pinned clean Sleet checkout:
bash scripts/check-cpp-oms-007-file-metadata.sh SCRATCH SLEET_CHECKOUT
# Expected to exit nonzero on the pinned broker's hash length bug.

python3 -I scripts/check-cpp-oms-006-codec-sweep.py --cli CANDIDATE_CLI \
  --baseline BASELINE_CLI --uci-2-5 ROOT25 --uci-2-6 ROOT26 \
  --scratch SCRATCH --workers 2 --require-no-loss
```

**Do not merge on the strength of codec readiness or synthetic success.**
Independent review is required, and full task completion remains blocked on
valid hash-present real-Sleet delivery.