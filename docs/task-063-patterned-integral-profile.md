# Task 063 — exact patterned integral value-space profile

## Isolation and immutable measurement identity

Task 063 was created as a sibling of Task 062, not its descendant. Actual Task 061 parent
and immutable BEFORE benchmark: `add348ce867daf4253a88510a65a48a3ed23f218`.
At creation PR #62 was OPEN, base `main`, at that SHA; PR #63 was OPEN,
base `feature/061-time-zulu-support`, head
`8765b455235197eb533ba3d89c8a0522cf81dd60`. This agent merges neither PR.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task063`; scratch, targets,
temporary files, schema copies and logs: `/tmp/task063/`.

## Pre-production evidence gate

The isolated schema copies match pinned root SHA-256 digests:

|Release|Root SHA-256|Declaration line|Immediate base|Authored facets|
|---|---|---:|---|---|
|2.5|`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`|145855|`xs:int`|minInclusive 1, maxInclusive 999, pattern `[0-9]{1,3}`|
|2.6|`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`|146103|`xs:int`|minInclusive 1, maxInclusive 999; no pattern|

Raw restriction-chain inventory finds exactly one patterned integral declaration
in 2.5 (`USMTF_SerialNumberOfQualifierType`) and zero in 2.6. There is one
restriction-level pattern group with one alternative, no explicit whiteSpace,
length or exclusive facets. The effective primitive is SignedInteger, whose
intrinsic whitespace is fixed collapse. The xs:int intrinsic bounds are narrowed
by the authored inclusive bounds.

Authority: [XML Schema 1.0 Part 2, second edition](https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/),
§3.3.13.1–2 (integer lexical/canonical representations), §4.3.4 (pattern),
§4.3.6 (whiteSpace), §4.3.7–10 (bounds), Appendix F (anchored regex language).
Pattern evaluation follows whitespace normalization. `[0-9]` is ASCII digits,
not Unicode digit classes, and `{1,3}` matches the whole normalized literal.

The normalized lexical space is the set of one-to-three ASCII digit strings
whose integer value is 1..999. Thus `001` is admitted, `+1` is not; `000`,
`-1` and `1000` are not admitted. Multiple spellings denote the same value.
For EVERY integer n in 1..999, ordinary decimal spelling of n has 1–3 digits,
no sign, and denotes n, so is schema-valid. Conversely every valid spelling
denotes a value in 1..999. The image of this lexical space is therefore EXACTLY
1..999. This is a **value-space lowering**, not XML lexical round-trip support.
Existing integral APIs take values, unlike spelling-preserving String/temporal
carriers; no original XML spelling is required or reconstructed.

Task 050's pinned OMSC-SPC-013 Rev B §6.1.4 item 3 maps XSD integers to JSON
numbers (authority identity retained in `docs/task-050-rust-oms-json-codecs.md`).
Production `dec_i64` calls `serde_json::Number::as_i64`; named decoding calls
the generated checked constructor. The actual locked serde_json 1.0.151 probe
before production changes establishes:

|Raw JSON|as_i64|Exact profile result|
|---|---|---|
|1, 999|Some(1), Some(999)|accept|
|0, 1000, -1|Some(0), Some(1000), Some(-1)|constructor rejects|
|1.0, 1e0, -0|None (floating representation)|integer decoder rejects|
|+1, 001|parser error|cannot reach decoder|

Canonical integer JSON emission for 1..999 has the same unsigned decimal
spelling as a schema-valid XML representative. The lexical facet is NOT
generically discarded: only this exact evidence-bounded profile is recognized.

## Shared capability scope

The Task 050 authority was re-fetched at its pinned commit: official DOCX and
Markdown hashes match the historical authority digests; §6.1.4 item 3 was
inspected again. This is not a new OMS mapping assumption.

`PatternedIntegralProfile::DecimalDigits1To3Range1To999` is name-free. A separate
`named_integral_domain` composes it with existing ordinary bounded capability.
Task 020's `inclusive_integral_domain` is unchanged and still rejects all
lexical constraints on direct fields. Coverage/readiness and all backends use
the shared named result. Rust/C++ retain checked numeric factories and numeric
accessors; Ada retains its existing constrained numeric representation. No
String storage, regex runtime, XML parser or codec-side range duplication.

## Validation status

Local implementation, compiler/codec/readiness, measured impact, fixture and
workspace gates pass. Task 061's Deep run completed successfully and PR #62
was merged externally during validation; Task 063 is reconciled to merged main
as documented below. No Task 063 hosted result is claimed before publication.
Historical Tasks 020/061/062 evidence is not overwritten.

## Measured inventory and coverage

Normalized IR confirms both immediate bases, complete effective facets and
SignedInteger classification. The sole declared reference is the optional
`USMTF_MessageIdentificationType.QualifierSerialNumber` (2.5 line 107083;
2.6 line 107521). Effective inherited-member inventory finds the same single
reference: no additional inherited reference, repeated direct use or Choice
alternative. It reaches two message closures through nested records. Neither
release has a generated-support-only reaching service in either world.

Frozen BEFORE figures come from the executable built at the immutable parent,
not Task 062. Counts below are measured; kind, field-type and occurrence metrics
are unchanged in every cell. The complete five-metric BEFORE/AFTER ledger is
`tests/fixtures/integral/task063-coverage.tsv`.

|Release|World|Backend|Fully renderable declarations BEFORE → AFTER|Message closures BEFORE → AFTER|
|---|---|---|---:|---:|
|2.5|closed|Ada|5531 → 5532|632 → 634|
|2.5|closed|Rust|5532 → 5533|664 → 666|
|2.5|closed|C++|5532 → 5533|664 → 666|
|2.5|open|Ada|5444 → 5445|566 → 567|
|2.5|open|Rust|5444 → 5445|566 → 567|
|2.5|open|C++|5444 → 5445|566 → 567|
|2.6|closed|Ada|5545 → 5545|637 → 637|
|2.6|closed|Rust|5546 → 5546|669 → 669|
|2.6|closed|C++|5546 → 5546|669 → 669|
|2.6|open|Ada|5458 → 5458|570 → 570|
|2.6|open|Rust|5458 → 5458|570 → 570|
|2.6|open|C++|5458 → 5458|570 → 570|

Kind renderability is 5553/5557 in 2.5 and 5566/5570 in 2.6, unchanged.
Both field metrics are 13160/13160 in 2.5 and 13198/13198 in 2.6, unchanged.
The exact patterned declaration gains capability, not a new primitive kind.

## Real service impact and smallest vertical

Every reaching service was checked using both parent and Task 063 production
`service-check`, across both releases, worlds and backends. UCI 2.5 gains:

* `OrdersMetadata`: newly READY in closed and open, all three backends;
  82 selected declarations, zero generated support.
* `PrioritizationList`: newly READY in closed, all three backends;
  351 selected plus 13 support declarations.
* Open `PrioritizationList`: unchanged projection/topology failure at
  `DataLinkIdentifierPET`, not an integral capability failure. No scope widening.

Exact nine `(release, world, backend, message)` tuples are frozen in
`tests/fixtures/integral/task063-newly-ready.tsv`; dropping world yields exactly
six unique `(release, backend, message)` tuples. UCI 2.6 gains none: both
services were already READY where projection permits them. No other reaching
services remain blocked by a second declaration in this measured inventory.

The deterministic smallest newly READY service is **OrdersMetadata** (82 versus
364 declarations). Production check/generate and compiled Ada model/body/API,
strict C++17 model/API, and Rust model/API/codec all passed for BOTH releases.
Real Rust payload exercises the optional qualifier serial inside
`MessageData.OrdersAssociation[0].Orders.ATO[0].Identifier`. Values 1 and 999
round-trip as numbers; 0, 1000, -1, 1.0, 1e0 and -0 reject. No handwritten
codec range predicate was added. The six backend/release markers completed in
137.65 seconds for the first complete vertical; the full pinned inventory and
readiness campaign completed in 228.89 seconds. These are local timings, not
hosted runner estimates. Task 061's 240-minute Deep timeout is unchanged.

## Synthetic and regression evidence

The synthetic schema admits `Serial` and `Renamed` without name checks. Neighbors
fail closed: min 0, max 1000, `[0-9]{1,4}`, `[1-9][0-9]{0,2}`, `[0-9]+`, two
alternatives, two groups, explicit whiteSpace (including collapse), exclusive
bounds, exact/min/max length, unsigned primitive, and direct field-local patterns
in both Record and Choice. A changed profile carrying the real declaration name
remains unsupported. PatternDialect currently has only XmlSchema, so an alien
dialect is not constructible in this IR; equality is still explicit in admission.

Production projection proves selected declarations renderable with an exact
support-only patterned integer READY; changing its minimum to 0 makes the same
selection NOT READY. Actual generated Ada/Rust/C++ programs accept 1, 9, 10, 99,
100 and 999 and reject 0, 1000, -1 and signed host extrema. One deliberately
wrong expectation per backend fails, followed by a restored successful execution.
Rust Copy/Eq/Ord/Hash and numeric access are exercised; C++ numeric copies and
factories and Ada numeric conversion/range checks are exercised.

The generated Rust synthetic codec validates required, optional, bounded repeated,
unbounded repeated and Choice integer use. Actual serde_json syntax and integer
representation assertions include floats, exponents, negative zero and invalid
JSON `+1`/`001`. Mock OWP routes valid boundaries through the typed handler and
publisher and reports decode errors for 0/1000/-1 without handler delivery.

Across all **189 XSD fixtures × 2 worlds × 3 backends = 1134 cells**, parent/final
generation comparison found **575 identical successes, 550 shared failures,
9 new successes, zero changed successes, zero regressions**. The two new fixtures
account for all nine gains: the abstract-bearing patterned model succeeds in
closed only (three); the codec model succeeds in both worlds (six). All
pre-existing integral, String, temporal and Binary successful files stay byte
identical. Patterned versus ordinary bounded synthetic source is itself byte
identical, across all backends. Per-backend/world fixture classifications:

|Backend/world|Identical success|Shared failure|New success|Changed success|Regression|
|---|---:|---:|---:|---:|---:|
|Ada closed|101|86|2|0|0|
|Ada open|85|103|1|0|0|
|Rust closed|105|82|2|0|0|
|Rust open|89|99|1|0|0|
|C++ closed|106|81|2|0|0|
|C++ open|89|99|1|0|0|

Long inherited OrderOfBattle/SMTI and DLZ campaigns are not rerun solely because
an integral classifier was added: the original classifier and direct-field
rendering/codec paths are unchanged, existing profile inputs do not match the
new classifier, producer hashes for String/temporal/scanners and corresponding
fixture bytes remain unchanged. The full workspace gate still runs their local
regressions; no historical pinned evidence is replaced by an unexecuted claim.

Dependency audit identified two inherited pinned impact assertions that freeze
PrioritizationList's old serial blocker (Task 061 Time impact and the Task 060
subset under Task 061). Their historical TSVs and counts remain untouched.
Current-source assertions explicitly require the exact UCI2.5 closed-world
PrioritizationList gain; every other historical readiness/blocker assertion
remains exact. The affected pinned impact tests are rerun, not the unaffected
long compiler verticals. These edits belong only to Task 063's worktree.

## CI and local environment controls

Fast runs seven exact synthetic/compiler/codec/readiness tests and three exact
ordinary lexical fail-closed controls. Registered names were inspected first.
Each invocation requires one nonzero test and prints captured diagnostics before
propagating command failure. Compiler probes include completion markers. The
wrapper has five adversarial controls (nonzero command, zero tests, wrong name,
missing marker, pass). Deep runs mandatory pinned inventory/coverage/readiness,
both-release smallest service vertical and all nine production CLI confirmations.

An initial workspace build encountered full shared `/tmp` tmpfs. Only Task 063's
own targets and compiler temporary directory were moved to
`/home/zboll/git/ams-gra-codegen-oms-task063-artifacts/`; symlinks retain the
`/tmp/task063` interface. No other task's target was cleaned or moved. Separate
Rust 1.95 target directories avoid toolchain/rustdoc contamination. A subsequent
workspace attempt detected an ordinary-profile diagnostic change; the backend
preflight bypass was narrowed to exact admission and all three old lexical tests
pass again. The final full workspace gate is tracked separately from these
failed attempts: exit 0, **1258 passed, zero failed, zero ignored** (140 test
summary blocks), with GNAT required. fmt, workspace all-target check,
warnings-denied Clippy, Rust 1.95.0 locked workspace/all-target check, both split
controls (125 adversarial checks), five wrapper controls and diff whitespace
checks pass. The inherited Task 061 exact Fast compiler/codec/readiness/mock-OWP
wrapper also passes.

Rust 1.95.0 also executes all five new CLI synthetic/capability/compiler tests
and the exact generated numeric codec test successfully, in its own target.

UCI 2.6 OrdersMetadata model/API files (all backends) and the Rust codec are
byte-identical to parent-generated source; frozen SHA-256 controls live in
`tests/fixtures/integral/task063-uci26-source-digests.tsv` and are enforced by the
pinned vertical. Original and corrected fixture comparisons both completed
with the same 575/550/9/0/0 classification. Publication waits for final source
identity and branch/PR state checks, not represented as completed by local tests.

## Parent merge reconciliation

PR #62 was merged externally at `2026-10-04T04:36:32Z` as
`cf9ebc518f49e58b1731417186c836f3a41e0e74`. Its second parent is the exact
immutable Task 061 benchmark; both trees are
`f0451bd2e04da6f298fab754b23afe1ad390e721`. `git diff` between benchmark and
merge is empty. Task 063 was fast-forwarded onto that merge while retaining
its isolated working changes. Thus **final integration base is merged main
`cf9ebc5…`**, but **immutable BEFORE remains `add348ce…`**. This ancestry-only
reconciliation invalidates no source-derived measurements.

Task 062 head `8765b4…` is not an ancestor. The shared merge-base with Task 062
remains `add348ce…`; no IPv6 commits/source were imported. PR #63 was still
OPEN at reconciliation. Task 061's associated-head Deep run `37169015690`
completed successfully before Task 063 publication consideration, satisfying
the wait-for-an-existing-Deep-completion condition. Any Task 063 PR must now
target `main`, not either task feature branch.

## PR #64 temp-directory corrective: preserved failure provenance

Reviewed previous head: `a0b7930d24ac3a963917685a81fd1a17a42b4a89`.
At corrective start, local HEAD, the remote Task 063 branch and PR #64 head
all matched that identity; the worktree was clean and `origin/main` remained
`cf9ebc518f49e58b1731417186c836f3a41e0e74`. No existing corrective was present.

* Fast CI `37178325975`, attempt 1: **SUCCESS**.
* Deep CI `37178326029`, attempt 1: **FAILURE**; `real-sleet` and
  `msrv-real-uci` succeeded. Failing job: `real-uci`, `111365537692`.
  Its actual Actions checkout was the synthetic PR merge
  `abb6d18906c5388574b62d93a620ef2ec7b0bdc4`, not the branch head or main base.
* The Task 061 Named TimeZulu step succeeded. Within Task 063, both
  `task063_pinned_inventory_coverage_and_service_impact` and
  `task063_smallest_real_service_compiler_codec_vertical` passed, with both
  release inventory/impact markers and all six Ada/Rust/C++ OrdersMetadata
  compiler markers. The subsequent Python helper crashed at line 11:
  `scratch = Path(os.environ["TMPDIR"]) / "task063-service-check"`, with
  **`KeyError: 'TMPDIR'`**. GitHub Actions did not define `TMPDIR`.

This is a helper/CI portability defect, not a patterned-integral production or
measurement failure. The old run was not rerun or replaced. Original run/job
metadata and the complete failing job log were retained outside the checkout
in `/var/tmp/task063-temp-corrective-evidence/` (`old-fast.json`, `old-deep.json`,
`old-real-uci-api.log`). A lightweight `env -u TMPDIR` load of the original
helper reproduced the exact exception before launching any service-check.

The correction adds one Python fallback policy: **`TMPDIR` → `RUNNER_TEMP` →
`tempfile.gettempdir()`**, retaining the `task063-service-check` subdirectory.
Execution is guarded by `main()` so importing `scratch_root()` launches no
campaign. All nine production checks, the six unique tuple assertion, threaded
execution, diagnostics and failure propagation remain intact. The pinned shell
wrapper is unchanged: locked fetch, exact names, one-test checks, release
markers and original error propagation are preserved. No workflow, production
code or measured fixture changes are needed.

The wrapper adversarial regression retains all five original checks and adds
three controlled-environment checks: explicit `TMPDIR` wins even with
`RUNNER_TEMP` present; absent `TMPDIR` selects `RUNNER_TEMP`; both absent select
Python's platform temp root. A subprocess guard rejects accidental service-check
execution during these tests. All eight checks passed. Separate lightweight
`env -u TMPDIR` probes established scratch directories successfully with
`RUNNER_TEMP` set and with both variables unset, without real UCI execution.
The latter selected `/tmp/task063-service-check` on this host via Python, not
a hardcoded Linux path.

Focused local gates passed: the Task 063 Fast wrapper, Python wrapper regression,
affected Python `py_compile`, `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets`, warnings-denied workspace/all-target
Clippy, `check-ci-split.sh`, all 125 `test-check-ci-split.sh` controls and
`git diff --check`. The Fast invocation inherited pinned roots and consequently
also generated real runtime-facade models. An unfinished workspace check with
that unnecessarily broad environment was stopped; the successful workspace
check and Clippy ran without UCI roots. No pinned Task 063 campaign was stopped
or repeated. Targets and compiler scratch remain Task 063-specific under
`/home/zboll/git/ams-gra-codegen-oms-task063-artifacts/`; corrective logs are in
the separate evidence directory above.

The real `check-task063-pinned.sh` campaign ran **once** after the fix, with
both pinned roots and isolated target/temp paths: exit **0**, completed
`2026-10-04T10:13:11-04:00`. Inventory/impact ran exactly one test (228.22 s)
with both release markers. The smallest compiler/codec vertical ran exactly
one test (126.75 s), with both release completion markers and all six backend
compiler markers. All nine actual production service-check tuples returned 0;
the helper emitted `TASK063 NEWLY READY SERVICE-CHECK: PASSED (9 world tuples;
6 release/backend/message tuples)` and the wrapper emitted
`TASK063 PINNED GATES: PASSED`. Complete output is retained as `pinned.log`.

Immutable BEFORE remains `add348ce867daf4253a88510a65a48a3ed23f218`.
UCI 2.5 gains remain +1 declaration, +2 closed closures, +1 open closure;
UCI 2.6 remains zero delta. Newly READY services remain OrdersMetadata
(closed/open) and PrioritizationList (closed only), with nine world tuples,
six release/backend/message tuples and the 82-declaration OrdersMetadata
smallest vertical. Task 062 and Task 064 worktrees/branches are not modified.

## Corrective #2 — reconciliation with merged Task 062

The immutable measurement benchmark remains
`add348ce867daf4253a88510a65a48a3ed23f218`. The **current final integration
base** is `52e22b67b94b431d96925c41ff730749c33b3c9e`, the normal main merge of
Task 062 / PR #63 (parents `cf9ebc518f49e58b1731417186c836f3a41e0e74` and
`4391761f3cd27901f11bd8c93fa35d6a31750156`). Task 062 merged after Task 063's
isolated measurements. Earlier integration/isolation statements above describe
their historical checkpoints, not this combined source tree.

Previous final Task 063 head: `d4d4dbbd83a1691b9337fcdf36f582bc713aa358`.
Fast **37208561985 / attempt 1 succeeded**. Deep **37208561994 / attempt 1
failed**, real-UCI job **111454836469**, with actual Actions checkout
`76bd0fc0c695613766b9e3cc5ac088a482e0f12c`: merge of that feature head into
old main `cf9ebc518f49e58b1731417186c836f3a41e0e74`.

The failure was `task058_closed_schema_ada_gap_evidence`, at
`crates/cli/tests/uci_bounded_ascii_string.rs:374`: **live 634, stale expected
632**. The immutable Task 063 coverage ledger independently records 632 → 634
closed UCI 2.5 Ada closures: exactly the legitimate +2 capability gain. The
Task 063 hosted pinned inventory/impact/vertical and all nine production
service checks **passed**, proving the TMPDIR corrective worked. Deep's
`msrv-real-uci` and `real-sleet` also succeeded. Tasks 059–060 were skipped
after inherited Task 058 failed. Original metadata and full job log are saved
outside the checkout in `/tmp/task063-reconcile-evidence/`.

The repair imports main's already-reviewed **capability-relative Task 058
test unchanged**, not a Task 063 bespoke total. It retains peer-set equality,
Ada subset, the three naming witnesses, Query unsafe declarations/collision,
bounded-ASCII members and projected naming/readiness invariants. No production
capability is rolled back. The TMPDIR → RUNNER_TEMP → platform-temp fallback,
`main()` guard and no-service-check import regression remain unchanged.

### Integrated-current oracle (not a replacement historical campaign)

`task063-coverage.tsv` and `task062-after.tsv` remain byte-for-byte historical.
The Task 063 pinned test computes each final expected cell as **Task 062 AFTER
+ (Task 063 isolated AFTER − BEFORE)**, and checks that the isolated delta
itself is exactly the documented gain. Kind/field metrics have zero Task 063
delta; UCI 2.6 has zero delta throughout.

Source review also identified the same inherited absolute-current hazard in
`task062_pinned_ipv6_inventory_coverage_and_impact`: its full coverage row was
compared directly with Task 062's isolated AFTER. It now calls the shared
Task 063 integrated-current oracle in `tests/task063_integrated_coverage.rs`,
while still checking unchanged schema totals and every exact Task 062 service
row. No Task 062 fixture or production-specific IPv6 implementation changes.
This is a test-layer integration correction, not numerical fixture regeneration.

|Release|World|Backend|Kinds|Full declarations|Field types|Field occurrences|Closures|
|---|---|---|---:|---:|---:|---:|---:|
|2.5|closed|Ada|5554|5533|13160|13160|643|
|2.5|closed|Rust|5554|5534|13160|13160|676|
|2.5|closed|C++|5554|5534|13160|13160|676|
|2.5|open|Ada|5554|5446|13160|13160|571|
|2.5|open|Rust|5554|5446|13160|13160|571|
|2.5|open|C++|5554|5446|13160|13160|571|
|2.6|closed|Ada|5567|5546|13198|13198|646|
|2.6|closed|Rust|5567|5547|13198|13198|679|
|2.6|closed|C++|5567|5547|13198|13198|679|
|2.6|open|Ada|5567|5459|13198|13198|574|
|2.6|open|Rust|5567|5459|13198|13198|574|
|2.6|open|C++|5567|5459|13198|13198|574|

Task 061 live Time impact uses `task062-time-impact-current.tsv`; Task 060's
subset uses `task062-task060-subset-current.tsv` and the 33-message
`task062-ada-full-schema-gap.tsv`. Only closed UCI 2.5 PrioritizationList
overrides its verified baseline serial-number blocker to READY; selected and
support counts are unchanged. Current subset summaries must measure 86 READY,
six topology failures and eleven NITF_DateAndTimeType blockers per release,
with no remaining serial-number blocker. Both task families' Fast/Deep and
runtime registration gates are retained; Deep keeps the 240-minute budget.

Validation and final hosted certification are recorded below when complete.
