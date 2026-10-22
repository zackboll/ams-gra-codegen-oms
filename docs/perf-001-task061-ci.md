# PERF-001 — Reuse parsed schemas in Task 061 service qualification

## Scope and provenance

Starting `origin/main`: `23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39`;
tree `61f9e921e89133bd43b65071186567e3f0f9d5bf`.
Branch: `feature/perf-001-service-check-batching`.
The clean-tree preflight fetched main and inspected all six open PRs (#99–104).
None was a performance PR. Python CLI PRs #99/#100 overlap
`crates/cli/src/lib.rs`; this branch does not incorporate or modify them.

The user-reported historical hosted times (~135 minutes for
`real-uci-regressions`, ~58 minutes for Task 061) are context, **not measurements
made by this change**. They have no supplied exact-head run URL.

## Bottleneck and architecture

Originally, `scripts/check-task061-service-impact.py` scheduled 609 standalone
production `service-check` processes with four pool workers. Every process ran
`load_service_inputs` in `crates/cli/src/lib.rs`, independently loading the same
release schema through `load_schema_set_with_overlays` in
`crates/xsd-frontend/src/lib.rs`. The frontend parses XML, normalizes
restrictions and validates the IR; the CLI then explicitly validates the IR
again. No validation has been removed.

`service-check` now separates schema loading from `evaluate_service_check`.
Both standalone and batch use that evaluator, including
`analyze_service_readiness`, codec checks and the existing deterministic
`render_service_check_with_codec`. Each contract is loaded, extension mapping
checked and `resolve_service_plan` called independently. No plan, projection,
readiness verdict or generated output is cached. No backend/core readiness
algorithm, generator output, dependency, unsafe Rust or global cache changes.

The new opt-in production CLI command is:

```sh
ams-gra-codegen-oms service-check-batch --schema /absolute/root.xsd --manifest /absolute/requests.json
```

```json
{
  "format": "service-check-batch-v1",
  "schema_version": "002.5.0",
  "requests": [
    {
      "id": "unique-selection-id",
      "contract": "/absolute/contract.yaml",
      "language": "rust",
      "world": "closed-schema",
      "with_codec": false
    }
  ]
}
```

Unknown fields, duplicate JSON fields/IDs, missing required fields, empty
batches, relative contract paths, invalid options and wrong schema versions
fail. `with_codec` defaults to false; optional `ada_codec_runtime` uses the
standalone parser and restrictions. This initial interface deliberately
supports **no extension overlays**: a contract declaring extensions fails the
existing exact mapping check, rather than ignoring extensions or loading a
different schema. Standalone overlay/extension behavior is unchanged.

The response is one `service-check-batch-results-v1` JSON object containing the
schema path/version, one schema load, explicit completed count, and ordered
records echoing each request plus `exit_code`, `stdout`, `stderr`. Those streams
are the standalone report and `error: ...\n` diagnostic, not an alternate
readiness representation. Any failed selection makes the process exit 1, but
does not prevent remaining selections from running. A process failure before
completion, invalid/truncated JSON or output-write failure is never completion.

## Schema identity, isolation and fail-closed driver

The unchanged fetch scripts verify the exact release commits and roots:

| Release | Upstream commit | Root SHA-256 |
| --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

The driver additionally hashes both roots before execution. The generic batch
CLI checks the normalized schema version; it is not a release download or
cryptographic pinning API. The qualification harness owns pinned identities.

Each worker process exclusively owns its loaded schema, borrowed immutably
during evaluation. Parallelism is across isolated processes, not shared mutable
state or unsafe Rust. Each run has a fresh `mkdtemp` directory; each batch has
its own manifest and raw output directory. Contract paths and IDs are unique.
The driver rejects duplicate/missing/extra/reordered/mismatched results,
incomplete counts, duplicate JSON keys, unexpected schema identity and process
exit/result disagreement. It persists individual logs before accepting exit
status. Invalid batch execution writes an attributed failure log for every
affected selection and raises; there is **no fallback to legacy**.

The unchanged fixture hash is
`351d418883ce73e3339bfe2b4af349ff901bf37788b1747d4544dff8082b960b`.
All 609 world tuples and 444 release/backend/message tuples remain required:

| Release | Closed per backend (Ada/Rust/Cpp) | Open per backend | Total |
| --- | --- | --- | --- |
| 2.5 | 72 / 72 / 72 | 26 / 26 / 26 | 294 |
| 2.6 | 76 / 76 / 76 | 29 / 29 / 29 | 315 |

Thus each backend has 203 checks; closed world has 444, open world 165.
Reports and `SERVICE-CHECK` lines are restored to original fixture order.
The historical PASSED marker is unchanged and only follows all 609 zero exits,
READY reports and empty diagnostics. Explicit exceptions, not removable Python
`assert` statements, enforce these requirements.

## Reproduction and instrumentation

Local measurements were made on Linux, AMD Ryzen 9 7945HX, 16 physical/32 logical
CPUs, ~85 GiB RAM, Rust 1.98.1, GNAT 14.2.0, release builds with locked
dependencies. This is a shared developer host with other builds running.
Four-core affinity is **not equivalent to a standard GitHub-hosted runner**;
thermal, SMT, cache and scheduling interference remain limitations. Benchmark
comparisons use CPU affinity 0–3. The full equivalence qualification uses a
separate eight-CPU affinity and is not a comparable benchmark.

`/usr/bin/time` was unavailable (initial launch failed with exit 127, before
running a check). `scripts/measure-task061.py` uses Linux `getrusage` child
accounting, including waited descendants, and monotonic wall time instead.
It records user/system CPU separately and effective CPU percentage (100% is
one fully occupied CPU). `max_process_rss_kib` is the kernel high-water mark
of the largest individual child, **not summed concurrent memory**. The later
wrapper also samples summed process-tree VmRSS at 50 ms intervals; this can
double-count shared pages and miss short peaks. The initial baseline did not
have that sampler; no baseline aggregate RSS is invented.

```sh
# Run from the repository; choose scratch directories outside it.
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh /tmp/perf-uci25)"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh /tmp/perf-uci26)"
export TMPDIR=/tmp/perf-evidence
mkdir -p "$TMPDIR"
cargo build --release --locked -p ams-gra-codegen-oms
export AMS_GRA_TASK061_WORKERS=4
python3 scripts/measure-task061.py "$TMPDIR/legacy.json" taskset -c 0-3 \
  python3 scripts/check-task061-service-impact.py target/release/ams-gra-codegen-oms --mode legacy
python3 scripts/measure-task061.py "$TMPDIR/batch.json" taskset -c 0-3 \
  python3 scripts/check-task061-service-impact.py target/release/ams-gra-codegen-oms --mode batch
# Mandatory initial semantic qualification, intentionally opt-in thereafter:
python3 scripts/check-task061-service-impact.py target/release/ams-gra-codegen-oms --mode equivalence
```

Every modern driver run preserves binary/fixture/schema hashes, source commit
and tracked diff hash, CPU affinity, manifests, individual logs, full ordered
results and per-mode resource/count summaries. It takes a private executable
copy before evaluation, so concurrent rebuilds cannot change the binary halfway
through a run. For a clean comparison build
once and use `--mode equivalence`: the same executable evaluates both modes
against the same files. Exact comparison includes request identity, every exit,
full report and full diagnostic. No missing selection can satisfy equality.

The additional starting-main baseline preserves an unmodified binary (SHA-256
`ea7d8d00ba99c1149c78181e8ea3faf7899d77bc05d014a9c2f21cd3e0e5f4c2`)
and original driver, rather than substituting a rewritten standalone algorithm.

### Narrow phase profiling

`docs/perf-001/profile.patch` applies **only to starting main in a disposable
worktree**. It writes timings through `PERF001_PHASE_LOG` to a separate file,
not stdout/stderr. Do not apply it to a production build. It separates XML
loading/parsing, restriction normalization, frontend IR validation, CLI IR
validation, contract resolution, readiness and report rendering/writing.
The two frontend validation timings are nested in frontend total; do not add
that total to its subphases. Raw TSV measurements are retained beside the
patch. Representative AMTI_CapabilityStatus reports were compared byte for
byte with uninstrumented starting-main production output. Sample timings
describe that selection only, not estimates of every selection's cost.
The patch uses zero context to avoid whitespace-only diff context; apply with
`git apply --unidiff-zero /absolute/path/to/docs/perf-001/profile.patch` in a
disposable checkout of the starting commit.

Three detailed samples per release (AMTI_CapabilityStatus, Ada, closed world,
single CPU affinity 4; shared-host interference present) gave these **measured
means**. The release-2.6 repeat used its own release-2.6 contract and compared
each report with its matching uninstrumented baseline log. An earlier detailed
2.6 exploratory sample reused the 2.5 contract; its raw rows are retained in
`phases-detailed.tsv` but are not used in this table.

| Non-overlapping phase | UCI 2.5 seconds (% phase sum) | UCI 2.6 seconds (% phase sum) |
| --- | --- | --- |
| XML loading/parsing | 6.762927 (98.232%) | 9.549239 (97.031%) |
| Normalize restrictions | 0.043756 (0.636%) | 0.082429 (0.838%) |
| Frontend IR validation | 0.022003 (0.320%) | 0.055496 (0.564%) |
| CLI IR validation | 0.021641 (0.314%) | 0.051040 (0.519%) |
| Contract loading | 0.000086 (0.001%) | 0.000125 (0.001%) |
| Resolve service plan | 0.005896 (0.086%) | 0.017801 (0.181%) |
| Analyze readiness | 0.028313 (0.411%) | 0.085272 (0.866%) |
| Render/write report | 0.000015 (0.00022%) | 0.000019 (0.00020%) |

These nested stopwatch samples exclude startup/destruction and are not a full
process wall-time baseline. The evidence supports targeting repeated parsing;
long runtime alone would not establish CPU saturation. Full-run CPU accounting
is the independent evidence for utilization.

## Worker configuration and operational guidance

`AMS_GRA_TASK061_WORKERS` defaults to **4**, validated as an ASCII integer
**1–32**; invalid values fail before evaluation. At four workers there are two
partitions per release: four CLI invocations/schema loads, rather than 609.
At one worker the two release batches run serially (two schema loads). At
two workers they run concurrently; larger counts split each release into
round-robin, isolated partitions while restoring original reporting order.

Use the default on four-vCPU GitHub runners. Higher values are permitted for
self-hosted machines but must be justified by wall/CPU/memory measurements;
oversubscription adds schema loads and memory and can increase total CPU cost.
Eight workers on four CPUs is a measurement experiment, not the new default.

## CI and qualification policy

Deep CI retains all six jobs, per-ref cancellation, 240-minute real-UCI
timeouts, pinned fetches and all existing Task 052–060 and generated-code/GNAT
gates. Fast CI is unchanged. The Task 061 driver and its transport test now
match the Deep PR path filter. A small mandatory control step runs actual
production fixture equivalence and Python transport negatives; the 609 real
checks remain mandatory. Full before/after baseline runs are **not** added to
routine CI. Individual Task 061 evidence is uploaded even after failure.

The existing layout guard needed a narrow exception for that exact,
evidence-only `if: always()` upload. Arbitrary conditional steps and
`continue-on-error` remain forbidden; new mutations verify the exception
cannot be repurposed. All inherited payload digests remain unchanged.

Results and hosted-head qualification are recorded below only once observed;
local passing tests alone do not establish independent-review readiness.

### Local validation observed

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --all-targets`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo +1.95.0 check --locked -p ams-gra-codegen-oms --all-targets`: pass.
- Production CLI tests (`service_check`, `service_check_batch`, `service_codec`):
  21 + 4 + 23 passed. Batch equivalence executes actual production CLI calls
  for 24 backend/world/codec/ready-or-blocked combinations plus missing input,
  unmapped extensions and a succeeding final request. Negative controls reject
  malformed JSON, duplicate IDs, missing fields/files, wrong schema versions,
  unknown fields, relative paths, invalid backend/runtime options and empty
  manifests.
- Python transport suite: six tests, including duplicate/missing/extra and
  reordered result identities, wrong root digest/version, partial completion,
  malformed/truncated/duplicate-key JSON, worker bounds, attributed failure
  logs and a successful subprocess exit attempting to hide a failed selection.
  Fault injection exercises **transport only**, not a substitute readiness
  algorithm.
- `scripts/check-ci-split.sh`: pass;
  `scripts/test-check-ci-split.sh`: pass, 214 checks including required batch
  control/path-filter mutations and the narrow-upload
  negative controls.
- `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`: pass, 180 libtest summaries,
  1,441 passed, zero failed, eight pre-existing ignored external campaigns.
  No pinned roots were supplied to this Fast-CI-equivalent invocation. Those
  ignored campaigns are not counted as real-UCI evidence. Relevant pinned
  regressions are run separately with both roots and GNAT required.

The exploratory equivalence run was explicitly stopped (exit 143) after a
help-only rebuild changed the executable on disk while standalone subprocesses
were still being launched. It is **not qualification evidence**. The replacement
uses the frozen production executable SHA-256
`39d31b52e98e3655083244df1706ac96e2b1e18e8cac9ff44f860ffcf2c46e07`
for both modes. The driver now freezes its own private copy to prevent this
measurement-provenance hazard in future comparisons.

## Remaining optimization opportunities

Schema reuse avoids repeated XML loading/parsing but intentionally repeats
contract resolution, binding checks, selected projection, projected IR
validation and backend readiness for each independent request. If those become
dominant, measure them before proposing immutable indexes or carefully scoped
caches. Never cache a verdict by message name alone: backend, world, schema
binding, wrapper names, generated support and codec options are semantic inputs.
The frontend parser itself is a potential follow-on target, but changing it
is outside this optimization and would need its own normalization/equivalence
qualification. Other real-UCI Deep gates still account for job runtime.