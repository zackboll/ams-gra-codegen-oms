# PERF-003 — bounded Task 062 service qualification

Status: **HOSTED_CI_PENDING**. Measured local qualification and hosted results are
tracked separately below; no pending or skipped external campaign is a pass.

## Provenance and scope

Starting main commit: `23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39`.
Starting tree: `61f9e921e89133bd43b65071186567e3f0f9d5bf`.
Fetched `origin/main` before creating branch
`feature/perf-003-task062-parallel` in independent worktree
`/home/zboll/git/ams-gra-codegen-oms-perf-003`. No existing PERF-003 branch/PR
was found. PERF-001 (#105) and PERF-002 (#106) branches were not modified or
cherry-picked. Production Rust sources, fixtures, workflows, compiler verticals,
inventory tests, runner/job counts, timeouts, cancellation and pin checks stay
unchanged. The PR head/tree are available from Git/PR metadata (a commit cannot
contain its own hash).

Historical successful main run:
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39699205279>.
Its head matches the starting commit. User-supplied hosted timings: full Task 062
20m41s; inventory/coverage ~8m33s; 54 sequential checks ~9m24s; compiler verticals
~2m45s. These are historical observations, **not** the local baseline.

Pinned production inputs (not checked into this repository):

| Release | Upstream commit | Root SHA-256 |
|---|---|---|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

Existing clean upstream checkouts were inspected read-only; 2.5 schema contents
match Git. The 2.6 pinned archive was independently extracted using the existing
safe extractor and the complete extraction compared identically to the reused
root directory. Both root hashes match the existing qualification pins.

## Execution and isolation

`ThreadPoolExecutor` launches at most `AMS_GRA_TASK062_WORKERS` production CLI
processes. Valid explicit counts are 1–4; unset uses the benchmark-selected
default. Empty, signed, fractional, padded, zero and excessive counts fail before
launch. One worker remains the reference mode. The maximum is deliberately four;
future self-hosted scaling requires explicitly raising and requalifying that cap,
not unbounded creation or extra Actions jobs.

Each invocation uses an exclusive `mkdtemp` campaign under `TMPDIR`. Fixture
indices 00–53 own distinct directories, unchanged contract bytes, diagnostic JSON,
raw `.stdout` and raw `.stderr`. Schema roots are shared **read-only**. Inspection
of production `service_check`, input loading, report rendering and XSD frontend
found no mutable on-disk cache or generated output on this command path. Cargo is
not launched by service-check. Contracts are prepared before dispatch and never
rewritten by a worker.

Workers may complete out of order. Futures are retrieved/published in frozen
fixture order; complete stdout/stderr bytes remain separate on disk and in their
respective console streams. The manifest records identity, index, exact argv,
contract content, exit status, elapsed duration and diagnostic path. A campaign
starts incomplete and becomes complete only after all results and logs validate.
Missing, duplicate, extra, wrong or reordered results; worker/publication errors;
signals; nonzero exits; absent logs; malformed completion; and incomplete or
non-READY reports fail closed. The exact production READY grammar, matching
backend/world and equal total/renderable counts, is checked with runtime
validation (no removable Python assertions). Subprocesses have a 600-second
per-selection limit; timeout kills and waits for the CLI, retains partial streams,
and fails the campaign. The original final success marker is unchanged:

```text
TASK062 ALL 54 NEWLY READY CLI TUPLES: PASSED
```

Evidence is retained on success and failure; CI console diagnostics remain visible.
This does not add an artifact upload job or change workflow retention policy.

### Frozen selections

The authoritative order remains
`tests/fixtures/string/task062-new-ready.tsv` (unchanged). For each release 2.5
then 2.6: ClosedSchemaSet includes Ada, Cpp, Rust, each selecting IO_PortCommand,
IO_PortStatus, ProductOrFileDisseminationDestination, RDMA_Initialize,
RDMA_InitializeSetup; OpenExtensions includes Ada, Cpp, Rust, each selecting the
same ordered messages except ProductOrFileDisseminationDestination. Thus
`2 × (3 × 5 + 3 × 4) = 54` exact selections and `2 × 3 × 5 = 30` unique
release/backend/message identities. Nothing is deduplicated at execution time.

## Reproduction and measurement

Local host: Linux x86-64, AMD Ryzen 9 7945HX, 16 physical/32 logical CPUs,
85 GiB RAM; commands restricted with `taskset -c 8-11` (four distinct physical
cores, one logical CPU each). This is four-vCPU-equivalent affinity, **not** a
dedicated GitHub VM or a hard whole-machine resource reservation. Background
desktop activity and frequency/thermal variation remain possible.
Debian kernel `6.12.113+deb13-amd64`, Python 3.13.5,
Rust 1.98.1 (`48a229cea`), GNAT 14.2.0, GCC/G++ 14.2.0.
The release production binary was rebuilt from the starting main in the new
worktree using `cargo build --release --locked -p ams-gra-codegen-oms`.

`/tmp` had exhausted inodes; the initial build failed and was retried successfully
with a campaign-owned home-filesystem TMPDIR. No other campaign's files were
removed. Detailed local evidence lives at `/home/zboll/perf-003-evidence`.
`docs/perf-003-task062-evidence.json` preserves portable baseline and corrected
benchmark metrics plus all 216 production command records (identity, exact argv,
contract, exit, duration, complete stdout/stderr encoded as base64). Per-selection
diagnostic paths are also recorded by the driver manifests and captured console.

```bash
cd /home/zboll/git/ams-gra-codegen-oms-perf-003
export TMPDIR=/home/zboll/perf-003-evidence/tmp
export AMS_GRA_UCI_2_5_ROOT=/tmp/perf-001-uci25/03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/tmp/perf-001-uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
export AMS_GRA_REQUIRE_GNAT=1 CARGO_BUILD_JOBS=4
mkdir -p "$TMPDIR"
cargo build --release --locked -p ams-gra-codegen-oms
# Original baseline ran the untouched driver before it was edited.
# To reproduce later, use a separate worktree at the starting commit, keeping
# its original scripts/ path so its fixture resolution remains unchanged.
for workers in 1 2 4; do
  AMS_GRA_TASK062_WORKERS=$workers taskset -c 8-11 python3 \
    scripts/benchmark-task062-service-impact.py \
    "$PWD/scripts/check-task062-service-impact.py" \
    "$PWD/target/release/ams-gra-codegen-oms" \
    "/home/zboll/perf-003-evidence/workers-$workers" \
    >"/home/zboll/perf-003-evidence/workers-$workers.stdout" \
    2>"/home/zboll/perf-003-evidence/workers-$workers.stderr"
done
```

The benchmark wrapper executes the driver and intercepts only subprocess
observation: it does not replace the CLI or change argv. It preserves per-command
stdout/stderr as base64, argv, contract and elapsed duration in `commands.json`.
`metrics.json` records wall time, aggregate child user/system CPU, effective CPU
utilization, page faults, block I/O, sampled `/proc` I/O and aggregate process-tree
RSS every 50 ms. Thread-launched children are enumerated under **all** parent
tasks. Memory is an observed sampled high-water mark, not an exact instantaneous
maximum; shared pages may be counted repeatedly. Driver/sampler CPU is reported
separately. Initial 1/2-worker runs used a main-thread-only children sampler and
their memory numbers are invalid; corrected repeat runs are the comparison set.

### Local results

Original unchanged driver: **311.594821 s** wall, **310.261035 s** user,
**1.284068 s** system, **311.545103 CPU s**, **99.984%** effective CPU,
**76,419,072 bytes (72.88 MiB)** sampled peak tree RSS; exit 0, 54/54 passed.
All original per-command streams, statuses, identities and durations are retained.

No major faults or block inputs/outputs were observed in that baseline. All
54 `--help` process launches together took **0.026051 s**. This strongly supports
CPU-bound readiness/schema analysis rather than process startup or physical disk
I/O as the primary bottleneck for these warm-cache local runs. It does not claim
cold-cache hosted I/O is identical.

The corrected four-worker run completed in **80.762953 s**, with
**311.049963 s user + 1.476107 s system = 312.526070 CPU s**, **386.967%**
effective CPU utilization and **264,290,304 bytes (252.05 MiB)** sampled peak
tree RSS; 54/54 checks passed, none missing. Versus the unchanged baseline this
is **3.858×** faster at **1.003×** child CPU cost. The corrected one-worker
reference passed 54/54 in **309.433053 s**, with **308.024646 s user +
1.323937 s system = 309.348583 CPU s**, **99.973%** CPU utilization and
**79,015,936 bytes (75.36 MiB)** sampled peak tree RSS.

| Metric | 1 worker | 2 workers | 4 workers |
|---|---:|---:|---:|
| Wall seconds | 309.433 | 155.995 | 80.763 |
| User CPU seconds | 308.025 | 310.421 | 311.050 |
| System CPU seconds | 1.324 | 1.336 | 1.476 |
| Child CPU seconds | 309.349 | 311.756 | 312.526 |
| Effective CPU utilization | 99.97% | 199.85% | 386.97% |
| Sampled peak process-tree RSS MiB | 75.36 | 134.54 | 252.05 |
| Production / successful checks | 54 / 54 | 54 / 54 | 54 / 54 |
| Missing checks | 0 | 0 | 0 |
| Output equivalence | Reference | 54/54 PASS | 54/54 PASS |
| Speedup over one-worker reference | 1.000× | 1.984× | 3.831× |
| Relative child CPU cost | 1.000× | 1.008× | 1.010× |

**Selected default: four workers.** Four is ~1.93× faster than two locally,
uses ~252 MiB sampled aggregate RSS, and adds only ~1% CPU work relative to the
one-worker reference. Two workers remain available for memory-constrained hosts.
These are single corrected runs per setting, not statistical confidence bounds.

The full qualification runs are functional gate evidence, not an additional
controlled speedup experiment: to shorten campaign turnaround they run in
separate four-CPU affinity sets (serial 8–11, optimized 12–15) with disjoint TMPDIRs.
The workspace regression concurrently uses CPUs 0–3. Only the sequentially run
service-check benchmark table above should be used for the claimed speedup.

## Equivalence and negative controls

Comparison is reproducible from retained manifests:

```bash
python3 scripts/check-task062-service-impact.py --compare \
  /absolute/serial/manifest.json /absolute/parallel/manifest.json
```

It revalidates complete campaigns against the frozen 54-row order and 30 identity
count, then compares identities, contracts, exact argv, exit status, complete
readiness reports and complete diagnostic streams. **Only argv's `--contract`
value is normalized**, because exclusive scratch ownership necessarily changes
that path. No report bytes, stderr, timestamps, binary paths or schema paths are
normalized. Durations, log paths and manifest worker count are measurement
metadata, not production output. Every successful comparison emits 54 attributed
`EQUIVALENT` lines and an exact completion count.

The corrected one-worker, two-worker and four-worker real production runs passed **all 54
comparisons**, byte-identical complete stdout and stderr, identical contract
content, exit statuses and argv except the isolated contract path. All three were also
compared directly with the unchanged original driver's captured commands and
streams: **54/54 identical** for each. These checks retain all 30 identities.

Fast entry point `python3 scripts/test-task062-ci-wrappers.py` retains its original
six checks and executes `scripts/test-task062-service-impact.py`. Controlled fake
CLI processes test the scheduler only; these are not real UCI readiness evidence.
Tests cover bounded active concurrency (1/2/4), delayed first worker, full fixture
order and isolation, serial equivalence, malformed/missing/duplicate/extra/wrong
results, changed commands/contracts/diagnostics, missing or malformed completion
logs, exit 23, SIGTERM, timeout, missing executable, NOT READY, truncated reports,
wrong backend, unequal counts, extra output, worker exceptions and invalid worker
settings. The scheduler suite also runs under `python3 -O` to demonstrate required
runtime validation does not depend on assertions.

## Validation manifest

| Gate | Outcome |
|---|---|
| Clean release production generator build | PASS after campaign TMPDIR retry |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Existing wrapper controls + scheduler tests | PASS, 6 existing controls + 13 scheduler tests |
| Scheduler controls under optimized Python | PASS, all 13 tests |
| `scripts/check-ci-split.sh` | PASS |
| `scripts/test-check-ci-split.sh` | PASS, 210 checks |
| Original driver, genuine pinned roots, 54 selections | PASS |
| Corrected one/two/four-worker benchmarks | PASS, 54/54 each |
| All 54 serial/parallel comparisons, both optimized modes | PASS, 54/54 each |
| Full inherited pinned one-worker qualification | PASS, 672 s (11m12s) local functional run |
| Full inherited pinned optimized qualification | PASS, 441 s (7m21s) local functional run |
| Task 062 Fast compiler/codec gates, GNAT required | PASS |
| `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` | PASS in private mount namespace; 1,438 passed across 179 libtest summaries, 8 inherited ignored; initial environmental failures retained |
| Final-head hosted Fast CI | HOSTED_CI_PENDING |
| Final-head hosted Deep CI | HOSTED_CI_PENDING |

Both pinned invocations have already passed the exact registered Rust
inventory/coverage/impact test and all four release/world markers: serial
264.45 s, optimized-campaign invocation 264.90 s (the inventory itself is not
parallelized). Optimized qualification also passed all six actual generated-code
compiler verticals (Ada/Rust/C++ for 2.5 and 2.6), both Rust codec executions,
the 54-service marker and final `TASK062 PINNED GATES: PASSED`. Serial qualification
also passed the same six compiler verticals, both Rust codec runs and both final
markers, completing in 672 seconds. The local full-wrapper times (11m12s versus
7m21s) are not a controlled same-affinity benchmark; the service-check table is.
The eight ignored workspace tests retain their inherited status and are not
claimed as executed; genuine Task 062 pinned evidence was executed separately.

The broad workspace run initially failed on unchanged
`json_cli_failures_and_writer_classification` at hard-coded `/tmp/task085` with
`StorageFull` (inode exhaustion). A first bubblewrap retry could not spawn GNAT
under its default namespace setup; it was not counted as passing. The unchanged
suite passed with a private mount namespace and home-backed `/tmp`:

```bash
AMS_GRA_REQUIRE_GNAT=1 TMPDIR=/home/zboll/perf-003-evidence/regression-tmp \
  CARGO_BUILD_JOBS=4 unshare --user --map-root-user --mount --fork bash -c \
  'mount --bind /home/zboll/perf-003-evidence/private-tmp /tmp && taskset -c 0-3 cargo test --workspace'
```

This changes filesystem isolation only; tests and GNAT requirements are unchanged.

## Remaining bottlenecks and deferred vertical opportunity

The pinned Rust inventory/coverage campaign is untouched. Compiler verticals are
also deliberately unchanged: release-specific generated directories and YAML
inputs are separate, but Rust verticals currently share `scratch/cargo-target`.
Safe concurrency would need explicitly separated Cargo target ownership and
additional compiler/codec evidence; it is not part of this primary, independently
reviewable scheduler optimization. Ada and C++ compiler flags and executions,
both Rust codec round trips, and all original success markers are preserved.

Hosted speedup and sub-15-minute full qualification remain **targets**, not
guarantees. Local affinity measurements must not be substituted for final-head
hosted checks or described as hosted observations.