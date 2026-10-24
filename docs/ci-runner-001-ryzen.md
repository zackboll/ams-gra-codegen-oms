# CI-RUNNER-001 — staged Ryzen integration

## Disposition: SECURITY_APPROVAL_REQUIRED

Inspection on 2026-10-23 started from `origin/main` commit
`73b555843dff36c1364674b558241823cfb76375`. No runner migration or automatic
self-hosted routing has been enabled. XMRig and its configuration were untouched.

### Deployment boundary — operator action required

OMS is **public**, permits forks, and has no repository runners. Its fork approval
policy is `first_time_contributors`; default workflow token permissions are read
only. The main protection API returned `Branch not protected` (404), and the
ruleset list was empty. These observations are not permission to weaken policy;
no repository policy was changed.

Authorized SSH from the development host to `ryzen7945hx` works as `zboll`.
The existing runner is `ci-runner`, a member of `docker`, with a persistent
`/opt/actions-runner/_work`. Service inspection reports `ProtectHome=no`,
`ProtectSystem=no`, `PrivateDevices=no`, `PrivateNetwork=no`, and
`NoNewPrivileges=no`. The real pilot smoke log confirms that even the pinned job
container mounts **the host `/var/run/docker.sock`**. Container pinning provides
reproducibility, not isolation from this host.

KVM modules and `/dev/kvm` exist. No installed QEMU/libvirt/cloud-image tooling
or existing VM image directory was found on this machine. The authorized SSH
account's `sudo -n -l` requires a password. A disposable VM boundary therefore
could not be established with the available noninteractive access. Host RAM was
13 GiB usable, approximately 9 GiB available, swap 0/976 MiB used, with existing
huge-page reservations; none were changed.

**Minimum next operator action:** authorize and provision a disposable CI VM on
this same physical machine, and provide an authorized administrative path for
its setup. It must have its own Docker daemon, no host socket/directory or
credential mounts, no access to mining/management services or private network
credentials, and externally enforced network restrictions. Destroy/recreate the
guest after each job; an ephemeral runner registration alone does not clean a
persistent filesystem. Preserve the installed host runner software directory;
do not start a second physical runner or run two listeners for this capacity.
Record the image, lifecycle, network restrictions, credential handling, and an
escape/access-negative test before approving public-repository registration.

Alternatively the operator must explicitly approve a documented deployment
boundary and its residual public-repository compromise risk. **A manual input,
same-repository PR test, custom label, approval setting, or repository variable
is not that boundary.** An attacker can change/add workflow YAML to target a
registered public-repository runner. Merely setting the variable below is not
adequate security approval. Automated untrusted/fork execution must stay hosted;
any future automatic trusted route needs separate review of trust enforcement
outside editable PR YAML. Never use `pull_request_target` to run PR source.

Reference: <https://docs.github.com/en/actions/reference/security/secure-use>.

## Repository changes

Only `CI / ada-float32-codec` has an optional manual runner selector. It retains
its job identity, 90-minute limit, Debian digest, Rust 1.95.0, verified Alire
2.1.1, GNAT 14.2.0/16.1.0, provider pins, assertion policies, corpus, composition,
rounding, naming regressions, and the existing evidence artifact. No assertions
or workload commands were removed. Cargo uses four workers; the three existing
Ada harnesses accept `OMS_CI_ADA_BUILD_JOBS=4` and pass `-j4` to gprbuild through
Alire's supported `build --` interface. Without the variable, they use one
worker. Values outside 1–4 fail; no 16/32-worker benchmarking is introduced.

| Event | Selected runner |
| --- | --- |
| Any PR, including fork and same-repository PR | `ubuntu-24.04` |
| Push to main | `ubuntu-24.04` |
| Dispatch with input omitted/false | `ubuntu-24.04` |
| Dispatch with approval variable absent/not `true` | `ubuntu-24.04` |
| Explicit qualification dispatch AND approved variable `true` | `[self-hosted, linux, x64, ryzen7945hx-test, oms-ci-trusted]` |

`OMS_RYZEN_MANUAL_APPROVED` was absent and has **not** been set. Normal checks
therefore cannot queue forever on a missing Ryzen runner through this intended
path. GitHub does not dynamically fall back for a queued self-hosted job; manual
qualification requires an operator online/idle check first. Job timeouts do not
bound queue time. Delete the variable after the single qualification dispatch;
this affects future runs, not runs already queued. If the runner goes offline,
diagnose the one manual run rather than repeatedly dispatching replacements.

All other Fast CI runner assignments and all of Deep CI are unchanged. Fast/deep
events, dependencies, concurrency and check names remain intact. There are no
Windows/macOS changes. Token permission remains `contents: read`, no secrets are
referenced, and Float32 checkout no longer persists its credential. Measurement
records contain the checked-in step commands, not environment dumps or tokens.

## Registration handoff (only after security approval)

Current registration remains `https://github.com/zackboll/ci-runner-pilot`:
`ryzen7945hx-01`, version 2.338.0, labels `debian13,ryzen7945hx-test` plus GitHub's
default labels. Service
`actions.runner.zackboll-ci-runner-pilot.ryzen7945hx-01.service` is active/running.
GitHub reported it online and idle before and after the smoke test.

1. Recheck GitHub's pilot runner API and local service logs for idle status:
   `gh api repos/zackboll/ci-runner-pilot/actions/runners`.
   Stop if `busy` is true; do not forcibly remove an active runner.
2. On the host, with authorized administrator access, gracefully stop the old
   service with `sudo systemctl stop actions.runner.zackboll-ci-runner-pilot.ryzen7945hx-01.service`.
   In `/opt/actions-runner`, use `sudo ./svc.sh uninstall`.
3. From authorized GitHub CLI access obtain the short-lived token via
   `POST repos/zackboll/ci-runner-pilot/actions/runners/remove-token`.
   With shell tracing disabled, supply it to supported `./config.sh remove
   --unattended --token "$REMOVE_TOKEN"` as `ci-runner`. Do not log tokens,
   paste them in a PR, or save them in repository files; unset immediately.
4. Preserve `/opt/actions-runner` and its installed software. Implement the
   approved VM lifecycle before enabling any OMS listener. The host's persistent
   installation must not remain an additional registered public-repo listener.
5. Obtain the short-lived registration token with
   `POST repos/zackboll/ams-gra-codegen-oms/actions/runners/registration-token`.
   In the approved execution boundary, configure as `ci-runner` with
   `./config.sh --unattended --url https://github.com/zackboll/ams-gra-codegen-oms
   --token "$REGISTRATION_TOKEN" --name ryzen7945hx-01
   --labels debian13,ryzen7945hx-test,oms-ci-trusted --work _work`.
   Do not use `--replace` to bypass an existing listener. Unset the token.
   A disposable per-job deployment additionally needs `--ephemeral` and an
   external provision/teardown controller; do not install a permanent guest
   service and call that disposable. Reconcile this lifecycle with service
   installation in the approved design before activation.
6. For an explicitly approved persistent deployment, install/start with
   `sudo ./svc.sh install ci-runner` and `sudo ./svc.sh start`, then verify the
   generated OMS systemd unit is enabled and GitHub reports online/idle. For
   disposable deployment verify the controller service and one-job lifecycle
   instead. No such service or controller has been installed by this change.

No removal/registration token was requested during this task; its security gate
precedes migration. GitHub access can inspect runners; token issuance was not
tested while blocked. Do not print token-bearing API responses.

## Full qualification procedure

Once the reviewed workflow is available for dispatch (GitHub requires the
workflow on the default branch), inspect the exact reviewed branch SHA and
record it. Do not merge this PR merely to claim qualification; the current
security-blocked handoff does not require a self-hosted dispatch.

After the deployment boundary is approved and the runner is online/idle:

```sh
gh api repos/zackboll/ams-gra-codegen-oms/actions/runners \
  --jq '.runners[] | {name,status,busy,labels}'
gh variable set OMS_RYZEN_MANUAL_APPROVED --body true \
  --repo zackboll/ams-gra-codegen-oms
# Use the reviewed branch/ref; verify it has not advanced before dispatch.
gh workflow run ci.yml --ref ci/ci-runner-001-ryzen-integration \
  --repo zackboll/ams-gra-codegen-oms -F ryzen_qualification=true
gh variable delete OMS_RYZEN_MANUAL_APPROVED \
  --repo zackboll/ams-gra-codegen-oms
```

This runs the **complete existing Float32 job**, not a smaller pilot substitute.
Other CI jobs also run on their original hosted assignments; dispatch once.
The first measured step asserts exact checkout SHA and, when requested, runner
name `ryzen7945hx-01`. Download `task095-platform-qualified-float32` and verify:

- `ci-runner001/identity.json`: source SHA, workflow SHA, URL, runner, architecture,
  four Cargo workers. Compare to the reviewed head and GitHub run/job metadata.
- Each `ci-runner001/step-*/resources.txt`: user/system CPU seconds, wall seconds,
  maximum process RSS in KiB and exit status, with step source/timestamps.
  GNU time's maximum RSS is a measured process high-water mark, **not summed
  concurrent memory**. Add CPU/wall values only with that scope understood.
- Memory before/after snapshots: `MemAvailable`, `SwapTotal`, `SwapFree`.
  These are Linux `/proc/meminfo` snapshots, not continuous VM peak/host swap
  monitoring. Collect hypervisor/host samples separately for isolated deployment.
- Every pre-existing `.exit`, result JSON cell, source hash, compiler identity,
  assertion-policy run and completion marker. Count executed tests/checks from
  their actual evidence, not from a pilot's Rust test count. The naming matrix
  alone has 120 executions; do not misreport that as the complete Float32 count.
- Artifact presence and GitHub job/check conclusion `success` on this exact head.

## Evidence collected during staging

| Evidence | Observed result |
| --- | --- |
| New real pilot smoke | <https://github.com/zackboll/ci-runner-pilot/actions/runs/39911737787> — success |
| Smoke exact workflow commit | `ebe5423aa3850003a48a8119447553bde3baedf6` (pilot, **not OMS**) |
| Smoke runner | `ryzen7945hx-01` for both jobs |
| Smoke scope/time | host/Docker job 3s; pinned container job 4s, glibc 2.41, x86-64 |
| Smoke resources | 32 logical CPUs, 9.0 GiB available memory, zero used swap |
| Smoke workload count/CPU/RSS | no OMS tests; CPU time and maximum RSS not collected by existing smoke |
| Exact baseline main hosted Float32 | <https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39893142529/job/120217308874> — success, 49m17s, commit `73b555843dff36c1364674b558241823cfb76375` |
| Baseline artifact verified via API | `task095-platform-qualified-float32`, ID `12477805153`, 773,571,552 bytes, not expired; contents not downloaded |
| Recent PR hosted Float32 | <https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39903862338/job/120251232094> — success, 39m06s, commit `fce503a4ef55fa1c3d87f133831e8245b9c36567` |
| OMS self-hosted full job | **Not executed: security gate blocks registration** |

Neither the smoke nor the previously reported 366-test Rust pilot qualifies
the full OMS Float32 job. No speedup or final exact-head success is claimed.

Local staging validation uses actionlint 1.7.12 (release archive checksum
verified), PyYAML parsing, `git diff --check`, the new routing/measurement tests,
the existing split guard/mutation suite, and compiler-placement regressions.
The new tests evaluate 80 event/input/approval combinations, including PRs with
both opt-in signals present, reject routing-condition mutations, exercise real
shell success/failure/pipefail and metrics, and verify bounded Ada build commands.
The hosted Rust job installs GNU `time` to run those tests; Float32 installs it
inside its pinned container. Local systems need GNU `time` on `PATH` too.

Completed locally: all three workflow YAML files parsed; actionlint exited 0
(shellcheck/pyflakes integrations disabled); the new eight-test suite and all
11 compiler-placement tests passed. The existing split guard passed, and its
adversarial suite passed **218 checks** (including two layout ownership tests).
Baseline comparison confirmed all original Float32 workload command bodies,
job identities, unrelated runner assignments, and the unchanged Deep workflow.

Additional **development-host** execution (not Ryzen qualification):

- `CARGO_BUILD_JOBS=4 cargo +1.95.0 build --locked -p ams-gra-codegen-oms`
  passed: 5.29s wall, 13.14s user + 2.50s system CPU, maximum process RSS
  416,928 KiB. This reused local build state, so it is not a clean-build benchmark.
- The real `check-task095-rounding.py` harness passed on x86-64/glibc 2.41:
  32 rounding pairs × 57 checks, four C2 modes totalling 137 checks, and 20
  bridge checks = **1,981 C checks**, with every recorded subprocess exit zero,
  completed result JSON and verified source hashes. 0.39s wall, 0.30s user +
  0.07s system CPU, maximum process RSS 33,940 KiB. This is only the independent
  C subset, not generated-Ada or full Float32 qualification.
- GNU time reported zero process swaps for both commands; this does not assert
  zero machine-wide swap usage on the development host. Ryzen host snapshots
  separately showed zero used swap and 9,498,060 KiB available memory after smoke.

Local raw logs are under `/tmp/ci-runner001-validation` on the development host
(temporary, not durable CI artifacts). The PR's hosted checks remain the source
of exact-final-head hosted conclusions; no successful final-head run is implied
by this staging evidence.

## Backlog impact

At inspection the other open PRs were #109, #104 and #102:

- #109: `real-uci` and `real-uci-regressions` in progress; Float32 already passed.
- #104: those Deep jobs plus Float32, repeated-temporal and hexbinary in progress.
- #102: queued Float32, Float64 (14/16), repeated-choice-real-uci, JSON guard,
  real-sleet and hexbinary-real; other Rust/Deep work in progress. Dependent
  codec checks also wait for their existing JSON guard dependency.

This is a point-in-time snapshot, not an assertion that these checks remain
queued. Existing runs use their original workflow revision and cannot migrate
mid-queue. New normal PR runs **also remain hosted in this staged change**.
Immediate backlog relief is therefore **zero until security approval,
registration, full qualification and a separately reviewed automatic trusted
routing decision**. Once enabled safely, only future eligible Float32 jobs
benefit directly; freeing their hosted slots can help other checks indirectly.
The one Ryzen runner processes one job at a time. No runs were cancelled or
expensive OMS workflows repeatedly retriggered.