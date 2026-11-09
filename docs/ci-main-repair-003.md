# CI-MAIN-REPAIR-003: isolated Deep worker Git trust

Starting main: `43fa1934235ae1f4c818a586a5022ddd7366b8d1`.

## Cause and correction

The Deep jobs establish `safe.directory` in the job user's global Git config.
Each worker then changes `HOME` to its private Alire setup home. The original
Task100–106 integration-identity commands invoke plain Git, which no longer sees
that global config and rejects the container checkout's ownership.

`worker_git_env` adds protected command-scope configuration to the private
subprocess environment using Git's `GIT_CONFIG_COUNT` / `KEY_n` / `VALUE_n`
interface. It retains existing entries and worker settings, then resets the
inherited safe-directory list and adds only `ROOT.resolve(strict=True)`.
Before returning that environment it checks Git's top-level path, HEAD and tree
against the verified orchestration identity. Missing, nested, wrong-identity and
wildcard paths fail closed. Ambiguous legacy `GIT_CONFIG_PARAMETERS` overrides
are rejected rather than allowing them to override the narrow trust policy.

No global or system configuration is written. There is no wildcard trust or
disabled ownership check. The unmodified shell commands and their descendants
inherit the worker environment. Errors still produce a failed worker completion
and propagate through the existing mandatory pack gate.

## Starting-main inventory reconciliation

Local source checks found two stale qualification-input hashes on starting main:
PR #204 had already changed `oms_lacal-transports-websocket.adb` and `.ads` for
caller deadlines. The inventory is reconciled to those exact starting-main
bytes, with a separate source-bound `qualification_updates` record. Historical
input identities remain intact. This repair changes no production files and
does not treat previous execution receipts as qualification of those updates.

## Historical hosted evidence (not fresh qualification)

Downloaded original pack diagnostic artifacts:

| Run | Artifact ID | Pack/compiler | Source SHA | Source tree |
| --- | --- | --- | --- | --- |
| 42529289943 | 13639188597 | A/14 | `2a8f0b5ad0f9bf9f544f0bf04556a583b77d4893` | `ef9ca4bb3569a781d4d33f44cf40163588d14119` |
| 42529289943 | 13638734968 | A/16 | `2a8f0b5ad0f9bf9f544f0bf04556a583b77d4893` | `ef9ca4bb3569a781d4d33f44cf40163588d14119` |
| 42529289943 | 13639837435 | B/14 | `2a8f0b5ad0f9bf9f544f0bf04556a583b77d4893` | `ef9ca4bb3569a781d4d33f44cf40163588d14119` |
| 42510678267 | 13647043670 | B/16 | `55b4cf1a35c7fc9dd1008deff28fea76a41ad39d` | `71245afe416697b424b25647937fa0bf1cd3f836` |

In each A artifact, all four suite-execution steps returned zero. In each B
artifact, all three suite-execution steps and the Task104 inventory step returned
zero. Each subsequent identity shell step returned 1 with Git's ownership error
and a Python traceback reporting Git exit 128. The pack remained failed; its
independent verification was not successful qualification.

## Focused local validation

- `python3 -m unittest discover -s scripts -p test_ada_deep_pack.py -v`:
  20 tests passed, including frozen original command hashes and all 330 current
  qualification-input hashes.
- Real ownership mismatch, without host root:
  `unshare --user --map-auto --map-root-user env -u SUDO_UID ADA_DEEP_TEST_CHOWN=1 PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$PWD/scripts" python3 -m unittest test_ada_deep_pack.WorkerGitTrust -v`:
  6 tests passed. Fixture checkout and `.git` ownership are changed to mapped
  UID/GID 1; worker commands run as mapped UID 0 without `SUDO_UID`.
- Both ownership-test modes execute all seven original integration-identity
  shell commands for compiler settings 14 and 16, through the real subprocess
  executor and CPU affinity. They verify HEAD, tree, status, receipt fields and
  rejection of a second untrusted repository. Compiler campaigns and successful
  evidence verification are mocked only inside this focused identity test;
  incomplete evidence is then rejected by the real suite verifiers.
- `test_task10*_evidence.py`: 47 tests run, 4 skipped; all others passed.
- `test_ada_ci_pack.py`: 20 passed. `test_owned_qualification.py`: 8 passed.
- `scripts/ci002` tests: 30 passed. `test_pr_smoke.py`: 40 passed.
- Deep workflow layout source validation and actionlint 1.7.12 passed.

Original suites, command bodies, environments, compiler pins, artifact patterns
and timeout budgets are unchanged. No long compiler campaign or full main run
was dispatched for this repair. Normal post-merge main CI must establish final
integration success; independent review is still required.