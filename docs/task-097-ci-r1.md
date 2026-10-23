# Task 097-CI-R1 — hosted ownership correction and main reconciliation

This is a bounded CI correction to PR #102, not a new Float64 implementation
or a replacement numerical qualification. The production scope and historical
results in `task-097-ada-float64-production.md` remain unchanged. No Owned_V1
work is introduced. Task 098 / PR #104 is not modified.

## Original hosted failure (verified before editing)

Reviewed source: `96489aa95f21f86706e227443a8bfd29b81c1d41`, tree
`f594331af4c5a42691f4563458a54b589af67a8b`.

[CI run 39547614586](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39547614586):

| Job | Job ID | Failed step |
|---|---|---|
| `ada-float64-codec (14)` | `119049245097` | Record exact source identity |
| `ada-float64-codec (16)` | `119049245170` | Record exact source identity |

Both downloaded job logs show:

```text
fatal: detected dubious ownership in repository at '/__w/ams-gra-codegen-oms/ams-gra-codegen-oms'
Process completed with exit code 128.
```

The hosted job step records confirm successful platform/GNAT preflight and
checkout of the reviewed SHA. The Rust toolchain, Alire installation,
exact-head generator build, and production numerical qualification were
**skipped**. The other ten CI jobs passed. All six jobs in
[Deep CI run 39547614524](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39547614524)
passed at the same source HEAD. This is an ownership/trust failure, **not an
observed Float64 numerical regression**. The old failed run does not qualify
Float64.

## Exact correction

Commit `d0b8f28effb00925ab5b081c6fbdd071df3c8772` changes only the shared
Float64 matrix's blocking source-identity step. After `set -euo pipefail`:

```bash
test -n "${GITHUB_WORKSPACE:-}"
test "$(pwd -P)" = "$(realpath "$GITHUB_WORKSPACE")"
git config --global --add safe.directory "$GITHUB_WORKSPACE"
git rev-parse HEAD 'HEAD^{tree}' | tee /tmp/task097-evidence/source.txt
actual_sha="$(git rev-parse HEAD)"
test "$actual_sha" = '${{ github.event.pull_request.head.sha || github.sha }}'
porcelain="$(git status --porcelain)"
test -z "$porcelain"
```

Only the actual, verified checkout directory is trusted. There is no wildcard,
disabled ownership checking, nonblocking step, branch-name substitution, or
checkout-SHA change. Assignments make failures of the identity/status Git
commands independently fatal under `set -e`. Both compiler entries (`14`, `16`)
use this same step.

An isolated, network-disabled Podman container reproduced the ownership failure
using a repository owned by a different UID and an isolated HOME/config. Before
the correction, `git rev-parse HEAD` returned 128 with the same diagnostic.
The actual extracted workflow script passed after registering the exact path.
Wrong HEAD, dirty checkout, empty workspace, and mismatched workspace negative
controls failed. A second foreign-owned repository remained untrusted. These
are corrective tests, not substitutes for hosted numerical qualification.

## Ordinary merge of current main

Fetched main: `23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39` (PR #98 merge),
containing PY-001 implementation `93d02dba9bdc398453a29b5dd3b9956ae26793fb`.
The inspected new commits add only the Python backend, its tests/report,
workspace membership/lockfile entry, and its CI job.

Normal merge commit: `18dfb458f5a466c37664fa51393134f82a359fae`.
Parents, in order: `d0b8f28effb00925ab5b081c6fbdd071df3c8772` and
`23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39`.
Merge tree: `7424f427448d86b4a388e9113f2a2dbd30fd2d69`.

The sole conflict was concurrent job insertion after `jobs:` in
`.github/workflows/ci.yml`. Resolution retained the entire corrected
`ada-float64-codec` matrix and the entire `python-model-backend` job. Parsed
workflow equality verifies the exact union, with every inherited Ada Float32,
JSON Guard, other Ada codec, and Rust job unchanged, as well as triggers,
permissions, concurrency, container and artifact policies. Deep CI is unchanged.
The Python crate, its mandatory tests, workspace manifest, lockfile and PY-001
report are byte-identical to fetched main. No rebase or history rewrite occurred.

## Production preservation and hashes

Comparison against the reviewed source found **1,127 existing files unchanged**.
Only the workflow, `Cargo.toml` and `Cargo.lock` differed before adding this
corrective report. Every Float64 C/Ada source, SAX ingress, typed emitter,
generator/model source, frozen corpus, independent oracle, runtime profile,
and real-UCI fixture/script is byte-identical to the reviewed implementation.

The lockfile comparison checks complete parsed package records, not just names:
all **86 existing package records are unchanged**. The only added package is
`ams-gra-oms-backend-python`, depending on existing workspace crates; no
production numerical dependency changes. The integration is qualified by the
workspace gates and Python runtime tests, not assumed harmless.

SHA-256 source evidence:

| Source | SHA-256 |
|---|---|
| Float64 C conversion/formatting (`ams_gra_float64.c`) | `637ac60a28da1226ffa208d89897d0132df12d7c4da88d77de8f23bad04d9840` |
| Ada Float64 body | `92d9b838fd5ee9e2774b6f177902d079148d269226d5911372302827d100cfd1` |
| Lossless ingress body | `28beb3505c086944d3d2b37e584378f1ad826ac6502e491187a9ca4ca83f77a3` |
| Typed emitter body | `87f3c0cdc95d3d4d36c406a805d10790674582322635560742e47e5b9e62f644` |
| Lossless runtime project | `6979be3b6a8300500fb557a9f62108ae55381f75711fe92d13a10587a4be1a37` |
| Frozen corpus manifest | `e33d3ef9e51635bb0ece91d27a72f4809b49b279586e4693377759473b04d4b3` |
| Numeric oracle | `5eb56198be734513f18f5831a3365cba0eb51df1a21eb9db03afbfe99bd64363` |
| Independent oracle tests | `d8b488458da231b92988ffdfb21a9063a310685dc6865c204cb78f9d069b6e89` |
| Full preserved-file SHA-256 manifest | `364708030e40950df6c6bfee9ef744d39ce6ecbff86d5cd7f78d6e9c90b38982` |

The reconciled generator regenerated both OAM and qualified-namespace Float64
fixtures: **all 16 generated files are byte-identical to both historical GNAT
14 and GNAT 16 campaign artifacts**. In particular:

| Generated artifact | SHA-256 |
|---|---|
| OAM `service_codec.adb` | `46427ab09f02499af3ee95ff1344f7054014b661e3c9c4a9a0077bcd044adfe3` |
| Qualified `service_codec.adb` | `c637a75c863e1df661bd49114639c2c6827e49898fe4febbdb1dc121ca640f0e` |
| Complete generated-artifact JSON hash ledger | `fbb098b5d5d36921c845e8165761320799b6d4a1759314a7b391ce75e7448e5e` |

PY-001's existing-backend comparison also passed using the historical reviewed
generator and the reconciled generator: six generation runs, twelve readiness
reports and two coverage reports. Canonical candidate evidence SHA-256:
`371b0223821c039ccbff57a1e5e1d99b06f010f5e820a10814a84bb9633d8ac2`.
The historical full local numerical campaigns were not restarted for this YAML
correction; no affected numerical implementation/dependency changed.

Both retained historical corpus directories were also checked against the
current pinned manifest and actual TSV content: 27,167 conversion rows
(`14c86c5fa674d279d198947dad03f71f517a40b583729df1184bbf6596a6451b`)
and 20,474 encoding rows
(`b1ad323c7d5d88c465bf966dc2edc1249c68982ad817044fa5579ce65e9abc82`).

## Corrective local qualification

Results are recorded separately for every command under
`/home/zboll/task097-scratch/ci-r1/` (command, output and exit files).

| Gate | Result |
|---|---|
| `cargo +1.95.0 fmt --all -- --check` | PASS |
| `cargo +1.95.0 check --locked --workspace --all-targets` | PASS |
| `cargo +1.95.0 clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo +1.95.0 test --locked --workspace --all-targets` | PASS: 1,445 passed, 0 failed, 8 ignored; 166 binary summaries |
| `bash scripts/check-ci-split.sh` | PASS |
| `bash scripts/test-check-ci-split.sh` | PASS: 210 checks |
| Layout guard with `.github/workflows/deep-ci.yml` | PASS |
| Layout mutation tests | PASS |
| Tasks 060/061/062/063/066/069/077 CI wrapper controls | PASS |
| Python backend `--all-targets -- --nocapture` | PASS: 3 unit + 7 integration tests; runtime probes execute |
| Frozen oracle unit tests | PASS: 4 tests |

The ignored tests retain their existing repository policy; no test was newly
ignored. Python integration tests are mandatory and have zero ignored cases.

The standalone workflow layout checker requires a workflow filename; an initial
invocation without that argument raised `IndexError`. Rerunning with
`.github/workflows/deep-ci.yml` passed without any repository change. The original
invocation/error is retained in the evidence directory. Layout mutation tests,
the CI split guard and its 210-check adversarial suite, Tasks 060/061/062/063/
066/069/077 wrapper checks, all four oracle unit tests, and direct execution of
the seven Python model integration tests have passed. Python probes execute
both ordinary and optimized interpreter modes; no mandatory test is removed or
made optional.

The scratch-only sequential runner also reported a trailing shell parse error
after all ten child commands had written their exit records: its script was
edited while Bash was reading it to correct the layout invocation. This is an
evidence-runner error, not a repository gate failure. The independently checked
`local-qualification.json` consolidates the six successful requested commands,
the corrected layout invocation, and the successful additional checks. The
original runner log and unsuccessful invocations are retained, not relabeled.

## Replacement hosted qualification requirement

Only a fresh run on the corrected pushed HEAD can qualify this correction.
Both Float64 jobs retain the complete unchanged production campaign after the
blocking source gate. The hosted completion assertion still requires twelve
named critical steps. The campaign itself requires 17 successful recorded steps
for compiler 14 (including the C boundary campaign) and 16 for compiler 16,
`completed: true`, `production_generated: true`, and the compiler-specific
completion marker. Evidence upload remains `if: always()` with
`if-no-files-found: error`.

Hosted verification must inspect source SHA/tree, successful preflight,
exact-head build, full numerical execution/counts, completion records and
published artifacts, plus all inherited CI, Python and six triggered Deep CI
jobs. A passed source gate alone is insufficient. Any later failure must be
diagnosed separately. Final exact-head hosted status and PR parity belong in
the PR corrective follow-up; this document does not preclaim a hosted pass.