# Task 065 — Ada generated artifact naming

## Historical first-phase provenance

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

## Historical first-phase smallest real vertical and review stop

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

**At the first-phase review stop, delivery was blocked.** Do not publish as completed, claim a passing real
vertical, suppress this failure, change the mandated smallest-service choice,
or broaden this task silently into field-identifier remediation. The Deep
wrapper deliberately preserves nonzero compiler status. No push/PR has been
performed while this required gate fails.

## Historical original naming output stability

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

## Historical first-phase validation status

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

## Task063 main reconciliation

The resumed session verified the existing branch/worktree, fetched origin,
inspected tracked/staged/untracked changes and durable evidence, and found no
other process working in the Task065 directory. Every source byte matched the
retained first-phase manifest (`5f5b029bcf7dc27d16b1c7ad154d90a370a114fc46308047a9e7df4dfc0883ff`).
Backups include tracked and untracked source content:

- `/tmp/task065/pre-main-reconcile.patch`, SHA256
  `531c749786f4b705c782193b8092ec00fc8ef5b9b5551980261689ed88ced3f2`;
- `/tmp/task065/pre-main-reconcile-status.txt`;
- `/tmp/task065/pre-main-reconcile-manifest.txt`, SHA256
  `269ff2bc43fac76b954ce03a96b40064893cf101f55e56534178458f633f61bd`.

Normal local first-phase checkpoint:
`d29a27d3f1f0e869c55b04b03d4ea282cdadbe39`.
Normal main reconciliation:
`ad2ffac9df3bd1ad8f9094c0f061e88bb1dad798`.
Integration base: `4fd8492b3603125e6ebe11e04c43fe68d85cf231`.
Immutable BEFORE and original integration base remain
`52e22b67b94b431d96925c41ff730749c33b3c9e`.
No rebase, reset, destructive stash, replacement worktree or cherry-pick was
used. Shared CI/docs/current-coverage conflicts were composed to retain both
Task063 patterned-integral support and Task065 naming repair. The current
coverage oracle composes Task063's frozen delta with Task065's frozen isolated
AFTER; neither historical ledger was rewritten. Integrated naming Fast gates
passed before implementing the component repair.

## Pre-existing CommType compiler blocker

The resumed task explicitly authorizes this separate correctness fix. It is
**not** a capability gain attributable to the Query closed-sum literal rename.
Both pinned releases select `Authorization`; model package `Programs.Oam`;
owner `CommSupportType`. The exact old component is:

```ada
   type CommSupportType is record
      CapabilityType : CommSupportEnum;
      CommType : CommType;
   end record;
```

Its earlier named target declaration is:

```ada
   type CommType is record
      CapabilityType : CommCapabilityEnum;
      CommTypeVariant : CommType_CommTypeVariant_Sequence;
   end record;
```

The immutable-parent, first-phase and pre-component-fix integrated controls
all fail identically:

```text
programs-oam.ads:2538:18: error: component "CommType" cannot be used before end of record declaration
programs-oam.ads:2172:18: error: component "CommType" cannot be used before end of record declaration
```

The first line is UCI2.5, the second UCI2.6. Exact generated sources and logs
are retained under `/tmp/task065/probes/authorization-control-final` and
`/tmp/task065/probes/authorization-integrated-control`. A pre-fix integrated
production CLI is frozen at
`/tmp/task065/probes/component-before/pre-fix-cli`.

## Generic component/type hiding policy

One private renderer helper, `ada_component_type`, derives the ordinary field
base subtype. Only a **named** target whose final identifier is Ada-equivalent
(`eq_ignore_ascii_case`, valid generated Ada identifiers are ASCII) to the
actual component identifier, a preceding emitted component, or the generated
discriminant receives its model package prefix. GNAT proves that a preceding
`Foo` component also hides a later `Other : Foo`, including across Choice
variants; reversing those declarations does not hide the earlier subtype mark.
Layout comes
from `package_name` / shared `ada_model_package`; no namespace is hardcoded.
GNAT also proves a library-package prefix can itself be hidden when a named
target/component equals that prefix (`Test : Test.Shadow.Test`). Only in that
case the expanded mark is anchored at `Standard`, giving
`Test : Standard.Test.Shadow.Test`. Real `Programs.Oam.CommType` bytes stay
unchanged. This is the same component-name-resolution class, not a field rename.
All other subtype strings remain unchanged. Public field and wire names remain
unchanged; Task054 remapping is untouched. No helper artifact or public API is
introduced. No new preflight rejection is needed: these are legal Ada records,
not duplicate top-level declarations.

```ada
      CommType : Programs.Oam.CommType;
```

The audit covers required Record fields, Choice variant components and inherited
effective members. Optional helpers use the same function with their actual
internal `Value` component, protecting `Value : Value` too. The outer optional
component refers to a distinct `{Owner}_{Field}_Optional` subtype. Bounded
storage introduces `{Owner}_{Field}_Item`, arrays and slot types; its `Value`,
`Items`, `Count` components cannot equal those generated subtype identifiers.
Unbounded storage uses distinct item subtypes and already-expanded vector
marks; generic instantiation formals are associations, not component names.
Closed sums use `{Descendant}_Value : Descendant`, necessarily distinct.
Other helper records use expanded `Binary_Vectors.Vector`, private carrier
representations and distinct storage names. Service API payload subtypes are
already model-qualified and the API does not render schema-member records.
Primitive subtype spelling is deliberately unchanged.

### Compiler-backed minimal reproduction and controls

Production XSD generation before the fix emits `Foo : Foo;` / `fOO : Foo;` in
Record and Choice. All four fail actual GNAT with the same component diagnostic.
Changing only the subtype mark to `Test.Shadow.Foo` compiles under
`-gnat2022 -gnatwe`. Exact old production sources/logs are frozen under
`/tmp/task065/probes/component-before` before the renderer edit.
The regression then plants the old subtype back into repaired generated files
and requires all four failures again.

Fast exact tests in `ada_component_hiding`:

- `task065_component_hiding_compile_matrix`: 23 successful cells (Record and
  inherited Record: required/optional/bounded/unbounded, each exact/mixed-case;
  Choice: required/bounded/unbounded, each exact/mixed-case; internal optional
  `Value` hiding). Optional named Choice is not admitted by the existing
  backend and is not broadened here.
- `task065_component_hiding_planted_failure_controls`: four failed controls.
- `task065_component_nonhiding_output_stability`: two successful controls.
- `task065_preceding_component_hiding_controls`: four successful cells with
  four planted failures (Record/Choice, exact/mixed-case).
- `task065_discriminant_hiding_controls`: Choice `Kind` and optional-wrapper
  `Is_Present` named targets, plus planted unqualified failures.
- `task065_package_prefix_hiding_control`: one strict successful cell and
  planted ambiguous-prefix failure.

Specs receive strict GNAT semantic compilation (`-gnatc` when a body exists);
generated bodies receive strict object compilation. No missing-body-code
diagnostic is mistaken for a generated-language defect.

## Authorization BEFORE/AFTER compile evidence

Integrated pre-fix versus repaired production CLI: both releases report READY
in all three language service checks and both Ada generation phases succeed.
BEFORE GNAT fails (exit 4) at the exact lines above; AFTER model/API succeeds
(exit 0), and each generated Ada body also compiles with
`-gnat2022 -gnatwe -gnato`. The historical control script intentionally exits 1
because it preserves BEFORE failures; its results ledger, not that aggregate
exit, establishes the AFTER success. Authorization was already projected-name-
safe on immutable BEFORE and is never described as newly READY from the
closed-sum companion repair.

## Real hiding inventory

The production effective-member/name inventory is frozen in
`tests/fixtures/string/task065-component-hiding.tsv`: 21 UCI2.5 and 22 UCI2.6
schema-name/target-name relations, all Record fields (no real Choice relation).
14 / 15 use required immediate named storage and need qualification; seven
per release are optional or repeated controls whose helper type avoids the
outer-component hiding. `CommSupportTaskType.CommType` is inherited.
UCI2.6 additionally has `VehicleFormationType.VehicleFormationEnum`.
The complete sorted fixture records owner, final component, target, cardinality
and inherited status, not just CommType witnesses. Its SHA256 is
`479f78f3f5cf0943c2e5da5b01bd3e8f00e330568f3f0f3686b8972bdbfcf9b7`.
Authorization reaches two relations in each release: required
`CommSupportType.CommType` and optional
`EntityPositionType.FixedPositionType`. The latter does not need qualification
because its immediate storage is a generated optional helper. Projected
selected/support classification is printed separately by the compact gate.
This compact inventory creates no API churn: every original component stays
unchanged, only necessary subtype marks are disambiguated.

### Separate component-fix output ledger

Pre-component-fix integrated CLI versus repaired CLI, 191 repository XSDs plus
both real Authorization projections, three backends and two worlds: **1,158
cells**. This is not the original 1,134-cell naming ledger.

| Backend | Byte-identical success | Qualification-only success | Shared failure | Lost/new statuses |
|---|---:|---:|---:|---:|
| Ada | 189 | 7 | 190 | 0 |
| Rust | 204 | 0 | 182 | 0 |
| C++ | 205 | 0 | 181 | 0 |

Every changed file is an Ada specification; all component identifiers and
every other byte are preserved. Exactly these successful cells change:

- `annotations.xsd`, both worlds: `Count : Urn.Annotations.Count`;
- `codec-oam.xsd`, closed: `Level : Programs.Oam.Level` and
  `Percent : Programs.Oam.Percent`;
- Authorization, both releases/worlds: `CommType : Programs.Oam.CommType`.

All successful Rust/C++ files are byte-identical; Ada bodies and service API
files are also byte-identical. The comparator verifies that removing only
expanded subtype marks whose terminal identifier is Ada-equivalent to the
unchanged component produces the exact old source. No status change is hidden.
Results and exact diff: initial scope campaign
`/tmp/task065/evidence/component-output-comparison/comparison.json` and
`qualification-only.diff`; final renderer campaign
`/tmp/task065/evidence/component-output-comparison-certified/comparison.json`.
Both ledgers have identical SHA256
`afd3e604addc4ecc8de02b8d69cc664467c4b36e17d33e7fd135192efb0bac6e`.
Runner:
`scripts/compare-task065-component-outputs.py`.

## Integrated current coverage

Fresh production analysis after normal Task063 reconciliation (base
`4fd8492b3603125e6ebe11e04c43fe68d85cf231`), separately from the immutable
Task065 naming campaign:

| Release | World | Ada | Rust | C++ | Peer-minus-Ada gap |
|---|---|---:|---:|---:|---:|
| 2.5 | closed | 676 | 676 | 676 | 0 |
| 2.5 | open | 571 | 571 | 571 | 0 |
| 2.6 | closed | 679 | 679 | 679 | 0 |
| 2.6 | open | 574 | 574 | 574 | 0 |

UCI2.5 fully renderable declarations: 5534 closed / 5446 open on all
backends; UCI2.6: 5547 / 5459. Total declarations, fields, occurrence and
reference metrics are unchanged. Task063's +2 closed/+1 open UCI2.5 capability
gain is not attributed to naming or the component compiler repair.
The original isolated `641 -> 674` and `646 -> 679`, each gap `33 -> 0`, stay
historical. Whole-schema name safety is not a claim that unrelated abstract
value/Unicode/topology blockers now render.

## Final local validation

The resumed complete-source Fast wrapper executes **nine exact one-test gates**
(three original naming gates and six component/scope gates), all passed with
GNAT required. Fmt check, workspace/all-target check, warnings-denied Clippy,
CI split and adversarial split (**137 checks**) passed. The final renderer's
`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` completed exit 0, 145 suites /
1279 reported test passes; real pinned tests without roots remain developer
skips and are not counted as real campaigns. Task054 member remapping,
generated-name preflight, optional/repeated/inherited storage, Choice and
Task060/061/062/063 compiler/carrier regressions ran in that workspace gate.
Rust 1.95 all-target and Task065 focused checks use
`/tmp/task065/msrv-target`, separate from `/tmp/task065/target`.
Resumed logs use `complete-*` / `certified-*` names and never overwrite the
historical phase-one logs. No sibling artifact cleanup was performed.

The compact pinned gate checks all 33 historical former-gap projections per
release, name safety and production generation feasibility for each READY
candidate, then ranks selected + support declarations with lexical tie-break.
Authorization remains minimum (230 / 231); its generated model/body and actual
CLI service API receive strict GNAT compilation. Larger candidates cannot
outrank this compiler-clean minimum, so their large object compilations are
not duplicated in Deep. `Task` itself remains deferred-value blocked; no
unsupported schema is relabeled READY.

The strengthened compact pinned wrapper completed exit 0 in 675.81 seconds:
`/tmp/task065/logs/certified-pinned.log`. Both roots passed exact original
collision attribution, frozen hiding inventory, current full-schema naming
preflight, zero gap in both worlds, 33 former-gap projection checks per release,
and strict Authorization model/API/body compilation. Selected hiding owners
`CommSupportType` and `EntityPositionType` are selected, not generated support,
in both Authorization projections. The final package-prefix safeguard does
not affect these real outputs; the final production CLI comparison independently
confirms unchanged seven-cell subtype-only delta.

## Final source manifest and publication state

Final source identity: `c884bf58b669f4be56dc7a4d65f4d93aa2eede97909c4df246dedbef49cdbe69`.
Manifest: `/tmp/task065/manifests/final-source.json`, including tracked files,
staged/unstaged patch hashes and untracked source/evidence input identities.
The canonical source identity normalizes only this self-reference to the
literal `TASK065_FINAL_SOURCE_IDENTITY`; every file remains represented, and
the manifest separately retains raw file hashes. The original phase-one
manifest remains untouched. This avoids a circular document/manifest digest.

Local required gates passed. Publication/hosted certification are separate:
this document does not claim final hosted SUCCESS before the head/base-keyed
collector records it. The Task065 PR must remain open, non-draft, unmerged,
with auto-merge disabled. No Task064/066 worktree, branch or process was changed;
read-only PR checks still showed both siblings open at their existing heads.