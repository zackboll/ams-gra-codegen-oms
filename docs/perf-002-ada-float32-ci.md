# PERF-002 — bounded helper-demand qualification

## Scope and baseline

Work is isolated in `/home/zboll/git/ams-gra-codegen-oms-perf-002`, branch
`feature/perf-002-ada-float32-parallel`. Latest fetched main at start:

- Commit: `23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39`
- Tree: `61f9e921e89133bd43b65071186567e3f0f9d5bf`

No existing PERF-002 branch/PR was found. The PERF-001 checkout and PR #105
were not modified, cherry-picked, merged, or used as the implementation base.
No workflow, generator, provider, fixture, numeric oracle, or index changes.

Historical reference is hosted run
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39538698494>,
head `93d02dba9bdc398453a29b5dd3b9956ae26793fb`, conclusion `success`.
GitHub API step timestamps give:

| Historical hosted operation | Seconds | Minutes |
|---|---:|---:|
| Entire Float32 job | 5,160 | 86.00 |
| Task 095 compiler/profile campaign | 1,192 | 19.87 |
| Task 093 mixed Binary/Float32 | 466 | 7.77 |
| Float32 helper-demand, GNAT 14 and 16 | 2,296 | 38.27 |
| Portable helper-demand, GNAT 14 | 949 | 15.82 |
| Both helper-demand steps | 3,245 | 54.08 |
| Evidence upload | 193 | 3.22 |

These are historical hosted timings, not same-head serial/parallel benchmarks.

## Inventory (unchanged)

Each campaign executes OAM and qualified profiles, in that order, with names:

1. `ArraySample`
2. `Array_Encode_Hex_Binary`
3. `Encode_Hex_Binary`
4. `Array_encode_hex_binary`
5. `Array_Decode_Hex_Binary`

Each naming case compiles and executes `normal`, `Check`, `Ignore`, in that
order: 30 positive cells per campaign. The Float32 job retains GNAT 14.2.0
Float32, GNAT 16.1.0 Float32, and GNAT 14.2.0 portable: **90 positive cells**.
The other existing portable GNAT 16 campaign automatically benefits too.
Each Float32 campaign retains 20 negative CLI invocations (check and atomic
generation rejection for all ten naming cases), 40 across both compilers.
Model/API byte comparisons, namespace reversibility, helper declaration counts,
runtime representation checks, portable opted-runtime byte parity, lockfile
version/archive integrity and per-policy compiler identity checks are retained.

## Execution and isolation

`AMS_GRA_TASK093_WORKERS` defaults to `2`; accepted values are exactly `1`,
`2`, `3`, `4`. Invalid values, including zero, whitespace, empty strings and
unbounded requests, fail before scratch creation. Duplicate profile/policy
arguments also fail. One worker is the serial reference for the new scheduler.

A bounded standard-library thread executor launches **separate Python
processes**, one per profile/name. Thread tasks only launch/wait/read their own
worker files; the parent alone mutates the result/failure lists. Each worker
imports its own `temporal` module, so `temporal.QUALIFIED` cannot race.
Each selection has private source, plain-generation comparison, support copy,
Alire sandbox/dependencies, object/bin directories, ADC files, logs, result,
resource record, and source manifest beneath its own scratch subtree. Policies
remain sequential inside a worker. Independently executable copies of each
positive policy consumer are retained with binary SHA-256 manifests.
Ephemeral settings/cache/toolchain links use hidden `.alire-seed` and
`.<profile>-<name>-alire` directories: upload-artifact@v4 follows directory
symlinks but excludes hidden directories by default. This prevents repeated
compiler payload uploads without changing workflow paths or suppressing any
visible qualification source/log/result evidence. A regression test guards it.

### Alire shared state

Alire 2.1.1 source inspection found that `Alire.Toolchains.Available` removes
`toolchains/must_reload` and deployment creates that flag. Separate build roots
alone would therefore not establish complete isolation.

The parent discovers configured paths using `alr version`, snapshots the
existing index without its Git metadata, disables automatic updates/community
addition, and uses private `-s` settings. A serial exact-version preflight
deploys any missing toolchain into the campaign seed, never global settings.
Each case copies index/config/cache state into its own private directory.
Only already deployed compiler/gprbuild **payload directories** are symlinked;
the containing toolchains directory and `must_reload` are private. Installed
seed payloads are also linked rather than copied. Alire's deployment skips an
already installed release; workers select only the preflighted exact releases.
Dependencies remain `dependencies.shared=false`, including each worker's
GNATCOLL/minimal build and pinned provider. No concurrent worker writes a
shared dependency/object/bin/settings/index/toolchain-metadata directory.

References inspected:
`https://github.com/alire-project/alire/blob/v2.1.1/src/alire/alire-toolchains.adb`
(`Available`, `Deploy`, `Path`) and `alire-cache.adb` (`Path`).

### Fixed build budget and forced policies

Alire normally passes `gprbuild -j0`, which is unbounded with respect to
higher-level workers. An explicit final `-jN` limits build jobs:

| Case workers | gprbuild jobs per case | Maximum compiler jobs |
|---:|---:|---:|
| 1 | 4 | 4 |
| 2 (default) | 2 | 4 |
| 3 | 1 | 3 |
| 4 | 1 | 4 |

Every original `alr -n build -- -f` remains a forced build. No incremental
reuse claim is made, and no policy compilation is skipped. Provider source
copies and policy-independent generated Ada remain unchanged. Builds execute
under the original exact toolchain identities and assertion-policy files.

## Completion and deterministic aggregation

The parent waits for **all** submitted workers, including after a failure, to
retain diagnostics for already executed and queued selections. Nonzero exit,
missing/invalid result, unexpected/duplicate identity, missing policy,
incomplete completion flag, wrong outcome, corrupt provenance, missing forced
build evidence, missing ADC, wrong lock/compiler or runtime evidence all fail
the campaign. The failed selection and worker log path are printed.

Results are flattened in original profile/name/policy order, independent of
completion order. The top-level `result.json` keeps exactly `completed`, `red`,
and `records`, with unchanged record fields and original completion marker.
Publication remains atomic and occurs only after source hashing and
verification. Existing recursive artifact paths retain worker evidence;
workflow files and upload paths are unchanged.

## Measurement method and limitations

`checked()` records monotonic elapsed time, aggregate child CPU seconds
(`wait4`, including compiler descendants), and kernel per-command max RSS.
`resources.json` records each case/campaign wall time, aggregate CPU seconds,
sampled peak **sum of process-tree RSS**, and explicit support/snapshot-copy
timings. Tree RSS is sampled every 200 ms; shared pages count per process and
short-lived peaks can be missed. It is not unique physical memory. Per-command
max RSS is not aggregate concurrently resident memory. Hashing and verification
wall time is in `provenance-timing.txt`; outer benchmark measurements include
all orchestration, copying, hashing, finalization, and subprocess work.

The unchanged original script can be passed to the new benchmark harness. It
retains Alire's original `-j0`, constrained by the **same four-CPU affinity**
as optimized runs. This is distinct from new one-worker/four-build-job mode.
Every benchmark invokes complete campaigns, validates provenance, then checks
logical equivalence before publishing a completed matrix summary.
`report-task093-helper-demand.py` aggregates completed measurement rows and
per-operation command costs; it does not mark an unfinished matrix complete.
Original script SHA-256:
`ba04c69a6b645433cc2521d3e86ee889f788ed7c736a77648bc1d5250b5429f5`.
Exploratory measured helper-demand implementation SHA-256:
`4f670f5d85417cda60d77b50023a064b10b09b187d89de897c761c0943f6b8f3`;
resource module:
`7a19b2a934f3e2fca67b86b490038fac31c9dad1b201e7765376b3687720bd72`.
Final artifact-safe implementation SHA-256:
`0b590ed20fc55b2dc249720b5903af3e490ff03221dc3a00dc98ccebbd779eaf`.
Each worker and top-level source manifest records these identities; the
benchmark uses the same exact-main generator binary throughout.

Local machine: AMD Ryzen 9 7945HX, 16 cores/32 logical CPUs, approximately
86 GiB RAM; Debian libc `2.41-12+deb13u4`, GNAT 14 `14.2.0-19`, exact Alire
2.1.1 and GNAT native 16.1.0. libc binary SHA-256:
`9792e3cbb541c8f44c7acf5f14f4022ea62998ecc787d326bed4d8b6547dfd92`.
This matches the qualified libc identity, but is **not** a hosted runner or a
fresh execution inside the pinned Debian container. Local concurrent unrelated
work and SMT siblings can affect timings; CPU affinity is not a physical-core
reservation. Do not extrapolate local measurements into hosted speedups.

An exploratory two-worker/-j1 smoke run showed ~40 CPU seconds per forced
policy build and negligible generated-consumer execution time. Its source
manifest correctly failed when implementation files changed while it ran;
that development run is **not completed qualification or benchmark evidence**.
This motivated the fixed four-job budget rather than two workers each using
only one build job. Compilation is predominantly CPU-bound; selection includes
I/O/dependency resolution; copying/hashing are filesystem/CPU work. Compilation
and linking currently share an Alire build measurement; separate compiler/link
phase timings and strict CPU-vs-I/O attribution remain a measurement limitation.

### Completed measurements

The final fixed-source serial/two/four-worker matrix is in progress. The rows
below completed before the artifact-safe hidden-directory adjustment and are
**development measurements, not final-head qualification**. The incomplete
development matrix was stopped; logs were preserved, not relabeled as complete.
Fresh final-source matrices repeat original and all worker modes:

| Local campaign / mode | Wall s | CPU s | Sampled tree RSS KiB | Positive / negative CLI |
|---|---:|---:|---:|---:|
| Portable 14 original | 480.539 | 1,556.740 | 2,863,268 | 30 / 0 |
| Portable 14 new one worker | 429.281 | 1,346.304 | 1,000,780 | 30 / 0 |
| Portable 14 two workers | 391.836 | 1,426.008 | 1,145,640 | 30 / 0 |
| Float32 14 original | 602.325 | 1,827.345 | 2,834,876 | 30 / 20 |
| Float32 14 new one worker | 560.188 | 1,801.794 | 1,000,968 | 30 / 20 |
| Float32 16 original | 724.241 | 2,380.138 | 2,922,288 | 30 / 20 |

Portable original versus new serial/two-worker equivalence passed all **351** evidence
items; complete ordered `result.json` and generated/provider source bytes match.
The small serial difference (480.539 / 429.281 = 1.119×) is not evidence for
the parallelization goal: explicit build-job bounding and local contention vary.
Original/two-worker wall speedup is 480.539 / 391.836 = **1.226×** for this
completed local portable campaign, below the 2× target; serial/two-worker is
429.281 / 391.836 = 1.096×. Two workers use 8.4% less CPU than original but
5.9% more CPU than new serial. Four-worker and Float32 results must finish;
this is not hosted evidence. All 30 archived serial policy consumers were also
independently re-executed with exact markers and their binary hashes verified.
Float32 GNAT 14 original/new-serial equivalence passed all **321** evidence
items, including all 20 negative CLI controls and complete three-policy coverage.

| Metric (all three required campaigns) | Original | One worker | Two workers | Four workers |
|---|---|---|---|---|
| Wall seconds | pending full matrix | pending | pending | pending |
| CPU seconds | pending | pending | pending | pending |
| Sampled peak tree RSS | pending | pending | pending | pending |
| Positive cases | pending 90 | pending 90 | pending 90 | pending 90 |
| Negative CLI cases | pending 40 | pending 40 | pending 40 | pending 40 |
| Missing cases | not yet established | not yet established | not yet established | not yet established |
| Compiler/policy coverage | pending | pending | pending | pending |

No full-matrix/hosted speedup, 2× target, or below-45-minute hosted job is claimed.

### Final artifact-safe source: completed default-mode qualification

All three final-source two-worker campaigns completed and independently matched
their fresh unchanged-original baselines (351 portable and 321 per Float32
campaign comparison items, **993 total**). Ordered result JSON, all generated
source bytes, policies, locks, runtime markers and 40 negative CLI diagnostics
match. Original and optimized totals each contain **90 positive cells**, zero
missing/duplicate cells, complete policies, and **40 negative CLI cases**.

| Final local campaign | Original wall s | Two-worker wall s | Original CPU s | Two-worker CPU s | Original RSS KiB | Two-worker RSS KiB |
|---|---:|---:|---:|---:|---:|---:|
| Portable GNAT 14 | 576.005 | 583.396 | 1,836.710 | 1,842.086 | 2,955,708 | 1,096,852 |
| Float32 GNAT 14 | 656.384 | 462.695 | 1,981.120 | 1,655.449 | 2,857,052 | 1,024,816 |
| Float32 GNAT 16 | 746.837 | 526.176 | 2,481.366 | 1,938.818 | 2,888,568 | 1,197,556 |
| Summed campaign wall/CPU (RSS is maximum) | 1,979.225 | 1,572.267 | 6,299.196 | 5,436.353 | 2,955,708 | 1,197,556 |

Wall ratios: portable **0.987×**, Float32 14 **1.419×**, Float32 16 **1.419×**;
combined helper-demand ratio **1.259×**. Total CPU decreased **13.7%** and
sampled peak tree RSS decreased **59.5%**. These are completed local campaign
measurements, **not** a completed one/two/four-worker matrix or hosted evidence.
The initial final-source two-worker campaigns ran on separate four-logical-CPU
affinities concurrently with baseline/matrix campaigns, some on SMT sibling
cores. That is a contention confounder, not a valid isolated runner prediction;
the staged one/two/four-worker matrices must finish for comparable mode data.
Portable does not show a speedup in this loaded final-source comparison. No
2× goal is claimed. Outer baseline includes finalization; optimized resources
exclude the last 2.4–2.9 s top-level hash/verify finalization (recorded separately).

Final-source OAM intentional-red GNAT 16 also completed all three policies,
with byte-identical result JSON and seven matching compiler-error lines per
policy versus original; both source manifests verify. The pre-existing
qualified-profile literal-diagnostic limitation remains documented below.

Final two-worker per-operation totals (summed case wall times overlap):

| Operation | Portable14 wall/CPU s | Float32-14 wall/CPU s | Float32-16 wall/CPU s |
|---|---:|---:|---:|
| 30 forced compilation/link invocations | 1,079.335 / 1,807.206 | 854.310 / 1,625.899 | 984.448 / 1,911.433 |
| Alire setup/selection/identity (73 commands) | 65.608 / 18.156 | 55.917 / 16.014 | 59.345 / 14.232 |
| Generator/runtime identity controls | 0.621 / 0.582 | 0.505 / 0.490 | 0.469 / 0.459 |
| Generated execution (30 commands) | 0.091 / 0.073 | 0.084 / 0.077 | 0.033 / 0.029 |

Build CPU/wall >1 is expected with two compiler jobs per worker. Selection
wall substantially exceeds CPU (solver/download/filesystem/wait behavior);
generated execution is negligible. The difference between campaign aggregate
CPU and summed commands includes Python, copying and provenance work; it is
not pure Python overhead and cannot be attributed more finely without profiling.
`-f` forces every build, so a no-force counterfactual is not benchmarked or
claimed safe; repeat-policy compilation dominates cost rather than execution.

## Equivalence

`compare-task093-helper-demand.py` verifies source manifests, full ordered
identity/policy inventories, completion JSON, generated Ada/model/consumer/
provider/C sources, schema/input JSON, parsed complete Alire lock identities,
compiler banners, expected/actual build outcomes, exact execution markers,
negative CLI logs and intentional-red diagnostics. Missing or different items
fail, not just unequal counts. Deterministic source bytes are not normalized.

The documented normalizations/exclusions are:

1. Original flat layout versus private worker subtree: map by profile/name.
2. Absolute case scratch paths in parsed lock strings and rejection/diagnostic
   text: replace only the exact case root with `<CASE>`. Relative pins, versions,
   provider archive hashes and external compiler identity remain compared.
3. Compiler identity: compare full GNAT banner, not deployment executable path.
4. Execution timestamps, elapsed/CPU/RSS values, command working/scratch paths,
   progress output and build log scheduling order are nondeterministic metadata,
   not byte-equality targets. Compare expected/actual exits and runtime/rejection
   evidence separately. Script hashes necessarily differ for original versus
   implementation; every individual manifest must nevertheless verify.
5. Generated Alire configuration paths/object files/executable bytes are not
   deterministic source outputs. Policy executables are retained but are not
   expected byte-identical across independent build roots.

No provenance paths or timestamps are silently ignored in manifest verification.

## Negative controls and local gates

The existing `test-task093-compiler-placement.py` CI entry point now also loads
stdlib scheduler tests: bounded settings, deterministic aggregation, missing/
duplicate/unexpected identity, malformed/incomplete result, missing/wrong policy,
wrong compiler/outcome, real subprocess exit 7, missing/malformed result files,
retained diagnostics and draining all selections, private Alire sentinels,
provenance corruption, and source/marker/lock/outcome comparator mutation tests.
Mocks in scheduler tests are explicitly not compiler qualification evidence.
The combined entry point passes **23 tests** (11 existing wrapper/placement
controls plus 12 scheduler/equivalence/resource/artifact controls).

Completed local gates:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace --all-targets`:
  **1,437 passed, zero failed, eight ignored**, 165 summaries; ignored is not passed.
- CI split guard/mutation controls, Task 077B layout/mutations (both 54-case
  profiles), Task 087A wrapper controls.
- Task 095 independent C rounding/bridge/environment campaign.
- Unmodified mixed Binary/Float32 GNAT 14 and GNAT 16 campaigns; all six
  profile/policy cells each, provenance verified.
- All **ten** unmodified Task 095 invocations from the hosted job: GNAT 14/16
  × OAM/qualified numeric/environment/rounding and adversarial identifiers,
  plus GNAT 14/16 OAM optimized/environment/rounding. Every invocation's three
  policies completed; all ten manifests verified.
- Portable GNAT 16 default two-worker campaign: all 30 positive cells and
  manifest verification passed (additional existing portable job coverage).

Intentional-red generator is built independently from reviewed broken revision
`168c8296fa219097e8d1502b0ccdad1b3bbf09cc`; production source is not changed.
OAM portable GNAT 16 intentional-red executes all three policies successfully
under both unchanged original and optimized runners. Results are identical;
all seven compiler-error lines per policy match exactly and both manifests
verify. This explicit three-cell subset is not the full positive matrix.
Running the original default `--red` selection exposes a pre-existing limitation:
the literal expected diagnostic names `Oam`, while qualified actual compiler
diagnostics name `Task093`. The optimized campaign correctly fails closed for
that selection, preserves its logs and does not publish `result.json`. The
expectation is intentionally not relaxed. Full qualified-profile intentional-red
success is therefore **not claimed**; treat this as a qualification blocker if
required, not as a passed test.

## Reproduction

From `/home/zboll/git/ams-gra-codegen-oms-perf-002` (use absolute paths for
the generator and scratch in other environments):

```sh
python3 scripts/test-task093-compiler-placement.py
AMS_GRA_TASK093_WORKERS=1 python3 scripts/check-task093-helper-demand.py \
  --family float32 --compiler 14 \
  --generator /home/zboll/git/ams-gra-codegen-oms-perf-002/target/debug/ams-gra-codegen-oms \
  --scratch /tmp/perf002-serial-float32-14
# Repeat with workers=2 and workers=4 and fresh scratch, both compiler versions,
# and portable/compiler=14. Never reuse scratch or edit source during a run.
python3 scripts/compare-task093-helper-demand.py \
  /tmp/perf002-serial-float32-14 /tmp/perf002-parallel-float32-14
python3 scripts/benchmark-task093-helper-demand.py \
  --generator /home/zboll/git/ams-gra-codegen-oms-perf-002/target/debug/ams-gra-codegen-oms \
  --scratch /tmp/perf002-complete-matrix --cpus 0,1,2,3
```

## Follow-on candidates / hosted disposition

Task 095 invocations are logically independent, but its provider copies,
Alire configuration and `-j0` must receive the same isolation/resource review;
parallelizing the workflow shell loop would violate the no-workflow-edit
preference. Task 093 mixed GNAT 14/16 campaigns also have independent scratch
roots but need isolated Alire state and a global four-job budget. Both are
**PERF-003 candidates**, not implemented or represented as qualified speedups.
No compiler cache, build reuse, generated-code or provider optimization.

Disposition at submission: **HOSTED_CI_PENDING**. Starting/latest fetched main
still agrees with the recorded SHA/tree. No workflow files changed. The final
two-worker 90-cell/40-negative qualification and all 993 original-equivalence
comparisons passed; the staged full one/two/four-worker benchmark matrix is
still running and cannot be reported complete. The 2× goal is not achieved by
the completed local comparisons. The literal qualified intentional-red mismatch
is a pre-existing limitation, not a passed control or a relaxed expectation.

Hosted PR URL, final SHA/tree, exact-head check conclusions, complete performance
matrix and overall optimized Float32 job runtime are pending. No PR is merged
and auto-merge is not enabled. **Not ready for independent review qualification**
until all mandatory exact-head checks and equivalence/benchmarks finish.