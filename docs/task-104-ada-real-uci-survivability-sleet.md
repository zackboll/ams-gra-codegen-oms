# Task 104 — real-UCI Ada SurvivabilityRiskLevel through pinned Sleet

## Scope and stacked dependency

Task104 starts directly from remote Task103, reviewed commit
`454bac6332b03fd78e40fa547e56cafce62e8cb0`, tree
`71597fa7fe70c748c722abf08cbd89952399eeba`. PR #142 was OPEN and targeted
`feature/102-ada-real-uci-log-sleet`. The initial Task104 PR base is
`feature/103-ada-real-uci-rdma-sleet`, not main. No Task104 branch, worktree or
PR existed before creation. Task103/Task102 branches, worktrees, evidence and
PR metadata are not modified. PR #133 remained independently owned and open
during implementation; if merged, reconcile actual main with an ordinary merge.

Worktree: `/home/zboll/git/ams-gra-codegen-oms-task104`.
Evidence: `/home/zboll/task104-scratch`. Exact committed source/tree, hosted
run/attempt/artifact identities and final disposition are delivery evidence,
not self-referential claims embedded in this document. Development evidence is
never substituted for clean-source execution. No approval, merge, auto-merge,
release or publication is authorized by this qualification.

## Authentic inputs and admission first

| Input | Pin |
| --- | --- |
| Sleet commit | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| UCI 2.5 MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| UCI 2.5 SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| Original Task092 golden SHA-256 | `996ac91fa9eb177d14a6fc3fd7169e781a2bd206b62a410dd908f0d0750f2349` |
| Locally rebuilt Sleet binary SHA-256 | `4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94` |
| Generator | Rust 1.95.0, locked production CLI |
| Compiler / builder | GNAT 14.2.0 and 16.1.0, Alire 2.1.1, GPRbuild crate 26.0.1 |

The complete authentic schema closure is the root plus SecurityMarkings.
Admission checks both original hashes, include/import structure, global message
binding and original fixture hash. The golden was also compared directly with
Task092 commit `09a1b2e630acfda7e971bbe2524f3763a54e6b50`.

Before developing the consumer, a Task104-owned standalone loopback OWP probe
freshly built unchanged pinned Sleet with its lockfile, loaded the complete
schema, negotiated `owp`, completed INIT/INFO, subscribed to exact global element
`SurvivabilityRiskLevel`, published the complete golden body verbatim, observed
`+OK` and actual `MSG 1`, and independently compared received JSON. UNSUB,
WebSocket closure, socket release, server exit 0 and `shutdown complete` passed.
This is independent broker admission, not generated Ada execution. The probe is
retained at `/home/zboll/task104-scratch/admission`; subsequent campaigns rerun it.

## Schema shape: no invented risk Choices

The real payload is generated `SurvivabilityRiskLevelMT`, not a replacement JSON
record. Its wire global name has no `MT` suffix. The required header contains a
valid UUID and schema version `002.5.0`; repeated `OwnerProducer` contains a
required GovernmentIdentifier `USA` Choice and additional NATO_SpecialWord
`NATO:EXAMPLE` Choice. The positive golden has HIGH priority and AllowedRisk NONE.

The authentic UCI 2.5 XSD defines:

* `RiskSetting`: **1..4** `SurvivabilityRiskSettingType` records, ordered sequence.
* Each setting: required `RiskPrioritization` enum HIGH, MEDIUM, LOW or NONE;
  `RiskLevel`: **1..5** `RiskSettingType` records.
* Each level: required `AllowedRisk` (`VulnerabilityTypeEnum`): NONE,
  ACQUISITION, TRACK, LAUNCH, INTERCEPT, KILL, ALL. Ada remaps ALL to `Value_ALL`.
* Optional `MaximumContinuousExposure`: `VulnerabilityLevelsType`, with required
  `CompositeExposureTime` and `CumulativeExposureTime` DurationType values.
* Optional repeated `AppliesToSystemID`: 0..unbounded.

**RiskLevel and MaximumContinuousExposure are records, not Choices.** There are
no legitimate alternate RiskLevel Choice arms/discriminants to invent. The
actual Choice qualification is repeated SecurityInformation.OwnerProducer;
the optional exposure record has an actual generated Is_Present discriminant.
The harness checks the generated shape as declared, not the task's speculative
Choice terminology. Multiple risk arms are investigated as unknown record
members; exactly-one Choice rejection is tested on OwnerProducer.

The independent positive corpus contains six documents: unchanged golden;
MEDIUM/ALL; LOW/TRACK; two ordered NONE/HIGH settings; maximum four settings and
five ordered ACQUISITION/TRACK/LAUNCH/INTERCEPT/KILL levels per setting, with
PT1S/PT2S optional exposure and extra CAN/NATO:MORE owner Choices; and explicitly
empty optional AppliesToSystemID. NONE/ALL are not mixed with other risks,
respecting the XSD annotation. Empty RiskSetting/RiskLevel are negative cases.
Canonical generated encoding omits empty zero-minimum sequences.

## Production generation and byte preservation

The Task104 service contract has input Receive and output Publish endpoints.
The production CLI uses `--with-codec`, closed-schema selection and the portable
Ada OMS JSON runtime. Service_API, Service_Codec, OMS_LACAL.Typed,
OMS_LACAL.Connections and the existing WebSocket Client are used unchanged.
No parser, protocol, enum replacement or transport is introduced.

Candidate readiness reproduces **45 selected / 45 renderable types** and
**44/44 codec declarations READY**. A separately rebuilt historical Task092
generator produces byte-identical programs.ads, programs-oam.ads,
programs-oam.adb, service_codec.ads and service_codec.adb. These five independent
historical hashes are pinned in the Task104 evidence verifier. Adding the service
endpoints changes only the appropriate API; model/codec parity and with-codec
model/API parity are checked again in every compiler campaign.

Compiler-negative probes must report `"Publisher" not declared` on the generated
input and `"Subscriber" not declared` on the output, not just nonzero exit.
The original Task092 Survivability consumer remains byte-identical and must
finish with exactly `TASK092_REAL_PASS checks= 8` under each compiler/policy.

## Typed transport, rejection stages and cleanup

For every positive publication the Ada handler independently checks header UUID,
SchemaVersion, required/additional OwnerProducer discriminants and values, risk
setting lengths/order/priorities, nested level lengths/order/AllowedRisk values,
optional exposure presence/durations, optional system sequence and complete
encode/decode/encode stability. Exact ordered `TASK104_RISK_OBSERVED` records
are independently verified, in addition to actual callback and wire documents.

Each cell requires twelve primary callbacks (independent external positives plus
generated publications), six generated publications, one witness callback and
one recoverable decoder rejection. A second live typed subscription receives
the post-unsubscribe witness; the removed primary count must remain unchanged.
Descriptor/task counts, connection state, reader thread, sockets and clean
Sleet exit/shutdown are checked. Silence alone does not prove unsubscribe.

| Investigation | Observed stage / diagnostic |
| --- | --- |
| UUID invalid variant nibble | Broker/XSD FacetViolation, SystemID.UUID pattern |
| Abbreviated SchemaVersion | Broker/XSD FacetViolation, SchemaVersion pattern |
| Invalid RiskPrioritization | Broker/XSD FacetViolation, enumeration |
| Invalid AllowedRisk | Broker/XSD FacetViolation, enumeration |
| Unknown nested RiskLevel member | Broker/XSD UnknownMember, NotARiskAlternative |
| Number instead of repeated RiskLevel | Broker/XSD OccurrenceMismatch, expected_array |
| Both OwnerProducer Choice arms | Broker/XSD ChoiceViolation, present: 2 |
| Missing priority / AllowedRisk | Broker/XSD MissingRequiredMember, actual field |
| Empty/overfull RiskSetting or RiskLevel | Broker/XSD OccurrenceMismatch, actual field |
| MT type name used as global element | Broker/XSD UnknownGlobalElement |
| Unauthorized topic | OWP authorization Illegal-State, not authorized |
| Unknown subscription 987 | OWP state Illegal-State, unknown subscription id |
| Malformed OMS JSON | Broker JSON ingress Invalid-Message, invalid JSON payload |
| Timestamp `not-a-dateZ` | Actually broker-accepted/delivered; Ada codec invalid named DateTime |

The timestamp candidate was executed, not assumed: real Sleet accepted it, the
live Ada subscription reported Decode_Error without invoking its handler, then
the same subscription delivered the unchanged valid golden. This is not a mock
recovery claim. All thirteen invalid payloads plus timestamp independently enter
the generated codec; exact contextual reasons (including RiskSetting item 1 and
RiskSetting cardinality) are required. Four direct model probes require exact
Constraint_Error diagnostics for UUID, version and both required-sequence minima,
including under Assertion_Policy Ignore.

## Compiler matrix and source-bound evidence

Mandatory matrix: UCI **2.5 only** × GNAT 14/16 × Normal/Check/Ignore × O0/O2 =
**12 real-Sleet cells**, six per compiler. Each compiler also executes the
original 8-check Task092 consumer and independent codec/model probes under all
three policies. Every command/exit/stream, source identity, toolchain lock,
compiler/builder/linker identity, generated source hash, schema closure,
Sleet source/tree/binary, wire transcript and cleanup result is preserved.

Development02 completed all twelve cell-level transport checks but correctly
failed campaign integrity because verifier development changed its inputs while
running. Those results are development-only. Final clean-source campaigns and
the final independent verifier, not those cell logs, govern acceptance.

## Separate UCI 2.6 inventory

Pinned 2.6 MessageDefinitions:
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`;
SecurityMarkings:
`ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88`.
Read-only inspection finds global SurvivabilityRiskLevel bound to
SurvivabilityRiskLevelMT. The candidate CLI reports 45/45 model types and 44/44
codec declarations READY for this exact selection. This is static inventory,
**not GNAT or transport qualification**. No UCI 2.6 transport cell is included
or claimed equivalent. `scripts/task104/inventory.py` records that boundary.

## Additive CI and inherited security contracts

Task104 owns Deep job `ada-real-uci-survivability-sleet` and artifacts
`task104-ada-real-uci-survivability-14` / `-16`. Current five-lane inventory and
parent hosted timings were inspected before assigning GNAT14 to lane 4 and
GNAT16 to lane 1. Parent run 41226112543 measured RDMA14 ~14m25s, RDMA16 ~23m40s,
DLZ14 ~14m55s, DLZ16 ~24m25s and Log14 ~27m59s; other running jobs were not
invented as completed timings. Existing lane assignments and queue:max remain.

Task104 ownership projects back through unchanged Task103/Task102 contracts;
original mandatory Task100/101/102/103 job bodies and verifier requirements are
not replaced. Aggregate selection expands from 46 to 48 runner cells and 18 to
20 Deep cells; test occupancy expectations change accordingly. Python selection,
CI002, trusted dispatch, manual-only benchmark isolation, exact-path Git trust
under isolated HOME and bounded hidden hash-listed provider uploads remain.

The final aggregate resolves each selected artifact using authoritative
CI-RETRY-001 producer resolution before the Task104 independent verifier. Wrong
source/tree/run/attempt/compiler, absent/duplicate/expired artifacts, missing
matrix/steps, schema/toolchain drift, missing model observations, wrong rejection
stage or missing cleanup fail closed. Synthetic unit-test bundles are explicitly
not qualification evidence. Final hosted identities and any remaining merge
conditions must be reported separately after actual execution.