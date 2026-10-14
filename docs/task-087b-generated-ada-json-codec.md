# Task 087B — fixture and model baseline only

**NOT_READY: implementation is incomplete.** There is no demonstrated upstream
or model blocker. The generated codec renderer, Ada codec readiness, CLI codec
integration, error boundary, executable codec corpus, hosted codec gate, and
publication qualification have not been implemented. This document must not be
read as completion of Task 087B or as a claim of executable Ada JSON codecs.

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
The partial baseline is preserved locally; no completed-codec PR is published.
Do not merge before PR #87 and do not begin Task 088.