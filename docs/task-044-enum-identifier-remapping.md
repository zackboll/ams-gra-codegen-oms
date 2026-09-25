# Task 044 — evidence-backed enumeration identifier remapping

**Historical evidence gate:** implementation initially stopped because the
leading-digit-only hypothesis failed: four selected Ada enumerations contain
the reserved word `AND`. Subsequent review explicitly authorized the second,
target-specific plain-enumeration reserved-word category. This report retains the
original evidence and its stop decision rather than retroactively hiding it.

## Inputs and baseline

Reviewed and actual starting `origin/main`: `184b2bac28d22c015a8d90c24f65760416b9b17c`.
The worktree was clean. `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`, and `git diff --check`
passed before changes; **868 tests passed**.

The inventory loaded normalized `SchemaIr` with `load_schema_set` using the
UCI 2.5 pinned checkout `093610b7753944059360d3236770ab446d039556`
(`03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd`)
and the UCI 2.6 extraction from
`78eb61b6112c8bffa40820c33124b57787fc5bd9`
(`UCI_MessageDefinitions_v2_6_0.xsd`). The inventory process completed with
exit status 0. Counts below are **wire-value occurrences**, not distinct strings;
one wire value can occur in several declarations.

| Release | Backend | Enumerations | Values | Legal identifier transformations | Leading-digit syntax failures | Other syntax failures | Reserved-word failures | Within-enum collisions |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2.5 | Ada | 725 | 7,766 | 7,640 | 126 | 0 | 40 | 0 |
| 2.5 | Rust | 725 | 7,766 | 7,640 | 126 | 0 | 0 | 0 |
| 2.5 | C++ | 725 | 7,766 | 7,640 | 126 | 0 | 0 | 0 |
| 2.6 | Ada | 710 | 7,767 | 7,641 | 126 | 0 | 41 | 0 |
| 2.6 | Rust | 710 | 7,767 | 7,641 | 126 | 0 | 0 | 0 |
| 2.6 | C++ | 710 | 7,767 | 7,641 | 126 | 0 | 0 | 0 |

"Legal identifier transformations" includes values that subsequently fail
reserved-word checking; the syntax and reserved counts are not disjoint.
Ada reserved values include `AND` in four selected security enums. The extra
2.6 Ada reserved occurrence is `VehicleFormationEnum.DELTA`. No existing
Ada enum literal matches a declared top-level type name in the scanned
pinned schema XSD; the proposed `Value_` prefix has no literal/type name
matches in those inputs either. This XSD-name check is an inventory observation,
not a substitute for backend preflight, which also registers generated helpers.

The 126 leading-digit occurrences represent 96 distinct wire strings in ten
enumerations per release. All are ASCII alphanumeric / underscore; they have
no other lexical defect. The candidate is the **wire spelling verbatim** for
Ada and the existing underscore-split upper-camel transformation for Rust/C++.
The detailed table below records each candidate and owner in both releases.

For a non-UCI `SignalEnum` containing `NORMAL` and `25X1`, baseline full
generation diagnostics were exactly:

```text
error: unsupported Ada IR construct: Ada cannot form a legal identifier from "25X1" in the members of SignalEnum
error: unsupported Rust IR construct: Rust cannot form a legal identifier from "25X1" in the members of SignalEnum
error: unsupported C++ IR construct: C++ cannot form a legal identifier from "25X1" in the members of SignalEnum
```

Baseline selected-service `service-check` for the non-UCI `SignalNotice`
message and its `SignalPayload` record reported `NOT READY`, 1/2 renderable
selected types, in all three languages, with `SignalEnum` named as the blocker
and the same language-specific `25X1` diagnostics.

## Selected-service scope gate

In **both** normalized releases, `DeclassExceptionEnum` has 22 leading-digit
values, including `25X1`, `25X1_EO_12951`, `25X2`–`25X9`, `50X1`,
`50X1_HUM`, `50X2`, `50X2_WMD`, `50X3`–`50X9`, and `75X`. In contrast,
`FGI_SourceOpenEnum`, `FGI_SourceProtectedEnum`, `OwnerProducerEnum`, and
`ReleasableToEnum` have **no** leading-digit values. Each contains exactly
the unsafe selected wire value `AND`, which is an Ada reserved word under the
existing case-insensitive preflight rule. These four are selected Ada blockers
recorded by Task 042. A prefix applied **only** to digit-starting values cannot
make any of them renderable. Generic reserved-word escaping is explicitly
outside the authorized narrow scope. Thus the hypothesis that *all* remaining
selected PositionReport blockers are leading-digit lexical failures is false.

At the original evidence gate, `Value_` was only a candidate, and no production
mapping had been implemented. The later review decision, implementation and
measured results are recorded below the historical table.

## All leading-digit values and current candidates

Each row applies to **both 2.5 and 2.6**. Every Ada candidate is rejected
because it starts with an ASCII digit; every Rust/C++ candidate is rejected
for the same reason after the existing upper-camel transformation. Distinct
wire values occurring in two enumerations appear once per owning declaration.

| Owning enumeration | Wire value | Current Ada candidate | Current Rust/C++ candidate |
| --- | --- | --- | --- |
| `CapabilityTransmitPowerEnum` | `70W` | `70W` | `70W` |
| `CapabilityTransmitPowerEnum` | `125W` | `125W` | `125W` |
| `CapabilityTransmitPowerEnum` | `200W` | `200W` | `200W` |
| `CommCapabilityEnum` | `5G` | `5G` | `5G` |
| `GCP_OffsetEnum` | `1METER` | `1METER` | `1METER` |
| `GCP_OffsetEnum` | `2METER` | `2METER` | `2METER` |
| `GCP_OffsetEnum` | `4METER` | `4METER` | `4METER` |
| `GCP_OffsetEnum` | `6METER` | `6METER` | `6METER` |
| `GCP_OffsetEnum` | `8METER` | `8METER` | `8METER` |
| `GCP_OffsetEnum` | `10METER` | `10METER` | `10METER` |
| `GCP_OffsetEnum` | `12METER` | `12METER` | `12METER` |
| `GCP_OffsetEnum` | `14METER` | `14METER` | `14METER` |
| `GCP_OffsetEnum` | `16METER` | `16METER` | `16METER` |
| `GCP_OffsetEnum` | `18METER` | `18METER` | `18METER` |
| `GCP_OffsetEnum` | `20METER` | `20METER` | `20METER` |
| `GCP_OffsetEnum` | `22METER` | `22METER` | `22METER` |
| `GCP_OffsetEnum` | `24METER` | `24METER` | `24METER` |
| `GCP_OffsetEnum` | `26METER` | `26METER` | `26METER` |
| `GCP_OffsetEnum` | `28METER` | `28METER` | `28METER` |
| `GCP_OffsetEnum` | `30METER` | `30METER` | `30METER` |
| `GCP_OffsetEnum` | `32METER` | `32METER` | `32METER` |
| `GCP_OffsetEnum` | `34METER` | `34METER` | `34METER` |
| `GCP_OffsetEnum` | `36METER` | `36METER` | `36METER` |
| `GCP_OffsetEnum` | `38METER` | `38METER` | `38METER` |
| `GCP_OffsetEnum` | `40METER` | `40METER` | `40METER` |
| `GCP_OffsetEnum` | `42METER` | `42METER` | `42METER` |
| `GCP_OffsetEnum` | `44METER` | `44METER` | `44METER` |
| `GCP_OffsetEnum` | `46METER` | `46METER` | `46METER` |
| `GCP_OffsetEnum` | `48METER` | `48METER` | `48METER` |
| `GCP_OffsetEnum` | `50METER` | `50METER` | `50METER` |
| `GCP_OffsetEnum` | `52METER` | `52METER` | `52METER` |
| `GCP_OffsetEnum` | `54METER` | `54METER` | `54METER` |
| `GCP_OffsetEnum` | `56METER` | `56METER` | `56METER` |
| `GCP_OffsetEnum` | `58METER` | `58METER` | `58METER` |
| `GCP_OffsetEnum` | `60METER` | `60METER` | `60METER` |
| `IFF_AltitudeResolutionEnum` | `25_FEET` | `25_FEET` | `25FEET` |
| `IFF_AltitudeResolutionEnum` | `100_FEET` | `100_FEET` | `100FEET` |
| `LateralAxisOffsetEnum` | `0_TO_2METERS` | `0_TO_2METERS` | `0TO2METERS` |
| `LateralAxisOffsetEnum` | `2_TO_4METERS` | `2_TO_4METERS` | `2TO4METERS` |
| `LongitudinalAxisOffsetEnum` | `0_TO_2METERS` | `0_TO_2METERS` | `0TO2METERS` |
| `LongitudinalAxisOffsetEnum` | `2_TO_4METERS` | `2_TO_4METERS` | `2TO4METERS` |
| `LongitudinalAxisOffsetEnum` | `4_TO_6METERS` | `4_TO_6METERS` | `4TO6METERS` |
| `LongitudinalAxisOffsetEnum` | `6_TO_8METERS` | `6_TO_8METERS` | `6TO8METERS` |
| `LongitudinalAxisOffsetEnum` | `8_TO_10METERS` | `8_TO_10METERS` | `8TO10METERS` |
| `LongitudinalAxisOffsetEnum` | `10_TO_12METERS` | `10_TO_12METERS` | `10TO12METERS` |
| `LongitudinalAxisOffsetEnum` | `12_TO_14METERS` | `12_TO_14METERS` | `12TO14METERS` |
| `LongitudinalAxisOffsetEnum` | `14_TO_16METERS` | `14_TO_16METERS` | `14TO16METERS` |
| `LongitudinalAxisOffsetEnum` | `16_TO_18METERS` | `16_TO_18METERS` | `16TO18METERS` |
| `LongitudinalAxisOffsetEnum` | `18_TO_20METERS` | `18_TO_20METERS` | `18TO20METERS` |
| `LongitudinalAxisOffsetEnum` | `20_TO_22METERS` | `20_TO_22METERS` | `20TO22METERS` |
| `LongitudinalAxisOffsetEnum` | `22_TO_24METERS` | `22_TO_24METERS` | `22TO24METERS` |
| `LongitudinalAxisOffsetEnum` | `24_TO_26METERS` | `24_TO_26METERS` | `24TO26METERS` |
| `LongitudinalAxisOffsetEnum` | `26_TO_28METERS` | `26_TO_28METERS` | `26TO28METERS` |
| `LongitudinalAxisOffsetEnum` | `28_TO_30METERS` | `28_TO_30METERS` | `28TO30METERS` |
| `LongitudinalAxisOffsetEnum` | `30_TO_32METERS` | `30_TO_32METERS` | `30TO32METERS` |
| `LongitudinalAxisOffsetEnum` | `32_TO_34METERS` | `32_TO_34METERS` | `32TO34METERS` |
| `LongitudinalAxisOffsetEnum` | `34_TO_36METERS` | `34_TO_36METERS` | `34TO36METERS` |
| `LongitudinalAxisOffsetEnum` | `36_TO_38METERS` | `36_TO_38METERS` | `36TO38METERS` |
| `LongitudinalAxisOffsetEnum` | `38_TO_40METERS` | `38_TO_40METERS` | `38TO40METERS` |
| `LongitudinalAxisOffsetEnum` | `40_TO_42METERS` | `40_TO_42METERS` | `40TO42METERS` |
| `LongitudinalAxisOffsetEnum` | `42_TO_44METERS` | `42_TO_44METERS` | `42TO44METERS` |
| `LongitudinalAxisOffsetEnum` | `44_TO_46METERS` | `44_TO_46METERS` | `44TO46METERS` |
| `LongitudinalAxisOffsetEnum` | `46_TO_48METERS` | `46_TO_48METERS` | `46TO48METERS` |
| `LongitudinalAxisOffsetEnum` | `48_TO_50METERS` | `48_TO_50METERS` | `48TO50METERS` |
| `LongitudinalAxisOffsetEnum` | `50_TO_52METERS` | `50_TO_52METERS` | `50TO52METERS` |
| `LongitudinalAxisOffsetEnum` | `52_TO_54METERS` | `52_TO_54METERS` | `52TO54METERS` |
| `LongitudinalAxisOffsetEnum` | `54_TO_56METERS` | `54_TO_56METERS` | `54TO56METERS` |
| `LongitudinalAxisOffsetEnum` | `56_TO_58METERS` | `56_TO_58METERS` | `56TO58METERS` |
| `MaxPOR_Enum` | `1_IN_1` | `1_IN_1` | `1IN1` |
| `MaxPOR_Enum` | `1_IN_2` | `1_IN_2` | `1IN2` |
| `MaxPOR_Enum` | `1_IN_4` | `1_IN_4` | `1IN4` |
| `MaxPOR_Enum` | `1_IN_8` | `1_IN_8` | `1IN8` |
| `MaxPOR_Enum` | `1_IN_16` | `1_IN_16` | `1IN16` |
| `TransponderAntennaOffsetLongitudinalEnum` | `0_TO_1METERS` | `0_TO_1METERS` | `0TO1METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `1_TO_2METERS` | `1_TO_2METERS` | `1TO2METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `2_TO_4METERS` | `2_TO_4METERS` | `2TO4METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `4_TO_6METERS` | `4_TO_6METERS` | `4TO6METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `6_TO_8METERS` | `6_TO_8METERS` | `6TO8METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `8_TO_10METERS` | `8_TO_10METERS` | `8TO10METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `10_TO_12METERS` | `10_TO_12METERS` | `10TO12METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `12_TO_14METERS` | `12_TO_14METERS` | `12TO14METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `14_TO_16METERS` | `14_TO_16METERS` | `14TO16METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `16_TO_18METERS` | `16_TO_18METERS` | `16TO18METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `18_TO_20METERS` | `18_TO_20METERS` | `18TO20METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `20_TO_22METERS` | `20_TO_22METERS` | `20TO22METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `22_TO_24METERS` | `22_TO_24METERS` | `22TO24METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `24_TO_26METERS` | `24_TO_26METERS` | `24TO26METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `26_TO_28METERS` | `26_TO_28METERS` | `26TO28METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `28_TO_30METERS` | `28_TO_30METERS` | `28TO30METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `30_TO_32METERS` | `30_TO_32METERS` | `30TO32METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `32_TO_34METERS` | `32_TO_34METERS` | `32TO34METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `34_TO_36METERS` | `34_TO_36METERS` | `34TO36METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `36_TO_38METERS` | `36_TO_38METERS` | `36TO38METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `38_TO_40METERS` | `38_TO_40METERS` | `38TO40METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `40_TO_42METERS` | `40_TO_42METERS` | `40TO42METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `42_TO_44METERS` | `42_TO_44METERS` | `42TO44METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `44_TO_46METERS` | `44_TO_46METERS` | `44TO46METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `46_TO_48METERS` | `46_TO_48METERS` | `46TO48METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `48_TO_50METERS` | `48_TO_50METERS` | `48TO50METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `50_TO_52METERS` | `50_TO_52METERS` | `50TO52METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `52_TO_54METERS` | `52_TO_54METERS` | `52TO54METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `54_TO_56METERS` | `54_TO_56METERS` | `54TO56METERS` |
| `TransponderAntennaOffsetLongitudinalEnum` | `56_TO_58METERS` | `56_TO_58METERS` | `56TO58METERS` |
| `UncertaintyEnum` | `1_SIGMA` | `1_SIGMA` | `1SIGMA` |
| `DeclassExceptionEnum` | `25X1` | `25X1` | `25X1` |
| `DeclassExceptionEnum` | `25X1_EO_12951` | `25X1_EO_12951` | `25X1EO12951` |
| `DeclassExceptionEnum` | `25X2` | `25X2` | `25X2` |
| `DeclassExceptionEnum` | `25X3` | `25X3` | `25X3` |
| `DeclassExceptionEnum` | `25X4` | `25X4` | `25X4` |
| `DeclassExceptionEnum` | `25X5` | `25X5` | `25X5` |
| `DeclassExceptionEnum` | `25X6` | `25X6` | `25X6` |
| `DeclassExceptionEnum` | `25X7` | `25X7` | `25X7` |
| `DeclassExceptionEnum` | `25X8` | `25X8` | `25X8` |
| `DeclassExceptionEnum` | `25X9` | `25X9` | `25X9` |
| `DeclassExceptionEnum` | `50X1` | `50X1` | `50X1` |
| `DeclassExceptionEnum` | `50X1_HUM` | `50X1_HUM` | `50X1HUM` |
| `DeclassExceptionEnum` | `50X2` | `50X2` | `50X2` |
| `DeclassExceptionEnum` | `50X2_WMD` | `50X2_WMD` | `50X2WMD` |
| `DeclassExceptionEnum` | `50X3` | `50X3` | `50X3` |
| `DeclassExceptionEnum` | `50X4` | `50X4` | `50X4` |
| `DeclassExceptionEnum` | `50X5` | `50X5` | `50X5` |
| `DeclassExceptionEnum` | `50X6` | `50X6` | `50X6` |
| `DeclassExceptionEnum` | `50X7` | `50X7` | `50X7` |
| `DeclassExceptionEnum` | `50X8` | `50X8` | `50X8` |
| `DeclassExceptionEnum` | `50X9` | `50X9` | `50X9` |
| `DeclassExceptionEnum` | `75X` | `75X` | `75X` |

## Review-authorized scope extension: Ada reserved enum literals

The original leading-digit-only hypothesis and stop above remain the historical
evidence gate. Review subsequently authorized a second **plain-enumeration-only**
lexical category: otherwise-valid Ada identifiers equal (case-insensitively) to
an Ada reserved word. The same target-specific policy applies to any Rust or
C++ enum variant whose ordinary generated spelling is reserved; no such
occurrence exists in either pinned release. The
`Value_` prefix is applied to the original wire spelling *before* the existing
backend transformation. An exhaustive check of all normalized enum variants
in both pinned releases found **zero** within-enum collisions after both
remappings; safe values retain their pre-Task-044 spelling. The 40/41 counts
are occurrences across owning declarations; there are 18 distinct reserved
wire values in 2.5 and 19 in 2.6. The additional 2.6 value is `DELTA`.

A fresh continuation inventory of normalized SchemaIr independently reproduced
725/710 enumerations, 7,766/7,767 wire occurrences, 126 digit-leading
occurrences in ten owners per release, 40/41 Ada reserved occurrences in
35/36 owners, **zero** other syntax failures, **zero** Rust/C++ reserved
occurrences, **zero** residual invalid or reserved names, and **zero** post-map
within-enum collisions (including Ada case-insensitive identities). The four
selected reserved-only owners remain `FGI_SourceOpenEnum`,
`FGI_SourceProtectedEnum`, `OwnerProducerEnum`, and `ReleasableToEnum`.

| Wire value | 2.5 occurrences and owners | 2.6 occurrences and owners | Ada generated name |
| --- | --- | --- | --- |
| `ABORT` | 2: `FailureGuidanceEnum`, `SubsystemMaintenanceCommandEnum` | 2: `FailureGuidanceEnum`, `SubsystemMaintenanceCommandEnum` | `Value_ABORT` |
| `ALL` | 6: `ComponentStatusRequestEnum`, `DataUpdateRequestCategoryEnum`, `PlanPolicyProcessingEnum`, `RequirementObjectEnum`, `StoreJettisonOptionsEnum`, `VulnerabilityTypeEnum` | 6: `ComponentStatusRequestEnum`, `DataUpdateRequestCategoryEnum`, `PlanPolicyProcessingEnum`, `RequirementObjectEnum`, `StoreJettisonOptionsEnum`, `VulnerabilityTypeEnum` | `Value_ALL` |
| `AND` | 5: `LogicalOperatorEnum`, `FGI_SourceOpenEnum`, `FGI_SourceProtectedEnum`, `OwnerProducerEnum`, `ReleasableToEnum` | 5: `LogicalOperatorEnum`, `FGI_SourceOpenEnum`, `FGI_SourceProtectedEnum`, `OwnerProducerEnum`, `ReleasableToEnum` | `Value_AND` |
| `BEGIN` | 1: `ActivityTransitionEnum` | 1: `ActivityTransitionEnum` | `Value_BEGIN` |
| `BODY` | 3: `ElevationScanStabilizationEnum`, `LOS_MeasurementEnum`, `LOS_ReferenceEnum` | 3: `ElevationScanStabilizationEnum`, `LOS_MeasurementEnum`, `LOS_ReferenceEnum` | `Value_BODY` |
| `DELAY` | 1: `EffectTypeEnum` | 1: `EffectTypeEnum` | `Value_DELAY` |
| `DELTA` | 0: absent | 1: `VehicleFormationEnum` | `Value_DELTA` |
| `END` | 2: `ActivityTransitionEnum`, `NavigationConstraintSupportEnum` | 2: `ActivityTransitionEnum`, `NavigationConstraintSupportEnum` | `Value_END` |
| `EXIT` | 1: `OpInteractionEnum` | 1: `OpInteractionEnum` | `Value_EXIT` |
| `FOR` | 1: `CapabilityCoverageAreaTypeEnum` | 1: `CapabilityCoverageAreaTypeEnum` | `Value_FOR` |
| `LIMITED` | 1: `CrashServiceEnum` | 1: `CrashServiceEnum` | `Value_LIMITED` |
| `NEW` | 9: `CommandStateEnum`, `CommConfigurationStateEnum`, `CommPointingRequestEnum`, `NotificationPerspectiveEnum`, `ObjectStateEnum`, `RequestStateEnum`, `ResourceAllocationStateEnum`, `RF_ReportStateEnum`, `SignalReportStateEnum` | 9: `CommandStateEnum`, `CommConfigurationStateEnum`, `CommPointingRequestEnum`, `NotificationPerspectiveEnum`, `ObjectStateEnum`, `RequestStateEnum`, `ResourceAllocationStateEnum`, `RF_ReportStateEnum`, `SignalReportStateEnum` | `Value_NEW` |
| `OR` | 1: `LogicalOperatorEnum` | 1: `LogicalOperatorEnum` | `Value_OR` |
| `PACKAGE` | 1: `MessageTypeEnum` | 1: `MessageTypeEnum` | `Value_PACKAGE` |
| `RANGE` | 2: `DF_ModeEnum`, `MeasurementTypeEnum` | 2: `DF_ModeEnum`, `MeasurementTypeEnum` | `Value_RANGE` |
| `SELECT` | 1: `SubsystemMaintenanceCommandEnum` | 1: `SubsystemMaintenanceCommandEnum` | `Value_SELECT` |
| `TASK` | 1: `MessageTypeEnum` | 1: `MessageTypeEnum` | `Value_TASK` |
| `TERMINATE` | 1: `PairingRelationshipEnum` | 1: `PairingRelationshipEnum` | `Value_TERMINATE` |
| `XOR` | 1: `LogicalOperatorEnum` | 1: `LogicalOperatorEnum` | `Value_XOR` |

## Implemented codegen policy and synthetic evidence

`generated_enum_variant_name(language, wire_value)` is the sole plain-enum
identifier policy for preflight, all three renderers, and Ada's independent
flat-package enum-literal/type conflict analysis. It never writes an identifier
into language-neutral IR. Only the following wire-value categories receive a
fixed `Value_` prefix **before** the existing backend transformation:

* initial ASCII digit, with all other lexical conditions already accepted by
  that backend's enumeration naming model: Ada `25X1 → Value_25X1`,
  `25_FEET → Value_25_FEET`; Rust/C++ `25X1 → Value25X1`,
  `25_FEET → Value25FEET`;
* Otherwise syntactically valid identifier reserved in its target: Ada
  `AND → Value_AND` (case-insensitive). Rust/C++ leave `AND` unchanged because
  their ordinary generated spelling is legal; the Rust synthetic `Self` control
  instead maps to `ValueSelf`. No pinned UCI enum hits a Rust/C++ reserved word.

Safe values (`UNKNOWN`, `FRIENDLY`, `Red`, `SOME_VALUE`) use the exact old
transformation. Malformed values and non-enum names are not escaped. Two distinct wire values converging on one generated name
fail closed, e.g. `25X1` / `Value_25X1` in all three backends and `AND` /
`Value_AND` in Ada. A remapped Ada literal colliding with a top-level type
marks **both** the owning enumeration and the type unsafe; literals belonging
to separate Ada enumerations retain their existing overload semantics.

The normalized synthetic `SignalCode` fixture has safe members, all nine
observed leading-digit representative shapes, `AND`, the Rust reserved `Self`
control, and a `SignalPayload`
record that stores the enumeration. `EnumVariant.wire_value` remains `25X1`
and `AND`; schema-docs pages and the search index contain the original values,
not `Value_25X1` or `Value_AND`. Direct helper/renderer agreement tests cover
all three backends; the exhaustive documented 126 owner/value rows are also
checked in a focused test. GNAT compiles and runs an Ada client assigning
every mapped literal (including `Value_AND`), `rustc` compiles and runs a client
constructing every variant, and strict
`g++ -std=c++17 -Wall -Wextra -Werror -pedantic-errors` compiles and runs the
equivalent C++ client.

Baseline synthetic digit-only `SignalEnum` / `SignalNotice` service-check was
NOT READY (1/2 selected types) for Ada, Rust and C++. The new synthetic
selected fixture is READY (2/2) in all three, and service-generate output
compiles in all three. The separate `AND`-only synthetic selection is READY
in all three after this change; the Ada-only GNAT client also compiles and
runs. On the original main SHA its Ada service-check was NOT READY (1/2),
with boundary `Ada name "AND" generates reserved word "AND" in the members of
SignalCode`; Rust and C++ were already READY (2/2). The unchanged Rust/C++
spelling is explicitly asserted in the compiler tests. CI requires both new
GNAT-backed probes to execute by name, rejecting a zero-match test filter.

This is a selected *type-model* capability, not a wire codec, serialization,
transport, CAL, WebSocket, runtime interoperability, arbitrary string
sanitization, declaration/field/Choice/namespace remapping, or a generic
reserved-word policy. Full-schema reserved declaration/member failures are
outside Task 044.

## Twelve-cell coverage comparison

The baseline and final reports below use the same pinned root and world,
rerun with the actual `origin/main` base binary and Task 044 binary. Each
entry is fully renderable declarations before → after; neither release changes
kind, field-type, or field-occurrence counts. UCI 2.5 has 5,557 declarations,
5,448 supported kinds, 13,147/13,160 field types, and Ada 13,150/13,160
field occurrences (Rust/C++ 13,160/13,160). UCI 2.6 has 5,570 declarations,
5,461 supported kinds and 13,198/13,198 field types and occurrences.

| Release | World | Ada | Rust | C++ | Message closures before → after |
| --- | --- | --- | --- | --- | --- |
| 2.5 | closed-schema | 5318 → 5363 | 5395 → 5405 | 5398 → 5408 | 0/722 → 0/722 |
| 2.5 | open-extensions | 5233 → 5278 | 5307 → 5317 | 5310 → 5320 | 0/722 → 0/722 |
| 2.6 | closed-schema | 5339 → 5385 | 5417 → 5427 | 5421 → 5431 | Ada 0 → 323/725, Rust 0 → 354/725, C++ 0 → 352/725 |
| 2.6 | open-extensions | 5254 → 5300 | 5329 → 5339 | 5333 → 5343 | Ada 0 → 316/725, Rust 0 → 347/725, C++ 0 → 345/725 |

Independent generated-name attribution removes 45 Ada and ten Rust/C++ unsafe
enum owners in 2.5, and 46 Ada and ten Rust/C++ owners in 2.6, in **each** world.
Those are ten digit-leading enums per release, plus 35/36 reserved-only Ada
enums. All declaration gains equal those enum-owner counts; there is no
additional transitive declaration gain. UCI 2.6 message closures gain
transitively where the affected enums were the remaining blockers:
323/354/352 in closed world and 316/347/345 in open world (Ada/Rust/C++).
Earlier saved "baseline" reports for 2.5 open-world and 2.6 had already-improved
counts; the fresh run with the actual baseline binary recovered the true
pre-change values. Coverage is not a count of bad wire-value occurrences. The
full-schema first failures are reported separately below.

## Authoritative selected PositionReport recheck (not a forced milestone)

The UCI 2.5 closed-world contract selecting only PositionReport was rechecked
after implementing both authorized enum categories. Task 042's baseline was
Ada 54/60 and Rust/C++ 58/60. All three new reports are **59/60, NOT READY**:
the five Ada / one Rust / one C++ enum declarations became renderable, but
`SecurityInformationType` remains the one unsupported selected declaration.
Its actual UCI 2.5 Record contains two direct primitive `xs:dateTime` fields, `DeclassDate` and
`CUI_DecontrolDate` (optional 0..1), not references to the supported named
Zulu-pattern temporal carrier. The existing backend capability explicitly
rejects direct primitive `xs:dateTime` fields (see the Task 036 direct temporal
reference inventory in `docs/backend-compatibility.md`). This is a separate
temporal representability boundary, not an enumeration identifier. It remains
out of Task 044 scope; no UCI declaration or field is special-cased.
Because readiness is NOT READY, selected PositionReport `service-generate` is
not run, and no compiler evidence for that particular UCI
selection is claimed. The synthetic selected models compile in all languages.

## Full-schema first failures

The existing first-failure reserved-name boundaries remain unchanged by this
enum-only task (checked with full-schema `generate`, closed-schema):

| Release | Ada | Rust | C++ |
| --- | --- | --- | --- |
| 2.5 | `AltitudeRangePairType.Range` (reserved `Range`) | `ConfigurationParameterType.Type` (reserved `type`) | `ApprovalResponseType.Operator` (reserved `operator`) |
| 2.6 | `AltitudeRangePairType.Range` (reserved `Range`) | `ConfigurationParameterType.Type` (reserved `type`) | `COMINT_ChangeDwellType.Delete` (reserved `delete`) |

These Record-member failures are outside plain-enumeration wire-value
remapping. Full-UCI generation was not claimed to succeed.

## Validation

The baseline on `184b2bac28d22c015a8d90c24f65760416b9b17c` passed 868
workspace tests. Focused core naming tests, selected-service/frontend/docs
tests, and generated GNAT/rustc/C++17 compiler probes run explicitly, not with
a zero-match filter. All full workspace tests run with
`AMS_GRA_REQUIRE_GNAT=1`; formatting, all-target check, all-target clippy with
`-D warnings`, and `git diff --check` passed. A fresh rerun passed **877**
workspace tests (baseline **868**). Local tools: rustc/cargo 1.98.1, GNAT
14.2.0, and Debian g++ 14.2.0. The CI workflow installs floating Ubuntu
`gnat` and uses `dtolnay/rust-toolchain@stable`; neither is a pinned compiler
version. CI-observed Task 044 compiler versions require the new PR run. No
`/tmp` inventory trees or generated UCI artifacts are part of the repository
change.
