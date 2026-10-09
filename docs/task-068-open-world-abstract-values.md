# Task 068 — Open-world abstract-value projection safety

## Decisions

**OPEN_WORLD_RUNTIME_CONTRACT_NOT_ESTABLISHED**

**GENERATION_TIME_EXTENSION_PATH_CONFIRMED**

This is an evidence/architecture gate, not an implementation. The production
`NotClosedUnderOpenExtensions` guard, world semantics, model APIs, codec semantics,
and runtime APIs are unchanged. No OpenExtensions service becomes newly READY.
Task027 is prior evidence: the public file set is closed but the UCI type universe
is not; private descendants are legitimate, including at zero-public-descendant
extension points. This audit does not repeat its thirteen prose classifications.

## Immutable source and isolation

- BEFORE: `5d284a346433d461f1520e599f969e82e00f3e82`.
- Branch: `feature/068-open-world-abstract-values`.
- Worktree: `/home/zboll/git/ams-gra-codegen-oms-task068`.
- Scratch: `/tmp/task068`; `CARGO_TARGET_DIR=/tmp/task068/target`;
  `TMPDIR=/tmp/task068/tmp`.
- Initial fetch found origin/main at BEFORE. The starting directory was Task065,
  whose clean HEAD was `e43d15207155dfe9ecfb627358c7f9677870ce26`, **not main**.
  It was not checked out or edited. The independent sibling was created directly
  at origin/main; its HEAD and origin/main were then verified equal to BEFORE.
- Source tree manifest was recorded before investigation at
  `/tmp/task068/manifests/source-before.txt` (HEAD, status, refs, worktrees, tree).
- No sibling worktree edited, cherry-pick, rebase, or force push.

### Pinned authoritative inputs

Both fetch scripts verified revision and root SHA-256 before loading:

| Release | Repository revision | Root SHA-256 |
|---|---|---|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

Source: `https://gitlab.com/open-arsenal/uci/standard.git`, tags v2.5/v2.6.
2.5 root: `03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd`.
2.6 root: `UCI_MessageDefinitions_v2_6_0.xsd` extracted from the pinned CDRL.
All real QNames below use `https://www.vdl.afrl.af.mil/programs/oam`.
Full identities and inspected document hashes are frozen in
`tests/fixtures/open-world/task068-input-manifest.txt`.

## Production boundary and evidence method

`resolve_service_plan` resolves a portable one-message service contract against
production frontend IR. `project_service_generation_schema` verifies bindings,
uses the raw selected closure, then calls shared support expansion. For every
abstract structural **value** demand, `admit_abstract_value` returns
`AbstractValue(NotClosedUnderOpenExtensions(target))` in OpenExtensions, before
backend generation. Abstract ancestry alone is not a value demand.
ClosedSchemaSet instead admits all concrete transitive descendants and their
support dependencies; that representation is exhaustive only under its explicit
closed-world assertion.

Every publishable message was resolved and projected once per world in the initial
pass, without duplicating language-independent projection six times. Each evidence
executable loads each release root once. Descendants, effective members, paths,
and recursion are indexed/reused in the path pass. A deterministic breadth-first
walk records the first path to each owner, not every infinitely repeatable path
through a recursive graph. Fields are visited lexically. Rows sort by release,
message QName, target QName, path; no HashMap order is exposed.

Two distinct measurements must not be conflated:

1. **Actual production first error:** 151 messages per release, 34 distinct first
   targets. Projection aborts at its first demanded open value. There is no
   successful Open projection or meaningful completed Open support count.
2. **Reachable known-representation value-use inventory:** 52 distinct targets,
   954 rows in 2.5 and 956 in 2.6. This follows known concrete support branches to
   expose masked targets; it does **not** claim those later targets were individually
   returned as the first production error. Every actual first target is present
   in its message's path inventory. `<payload>` denotes direct abstract payloads.

`selected_or_support` classifies the **owner**, not the target. In the TSV,
`inherited` describes whether the member is inherited into that owner. Repeated
appearances of the same declaration at different owners are intentional.
Generated support counts used for ranking are ClosedSchemaSet costs, not fake
Open or AFTER numbers. For the 46 Closed projection failures per release, an
independent indexed fixed-point dependency/descendant count estimates the support
cost without claiming emission succeeds; it matches production support counts
for every successful projection. Open production fails before admitting any
closed-sum variants. A partial traversal count before failure is not exposed by
its public error API, so no such number is invented.

### Reproducibility and timings

Run `bash /home/zboll/git/ams-gra-codegen-oms-task068/scripts/run-task068-audit.sh`.
Probes are isolated external Cargo packages compiled against the frozen production
crates. They call public resolution, projection, readiness, frontend and renderer
APIs. No private production internals are patched or called.

The fixture consistency check is `python3 scripts/check-task068-fixtures.py`.

Initial complete pinned inventory, including six coverage cells: **140.357 s**.
Indexed full path inventory: **60.157 s**. Independent support-cost pass:
**53.735 s**. Per-message Closed readiness pass: **347.134 s**. Open readiness
uses one combined affected-message plan per release and all three backends;
it reports exactly 151 blocked messages per cell, zero renderable selected
messages, zero generated support, no backend/global or service-API blocker.
This proves closure attribution parity without 906 full-schema readiness builds.
The compact synthetic proves one-message readiness/projection parity directly.

### Frozen fixtures

Under `tests/fixtures/open-world/`:

- `task068-open-projection.tsv`: exact QName/owner/member/cardinality/descendants/
  depth/recursion/inheritance/first path; selected versus support owners.
- `task068-impact.tsv`: exact 302 release/message first-error entries, counts,
  readiness, overlapping classes, all reachable targets and other blockers.
- `task068-targets.tsv`: all 104 release/target topology entries.
- `task068-coverage.tsv`: six BEFORE cells.
- `task068-closure-failures.tsv`: exact six coverage closure failure sets.
- `task068-open-readiness.tsv`: exact per-language affected message sets/blockers.
- `task068-runtime-matrix.tsv`, `task068-synthetic.txt`: executable controls.
- `task068-private/`: authored public/private/zero/concrete schemas and contract.

## Real impact and ranking

Both releases have the **same exact 151 affected messages**. The six production
coverage closure deficits are also exactly these 151 messages (not all deficits
are repairable solely by removing the Open guard).

For each release:

| Class (overlapping) | Count | Meaning |
|---|---:|---|
| A | 94 | Closed READY in Ada/Rust/C++; Open value boundary blocks |
| B | 57 | Closed has another blocker too: 46 projection errors, 11 NOT READY |
| C | 65 | More than one distinct reachable abstract value target |
| D | 46 | At least one recursive known-descendant value graph |
| E | 38 | At least one zero-known-concrete-descendant target |

Rank by selected + Closed support cost, then lexical message QName. The smallest
meaningful real vertical is **DataRecordListManagementRequest** in both releases:
43 selected + 4 support = **47 declarations**, Closed READY in all backends,
Open first error `ManagedListBaseType` (one concrete descendant, acyclic).
`DataRecordListManagementRequestMDT.AddRecord` and `.DeleteRecord` are both
`0..1` value positions. Path:

```text
DataRecordListManagementRequest -> DataRecordListManagementRequestMT.MessageData
 -> DataRecordListManagementRequestMDT.AddRecord -> ManagedListBaseType
```

The next ranked vertical is CommSupportCommandStatus: 48 selected + 0 support,
zero-descendant CommSupportCommandStatusEXT. This is not a reason to erase the
extension. Even A is a model/service readiness result, not proof of codec readiness
in all languages. No claim is made that deleting the guard would make any service
safe, or that it clears all other backend/codec requirements.

### Exact affected message set (identical in 2.5 and 2.6)

`AO_CapabilityStatus`, `AO_SettingsCommand`, `AccessAssessment`, `AccessAssessmentRequest`, `AccessAssessmentRequestStatus`, `Action`, `ActionCommand`, `ActionPlan`, `ActionPlanCommand`, `ActivityPlan`, `ActivityPlanCommand`, `AirRecord`, `AirfieldReport`, `ApprovalPolicy`, `Assessment`, `AssessmentRequest`, `AssessmentRequestStatus`, `COMINT_Activity`, `CommPointing`, `CommSupport`, `CommSupportActivity`, `CommSupportCapability`, `CommSupportCapabilityStatus`, `CommSupportCommand`, `CommSupportCommandStatus`, `CommSupportPlan`, `CommSupportPlanCommand`, `CommSupportPlanningStatus`, `CommSupportStatus`, `CommTerminalActivity`, `CommTerminalCommand`, `CommTerminalPlan`, `CommTerminalPlanOverrideRequest`, `CommandedTimelineActivity`, `ComponentConfiguration`, `ComponentConfigurationDataRequestStatus`, `ComponentStatus`, `ComponentStatusDataRequestStatus`, `DMPI`, `DataPlan`, `DataPlanOverrideRequest`, `DataRecordListManagementRequest`, `DataRecordManagementRequest`, `DataUpdateRequest`, `EA_Activity`, `EA_Command`, `Effect`, `EffectCommand`, `EffectPlan`, `EffectPlanCommand`, `Entity`, `EntityManagementRequest`, `EntityMetadata`, `GatewayActivity`, `GatewayCommand`, `LandRecord`, `MessageTransmissionFilterRecord`, `MissileRecord`, `MissionEnvironmentOverride`, `MissionPlan`, `MissionPlanCommand`, `NavigationCommand`, `OB_CorrelationRecord`, `ObservationMeasurementReport`, `ObservationReport`, `OpLine`, `OpNotification`, `OpPoint`, `OpRouting`, `OpVolume`, `OpZone`, `OrbitActivityPlan`, `OrbitActivityPlanCommand`, `OrbitPlan`, `OrbitPlanCommand`, `OrderOfBattle`, `OrderOfBattleRequest`, `PO_Activity`, `PO_Capability`, `PO_CapabilityStatus`, `PO_Command`, `PO_SettingsCommand`, `PlanningFunction`, `PlanningFunctionSettingsCommand`, `PlanningFunctionStatus`, `PrioritizationList`, `ProductDownloadPlan`, `ProductDownloadTask`, `ProductOrFileClassificationPlan`, `ProductOrFileClassificationReport`, `ProductOrFileClassificationRequest`, `ProductOrFileClassificationRequestStatus`, `ProductOrFileClassificationTask`, `ProductOrFileClassificationTaskStatus`, `ProductOrFileDisseminationDestination`, `ProductOrFileDisseminationPlan`, `ProductOrFileDisseminationReport`, `ProductOrFileDisseminationRequest`, `ProductOrFileDisseminationTask`, `ProductProcessingFunction`, `ProductProcessingPlan`, `ProductProcessingReport`, `ProductProcessingRequest`, `ProductProcessingRequestStatus`, `ProductProcessingTask`, `ProductProcessingTaskStatus`, `QueryDataRequest`, `QueryDataRequestStatus`, `RelationshipDesignation`, `RequirementOptions`, `RequirementOptionsCommand`, `ResendDataRequestStatus`, `Response`, `ResponseCommand`, `ResponsePlan`, `ResponsePlanCommand`, `RouteActivityPlan`, `RouteActivityPlanCommand`, `RoutePlan`, `RoutePlanCommand`, `SAR_CapabilityStatus`, `SAR_Command`, `SAR_SettingsCommand`, `SMTI_CapabilityStatus`, `SMTI_Command`, `SMTI_SettingsCommand`, `SeaSubsurfaceRecord`, `SeaSurfaceRecord`, `SignalReport`, `StoreLoadoutConfiguration`, `SubsystemConfiguration`, `SubsystemConfigurationDataRequestStatus`, `SubsystemMaintenanceCommand`, `SubsystemMaintenanceConfiguration`, `SubsystemMaintenanceStatus`, `SubsystemStatus`, `SubsystemStatusDataRequestStatus`, `SupportPlan`, `SupportPlanRequest`, `SupportRequest`, `SystemDeploymentCapability`, `SystemEstimationRequestStatus`, `SystemManagementRequest`, `SystemMetadata`, `SystemReadiness`, `SystemStatus`, `SystemsNeededRequest`, `Task`, `TaskCommand`, `TaskPlan`, `TaskPlanCommand`.

### Actual first-error targets (34)

`AirfieldStoresPET`, `CapabilityCommandBaseType`, `CommSupportCapabilityEXT`, `CommSupportCapabilityStatusEXT`, `CommSupportCommandEXT`, `CommSupportCommandStatusEXT`, `CommSupportPlanningStatusEXT`, `CommSupportPointingEXT`, `CommSupportStatusEXT`, `CommSupportWindowEXT`, `CommWaveformActivityPET`, `CommWaveformCapabilityCommandPET`, `CommandedTimelineActivityReportPET`, `ComponentConfigurationPET`, `ComponentExtendedStatusPET`, `ConstraintEXT`, `DataLinkIdentifierPET`, `DataLinkNativeInfoPET`, `DataRecordBaseType`, `EA_TechniqueParametersPET`, `EntityMetadataPET`, `ManagedListBaseType`, `MessageType`, `NITF_PackingPlanPET`, `OpNotificationEXT`, `QueryPET`, `RecordDRLE`, `STANAG_4607_PackingPlanPET`, `StoreLoadoutItemPET`, `SubsystemExtendedStatusPET`, `SubsystemMaintenanceTestCommandPET`, `SubsystemMaintenanceTestPET`, `SubsystemMaintenanceTestResultPET`, `SystemMetadataPET`.

### All reachable targets including masked support (52)

`AchievabilityAssessmentPET`, `AchievabilityAssessmentRequestPET`, `AirfieldStoresPET`, `CapabilityBaseType`, `CapabilityCommandBaseType`, `CapabilityStatusBaseType`, `CommSupportCapabilityEXT`, `CommSupportCapabilityStatusEXT`, `CommSupportCommandEXT`, `CommSupportCommandStatusEXT`, `CommSupportPlanningStatusEXT`, `CommSupportPointingActivityEXT`, `CommSupportPointingEXT`, `CommSupportStatusEXT`, `CommSupportTaskEXT`, `CommSupportWindowEXT`, `CommWaveformActivityCommandPET`, `CommWaveformActivityPET`, `CommWaveformCapabilityCommandPET`, `CommandedTimelineActivityReportPET`, `ComponentConfigurationPET`, `ComponentExtendedStatusPET`, `ConstraintEXT`, `DataLinkIdentifierPET`, `DataLinkNativeFilterPET`, `DataLinkNativeInfoPET`, `DataRecordBaseType`, `EA_TechniqueParametersPET`, `EntityMetadataPET`, `GatewayConfigurationPET`, `GatewayNativeStatisticsPET`, `ImageRegionOfInterestPET`, `ManagedListBaseType`, `MessageType`, `NITF_PackingPlanPET`, `OpNotificationEXT`, `OpZoneFilterAreaPET`, `ProcessingParametersPET`, `ProcessingResultsPET`, `QueryPET`, `QuerySpecificDataPET`, `RecordDRLE`, `STANAG_4607_PackingPlanPET`, `SourceCommandEXT`, `StoreLoadoutItemPET`, `SubsystemCommandBaseType`, `SubsystemExtendedStatusPET`, `SubsystemMaintenanceTestCommandPET`, `SubsystemMaintenanceTestPET`, `SubsystemMaintenanceTestResultPET`, `SupportCapabilityCommandBaseType`, `SystemMetadataPET`.

## DataLinkIdentifierPET lead, measured

The abstract declaration has no base or local value members:

```xml
<xs:complexType name="DataLinkIdentifierPET" abstract="true"
                uci:version="002.000.000.000">
  <!-- Polymorphic Extension Type for defining data link identifiers ... -->
</xs:complexType>
```

Both releases: **8 concrete**, **0 abstract** descendants, maximum depth **1**,
**acyclic** known descendant representation graph. Exact concrete descendants:

- DiscreteDataLinkIdentifierType
- EW_CoordinationDataLinkIdentifierType
- EW_IndexNumberDataLinkIdentifierType
- IJMS_DataLinkIdentifierType
- Link11_DataLinkIdentifierType
- Link16_DataLinkIdentifierType
- NATO_Link1_DataLinkIdentifierType
- SpecialCodeDataLinkIdentifierType

It is the actual first projection error for **71 messages** per release, reached
through selected owners. It occurs additionally through support owners for **11**
messages: Assessment, AssessmentRequest, AssessmentRequestStatus,
DataRecordManagementRequest, OrderOfBattle, ProductProcessingPlan,
ProductProcessingReport, ProductProcessingRequest, ProductProcessingTask,
QueryDataRequestStatus, ResendDataRequestStatus. Thus **82** affected message
representation graphs reach it. There are **271** frozen owner/member paths in
2.5 and **273** in 2.6. These are first paths to value-use owners, not all possible
message routes. The fixture is the authoritative exact path set.

Of those 82 per release, **65 are otherwise Closed READY**, **16 fail Closed
projection** (recursive topology), and **Task** is NOT READY with unsupported
support declarations `NITF_DateAndTimeType`, `NITF_DateType`,
`NITF_MSTGTA_TargetLocationType`. All three backends agree.

The Task063 lead is confirmed: PrioritizationList has 351 selected + 13 support
in 2.5 (352 + 13 in 2.6), Closed READY, Open blocked at DataLinkIdentifierPET.
Action, ActionCommand, ActionPlan are also confirmed; ActionPlan additionally
reaches EntityMetadataPET, OpZoneFilterAreaPET and SystemMetadataPET.

PrioritizationList's exact first path (namespace omitted only here):

```text
PrioritizationList -> PrioritizationListMT.MessageData -> PrioritizationListMDT.ListItem
 -> PrioritizationListItemType.Subject -> IdentityKindInstanceType.ByIdentity
 -> IdentityType.SpecificVehicle -> VehicleIdentificationType.DataLinkIdentifier
 -> DataLinkIdentifierPET
```

`VehicleIdentificationType.DataLinkIdentifier` is local `0..unbounded`.
Other exact owners, inherited positions and cardinalities are in the path fixture.
A known list of eight does not imply it exhausts future legal private descendants.

### Exact DataLink-reaching messages (82, identical sets)

`AccessAssessment`, `AccessAssessmentRequest`, `AccessAssessmentRequestStatus`, `Action`, `ActionCommand`, `ActionPlan`, `ActionPlanCommand`, `ActivityPlan`, `ActivityPlanCommand`, `AirRecord`, `ApprovalPolicy`, `Assessment`, `AssessmentRequest`, `AssessmentRequestStatus`, `COMINT_Activity`, `CommSupport`, `CommSupportPlan`, `CommSupportPlanCommand`, `DMPI`, `DataRecordManagementRequest`, `Effect`, `EffectCommand`, `EffectPlan`, `EffectPlanCommand`, `Entity`, `EntityManagementRequest`, `LandRecord`, `MissileRecord`, `MissionEnvironmentOverride`, `MissionPlan`, `MissionPlanCommand`, `OB_CorrelationRecord`, `ObservationMeasurementReport`, `ObservationReport`, `OpLine`, `OpPoint`, `OpRouting`, `OpVolume`, `OpZone`, `OrbitActivityPlan`, `OrbitActivityPlanCommand`, `OrbitPlan`, `OrbitPlanCommand`, `OrderOfBattle`, `PlanningFunction`, `PlanningFunctionSettingsCommand`, `PlanningFunctionStatus`, `PrioritizationList`, `ProductProcessingFunction`, `ProductProcessingPlan`, `ProductProcessingReport`, `ProductProcessingRequest`, `ProductProcessingTask`, `QueryDataRequestStatus`, `RelationshipDesignation`, `RequirementOptions`, `RequirementOptionsCommand`, `ResendDataRequestStatus`, `Response`, `ResponseCommand`, `ResponsePlan`, `ResponsePlanCommand`, `RouteActivityPlan`, `RouteActivityPlanCommand`, `RoutePlan`, `RoutePlanCommand`, `SeaSubsurfaceRecord`, `SeaSurfaceRecord`, `SignalReport`, `SupportPlan`, `SupportPlanRequest`, `SupportRequest`, `SystemDeploymentCapability`, `SystemEstimationRequestStatus`, `SystemManagementRequest`, `SystemReadiness`, `SystemStatus`, `SystemsNeededRequest`, `Task`, `TaskCommand`, `TaskPlan`, `TaskPlanCommand`.

## Zero-descendant extension control and SourceCommandEXT

Synthetic `ExtensionBase` has zero descendants; `Holder.Extensions` is
`0..unbounded`. Closed projection retains existing semantics and fails with
`ProjectedEmissionPlan(... ExtensionBase has no concrete structural descendants)`;
Open returns `AbstractValue(NotClosedUnderOpenExtensions(ExtensionBase))`.
There is no repeated absent-only elision. Existing Task028 tests confirm this in
all three backends; Task029 overlay tests compile repeated private descendants.

Real SourceCommandEXT remains abstract, no base/local payload, version
`000.000.000.000`, zero concrete/abstract descendants in both roots. It is masked
by an earlier error, but reached in **six** message representation graphs:
ProductOrFileDisseminationPlan, ProductOrFileDisseminationReport,
ProductOrFileDisseminationRequest, ProductOrFileDisseminationTask,
QueryDataRequestStatus, ResendDataRequestStatus. Nine paths per release include
DisseminationSubplanType/DisseminationConstrainedSubplanType.ExtensionCommand,
`0..unbounded`. These six are not Closed READY; QueryPET/other recursive generated
graphs are separate blockers. Do not portray removing the open guard as a fix.

All 13 measured zero-concrete-descendant targets:

`CommSupportCapabilityEXT`, `CommSupportCapabilityStatusEXT`, `CommSupportCommandEXT`, `CommSupportCommandStatusEXT`, `CommSupportPlanningStatusEXT`, `CommSupportPointingActivityEXT`, `CommSupportPointingEXT`, `CommSupportStatusEXT`, `CommSupportTaskEXT`, `CommSupportWindowEXT`, `ConstraintEXT`, `OpNotificationEXT`, `SourceCommandEXT`.

## Current OpenExtensions coverage — BEFORE only

| Release | Backend | Declarations | Kind renderable | Fully renderable | Field refs | Occurrences | Message closures |
|---|---|---:|---:|---:|---:|---:|---:|
| 2.5 | Ada | 5557 | 5554 | 5446 | 13160/13160 | 13160/13160 | 571/722 |
| 2.5 | Rust | 5557 | 5554 | 5446 | 13160/13160 | 13160/13160 | 571/722 |
| 2.5 | C++ | 5557 | 5554 | 5446 | 13160/13160 | 13160/13160 | 571/722 |
| 2.6 | Ada | 5570 | 5567 | 5459 | 13198/13198 | 13198/13198 | 574/725 |
| 2.6 | Rust | 5570 | 5567 | 5459 | 13198/13198 | 13198/13198 | 574/725 |
| 2.6 | C++ | 5570 | 5567 | 5459 | 13198/13198 | 13198/13198 | 574/725 |

Raw field reference/occurrence counts are not semantic closure readiness; they
must not be misread as proof an abstract value can be decoded. `CoverageAnalysis`
combines value topology/world rules into declaration and message renderability.

## Generated model audit

Inspected codegen-core abstract_value, service_generation, coverage,
service_readiness, backend_names and service_codec, plus all three backend
`render_abstract_value` paths. Closed sums are compile-time enumerations of
concrete transitive descendants, including non-leaf concrete descendants;
abstract intermediates are ancestry, not payload variants. Name/layout preflight
protects generated identities but supplies no dynamic payload carrier.

| Backend | Base + A + B representation | Unknown generation-time subtype? |
|---|---|---|
| Ada | `Base_Kind` enum plus discriminated `Base` record; each arm stores A or B by value | No; discriminant and arms are exhaustive |
| Rust | `pub enum Base { A(A), B(B) }`; Debug/Clone and conditional PartialEq/Eq | No; exhaustive variants and typed fields |
| C++ | `struct Base { std::variant<A,B> value; };` | No; fixed alternative list |

Ada would need a reviewed envelope or tagged/classwide dispatch plus ownership,
possibly access types/heap objects. These are not present and must not be introduced
implicitly. Unchecked conversion, uncontrolled dispatch and unbounded opaque
bytes have SPARK proof/resource-policy consequences. An envelope need not require
all those mechanisms, but its boundedness and proof obligations must be designed.

Rust could theoretically use a tagged Value/bytes envelope, Box/Arc trait object,
Any or callbacks/registry. That is **new public API**, not today's enum. Box implies
unique ownership, Arc shared ownership; trait-object Clone/equality require explicit
contracts, downcast/type identity and Send/Sync bounds are not automatic. A Value
carrier provides structural clone/equality but couples the public model to JSON,
not schema-typed field invariants. Vec<u8> needs encoding/identity/validation rules.
Registry callbacks need lifetime/thread-safety/version rules. No new MSRV requirement
is established here; dependencies and any candidate features must be checked against
workspace rust-version 1.85 and the project's tested Rust 1.95 policy, not assumed.

C++ would need virtual ownership (`unique_ptr`/`shared_ptr`) or type erasure/any or
an envelope. unique_ptr breaks implicit copying; shared_ptr changes aliasing/lifetime;
virtual clone/equality and destructor rules need explicit design. std::any is not a
serialization registry. All alter generated APIs; C++17 availability of facilities
does not prove interoperable wire behavior or finite recursive ownership.

## Rust OMS JSON executable audit

Current codec consumes `serde_json::Value` at the codec seam, but **the generated
model does not store unknown Value objects**. Generated `decode_tNNN` functions
match statically known concrete `$type` names. Concrete record `check_members`
accepts `$type` only if equal to its own QName; all unknown members are rejected.
For OAM, names are bare; otherwise they are Clark QNames. There is no field-based
subtype guessing. Abstract targets require a string `$type` and dispatch to a known
concrete decoder; missing/type-mismatched tags are errors.

Executed production-generated controls:

| Input | Result |
|---|---|
| Known abstract `$type` BoxShape | Decodes; semantic typed encode/decode round trip |
| Wrong known `$type` CircleShape with BoxShape Width | `ShapePayload.{urn:shape}Shape: unknown member "{urn:shape}Width"` |
| Unknown `$type` `{urn:private}NeverGenerated` | `...: $type "{urn:private}NeverGenerated" is not a known concrete ShapeBase` |
| Correct BoxShape + unknown nested payload member | `...: unknown member "{urn:private}Payload"` |
| Concrete PublicA + own `$type` | Accepted; encoder omits unnecessary concrete tag |
| Concrete PublicA + wrong known/unknown `$type` | `Holder.{urn:audit}Value: $type "{urn:audit}PrivateB" is not "{urn:audit}PublicA"` (same with PrivateC) |
| Abstract Base + private generation-time-known PrivateB | Decodes/encodes both Flag/Enabled and PrivateB identity |
| Abstract Base + future PrivateC | `Holder.{urn:audit}Value: $type "{urn:audit}PrivateC" is not a known concrete Base` |
| PrivateB + extra Secret object | `Holder.{urn:audit}Value: unknown member "{urn:audit}Secret"` |

Permanent `task068_open_contract.rs` asserts exact unknown/type/member errors and
known semantic round-trip; existing generated_codec_qname tests cover missing and
wrongly qualified tags. Frozen matrix contains private/concrete exact outputs.
No byte-equivalence promise: JSON parsing collapses duplicate keys, and encoding
normalizes member order; concrete type tags may be omitted where redundant.
A detached Value can hold a JSON object, but it cannot inhabit today's generated
closed enum as an unknown variant. Unknown typed-model round-trip therefore does
not exist. Transport-level Value use is not a polymorphic-model solution.

## Runtime registry and language symmetry

| Capability | Current result |
|---|---|
| Register derived type at runtime | ABSENT |
| Runtime subtype/accessor registry | ABSENT |
| Dynamic `$type` / xsi:type decoder registry | ABSENT |
| Type-erased **subtype payload** storage in generated model | ABSENT |
| Unknown extension payload + identity retention | ABSENT |
| Runtime message subscription/handler dispatch | EXISTING, by subscription ID, not subtype |
| serde_json Value at Rust codec/transport seam | EXISTING, not unknown generated model storage |
| Ada/C++ equivalent unknown identity/payload ownership + round-trip contract | ABSENT |

`runtime-api-rust` exposes only generic PublishAdapter/SubscribeAdapter.
`runtime-rust` exposes `OmsJsonCodec<P>` with Send + Sync + 'static provider.
The worker's type-erased `Dispatch` is a typed message handler stored by subscription
ID; it neither registers derived schema types nor preserves unknown values.
Generated facade codecs are static. No Ada/C++ runtime equivalent is implemented;
core codec readiness explicitly returns LanguageNotImplemented for them.
Rust alone hypothetically retaining a Value would not establish language-neutral
OpenExtensions. No reviewed policy allows silently different value universes.

## Recursion

Ten reachable target families have recursive known-descendant value graphs in
both releases:

`CapabilityCommandBaseType`, `ComponentConfigurationPET`, `DataRecordBaseType`, `MessageType`, `QueryPET`, `StoreLoadoutItemPET`, `SubsystemCommandBaseType`, `SubsystemMaintenanceTestCommandPET`, `SubsystemMaintenanceTestPET`, `SubsystemMaintenanceTestResultPET`.

All other measured target rows are acyclic (including zero-known-descendant
families). The fixture freezes counts/depth/classification for every target.
Recursive by-value generated emission fails closed today. Future typed dynamic
objects need finite indirection/ownership, cycle/alias policy and bounded resource
handling; opaque JSON has finite tree storage but does not by itself solve typed
registry recursion, SPARK resource limits, or cross-language representation.
No recursion solution is added here.

## Generation-time private extension experiment

Production frontend overlay composition, public one-message resolution/projection,
readiness and backend renderers were exercised with:

- public abstract Base, PublicA(PublicFlag: boolean), Holder.Value: Base;
- private PrivateB extends Base with **Flag** and **Enabled** booleans;
- HolderReport selects Holder.

| Schema set | World | Result |
|---|---|---|
| Public only | ClosedSchemaSet | support PublicA; Base closed enum/sum has PublicA |
| Public + private | ClosedSchemaSet | support PublicA + PrivateB; both variants generated |
| Public only | OpenExtensions | exact NotClosedUnderOpenExtensions(Base) |
| Public + private | OpenExtensions | same exact error; future PrivateC remains possible |

A separate exact generic control uses ConcreteA extends Base, Holder.Value: Base,
and HolderReport; Closed adds ConcreteA support and generates all three models,
Open returns the exact Base error, with three-backend readiness parity.

Public/private Closed readiness is green in Ada/Rust/C++. Private combined model
compiled with GNAT, rustc (edition 2024), and strict C++17 (-Wall -Wextra -Werror).
Production `service-generate --extension private=... --with-codec` generated the
Rust codec. Decoding PrivateB and re-encoding preserved both fields and type
identity semantically. It did not preserve a never-generated PrivateC.

Optional zero-public-descendant repeated overlay control is already present in
Task029's fixture and executable tests. All three were rerun: private derived
payload supplies a real closed-sum alternative and repeated storage, generated
models compile (GNAT and strict C++ controls available), and Open still fails.
This confirms the workflow, not general multi-namespace overlays, every schema
shape, nor a runtime-open implementation. Normal backend and codec gates remain.

## External authoritative evidence

Task027's CAL 2.3.2 commit `87409b91e163931b6ac0905134367e5fb48729c9`
remains **secondary**, not normative for 2.5/2.6. GitHub repository searches for
UCI CAL 2.5/2.6 yielded no usable authoritative interface; unrelated results were
not treated as evidence. Pinned public UCI repositories were inspected; they
contain standards, schema/style/normalized-interface documents, not a reproducibly
identified current CAL implementation/API contract for runtime-unknown dispatch.

The exact pinned 2.5/2.6 DOCX files were extracted and searched for derived types,
xsi:type, registry, accessor, serialization and common abstraction terminology.
Their hashes and relevant hits are frozen in the input manifest. Style/design
specifications describe PET access to derived types; that does not define a
registry, unknown decoder fallback, safe ownership or lossless unknown payload
round-trip. No authoritative pinned CAL 2.5/2.6 runtime contract was identified.
This is an availability finding from these reproducible searches, not a claim
that no private/adopting-program CAL exists.

## Representation strategy decision matrix

Scores describe **now**, not an implemented future feature. “Potential” means a
new reviewed contract is necessary, not a pass. Determinism of generation is
separate from determinism/registration order at runtime.

| Strategy | Fidelity / round trip / identity | Ada/SPARK | Rust ownership | C++ ownership | Codec / runtime dependencies | Recursion | Determinism | Private extensions | API churn | Feasible now |
|---|---|---|---|---|---|---|---|---|---|---|
| A Status quo fail-closed | Honest refusal; no false round-trip claim | Existing policy | Existing enum | Existing variant | Existing only | Reject recursive graph | Stable | Refuses unknown | None | **Yes, safe refusal** |
| B Known descendants only in Open | Incomplete; cannot preserve unknown identity/payload | Current sum | Current enum | Current variant | Static codec | Existing rejection | Stable but wrong universe | Drops/rejects outside list | Semantic break | **Reject** |
| C Known + payload-less Unknown | Loses payload; may lose identity | Marker possible, not faithful | Unit variant | Marker variant | New fallback | No payload graph | Stable | Not lossless | New variant | **Reject** |
| D Opaque bytes | Potential if identity + exact wire/media contract accompany bytes | Boundedness/proof policy needed | Vec/buffer ownership | vector/buffer ownership | New capture/replay contract | Finite bytes, typed cycles unspecified | Can be stable | Potential only | High | **Not established** |
| E JSON object + type name | Potential semantic JSON retention, not bytes/duplicates/schema semantics | No equivalent carrier contract | Value clone/structural equality, JSON API coupling | JSON library/envelope policy absent | JSON-specific codec + cross-language storage | Finite tree, resource limits needed | Generated carrier stable | Potential only | High | **Not established** |
| F Runtime registered typed extension | Potential for registered types; unknown-unregistered still needs fallback | Dynamic dispatch/access/SPARK decision | Box/Arc/trait ownership + clone/equality contract | Virtual/type-erased ownership + copy/equality | Registry/version/codec ABI required | Indirection/cycle policy | Runtime order must be defined | Only registered unless fallback | Very high | **Absent** |
| G Private schemas + ClosedSchemaSet | Complete within explicitly supplied universe; tested identity/fields | Existing sum/collections | Existing enum/owned values | Existing variant/value semantics | Existing generation/static codec | Existing recursion gate | Explicit overlay order | **Confirmed known extensions** | Variants change on regeneration | **Yes, scoped** |

Null record, empty struct, zero-field base object, unit/payload-less Unknown,
payload without identity, identity without payload, silent extension-field drop,
always-empty repeated list, treating abstract base as concrete, and known-only
Open enums all fail the decisive question: can an unseen legitimate subtype be
decoded, owned, identified, and re-encoded with its complete payload? **No.** They
are rejected. The fail-closed refusal is accepted; it makes no false preservation
claim. Closed supplied schemas are accepted only under their declared universe.

## Mandatory gate checklist

| Requirement | Proven now? |
|---|---|
| Unknown subtype representation | No |
| Lossless unknown payload representation in generated model | No |
| Safe generated ownership | No open carrier exists |
| Runtime unknown decode | No, rejected |
| Runtime unknown re-encode | No typed value exists |
| Repeated open values | No (known Closed repeated values are different) |
| Finite open recursive ownership | No policy/implementation |
| Ada/Rust/C++ behavior or reviewed divergence | No |
| No private data discarded | Fail-closed avoids discard; placeholders would violate it |
| Open not redefined as known descendants | Guard enforces this distinction |

Primary disposition: **OPEN_WORLD_RUNTIME_CONTRACT_NOT_ESTABLISHED**.
Independent secondary disposition: **GENERATION_TIME_EXTENSION_PATH_CONFIRMED**.

## Recommended next task

Do **not** remove the projection guard. First review a language-neutral runtime
contract/ADR: subtype QName/wire identity, unknown payload media/normalization,
bounded resource ownership, repeated storage, recursive indirection, registry and
fallback/version behavior, equality/copy/thread-safety, codec dependencies, and
explicit Ada/SPARK acceptance or reviewed backend divergence. Require executable
unknown-private-subtype round trips in all permitted backends before implementation
can change Open readiness. A smaller immediately useful task may improve documented
private-overlay Closed workflows without changing OpenExtensions.

## Validation and publication boundary

At BEFORE: cargo fmt --all -- --check, cargo check --workspace --all-targets,
and cargo clippy --workspace --all-targets -- -D warnings passed. Focused core
abstract-value, service-generation (16), service-readiness (30), generated Rust
QName codec (8), runtime Rust (10) and runtime-api tests passed on Rust 1.95.
Task028 controls (6/backend), Task029 overlay controls (2/backend) passed.
One initial runtime-api invocation used the directory name instead of package
name and failed to select a package; it was corrected to ams-gra-oms-runtime-api
and passed. Scratch frontend probes initially lacked the required message UCI
version/namespace and were corrected before the successful measurements.
Permanent Task068 codec regression passed (1). Generated private models compiled
in Ada/Rust/C++; generated private/concrete codecs executed successfully.

A post-edit validation attempt exhausted /tmp disk while writing compiler caches.
Only Task068 disposable incremental artifacts were removed; validation was rerun
with CARGO_INCREMENTAL=0. No sibling artifacts were removed.

Post-edit cargo fmt --all -- --check, Rust 1.95 workspace check and clippy
(--all-targets, -D warnings), permanent codec regression and git diff --check
passed after that cleanup. Frozen fixture accounting and exact six readiness
message-set parity passed. The first pinned inventory reproduction was byte-identical.
The indexed path inventory also reproduced byte-identically. The exact six
coverage closure failure sets were independently measured and verified.
Synthetic/codec/compiler reproduction status is recorded in the PR. No historical multi-hour Deep campaign is claimed. Commit and
PR identifiers are publication metadata, not part of immutable BEFORE evidence.
Stop for review; do not merge or enable auto-merge.
