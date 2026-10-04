# Task 061 — named UCI Zulu Time

## Status and isolated provenance

Production capability and local synthetic/real-service evidence implemented.
The original delivery and its measurements below are historical checkpoints;
the corrected-parent reconciliation and fresh final-head/base hosted gates are
recorded in the final section and on PR #62. No merge approval is implied.

* Original Task 060 parent, actual starting SHA, and frozen BEFORE benchmark:
  `ca31412f78a157efeef43157de58ad36471e2975`.
* PR #61 was OPEN and unmerged at branch creation; fetched parent did not advance.
* Linked worktree: `/home/zboll/git/ams-gra-codegen-oms-task061`.
* Branch: `feature/061-time-zulu-support`.
* Task 060 worktree, branch, validation processes and watchers remain untouched.
* `CARGO_TARGET_DIR=/tmp/task061/target`, `TMPDIR=/tmp/task061/tmp`;
  logs, manifests, probes, evidence live under `/tmp/task061/`.
* Relevant test helpers use `std::env::temp_dir()` for fixed scratch names;
  no literal `/tmp` scratch paths were found under crates/scripts. Setting TMPDIR
  isolates those names. No shared cargo clean or Git configuration change.

## Pre-production evidence gate

Pinned public source: `https://gitlab.com/open-arsenal/uci/standard.git`.

|Release|Tag commit|Root SHA-256|
|---|---|---|
|2.5|`093610b7753944059360d3236770ab446d039556`|`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`|
|2.6|`78eb61b6112c8bffa40820c33124b57787fc5bd9`|`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`|

Reused read-only roots from `/tmp/task058-corrective/root25` and `root26`.
Both checkout HEAD/tag identities and both root digests were verified before use;
the existing fetch scripts were not rerun (2.6 extraction would mutate reused
artifacts). The inventory test checks root digests independently.

Raw XSD cross-check finds `TimeType` restricted directly from `xs:time` with
exactly `<xs:pattern value=".+Z"/>`, no other authored facet. Restriction lines:
2.5 **145157**, 2.6 **145408**. Raw root scans find no direct `xs:time`
element/attribute reference. Normalized complete inventory, inherited members,
selected and generated-support reach and readiness are being captured by
`task061_pinned_time_inventory_and_impact`; raw name matching alone is not the
admission oracle. Baseline logs: `/tmp/task061/logs/before-inventory-impact.log`;
durable exit status: same stem `.exit`. A missing completion marker is NOT a pass.

Admission: `PrimitiveKind::Time`, one effective pattern group with one alternative,
XML Schema dialect, expression exactly `.+Z`, and no other effective authored
facet. Classification must not inspect the name. Excluded: unconstrained named
Time, other patterns, other dialects, additional alternatives/groups, explicit
whitespace, bounds/other facets, and **all direct Time**, including Choice.

## Standards authority and lexical policy (before production)

Authority read: [XML Schema 1.0 Part 2, second edition, 28 October 2004](https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/).
Sections 3.2.8, 3.2.7.1–3, 4.3.6, Appendix D, and Appendix F were inspected.
[Second-edition errata](https://www.w3.org/2004/03/xmlschema-errata) states
“Errata for Part 2 (Datatypes): None to date.” Local authority copies are
`/tmp/task061/evidence/xsd10.html` and `errata.html`.

* §3.2.8.1: Time is the left-truncated dateTime lexical representation.
  Complete structure: `hh:mm:ss ('.' [0-9]+)?`, optional timezone in the base
  domain; this profile requires uppercase terminal `Z`.
* Appendix D.1: decimal digits are ASCII `#x30–#x39`; hour is 0–24,
  minute 0–59, whole seconds 0–60, arbitrary fractional precision permitted.
  Second **60 and 60.fraction are accepted**; second **61 is rejected**.
  Inappropriate leap-second placement maps into the following minute rather
  than imposing a lexical rejection/calendar table. Time has no calendar date.
* §3.2.7.1: hour 24 is allowed only when represented minutes and seconds are
  zero. Thus `24:00:00Z` and `24:00:00.000Z` are accepted; any nonzero minute,
  second or fractional digit is rejected, even far into a long fraction.
* §3.2.7.2 and §3.2.8.2 distinguish canonical representation. Lexically valid
  hour 24 and trailing fractional zeros are retained, not canonicalized.
* §4.3.6: replace only TAB/LF/CR with SPACE; squeeze SPACE runs; remove
  leading/trailing SPACE. Unicode trim is not equivalent. Remaining embedded
  whitespace fails lexical validation. Store collapsed spelling unchanged.

These outcomes agree with the existing `datetime-zulu.txt` corpus and generated
DateTime policy. No material boundary contradiction was found; DateTime behavior
will not be changed. Fraction scanning uses digits, never floating point.

### Why the exact pattern is a terminal-Z gate, not a validator

Appendix F describes implicitly head/tail-anchored patterns; `.` excludes LF/CR.
On the already-validated, collapsed Time domain, the complete time prefix is
nonempty and has neither LF nor CR. Its permitted timezone spellings are absent,
numeric `+/-hh:mm`, or literal `Z`. Only the last ends in `Z`. Therefore `.+Z`
is equivalent to terminal literal `Z` **on that domain**, not on arbitrary text.
Suffix-only validation would wrongly accept `garbageZ`. No artificial date,
native date/time parser, regex engine, platform clock or temporal arithmetic.

## Test-first corpus

`tests/fixtures/temporal/time-zulu.txt` is an independent standards-derived oracle.
It includes malformed components, boundary seconds/hours, long fractions,
XML/non-XML whitespace, spelling preservation, timezone exclusions, NUL, DEL,
and non-ASCII digit lookalikes. One included compiler harness drives the generated
Ada/Rust/C++ probes; positive and deliberately wrong expected outcomes are checked
separately. Compilation, lifecycle, storage, readiness, codecs, impact, workflow
gates and hosted final-head evidence remain pending until recorded as executed.
## Frozen complete normalized inventory and 12-cell BEFORE

Executed preproduction gate: **1 passed**, both release markers, exit **0**, 189.40s. Registered exact target `task061_pinned_time_inventory_and_coverage`; source is parent production plus inventory-only test. Log `/tmp/task061/logs/before-inventory-coverage-complete.log`. Complete declaration/member rows are frozen in `tests/fixtures/temporal/task061-pinned-inventory.tsv`.

Each release: **one named Time declaration**, `TimeType <- xs:time`, exact single-group/single-alternative XML Schema `.+Z`, no other effective facets; **12 named member references**, **zero direct Time references**, **zero inherited effective Time members**, **zero repeated Time members**, **zero nillable Time members**. Two required Choice alternatives; five required Record members; five optional Record members. Root document declaration lines 145152 / 145403; restriction lines 145157 / 145408. Raw required/optional member cardinalities agree with normalized IR. Line numbering differs by one for two root members due to preceding indentation/newline handling; raw and normalized locations are independently retained.

|Release|World|Backend|Kind|Renderable|Field types|Occurrences|Message closures|
|---|---|---|---:|---:|---:|---:|---:|
|2.5|ClosedSchemaSet|Ada|5552|5530|13160|13160|587|
|2.5|ClosedSchemaSet|Rust|5552|5531|13160|13160|590|
|2.5|ClosedSchemaSet|Cpp|5552|5531|13160|13160|590|
|2.5|OpenExtensions|Ada|5552|5443|13160|13160|540|
|2.5|OpenExtensions|Rust|5552|5443|13160|13160|540|
|2.5|OpenExtensions|Cpp|5552|5443|13160|13160|540|
|2.6|ClosedSchemaSet|Ada|5565|5544|13198|13198|588|
|2.6|ClosedSchemaSet|Rust|5565|5545|13198|13198|591|
|2.6|ClosedSchemaSet|Cpp|5565|5545|13198|13198|591|
|2.6|OpenExtensions|Ada|5565|5457|13198|13198|541|
|2.6|OpenExtensions|Rust|5565|5457|13198|13198|541|
|2.6|OpenExtensions|Cpp|5565|5457|13198|13198|541|

Denominators: 2.5 5557 declarations / 13160 fields / 722 messages; 2.6 5570 / 13198 / 725. Full BEFORE readiness campaign completed against an isolated archive of the exact parent, with the same evidence harness used AFTER. Original scheduling attempt was stopped only after verifying its executable belonged to Task061: open-world diagnostic fallback rescanned the full schema for every unrepresentable projection. Replacement harness records projection failure directly and runs production readiness for renderable projections. No production policy changed and no Task060 process was touched. Complete BEFORE `/tmp/task061/logs/before-efficient.log`, AFTER `/tmp/task061/logs/after-impact.log`, both exit 0 with both markers.

## Capability, lifecycle, generated names and codec

The shared temporal classifier now admits TimeZulu without name matching. Ada, Rust and C++ validation/rendering accept that result together; direct Time remains unsupported. Rust codec readiness classifies named Time through the same boundary; direct Time retains its negative decision. Coverage and selected/support readiness already consume the shared temporal decision.

Rust uses private String storage, checked `new(&str) -> Option<Self>`, `as_str`, Clone/Debug only, no Default. C++ uses private string storage, checked `create(string_view)`, value(), no public default constructor, explicitly defaulted copy members suppressing destructive move. Ada uses a private type, Create/Value and a raising component default independent of assertion policy. No ordering or value equality is added. Ada intrinsic predefined equality remains lexical, explicitly documented just as for existing temporal carriers.

Helpers are carrier-private/nested; no generated top-level Time parser name exists. Ada reuses the established collapse body without modifying DateTime output. The compiler corpus has **56 cases: 13 valid, 43 invalid**. Required/optional/inherited fields, bounded/unbounded sequences, Choice, concrete closed sums are constructed and accessed in all three compiled probes. Ada exercises empty bounded capacity and indefinite-vector reserve/growth. Rust compilation rejects PartialEq for scalar, record, optional, bounded, unbounded, Choice, inherited and concrete-sum storage; unrelated and absent-only storage retain derives. The sabotage path is a separate invocation of each probe; ordinary committed expectations are unchanged.

OMS mapping provenance: `open-arsenal/oms` @ `726272bd0390982a759c91a9cf4e13b81c2b510b`; official OMSC-SPC-013 Rev B digest `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`, verified from `/tmp/task061/evidence/oms-source`. §6.1.4 maps by primitive definition: Time meets none of cases 1–4, hence case 5 “Otherwise string”. §6.1.5.4 supplies string characters as the lexical value. Production codec encodes stored spelling verbatim, requires JSON strings and decodes only through checked construction; no independent codec normalization. Compiled valid/invalid round trips cover each storage position. Mock OWP publishes the unchanged collapsed spelling, delivers valid data and reports invalid numeric-offset Time as a codec event, never a typed-handler call.

## 12-cell measured coverage delta (frozen Task060 parent → Task061)

|Release|World|Backend|Kinds|Renderable|Field types|Occurrences|Message closures|
|---|---|---|---|---|---|---|---|
|2.5|ClosedSchemaSet|Ada|5552→5553|5530→5531|13160→13160|13160→13160|587→632|
|2.5|ClosedSchemaSet|Rust|5552→5553|5531→5532|13160→13160|13160→13160|590→664|
|2.5|ClosedSchemaSet|Cpp|5552→5553|5531→5532|13160→13160|13160→13160|590→664|
|2.5|OpenExtensions|Ada|5552→5553|5443→5444|13160→13160|13160→13160|540→566|
|2.5|OpenExtensions|Rust|5552→5553|5443→5444|13160→13160|13160→13160|540→566|
|2.5|OpenExtensions|Cpp|5552→5553|5443→5444|13160→13160|13160→13160|540→566|
|2.6|ClosedSchemaSet|Ada|5565→5566|5544→5545|13198→13198|13198→13198|588→637|
|2.6|ClosedSchemaSet|Rust|5565→5566|5545→5546|13198→13198|13198→13198|591→669|
|2.6|ClosedSchemaSet|Cpp|5565→5566|5545→5546|13198→13198|13198→13198|591→669|
|2.6|OpenExtensions|Ada|5565→5566|5457→5458|13198→13198|13198→13198|541→570|
|2.6|OpenExtensions|Rust|5565→5566|5457→5458|13198→13198|13198→13198|541→570|
|2.6|OpenExtensions|Cpp|5565→5566|5457→5458|13198→13198|13198→13198|541→570|

The field metrics already considered named references renderable at the parent; admitting their declaration increases kinds/renderable by one, not the named field-reference counts. Full-schema closures and projected-service READY are distinct measurements.

## Complete Time impact (selected closure OR generated support)

`tests/fixtures/temporal/task061-message-impact.tsv` freezes unique release/world/backend/message verdicts; `task061-newly-ready.tsv` freezes exact gains. Every reachable Time declaration is in the semantic selected closure in these roots: support-only additional reach is zero. Synthetic support-only negatives remain mandatory.

|Release|World|Reach per backend|Newly READY per backend|Already READY|Other declaration blockers|Projection failures|
|---|---|---:|---:|---:|---:|---:|
|2.5|Closed|103|72|0|4|27|
|2.6|Closed|106|76|0|3|27|
|2.5|Open|103|26|0|0|77|
|2.6|Open|106|29|0|0|77|

Backend projected gains are identical. There are **444 unique (release,backend,message) gains**, or 609 tuples when world is included; open gains are a subset of closed gains, not additional messages. Closed remaining declaration-first blockers: UCI2.5 PrioritizationList → USMTF_SerialNumberOfQualifierType; both releases SAR_CapabilityStatus and SAR_Command → NITF_DateAndTimeType; Task → IPv6_AddressType. Closed projection failures: 23 cyclic graphs and 4 unrepresentable abstract-value topologies per release. Open: 77 abstract/world projection failures. No READY is claimed for projection failures.

Task060 historical 45-message Time-first subset: **42 READY / 3 remaining** in 2.5; **43 READY / 2 remaining** in 2.6, with the blockers above excluding Task. Historical Task060 fixture/document measurements are preserved; current test expectations use a separate measured fixture. Its complete 103-message String-impact subset now has 80/81 READY, 11 IPv6-first, 5 deferred Unicode-first, 1 USMTF-first in 2.5 only, 6 topology failures. This is not the complete Time campaign.

### Exact newly READY closed set

UCI2.5 (each backend): AMTI_CapabilityStatus; Action; ActionCommand; ActionPlan; ActionPlanCommand; ActionStatus; ActivityPlanCommand; AirSampleCapabilityStatus; ApprovalPolicy; COMINT_CapabilityStatus; CommSupport; CommSupportCapabilityStatus; CommSupportPlan; CommSupportPlanCommand; CommSupportStatus; CommandedTimelineActivity; CommandedTimelineCapabilityStatus; CommandedTimelineCommand; CoordinatedRequirementSet; DLZ; DLZ_RequestStatus; EA_CapabilityStatus; ESM_CapabilityStatus; Effect; EffectCommand; EffectPlan; EffectPlanCommand; EffectStatus; FusionSettings; FusionSettingsRequest; IFF_CapabilityStatus; MissionEnvironmentOverride; MissionPlan; MissionPlanCommand; MissionRequirementStatus; OpLine; OpPoint; OpRouting; OpVolume; OpZone; OrbitActivityPlanCommand; OrbitPlan; OrbitPlanCommand; PackageReadiness; PlanningFunction; PlanningFunctionSettingsCommand; PlanningFunctionStatus; RF_ControlCommand; RF_Report; RadarAltimeterCapabilityStatus; RadarAltimeterReport; RequirementOptions; RequirementOptionsCommand; ResponseCommand; ResponsePlan; ResponsePlanCommand; ResponseStatus; RouteActivityPlanCommand; RoutePlan; RoutePlanCommand; SMTI_CapabilityStatus; SpaceCharacteristics; SupportPlan; SupportPlanRequest; SupportRequest; SystemsNeededRequest; TaskCommand; TaskPlan; TaskPlanCommand; TaskStatus; WeatherDataset; WeatherRadarCapabilityStatus.

UCI2.6 adds: PrioritizationList; SystemSchedule; SystemScheduleDataRequest; SystemScheduleDataRequestStatus. Exact open-world subsets and all backend tuples are in the fixture.

## Smallest newly READY vertical and regressions

Deterministic minimum of selected + support declarations, then message name, is **DLZ**, 54 + 0 in both releases. Production service-check/service-generate, Ada model/API, strict C++17 model/API and Rust warnings-denied model/API/codec valid-invalid round trips passed in `/tmp/task061/logs/dlz-second.log` (1 test, both markers, exit 0). The settled gate also compiles Ada generated bodies. Initial vector incorrectly used a number for TargetClosureRate; checked codec rejected it. Raw Velocity2D_Type requires NorthSpeed/EastSpeed, and the corrected vector uses that object. No production relaxation was made. OrderOfBattle and SMTI_SettingsCommand remain parent-READY regressions, not Task061 gains.

## Historical delivery fixture comparison

Frozen-parent source archive and rebuilt CLI versus the original delivered Task061 source, all 187 repository XSD fixtures × Ada/Rust/C++ × closed/open: **563 byte-identical successes; 547 shared failures; 12 new successes; zero changed-success cells; zero regressions**. All previously successful DateTime, Duration and String output (including Rust derives) was byte-identical at that source identity. This result is retained unchanged as historical evidence, not asserted for the later Ada-corrected source. New fixtures contribute expected new successes. Log `/tmp/task061/logs/fixture-compare.log`, exit 0; detailed result `/tmp/task061/evidence/fixture-results.json`.

## Ada naming attribution and whole-schema first blockers

`task061_pinned_ada_full_schema_naming_attribution` executed (1 test, 113.51s, both markers, exit 0). Rust/C++ full-schema renderable message closures exceed Ada by exactly **32** in each release. The frozen exact names are in `task061-ada-full-schema-gap.tsv`; all their closures reach Ada-unsafe declarations. Existing full-schema QueryPET/QueryType name context, not the Time scanner, accounts for this distinction. Projected service preflight narrows the context: every one of the 444 unique newly READY release/backend/message tuples was independently confirmed by production service-check (609 world tuples, all exit 0). No QueryType_Kind behavior was changed.

Whole generation failed in all 12 cells BEFORE and AFTER, with unchanged first blockers: closed Ada → QueryType companion/QueryType both generate QueryType_Kind; closed Rust/C++ → abstract SourceCommandEXT has no concrete structural descendants; open all backends → CapabilityCommandBaseType not closed under open extensions. Logs `/tmp/task061/logs/whole-{before,after}-<release>-<world>-<backend>.log`, campaign `/tmp/task061/logs/whole-blockers.log` exit 0. The two initial completed logs were reused after a foreground timeout; remaining cells ran once in background. Generation failures are expected evidence, never counted as successes.

## Local validation provenance

Linux, Rust/Cargo 1.98.1; GCC/G++/GNAT 14.2.0; explicit MSRV 1.95.0. Task061 roots/digests and scratch isolation above apply to every pinned command. All commands are launched once per recorded source/input identity and write durable `.exit` siblings; captured diagnostics are printed by workflow helpers before propagating failure.

Source manifests under `/tmp/task061/manifests/` include Git tracked, unstaged and untracked source files, not HEAD alone. `scripts/task061-source-manifest.py` reproduces that identity. Settled validation identity before documentation-only updates: `7c5cabcc8d9389f9497015219e68833198b08050fba82c186dd9c945a70ee450` (`final-code.json`). Production scanner/carrier code is unchanged from successful compiler/codec/fixture campaigns; later edits strengthen evidence assertions and documentation.

|Command / input|Observed status|Log under `/tmp/task061/logs/`|
|---|---|---|
|`bash scripts/check-task061-fast.sh`, GNAT required, no roots|pass; each exact registered target 1 passed|`fast-final.log`|
|`cargo fmt --all -- --check`|pass, exit 0|`fmt-final.log`|
|`cargo check --workspace --all-targets`|pass, exit 0|`check-final.log`|
|`cargo clippy --workspace --all-targets -- -D warnings`|pass, exit 0|`clippy-final.log`|
|`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`|pass, exit 0; pinned-root skips are not counted as real-UCI evidence|`workspace-final.log`|
|actual pinned helper, both roots|pass, both inventory/impact and DLZ compiler/codec release markers; 1 test per exact invocation|`pinned-helper.log`|
|production unique service-check, both roots|pass, 609 unique world tuples / 444 release-backend-message tuples|`service-impact-check.log`|
|all fixture BEFORE/AFTER cells|pass campaign; 0 regressions/changed successes|`fixture-compare.log`|
|`scripts/check-ci-split.sh` / adversarial helper|pass, 125 checks|`ci-split-final.log`|
|Task060 and Task061 actual CI-wrapper adversarial scripts|pass, 4 checks each; failure diagnostics/zero tests/wrong names rejected|captured tool output|
|1.95.0 runtime API/runtime/facade all-target checks|pass, exit 0 (includes generated synthetic Time codec)|`msrv-api.log`, `msrv-runtime.log`, `msrv-final.log`|
|1.95.0 facade all-target with pinned 2.5 root|pass, exit 0; actual real UCI generated model/codec included, 9m18s|`msrv-pinned.log`|
|OrderOfBattle/SMTI_SettingsCommand real regressions|pass, exit 0; all four release/message markers; 384.16s|`oob-smti-regression.log`|
|current historical subset and Ada gap assertions|pass, 1 executed test each, both release markers, exits 0|`subset-regression.log`, `gap-regression.log`|

Wrong dialect: the current normalized IR has only `PatternDialect::XmlSchema`; a wrong dialect cannot be constructed through its safe API. Explicit dialect equality remains in shared admission, and the boundary test documents this limitation instead of inventing a new regex dialect. Additional invalid whitespace facets/illegal Time length facets fail either schema validation or classification; redundant authored collapse still fails classification.

## Delivery and hosted review gates

PR #61 remains open at original head; Task061 targets its feature branch while
unmerged. Task060 Fast passed; its final Deep run `37135864214` later failed
because the fresh runner's offline OrderOfBattle Rust probe lacked
`block-buffer v0.10.4`, not because carrier validation failed. The exact inherited
failure is in `/tmp/task061/logs/parent-final-deep-failure.log`. Task061's pinned
helper now explicitly `cargo fetch --locked`s workspace dependencies before
offline standalone probes, without changing the parent, shared Cargo cache
configuration, runtime semantics or its watchers.

PR **#62**, OPEN, non-draft, auto-merge disabled, base
`feature/060-alternating-ascii-string-profiles` @ `ca31412f78a157efeef43157de58ad36471e2975`.
Initial committed head `f1bdd978cdfa22f3cca5a4f7eae0f5dc5b25dc46` triggered Fast
`37141773853` and Deep `37141773883`, attempts 1, both pull_request. Associated
head/base identities were verified with Actions API. Synthetic merge ref was
`1a39bcc33098a0053694c951dcffe0c70cc55d0c` with exactly those base/head parents;
actual checkout log is not yet available while jobs run. These initial runs are
historical after the evidence/cache prerequisite correction; final-head gates
remain mandatory. Stacked green is only stacked-review evidence, not
final-main-base review approval. Both PRs remain unmerged.

### Settled local delivery checkpoint

All late local gates completed: exact current subset 1 test / both markers / 387.80s; Ada gap 1 test / both releases / 177.49s; OrderOfBattle/SMTI 1 test / four markers / 384.16s; pinned MSRV exit 0 / 9m18s. GNAT-required workspace completed 136 test-suite summaries with **1248 passed tests**, exit 0. Final fixture replay confirmed the same 1122-cell outcome and zero regressions. Locked cache prerequisite `cargo fetch --locked` passed (`fetch-locked.log`) and actual wrapper/split adversarial guards passed after the correction. No expensive production campaign was repeated solely for the cache/doc change.

Capability commit `f1bdd978cdfa22f3cca5a4f7eae0f5dc5b25dc46`; cache/evidence correction `117fd6af0ea738b4ca8c3c8c5dc0e3d9a7f6e142`. Source manifest for the latter is `/tmp/task061/manifests/117fd6a-source.json`, identity `f21e2f1ef9f47c2b17c3fc9e6a40459c1e4588c08f6425620224cb5dd5d50e76`. Current correction-head Fast **37142245678**, Deep **37142245733**, attempts 1, associated base/head verified; synthetic merge ref **2de91e1b05368969e11db5f2d64e865ccc56715e**. Initial runs were cancelled by normal same-PR concurrency after push. Actual checkout log and final conclusions remain outstanding until jobs finish. Documentation-only updates must also receive final-head associated gates; final run/head/base/checkout outcomes will be retained on PR #62 and in the delivery report, never inferred from this checkpoint.

Historical review status at this checkpoint: implementation/local evidence
complete; hosted gate pending. The following continuation supersedes those
pending-run and original-parent status statements without relabeling their evidence.

## Corrected-parent and complete hosted-validation continuation

### Closed original run record

Original delivery `1a836075a973a5ccb8de2bbbfe65d1d47d187e46` associated with
base `ca31412f78a157efeef43157de58ad36471e2975`:

* Fast **37142559823**, attempt **1**: completed / **success**.
* Deep **37142559805**, attempt **1**: completed / **cancelled**, not success.
* Every completed job's checkout log identifies synthetic merge
  `678cc2d50cbe554c929a3a4f076ce09dc2a70757`, distinct from the associated
  head/base and not evidence for a later reconciliation.
* Real-UCI job **111259893530** passed Time inventory/impact, the complete
  **609 world-specific CLI tuples / 444 unique release/backend/message gains**,
  and inherited Tasks **052, 053, 054, 056 and 057**. Task **058 cancelled**;
  Tasks **059 and 060 skipped**. Real-Sleet and MSRV-real-UCI passed.
* The explicit annotation says **"The job has exceeded the maximum execution
  time of 2h0m0s"**. The cancellation log is at **2026-10-03T20:00:33Z**, during
  Task 058. This is a documented timeout cancellation, not an inferred
  dependency or Duration failure. Successful sections are partial historical
  evidence; cancelled/skipped gates are not counted as executed successes.
* The original collector finished with exit **0**, posted its terminal report,
  and has no active waiting process. Retained completed job logs and
  `/tmp/task061/evidence/delivery-hosted-completed.json` are reused. Closure and
  the once-retrieved annotation are under
  `/tmp/task061/ada-reconciliation/evidence/`.

### Normal ancestry and distinct integration/measurement identities

The previously local-only reconciliation
`3527d32e133a82b887eed61a4f45fd511fc70a09`, containing Task 060
`49ca06f24f891b4c041e8f3caabcd39e441f3f05`, is preserved as an ancestor.
The complete required corrected parent is
`770e288e8b547cd1b8c1ac2bda75f65292b9cc63`. Its later production Ada renderer
correction, production-generated bounds regression and actual Fast execution
are incorporated by a normal merge, not a duplicate cherry-pick or rewrite.

Before this continuation's publication, PR #61 independently merged at
`0800e97c71d8bf3edcef40ef1dc4ef5200b76deb` on **2026-10-04T01:28:47Z**.
The two-parent GitHub merge has corrected parent `770e288` as an ancestor;
its tree equals that corrected parent. A further normal child merge preserves
the actual main ancestry. No Task 060 branch/worktree was modified, its
validation was not repeated, and this task did not merge PR #61. The child-only
diff against that main base retains Task 061 and its narrow CI integration,
with no duplicate Task 060 corrective diff. PR #62 may therefore target main
only after this verified actual merge; fresh head/main-base Fast and Deep
remain mandatory.

**Immutable BEFORE benchmark remains**
`ca31412f78a157efeef43157de58ad36471e2975`. The later integration parent and
main merge do not replace it, its pinned schema identities, selection rules,
BEFORE/AFTER measurements, or historical artifacts.

### Finite full-campaign time budget

Only `jobs.real-uci.timeout-minutes` changes from **120 to 240**, with the
adjacent comment identifying Task 061 plus all inherited Tasks 052–060.
Original Time steps measured **18m21s + 56m39s = 75m**. The parent's complete
inherited sequence took approximately **93m** on a different run; **168m** is
a planning estimate, not a measured completed child duration. The finite
four-hour budget accommodates the complete campaign without skipping,
sampling, reducing the 609 tuple set, weakening counts/markers, adding blind
retries, changing triggers/concurrency, or altering other jobs' budgets.
Locked fetching, failed-command diagnostics/original exit status, qualified
AND registration, all bounds/Time gates and inherited target checks remain.

### Production Ada output impact and reused evidence

This continuation is **not workflow-only**. The shared factored Ada renderer
now emits `Text'First + (Finish - 1)` so an endpoint at `Positive'Last` does not
overflow before subtraction. Empty-component short-circuit and lexical
admission remain unchanged. The new production-generated regression checks
19 spellings × three placements × three assertion policies = **171 constructor
checks**, with `-O0 -gnato` and actual compilation/execution.

Manifest `/tmp/task061/ada-reconciliation/manifests/reconciled-code.json`,
identity `899dc98c7968b7852f997dbf8c7eca8e0a909ac4b18d406ff90d5000afc9fcbc`,
records all tracked/unstaged/untracked inputs before this documentation-only
addition. Compared with delivery identity
`8cad96e6904e6ef6c74b546443d708710e99c88466c01c20768986faa086483c` and earlier
reconciliation identity
`2e42424f8da667572d30e7748afd76bf4d96273b8f1be468aa9444905ae8b2bf`, the only
additional production change is the Ada renderer; the new bounds test and
Fast execution, finite Deep budget and parent document are recorded separately.
Comparison ledger: `evidence/source-manifest-comparison.json` in that scratch.
All frontend/IR, classifier/readiness/naming, Rust/C++ producer/codec, pinned
schemas, Cargo inputs, measurement fixtures and benchmark selection inputs
retain their prior hashes. Original 1,248 workspace passes, MSRV checks,
609 CLI confirmations and 444 unique gains retain original provenance; they
are not relabeled as fresh final-source hosted results.

To establish the complete child fixture boundary rather than assuming the
parent's changed slices exhaust it, **all 187 child XSD fixtures × both worlds
= 374 Ada cells** were regenerated using frozen, earlier reconciled and
corrected child CLIs. Previous-to-corrected result: **184 identical successes,
188 shared failures, two expected changed successes, zero acceptance changes
and zero unexplained changes**. Only
`tests/fixtures/service-generate/codec-alternating-ascii.xsd` changes, in each
world; each `programs-oam.adb` differs only by five corrected endpoint
expressions (**10 replacements across two cells**). Exact normalization of
those expressions reproduces the corrected bytes. Files/digests/diffs are
retained in `outputs/`; full inventory/ledger is
`evidence/ada-fixture-comparison.json` under the reconciliation scratch.

Corrected Ada versus frozen: **180 identical successes, two expected endpoint
changed successes, four new successes, 188 shared failures**. Rust/C++ outputs
reuse the historical cells because their complete producer/input dependency
hashes are unchanged and the production change is confined to Ada rendering;
the combined **derived** final-source matrix is therefore **561 identical
successes, two expected changed successes, 12 new successes, 547 shared
failures, zero regressions**. It is not represented as a fresh all-backend
1,122-cell run. In particular, historical “zero changed successes” is not an
unqualified final-source claim. New compiler checks validate the affected Ada
behavior; original unchanged long campaigns are not repeated merely for SHAs.

### Reconciled local checks and publication gate

On the actual reconciled source, registered-test lists were audited before
exact execution. The inherited
`generated_factored_ascii_ada_bounds_under_strict_overflow` executed **one Rust
test**, all **171** constructor checks and compile/run exit 0 under default,
assertions-enabled and assertions-ignored policies. The existing Time helper
executed every exact compiler corpus/lifecycle/storage/equality,
readiness/naming, checked codec and mock-OWP gate. Split/wrapper checks pass
**125 / 47 / 4** adversarial checks, retaining failed-command propagation,
qualified names and zero-test rejection. fmt, workspace all-target check,
warnings-denied Clippy, script syntax and `git diff --check` pass.

The affected inherited OrderOfBattle/SMTI compiler-codec vertical completed
once on this Ada-corrected child identity: **one exact test, all four
release/service markers, exit 0, 400.00s**, not borrowed from `3527d32` or the
parent's completed validation. The focused Time DLZ model/API/compiler/codec
vertical also completed **one exact test, both release markers, exit 0,
117.37s**. Their completion artifacts and the validation ledger are retained under
`/tmp/task061/ada-reconciliation/logs/` and `evidence/`.

Publication uses the existing child branch/PR and a normal push after local
checks complete. One head/base-keyed read-only owner collects automatically
triggered Fast and Deep runs with restrained polling, exact attempts and
terminal conclusions; it does not silently retry workflows. Completed logs
must prove bounds execution, both Time inventory/DLZ releases, all 609 CLI
tuples, inherited Tasks 052–060, Real-Sleet and MSRV-real-UCI. Actual per-job
checkout SHA and synthetic parentage are distinct from associated PR head/base.
No CI-status-only source commit follows the settled validated head. Final
results supersede pending status on PR #62 and in scratch completion records.
Final-main-base readiness is claimed only after both fresh complete workflows
succeed on that head/base; neither success nor this document authorizes merge.
