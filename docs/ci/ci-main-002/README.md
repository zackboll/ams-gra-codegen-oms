# CI-MAIN-REPAIR-002: bounded hosted recovery

## Source and scope

Incorporated main: `55b4cf1a35c7fc9dd1008deff28fea76a41ad39d`, tree
`71245afe416697b424b25647937fa0bf1cd3f836` (fetched again before delivery).
Branch: `fix/ci-main-002-hosted-recovery`; target: `main`.
No full-main dispatch, cancellation, runner-count change, timeout increase,
product/generated-code change, compiler/index/dependency change, or security
policy change is part of this correction.

`baseline.json` records all 139 initially observed jobs in the five requested
runs, including exact source revisions, failing job IDs/steps, process exit
codes, artifact-upload errors and log digests. It is an observation, not a live
qualification receipt: pending/in-progress jobs have no inferred result.

## Confirmed failures and corrections

| Evidence | Diagnosis | Correction |
| --- | --- | --- |
| Run 42447617726, source `1c48bcbff22535ef92b870d7a12ef6be7fdbf90b`, job 128629774798, step 10, exit 1 | All three structured-ASCII tests passed (836.26 s), but the shell requires two | Exact `3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` completion, preserving all four 2.5/2.6 markers |
| Run 42451014054, source `9528fd852be74beddcebe00b9d52af8123a2e9a0`, baseline job 128640060075, Actionlint step 4, exit 22 | Pinned actionlint 1.7.12 asset returns HTTP 504 through all existing curl retries | Shared bounded pinned transport, plus independent official release-asset API path |
| Run 42447617726 jobs 128629774203/128629774207 step 7, 128629774480 step 6; run 42451014054 jobs 128640059911 step 7, 128640060037 step 6; exit 22 | Alire 2.1.1 release-asset HTTP 504 (more affected jobs in baseline) | Same helper at all main/Deep Alire installation sites; existing SHA256/extraction/version assertions remain |
| Run 42451014117, source `9528fd852be74beddcebe00b9d52af8123a2e9a0`, build job 128640060724, step 4, exit 1 | Historical `git show afe0c510866c08fc188fef335e74879840794a70:.github/workflows/ci.yml` fails in shallow checkout | Only `fetch-depth: 0` added to Pages checkout; action v7 and deployment security unchanged |

### Preserved Alire provisioning logs (not compiler defects)

Hosted provisioning exits 1 after a failing Alire subprocess; the underlying
curl exits are distinct and preserved:

* Run 42447617726, job 128629774243, artifact 13612768660,
  `task099-evidence/setup/step-04.stderr`: GNAT 16.1.0 URL, HTTP 504, curl 22.
* Same run, job 128629774627, artifact 13613687891,
  `task101-evidence/setup/step-04.stderr`: same GNAT URL, HTTP 503, curl 22.
* Same run, job 128629774812, artifact 13612346249,
  `setup/step-04.stderr`: GPRbuild 26.0.1's unchanged index origin
  `gprbuild-26.0.0-1` release asset, HTTP 504, curl 22.
* Same run, job 128629774755, artifact 13614908791,
  `setup/step-04.stderr`: GNATCOLL 26.0.0 archive connection failure, curl 56.
* Same run, job 128629774599, artifact 13615690957,
  `setup/step-05.stderr`: GNATCOLL 26.0.0 archive HTTP 504, curl 22.
* Run 42451014054, job 128640060224, artifact 13614837876,
  `task101-evidence/setup/step-04.stdout/.stderr`: GNAT and GPRbuild installed
  successfully; subsequent GNATCOLL archive download HTTP 504, curl 22.
* Same run, job 128640060266, artifact 13611233215,
  `setup/step-04.stderr`: GNAT asset HTTP 504, curl 22.

Configure the supported `origins.archive.download_cmd` only in task-owned
settings and private main-pack workers. Do not retry selection, compilation,
tests or qualification commands. Alire still verifies every index origin hash.
The Deep input reconciliation records the one reviewed setup-script change;
all original suite commands, cells and artifact requirements stay intact. The
new helper also has an independently checked infrastructure source digest.

### Transport and integrity contract

`scripts/pinned_download.py` accepts only five exact pins: Alire, actionlint,
GNAT, GPRbuild, GNATCOLL. Digests are the existing workflow/index SHA256/SHA512
values. Each primary and official alternate gets at most three attempts with
2/4-second backoff, a 30-second connect deadline and 300-second request deadline
(at most six requests / 1,812 seconds). Retry only transient 502/503/504 or
connection/partial-transfer errors. HTTPS and redirect protocol enforcement are
mandatory; TLS/auth/not-found and integrity failures are immediately fatal.

Official API release-asset IDs identify exact assets without accepting ambiguous
release metadata. GNATCOLL's alternate is GitHub's official codeload URL. No
third-party mirror, token, cache trust, or unchecked execution is introduced.
Hash, ZIP CRC/readability or tar/gzip integrity must pass before atomic
publication. Existing local archives are reverified and wrong bytes fail closed.
Alire and actionlint installation retains/adds exact binary version checks.

## Historical and pending evidence

PR #187 already fixed Rust history and verified insufficient deadlines; those
fixes remain unchanged. PR #184's fourteen-to-four Deep allocations and PR
#185's production portable-provider reuse remain unchanged.

Run 42451014054 pack A (job 128640059591), preserved artifact 13628777022:
`ada-hexbinary-codec/completion.json` reports 4,200.292 s total, last command
`check-task093-helper-demand.py --family portable --compiler 16`, timeout,
exit -15 inside the worker; hosted step exit 1. This source predates PERF-009R.
Other pack-A workers completed (generated 1,930.27 s, temporal 2,569.81 s,
datetime 1,456.21 s). Pack B (job 128640060032, artifact 13611990963) failed its
JSON guard prerequisite toolchain provisioning, not an observed worker timeout.
Missing downstream artifact failures are retained as fail-closed consequences,
not repaired by removing mandatory evidence.

Current-main run 42510678267 at source `55b4cf1` initially has all 29 jobs
pending, including main packs 128822842371/128822842452 and four Deep allocations
128822842639/128822842696/128822842770/128822842807. Previous run 42509397852 at
`b412920dc33a965464a7e6359d98a3c3698f8560` also remains incomplete. These are not
evidence of success, failure, or a current timeout. Preserve present worker
limits, nested parallelism controls, independent timeouts, failure propagation,
and complete artifact validation. No new performance campaign is warranted.

## Qualification receipts

Raw local logs: `/tmp/ci-main-002-evidence/` (outside the repository). The PR
description carries the final exact commit/tree and hosted check receipts.

Completed focused checks:

* 259 Python discovery regressions, zero failures/errors/skips, with all five
  opt-in disposable Podman ownership fixtures enabled; 30 CI002 security tests
  using the unchanged hash-locked PyYAML requirement.
* Task060 wrappers: 106 controls, including independent three-test Rust source
  inventory, wrong/missing counts, failures, ignored/filtered tests and each
  missing marker. Task061 wrapper controls pass.
* Twelve download controls: primary success, 504 retry, exhausted primary then
  official alternate, all six requests exhausted, wrong checksum, truncated ZIP,
  truncated gzip footer even with matching digest, wrong/valid cached bytes,
  transport classification/TLS failure, exact/wrong versions and unknown URL.
* Real pinned Alire/actionlint release downloads and exact binary version checks;
  real actionlint official API alternate bytes match the original SHA256.
* Fresh task-owned setup/build/provider smoke with both GNAT 14.2.0 and 16.1.0;
  pinned GPRbuild and GNATCOLL retain Alire origin verification.
* Main/Deep packing inventory, source reconciliation, mandatory evidence,
  independent timeout and propagation controls; compiler-placement regressions;
  Deep layout source check and 1,255-case mutation self-test.
* PERF-005 and PERF-009R integration protections; PR routing/required-result
  regressions; Python 3.11, Ada, Rust and C++ actual backend smoke. C++ uses
  extracted Boost 1.83 headers (same hosted version); no repository pin changed.
* Pinned actionlint 1.7.12, Rust formatting, locked workspace check, locked
  all-target Clippy with warnings denied, `git diff --check`.

Reproductions:

* Actual preserved hosted structured-ASCII three-test output: original gate
  exits 1, corrected exact shell exits 0. The rebuilt release/locked current-source
  suite passes all three tests in 630.31 s with both pinned real UCI roots.
  Its actual output passes the corrected exact workflow shell (exit 0);
  removing a test or introducing an ignored test is rejected (exit 1).
* Fresh HTTPS clone with `--depth=1`: historical `git show` exits 128. After
  `fetch --unshallow`, the unchanged original inventory regression passes and
  historical workflow SHA256 is
  `5c7e5e59daa12934862875d4dac4ea8522f996e05d40f9ffdf3c55cc4b1dc1b3`.

Initial local attempts exposed missing strace, default Python 3.13, generator
target-directory mismatch and missing Boost headers. These were resolved only
in disposable local tooling/environment before rerunning; no CI assertion was
weakened to accommodate this workstation. Supplemental whole-workspace Rust
tests are tracked separately until their process finishes; no pending test is
represented as passing.

No completion of the backed-up full main run is required to deliver this PR.
Exact-head PR smoke and trusted signal must be successful before declaring
`READY_FOR_INDEPENDENT_REVIEW`; otherwise use `HOSTED_CI_PENDING` and name the
unfinished checks. No approval, auto-merge or merge is authorized.