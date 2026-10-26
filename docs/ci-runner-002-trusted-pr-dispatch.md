# CI-RUNNER-002A — automatic trusted PR dispatch (disabled)

## Scope and trust decision

This is an **authorization architecture, not host isolation**. After review and
one-time operator configuration, the normal path is:

`owner opens/updates OMS PR → hosted signal → hosted default-branch dispatcher
→ private repository_dispatch → hosted reauthorization → Ryzen complete Float32
→ hosted ryzen/float32 commit status`.

There is no routine manual dispatch, VM, public self-hosted registration or
runner migration. Both activation variables are absent by default. Existing
`CI`, `Deep CI`, their runner assignments and required qualification are untouched.
PR #110 remains open/unmodified; none of its staging implementation was assumed
available. Inspection started at OMS main
`a849b87d94948980133038ba909c0b6528cadde6`, pilot main
`ebe5423aa3850003a48a8119447553bde3baedf6`.

The existing private runner is persistent, is in the host `docker` group, and
job containers have the **host Docker socket**. Authorized malicious code,
compromised owner/App credentials, compromised dependencies/actions/toolchain
downloads, or a contaminated previous build can compromise the mining host and
future results. Neither numeric identities nor a pinned container prevent that.
**Operator must explicitly accept this residual risk before activation.** No
such approval or production credential configuration was performed by this task.

## Authoritative identity and fail-closed decisions

Verified through GitHub API:

| Object | Immutable ID |
| --- | --- |
| `zackboll` user | **4860920** |
| `zackboll/ams-gra-codegen-oms` | **1373902528** |
| `zackboll/ci-runner-pilot` | **1430048464** |

Only this user appeared in either collaborator list, with admin permission.
OMS interaction limits returned `{}`; main protection returned 404 and no public
rulesets were listed. No settings were changed. GitHub interaction limits offer
collaborators-only restrictions, not a distinct permanent owner-only PR mode.
Where available, collaborators-only plus retaining the owner as the only
collaborator is defense in depth, but availability/duration/settings should be
checked by the operator. It cannot replace this gate and does not make forks safe.

The no-secret signal handles `opened`, `synchronize`, `reopened`,
`ready_for_review`, and `edited` (including base retargets), runs a constant
`printf` on `ubuntu-24.04`, and has empty token permissions. It checks out no
code, reads no artifacts and performs no dispatch or build. It may run on an
untrusted PR; **its successful conclusion is not authorization by itself**.

The dispatcher runs only via `workflow_run` from default-branch code. It checks
API repository IDs/names, configured numeric workflow IDs plus exact paths and
active state, event `pull_request`, successful completed first attempt, owner
actor and triggering actor IDs, one unambiguous associated PR, creator ID,
same-repo source and base IDs, base `main`, open/current exact head, branch ref
existence/head, commit existence, and unique commit-to-PR association. A signal
older than 24 hours is rejected. Display names and Git commit author/committer
strings are never identity proofs.

The API's PR association head is authoritative. A different run head is accepted
only if GitHub's commit API confirms a two-parent synthetic merge whose first
parent is the associated base and second parent is that exact PR head. Signal
workflow revision comes from GitHub's authenticated executed-file record and
must match the trusted dispatcher's immutable baseline bytes (receipt v2).
No PR YAML is evaluated. Dispatcher modifications in a PR do not change
default-branch code. See [R1 provenance and limitations](ci-runner-002-r1-provenance.md).
The private side checks all of this again and requires the originating dispatcher
to have successful first-attempt `workflow_run` provenance on `main`; its commit
must be an ancestor of current main, verified with Compare API.

Payload contains only `dispatcher_run`, `signal_run`, `key`. All three are
untrusted hints. The private gate independently rederives PR/SHA/key and downloads
a small, single-JSON **trusted dispatcher receipt**, tied to that actual workflow
run. It never extracts/runs it. Extra/missing fields, malformed types, renamed
workflows/IDs, ambiguous associations, inconsistent or unavailable API data deny.
The dispatch sender must be the configured immutable dispatch App bot user ID.
Runtime hints never select a command or a checkout ref without reauthorization.

## Credentials: one-time operator setup only

Prefer **two narrowly installed Apps**, not a classic PAT:

1. **Dispatch App:** install only on private `ci-runner-pilot`; Metadata read
   (implicit), **Contents write** (GitHub repository-dispatch API requirement).
   No Actions, Administration, Secrets, Workflows or status permission needed.
   Store its private key only in **public OMS** repository secret
   `OMS_DISPATCH_APP_PRIVATE_KEY`; variable `OMS_DISPATCH_APP_ID` is its App ID.
2. **Status/metadata App:** install only on public `ams-gra-codegen-oms`;
   **Actions read, Checks read, Contents read, Pull requests read, Commit statuses write**.
   Store its private key only in **private pilot** secret
   `OMS_STATUS_APP_PRIVATE_KEY`; variable `OMS_STATUS_APP_ID` is its App ID.
   It is not installed on pilot and cannot dispatch or modify pilot contents.

Apps need no webhook receiver. Token creation uses the SHA-pinned
`actions/create-github-app-token` action, explicitly repository-scoped, with
short-lived installation tokens revoked in its post action. Private hosted
coordinators use their normal job-scoped `GITHUB_TOKEN` with Actions read/write
as specified for job/artifact inspection and cancelling only this integration's
superseded runs. No Administration read is requested for runner inventory.
Public metadata reads use a read-only source-repository `GITHUB_TOKEN`.

**No App key, installation token, PAT, dispatch/status token or private checkout
credential is passed to Ryzen.** Its job has `permissions: {}`; it uses public
HTTPS Git fetch and public unauthenticated metadata only. GitHub's intrinsic
job runtime/artifact credentials still exist for Actions artifact upload, and
the implicit token can have unavoidable metadata scope; these are not dispatch,
status, or App credentials and provide no repository contents write privilege.
The trusted control bundle is inline in the reviewed private workflow, not
downloaded using private-repository credentials on the persistent runner.

Use GitHub UI or an operator's secure secret-input channel for keys; do not paste
keys into shell history, PRs, logs or repository files. Rotate by adding a new App
key, replacing the one repository secret, verifying the next automatic event,
then revoking the old key. Disable both activation variables before emergency
rotation; revoke App keys/installations and cancel outstanding integration runs
if compromise is suspected. Masking is not a protection against malicious code.

### Initial activation sequence (after independent review/merge of both PRs)

1. Record explicit acceptance of trusted-code/supply-chain/host-Docker risks.
   Independently review both default-branch implementations and inline bundle.
2. Create/install the two Apps with scopes above; configure their two secrets
   and App ID variables. This task does **not** create/install Apps or secrets.
3. Resolve numeric workflow IDs **after merge**, through the API, not invented
   placeholders:

   ```sh
   gh api repos/zackboll/ams-gra-codegen-oms/actions/workflows/oms-trusted-pr-signal.yml --jq .id
   gh api repos/zackboll/ams-gra-codegen-oms/actions/workflows/oms-ryzen-trusted-dispatch.yml --jq .id
   gh api repos/zackboll/ci-runner-pilot/actions/workflows/oms-trusted-float32.yml --jq .id
   ```

   Set `OMS_SIGNAL_WORKFLOW_ID`, `OMS_DISPATCH_WORKFLOW_ID` in **both** repositories;
   set `OMS_EXECUTOR_WORKFLOW_ID` in **pilot**. Resolve each installed App bot via
   `GET /users/<actual-app-slug>[bot]`; record numeric user IDs, not App IDs.
   Set `OMS_DISPATCH_BOT_USER_ID` in pilot, and `OMS_STATUS_BOT_USER_ID` in **both**.
   Verify all resolved IDs and exact workflow paths. Missing variables fail closed.
4. Verify existing pilot runner `ryzen7945hx-01` online, labels
   `[self-hosted,linux,x64,debian13,ryzen7945hx-test]`, service active and enough
   resources; do not modify its installation, mining or host settings.
5. Set pilot variable `OMS_TRUSTED_PR_DISPATCH_ENABLED` to exact string `true`,
   then OMS variable of the same name to `true`. The variable is an operational
   switch, **not an authorization or isolation boundary**.
6. Open a normal owner-created same-repo PR targeting main. **Do not manually
   dispatch a pilot and call it end-to-end qualification.** Observe the ordinary
   lifecycle below. Subsequent pushes need no additional approval/configuration.

## Automatic end-to-end acceptance procedure

Use a real reviewed owner PR after activation; record source head/tree SHA and
both implementation default-branch SHAs. Observe signal → dispatcher receipt →
repository dispatch → independent hosted authorization → pending `ryzen/float32`
on that exact SHA → full private Float32 job → hosted final status. Verify actual
runner via Jobs API, complete artifact, driver identity JSON, source hashes and
every test family. Record run URLs, counts, CPU/wall/process RSS, memory/swap and
all check conclusions. The status links to the private run (visible to the owner;
public readers may receive 404, but the status description remains visible).

Push a correction while queued/running. Verify the new SHA gets fresh automatic
qualification, old queued work is rejected/cancelled or stops at the next
campaign boundary, and old results never bless the new SHA. Test one duplicate,
fork/external-creator denial, real failure, cancellation, missing artifact and
offline queue deadline. Reconnect/recover and observe subsequent valid SHA pass.
Keep hosted Float32 running throughout. No automatic success is claimed here:
credentials/approval/merged default workflows are absent, so **no live private
qualification was dispatched during implementation**.

## Idempotency, errors and recovery

Identity is `(source repository ID, PR number, exact head SHA, float32-v1)`.
The public dispatcher serializes per PR, reserves a uniquely named artifact
before dispatch, and refuses previous receipts or its designated reporter's
existing `ryzen/float32` status. Private authorization serializes by exact key,
refuses existing execution receipts/statuses, and requires a unique PR association.
GitHub may replace excess pending concurrency entries; only the latest head is
useful, and signal/PR/current-head verification prevents obsolete execution.

Receipts retain 90 days; signals expire after 24 hours, and existing status is
an additional replay marker. First-attempt-only workflow provenance means manual
reruns do not bypass reservation. For a failed exact SHA **push a new corrected
commit**, which automatically produces a new identity. Recovery republishes final
statuses without rerunning tests. Retrying the same head is intentionally not
automatic: after an infrastructure failure use a fresh owner commit; do not
delete receipts/statuses or edit payloads to bypass authorization.

The single installed runner executes one job at a time; no global Actions
concurrency group silently evicts other PRs' queued Float32 jobs. New authorized
heads cancel only older runs of this integration for the same PR. Public/hosted
CI and existing pilot runs are never cancelled by this code. The worker checks
the current head before significant campaigns; the final reporter checks again.

Private hosted recovery runs every 15 minutes and on executor completion. It
uses trusted receipts/API metadata, never executes runner artifacts, chooses the
earliest duplicate delivery, reports `error` for lost dispatch (>15 min), and
cancels/reports work exceeding a three-hour queue+run deadline. Test failure is
`failure`; cancellation/timeout/skips/offline/missing artifacts are never success.
API failures deny dispatch/execution or leave a failed hosted reporter for later
reconciliation; a GitHub-wide outage can delay scheduled recovery. Recovery
scans the latest 100 dispatcher runs and paginates private runs; prolonged outages
or >100 dispatches require operator reconciliation from receipts. It cannot
promise a strict wall-clock bound while GitHub-hosted execution is unavailable.

To disable immediately set **both** activation variables to `false` (public first)
and cancel queued/in-progress `OMS Trusted Float32 Qualification` runs through
GitHub. Disabling does not kill an already authorized job. Revoke Apps for a
security incident. No mining/service stop is necessary. Repository workflow
disable controls are an additional emergency option.

## Validation and future capacity savings

Production authorization functions are tested against API-shaped fixtures, not
independent mock authorization logic. Tests mutate source workflow trust,
identities, event provenance, association count, source SHA, receipts, payloads,
activation and status targets; private tests additionally cover result API job
steps/artifacts, idempotency, recovery and resource preflight. Dispatch/status
API tests issue **no real writes**. Workflow contracts check empty runner
permissions and hosted credential-bearing jobs. YAML and actionlint validation
are separate from live acceptance; no full-job performance is claimed.

Local evidence on 2026-10-24: **20** public fixture/workflow tests and **28** private
tests passed (negative scenarios include multiple parameterized subcases).
Actionlint **1.7.12** passed all workflows in both repositories with shellcheck
and pyflakes integrations disabled; private custom labels are declared in its
actionlint config. All new shell snippets passed `bash -n`; all Python files
parsed. Existing OMS PERF-005 **11 tests**, compiler placement **11 tests** and
split guard/mutation suite **218 checks** passed. The current canonical job and
private snapshot/command inventory compare exactly; mirrored provenance module
bytes match. PyYAML 6.0.2 source archive SHA256 was verified by hash-locked pip
download; it is a validation-only dependency. No live dispatch/status API write,
full Ryzen qualification, performance benchmark or automatic E2E test occurred.

This initially **duplicates hosted Float32 work** and therefore claims no hosted
capacity saving. A later independently reviewed change must establish reliable
required `ryzen/float32` merge gating, hosted fallback for unauthorized PRs,
non-skipped replacement semantics, offline/cancellation/stale behavior, full
compiler/evidence parity and branch protection before skipping/removing hosted
Float32. Other jobs and mandatory merge gates are explicitly out of scope.