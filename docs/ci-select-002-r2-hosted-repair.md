# CI-SELECT-002-R2: Float32 provisioning and PY-008 reconciliation

## Attributed sources

Starting PR #123 head: `91eab0f938ae65cd2fa5425bf1ac50dc4e05de29`,
tree `488fe18e1d3ce13de6de4cb3eb3a479becadcaad`.
Fetched main: `3f69cb0e5b0801cf01b5cbde9018bb7a3b43fb9b`,
tree `13f9261ec176572d09f47f9392eeaf51741a7bc7`.

Main is merged normally, preserving both parents. PY-008 implementation,
fixtures, evidence, Python 3.11/3.13 controls, pinned UCI/Sleet qualification,
Rust pin, source checks and additive policy contracts remain inherited from
main. CI-SELECT-002 retains manifest v2, Deep job ownership/dependencies,
pre-runner conditions, reusable input, versioned authoritative report and
fail-closed selected-execution/evidence validation.

Historical immutable source identities are not replaced with this merge.
The old Deep baseline digest still binds the original payloads and cells.
The sole PY-008 cell-inventory delta, `py008-numeric` on `real-sleet`, is
required explicitly and projected out only while checking that historical
digest. Independently frozen PY-008 payload and parsed-step contracts still
bind all new mandatory commands, evidence and final-gate step checks.
The inventory remains 40 runner cells, including 12 Deep cells, with five
common main lanes and `queue: max` unchanged.

## Preserved hosted failure

Run: <https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40809839254>
Job: `123239613221`, `ada-float32-codec`, attempt 1.
Artifact: `task095-platform-qualified-float32`, ID `12870773381`, SHA-256
`c1f5cf404be7df104a021ed612c7f6a815e9bb468e3da63f6a35375a3e7d61b6`.

The complete archive and job log are retained. In `optimized-14/toolchain.log`,
curl reports exit 28: failure to connect to `github.com:443` after 132421 ms
while fetching the original AdaCore GNATCOLL v26.0.0 archive. The logged
invocation was:

```sh
alr -n --force toolchain --select --local gnat_external=14.2.0 gprbuild=26.0.1
```

Alire returned 1. The preceding ABI/bridge controls and the GNAT14 standard
and identifier campaigns for both namespace profiles completed. Each completed
campaign contains all Normal/Check/Ignore policies. The failing optimized
campaign stopped during provisioning, before its build/numerical execution.
This is an evidenced network-connectivity failure, not a demonstrated compiler
or numerical regression. Independent consumer workspaces intentionally disable
shared dependencies and therefore repeatedly download provider archives.
No underlying network cause beyond the connection failure is asserted.

## Narrow bounded recovery

`task095_download.py` is configured through Alire 2.1.1's documented
`origins.archive.download_cmd`. Because that option is global-only, Task095
uses a fresh scratch-owned settings directory rather than modifying the
developer or runner's global configuration. Consumers remain independent.

Only the exact original GNATCOLL v26.0.0 URL gets up to three attempts.
Transport failures and HTTP 408/429/500/502/503/504 are eligible; certificate,
404, hash/provider and build failures are not. Delays are capped at 2 and 4
seconds; each eligible curl invocation has a 30-second connect timeout and 180-second
total timeout. Other archive URLs receive one attempt, without substitution.
No qualification command, build or entire job is retried.

Every attempt retains command/cwd, combined stdout/stderr, exit status and
decision/retry reason in files already covered by the mandatory artifact
patterns. Transfers use distinct sibling `.part` files, removed on failure.
The original SHA-512 must match before the pinned archive is atomically
published. Alire's own origin verification remains active for every archive.
GNAT identity, provider versions/origin hashes/source digest, lock copies,
all numerical tests, O0/O2 paths, assertion policies and markers are unchanged.
The downloader itself is included in `sources.sha256`.

Failure-injection tests cover transient transport/HTTP recovery, exhausted
timeouts, corrupt success payload, wrong provider/source, non-transient
rejection, other-origin nonretry and pre-existing partial destination refusal.
The actual GNAT14 O2 environment/rounding path was also executed locally on
x86_64/glibc 2.41, with all three policies, all 16 rounding pairs per policy,
and exact generated-execution markers. Fresh provider downloads passed the
original archive and source checks.

## Acceptance boundary

Local tests do not authorize hosted qualification. Preserve all original-run
terminal conclusions without cancelling outstanding jobs, then qualify the
reconciled exact head. Require all ordinary/Deep cells, classifier, baseline,
authoritative gate, CI002, trusted signal and fresh mandatory evidence.
The original failed run cannot authorize merge. Selective fixtures and measured
savings remain unverified until their actual hosted executions finish.

No replacement implementation PR, merge, approval, force push, runner or
permission change, trusted-dispatch enablement or qualification weakening is
part of this repair. PR #123 stays open, non-draft and without auto-merge.