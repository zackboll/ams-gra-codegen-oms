# CPP-OMS-007R1 acceptance follow-up

Status: **READY_FOR_INDEPENDENT_REVIEW** for the Sleet correction;
**BLOCKED_EXTERNAL_DEPENDENCY** for feature acceptance. The production Sleet
pin remains `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`. No merge, approval,
auto-merge, force-push or history rewrite is authorized or performed.

## Source identities and review dependencies

| Source | Commit | Tree |
|---|---|---|
| Original Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Sleet corrective candidate | `dff18c05d2d318b60b6a87e889f0daf15864f0e3` | `794069ffa2c1d6a5ea73166fd0b20f06f652c6ca` |
| Recorded OMS review head | `e5c4320e004620df66d16f514a28c16052a35402` | `52232193457e2bb026dd11557ec1156e7f1c96e9` |
| Main reconciled by ordinary merge | `aadf622` | recorded by Git |

* Sleet correction: https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet/pull/2
* Separate test-observation correction: https://github.com/zackboll/ams-gra-codegen-oms/pull/200
* Existing feature PR (not replaced): https://github.com/zackboll/ams-gra-codegen-oms/pull/198

The original clean broker rejection was reproduced anew for both releases:
`FileMetadata.MessageData.SHA_2_Hash`, `length 64 != 32`, exit 1,
5/23 failed. The full earlier original-pin six-configuration matrix also ended
with 12 failing hash-present executions and zero unavailable configurations.
These are retained negative results, not expected passes.

## Sleet correction

Primitive resolution already supplies `xs:hexBinary`. The facet evaluator
ignored it and counted lexical characters. It now checks an even number of
ASCII hexadecimal digits, permits only XML whitespace at the edges, and applies
length/minLength/maxLength to octets. Invalid types fail closed. Enumeration,
pattern and length facet precedence remains in place; strings still count
Unicode characters and base64 behavior is unchanged. Unrestricted hexBinary
also receives lexical validation rather than bypassing it via empty facets.
There are no new dependencies or FileMetadata-name special cases.

Candidate Sleet tests: **276 default workspace tests passed**, two authentic
schema tests explicitly selected and passed separately; **0 failures**.
`cargo fmt --all -- --check` and workspace/all-target Clippy with denied
warnings passed. Tests cover direct facets, nested production-validator
restrictions, 31/32/33-octet hashes, min/max boundaries, empty permitted values,
upper/lower case, zero/maximum bytes, invalid digits/odd counts/internal and
non-XML whitespace, invalid JSON types, exact paths/facet identification,
and unchanged string semantics. Authentic full schemas are separately supplied
and authenticated, not reduced fixtures.

## CPP-OMS-004 investigation

The original base `a6a0f575` and candidate have identical runtime, PositionReport
test, and thread helper sources. Both are tested with clang 19.1.7, Boost 1.83,
`-std=c++17 -g -fsanitize=thread -O1 -pthread`, original pinned Sleet, and full
UCI 2.6. Repeated isolated executions retain stdout, stderr, sanitizer output,
exit statuses and thread snapshots. The unmodified head reproduced the immediate
post-close assertion (1/247 failed); baseline comparisons are tracked below.

`Runtime::close()` joins the I/O and dispatch workers and destroys the Asio
context, including its resolver. A broker-free joined-thread probe proves a
Linux observation race: after `join()` and an independently observed completion
flag, `/proc/self/task` may still contain the exiting named task. In 20,000
iterations this occurred 14 times with clang TSan and 7 without instrumentation.
A stat-capturing repeat recorded `PF_EXITING`, including tasks still displayed
as `R`, and tasks disappearing between comm/stat reads. Linux v6.12
`exit_signals()` sets PF_EXITING before `exit_mm()`/`mm_release()` wakes the
joiner; `/proc` unlink occurs later in exit processing.

PR #200 changes test-only enumeration to count userspace-live workers, exclude
only PF_EXITING/proven-vanished records, and fail closed on other read/parse
errors. It preserves the immediate zero-worker assertion and production runtime.
No timeout increase is used. All six compiler/sanitizer configurations pass its
3 tests/6007 checks, including a blocked live-worker negative control and 2000
joined workers. Independent review is required before incorporating that
separate correction into the feature acceptance baseline.

## C++ boundary and transport coverage

The C++ feature implementation is unchanged. The synthetic fixture adds
minimum-only (2 octets), maximum-only (4), combined (2..4), and exact-zero
profiles. A 98-check executable tests every length 0..5, carrier/codec agreement,
independent bytes, exact uppercase encoding, and rejection paths. The existing
272-check/67-rejection consumer and five executable mutation controls remain.
The mutation control is scoped to Digest, since multiple range carriers now
legitimately share the same generated validation statement.

Authentic transport now also sends five malformed hashes through an independent
raw OWP publisher, requires broker rejection at the correct hash path, proves
no typed callback or decode event, and verifies valid same-subscription recovery.
Each hash-present/absent execution requires **75 checks**, including two
different valid hashes where applicable, exact delivered bytes, retained typed
values, UNSUB acknowledgment, no post-UNSUB callbacks and close cleanup.

Candidate-only qualification is explicit, source-verified and separately built:

```sh
export CPP_OMS_007_SLEET_CANDIDATE_REV=dff18c05d2d318b60b6a87e889f0daf15864f0e3
export CPP_OMS_007_SLEET_CANDIDATE_TREE=794069ffa2c1d6a5ea73166fd0b20f06f652c6ca
# Also set AMS_GRA_UCI_2_5_ROOT / AMS_GRA_UCI_2_6_ROOT to authenticated roots.
bash scripts/check-cpp-oms-007-file-metadata.sh SCRATCH CORRECTED_SLEET_CHECKOUT
```

Without these variables the script still requires the original pin and fails
on its bug. Candidate runs print exact source identities and binary SHA-256,
verify a clean descendant checkout and all four schema hashes, build locked
source in their own target directory, and label results conditional.

## Live qualification ledger

Evidence directory: `/home/zboll/cpp-oms-007r1-evidence`. A missing `.exit` file
is not a completed pass. The full driver and isolated clang split use identical
flags and tests; the split changes only compiler selection to run independently.

| Check | Result at initial R1 commit |
|---|---|
| Codec suite | 27 passed, including new boundaries under g++/clang++ strict and ASan+UBSan |
| Router harness after main reconciliation | 39 passed |
| Baseline no-codec / Ada+Rust codec parity | 59 pairs / 472 comparisons / 0 mismatches |
| CPP-OMS-001 standalone | PASS, 6313 checks per execution |
| CPP-OMS-002 standalone | PASS both releases, 307 checks each configuration |
| CPP-OMS-003 through 006 full reruns | Running; not claimed complete |
| Workspace library tests and C++ smoke | Running; not claimed complete |
| Corrected Sleet g++ strict | Both releases: codec 42 checks; absent/hash-present transport 75 checks each, PASS |
| Remaining corrected-broker compiler matrix | Running; not claimed complete |
| Full-release readiness/parity refresh | Running; require >=401 READY each, zero losses |

Later exact-head receipts update completed evidence without extrapolating hosted
CI from earlier commits. The prior review head's hosted smoke passed. Current
head's hosted status must be inspected separately after push.

## Acceptance decision

Successful candidate-broker execution is **conditional evidence only**. Sleet
PR #2 is not yet independently accepted and the OMS production pin is unchanged.
After independent approval of an exact corrected dependency commit, prepare
the narrow pin correction and rerun mandatory acceptance. Until then, and until
all required regressions and review dependencies are resolved, **not
READY_FOR_MERGE**. An unavailable sanitizer must be reported as
**QUALIFICATION_ENVIRONMENT_BLOCKED**, never passed or silently skipped.