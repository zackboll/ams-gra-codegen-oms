# PY-006 — Python UCI header profiles and real-UCI qualification

## Source and authority

Started from fetched `origin/main`
`a03684eb963e1eaa8a9df3db17c9d1783b7add03`, tree
`af76aa248a8dcef9cec23dc3b83ee5ce4956c4e1`. Both the original checkout
and isolated worktree were clean. PRs #98, #99, #100, #101 and #103 were
verified MERGED into main before creating
`feature/py-006-uci-header-profiles`. No other task's branch, worktree or
evidence was changed. Historical PY-001–005 results remain historical.

Reviewed PY-001–005, the Task 037/038 normalized contracts in
`codegen-core/src/string_profile.rs` and backend compatibility documentation,
the original pinned XSD declarations, and both frozen String corpora.
The original OMS authority was fetched and independently hash-verified:

* OMS standard revision `726272bd0390982a759c91a9cf4e13b81c2b510b`.
* `docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`.
* SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
* Read `word/document.xml`: §6.1.4 maps the exact OAM
  `UniversallyUniqueIdentifierType` identity to a String conforming to RFC 4122
  §3; other Strings use ordinary String mapping. §6.1.2 supplies member QNames
  and repetition rules. The pinned XSD, not Python's `uuid.UUID`, defines the
  checked carrier's narrower lexical space, including nil and case preservation.

Local dependencies: Rust/Cargo 1.95.0, Python **3.11.17** and **3.13.16**
(distinct executables), websockets **17.2**, GNAT/GCC/G++ **14.2.0**.
`Cargo.lock` SHA-256:
`45430d24281bdd7d22f41ea4d93d26fb0bd7bfe8746c223ccf9e56f5240a7f58`.
No dependency or lock-file changes.

## Exact admitted facets

Admission is name-free and delegated to the existing shared `string_profile`
classifier. Only named `Primitive(String)` declarations gain these carriers.
The original direct field-local constrained-String boundary is unchanged.

| Profile | Length facets | One XML-Schema pattern expression in one group |
| --- | --- | --- |
| `UciSchemaVersion` | minLength 7, maxLength 57; no length | `[0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?` |
| `UniversallyUniqueIdentifier` | length 36; no min/maxLength | `(0{8}(-0{4}){3}-0{12})\|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})` |

Neither admits explicit whiteSpace or numeric facets. Changed bounds,
expressions, multiple alternatives/groups and near-matches remain unsupported.
The table's escaped `\|` denotes the UUID expression's **internal alternation**,
not two IR alternatives. There is no regex translator or general XSD engine.
Schema-version validation uses fixed ASCII groups and bounded suffixes; UUID
uses fixed positions plus the explicit nil branch.

Accepted examples (stored unchanged):

* `000.0.0`, `002.5.0`, `002.5.0ab_Release-2A`.
* `999.99.99zz_` followed by exactly 45 ASCII letters (57 characters total).
* `00000000-0000-0000-0000-000000000000` (nil).
* `123E4567-e89B-42d3-A456-426614174000` (mixed case).

Rejected examples:

* `000.001.000.000`, `002.5`, `002.5.0B`, empty/46-character underscore tails.
* Non-nil version 0, versions 6–f/F, variants outside `89abAB`, nil-lookalikes.
* Braces, URNs, missing/moved hyphens, wrong widths, non-hex characters.
* Whitespace, controls, Unicode lookalikes and lone surrogates; no trimming,
  replacement, coercion, case conversion or normalization.

## Generated Python API and codec

The existing API conventions remain: `model.py`, `service_api.py`, optional
`service_codec.py`; frozen slotted dataclasses with `value: str`, declared
`__schema_name__: ClassVar[tuple[str, str]]`, typed constructors and explicit
`__post_init__` validation. Exact `str` input is required (`str` subclasses,
bytes, bools and numbers fail with `TypeError`); bad lexical values raise
`ValueError`. Validation is independent of assertions and remains active under
`-O`. Required/optional/nested fields and bounded immutable tuples retain the
existing type/cardinality rules. Deliberate reflection is not a model security
sandbox; codecs revalidate reflected/corrupted outbound values.

Both profiles encode as JSON Strings without changing spelling. Decode checks
exact String shape and calls the actual generated checked constructor. Encode
checks the exact carrier, presence of its value slot, and the same constructor.
Failures become `CodecError` with the declared schema type and member/index path,
for example `$.UUIDs[1]`. The OAM name-specific exception is removed **only** for
the exact checked UUID profile. An unconstrained OAM type of that name is still
rejected. A foreign namespace does not acquire that special mapping by name;
its similarly named unconstrained String remains an ordinary String.

The existing codec and LA-CAL runtime templates are unchanged. Strict duplicate
handling (including escaped duplicate keys), integer lexemes, Unicode, unknown
members, `$type`, schema-derived QNames and member ordering remain in force.
Helpers are emitted only for schemas using the new profiles, preserving old
generated bytes and standard-library-only, transport-independent imports.

## Pinned real-UCI BEFORE/AFTER

The existing external pinned roots were checked against both repository
revisions and root hashes. Every requested global element is present in both
releases. Task-owned single-message contracts select the actual global element,
not a substituted synthetic payload or type-only approximation.

| Release | Revision | Root SHA-256 |
| --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

All message QNames below use namespace
`https://www.vdl.afrl.af.mil/programs/oam`. Each row was measured in both
`closed-schema` and `open-extensions`, without and with `--with-codec`:
**24 BEFORE and 24 AFTER measurements**. Reports are byte-identical across the
change, including their process exit and first rejection.

| Release | Selected actual message | Closure | Model before → after | API before → after | Codec with flag | Aggregate before → after |
| --- | --- | ---: | --- | --- | --- | --- |
| 2.5 | PositionReport | 60 | NOT READY → NOT READY | NOT READY → NOT READY | not measured → not measured | NOT READY → NOT READY |
| 2.5 | RDMA_InitializeSetup | 50 | NOT READY → NOT READY | NOT READY → NOT READY | not measured → not measured | NOT READY → NOT READY |
| 2.5 | SubsystemStream | 40 | NOT READY → NOT READY | NOT READY → NOT READY | not measured → not measured | NOT READY → NOT READY |
| 2.6 | PositionReport | 60 | NOT READY → NOT READY | NOT READY → NOT READY | not measured → not measured | NOT READY → NOT READY |
| 2.6 | RDMA_InitializeSetup | 50 | NOT READY → NOT READY | NOT READY → NOT READY | not measured → not measured | NOT READY → NOT READY |
| 2.6 | SubsystemStream | 41 | NOT READY → NOT READY | NOT READY → NOT READY | not measured → not measured | NOT READY → NOT READY |

Exact first blocker in every row/world:
`unsupported Python IR construct: abstract declarations are unsupported`.
The CLI executed and returned **1**, not a loading failure or a READY result.
Without the flag, codec readiness is not requested. With it, model/API failure
means codec is **not measured**, not a fabricated codec NOT READY verdict.
No artifacts are generated, so no real-service artifact hashes or runtime
qualification exist. **No selected real-UCI service is fully READY.**

BEFORE reports were preserved before production changes in the dedicated
fixture's `before.json`, including source identity, exact reports, commands and
exit codes (machine-local path prefixes are represented symbolically). The raw
BEFORE evidence SHA-256 is
`4e48a6f7feda270b5a1e4c1b81db570a8fddf2756ed1dcf4eebe3bcd9f2cdc5e`.

`scripts/check-py006-uci.py` repeats the selections, verifies root hashes and
message presence, and compares to that frozen baseline (or an explicitly
supplied baseline binary). Its deterministic report records contracts,
diagnostics, identities and artifact hashes. The same script additionally
extracts each actual pinned scalar declaration into a task-owned one-value
selection, explicitly labeled **not a complete real-UCI service**. This measures
the bounded profile improvement separately from unchanged service readiness.

The isolated pinned campaign executed successfully: **eight selections**
(two profiles × two releases × two worlds), each changing model/API/codec and
aggregate **NOT READY → READY**. Baseline first rejection is
`unsupported Python IR construct: unsupported constraints`. Each generated
one-value selection passed independent literal-oracle checks normally and under
`-O`, standalone and package imports (32 one-test subprocesses per interpreter).
The deterministic full comparison, including exact commands, contracts,
BEFORE/AFTER diagnostics, runtime qualification labels and artifact hashes, is
stored beside the fixture as `after.json`. Artifact bytes are identical across
releases and worlds for each extracted profile:

| Extracted profile | Artifact | SHA-256 |
| --- | --- | --- |
| Schema version | model.py | `dc9537778403b1123d850aa67d0b5bcbb6c7d697b98404004d601087010bf6ce` |
| Schema version | service_api.py | `f9fe3508a33b8bebd86776dd90096507c4123242b377cfc2730bc5565db74e97` |
| Schema version | service_codec.py | `835982ccd1c74bfd5f5d6039706da034050ea50fe883e22eed67204b29701262` |
| UUID | model.py | `68d4994c2c5dd7d09ed82070d091043081df7b4ea286fad1d1d45c5f54842445` |
| UUID | service_api.py | `fe9b3be5481eede07bc6f0958a84149c8f8e223fd3a86a5044c181d8e8f84064` |
| UUID | service_codec.py | `10d8f6c7d449433f984cae9aee7134922381645748262788b32c7e4c67f49cd7` |

These isolated profile artifacts are not PositionReport artifacts and do not
qualify any complete real-UCI service against LA-CAL or Sleet.

## Independent synthetic and broker qualification

`tests/fixtures/python-header-profiles` is new task-owned input; PY-004/PY-005
fixtures and expected results are untouched. It covers direct profile payloads,
required/optional fields, nested records, bounded repetitions, distinct global
messages sharing one payload, and a selected/unselected negative near-match.

The Python probe has seven must-execute unittest methods, running in both worlds,
standalone/package import configurations, normal/optimized execution: **56
method executions per interpreter**, including the 41-case schema-version and
78-case UUID frozen corpora and an independently authored Cartesian
version/variant oracle. Literal expected JSON is checked independently of
round trips. Rust admission/CLI tests mutate bounds, pattern branches,
alternatives and whitespace and verify no output or overwrite on rejection.
Foreign-namespace and OAM-name-only controls distinguish value-space admission
from wire mapping identity.

Actual synthetic OAM broker tests executed on both Python versions, normal and
`-O` (four executions), using one shared pinned Sleet checkout/build:

* Sleet revision `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`.
* Clean tree `7e705f4eb5f658957bd414b4abc8207311f3c1e4`.
* Executable SHA-256
  `4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
* Execution marker:
  **`PY006 PINNED SLEET HEADER RECEIPT AND TYPED DELIVERY: PASSED`**.

The production CLI generates real models/API/codec; actual types and functions
are registered with `LacalRuntime`. Two validated OWP connections subscribe and
publish via the generated facade. The broker's own route marker and the typed
handler's exact values/spellings are required. An injected invalid UUID is
rejected by the **unmodified broker** as `Invalid-Message` before routing; the
generated decoder independently rejects the same body. Subsequent valid typed
delivery succeeds, corrupted outbound models fail, unsubscribe removes the
subscription, and close leaves no runtime tasks. This does not pretend an
invalid broker-rejected body reached the typed handler. Raw duplicate-safe
envelope processing is unchanged.

The existing 19-case PY-005 real-network mock suite also passed both versions
normal/optimized (38 method executions per interpreter). The shared Sleet
harness retains all inherited Rust campaigns and reuses its broker for PY-006.
Synthetic broker success is **not** real-UCI Python runtime qualification.

The full shared Sleet harness also completed with exit **0**: nine inherited
Rust test executions, existing PY-005 normal/optimized tests, and all four
PY-006 interpreter/policy executions. Required legacy markers included
`REAL UCI POSITIONREPORT THROUGH REAL SLEET: PASSED` and
`REAL UCI SUBSYSTEMSTREAM THROUGH REAL SLEET: PASSED`; those are **Rust**
regressions, not Python real-service support. The final harness marker was
`real pinned Sleet (e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b): PASSED`.
The first slow unoptimized build-script attempt was explicitly interrupted and
not counted. The successful rerun used a task-owned Cargo target directory with
`CARGO_PROFILE_DEV_BUILD_OVERRIDE_OPT_LEVEL=3`, unchanged tests and the same
single clean pinned broker build.

## Reproduction and delivery gates

Use distinct interpreters, never count one executable twice:

```sh
for PYTHON in python3.11 python3.13; do
  export PYTHON
  cargo +1.95.0 test --locked -p ams-gra-oms-backend-python -- --nocapture
  cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
    --test python_cli --test python_service --test python_codec \
    --test python_header_profiles -- --nocapture
  "$PYTHON" -m pip install ./python/oms-lacal-runtime
  bash scripts/check-py005-mock.sh
done
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --workspace --all-targets --locked
cargo +1.95.0 clippy --workspace --all-targets --locked -- -D warnings
AMS_GRA_REQUIRE_GNAT=1 cargo +1.95.0 test --workspace --locked
bash scripts/check-ci-split.sh
bash scripts/test-check-ci-split.sh
git diff --check

# External verified roots; downloads remain Deep-only.
export AMS_GRA_UCI_2_5_ROOT="$(bash scripts/fetch-pinned-uci-2.5.sh /external/uci25)"
export AMS_GRA_UCI_2_6_ROOT="$(bash scripts/fetch-pinned-uci-2.6.sh /external/uci26)"
cargo +1.95.0 build --release --locked -p ams-gra-codegen-oms
python3.11 scripts/check-py006-uci.py --generator target/release/ams-gra-codegen-oms \
  --output /external/py006-measurements
PY006_PYTHONS='python3.11 python3.13' bash scripts/run-real-sleet-test.sh /external/sleet
```

Fast CI adds only the new synthetic CLI suite to the established Python matrix.
Deep reuses existing root fetches and the single Sleet build, preserving job
names, dependencies and hard-failure behavior. Local split guard and its
**218-check self-test** passed. The original model/API/report parity campaign
passed: six Ada/Rust/C++ full models, twelve selected model/API runs, twelve
readiness reports, two coverage reports, and eight Python model/API runs and
reports. Canonical parity digest:
`b0749e258b6484b568c07b60f81b855353565488a5b26fc3ed224b160cfab235`.

Local GNAT-required locked workspace regression completed: **1,479 passed,
zero failed, eight pre-existing ignored external/environment-gated tests**.
Compiler-backed Ada, Rust and C++ probes actually executed; those eight ignored
tests are not counted as qualification, and the separate pinned/Sleet campaigns
must execute their own markers. Per target interpreter the focused backend suite
has **12 Rust tests**, and Python CLI/service/codec/header suites have **19 Rust
tests**; their subprocess output records **272 Python unittest method
executions** per interpreter (136 normal, 136 optimized). The new header suite
accounts for 56 of those executions. Both target versions passed, zero failures
or skips. Mock-network and Sleet totals above are additional, not double-counted
as model/CLI tests. Formatting and the first locked workspace check/Clippy runs
passed; final-head reruns and hosted status belong in delivery evidence.

Final test totals, source/tree identities, local/remote/PR parity and exact-head
hosted checks are recorded in the PR handoff. A running or missing mandatory
hosted check is **HOSTED_CI_PENDING**, never READY_FOR_INDEPENDENT_REVIEW.
Any unresolved mandatory failure is **BLOCKED**. No self-approval, merge,
auto-merge, force-push, release or follow-on task is authorized by this work.

## Explicit remaining boundaries

Generic XSD patterns/arbitrary constrained Strings; Choice; abstract and
polymorphic values; general complex inheritance; Float32/64, Decimal, Binary
and temporal fields; unsupported nillability/repetition; unknown `$type`
subtypes; reconnect/resubscribe/replay; dynamic schema authority; packaging
publication and JavaScript remain unsupported. Existing selected projection,
ServiceApiModel authority, GenerationWorld behavior and three-language legacy
inventory are unchanged. The first real-service blocker is recorded above,
not worked around to force a READY verdict.