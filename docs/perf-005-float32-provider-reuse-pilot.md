# PERF-005 — Float32 GNATCOLL attribution and opt-in reuse pilot

## Scope and disposition

**PROVIDER_REUSE_PILOT_INCOMPLETE (R1)**: complete consumer compilation,
execution and isolation controls pass, but strict provider effective-input
equivalence is rejected for GNAT mapping inputs. Controlled repeatable
performance is also unavailable. The R1 reconciliation below supersedes earlier
claims that only the performance gate remained.
This experiment is not production rollout, qualification of the complete
132-build job, or an assessment that PR #106 is ready to merge. Ordinary
qualification scripts, workflow allocation, generator, providers, algorithms,
corpora, expected diagnostics and numerical checks are unchanged.

The explicit entry point dynamically loads the existing helper-demand runner
and adapts its `checked` boundary; it does not copy its naming, generation,
negative-CLI, execution or numerical-check implementation. The qualification
change is deliberate: construct GNATCOLL freshly once for each effective
configuration; continue forcing generated and repository-owned support
compilation for **every** cell, followed by binding, linking and execution.

## Source and environment

- Refreshed `origin/main` starting commit:
  `401d336ecfe88992e5afabf44e8833986e398cc4`.
- Starting tree: `88d5cef07013137404f947cfd6bf35a858329c28`.
- Independent worktree: `/home/zboll/git/ams-gra-codegen-oms-perf-005`.
- Branch: `feature/perf-005-float32-provider-reuse-pilot`.
- Raw evidence: `/home/zboll/perf-005-evidence` (not committed).
- Final source identity: the PR head; `tooling.sha256` records experiment tool
  identities. The production source remains the starting commit.
- Production generator built using Rust 1.95.0, `--locked`, from this worktree:
  SHA-256 `a6797b3ea64e5bca24960fa2e20aab7d8b9fdb2021ae11804e234181cc3c3760`.
- Local qualified platform image:
  `3be2d67b84bc116c44fa08d25714359dfd5aff95fddc3c8a270dd359633e03ab`;
  x86_64, libc `2.41-12+deb13u4`, GNAT 14 package `14.2.0-19`.
  This is the existing local Task095 platform, not a claim to have recreated
  GitHub's entire pinned image/provisioning history.
- libc SHA-256:
  `9792e3cbb541c8f44c7acf5f14f4022ea62998ecc787d326bed4d8b6547dfd92`.
- Alire 2.1.1 binary SHA-256:
  `2e2b8de009b793ad2aed10096e162649bbcc5b0711e3c21785ebe8a1261c5d64`.
  The workflow-pinned release zip SHA-256
  `09c66bcd8c35dd4b97b72c3d9b76e44caa6964a2db35aba069f396f00f1f64c7`
  was downloaded, verified and its extracted binary matched exactly.
- GPRbuild crate 26.0.1; actual banner **GPRBUILD 26.0.0 (2026-04-15)**,
  binary SHA-256 `aff9fde5ca766d5497da9dec185cb8b96878206a0d02512aa5685e1e56d0a4d3`.
- Private index seed commit: `4635347cb4460e87ea70d6599aeff1eb615bd472`.
- strace 6.13, GNU time 1.9, Python 3.13, Linux 6.12.113.

PR #106's published report and R1 checkpoint were read as background and retained
in evidence. No code was imported; its worktree and unfinished corrections were
not modified. Other active worktrees/PRs were not changed.

Before execution the repository filesystem had approximately 501 GiB free and
114 million free inodes. Host `/tmp` had **zero free inodes**. Task temporary
files use `/home/zboll/perf-005-evidence/tmp`; the task-owned container has a
fresh 4-GiB `/tmp` tmpfs. Nothing belonging to other tasks was cleaned.

### Resource arrangement and limitations

One sequential campaign at a time, existing Alire `-j0`, affinity CPUs
**4,6,8,10**, on a Ryzen 9 7945HX (16 cores / 32 threads). SMT siblings are
20,22,24,26. Rootless Podman could not apply a cgroup cpuset; `taskset` inside
the container enforces the four-CPU allocation instead. No cgroup CPU or memory
quota was imposed. This is **not an exclusive physical-core reservation**.
Other host activity was present. Local timings must not be promoted to a
qualified hosted-runner speedup.

Both sides use fresh consumer/provider output, copied private settings, index,
toolchain metadata and HOME. GNAT 16 and GPRbuild toolchain installations are
preseeded identically (compiler binaries are inputs, not prebuilt GNATCOLL).
Provider archives are resolved/deployed by the unchanged Alire flow; no provider
objects are seeded. The exact provider zip is separately retained and SHA-512
verified. Network/filesystem page-cache effects are not controlled cold-machine
effects: “cold” here means **fresh build outputs**, not rebooted OS caches.
Candidate totals include its three fresh provider builds, validation, hashing,
all orchestration and runner evidence finalization.

## Verified inventory and applicability

The current Float32 job still has **132 positive forced-build invocations**:

| Campaign | Expansion | Positive builds |
|---|---|---:|
| Helper-demand Float32 GNAT14/GNAT16, portable GNAT14 | 3 × 2 profiles × 5 names × 3 policies | 90 |
| Task095 numeric/environment/rounding + identifiers | 2 compilers × 2 profiles × 2 campaigns × 3 policies | 24 |
| Task095 optimized OAM | 2 compilers × 3 policies | 6 |
| Mixed Binary/Float32 | 2 compilers × 2 profiles × 3 policies | 12 |

Portable GNAT16 is another job's 30 cells, not part of 132. The focused pilot is
only two complete Float32 helper-demand campaigns: **60 positive cells and 40
negative CLI cases**. Each campaign is ordered OAM then qualified; names are
`ArraySample`, `Array_Encode_Hex_Binary`, `Encode_Hex_Binary`,
`Array_encode_hex_binary`, `Array_Decode_Hex_Binary`; policies are normal,
Check, Ignore. There are two portable rejection invocations for each name/profile.

The historical published-head hosted job
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39710977367/job/119601977281>
was 84:44: Task095 20:22, mixed 7:49, Float32 helpers 36:38, portable helpers
15:17, upload 3:23. API step timestamps were independently retrieved. These
times are context only, **not this experiment's baseline**.

Portable imports the Core provider through the ordinary support project but
not Float32/C support. Mixed adds consumer-side C harnesses and multiple mains.
Task095's `--optimized` changes consumer Ada `-O0` to `-O2`; provider projects
have their own switches. These labels are not sufficient cache keys. Extension
to those matrices requires checking their actual effective commands; none of
them is qualified by this pilot.

## Attribution method

`perf005_profile.py` uses strace `execve` to observe actual commands, including
absolute compiler/backend paths chosen through Alire. It handles unfinished /
resumed exec records and records exit status and inclusive lifetime. Inclusive
lifetimes are **not summed as campaign wall time**. Individual command CPU is
sampled, not exact; the detailed records retain sampled process identities and
CPU observations. The sample interval is 20 ms.

Only `/proc/PID/stat` **utime + stime** are attributed, never cumulative
cutime/cstime. Each observed process tick delta has one phase. Source paths and
working-directory ownership distinguish Minimal, Core, repository support
(including C), generated/consumer, archive and bind/link. Alire self CPU and
unclassified CPU remain separate. Short processes can vanish between samples;
the difference from outer `RUSAGE_CHILDREN` CPU is explicitly residual, not
silently redistributed. Trace/sampler overhead is separate from clean timing.
The outer GNU time CPU also includes Python orchestration/sampling self CPU.

Controlled tests verify that a waiting parent does not duplicate a CPU-burning
child's CPU and that an absolute-path real compiler failure is observed and
propagated. Detailed subsequent traces retain GPR temporary configuration and
response-file contents. Initial representative traces predated that retention;
their explicit policy argv, source identities and project defaults are retained.

### Initial real representative builds (three policies, first OAM naming case)

| CPU seconds, sampled unless marked outer | GNAT14 | GNAT16 |
|---|---:|---:|
| GNATCOLL Minimal compilation | 5.17 | 6.76 |
| GNATCOLL Core compilation | 111.21 | 146.21 |
| Repository JSON/Float32/C compilation | 0.94 | 1.54 |
| Generated model/codec/consumer compilation | 0.38 | 0.69 |
| All archiving | 0.33 | 0.17 |
| Bind/link | 0.15 | 0.43 |
| Alire self/setup within builds | 0.60 | 0.63 |
| Unclassified sampled CPU | 5.69 | 5.83 |
| Unobserved residual CPU | 7.616 | 7.448 |
| **Outer build CPU** | **132.086** | **169.708** |
| Sequential outer build elapsed | 42.059 | 52.452 |
| Non-build child CPU (resolution/deployment/execution etc.) | 2.978 | 2.928 |
| Non-build elapsed, including finalization | 6.324 | 5.916 |
| Complete representative child CPU | 135.064 | 172.636 |
| Python self CPU (includes sampler) | 3.500 | 4.420 |
| Complete representative elapsed | 48.384 | 58.368 |

Provider compilation alone is **88.11% / 90.14% of outer build CPU** (not of
hosted job elapsed). Residuals are 5.77% / 4.39%. Both compilers execute 21
Minimal and 85 Core compilation-driver invocations per construction, including
provider C; four generated, four support, one binder compilation per cell.
Archiver/index commands are separately visible. No provider assembly is selected
with the actual generic hash implementations.

This material provider share supports trying reuse rather than another worker
matrix. GNAT16's provider CPU is materially higher; do not pool compiler results.

## Provider configuration and consumption

Both resolved crates are 26.0.0 from the exact existing zip, SHA-512:

```
cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b
```

Resolved Core/Minimal project bytes match upstream commit
`8e270f83719d4284782a965edd80246be15b949b`. Their `EXTERNALLY_BUILT` and
`GNATCOLL_OBJECT_ROOT` attributes are verified against the **resolved** projects.
Alire 2.1.1 source and actual traces both confirm `-s -j0 -p` already applies.

Actual defaults: PROD, static, unix, version 26.0.0, mmap/madvise yes,
Blake3 generic, XXHash generic; provider Ada `-O2 -gnatn -gnatwaCJ`, provider C
`-O2 -Wunreachable-code` (plus the upstream unit-specific flags). Generated and
support Ada stay `-gnat2022 -gnato -O0`; Float32 C stays
`-std=c11 -O2 -Wall -Wextra -Werror`.

Each variant identity records compiler/target/runtime banners and executable
hashes, runtime `system.ads` and `libgnat.a` hashes, relevant environment,
provider source/project/constant file hashes, exact policy contents, Alire and
GPRbuild versions. Normal has no added ADC; Check and Ignore contain exactly
`pragma Assertion_Policy (Check);` and `pragma Assertion_Policy (Ignore);` plus
newline. The actual provider invocations contain their respective ADC. Compiler
versions, sources, target, options, externals, ABI/library type and policy must
not be conflated merely because generated code executes successfully.

Preparation uses the resolved Core project, matching Alire environment and
`-p -f -j0`, with a fresh `GNATCOLL_OBJECT_ROOT` and
`EXTERNALLY_BUILT=false` for each policy. Completed artifacts are hashed and
published with `complete.json`, then made read-only. Consumers pass
`EXTERNALLY_BUILT=true`, the matching object root, and retain `-f` and the
original per-cell ADC. Every consumption checks identities and artifact hashes
before and after; provenance connects every cell to its completed manifest.

Landlock ABI >=3 additionally denies writes outside private consumer/support,
Alire-state, HOME, `/tmp` and `/dev` roots. The shared providers are readable but
not writable by consumers, even if an owning process chmods a file writable.
This Linux-specific experimental isolation is a tradeoff, not a new production
requirement. A missing sandbox is a failure, never a silent fallback.

Intentional path mappings for equivalence are only campaign-root and private
state-root substitutions. Relative pins, locks, policy contents, source bytes,
banners, ordered outcomes and runtime markers must otherwise agree.

## Reproduction

Use the qualified x86_64/glibc2.41 environment, verified Alire 2.1.1 and exact
toolchain-only seeds. All paths below are absolute; substitute a fresh task-owned
evidence root. Set `TMPDIR` on the host to a filesystem with free inodes. Inside
the qualified container, `/tmp` must be private and have sufficient inodes.

```sh
R=/home/zboll/git/ams-gra-codegen-oms-perf-005
E=/home/zboll/perf-005-evidence
export PYTHONDONTWRITEBYTECODE=1

python3 "$R/scripts/perf005-state.py" \
  --index "$E/seed/settings/indexes/community" \
  --toolchains "$E/seed/cache/toolchains" --output "$E/new-baseline14-state"
# Repeat with --output "$E/new-candidate14-state" (never share mutable state).

HOME="$E/new-baseline14-state/home" PATH="$E/new-baseline14-state/bin:$PATH" \
taskset -c 4,6,8,10 /usr/bin/time -v -o "$E/new-baseline14-time.txt" \
python3 "$R/scripts/perf005-float32.py" -- --compiler 14 --family float32 \
  --scratch "$E/new-baseline14" --generator "$E/generator-target/debug/ams-gra-codegen-oms"

HOME="$E/new-candidate14-state/home" PATH="$E/new-candidate14-state/bin:$PATH" \
taskset -c 4,6,8,10 /usr/bin/time -v -o "$E/new-candidate14-time.txt" \
python3 "$R/scripts/perf005-float32.py" --reuse-provider -- \
  --compiler 14 --family float32 --scratch "$E/new-candidate14" \
  --generator "$E/generator-target/debug/ams-gra-codegen-oms"

python3 "$R/scripts/perf005-compare.py" "$E/new-baseline14" "$E/new-candidate14"
python3 "$R/scripts/perf005-report.py" "$E/new-candidate14"
```

Repeat for compiler 16 with fresh state/output directories. For separate command
coverage evidence repeat the candidate with `--trace` and pass that campaign to
comparison using `--traced`. `--trace --representative -- --profiles oam ...`
restricts only to the first name and is **not full qualification**. All production
runner selectors must follow the entry point's `--` delimiter.

Run controls separately, under the candidate's exact state/HOME:

```sh
python3 "$R/scripts/test_perf005_profile.py"
python3 "$R/scripts/test_perf005_provider.py"
HOME="$E/new-candidate14-state/home" PATH="$E/new-candidate14-state/bin:$PATH" \
taskset -c 4,6,8,10 python3 "$R/scripts/perf005-controls.py" \
  "$E/new-candidate14" "$E/new-controls14"
```

## Correctness, timings, remaining gates

The initial three-policy GNAT14 smoke passed, with no provider compilation in
consumer build logs. Nine disposable controls per compiler (generated Ada,
support Ada, support C × three policies) retained actual nonzero compiler
evidence. Nine accounting/provider/equivalence unit tests pass, including artifact corruption/missing/
incomplete publication, wrong source/compiler/policy identities, attempted wrong
variant, symlinks, absolute compiler observation and owner-write rejection.
The comparator rejects partial inventories rather than treating smoke evidence
as complete. Existing intentional-red expectations were not modified; the known
qualified-profile `Oam` diagnostic-harness limitation is not counted as passing.

Full comparison and fresh-output baseline/candidate observations are recorded
below. Runtime success is supplemented by actual compiler-command coverage.
The remaining qualification blocker is controlled, repeatable performance.

### Complete fresh-output timing observations (not controlled speedup evidence)

| Compiler / strategy | Whole elapsed s | Aggregate CPU s (child + Python self) | Largest-child peak RSS KiB |
|---|---:|---:|---:|
| GNAT14 baseline | 381.858 | 1192.752 | 277304 |
| GNAT14 pilot, including all provider preparation | 102.744 | 161.647 | 277284 |
| GNAT16 baseline | 480.263 | 1576.122 | 310860 |
| GNAT16 pilot, including all provider preparation | 113.808 | 213.170 | 301940 |

RSS here is kernel largest-child high-water RSS, **not simultaneous process-tree
Complete traces additionally sample peak sum-of-process RSS, which
double-counts shared pages and can miss short peaks. Neither is unique physical
memory. The nominal GNAT14/GNAT16 elapsed ratios are 3.717x / 4.220x, but there
is only one ordered local pair per compiler and host contention is not excluded.
These are not qualified speedups. No uncontrolled repetition was launched to
turn these observations into a performance qualification claim.

Both full comparators passed 30 positive cells and 20 negative CLI cases each:
ordered records, generated/API/codec/support bytes, provider source bytes,
runtime/compiler evidence, policy contents, normalized locks/diagnostics,
atomic rejection and exact seven-check runtime marker agree. All 30 clean
candidate logs per compiler contain eight project-owned source compilations (four generated,
four support including C), binding/linking, and no GNATCOLL compilation. Actual
executed per-cell compiler coverage is a separate traced gate, not inferred
solely from these logs.

| Per compiler campaign | Baseline | Pilot | Eliminated |
|---|---:|---:|---:|
| Fresh provider constructions | 30 | 3 | 27 (90%) |
| Minimal compile drivers | 630 | 63 | 567 |
| Core compile drivers | 2550 | 255 | 2295 |
| Provider library archives | 60 | 6 | 54 |
| Generated source compile drivers | 120 | 120 | 0 |
| Repository support compile drivers, including C | 120 | 120 | 0 |
| Binder compile drivers | 30 | 30 | 0 |
| Positive build/execution cells | 30 | 30 | 0 |
| Negative CLI cases | 20 | 20 | 0 |

Baseline full provider counts are from complete build logs, cross-checked
against the three actual-command representative traces (106 provider drivers
per build). Pilot actual preparation/consumer counts are a separate detailed
trace gate. Across both clean pilots the construction reduction is 60 to six,
not 132 to six; no full-job reduction is claimed.

### Reconciliation and instrumentation overhead

Clean runs deliberately have **no sampled phase estimates**. Their exact outer
child CPU partitions as follows (Python self CPU is separately in the timing
table):

| CPU seconds | GNAT14 baseline | GNAT14 pilot | GNAT16 baseline | GNAT16 pilot |
|---|---:|---:|---:|---:|
| Consumer forced-build subprocesses (includes providers in baseline) | 1177.539 | 23.562 | 1560.860 | 31.855 |
| Fresh provider preparation subprocesses | 0 | 115.070 | 0 | 157.939 |
| Other child CPU (setup/identity/execution) | 14.036 | 18.329 | 14.056 | 18.693 |
| **Outer child CPU** | **1191.576** | **156.961** | **1574.916** | **208.487** |

Non-build elapsed, including resolution/deployment, identity checks, executions,
provider verification, source hashing and evidence finalization, is 43.481 /
50.941 s (GNAT14 baseline/pilot) and 42.027 / 48.035 s (GNAT16). Individual
`*.profile.json` logs separate executable calls and setup calls. Remaining
Python orchestration/finalization wall is not incorrectly attributed to compiler
work. Source ownership, not overlapping parent/descendant CPU, drives tracing.

The full GNAT14 detailed pilot took 139.207 s versus 102.744 s clean (+35.5%),
so detailed tracing is emphatically **not used for the timing comparison**.
Its sampled phases were Minimal 5.10, Core 110.58, support 6.95, generated 2.36,
archive 0.33, bind/link 1.67, Alire 6.59, unclassified 20.90 CPU s. Residual
17.180 s reconciles to outer build CPU 171.660 s. Other child CPU 25.688 s
brings child total to 197.348 s; Python self CPU is 10.153 s. These traced
figures include instrumentation-induced work and are not clean performance.

Both compilers' detailed traces prove 120 generated, 120 support, 30 binder and 318
provider compilation drivers; all three provider command multisets match their
original representative policy build (106 source/option records each), after
only dependency-root, temporary response/config path and ADC-path mapping.
The matching manifest still requires exact ADC contents and source hashes.

The GNAT16 detailed pilot took 155.856 s versus 113.808 s clean (+36.9%).
Sampled CPU seconds: Minimal 6.89, Core 148.09, support 12.46, generated 5.44,
archive 0.19, bind/link 4.57, Alire 6.82, unclassified 24.04. Residual 16.830 s
reconciles to outer build CPU 225.330 s. Other child CPU 25.990 s gives 251.320 s;
Python self CPU adds 11.343 s. Both complete traced comparators passed, including
actual absolute compiler coverage and matching per-policy provider options.

Both final disposable compiler-failure sets passed **nine controls each**.
Missing/corrupt/unpublished/incomplete artifacts, wrong identities/variants and
consumer write attempts are rejected by the targeted unit controls. No runtime,
negative-CLI or intentional-failure expectation was relaxed.

Local validation also includes compiler-placement controls, CI split/layout and
layout mutation controls, syntax parsing, `git diff --check`, and Rust formatting.
No Rust source changed. An initial layout-check invocation omitted its required
arguments and failed; the correctly invoked layout and mutation checks passed.

### Recommendation

The provider share justifies the focused experiment. Do not wire it into
production until complete per-compiler equivalence, actual rebuild evidence,
negative controls and a controlled, repeatable end-to-end comparison exist.
The smallest next task is **one repeated same-head baseline/pilot pair for each
compiler on an exclusive four-CPU allocation**, with identical immutable seeds
and provider preparation included. If confirmed, extract only the validated
build boundary into an opt-in production adapter. Do not introduce persistent
caching or broaden the matrices as part of that first integration.

### Evidence index and local validation

Under `/home/zboll/perf-005-evidence`:

- `environment.txt`, `container-image.txt`, `container-preflight.txt`,
  `state-version-control.txt`, `gnat16-identity.txt`: environment/tool identities.
- `archives/`: checksum-verified provider and Alire archives;
  `upstream/`: independently fetched pinned provider projects.
- `attribution14`, `attribution16`: representative original three-policy
  commands, traces, sampled phases, locks, sources and execution evidence.
- `baseline14`, `baseline16`, `candidate14`, `candidate16`: complete clean
  campaigns; `*-time.txt` contains independent outer GNU time measurements.
- `traced14`, `traced16`: separate complete pilot traces, retained GPR temporary
  inputs, compile ownership/policy evidence and provider preparation commands.
- `equivalence14-traced.json`, `equivalence16-traced.json`: fail-closed complete
  comparison results; `*-summary.json`: classified accounting/reconciliation.
- `candidate14-controls`, `candidate16-controls`: nine actual compiler-failure
  controls per compiler, separate from positive/performance evidence.
- `unit-final.log`: nine profiler/provider/equivalence unit controls;
  `placement-final.log`: eleven existing compiler-placement controls;
  `ci-layout-mutations.log`: two tests exercising 54 mutations twice;
  `ci-split-final.log`, `fmt-final.log`: unchanged layout/split/format gates.
- `tooling.sha256`, `evidence.sha256`: final tooling and selected raw-evidence
  manifests. Full generated/provider source provenance remains in each runner's
  `sources.sha256`; external provider artifact provenance is explicitly added
  in `perf005-providers/*/complete.json` and `perf005-provider-consumers.json`.

Large source/object/toolchain trees are deliberately outside Git. Failed
disposable exploratory controls remain labeled separately and are not counted
as passes. No new CI wiring is introduced. Hosted CI status is reported on the
PR head separately; local passes are not a hosted-check conclusion.

## PERF-005-R1 reconciliation (supersedes earlier qualification claims)

### Source, boundaries and evidence history

R1 starts at `3fef1dd83fde608029c025a48dfce558daf5ac2d`, tree
`bd952b10b0557a2252be0614a04045fea2b6dbf0`, on the same branch/worktree and
PR #109. Corrected executable/tooling implementation commit:
`4107429acc11e9567e2fcbb9b1f5dd8b7ce6bc62`, tree
`adbfce5f2188381bd349d4823cc82e3125c28b04`
(full identities in `r1/implementation-source.txt`). Subsequent report
changes are documentation-only. **No R1 controlled measurement commit exists**:
none of the requested eight clean campaigns was launched. Production source,
generator, providers, fixtures, toolchain seeds and ordinary workflow remain
unchanged. PR #106 and its unfinished corrections remain untouched.

Original evidence is retained under `/home/zboll/perf-005-evidence`. R1 evidence
is the fresh `r1/` subdirectory there; earlier data is historical, not upgraded
to controlled performance. Generator SHA-256 remains
`a6797b3ea64e5bca24960fa2e20aab7d8b9fdb2021ae11804e234181cc3c3760`.
Platform, GNAT14/16 identities, index commit and immutable seed identities are
the ones above. Fresh state seed manifests record copied toolchain file hashes.

### Compiler evidence checks and focused controls

The comparator now derives the complete inventory from owned generated bodies,
bodyless `programs*.ads` namespaces, support bodies and Float32 C sources. It
requires each absolute owned source exactly once with its retained source hash,
the qualified compiler path/hash/version, a successful completed driver and
matching successful `gnat1`/`cc1` backend. Binder-generated source is separate;
provider/unrelated compilation is rejected rather than matched by basename.
Every Ada driver **and backend** must receive the exact expected policy contents
from its ADCs; normal must not receive an assertion-policy ADC. Compiler options
and indirect inputs are checked too. Specs/API dependencies are not falsely
counted as independent compiler-driver inputs.

A complete synthetic 30-positive/20-negative campaign fixture passes the real
comparator. Removing namespace compilation, substituting Ignore in a Check cell,
redirecting to an unrelated same-basename file, failed/incomplete drivers and
failed backends all reject. Response/config/specs content mutations reject;
Check/Ignore never collapse into a normalized filename. Missing indirect inputs
fail closed. Thirteen focused tests pass, including resumed trace reads.

### Effective provider inputs: a remaining functional gate, not a pass

The old command comparison **dropped** response/specs/configuration/source-map
arguments and normalized ADC filenames without checking contents. Its earlier
“command equivalence” success is withdrawn. The corrected representation keeps
all switches and recursively includes response, GCC specs, generated config,
ADC and mapping contents. Only campaign/state roots and expanded temporary
filenames are abstracted. Unit/kind/name/path triples in GNAT mapping files are
sorted only after checking for duplicate/ambiguous unit entries. No semantic
compiler input is excluded on the basis of a temporary filename.

The first regenerated originals retained post-build mapping files and exposed
differences. GCC's `gcc/ada/fmap.adb` confirms `Update_Mapping_File` appends new
entries. Post-build mapping contents are therefore **insufficient input evidence**.
The profiler now additionally records `strace -yy` read-time bytes for each
`gnat1` mapping file, keyed by source and mapping path; absent input bytes cause
incomplete evidence, never fallback to the mutated output. These verbose traces
are functional evidence only and are excluded from clean timings.

Even the read-time maps differ: GPR recycles these in/out files between concurrent
compiler invocations, so later files contain scheduling-dependent accumulated
entries. Strict per-source effective-input comparison rejects these differences.
This report does **not** assert they are harmless or silently union/drop them.
Resolving that semantic question (or using a validated equivalent provider build
preparation context) is required before pilot qualification. All other compared
provider compiler options/response/config/specs/ADC inputs match after documented
root mapping. See `r1/reconciliation.json` for policy/source differences.

The final implementation additionally resolves `bin/../libexec` backend paths
before comparing qualified identities. This is a validation-only correction;
no compiler invocation or provider/consumer build behavior changed after the
refreshed traces. Raw traces were rechecked, not relabeled as measured runs.

### Isolation correction and environment requirements

The pilot and disposable controls now share `consumer_layout`. After resolving
paths, it rejects a writable ancestor of any protected provider and any writable
subtree inside it, including symlink aliases. `run` validates the final allowlist
including trace output before launch. Pilot startup rejects unsafe state/HOME/
`/tmp`/`/dev` overlap before preparation. Providers beneath `/tmp` are unsupported
with this allowlist and are explicitly rejected; a regression uses the actual
adapter list rather than a hand-selected narrower test sandbox.

The supported Linux x86_64 host test demonstrates that a chmod-capable owning
consumer still cannot overwrite provider bytes, while private writes work.
Landlock ABI <3, disabled Landlock, wrong architecture or failed enforcement
are fatal—there is no permission-only fallback. This remains an experimental
host requirement, not a production portability claim.

### Reuse versus regeneration

| Evidence | R1 handling |
|---|---|
| Clean baseline14/16 and candidate14/16 | Retained unchanged; ordered outcomes, bytes, locks, diagnostics, atomic rejection and runtime markers still pass |
| Initial original three-policy traces | Retained historically; insufficient temporary/read-time inputs, not accepted for effective-input equivalence |
| Original `traced14/16` complete pilots | Preserved; complete source/driver/backend/policy evidence reconciled, but no read-time mapping capture; stricter indirect-input gate rejects them as incomplete |
| `r1/original14/16` | First bounded refresh; exposed mutable mapping output limitation; not substituted for valid effective-input evidence |
| `r1/original-v214/16` | Bounded three-policy originals regenerated with read-time input tracing; parser-only re-extraction from unchanged raw trace is recorded, with old JSON copies preserved |
| `r1/pilot14/16` | Complete traces refreshed because original candidate traces cannot supply read-time mapping inputs; uses corrected overlap validation |
| `r1/final-controls14/16` | Disposable generated/support Ada/C failure controls under corrected shared allowlist; nine per compiler |

Each complete refreshed compiler campaign preserves **30 positive cells and 20
negative CLI cases**, 120 generated and 120 support compiler drivers plus 30
binder compilations. Provider preparations remain three × 106 drivers (318),
not 30 × 106 (3180). These counts and executions are not a substitute for the
remaining provider effective-input equivalence gate.

### Controlled repetitions: unavailable, not run

`r1/resource-allocation.txt` records topology, SMT siblings, process activity,
CPU counters, boot options and privilege/controller availability. Four logical
CPUs 4,6,8,10 are distinct cores, but their siblings 20,22,24,26 are unreserved.
There are no isolated/nohz-full CPUs, no cpuset controller delegated to this
session (only cpu/memory/pids), and `sudo -n` requires a password. Desktop,
browser and unrelated development activity is free to migrate across all CPUs.
Affinity cannot establish those cores/siblings remain free of competing work.
No unrelated process/worktree was changed to manufacture a reservation.

| Compiler | Required order | Baseline A elapsed/CPU | Pilot A elapsed/CPU | Pilot B elapsed/CPU | Baseline B elapsed/CPU |
|---|---|---|---|---|---|
| GNAT14 | baseline A → pilot A → pilot B → baseline B | unavailable | unavailable | unavailable | unavailable |
| GNAT16 | baseline A → pilot A → pilot B → baseline B | unavailable | unavailable | unavailable | unavailable |

Pair improvement, CPU reduction and repeat spread are **not available**.
The previous 381.858/102.744 s and 480.263/113.808 s observations remain single
contended pairs only. No observation was discarded as a slow controlled run;
there were zero controlled attempts. Refreshed tracing is not timing evidence.

**Disposition: PROVIDER_REUSE_PILOT_INCOMPLETE.** Missing gates are now explicit:
(1) substantiate semantic equivalence of differing read-time GNAT mapping inputs;
(2) obtain an exclusive four-distinct-core allocation including SMT siblings and
run the requested eight fresh-output campaigns under independent outer time.
The smallest next task is the mapping-input analysis/preparation correction,
then controlled pairs on a suitable host. Production integration is premature.

### R1 evidence index and reproduction

All paths below are within `/home/zboll/perf-005-evidence/r1`:

- `implementation-source.txt`, `tooling.sha256`: corrected implementation
  commit/tree and script identities; `evidence.sha256`: selected R1 raw evidence.
- `resource-allocation.txt`: allocation blocker, topology and activity.
- `original-v214/16`: refreshed original representative traces and read-time
  mapping bytes. `reparse16.log` / preserved `*.pre-parser-fix.json` identify
  parser-only extraction corrections; raw trace bytes were not changed.
- `pilot14/16`: refreshed complete positive/negative campaigns and provider
  preparation, with source/seed/artifact manifests and mapping-read capture.
- `reconciliation.json`, `reconcile-final.log`: complete source compilation
  counts/outcomes plus strict provider-input rejection details.
- `final-controls14/16`, `final-controls*.log`, `final-controls.status`: affected
  compiler-failure controls rerun using the corrected shared isolation path.
- `tests-final.log`: 13 tests, with all complete-fixture mutations and isolation
  controls; `placement.log`, `ci-split.log`, `layout-mutations.log`, `fmt.log`:
  existing relevant gates. No Rust source changed; no full workspace rerun claim.
- `starting-head-hosted.txt`, `final-head-hosted.txt`: hosted checks, separate
  from opt-in qualification.

```sh
R=/home/zboll/git/ams-gra-codegen-oms-perf-005
E=/home/zboll/perf-005-evidence
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s "$R/scripts" -p 'test_perf005*.py'
for c in 14 16; do
  # Consumer equivalence and full actual driver/backend/policy/source evidence:
  PYTHONDONTWRITEBYTECODE=1 python3 "$R/scripts/perf005-compare.py" \
    "$E/baseline$c" "$E/candidate$c" --traced "$E/r1/pilot$c"
  # Strict provider effective-input gate: currently EXPECTED TO REJECT,
  # not an intentional-failure qualification pass:
  PYTHONDONTWRITEBYTECODE=1 python3 "$R/scripts/perf005-compare.py" \
    "$E/baseline$c" "$E/candidate$c" --traced "$E/r1/pilot$c" \
    --attribution "$E/r1/original-v2$c"
done
```

The environment must have the qualified compiler/runtime paths or retained
identity evidence and unchanged source paths. Run actual compilation and
Landlock controls in the qualified task-owned container; Landlock tests require
a supported Linux x86_64 host and do not silently skip enforcement failures.