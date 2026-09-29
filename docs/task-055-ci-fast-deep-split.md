# Task 055 — Split fast PR CI from deep post-merge validation

Branch `feature/055-ci-fast-deep-split`, from `main` at
`e1810ce4b8ddb1ab89727b6ecada88999b4dcb49` (merge of PR #55, Task 054).

CI infrastructure only. No Schema IR, codegen-core, backend, runtime, service
readiness, fixture, generated-output or test-expectation change. This task
changes **when** the expensive external/reference evidence runs, not **whether**
it exists.

## 1. Baseline (before Task 055)

The final PR #55 head `d82a2a94ef67f756d074f547b378302c6f85fc93` ran the SAME
monolithic `CI` workflow twice, because it triggered on both `push` (any
branch) and `pull_request`:

| Run | Event | Created | Completed | Wall time |
| --- | --- | --- | --- | --- |
| [36500426797](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/36500426797) | `push` | 2026-09-28T23:54:32Z | 2026-09-29T00:21:10Z | 26m38s |
| [36500431807](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/36500431807) | `pull_request` | 2026-09-28T23:54:36Z | 2026-09-29T00:28:09Z | 33m33s |

Same source commit, same steps: **~60 runner-minutes** of duplicated full
validation for one PR head, with the slowest run (33m33s) on the review
critical path.

Per-step durations of the slow steps (from the job step timestamps):

| Step | push run | PR run |
| --- | --- | --- |
| Real UCI Binary provenance inventory (Task 052) | 0m24s | 0m26s |
| Real UCI constrained Binary inventory (Task 053) | 2m29s | 3m15s |
| Real UCI member identifier inventory (Task 054) | 3m53s | 5m02s |
| Rust LA-CAL runtime MSRV (1.95.0) incl. real-UCI compile | 7m49s | 9m54s |
| Runtime against real pinned Sleet (e38f61d8) | 8m04s | 10m41s |
| **Total of steps that are real-UCI/Sleet/MSRV-heavy** | **22m39s** | **29m18s** |

## 2. Inventory of the merged monolithic workflow (`e1810ce`)

Single job `rust`, steps in order, classified:

| # | Step | Class |
| --- | --- | --- |
| 1 | `actions/checkout@v4` | FAST |
| 2 | `dtolnay/rust-toolchain@stable` (rustfmt, clippy) | FAST |
| 3 | Install GNAT | FAST |
| 4 | Verify GNAT availability (hard gate) | FAST |
| 5 | `cargo fmt --all -- --check` | FAST |
| 6 | `cargo check --workspace --all-targets` | FAST |
| 7 | `cargo clippy --workspace --all-targets -- -D warnings` | FAST |
| 8 | Ada regressions (GNAT-backed; Tasks 033/040/041/042/044/046/047/048) | FAST |
| 9 | Rust LA-CAL runtime, generated facade -> mock OWP (Task 049) | FAST |
| 10 | Generated Rust OMS JSON codec (Task 050, synthetic) | FAST |
| 11 | Member QName provenance (Task 051, synthetic) | FAST |
| 12 | hexBinary provenance and Binary codecs (Task 052, synthetic) | FAST |
| 13 | Real UCI Binary provenance inventory (Task 052) | REAL-UCI |
| 14 | Constrained Binary carriers (Task 053, synthetic, GNAT + strict C++) | FAST |
| 15 | Real UCI constrained Binary inventory / message impact (Task 053) | REAL-UCI |
| 16 | Member identifier remapping (Task 054, synthetic, GNAT + strict C++) | FAST |
| 17 | Real UCI member identifier inventory / whole-schema first blocker / real category-A compile (Task 054) | REAL-UCI |
| 18a | Rust 1.95.0: `runtime-api`, `runtime-rust --all-targets`, `runtime-rust-facade-tests --all-targets` (no UCI root) | MSRV-local |
| 18b | Rust 1.95.0: `runtime-rust-facade-tests --all-targets` with `AMS_GRA_UCI_2_5_ROOT` (generated real PositionReport + SubsystemStream codecs) | MSRV-real-UCI |
| 19a | Real UCI 2.5 PositionReport codec + SubsystemStream hexBinary codec tests | REAL-SLEET (needs real UCI 2.5, no Sleet) |
| 19b | `scripts/run-real-sleet-test.sh` (Tasks 049/050/051/052/053 Sleet tests + real UCI PositionReport/SubsystemStream through Sleet) | REAL-SLEET |
| 20 | `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` | FAST |

Step 18 was one workflow step containing both 18a and 18b; step 19 was one
step containing 19a and 19b. Both are split along that seam here. Step 19a does
not start Sleet, but it needs the real UCI 2.5 root and shares the
`real_uci_*` test binaries with 19b, so it moves with the Sleet job.

Fetches in the old workflow: UCI 2.5 was fetched **5 times** (steps 13, 15, 17,
18b, 19) and UCI 2.6 **3 times** (13, 15, 17) in one job. The later calls skipped the network
`git fetch` (both scripts reuse an existing `$RUNNER_TEMP` clone), but every call
still re-checked out and re-hashed, and every 2.6 call deleted and re-extracted
the release CDRL zip.

## 3. Architecture after Task 055

```text
  feature branch / PR
          |
          v
      Fast CI            (pull_request; the only PR-blocking workflow)
          |
          | merge
          v
        main
       /    \
      v      v
  Fast CI   Deep CI      (push to main; Deep CI also nightly + manual)
```

- **Fast CI** (`.github/workflows/ci.yml`, workflow `CI`, job `rust`): "Does
  this commit compile and pass deterministic local/synthetic regression?"
- **Deep CI** (`.github/workflows/deep-ci.yml`, workflow `Deep CI`, jobs
  `real-uci`, `msrv-real-uci`, `real-sleet`): "Does merged main still agree
  with pinned real UCI and pinned Sleet?"

### 3.1 Fast CI triggers

```yaml
on:
  pull_request:
  push:
    branches:
      - main
  workflow_dispatch:
```

- A push to a feature branch with an open PR starts ONE run (`pull_request`).
- A push to a feature branch with no PR starts nothing; opening the PR starts
  it. There is no branch-name heuristic.
- A merge starts ONE `push` run on `main`.
- `workflow_dispatch` allows a manual run on any ref.

### 3.2 Fast CI concurrency

```yaml
concurrency:
  group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
```

- PR #N: group `ci-CI-N`. A newer head on PR #N cancels the in-flight run for
  the older head.
- Different PRs have different numbers, so they never cancel each other.
- `main`: the PR number is empty, so the group is `ci-CI-refs/heads/main`,
  shared with no PR. Back-to-back merges cancel the older main run; the newer
  commit contains it.
- A manual dispatch groups by its ref (`refs/heads/<branch>`), never a PR
  number, so it cannot cancel PR runs. (A dispatch on `main` does share the
  main group.)

### 3.3 Deep CI triggers

```yaml
on:
  push:
    branches:
      - main
  schedule:
    - cron: "0 6 * * *"      # daily, 06:00 UTC
  workflow_dispatch:
  pull_request:
    paths:
      - ".github/workflows/**"
      - "scripts/fetch-pinned-uci-2.5.sh"
      - "scripts/fetch-pinned-uci-2.6.sh"
      - "scripts/run-real-sleet-test.sh"
```

- **push to main**: every merge gets the real evidence as a hard, visible
  signal on the merge commit.
- **nightly**: catches drift no commit introduced (runner image, floating
  stable toolchain, crates.io, upstream UCI/Sleet hosts).
- **manual**: investigation on any ref.
- **pull_request (paths)**: only a PR that changes the CI harness itself (any
  workflow file, or one of the three scripts that exist only for the deep
  evidence) runs Deep CI before merge, so a harness change is exercised by the
  evidence it changes. Ordinary source PRs match none of these paths and do
  not run Deep CI.

### 3.4 Deep CI concurrency

```yaml
concurrency:
  group: deep-ci-${{ github.ref }}
  cancel-in-progress: true
```

If commits A and B merge to `main` while A's deep validation is still running,
B contains A and is the more useful target, so A's run is cancelled; a
cancelled main Deep CI run in that situation is expected, not a failure. The
nightly schedule runs on `refs/heads/main` and shares the main group. PR
harness validation uses `refs/pull/N/merge` and cannot cancel main.

## 4. What stays in Fast CI (pre-merge, blocking)

- checkout; stable Rust with rustfmt + clippy;
- Install GNAT + the hard `gnatmake --version` gate;
- `cargo fmt --all -- --check`;
- `cargo check --workspace --all-targets`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- GNAT-backed deterministic Ada regressions (Tasks 033/040/041/042/044/046/
  047/048);
- Task 049 generated facade -> mock OWP, plus the runtime fairness tests;
- Task 050 synthetic generated OMS JSON codec + codec readiness;
- Task 051 synthetic member-QName tests;
- Task 052 synthetic hexBinary tests;
- Task 053 synthetic constrained-Binary tests (GNAT under both assertion
  policies, strict C++, Rust carrier + codec, mock OWP);
- Task 054 synthetic member-remapping / codec tests (GNAT, strict C++, Rust,
  mock OWP);
- **MSRV-local**: `rustup toolchain install 1.95.0 --profile minimal`, then
  `cargo +1.95.0 check --locked` of `ams-gra-oms-runtime-api`,
  `ams-gra-oms-runtime-rust --all-targets` and
  `ams-gra-oms-runtime-rust-facade-tests --all-targets`, with NO real UCI
  root. This still compiles every generated synthetic facade and codec under
  the declared floor;
- **new**: `Fast/Deep CI split guard (Task 055)` (section 7);
- final `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`.

Fast CI still exercises Rust, Ada/GNAT, strict generated C++, mock OWP,
generated codecs and generated model APIs. It is not lint-only.

Outside comments, Fast CI contains no `fetch-pinned-uci-2.5.sh`,
`fetch-pinned-uci-2.6.sh`, `run-real-sleet-test.sh`, `AMS_GRA_UCI_2_5_ROOT` or
`AMS_GRA_UCI_2_6_ROOT`; `scripts/check-ci-split.sh` enforces it. Without those
variables the real-UCI tests inside `cargo test --workspace` print `SKIPPED`,
exactly as they already did in that final step before Task 055. Their real
execution, gated on PASSED markers, is Deep CI's job.

## 5. What moved to Deep CI

| Old Fast CI step | Deep CI job / step | Check preserved |
| --- | --- | --- |
| 13. Real UCI Binary provenance inventory (Task 052) | `real-uci` / same name | both inventory markers + `2 passed` |
| 15. Real UCI constrained Binary inventory (Task 053) | `real-uci` / same name | 2 inventory + 2 message-impact markers + `3 passed` |
| 17. Real UCI member identifier inventory / first blocker / category-A (Task 054) | `real-uci` / same name, `AMS_GRA_REQUIRE_GNAT=1` | 2 inventory + 2 first-blocker markers, prefix-aware CATEGORY-A line, `4 passed` |
| 18b. Real-UCI generated-code compile under 1.95.0 | `msrv-real-uci` / `Real-UCI generated code MSRV (1.95.0)` | same `cargo +1.95.0 check --locked -p ams-gra-oms-runtime-rust-facade-tests --all-targets` with `AMS_GRA_UCI_2_5_ROOT` |
| 19a. Real UCI 2.5 PositionReport / SubsystemStream codecs | `real-sleet` / `Real UCI 2.5 codecs (Tasks 050/052, must execute)` | both markers + `1 passed` each |
| 19b. `scripts/run-real-sleet-test.sh` | `real-sleet` / `Rust LA-CAL runtime against real pinned Sleet (e38f61d8)` | all 7 Sleet tests (5 synthetic + 2 real UCI) |

Every test target, filter, `--release` / `--test-threads 1` setting, marker
text and `N passed` count is unchanged. Nothing was deleted.

**Moving a test to Deep CI is not weakening or deleting it.** The evidence runs
on every merge to `main`, every night and on demand, and a red Deep CI on
`main` is a real failure to fix with the same priority as before. What changes
is that an ordinary feature PR no longer waits ~30 minutes, twice, for evidence
about external pinned artifacts that it did not change.

### 5.1 Job layout and GNAT

The three jobs have no `needs:` between them and run in parallel.

| Job | Rust | GNAT | UCI | Sleet |
| --- | --- | --- | --- | --- |
| `real-uci` | stable | **installed + hard gate**: the Task 054 category-A test runs `gnatmake -gnatc` on generated Ada; `AMS_GRA_REQUIRE_GNAT=1` turns a missing GNAT into a failure | 2.5 + 2.6 | no |
| `msrv-real-uci` | 1.95.0 only | not installed: only `cargo check` (build.rs generates Rust; nothing invokes GNAT) | 2.5 | no |
| `real-sleet` | stable (Sleet's own `rust-toolchain.toml` pins 1.95.0, which rustup installs for the Sleet build) | not installed: the Sleet and real-codec tests are Rust-only | 2.5 | built from the pinned revision |

`msrv-real-uci` uses only `cargo +1.95.0`, so it skips the stable toolchain
action it would never use.

### 5.2 UCI fetch reuse

Each job fetches each release it needs exactly once, in a dedicated step that
exports the verified root paths:

```yaml
- name: Fetch pinned UCI 2.5 and 2.6 (once)
  run: |
    set -euo pipefail
    UCI25="$(scripts/fetch-pinned-uci-2.5.sh "$RUNNER_TEMP/uci-2.5")"
    UCI26="$(scripts/fetch-pinned-uci-2.6.sh "$RUNNER_TEMP/uci-2.6")"
    echo "AMS_GRA_UCI_2_5_ROOT=$UCI25" >> "$GITHUB_ENV"
    echo "AMS_GRA_UCI_2_6_ROOT=$UCI26" >> "$GITHUB_ENV"
```

Later steps read `AMS_GRA_UCI_2_5_ROOT` / `AMS_GRA_UCI_2_6_ROOT` from the job
environment. SHA verification is intact: the scripts still pin the tag revision
and root SHA-256 and fail hard (a failing command substitution in an
assignment aborts the step under `set -e`), and the tests and the facade-tests
`build.rs` still re-hash the root before using it. The jobs run on separate
runners, so 2.5 is fetched once in each of the three jobs and 2.6 once in
`real-uci`, instead of 5 + 3 invocations in one job.


## 6. Marker-check robustness

Task 054 found two harness hazards:

1. `printf '%s\n' "$output" | grep -q ...` under `set -o pipefail`: `grep -q`
   exits at its first match, and a writer still emitting a large output can be
   killed by SIGPIPE, failing an otherwise-passing step.
2. A `--nocapture` marker from a test that prints nothing else lands on
   libtest's unterminated `test <name> ... ` line.

The Task 054 real-UCI step already used here-strings and the prefix-aware
CATEGORY-A match. The moved Task 052 / 053 real-UCI steps and the real codec
checks from old step 19 still used `printf | grep -q`. While moving them:

- every moved marker check is `grep -Fqx '<marker>' <<<"$output"` (fixed
  string, whole line: the same match as the old `'^<marker>$'`); `N passed`
  checks are `grep -qE '^test result: ok\. N passed' <<<"$output"`;
- the Task 054 CATEGORY-A check is kept verbatim:
  `grep -Eqx '(test task054_real_uci_category_a_selection_generates_and_compiles \.\.\. )?UCI 2\.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED' <<<"$output"`;
- `scripts/run-real-sleet-test.sh` `run_one` had the same `printf | grep -q`
  shape and is fixed the same way (`grep -Fqx -- "$3" <<<"$output"`); its
  tests, markers and flow are unchanged;
- every Deep CI `run:` block keeps `set -euo pipefail`.

No Rust test changed to simplify matching.

The synthetic Fast CI steps keep their existing `require_one_test` helper.
Rewriting them is out of scope (task section 16) and is a candidate for the
follow-up that measures this split.

## 7. Structural guard

`scripts/check-ci-split.sh` runs as a Fast CI step and locally. It inspects
the non-comment lines of both workflows and the Sleet script, and fails if:

- Fast CI references `fetch-pinned-uci-2.5.sh`, `fetch-pinned-uci-2.6.sh`,
  `run-real-sleet-test.sh`, `AMS_GRA_UCI_2_5_ROOT` or `AMS_GRA_UCI_2_6_ROOT`;
- Fast CI loses the GNAT gate, fmt / check / clippy, the three MSRV-local
  checks, `AMS_GRA_REQUIRE_GNAT` or the final `cargo test --workspace`;
- Deep CI loses any moved test target or marker, the 1.95.0 real-UCI compile,
  either fetch invocation, or the Sleet script;
- Deep CI invokes the 2.5 fetch more than three times (once per job) or the
  2.6 fetch more than once;
- Deep CI or the Sleet script contains a `printf ... | grep -q` check.

It matches substrings, not layout: reindenting, renaming or reordering steps
stays legal, and comments may mention anything.

`scripts/test-check-ci-split.sh` is its adversarial self-test (34 checks):

- the guard passes on the committed files, and on Fast CI plus a comment that
  only mentions the Sleet script;
- the guard fails for: a UCI 2.5 fetch, a UCI 2.6 fetch or the Sleet script
  added to Fast CI; the local MSRV check or the workspace test removed; each of
  8 moved Deep CI checks removed; a second 2.6 fetch; `printf | grep -q`
  reintroduced in Deep CI or in the Sleet script;
- the exact Deep CI marker functions, under `set -euo pipefail`, against
  synthetic libtest output with the markers FIRST and a 20 000-line tail:
  known-good output passes (CATEGORY-A both prefixed and standalone); removing
  any required marker or the `N passed` line fails; a CATEGORY-A marker behind
  a different test name fails; a marker that is not a whole line fails;
- the tested marker lines are asserted byte-identical to those in
  `deep-ci.yml`.

The marker functions run as plain statements, never under `if` / `||`: bash
suspends `errexit` throughout a conditional, even inside subshells, so every
grep but the last would be ignored. The first draft of this self-test had
exactly that bug and accepted output with a removed marker. After the fix, a
deliberately weakened marker function (one required grep replaced by `true`)
makes the self-test fail.

## 8. Scope

Changed: `.github/workflows/ci.yml`, new `.github/workflows/deep-ci.yml`, the
`run_one` marker check in `scripts/run-real-sleet-test.sh`, new
`scripts/check-ci-split.sh` + `scripts/test-check-ci-split.sh`, and docs
(this file, `README.md`, `CONTRIBUTING.md`, `docs/roadmap.md`).

Unchanged: every `.rs` file, every fixture, every test expectation, Schema IR,
codegen-core, backends, runtimes, service readiness and generated output.
`.github/workflows/uci-pages.yml` is untouched. Branch protection is not
modified.

The Task 054 roadmap item stays open: `OrderOfBattle` is `service-check`
READY yet `service-generate` fails on types reached only through later
generated-support expansion. It remains the leading candidate for the next
product-correctness task.

## 9. Validation

### 9.1 Local static checks

| Check | Result |
| --- | --- |
| `actionlint` 1.7.7, `.github/workflows/*.yml` | clean |
| YAML parse with `yaml.BaseLoader` (every scalar a string, so YAML 1.1 does not turn `on` into `true`) | both files: `on`, `concurrency`, `jobs` as intended |
| `bash -n` on the three scripts | clean |
| `scripts/check-ci-split.sh` | PASSED |
| `scripts/test-check-ci-split.sh` | PASSED (34 checks) |
| weakened marker function (mutation of the self-test) | self-test FAILS, as required |
| `git diff --check` | clean |

### 9.2 Deep CI bodies executed locally

A small driver read the literal `run:` strings from `deep-ci.yml` and ran each
under `bash -e -c` with the step `env:` and `GITHUB_ENV` carried between steps,
as the runner does. Only `Install GNAT` (apt) was skipped; GNAT is installed
locally and the hard `Verify GNAT availability` gate still ran.

| Job | Steps | Result | Local wall time |
| --- | --- | --- | --- |
| `real-uci` | GNAT gate, fetch 2.5 + 2.6 once, Task 052, Task 053, Task 054 | all exit 0 | 4m34s |

The Task 054 step's local output contained the CATEGORY-A marker on libtest's
line exactly as expected:
`test task054_real_uci_category_a_selection_generates_and_compiles ... UCI 2.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED`.
The other two jobs and the full-repository regression are recorded below.

### 9.3 GitHub observation

Recorded after the push; see the follow-up commit.

