# CI-SELECT-003 — Python-owned qualification without workflow edits

## Scope

CI-SELECT-003 installs fixed, fail-closed Python qualification hooks in the
existing Fast Python, pinned-UCI, and pinned-Sleet jobs. Future Python features
can add flat, component-owned test files without modifying CI YAML or scripts:

- python/qualification/fast/test_*.py — both Python Fast matrix cells, normal
  and optimized; no Deep selection from this path alone
- python/qualification/uci/test_*.py — selected real-uci-regressions Deep job,
  after the pinned UCI roots/release generator have been verified
- python/qualification/sleet/test_*.py — selected real-sleet Deep job, reusing
  the one verified pinned broker build for both Python versions and policies
- crates/cli/examples/python_*.rs — Python evidence-only CLI examples compile
  through Rust/Python ordinary gates and select exactly three Deep owners:
  `msrv-real-uci`, `real-sleet`, and `real-uci-regressions`

For Python CLI examples, MSRV retains Rust all-target compilation and the Python
UCI regression job owns release example execution. The Sleet owner preserves the
inherited PY-009 Python broker qualification, including the datetime example's
contract. This bounded set does not select unrelated Ada or other UCI Deep jobs.

Every mode keeps Rust and both existing Python ordinary cells. The Deep job
selection retains dependency closure. Neither the workflow inventory, lane
budget, matrix, runner selection, permissions nor required artifact names change.

## Fail-closed source and test discovery

The checked-in runner requires at least one test per registered flat file. It
rejects unknown Python modules, symlinks, subdirectories, duplicate test IDs,
import failures and empty suites. Bytecode creation is disabled so the normal
and optimized runs do not introduce unregistered __pycache__ directories.

Pinned-UCI mode requires the two existing UCI roots and release generator; its
versioned test inventory receipt goes in the existing py006-uci artifact.
Pinned-Sleet mode requires the existing AMS_GRA_SLEET_BIN; it never downloads,
rebuilds or edits Sleet. Test failures propagate as nonzero job exits.

Only exact test_*.py paths within those three flat mode roots (and verified
python_*.rs evidence examples) receive narrow ownership. Unrecognized helpers,
the runner itself, scripts/**, workflow/security, frontend/IR/shared generator,
Cargo, toolchains, cross-language fixtures, deletions, renames and nonregular
files remain fail-broad. Main push/manual/nightly continue to run full CI.

## Inherited CI protection

CI-SELECT-003 adds exactly four reviewed, ordered path rules. The original
CI-SELECT-002 rules are still checked against their immutable historical
digest. The original PY-008/PY-009 payload and source contracts are checked
after removing ONLY these three mandatory, location-verified hook additions.
No source identity, artifact provenance, final gate, security, concurrency or
five-lane guard is waived.

The new rule and hook negative controls exercise path ownership, selected and
skipped Deep matrices, main/full fallbacks, deletion/type/rename/unknown paths,
test-discovery completeness, hook removal, and relocation into wrong jobs.

## Delivery and outstanding qualification

This infrastructure PR changes CI workflows and scripts and therefore requires
full hosted CI at its own exact head. Review hosted Fast/Deep, CI002, required
final gate and all source-bound artifacts before independent review.

After exact-head green acceptance, use disposable no-merge PRs against the
candidate branch for Fast-only, UCI-only, Sleet-only, Python evidence-example,
documentation-only, infrastructure-full and selected-failure controls. Record
actual selected/skipped jobs, runner allocation, artifacts and runner-minutes.
Local selection simulation alone does not establish hosted selectivity savings.

This change does not retroactively narrow the open PY-010 PR #132, whose YAML
and shared script changes independently require full CI. Future Python features
must actually use the fixed component-owned entrypoints to obtain selectivity.
Unknown or shared surfaces remain full qualification.

Disposition: HOSTED_CI_PENDING until evidence and independent review complete.
No merge, approval, auto-merge, rebase or force push.
