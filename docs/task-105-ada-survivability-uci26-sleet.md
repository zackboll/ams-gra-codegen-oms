# Task 105 — UCI 2.6 Ada SurvivabilityRiskLevel through pinned Sleet

## Main integration, isolation and disposition boundary

Task105 originally branched from Task104 commit
`660a680a856a6775eba937784ffa2f99c41d3f8f`, tree
`068c24ccd4572425fe3fa3bda1cf8f48628f3a48`. Task104 is now merged.
TASK105-R1 incorporates current main
`070a9f8b8d40b7e8a48c093af655ca882034f987` with an ordinary merge and
retargets PR #145 to `main`, preserving history and CI-RESET-001's simplified
architecture. Tasks102–104, full-main CI and existing Deep jobs remain unchanged.
PR #147 remains stacked on the Task105 branch; its branch, HEAD and base are
not modified by this reconciliation.

Historical qualification evidence remains at `/home/zboll/task105-scratch`.
Those executions retain their original source identities; they are not
relabelled as executions of the reconciled HEAD. Task104's UCI 2.5 transport
qualification is separate from Task105's UCI 2.6 transport qualification.

Final source/tree, local focused regression results and exact-head hosted
`PR smoke` outcome are external delivery evidence, not self-referential hashes
in this document. Merge readiness requires a clean incremental diff, passing
focused local checks, GitHub mergeability and exact-head `PR smoke` success.
Full GNAT14/16 qualification runs on main after merge, not on this PR.
Stop before approval, merge, auto-merge, release or publication.

## Authentic authorities

| Input | Immutable identity |
| --- | --- |
| UCI 2.6 release commit | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| UCI 2.6 MessageDefinitions SHA-256 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| UCI 2.6 SecurityMarkings SHA-256 | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| UCI 2.5 MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| UCI 2.5 SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| Sleet commit / tree | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` / `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Task105 authored fixture SHA-256 | `350a884fe52c8513260fe064fe5b527707c755b10536b4a2da4d1b9f30713071` |
| Generator | unchanged production CLI; Rust 1.95.0, Cargo.lock, closed-schema |
| GNAT / Alire / GPRbuild crate | 14.2.0 and 16.1.0 / 2.1.1 / 26.0.1 (binary reports 26.0.0) |
| GNATCOLL / protocol probe | 26.0.0, source archive hash enforced by inherited guard / websockets 15.0.1 |

Both releases were fetched afresh outside the repository. Each closure contains
exactly MessageDefinitions and SecurityMarkings. No schema, broker, production
generator, runtime or codec implementation is modified. Transport dependencies
remain AHT `664d5ddeb568626b2f9ff66054c7091894b3b8d4` and websocket-ada
`765d9cd612838eb00645037977c8cc9af7171c3e`.

## Independent schema comparison, not count-based equivalence

`scripts/task105/schema.py` independently follows named XSD references from
global `SurvivabilityRiskLevel` across both authentic documents. It does not
use generator IR/traversal or generated code. Each release has 46 selected
named declarations including the global element, hence 45 types. Fresh candidate
CLI inventory reproduced **45/45 model types, 44/44 codec declarations READY**.

The risk declarations remain records: RiskSetting 1..4 ordered records, each
with required HIGH/MEDIUM/LOW/NONE priority and RiskLevel 1..5 ordered records.
AllowedRisk is NONE/ACQUISITION/TRACK/LAUNCH/INTERCEPT/KILL/ALL; generated Ada
spells ALL as Value_ALL. Optional MaximumContinuousExposure contains required
CompositeExposureTime and CumulativeExposureTime named Duration values.
RiskLevel is **not a Choice**. Real Choices are repeated OwnerProducer entries.
OwnerProducer alternatives/cardinalities, UUID variant/nil constraint and
UCI_SchemaVersionStringType syntax are unchanged.

There are three semantic declaration changes:

* SecurityInformationType.DeclassDate and CUI_DecontrolDate change from direct
  xs:dateTime to uci:DateTimeType, requiring Zulu time. The named DateTimeType
  definition itself moves from the root to SecurityMarkings without changing its
  constraint. Both 2.6 fields now use that generated named model type.
* WhitespaceVisibleString1024Type and WhitespaceVisibleString4096Type change
  minLength/pattern minimum from zero to one and remove whiteSpace=collapse.
  Generated 2.6 carriers retain whitespace and reject empty strings.

Declaration-version metadata also changes on MessageType, the global element,
SecurityInformationType and those two string types. Complete structural values
and metadata differences are retained in schema-comparison.json. Matching
readiness counts do not imply byte identity, behavioral identity or general
cross-release wire compatibility.

## Admission before typed implementation

Task105 owns `tests/fixtures/task105/survivability26.json`. It preserves the
full original-style Task092/104 authored record shape, with valid RFC4122 UUIDs,
U classification, USA and NATO:EXAMPLE OwnerProducer arms, HIGH/NONE risk,
UTC Timestamp, LIVE Mode and **002.6.0** SchemaVersion. Every member was checked
against authentic 2.6 declarations; the 2.5 fixture was not edited.

Before implementing the typed consumer, `scripts/task105/admission.py` observed
full schema load, loopback-only WebSocket `owp`, INIT/+OK/INFO, SUB/+OK,
verbatim PUB/+OK, actual MSG 1 and independently equal JSON, UNSUB/+OK,
WebSocket closure, server exit 0, shutdown complete and listening-socket release.
The server has one explicit allowed topic, task105-survivability. Exact fixture,
publication, wire transcript, source/tree, binary hash and logs are retained in
`/home/zboll/task105-scratch/admission/phase-a-26`. No Ada execution is claimed
for this independent admission gate.

## Generation and API boundaries

Task105 owns both UCI 2.6 input-only and bidirectional service contracts. For
each campaign the unchanged CLI generates with and without --with-codec.
Model and service API bytes must match across codec opt-in. Model/codec bytes
must match the input-only selection within **2.6**, not the 2.5 baseline.
The input endpoint remains byte-identical; the only intentional service API
addition is Exchange_Publish with output Publisher generic and descriptor.
Compiler-negative probes instantiate input Publisher / output Subscriber and
must fail specifically because those generic declarations do not exist.

2.6 removes the now-unused direct XML_Schema_Date_Time carrier, changes the two
optional security date carriers to DateTimeType, reorders schema dependencies,
and changes checked-string minLength/whitespace handling. Generated model body,
specification and codec body therefore legitimately differ from 2.5.
Fresh generation from the authentic **2.5** schema still must match Task104's
five frozen model/codec hashes. The original Task104 consumer, fixture,
qualification, job body and source-bound verifier remain unchanged.

## Typed risk observations and validation stages

The Task105 Ada consumer uses only generated SurvivabilityRiskLevelMT,
Encode/Decode_SurvivabilityRiskLevelMT, Service_API Subscriber/Publisher,
OMS_LACAL.Typed/Connections and WebSocket.Client. It checks UUID/version,
required fields, every risk sequence length/order/value, optional exposure
discriminants and durations, repeated OwnerProducer discriminants/values,
and exact generated codec round-trip stability. The maximum case also checks
the 2.6 named security dates, nonempty SAR marking and preserved ClassifiedBy
whitespace against independently authored values.

Six positives cover full, MEDIUM/ALL, LOW/TRACK, ordered settings,
four settings each with five ordered levels and exposure durations, and empty
optional AppliesToSystemID with canonical omission. Each receives independent
external broker admission/typed delivery and a generated Publisher echo.

The corpus contains 22 invalid documents, plus invalid Timestamp as a separately
classified decoder input. Broker/XSD cases include UUID variant, abbreviated
version, enums, missing/unknown nested members, wrong repeated-field kinds,
both under/over sequence bounds, multiple/unknown OwnerProducer arms, empty
2.6 constrained strings and offset security dates. Wrong global name and
malformed JSON are broker/document failures; unauthorized topic and unknown SID
are OWP authorization/state failures. Diagnostics were reproduced on 2.6, not
inferred from the 2.5 run. Independent generated codec execution checks exact
contextual reasons for all 23 inputs. Seven direct model probes check UUID,
version syntax, both sequence minima, both nonempty 2.6 string carriers and
Zulu DateTime, including under Assertion_Policy Ignore.

Sleet actually accepts `not-a-dateZ`; live Ada must report Decode_Error with no
typed callback, then deliver a valid message on the **same subscription**.
A second live subscriber witnesses post-unsubscribe publication while the
original handler count remains unchanged. Ada connection state, descriptor/task
counts, consumer/output-thread exits, server exit/shutdown and socket release
must all pass.

The schema constrains payload version **syntax**, not equality to the loaded
release: Sleet accepts/delivers payload SchemaVersion 002.5.0 under a 2.6 session.
This is observed separately after the typed consumer exits, never represented
as a 2.6 typed positive or invented rejection. Conversely INIT requesting
002.5.0 or abbreviated 002.6 is rejected as Unsupported-Schema with closure.
The release-qualified Ada handler independently requires exact 002.6.0.

## Mandatory execution and main-only Deep CI

The real-Sleet matrix is UCI **2.6 only** × GNAT 14.2.0/16.1.0 ×
Normal/Check/Ignore × O0/O2 = **12 cells, six per compiler**. Each cell requires
12 primary callbacks, one witness callback and one codec rejection. Each
compiler separately runs three policy-specific eight-check codec consumers,
69 independent invalid-input rejections and 21 direct model rejections.
Task104 retains its independent **2.5** 12-cell job; none is relabeled 2.6.

New main-only Deep job: `ada-real-uci-survivability-26-sleet`; artifacts:
`task105-ada-survivability-26-14` and `task105-ada-survivability-26-16`.
Successful parent run 41233400825 yielded measured assigned execution totals
of 7767/10053/11132/8915/10254 seconds on lanes 1..5. Accordingly Task105 assigns
GNAT14 to lane 1 and GNAT16 to lane 4. Measurements are job occupancy, not a
prediction of future queue latency. Every inherited assignment, five shared
main lanes, queue:max and manual-only benchmark boundary remain.

The job requires `github.ref == 'refs/heads/main'`, with no PR selection input
or aggregate-gate integration. Actual qualification failures fail the job;
diagnostics upload with `always()`. Its independent verifier checks each
Task105 bundle against current source, schema, compiler/toolchain, commands and
exits, generated source, actual wire delivery and cleanup. Wrong or duplicate
cells, forged markers, missing commands, wrong source/tree/run/attempt and
missing cleanup fail closed. Exact-path Git trust under isolated HOME and
bounded hidden-file uploads are retained. No policy modules, historical
inventory changes, automatic benchmark or credential upload are added.

Standalone verifier and Git trust-boundary controls live in
`scripts/test_task105_evidence.py`. Task105 implementation, verifier, tests,
Ada consumers and fixture paths independently select Ada smoke only through
the existing lightweight `scripts/pr_smoke.py` router. This reconciliation
also changes the shared router, so PR #145 runs Python, Rust and Ada smoke
plus pinned workflow syntax validation. There is no PR Deep workflow.

## Evidence accounting

Development01 stopped before compilation because its verifier source did not
yet exist. Development02 reproduced an exact diagnostic mismatch while adding
the independent 2.6 string probes. Development03 exercised all six GNAT14
transport cells but correctly failed end-of-campaign input integrity after
concurrent source edits. These are retained development evidence, not clean
qualification. Standalone expanded negative/model and version-boundary controls
subsequently passed; they do not substitute for the final clean matrix.

Historical clean-source campaigns retain their original source identities and
limitations. Reconciliation acceptance runs the retained Task105 verifier and
Git trust controls, Tasks102–104 verifier/unit regressions, PR routing/execution
tests, actual lightweight Ada smoke, pinned actionlint 1.7.12, whitespace checks
and an incremental-diff preservation audit. It does not repeat historical
pinned-UCI/Sleet campaigns or treat synthetic unit bundles as transport execution.
Full qualification remains the main-only Deep job's responsibility after merge.