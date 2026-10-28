# CI-RUNNER-002-R1 provenance and receipt v2

## Verified starting identities

Fetched before edits on 2026-10-24:

| Repository | Feature head | Main |
| --- | --- | --- |
| OMS | `4c79fff41970a8ba7109df9bd185a6b166c0ffb0` | `a03684eb963e1eaa8a9df3db17c9d1783b7add03` |
| Pilot | `d5f7aac7f34f0f298804b32dbd9abb54d595b5c7` | `ebe5423aa3850003a48a8119447553bde3baedf6` |

## R1-01: independently checked installer pin

The official Rustup book documents version-specific archives and `.sha256`:
<https://rust-lang.github.io/rustup/installation/other.html#manual-installation>.

Reviewed artifact:
<https://static.rust-lang.org/rustup/archive/1.28.2/x86_64-unknown-linux-gnu/rustup-init>

Official checksum source (review-time only, never fetched to select trust at runtime):
<https://static.rust-lang.org/rustup/archive/1.28.2/x86_64-unknown-linux-gnu/rustup-init.sha256>

Expected SHA-256:
`20a06e644b0d9bd2fbdbfd52d42540bdde820ea7df86e92e533c073da0cdd43c`.

Independently cross-checked against Signal's immutable source pin:
<https://github.com/signalapp/libsignal/blob/fc90b90fa2b9c37822fa47ba0e0dcdf387a0c071/node/Dockerfile>.
That Dockerfile pins the same archived URL and checksum. The downloaded binary
was also hashed locally and matched. Rustup upstream tag `1.28.2` resolves to
commit `e4f3ad6f893e56ca10f0a329c401a5eafab69607`; that tag is unsigned, so it
is not claimed as a cryptographic release signature.

The driver downloads only this HTTPS URL into a fresh job-local temporary
directory, bounds the download, rejects redirects/empty/truncated/altered bytes,
and verifies the reviewed checksum before giving the binary execute permission
or invoking it. There is no shell bootstrap, mutable installer, fallback, or
runtime checksum trust decision. HOME, CARGO_HOME and RUSTUP_HOME must resolve
inside the unique job directory. Upstream override variables are removed.

Rust/Cargo are selected explicitly with `+1.95.0`, checked against full versions:
`rustc 1.95.0 (59807616e 2026-04-14)` and
`cargo 1.95.0 (f2d3ce0bd 2026-03-21)`.
These agree with the official 1.95.0 channel manifest and existing local tools.
The installer and toolchain are not installed globally. Toolchain downloads
retain Rustup's upstream checksum trust model; this change pins the bootstrap,
not a new sandbox against compromised authorized source or a compromised host.

## R1-02: run-bound immutable workflow evidence

GitHub documents `pull_request` GITHUB_SHA as the merge commit and GITHUB_REF as
`refs/pull/N/merge`, while `github.workflow_sha` identifies the workflow file's
commit and `github.workflow_ref` its repository/path/ref:

- <https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request>
- <https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#github-context>

Neither a caller-provided context dump nor REST run.head_sha is sufficient
workflow-file evidence. The current merge ref can change after a run.

Read-only inspection of successful signal run **40108707517**, workflow
**395223123**, attempt **1**, PR **113** (ID **4996824906**) established:

- REST head_sha: `4c79fff41970a8ba7109df9bd185a6b166c0ffb0`.
- Associated PR head: the same SHA; associated base:
  `a849b87d94948980133038ba909c0b6528cadde6`.
- GitHub GraphQL `CheckSuite.workflowRun.file` returns the executed workflow
  repository/path and `repositoryFileUrl`, here an immutable `/blob/4c79fff.../`
  URL, **not a synthetic merge revision**. Run ID/attempt/event/workflow ID are
  checked in the same authenticated GraphQL record.
- Current PR merge SHA at inspection was `3440e5afdc1d2674c44ae344de0f9539b5d2387e`,
  with parents `a03684e...` and `4c79fff...`; it differs from the historical base.
  It must not substitute for the original executed revision.

The gate now uses that GitHub-authenticated executed-file record. If its revision
is not the associated PR head, it must be an available two-parent commit with
parents exactly `[associated base, associated head]`. Bytes at that exact
revision must equal the signal bytes at the independently authenticated
default-branch dispatcher's exact head SHA. No arbitrary source YAML is parsed
or executed. The dispatcher has its own executed-file proof, default-main ref,
ancestry, owner actors, workflow ID/path, event and first-attempt checks.

**Older-branch limitation:** the historical observed run does not demonstrate
GitHub selecting a merge workflow for a head missing the file. A conditional
merge-shaped regression proves the gate accepts only authenticated merge-file
evidence with exact parents and baseline bytes. An older branch without such
evidence remains denied. Do not claim live older-branch compatibility from that
fixture. Missing GraphQL provenance or historical file content is fail-closed.

Receipt **version 2** adds source repository ID/name, immutable PR ID, signal
path/attempt, associated base/head, effective workflow/merge SHA, trusted baseline
SHA, file Git blob ID and SHA-256, dispatcher workflow ID/path/attempt/SHA and
file identity. The dispatch payload stays minimal (`dispatcher_run`,
`signal_run`, `key`); it supplies no trusted revision. Reauthorization/reporting/
recovery rederive all fields from the original run and compare the whole receipt.
v1 receipts fail closed and are not silently upgraded. Replay keys remain v1 to
avoid bypassing an existing exact-SHA ledger.

Main is still consulted only for the dispatcher's existing ancestry trust check,
not current workflow bytes. Ordinary main advancement/trusted workflow updates
therefore do not change a historical receipt. Rewritten/divergent ancestry still
denies. Stale heads never succeed; publication is bound to the original head/key.

## R1-03: repository isolation

Repository `1430048464` must remain exactly `zackboll/ci-runner-pilot`, default
branch `main`, `private is True`, and `visibility == "private"`. Missing, string,
conflicting, internal, or public visibility evidence denies. This is checked
before public handoff (including immediately before POST), private preparation,
reporting and recovery, including error-reporting fallback. Source repository
identity checks remain separate; OMS is not required to be private.

No registration, credentials, approval variables, host configuration, mining,
Docker, services or runner installation is changed. Automatic dispatch and
private OMS execution remain disabled. Full Float32 snapshot/steps are unchanged.
Review and hosted validation are not activation or end-to-end Ryzen qualification.

The staged hosted source-metadata permission declaration adds **Checks read**
for the GraphQL CheckSuite/WorkflowRun query; it does not add a write permission.
Any future operator-created source App needs that read scope too. No App or
credential was created or modified. Hosted qualification checks out and verifies
the exact PR head, exercises the historical GraphQL query with a job token, and
performs a real verified installer/toolchain bootstrap in a temporary directory
on hosted Ubuntu. The latter does not dispatch to the persistent runner.