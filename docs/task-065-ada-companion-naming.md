# Task 065 — Ada generated artifact naming

## Provenance

Selected main and immutable BEFORE:
`52e22b67b94b431d96925c41ff730749c33b3c9e`.
Independent branch `feature/065-ada-companion-naming`, worktree
`/home/zboll/git/ams-gra-codegen-oms-task065`. Isolated target/TMPDIR:
`/tmp/task065/target`, `/tmp/task065/tmp`; evidence/logs/probes/manifests use
separate directories under `/tmp/task065`.

Origin was fetched before setup. No existing Task065/066 branch, worktree or
PR existed. Task063 PR64 head `ef47893e465bb714cd8c7e151c7a30895ed2142d` and
Task064 PR65 head `308333da38e38728363f9fbae3d594de6ea2ad1b` both had base
`52e22b67b94b431d96925c41ff730749c33b3c9e`; both remain untouched. Task066 is
an independent sibling. No sibling feature commits were copied.

## Exact defect and actual artifact identities

Both independently rehashed pinned roots reproduce:

```text
Ada names "QueryType companion" and "QueryType" both generate "QueryType_Kind" in the generated top-level scope
```

This is **not** an authored type named `QueryType_Kind`. `QueryType` is a
concrete Choice extending abstract empty Record `QueryPET` (2.5 source lines
78902 and 78821). Its Choice discriminant enumeration is `QueryType_Kind`.
The closed-sum wrapper emitted for a value reference to `QueryPET` also
declares a top-level enumeration literal for its concrete descendant
`QueryType`, formerly spelled `QueryType_Kind`. The diagnostic's second label
is the literal's source identity; its owning declaration is `QueryPET`.

Ada case-insensitive comparison is enforced by canonical ASCII-lowercase
identity keys, but the real spelling already matches exactly. Reserved-word
or structural member remapping contributes nothing to this defect.

Pipeline audited: emission selection/name-preflight fallback; generated
declaration registration; Choice/abstract-sum companion registration; Ada
enumeration literal validation; unsafe-declaration owner attribution;
backend_preflight; Ada Choice and abstract-value rendering. Originally the
companion format and literal format were duplicated in the renderer and
planner, not literally obtained from the same naming helper.

The shared mechanism's companion categories are Choice discriminant types
and abstract closed-sum discriminant types. Generated Choice alternative and
closed-sum descendant literals occupy the enclosing package too; unlike types,
enumeration literals can overload each other but not type declarations.

## Narrow generic semantic policy

Companion types remain `{Owner}_Kind`. A closed-sum descendant literal is:

- concrete Choice: `{Descendant}_Choice_Value_Kind`;
- concrete Record: unchanged `{Descendant}_Kind`.

`ada_closed_sum_literal_name` is shared by literal preflight and rendering.
`ada_kind_companion_name` is shared by companion registration and both Ada
renderer paths. The new decision depends only on semantic descendant kind,
not occupancy, traversal, filesystem, message selection, or declaration name.
There is no Query/UCI exception, numeric retry, random suffix, suppression,
or dropped artifact. An authored collision with the fixed semantic spelling
still fails closed under normal Ada canonical comparison.

This repairs the observed intrinsic artifact-category collision, not every
possible adversarial authored top-level identifier. Successful emitted units
remain subject to complete naming preflight.

A broad companion rename was **not** used: the BEFORE inventory measured 57
companion artifact/world instances (45 Choice, 12 sum), across 51 successful
fixture/world cells and 31 fixture files. Raw roots contain 416/420 authored
Choices; only one per release has an abstract ancestor, identified through
structural inspection rather than a name-specific production policy.

## BEFORE freeze

Pinned roots:

- 2.5 commit `093610b7753944059360d3236770ab446d039556`, SHA-256
  `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`.
- 2.6 commit `78eb61b6112c8bffa40820c33124b57787fc5bd9`, SHA-256
  `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

Production preflight: Ada fails as above; Rust/C++ succeed. Ada-only unsafe
declarations are exactly `QueryPET`, `QueryType`. The measured closed-message
gap is exactly 33 in **each** release. Every message's semantic dependency
closure attributes its Ada-only unsafe naming to exactly `QueryPET`.
`tests/fixtures/string/task065-before-naming.tsv` freezes diagnostics,
attribution, coverage, projected-name status and size; the historical Task062
gap fixture remains unchanged.

Exact gap names (same in both releases):

```text
Action
ActionCommand
ActionPlan
ActionPlanCommand
ApprovalPolicy
Authorization
AuthorizationRequest
CommSupport
CommSupportActivity
CommSupportPlan
CommSupportPlanCommand
Effect
EffectCommand
EffectPlan
EffectPlanCommand
MissionPlan
MissionPlanCommand
PlanningFunction
PlanningFunctionSettingsCommand
PlanningFunctionStatus
RequirementOptions
RequirementOptionsCommand
ResponseCommand
ResponsePlan
ResponsePlanCommand
SupportPlan
SupportPlanRequest
SupportRequest
SystemsNeededRequest
Task
TaskCommand
TaskPlan
TaskPlanCommand
```

## Synthetic production proof

`companion-collision.xsd` contains abstract empty `Base`, concrete Choice
`Dispatch`, and `Holder.Item : Base`. Production emission planning creates
both the Choice companion and the closed-sum literal. BEFORE preflight and
Ada renderer fail with the same artifact-category diagnostic, independent
of UCI. Unsafe owners are exactly `Base`, `Dispatch`; peers preflight succeeds.
The pre-fix exact test passed and its original log is retained.

AFTER focused tests cover original/renamed/case spellings, suffix-containing
source identifiers, reversed order, unrelated declarations, unchanged
Record-descendant literal policy, projected subset/full stability, repeated
rendering, planner/render spelling parity, and deliberately occupied fixed
escape/case-insensitive old-policy conflicts that must still fail closed.
GNAT compiles the repaired synthetic variants with `-gnat2022 -gnatwe -gnato`.

## Measured AFTER and distinction from full rendering

The initial complete AFTER campaign executed exactly one pinned test, both
release markers, exit 0, in 381.74 seconds. All 12 cells match the separately
owned `task065-current-coverage.tsv` (not rewritten historical tables).

| Release | World | BEFORE Ada | AFTER Ada | Rust/C++ unchanged |
|---|---|---:|---:|---:|
| 2.5 | closed | 641/722 | 674/722 | 674/722 |
| 2.6 | closed | 646/725 | 679/725 | 679/725 |
| 2.5 | open | 570/722 | 570/722 | 570/722 |
| 2.6 | open | 574/725 | 574/725 | 574/725 |

Unsafe named declarations are empty and all three whole-schema naming
preflights pass in both worlds. Peer-minus-Ada gaps: closed **33 -> 0** per
release; open **0 -> 0**. Ada fully renderable declarations gain one (5532 ->
5533; 5546 -> 5547); fields and kind counts remain unchanged.

Name-safe does not mean the entire UCI schema can be rendered. Other unchanged
semantic blockers remain, including zero-descendant required abstract value
`SourceCommandEXT`, open-world abstract extension points, and the three
deferred Unicode constrained Strings. No general capability expansion is
claimed. Historical Task058/060/061/062 measurements are preserved; only
current executable naming expectations are superseded.

## Smallest real vertical

Measured ranking by selected + generated-support declarations, then lexical
name: `Authorization`, 230 declarations (2.5) / 231 (2.6). BEFORE full-schema
attribution reaches `QueryPET`, while its projected schema was already
name-safe. It is **not** a newly READY projected service. The targeted pinned
gate verifies whole-schema safety, production three-backend service-check,
Ada service-generate and strict GNAT model/API/body compilation.
The extended targeted test **FAILED** after successful production checks and
generation: GNAT rejects `programs-oam.ads:2538:18`, component `CommType` cannot
be used before end of record declaration. An immutable BEFORE control also
generates and fails with the identical diagnostic in UCI 2.5, establishing
that this is not output churn from the companion repair. Completed controls
also reproduce the same BEFORE/AFTER failure in UCI 2.6 at line 2172.
All twelve release/state/backend service-check controls report READY, and all
four Ada service-generation controls succeed; all four GNAT controls fail
with the same `CommType` diagnostic (exit 4). Control runner exit 1 preserves
the failed compiler gate. The complete results are retained at
`/tmp/task065/probes/authorization-control-final/results.json`.
This is an additional component/type-name
compiler blocker, outside the repaired companion/literal category; it is not
an unsupported value, topology failure, or cycle.

**Delivery is blocked.** Do not publish as completed, claim a passing real
vertical, suppress this failure, change the mandated smallest-service choice,
or broaden this task silently into field-identifier remediation. The Deep
wrapper deliberately preserves nonzero compiler status. No push/PR has been
performed while this required gate fails.

## Output stability

Production CLI comparison uses an archive of the immutable parent, an isolated
parent build target, and current CLI. Across 189 repository XSD paths, all
three languages and both worlds (1134 cells):

| Backend | Byte-identical success | New success | Shared failure | Changed/lost success |
|---|---:|---:|---:|---:|
| Ada | 188 | 1 | 189 | 0 |
| Rust | 197 | 0 | 181 | 0 |
| C++ | 198 | 0 | 180 | 0 |

The single new success is the repaired synthetic closed-sum/Choice fixture.
No existing successful output changes, including IPv6, TimeZulu, structured
and alternating String fixtures and existing service/model fixtures. Failed
cells retain both diagnostics; this table does not claim error-text identity.
Durable JSON/hashes: `/tmp/task065/evidence/output-comparison/comparison.json`.

## Delivery validation status

Focused Fast wrapper, repaired synthetic regression and strict GNAT matrix
passed. Fmt, workspace all-target check, warnings-denied all-target Clippy,
affected Rust 1.95 checks passed. The first workspace test build failed when
the 43 GB `/tmp` tmpfs exhausted space (linker bus errors); only Task065's own
rebuildable debug target was removed. Failure logs remain; reduced-debug,
two-job retry completed **exit 0**, including the required GNAT workspace
tests (pinned tests skip when roots are unset, so this does not supersede the
failed exact pinned vertical). CI split and adversarial
split checks passed, as did diff check. The production CLI fixture comparison
passed with zero changed/lost successful outputs. No hosted validation,
commit, push, PR or merge is claimed. Worktrees intentionally retain uncommitted
source and evidence for review; they are **not clean published worktrees**.