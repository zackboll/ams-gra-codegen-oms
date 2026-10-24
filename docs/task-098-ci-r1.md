# Task 098 CI-R1 — stacked-parent reconciliation and hexBinary timeout

## Identity and scope

* Existing PR: #104; branch: `feature/098-ada-time-zulu-codec`.
* Starting HEAD: `67e5b0144ef434c9ba38a128bbd6df482a8b85d2`.
* Fetched and independently verified Task 097 remote HEAD:
  `4d3a1803c3a3c842bcdd9283218506378b7a6c46`.
* Ordinary two-parent merge: `63f7bbea0e1f60e103f40060a7378c0023a65cac`;
  first parent is the starting Task 098 HEAD, second parent is the verified
  Task 097 HEAD. No rebase, reset, amend, cherry-pick or force push.
* The PR remains stacked on `feature/097-ada-float64-codec`. Do not merge
  #104 until parent PR #102 is successfully qualified and integrated.

This repair does not reimplement or modify TimeZulu or Float64. PR #102 and
its worktree are not modified. Current main is not merged into Task 098.

## Original hosted cancellation: exact evidence

Run `39698873849`, job `119563614781` (`ada-hexbinary-codec`), tested the
starting Task 098 HEAD. GitHub reports:

* Started: `2026-10-22T00:20:21Z`.
* Completed: `2026-10-22T00:51:32Z` (31 minutes 11 seconds).
* Overall job conclusion: **cancelled**, not success.
* Exact failure annotation:
  **“The job has exceeded the maximum execution time of 30m0s”**.
* The workflow at that SHA configured `timeout-minutes: 30`.

Every recorded step concluded `success`, including the OAM and qualified
hexBinary consumers, combined Binary/Choice/temporal consumers, portable
helper-demand controls, mandatory evidence validation, and evidence upload.
The qualification artifact was finalized at `00:51:27Z`, before cleanup.
This is an inherited job timeout, **not a TimeZulu codec failure**, and the
cancelled historical run is not counted as successful CI.

Artifact `12392753845`, `task093-ada-hexbinary-codec`, has 325,323,087 bytes
and uploaded ZIP SHA-256
`c0c1303adba7eda2cec51db201d15934394d30a6d26a6c418cd7ce0fb8038359`.
The job also has a non-cancellation warning about Node.js 20 deprecation.

The original job log, steps/metadata, annotations, run metadata, artifact
listing, exact workflow, and downloaded artifact are preserved under
`/home/zboll/task098-r1-scratch/hosted-original/`. Local qualification and
preservation evidence for this repair is under
`/home/zboll/task098-r1-scratch/`. Earlier Task 098 evidence under
`/home/zboll/task098-scratch/` is retained.

## Workflow resolution and temporary stacked correction

The only merge conflict was `.github/workflows/ci.yml`. Resolution retains:

* The updated parent's complete Float64 job, including its verified-checkout
  directory trust and exact-source identity checks.
* Task 098's complete, unchanged TimeZulu GNAT 14/16 matrix job.
* The updated parent's Python model backend job and every inherited Ada and
  Rust job, without changing their qualification commands or artifacts.
* Unchanged workflow permissions, concurrency, triggers and Deep CI.

The updated Task 097 parent still had a 30-minute hexBinary timeout.
Main at inspection (`73b555843dff36c1364674b558241823cfb76375`) already
contains commit `09e01917921844e713fd4063f81644e796393b75`, which changes
only that timeout to 60 minutes. This branch applies the identical narrow
setting to `ada-hexbinary-codec`:

```yaml
ada-hexbinary-codec:
  needs: ada-json-guard
  runs-on: ubuntu-24.04
  timeout-minutes: 60
```

No other timeout, command, qualification case, evidence validation, source,
or artifact changes. This is a temporary stacked-branch correction; when
Task 097 inherits the same setting from main, reconcile the identical value
without introducing duplicate behavior.

## Source and stacked-diff preservation

Byte comparisons confirm all Task 098 production, test, fixture and
qualification files are unchanged from the starting HEAD. Ada support,
Task 097 qualification sources, Task 096 provisioning, model APIs and
temporal/numerical oracles remain unchanged. The inherited Python backend,
workspace manifest and lockfile match the updated parent exactly.

The release generators rebuilt with locked Rust 1.95.0 have the same
SHA-256 identities as the original qualification:

| Generator | SHA-256 |
|---|---|
| Task 098 | `10a82d76a0ef03b35b227924f1596c6366081f392a3f54e18131e97a71839fc3` |
| Updated Task 097 parent | `eeca5d517630c98949b66c445578f38b551198fa27abc16a754eadb7499358c4` |

The final stacked diff against the updated parent contains the original
Task 098 changes, this repair note, and the single hexBinary timeout
correction. It does not duplicate Python or Float64 feature changes. All
parent jobs are text-identical except the explicitly documented timeout;
the TimeZulu job is text-identical to the starting Task 098 job.

The repeated 464-case generation/rejection preservation sweep against the
updated parent passed: 458 identical outcomes (336 successful generated
trees and 122 rejections). The six new successful outcomes are the expected
named-TimeZulu codec admissions. Six still-rejected Time-bearing cases
advance to an existing non-Time first blocker; all other 116 rejection
diagnostics are identical. Rust, C++, Ada model/API and Time-free outputs
remain byte-identical. Every rejected generation remains atomic.

## Qualification and disposition

Exact commands, stdout/stderr, exit records and result manifests are
retained in the repair evidence directory. Compiler campaigns reuse the
previously task-owned Alire 2.1.1 setups (GNAT 14.2.0 external and GNAT
16.1.0 native, GPRBUILD 26.0.1, GNATCOLL 26.0.0), revalidate all recorded
setup identities, verify compiler versions, and build with private provider
copies. Test temporary files use task-owned disk-backed storage.

The locked Rust 1.95.0 gates passed: formatting (`fmt --all -- --check`),
workspace/all-target `check`, Clippy with `-D warnings`, and workspace/all-target
tests (**1,449 passed, 0 failed, 8 ignored**). `check`, Clippy, tests and both
generator builds use `--locked`. CI split, layout and mutation controls pass,
including the 210-check split self-test and the 54-case layout controls.
An auxiliary scratch-driver invocation omitted the layout checker's required
workflow path and raised `IndexError`; its log is preserved in
`rust-gates/ci-layout.stderr`. The independently recorded correct invocation
(`python3 scripts/check-task077b-ci-layout.py .github/workflows/deep-ci.yml`)
and layout mutation tests both pass in `ci-controls/`. This was a driver
argument error, not a repository or codec failure.

Focused generated TimeZulu qualification passes under both GNAT 14.2.0 and
GNAT 16.1.0: 18 cells per compiler, covering both OAM and qualified
namespaces, portable/Float32/lossless profiles, and Normal/Check/Ignore.
Every cell executes 156 documents: 38 accepted and 118 rejected. Each
compiler records 57 successful qualification steps. Generated models,
Service_API and codecs are actually compiled, and model/API byte parity,
JSON kinds, lexical preservation and all expected negative cases pass.
The aggregate local verifier passed and records its results in
`/home/zboll/task098-r1-scratch/local-verification.json`.

Real DLZ qualification passes for GNAT 14.2.0 and GNAT 16.1.0, UCI 2.5 and
2.6, and Normal/Check/Ignore: 12 cells, each with 26 documents (10 accepted,
16 rejected). Both releases are lossless READY at 53/53, versus the parent's
52/53 TimeType blocker. Portable and Float32 still reject Float64; rejected
generation remains atomic. Generated model/API bytes equal the updated
parent's model-only output. Time values preserve their JSON string spelling,
and Float64 values remain JSON numbers with exact binary64 values.

No full local Float64 numerical corpus is repeated: production sources,
runtime dependencies, provisioning and the parent generator are unchanged.
The inherited hosted Float64 GNAT 14/16 jobs remain mandatory and intact.

Hosted readiness requires new exact-final-head CI and Deep CI runs, every
mandatory individual job successful (including TimeZulu 14/16, Float64
14/16 and hexBinary), successful overall workflows, and no merge conflicts.
Historical cancelled runs never satisfy that requirement. Auto-merge must
remain disabled; neither PR is approved or merged by this repair.