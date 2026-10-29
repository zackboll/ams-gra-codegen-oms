# Task100 current-main integration (PR #118)

## Source and scope

The existing branch is `feature/100-ada-websocket-transport`. Integration
started from PR head `904ad3baebbf237a91e118c91457ac7207e8edd9`, tree
`3f197e88cc79c65389ed7c78f4f0020ff79a3a35`, with a clean worktree.
Fetched current main is `ac046599c492241aefc9b1d6b742668c0e9cd39a`.
The original incorporated main was `4978c1c5728d062b9ea64bbcdb39aa96b18361a3`.

Use an ordinary merge commit, not a rebase or replacement PR. The two workflow
conflicts retain current main's CI-SELECT-001 orchestration, CI002 security,
five common lanes, reusable Deep invocation and all three PERF-007 primary
real-UCI shards. Task100's two original compiler-matrix jobs are additive.
No production Ada transport or original Task099/Task100 harness is changed.
Loopback-only, non-TLS, synthetic-schema interoperability remains the scope;
this integration makes no remote/TLS or real-UCI transport claim.

The original Task100 document's hosted-source and scheduling descriptions are
historical: this amendment supersedes raw-PR checkout with synthetic-merge
checkout, adds shared lanes, and adds mandatory artifact reconciliation.

## Independent review resolutions

The first independent read-only technical review found no blocking file-content
defect, but required completing conflict resolution and staging all imported
modules before committing. The index must be audited against reviewed files;
generated Python bytecode is retained outside the worktree, not committed.

Actionable CI findings are addressed: CI002's exact seven-file source inventory
cannot be removed or weakened; local case identities and Float64/resource/
partial-send/ownership markers are verified, not merely counts; pinned
dependency LICENSE bytes and recorded source hashes are checked; per-execution
Sleet source identities are uploaded. Original dependency-source metadata now
includes shared generator/CLI/schema/provisioning inputs.

The follow-up independent review reported no blocking technical finding. Its
additional hardening suggestions were applied: all 17 selected upstream source
files must appear in the provenance map; lifecycle requires the full 30-cycle
marker; Sleet's exact tree and binary-hash record are required. Manual hashing
confirmed both pinned LICENSE values. The review incorrectly stated AHT had no
`LICENSES` directory: Git inspection confirms `LICENSES/Apache-2.0.txt`, which
is now explicitly hash-verified and included by the campaign text upload.
Binary SHA-256 records are also uploaded. Synthetic test license values test
the rejection logic; real local evidence was separately reconciled against the
actual constants, not treated as hosted source acceptance.

The reviewer also identified original transport behavior worth making explicit.
Any failed Open drops the socket, including an already-open-client misuse;
this is retained as fail-closed behavior, not silently changed to preserve a
session. Private writes rely on the verified pinned encoders' one-based arrays.
Small/default Ada task stacks are not qualified for multiple maximum-message
temporaries. Poll copying is bounded per call, not linear over a full message.
Task099 Connection finalization invokes bounded Close, unlike direct Client
finalization; Connection.Send stops on all transport exceptions. These limits
are clarified in the transport qualification document without changing protocol
behavior to accommodate CI.

Artifact reconciliation intentionally rejects duplicate names, including stale
earlier-attempt artifacts if GitHub returns them. A rerun is not allowed to
choose favorable evidence ambiguously; use a fresh complete run/revision rather
than delete evidence or bypass a failure. This conservative availability limit
also exists in inherited PERF-007 reconciliation. Python assertions and the
Sleet ephemeral-port reservation race are inherited harness limitations; the
mandatory hosted matrix and final gate independently recheck completion.

## Historical evidence (not integration acceptance)

- [Ordinary run 40451020872](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40451020872):
  original 20/20 successful jobs.
- [Deep run 40451020802](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40451020802):
  original 10/10 successful jobs.
- Original artifacts remain published:
  `task100-ada-websocket-transport-14`, `task100-ada-websocket-transport-16`,
  `task100-ada-websocket-sleet-14`, `task100-ada-websocket-sleet-16`.

These qualify only the original revision. Do not count them as final-head
acceptance. Retain their source/run identities and all failed local attempts.

## Selection and evidence contracts

Both ordinary cells check out the synthetic PR merge through `github.sha`.
Deep uses the existing `workflow_call` dependency with the same source.
WebSocket fixtures, companion projects, runtime interfaces, shared OMS JSON
support and qualification scripts select the affected downstream campaigns.
Unknown transport roots and all infrastructure changes fail broad. Workflow
changes in this PR require complete ordinary and Deep qualification.

The final `pr-ci-required` job depends on the ordinary transport matrix and
the reusable Deep result. It independently requires each authoritative matrix
cell, successful mandatory steps, and unique fresh nonempty artifacts. It
also downloads all selected Task100 bundles and reconciles source SHA/tree,
run/attempt, exact compiler, all original 24 cells per compiler, local suite
counts, Float64 probes and six pinned-Sleet executions per Deep compiler.
Original qualification steps are digest-pinned separately from the additive
integration-identity step. Missing, duplicate, stale, failed, skipped and
cancelled qualification must fail, not degrade to a successful placeholder.

## Runner inventory and lane assignment

The unchanged main inventory contains 36 expanded runner cells. The two
Task100 compiler matrices add four: **40 expanded cells total**, including
one PR-only signal cell and **39 main-eligible cells**. The reusable caller
itself consumes no runner. The count includes Pages, CI002 and the disabled
trusted dispatcher; it is an inventory, not the number executing in one run.

Actual historical hosted runtimes below use first job-step start through job
completion, not a queued/pending job's `started_at`:

| New cell | Run | Job ID | Minutes | Main lane |
| --- | --- | --- | ---: | --- |
| transport GNAT14 | 40451020872 | 122013509731 | 10.38 | 2 |
| transport GNAT16 | 40451020872 | 122013509728 | 10.92 | 4 |
| real Sleet GNAT14 | 40451020802 | 122013509451 | 8.42 | 4 |
| real Sleet GNAT16 | 40451020802 | 122013509585 | 16.10 | 2 |

Preserve every existing lane assignment. Starting with the reviewed
`docs/evidence/perf-007-lane-model.json` candidate loads, the modeled totals
after these additions are 118.00, 128.42, 124.08, 134.53 and 115.47 minutes
for lanes 1 through 5. The primary shards remain on lanes 1/3/4. The new
cells use 2/4 to avoid additional load on the heavier primary-shard 1/3
paths and orchestration/final-gate lane 5. Ordinary Task100 depends only on
`changes`; Deep Task100 depends only on the reusable invocation. No new job
holds a lane while awaiting another lane.

This model includes the earlier illustrative shard setup assumptions and is
**not observed final-head performance or a makespan guarantee**. Every new
cell uses the existing shared job-level `queue: max` group. Main runs do not
cancel; PR groups include workflow, run, attempt and compiler identity.
The actual post-merge aggregate audit must use runner step timestamps across
all participating main-eligible workflows: at most five executing jobs and
one per lane. Pending/queued jobs are not executing jobs.

## Acceptance remains fail-closed

Local reports, independent technical review and hosted final-head evidence
are distinct requirements. Record the final raw PR head, synthetic merge
SHA/tree, run IDs and attempts in the PR handoff after publication; source
changes require new qualification. Do not merge until independent review,
the complete selected hosted graph, CI002, Trusted PR Signal, all four
Task100 artifacts and `pr-ci-required` pass for the exact final source.
Post-merge main qualification and the observed five-runner audit are required
before reporting `COMPLETE`.