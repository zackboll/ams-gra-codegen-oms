# Task 087B — fixture and model baseline only

The fixture-first baseline below is preserved as historical evidence. The
implementation continuation is recorded in the final section; review readiness
also requires completed regression and publication gates.

## Isolation and dependency

- Fetched main: `89702ef09c5966b54dc43222da5823e8abd05fb2`.
- Upstream PR #87 head and Task 087B starting SHA:
  `513139017097e23800b88a4973e53e07381f6753`.
- PR #87 was open at inspection; `ada-json-guard` passed, while Rust and Deep CI
  checks were still running. No failed upstream gate was observed.
- Branch: `feature/087b-ada-oms-codec`.
- Worktree: `/home/zboll/git/ams-gra-codegen-oms-task087b`.
- Scratch, generator target, model sources, and compilation logs:
  `/tmp/task087b`.
- The new worktree was created directly from
  `origin/feature/087-ada-oms-json-codec`; exact upstream ancestry was verified.
  Neither the upstream branch nor its worktree was modified.

## Fixture-first baseline

`codec-ada-minimal.xsd`, `.yaml`, and `.json` in
`tests/fixtures/service-generate` are synthetic, task-owned inputs. JSON was
authored independently before any codec implementation. The qualified OAM
fixture selects two global message declarations with one identical payload
QName and five selected type declarations. The first generation attempt lacked
the required UCI version attributes on the global elements; correcting the
fixture attributes made the unchanged production generator succeed.

Production `service-generate`, without `--with-codec`, emitted:

- `programs.ads`
- `programs-oam.ads`
- `programs-oam.adb`
- `service_api.ads`

The actual model has:

- `Level` as an Ada integer type with range `-10 .. 10`.
- Direct Boolean as `Boolean`; direct String as `Unbounded_String`.
- `SignalCode` literals `NORMAL` and `Value_5G`.
- `Label` as a private checked carrier with `Create` and `Value`.
- A nested public `Details` Record.
- Wire member `begin` represented as `Field_begin`.
- Optional direct String as shared `Optional_String`.
- Optional named Label as `AdaPayload_OptionalLabel_Optional`.
- Both optionals as discriminated Records with `Is_Present` and a `Value`
  component only in the present variant.

`tests/ada-generated-codec/model_consumer.adb` is a **model-only** construction
probe, not a codec test. Complete aggregates construct valid checked values and
present/absent optionals without evaluating the carrier's failing default.

GNAT **14.2.0** and **16.1.0** each compiled and executed the generated model,
wrapper, and construction probe under normal, Check, and Ignore assertion
policies. All six runs printed `TASK087B_MODEL_BASELINE_PASS`.

Reproduction: generate the fixture into a fresh scratch directory without
`--with-codec`, copy the model consumer there, and compile with
`gnatmake -gnat2022 -gnato -O0 model_consumer.adb`. For the explicit policies,
add `-f -gnatec=/absolute/scratch/policy.adc`, containing
`pragma Assertion_Policy (Check);` or `pragma Assertion_Policy (Ignore);`.

## Limited regression evidence

The existing CLI `service_codec` integration suite passed: **11 passed, 0 failed,
0 ignored**. Rust 1.95.0 formatting and `git diff --check` passed. Expected JSON
was parsed with Python's JSON validator. No production source or workflow was
changed. The existing Ada codec LanguageNotImplemented boundary remains intact.

No codec positive/negative execution, byte-parity campaign, full locked workspace
check, warnings-denied Clippy, hosted generated-codec execution, real UCI codec
measurement, remote/PR SHA parity, or independent-review readiness is claimed.
The partial baseline was preserved locally without publishing a codec PR.
Do not merge before PR #87 and do not begin Task 088.

## Implementation continuation

Preserved baseline commit: `0e659c020392955f28c30f4f96b244e55537ed95`.
No reset, amendment, rebase, or upstream modification was performed.

Shared Ada codec readiness now checks the actual projected emission plan,
including generated support declarations, in deterministic emission/member
order. Rust's capability branch is unchanged; C++ remains unimplemented.
Zero-OMS services emit no codec. Predictable model-package/dependency visibility,
artifact, and case-insensitive codec declaration collisions fail pre-generation.
Model and wrapper naming preflight remain authoritative in their own scopes.

Supported: Boolean; signed integers within the model's supported signed-64
domain; unsigned domains wholly within `0 .. Long_Long_Integer'Last`; plain
direct String; existing checked named String profiles; enums with explicit wire
mapping; concrete Records with required/optional single-occurrence fields;
nested Records and effective inherited fields. Optional values use complete
discriminated aggregates, never default-initialized checked carriers.

Unsupported: Choice; repetition; abstract/polymorphic values and `$type`;
Float32/Float64/Decimal; DateTime/Time/Duration; Binary; nillability; unqualified
members; unsupported field facets; alias/list declarations; integer domains
outside the signed-64 provider boundary. Existing model boundaries are not
expanded. In particular, unconstrained named String and `xs:unsignedLong` are
already rejected upstream by model readiness/frontend respectively. The core
IR test independently checks the unsigned signed-64 boundary.

The production CLI emits `service_codec.ads` and `service_codec.adb` alongside
the original model/API, reporting **two** codec files. `Service_Codec` exports:

```ada
Codec_Error : exception;
function Encode_AdaPayload (Item : Programs.Oam.AdaPayload) return String;
function Decode_AdaPayload (Text : String) return Programs.Oam.AdaPayload;
```

Payloads are deduplicated by full QName. All document reads/writes use the
unchanged `AMS_GRA_OMS_JSON_Guard`; no scanner or unguarded document operation
is emitted. GNATCOLL JSON integer kind is mandatory; fractional/exponent tokens
are rejected. Unknown/missing Record members, wrong kinds, null, invalid enums
and model-domain violations raise `Codec_Error`. Checked model constructor and
integer conversion `Constraint_Error` are translated only at those narrow
operations; unexpected internal exceptions remain failures. Duplicates retain
the documented last-value-wins behavior. No envelope or transport is added.

### Executable evidence

`scripts/check-task087b.py` invokes the actual production generator, copies the
unchanged support project to task-owned scratch, pins it as an Alire path
dependency, checks Alire 2.1.1 / GNATCOLL 26.0.0 archive hashes / GPRBUILD 26.0.1
and the resolved compiler, and builds and executes each assertion policy.

The original golden corpus executes **41** explicit checks per policy,
including field equality, independent golden structural comparison, round trips,
optional absence/presence, nested Records, reordered members, exact wire names,
Unicode, and the requested rejection categories. Its exact completion marker is
`TASK087B_CODEC_COMPLETE cases= 41`. Only `Codec_Error` is caught in negative
tests; any other exception fails execution. An additional generated in-subset
consumer executes **7** checks per policy for inheritance, direct signed/unsigned
bounds, named unsigned/Boolean values, and optional nested Records; exact marker
`TASK087B_SUBSET_COMPLETE cases= 7`. Both complete campaigns pass with GNAT
14.2.0 and 16.1.0 under normal, Check and Ignore policies.

Final evidence roots:
`/home/zboll/task087b-scratch/certified14` and
`/home/zboll/task087b-scratch/certified16-final`; logs and exit records are also
under `/tmp/task087b`. Scratch was moved to disk after `/tmp` tmpfs exhaustion
caused compilation failures; those failed runs were not certified. One extra
subset consumer needed a `use type Interfaces.Unsigned_64` visibility clause;
the corrected consumer was requalified. No support implementation was changed.

The protected `task050_ada_and_cpp_have_no_generated_codec` name and workflow
registration remain; Ada now asserts its actual unsupported feature boundary,
while C++ retains LanguageNotImplemented. Two independently named CLI tests
cover successful Ada generation/deduplication/model/API byte preservation and
unsupported/collision atomicity. The CLI suite passes **13/13**, and focused
core readiness tests pass **2/2**.

An upstream generator was built from an archived exact Task 087A commit.
Recursive byte comparisons on `codec-oam` passed for Rust model/API/codec,
Rust no-codec output, Ada no-codec output, and C++ output. The new synthetic
Ada test additionally compares every model/API file with and without codecs.
Zero-OMS tests pass for all three languages. Protected support, Rust renderer
and C++ backend trees have an empty diff against upstream.

The separate mandatory hosted job `ada-generated-codec` depends on the intact
`ada-json-guard` job. It uses the same SHA-256-verified Alire installer, builds
the production generator, and executes both generated consumers under all three
GNAT 16 policies with exact nonzero completion counts. Hosted status is reported
separately from local success; registration alone is not hosted execution.

Final affected suite command (Rust 1.95.0, locked; disk-backed TMPDIR/target):
`cargo test -p ams-gra-oms-codegen-core -p ams-gra-oms-backend-ada
-p ams-gra-oms-backend-rust -p ams-gra-oms-backend-cpp -p ams-gra-codegen-oms`.
It completed exit **0**, with **1,077 passed, 0 failed, 6 existing ignored** across
112 target summaries. The ignored cases require independently provisioned real
UCI research inputs; no new synthetic test was ignored. The final 13-test CLI
suite was rerun after its added atomicity controls. Rust 1.95.0 locked workspace
all-target check, warnings-denied Clippy, formatting, whitespace checks, Python
syntax and workflow YAML parsing pass. Task 087A regression executes 199 corpus
cases plus four boundaries under all three GNAT 16 policies, retains both
frozen hashes, and passes its clean consumer and eight adversarial wrapper tests.

### Bounded real UCI measurement

Existing cached UCI roots were hash-checked: 2.5 SHA-256
`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`,
2.6 SHA-256
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
An RDMA_InitializeSetup service-check measurement was launched separately from
synthetic qualification. No real-UCI codec compilation/execution or READY claim
is made. Real measurement results, when available, remain research-only; no
implementation expansion is authorized by them.