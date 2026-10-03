# Task 060 — alternating ASCII profiles and compiler/codec evidence

> **Current verification status:** the original local capability evidence passes,
> but hosted head `cdbf095` failed Fast (zero matched AND unit tests) and Deep
> real-uci (stale constrained-Binary ProductMetadata assertion). Corrective
> tests/workflow changes and explicit provenance are recorded below. Hosted
> final-head success remains mandatory; no merge/readiness claim is made.


## Pattern-group semantics

## PRE-PRODUCTION FINAL GATE

The boundary below is asserted by the pinned normalized-IR target
`uci_alternating_admission`, which compares all 38 release/declaration rows
against `tests/fixtures/string/task060-pinned-rows.tsv`, including exact pattern
order, grouping, dialect, effective facets and absence of extra facets. Both
pinned roots passed this gate locally: **2 tests passed in 11.54 seconds**.
This is an admission decision, not a claim that production support exists.

|Boundary|Declarations per release|Exact names|
|---|---:|---|
|Baseline|19|All rows in the frozen inventory below|
|ADMITTED|15|FIPS_CountryCodeType; IPv4_AddressType; MilitaryGridType; NITF_AIMIDB_MissionNumberType; NITF_CodewordsType; NITF_DeclassificationExemptionType; NITF_DeclassificationType; NITF_IPON_IID2_ProgramCodeType; NITF_MSTGTA_TargetCategoryType; NITF_MSTGTA_TargetPriorityType; NITF_PATCHB_GravityType; NITF_ReleasingInstructionsType; NITF_UTC_TimeType; NotationType; RecordOriginatorType|
|DEFERRED_UNICODE|3|NITF_DateAndTimeType; NITF_DateType; NITF_MSTGTA_TargetLocationType|
|DEFERRED_COMPLEX|1|IPv6_AddressType|

There are **15 unique exact admitted profiles per release**, and **16 across
both releases**: only MilitaryGridType differs. The profile count includes the
authoritative facet tuple; it is not a count of leaf branches. The exact rows
and count are asserted independently of declaration naming in the fixture test.
Production classification must compare constraints, not these names.

Expected OrderOfBattle blockers removed: MilitaryGridType, NotationType,
RecordOriginatorType. Expected remaining constrained-String support blockers:
**none**. Actual READY and generated compilation are not yet measured.

### Unicode deferral preserves the authoritative language

W3C XML Schema Part 2, Appendix F, multi-character escapes defines `\d` as
`\p{Nd}` (decimal digit category), not `[0-9]`:
<https://www.w3.org/TR/xmlschema-2/#regexs>.
The IR retains the expression/dialect, not an enumerated finite Unicode set.
All three declarations retain Unicode-digit positions compatible with their
explicit character-count lengths. For example, replacing an unconstrained year
digit in `20000101` or `200001010000` by Arabic-Indic U+0661 preserves the branch
shape and character count. The coordinate branch
`+12.123456+012.123456` has 21 characters and also has unconstrained `\d`
positions where such substitution is permitted. These are semantic witnesses,
not compiler-backed backend acceptance claims. Emitting ASCII-only validation
would incorrectly reject authoritative values. Deferral avoids that narrowing.

An independent libxml2/lxml XML Schema validation of synthetic restrictions
carrying these exact pinned facets and patterns confirms concrete Unicode
witnesses: `2١000101` (DateType, 8 characters), `2١0001010000`
(DateAndTimeType, 12), and `+1١.123456+012.123456` (TargetLocationType, 21)
all validate. This check is evidence-only; lxml is not a repository dependency
and no generated backend uses it.

### Coherent finite model and IPv4 admission

Pattern constraints are AND of restriction groups; each group is OR of exact
alternatives; each alternative consumes the whole value with independent state.
Admitted bodies use literals, finite ASCII classes, Task 059 repetitions and
finite expansion of deterministic subsequences/optional subsequences. No regex
parser, VM, recursion, source evaluator or platform networking parser is allowed.

IPv4 octet alternatives are `[0-9]`, `[1-9][0-9]`, `1[0-9][0-9]`,
`2[0-4][0-9]`, `25[0-5]`. Four such octets with three literal dots give
625 finite deterministic branch combinations. A renderer should share
equivalent private body helpers or delimiter-bounded octet validation rather
than copy all 625 bodies. Delimiter slicing must be justified by the exact row:
no octet alternative admits dot, so boundaries are unique. No integer or host
address parser is necessary. Leading zeros are allowed only for the one-digit
`0`, not multi-digit octets.

### IPv6 complexity deferral

The exact expression is finite, but has empty/variable-width hex runs,
0..5 repeated colon-terminated runs, an optional hex-run suffix and an embedded
decimal-address alternative whose optional decimal prefixes overlap. A direct
fixed-width/body expansion before deduplication has
`6 * (1+5+25+125+625+3125) * (6*6 + 6^4) = 31,216,752`
branch combinations. This is a size diagnostic, not a minimal-language bound.
Compact treatment would need separator ambiguity/overlapping optional-prefix
machinery not justified by the other admitted deterministic bodies, or a
profile-specific algorithm. Neither should dictate this task's architecture.
IPv6 is therefore DEFERRED_COMPLEX, not approximated or platform-parsed.

### MilitaryGrid release-specific decomposition

UCI 2.5: zone OR `[1-9]`, `[1-5][0-9]`, literal `60`, followed by
`[C-HJ-NP-X]`; optional square suffix consists of `[A-HJ-NP-Z]`,
`[A-HJ-NP-V]` and 0..5 exact digit pairs. Separate polar alternative is
`[ABYZ]` followed by optional `[A-CF-HJ-LP-UX-Z]`, `[A-HJ-NP-Z]` and
0..5 digit pairs. Explicit minLength=14/maxLength=15 stays independent.
The polar branch has maximum length 13, so **no polar-branch positive can
satisfy the explicit facets in 2.5**. Missing square suffixes also fail those
facets. Preserve those pattern branches nevertheless and test branch acceptance
separately from carrier rejection; do not fabricate a positive or pad it.

UCI 2.6: the same three zone forms and latitude class require both square
letters; the polar form requires both letters too. Either form is followed
by 0..5 exact digit pairs. Explicit minLength=3/maxLength=15 is separate.
`AAB` satisfies the 2.6 polar form and facets but fails 2.5 minLength;
`1CAA0000000000` satisfies both profiles. Synthetic renamed declarations must
admit both exact constraint tuples without relying on MilitaryGridType's name.

Likewise NITF_DeclassificationExemptionType's single-letter `[DNIO]` branch
is valid at the pattern layer but always rejected by length=4. The corpus must
record this unreachable carrier branch explicitly rather than invent padding.

### Complete admitted branch decomposition

The following is a semantic blueprint, not executable regex parsing. Fixed
classes retain the exact members of the corresponding frozen row. Finite repeat
expansion never synthesizes length facets. All rows have one restriction group;
multi-group behavior must be tested separately without admitting unobserved rows.

|Declaration/profile|Normalized alternatives|Deterministic semantic bodies|
|---|---:|---|
|FIPS_CountryCodeType|1|two uppercase letters OR two SPACE characters|
|IPv4_AddressType|1|four five-way octet bodies separated by exactly three literal dots; 625 flattened combinations, unique delimiter boundaries|
|MilitaryGridType 2.5|1|three zone forms × (absent square OR square plus 0..5 digit pairs), OR polar × (absent square OR square plus 0..5 pairs): 28 finite bodies before facet rejection|
|MilitaryGridType 2.6|1|three zone-square forms OR one polar-square form, each followed by 0..5 digit pairs: 24 finite bodies|
|NITF_AIMIDB_MissionNumberType|1|literal U0 + two digits OR two uppercase letters + two digits OR literal UNKN|
|NITF_CodewordsType|5|11 SPACE; 1/2/3 repetitions of uppercase pair + SPACE followed by 8/5/2 SPACE; three pair-SPACE bodies followed by uppercase pair|
|NITF_DeclassificationExemptionType|1|X + digit 1..8 + two SPACE; literal 25X + digit 1..9; one member of DNIO; four SPACE|
|NITF_DeclassificationType|1|literal DD, DE, GD, GE, O SPACE, X SPACE, or two SPACE|
|NITF_IPON_IID2_ProgramCodeType|1|digit + uppercase OR uppercase + digit|
|NITF_MSTGTA_TargetCategoryType|1|digit 1..9 + four digits OR five SPACE|
|NITF_MSTGTA_TargetPriorityType|1|digit + digit 1..9 + digit OR two digits + digit 1..9 OR digit 1..9 + two digits; overlapping branches are intentional|
|NITF_PATCHB_GravityType|1|literal 3 + digit 1..3 + literal dot + four digits OR seven SPACE|
|NITF_ReleasingInstructionsType|8|20 SPACE; 1..6 repetitions of uppercase pair + SPACE followed by 17/14/11/8/5/2 SPACE; six pair-SPACE bodies followed by uppercase pair|
|NITF_UTC_TimeType|2|hour branch 0..1 + digit OR literal 2 + digit 0..3, then digit 0..5 + digit twice, then literal Z; OR seven SPACE|
|NotationType|1|five uppercase alphanumeric members OR literal UNKN OR literal NONE|
|RecordOriginatorType|1|two uppercase letters OR literal E OR literal hyphen|

## Pattern-group semantics (continued)

Internal `|` is union within an expression. Alternatives within one normalized group are OR. Restriction-level groups are AND, in base-to-derived order. The frontend collects local patterns into one group and appends that group to inherited groups; flattening those groups would be incorrect.

## Inventory

Regenerated from pinned normalized IR using the unchanged Task 059 classifier. The complete unsupported String name set is asserted against all 19 names, not inferred from a prior markdown list. Counts below distinguish semantic message closures from generated-support projections. Projection failures are recorded separately, not treated as absence of support reach. Exact effective constraints include all numeric, length and lexical facets.

### UCI 2.5

Root SHA-256: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`.

|Qualified name|Groups / alternatives per group|Exact effective constraints|Effective whiteSpace|ASCII gate|Semantic message reach|Generated-support reach|OrderOfBattle selected / support|Branch assessment|
|---|---|---|---|---|---:|---:|---|---|
|`{https://www.vdl.afrl.af.mil/programs/oam}FIPS_CountryCodeType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(2), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[A-Z]{2}&#124;[ ]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|39|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}IPv4_AddressType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(7), max_length: Some(15), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])\\.){3}([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|11|1|false / false|five finite octet branches; four octets with literal dot separators; needs finite nested union/sequence, not a networking parser|
|`{https://www.vdl.afrl.af.mil/programs/oam}IPv6_AddressType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(2), max_length: Some(45), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "((:&#124;[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:&#124;[0-9a-fA-F]{0,4}))&#124;(((25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])\\.){3}(25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])))" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|11|1|false / false|finite hex runs (0..4), colon alternatives, 0..5 repeated runs, finite optional suffix, embedded decimal alternatives; model decision pending, no platform parser permitted|
|`{https://www.vdl.afrl.af.mil/programs/oam}MilitaryGridType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(14), max_length: Some(15), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([1-9]&#124;[1-5][0-9]&#124;60)[C-HJ-NP-X]([A-HJ-NP-Z][A-HJ-NP-V]([0-9]{2}){0,5})?&#124;[ABYZ]([A-CF-HJ-LP-UX-Z][A-HJ-NP-Z]([0-9]{2}){0,5})?" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|7|1|false / true|three zone alternatives plus polar alternative; finite 0..5 digit pairs; release-specific optional suffix in 2.5 versus mandatory letters in 2.6; finite expansion possible|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_AIMIDB_MissionNumberType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(4), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "((((U0)&#124;[A-Z]{2})[0-9]{2})&#124;UNKN)" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_CodewordsType`|1 / [5]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(11), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: " {11}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){1} {8}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){2} {5}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){3} {2}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){3}[A-Z]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|same-level OR of finite fixed-width uppercase pairs and exact literal SPACE padding; deterministic branch sequences|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DateAndTimeType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(12), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([12]\\d\\d\\d)((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01])([01][0-9]&#124;[2][0-3])([0-5][0-9]))" }, PatternExpression { dialect: XmlSchema, expression: " {12}" }] }], white_space: None } }`|Preserve|NON-ASCII: XSD \d|0|11|false / false|NOT ASCII-only: XML Schema \d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DateType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(8), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([12]\\d\\d\\d((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01]))" }, PatternExpression { dialect: XmlSchema, expression: " {8}" }] }], white_space: None } }`|Preserve|NON-ASCII: XSD \d|0|11|false / false|NOT ASCII-only: XML Schema \d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DeclassificationExemptionType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(4), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(X[1-8]( ){2})&#124;25X[1-9]&#124;[DNIO]&#124;( ){4}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|four internal alternatives; length=4 independently rejects the single-letter DNIO branch; retain this branch in semantics rather than invent padding|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DeclassificationType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(2), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(DD&#124;DE&#124;GD&#124;GE&#124;O( )&#124;X( )&#124;( ){2})" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_IPON_IID2_ProgramCodeType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(2), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([0-9][A-Z]&#124;[A-Z][0-9])" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_MSTGTA_TargetCategoryType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(5), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[1-9][0-9]{4}&#124;[ ]{5}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_MSTGTA_TargetLocationType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(21), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([0-8]\\d([0-5]\\d){2}\\.\\d{2}(N&#124;S)(0\\d{2}&#124;1[0-7]\\d)([0-5]\\d){2}\\.\\d{2}(E&#124;W))" }, PatternExpression { dialect: XmlSchema, expression: "([\\+\\-]{1}[0-8]\\d\\.\\d{6}[\\+\\-]{1}(0\\d{2}&#124;1[0-7]\\d)\\.\\d{6})" }] }], white_space: None } }`|Preserve|NON-ASCII: XSD \d|0|11|false / false|NOT ASCII-only: XML Schema \d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_MSTGTA_TargetPriorityType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(3), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[0-9][1-9][0-9]&#124;[0-9]{2}[1-9]&#124;[1-9][0-9]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_PATCHB_GravityType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(7), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([3][1-3]\\.[0-9]{4})&#124;( {7})" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_ReleasingInstructionsType`|1 / [8]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(20), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: " {20}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){1} {17}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){2} {14}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){3} {11}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){4} {8}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){5} {5}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){6} {2}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){6}[A-Z]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|same-level OR of finite fixed-width uppercase pairs and exact literal SPACE padding; deterministic branch sequences|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_UTC_TimeType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(7), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([01][0-9]&#124;[2][0-3])([0-5][0-9])([0-5][0-9])Z)" }, PatternExpression { dialect: XmlSchema, expression: "( ){7}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NotationType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(4), max_length: Some(5), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[A-Z0-9]{5}&#124;UNKN&#124;NONE" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|79|1|false / true|upper-alphanumeric exact 5 OR literal UNKN OR literal NONE; independent minLength=4/maxLength=5|
|`{https://www.vdl.afrl.af.mil/programs/oam}RecordOriginatorType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(1), max_length: Some(2), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[A-Z][A-Z]&#124;[E]&#124;[\\-]" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|72|1|false / true|two uppercase letters OR literal E OR literal hyphen; independent minLength=1/maxLength=2|

<!-- FIPS_CountryCodeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "ActionPlan", "ActionPlanCommand", "ActivityPlanCommand", "CommSupportPlan", "CommSupportPlanCommand", "EffectPlan", "EffectPlanCommand", "MissionEnvironmentOverride", "MissionPlan", "MissionPlanCommand", "OpPoint", "OpVolume", "OpZone", "OrbitActivityPlanCommand", "OrbitPlan", "OrbitPlanCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "RequirementOptions", "RequirementOptionsCommand", "ResponsePlan", "ResponsePlanCommand", "RouteActivityPlanCommand", "RoutePlan", "RoutePlanCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "SMTI_CapabilityStatus", "SMTI_Command", "SMTI_SettingsCommand", "Task", "TaskPlan", "TaskPlanCommand"} -->


<!-- IPv4_AddressType semantic messages: {"IO_PortCommand", "IO_PortStatus", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "RDMA_Initialize", "RDMA_InitializeSetup", "Response", "Task"}; support messages: {"ProductOrFileDisseminationDestination"} -->


<!-- IPv6_AddressType semantic messages: {"IO_PortCommand", "IO_PortStatus", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "RDMA_Initialize", "RDMA_InitializeSetup", "Response", "Task"}; support messages: {"ProductOrFileDisseminationDestination"} -->


<!-- MilitaryGridType semantic messages: {"AirRecord", "FacilityRecord", "LandRecord", "MissileRecord", "SeaSubsurfaceRecord", "SeaSurfaceRecord", "UnitRecord"}; support messages: {"OrderOfBattle"} -->


<!-- NITF_AIMIDB_MissionNumberType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_CodewordsType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DateAndTimeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DateType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DeclassificationExemptionType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DeclassificationType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_IPON_IID2_ProgramCodeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_MSTGTA_TargetCategoryType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_MSTGTA_TargetLocationType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_MSTGTA_TargetPriorityType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_PATCHB_GravityType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_ReleasingInstructionsType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_UTC_TimeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NotationType semantic messages: {"AccessAssessment", "AccessAssessmentRequest", "AccessAssessmentRequestStatus", "Action", "ActionCommand", "ActionPlan", "ActionPlanCommand", "ActivityPlan", "ActivityPlanCommand", "AirRecord", "ApprovalPolicy", "COMINT_Activity", "COMINT_Capability", "COMINT_Command", "CommSupport", "CommSupportPlan", "CommSupportPlanCommand", "DMPI", "EA_Activity", "EA_Capability", "EA_Command", "ESM_Activity", "ESM_Capability", "ESM_Command", "ESM_SettingsCommand", "Effect", "EffectCommand", "EffectPlan", "EffectPlanCommand", "EmitterRecord", "Entity", "EntityManagementRequest", "LandRecord", "MissileRecord", "MissionEnvironmentOverride", "MissionPlan", "MissionPlanCommand", "MultistaticEmitterData", "OB_CorrelationRecord", "ObservationMeasurementReport", "ObservationReport", "OpPoint", "OpRouting", "OpVolume", "OpZone", "OrbitActivityPlan", "OrbitActivityPlanCommand", "OrbitPlan", "OrbitPlanCommand", "PlanningFunction", "PlanningFunctionSettingsCommand", "PlanningFunctionStatus", "PrioritizationList", "ProductProcessingFunction", "RequirementOptions", "RequirementOptionsCommand", "Response", "ResponseCommand", "ResponsePlan", "ResponsePlanCommand", "RouteActivityPlan", "RouteActivityPlanCommand", "RoutePlan", "RoutePlanCommand", "SeaSubsurfaceRecord", "SeaSurfaceRecord", "SignalReport", "SupportPlan", "SupportPlanRequest", "SupportRequest", "SystemEstimationRequestStatus", "SystemManagementRequest", "SystemReadiness", "SystemStatus", "SystemsNeededRequest", "Task", "TaskCommand", "TaskPlan", "TaskPlanCommand"}; support messages: {"OrderOfBattle"} -->


<!-- RecordOriginatorType semantic messages: {"AccessAssessment", "AccessAssessmentRequest", "AccessAssessmentRequestStatus", "Action", "ActionCommand", "ActionPlan", "ActionPlanCommand", "ActivityPlan", "ActivityPlanCommand", "AirRecord", "ApprovalPolicy", "COMINT_Activity", "CommSupport", "CommSupportPlan", "CommSupportPlanCommand", "DMPI", "Effect", "EffectCommand", "EffectPlan", "EffectPlanCommand", "Entity", "EntityManagementRequest", "FacilityRecord", "LandRecord", "LaunchObservation", "MissileRecord", "MissionEnvironmentOverride", "MissionPlan", "MissionPlanCommand", "OB_CorrelationRecord", "ObservationMeasurementReport", "ObservationReport", "OpPoint", "OpRouting", "OpVolume", "OpZone", "OrbitActivityPlan", "OrbitActivityPlanCommand", "OrbitPlan", "OrbitPlanCommand", "PlanningFunction", "PlanningFunctionSettingsCommand", "PlanningFunctionStatus", "PrioritizationList", "ProductMetadata", "ProductProcessingFunction", "RequirementOptions", "RequirementOptionsCommand", "Response", "ResponseCommand", "ResponsePlan", "ResponsePlanCommand", "RouteActivityPlan", "RouteActivityPlanCommand", "RoutePlan", "RoutePlanCommand", "SeaSubsurfaceRecord", "SeaSurfaceRecord", "SignalReport", "SpaceRecord", "SupportPlan", "SupportPlanRequest", "SupportRequest", "SystemEstimationRequestStatus", "SystemManagementRequest", "SystemReadiness", "SystemStatus", "SystemsNeededRequest", "Task", "TaskCommand", "TaskPlan", "TaskPlanCommand"}; support messages: {"OrderOfBattle"} -->


Projection failures (46): ["ActivityPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "Assessment: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "AssessmentRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "AssessmentRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "ComponentConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "ComponentConfigurationDataRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "DataPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "DataPlanOverrideRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "DataRecordManagementRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "DataUpdateRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "GatewayActivity: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "GatewayCommand: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "MessageTransmissionFilterRecord: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "OrbitActivityPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "OrderOfBattleRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "ProductDownloadPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductDownloadTask: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationReport: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationTask: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationTaskStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileDisseminationPlan: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductOrFileDisseminationReport: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductOrFileDisseminationRequest: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductOrFileDisseminationTask: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductProcessingFunction: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingReport: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingTask: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingTaskStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "QueryDataRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "QueryDataRequestStatus: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ResendDataRequestStatus: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "Response: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType", "RouteActivityPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "StoreLoadoutConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}StoreItemType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutCarriageType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutItemPET -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutItemType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreItemType", "SubsystemConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "SubsystemConfigurationDataRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "SubsystemMaintenanceCommand: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestCommandType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceSubtestCommandChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestCommandPET -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestCommandType", "SubsystemMaintenanceConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceSubtestChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestPET -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestType", "SubsystemMaintenanceStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestResultType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceSubtestResultChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestResultPET -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestResultType", "SystemReadiness: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType"]

#### OrderOfBattle parent baseline

- Ada: selected 55/55, support 439/442, READY=false; unsupported support: [QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "MilitaryGridType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "NotationType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "RecordOriginatorType" }].
- Rust: selected 55/55, support 439/442, READY=false; unsupported support: [QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "MilitaryGridType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "NotationType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "RecordOriginatorType" }].
- Cpp: selected 55/55, support 439/442, READY=false; unsupported support: [QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "MilitaryGridType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "NotationType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "RecordOriginatorType" }].

#### Parent coverage

- ClosedSchemaSet Ada: BackendCoverage { language: Ada, declarations_total: 5557, declaration_kinds_renderable: 5537, declarations_fully_renderable: 5515, fields_total: 13160, field_type_references_renderable: 13160, field_occurrences_renderable: 13160, messages_total: 722, message_closures_renderable: 552 }
- ClosedSchemaSet Rust: BackendCoverage { language: Rust, declarations_total: 5557, declaration_kinds_renderable: 5537, declarations_fully_renderable: 5516, fields_total: 13160, field_type_references_renderable: 13160, field_occurrences_renderable: 13160, messages_total: 722, message_closures_renderable: 555 }
- ClosedSchemaSet Cpp: BackendCoverage { language: Cpp, declarations_total: 5557, declaration_kinds_renderable: 5537, declarations_fully_renderable: 5516, fields_total: 13160, field_type_references_renderable: 13160, field_occurrences_renderable: 13160, messages_total: 722, message_closures_renderable: 555 }
- OpenExtensions Ada: BackendCoverage { language: Ada, declarations_total: 5557, declaration_kinds_renderable: 5537, declarations_fully_renderable: 5428, fields_total: 13160, field_type_references_renderable: 13160, field_occurrences_renderable: 13160, messages_total: 722, message_closures_renderable: 526 }
- OpenExtensions Rust: BackendCoverage { language: Rust, declarations_total: 5557, declaration_kinds_renderable: 5537, declarations_fully_renderable: 5428, fields_total: 13160, field_type_references_renderable: 13160, field_occurrences_renderable: 13160, messages_total: 722, message_closures_renderable: 526 }
- OpenExtensions Cpp: BackendCoverage { language: Cpp, declarations_total: 5557, declaration_kinds_renderable: 5537, declarations_fully_renderable: 5428, fields_total: 13160, field_type_references_renderable: 13160, field_occurrences_renderable: 13160, messages_total: 722, message_closures_renderable: 526 }
### UCI 2.6

Root SHA-256: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

|Qualified name|Groups / alternatives per group|Exact effective constraints|Effective whiteSpace|ASCII gate|Semantic message reach|Generated-support reach|OrderOfBattle selected / support|Branch assessment|
|---|---|---|---|---|---:|---:|---|---|
|`{https://www.vdl.afrl.af.mil/programs/oam}FIPS_CountryCodeType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(2), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[A-Z]{2}&#124;[ ]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|39|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}IPv4_AddressType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(7), max_length: Some(15), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])\\.){3}([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|11|1|false / false|five finite octet branches; four octets with literal dot separators; needs finite nested union/sequence, not a networking parser|
|`{https://www.vdl.afrl.af.mil/programs/oam}IPv6_AddressType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(2), max_length: Some(45), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "((:&#124;[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:&#124;[0-9a-fA-F]{0,4}))&#124;(((25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])\\.){3}(25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])))" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|11|1|false / false|finite hex runs (0..4), colon alternatives, 0..5 repeated runs, finite optional suffix, embedded decimal alternatives; model decision pending, no platform parser permitted|
|`{https://www.vdl.afrl.af.mil/programs/oam}MilitaryGridType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(3), max_length: Some(15), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([1-9]&#124;[1-5][0-9]&#124;60)[C-HJ-NP-X][A-HJ-NP-Z][A-HJ-NP-V]&#124;[ABYZ][A-CF-HJ-LP-UX-Z][A-HJ-NP-Z])(([0-9]{2}){0,5})" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|7|1|false / true|three zone alternatives plus polar alternative; finite 0..5 digit pairs; release-specific optional suffix in 2.5 versus mandatory letters in 2.6; finite expansion possible|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_AIMIDB_MissionNumberType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(4), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "((((U0)&#124;[A-Z]{2})[0-9]{2})&#124;UNKN)" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_CodewordsType`|1 / [5]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(11), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: " {11}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){1} {8}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){2} {5}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){3} {2}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){3}[A-Z]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|same-level OR of finite fixed-width uppercase pairs and exact literal SPACE padding; deterministic branch sequences|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DateAndTimeType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(12), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([12]\\d\\d\\d)((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01])([01][0-9]&#124;[2][0-3])([0-5][0-9]))" }, PatternExpression { dialect: XmlSchema, expression: " {12}" }] }], white_space: None } }`|Preserve|NON-ASCII: XSD \d|0|11|false / false|NOT ASCII-only: XML Schema \d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DateType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(8), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([12]\\d\\d\\d((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01]))" }, PatternExpression { dialect: XmlSchema, expression: " {8}" }] }], white_space: None } }`|Preserve|NON-ASCII: XSD \d|0|11|false / false|NOT ASCII-only: XML Schema \d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DeclassificationExemptionType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(4), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(X[1-8]( ){2})&#124;25X[1-9]&#124;[DNIO]&#124;( ){4}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|four internal alternatives; length=4 independently rejects the single-letter DNIO branch; retain this branch in semantics rather than invent padding|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_DeclassificationType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(2), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(DD&#124;DE&#124;GD&#124;GE&#124;O( )&#124;X( )&#124;( ){2})" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_IPON_IID2_ProgramCodeType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(2), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([0-9][A-Z]&#124;[A-Z][0-9])" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_MSTGTA_TargetCategoryType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(5), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[1-9][0-9]{4}&#124;[ ]{5}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_MSTGTA_TargetLocationType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(21), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([0-8]\\d([0-5]\\d){2}\\.\\d{2}(N&#124;S)(0\\d{2}&#124;1[0-7]\\d)([0-5]\\d){2}\\.\\d{2}(E&#124;W))" }, PatternExpression { dialect: XmlSchema, expression: "([\\+\\-]{1}[0-8]\\d\\.\\d{6}[\\+\\-]{1}(0\\d{2}&#124;1[0-7]\\d)\\.\\d{6})" }] }], white_space: None } }`|Preserve|NON-ASCII: XSD \d|0|11|false / false|NOT ASCII-only: XML Schema \d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_MSTGTA_TargetPriorityType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(3), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[0-9][1-9][0-9]&#124;[0-9]{2}[1-9]&#124;[1-9][0-9]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_PATCHB_GravityType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(7), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "([3][1-3]\\.[0-9]{4})&#124;( {7})" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_ReleasingInstructionsType`|1 / [8]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(20), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: " {20}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){1} {17}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){2} {14}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){3} {11}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){4} {8}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){5} {5}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){6} {2}" }, PatternExpression { dialect: XmlSchema, expression: "([A-Z]{2} ){6}[A-Z]{2}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|same-level OR of finite fixed-width uppercase pairs and exact literal SPACE padding; deterministic branch sequences|
|`{https://www.vdl.afrl.af.mil/programs/oam}NITF_UTC_TimeType`|1 / [2]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: Some(7), min_length: None, max_length: None, lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "(([01][0-9]&#124;[2][0-3])([0-5][0-9])([0-5][0-9])Z)" }, PatternExpression { dialect: XmlSchema, expression: "( ){7}" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|0|11|false / false|finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies|
|`{https://www.vdl.afrl.af.mil/programs/oam}NotationType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(4), max_length: Some(5), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[A-Z0-9]{5}&#124;UNKN&#124;NONE" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|79|1|false / true|upper-alphanumeric exact 5 OR literal UNKN OR literal NONE; independent minLength=4/maxLength=5|
|`{https://www.vdl.afrl.af.mil/programs/oam}RecordOriginatorType`|1 / [1]|`ConstraintSet { min_inclusive: None, max_inclusive: None, min_exclusive: None, max_exclusive: None, length: None, min_length: Some(1), max_length: Some(2), lexical: LexicalConstraintSet { pattern_groups: [PatternGroup { alternatives: [PatternExpression { dialect: XmlSchema, expression: "[A-Z][A-Z]&#124;[E]&#124;[\\-]" }] }], white_space: None } }`|Preserve|finite ASCII classes/literals|72|1|false / true|two uppercase letters OR literal E OR literal hyphen; independent minLength=1/maxLength=2|

<!-- FIPS_CountryCodeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "ActionPlan", "ActionPlanCommand", "ActivityPlanCommand", "CommSupportPlan", "CommSupportPlanCommand", "EffectPlan", "EffectPlanCommand", "MissionEnvironmentOverride", "MissionPlan", "MissionPlanCommand", "OpPoint", "OpVolume", "OpZone", "OrbitActivityPlanCommand", "OrbitPlan", "OrbitPlanCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "RequirementOptions", "RequirementOptionsCommand", "ResponsePlan", "ResponsePlanCommand", "RouteActivityPlanCommand", "RoutePlan", "RoutePlanCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "SMTI_CapabilityStatus", "SMTI_Command", "SMTI_SettingsCommand", "Task", "TaskPlan", "TaskPlanCommand"} -->


<!-- IPv4_AddressType semantic messages: {"IO_PortCommand", "IO_PortStatus", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "RDMA_Initialize", "RDMA_InitializeSetup", "Response", "Task"}; support messages: {"ProductOrFileDisseminationDestination"} -->


<!-- IPv6_AddressType semantic messages: {"IO_PortCommand", "IO_PortStatus", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "RDMA_Initialize", "RDMA_InitializeSetup", "Response", "Task"}; support messages: {"ProductOrFileDisseminationDestination"} -->


<!-- MilitaryGridType semantic messages: {"AirRecord", "FacilityRecord", "LandRecord", "MissileRecord", "SeaSubsurfaceRecord", "SeaSurfaceRecord", "UnitRecord"}; support messages: {"OrderOfBattle"} -->


<!-- NITF_AIMIDB_MissionNumberType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_CodewordsType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DateAndTimeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DateType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DeclassificationExemptionType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_DeclassificationType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_IPON_IID2_ProgramCodeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_MSTGTA_TargetCategoryType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_MSTGTA_TargetLocationType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_MSTGTA_TargetPriorityType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_PATCHB_GravityType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_ReleasingInstructionsType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NITF_UTC_TimeType semantic messages: {}; support messages: {"AO_CapabilityStatus", "AO_SettingsCommand", "PO_Activity", "PO_Capability", "PO_CapabilityStatus", "PO_Command", "PO_SettingsCommand", "SAR_CapabilityStatus", "SAR_Command", "SAR_SettingsCommand", "Task"} -->


<!-- NotationType semantic messages: {"AccessAssessment", "AccessAssessmentRequest", "AccessAssessmentRequestStatus", "Action", "ActionCommand", "ActionPlan", "ActionPlanCommand", "ActivityPlan", "ActivityPlanCommand", "AirRecord", "ApprovalPolicy", "COMINT_Activity", "COMINT_Capability", "COMINT_Command", "CommSupport", "CommSupportPlan", "CommSupportPlanCommand", "DMPI", "EA_Activity", "EA_Capability", "EA_Command", "ESM_Activity", "ESM_Capability", "ESM_Command", "ESM_SettingsCommand", "Effect", "EffectCommand", "EffectPlan", "EffectPlanCommand", "EmitterRecord", "Entity", "EntityManagementRequest", "LandRecord", "MissileRecord", "MissionEnvironmentOverride", "MissionPlan", "MissionPlanCommand", "MultistaticEmitterData", "OB_CorrelationRecord", "ObservationMeasurementReport", "ObservationReport", "OpPoint", "OpRouting", "OpVolume", "OpZone", "OrbitActivityPlan", "OrbitActivityPlanCommand", "OrbitPlan", "OrbitPlanCommand", "PlanningFunction", "PlanningFunctionSettingsCommand", "PlanningFunctionStatus", "PrioritizationList", "ProductProcessingFunction", "RequirementOptions", "RequirementOptionsCommand", "Response", "ResponseCommand", "ResponsePlan", "ResponsePlanCommand", "RouteActivityPlan", "RouteActivityPlanCommand", "RoutePlan", "RoutePlanCommand", "SeaSubsurfaceRecord", "SeaSurfaceRecord", "SignalReport", "SupportPlan", "SupportPlanRequest", "SupportRequest", "SystemEstimationRequestStatus", "SystemManagementRequest", "SystemReadiness", "SystemStatus", "SystemsNeededRequest", "Task", "TaskCommand", "TaskPlan", "TaskPlanCommand"}; support messages: {"OrderOfBattle"} -->


<!-- RecordOriginatorType semantic messages: {"AccessAssessment", "AccessAssessmentRequest", "AccessAssessmentRequestStatus", "Action", "ActionCommand", "ActionPlan", "ActionPlanCommand", "ActivityPlan", "ActivityPlanCommand", "AirRecord", "ApprovalPolicy", "COMINT_Activity", "CommSupport", "CommSupportPlan", "CommSupportPlanCommand", "DMPI", "Effect", "EffectCommand", "EffectPlan", "EffectPlanCommand", "Entity", "EntityManagementRequest", "FacilityRecord", "LandRecord", "LaunchObservation", "MissileRecord", "MissionEnvironmentOverride", "MissionPlan", "MissionPlanCommand", "OB_CorrelationRecord", "ObservationMeasurementReport", "ObservationReport", "OpPoint", "OpRouting", "OpVolume", "OpZone", "OrbitActivityPlan", "OrbitActivityPlanCommand", "OrbitPlan", "OrbitPlanCommand", "PlanningFunction", "PlanningFunctionSettingsCommand", "PlanningFunctionStatus", "PrioritizationList", "ProductMetadata", "ProductProcessingFunction", "RequirementOptions", "RequirementOptionsCommand", "Response", "ResponseCommand", "ResponsePlan", "ResponsePlanCommand", "RouteActivityPlan", "RouteActivityPlanCommand", "RoutePlan", "RoutePlanCommand", "SeaSubsurfaceRecord", "SeaSurfaceRecord", "SignalReport", "SpaceRecord", "SupportPlan", "SupportPlanRequest", "SupportRequest", "SystemEstimationRequestStatus", "SystemManagementRequest", "SystemReadiness", "SystemStatus", "SystemsNeededRequest", "Task", "TaskCommand", "TaskPlan", "TaskPlanCommand"}; support messages: {"OrderOfBattle"} -->


Projection failures (46): ["ActivityPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "Assessment: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "AssessmentRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "AssessmentRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "ComponentConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "ComponentConfigurationDataRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "DataPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "DataPlanOverrideRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "DataRecordManagementRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "DataUpdateRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "GatewayActivity: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "GatewayCommand: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "MessageTransmissionFilterRecord: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "OrbitActivityPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "OrderOfBattleRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "ProductDownloadPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductDownloadTask: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationReport: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationTask: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileClassificationTaskStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductOrFileDisseminationPlan: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductOrFileDisseminationReport: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductOrFileDisseminationRequest: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductOrFileDisseminationTask: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ProductProcessingFunction: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingReport: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingTask: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "ProductProcessingTaskStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType", "QueryDataRequest: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "QueryDataRequestStatus: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "ResendDataRequestStatus: projected service schema cannot be planned for generation: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants", "Response: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType", "RouteActivityPlan: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}QueryPET -> {https://www.vdl.afrl.af.mil/programs/oam}QueryType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryMatchType -> {https://www.vdl.afrl.af.mil/programs/oam}QueryPET", "StoreLoadoutConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}StoreItemType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutCarriageType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutItemPET -> {https://www.vdl.afrl.af.mil/programs/oam}StoreLoadoutItemType -> {https://www.vdl.afrl.af.mil/programs/oam}StoreItemType", "SubsystemConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "SubsystemConfigurationDataRequestStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType", "SubsystemMaintenanceCommand: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestCommandType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceSubtestCommandChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestCommandPET -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestCommandType", "SubsystemMaintenanceConfiguration: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceSubtestChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestPET -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestType", "SubsystemMaintenanceStatus: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestResultType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceSubtestResultChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestResultPET -> {https://www.vdl.afrl.af.mil/programs/oam}SubsystemMaintenanceTestResultType", "SystemReadiness: projected service schema cannot be planned for generation: cyclic generated value dependencies are unsupported: {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationPET -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationTreeType -> {https://www.vdl.afrl.af.mil/programs/oam}ComponentConfigurationChoiceType"]

#### OrderOfBattle parent baseline

- Ada: selected 56/56, support 439/442, READY=false; unsupported support: [QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "MilitaryGridType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "NotationType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "RecordOriginatorType" }].
- Rust: selected 56/56, support 439/442, READY=false; unsupported support: [QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "MilitaryGridType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "NotationType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "RecordOriginatorType" }].
- Cpp: selected 56/56, support 439/442, READY=false; unsupported support: [QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "MilitaryGridType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "NotationType" }, QualifiedName { namespace_uri: "https://www.vdl.afrl.af.mil/programs/oam", local_name: "RecordOriginatorType" }].

#### Parent coverage

- ClosedSchemaSet Ada: BackendCoverage { language: Ada, declarations_total: 5570, declaration_kinds_renderable: 5550, declarations_fully_renderable: 5529, fields_total: 13198, field_type_references_renderable: 13198, field_occurrences_renderable: 13198, messages_total: 725, message_closures_renderable: 553 }
- ClosedSchemaSet Rust: BackendCoverage { language: Rust, declarations_total: 5570, declaration_kinds_renderable: 5550, declarations_fully_renderable: 5530, fields_total: 13198, field_type_references_renderable: 13198, field_occurrences_renderable: 13198, messages_total: 725, message_closures_renderable: 556 }
- ClosedSchemaSet Cpp: BackendCoverage { language: Cpp, declarations_total: 5570, declaration_kinds_renderable: 5550, declarations_fully_renderable: 5530, fields_total: 13198, field_type_references_renderable: 13198, field_occurrences_renderable: 13198, messages_total: 725, message_closures_renderable: 556 }
- OpenExtensions Ada: BackendCoverage { language: Ada, declarations_total: 5570, declaration_kinds_renderable: 5550, declarations_fully_renderable: 5442, fields_total: 13198, field_type_references_renderable: 13198, field_occurrences_renderable: 13198, messages_total: 725, message_closures_renderable: 527 }
- OpenExtensions Rust: BackendCoverage { language: Rust, declarations_total: 5570, declaration_kinds_renderable: 5550, declarations_fully_renderable: 5442, fields_total: 13198, field_type_references_renderable: 13198, field_occurrences_renderable: 13198, messages_total: 725, message_closures_renderable: 527 }
- OpenExtensions Cpp: BackendCoverage { language: Cpp, declarations_total: 5570, declaration_kinds_renderable: 5550, declarations_fully_renderable: 5442, fields_total: 13198, field_type_references_renderable: 13198, field_occurrences_renderable: 13198, messages_total: 725, message_closures_renderable: 527 }

## Scope gate status

The final admission boundary is frozen in PRE-PRODUCTION FINAL GATE above:
15 admitted declarations per release, three DEFERRED_UNICODE declarations and
one DEFERRED_COMPLEX declaration. Sixteen unique admitted profiles cover both
releases. IPv4 remains lexical and IPv6 remains deferred; neither may use a
platform IP parser.
Production implementation has now begun at the core semantic layer only:
`alternating_ascii.rs` represents AND-groups of OR-branches, with independent
whole-input cursors, separate facets and sixteen exact name-free rows. Optional
grid suffixes and bounded digit-pair repetitions expand to fixed bodies. IPv4
currently has 625 core bodies; generated helper factoring remains outstanding.
The new finite-alphabet variant retains ordinal ASCII ranges and does not
evaluate character-class or regex source.

This layer is deliberately **not yet wired into `StringProfile` or backend
coverage**, so no unimplemented generated carrier can be counted as supported.
Core tests cover grouping, branch order independence of acceptance, changed
grouping rejection, exact rows/facet neighbors, high-value lexical examples,
release-specific grids, IPv4 and unreachable-branch facet rejection. Existing
Task 059 classifier tests pass unchanged. Backend rendering, codec, corpus and
after-impact validation remain incomplete; this is not the review gate.

## Validation status

### Completed compiler matrix and group-AND codec composition

The existing Ada 2.6 explicit compile exit was recovered as **0** from
`/tmp/task060-oob-2.6-ada-explicit.exit`, without another generation or inventory.
Thus the preserved six-way OrderOfBattle model/API compiler matrix passes:
Ada 2.5/2.6 exit 0, Rust 2.5/2.6 exit 0 after the generic correction, and C++
2.5/2.6 exit 0. These compiler results correspond to the recorded source state;
the later world/elision analysis refinement requires Rust result verification
against its newly generated output before final-head provenance is claimed.

The private backend unit test now renders an explicit test-only carrier with
length=2 and groups (AB OR AC) AND (AB OR AD), replacing only the emitted
NotationType carrier inside a normally generated model. The service codec and
its expected model path come from the existing production emitter. Generated
Rust source compiles with warnings denied and codec decoding/re-encoding proves
AB preserved; AC, AD, AE and integer/boolean/null/array/object rejected. No
handwritten matcher/codec, public override or pinned-classifier exception was
added. Normal changed-group classifier/readiness rejection remains tested.
Composition test passed in a clean target in 0.15 seconds, and again after
world/elision refinement in 0.16 seconds. This is generated composition proof,
not admission of arbitrary two-group schemas. Test-only dependencies reuse the
existing runtime-rust and serde_json crates.

### Initial output comparison exposed an elision refinement

The first parent/final fixture campaign covered 185 fixtures × 6 = 1110 cells:
551 identical successes, 547 shared failures, six changed successes, six new
successes, zero regressions. All six changed cells were Rust closed-world
absent-only member fixtures: fields removed by Task 026 storage elision caused
the first derive correction to remove equality unnecessarily. Those changes
were **unexpected**, not correctness fixes, and were investigated before delivery.

The existing trait analysis now receives the actual generation world throughout
and uses the same `field_storage_semantics` decision as emission to ignore
absent-only fields. Concrete descendants outside emitted storage do not
invalidate otherwise valid equality. Closed-sum dependency analysis still uses
the production projection API; no second graph or name exception. Compiler-backed
Boolean/float/temporal payload tests and AND codec composition pass after this
refinement; workspace Clippy passes. A new fixture comparison against the same
archived exact parent is running at `/tmp/task060-final-byte.log`; its result
will supersede the initial diagnostic comparison rather than hiding it.

The refined comparison completed with exit 0: **557 byte-identical successes,
547 shared failures, six new successes, zero changed successes, zero regressions**
across all 1110 cells. Thus no existing successful fixture emitted a derive-only
byte change after absent-only storage was handled correctly. The six new cells
are the new Task 060 codec fixture across three backends/two worlds. The final
subsequent one-line change passes the actual world to the projection API; focused
derive tests/Clippy pass, but final output provenance must still verify that
world-threading adjustment before treating these totals as final-head evidence.

Parent attribution is grounded in the archived parent source: its named stored
reference eligibility looks up the declaration and calls declaration eligibility
without projecting concrete closed-sum payloads. The compiler-backed abstract
Boolean/float/temporal reproduction diagnoses that structural omission; merely
the code's age is not used as proof.

### Generic Rust stored-closed-sum derive correction

Task 060 exposed a parent-analysis inconsistency (unchanged parent named-reference
logic and a compiler-backed concrete-payload reproduction support attribution): in both real releases the generated
`OrderOfBattleML` had `#[derive(Debug, Clone, PartialEq, Eq)]` over its sole
`records: UnboundedVec<RecordDRLE, 0>` field, while `RecordDRLE` correctly had
only Debug/Clone. Its concrete variants include AirRecordMDT, whose dependency
path reaches AirRecordDataType → optional OrderOfBattleTimestampsType →
DateTimeSigmaType → DateTimeType (and optional DurationType). Temporal carriers
intentionally implement neither equality trait: lexical spelling equality is
not XML Schema temporal value equality. Thus PartialEq already fails; Eq fails
as well. Floats independently permit PartialEq but not Eq.

The existing transitive Record/Choice analysis handled effective inherited
fields and wrapper element bounds, but a named abstract VALUE reference inspected
only the abstract declaration's fields instead of the emitted closed sum's
concrete payloads. The correction extends that same reference analysis through
the production `abstract_value_projection_for_ref` API. No UCI/schema names or
parallel dependency graph are used. Optional and sequence trait bounds follow
their element. Missing/projection failures decline eligibility; recursion declines
both traits conservatively using the existing visiting set and terminates.

Compiler-backed synthetic tests pass for Boolean leaves (PartialEq+Eq), float
leaves (PartialEq only), and direct DateTime leaves (neither), propagating through
closed sums, unbounded/bounded storage, nested records, choices and optional
members. Reversing declaration order preserves decisions. A cyclic Vec storage
control terminates, conservatively emits Debug/Clone, and compiles.

Both real Rust OrderOfBattle services were regenerated after this correction and
compile under `-Dwarnings`: **2.5 exit 0, 2.6 exit 0**, recorded at
`/tmp/task060-rust-fixed-results.json`. Final RecordDRLE and OrderOfBattleML
derives are Debug/Clone only in both releases. Previous E0369/E0277 are resolved
without inventing equality implementations or stripping valid float PartialEq.
The generic correction's fixture byte-change blast radius remains to be measured.

Ada output was not regenerated. An explicit compile of already-generated 2.5
OrderOfBattle model/API completed with **exit 0**. The explicit 2.6 compile is
still outstanding; no universal compiler-backed success is claimed yet.
C++ both releases previously compiled with strict flags. Task 060 core seven,
Task 059 classifier three, normal compiled codec, mock-OWP, all three readiness
controls and Task 059 Rust corpus pass after the correction; warnings-denied
workspace Clippy passes. Current historical-state tests remain to be updated.

### Compiled codec and selected-readiness proof

The new synthetic `codec-alternating-ascii` service is generated by the normal
CLI in the runtime facade test build. Its generated Rust model, service API and
existing checked-String codec compile and execute. The Task 060 codec test passes
Notation values UNKN/NONE0/ABC12, Origin values hyphen/E/ZZ, and lexical IPv4
values unchanged through JSON round trips. Invalid literals/classes, leading
zero and out-of-range octets, Unicode digits, integer/boolean/null/array/object
members are rejected through the checked constructor path. No parallel codec.

The generated mock-OWP test passes typed publish/subscribe lexical preservation
and invalid-address decode-event rejection without handler delivery. Focused
codec results: one round-trip test passed and one mock-OWP test passed in 0.09
seconds, log `/tmp/task060-codec.log`. The generated synthetic two-group AND
carrier remains a standalone renderer test only; no multi-group pinned row is
admitted, and its compiled codec-specific control is still outstanding.

Selected-service readiness controls pass in all three backends for the exact
fixture and fail closed after branch/facet/group changes. Two capability tests
passed. Generated-support-only negative controls and deferred-profile readiness
controls remain outstanding. Core seven tests, Task 059 classifier three tests,
and Task 059 generated corpus across all backends pass after these additions.
Workspace all-target Clippy with warnings denied, formatting and diff checks pass.

The pinned AFTER target `uci_alternating_after` is running once, logging to
`/tmp/task060-after-coverage.log` with exit marker
`/tmp/task060-after-coverage.exit`. It measures twelve coverage cells and exact
OrderOfBattle model readiness/support counts. It does not yet prove CLI READY,
service generation or generated compilation, and no such claim is made.

### Standalone gate passed; shared capability integrated

The factored campaign completed with exit 0 at
`/tmp/task060-factored-probes.log`: Ada one test passed in **327.56 seconds**,
C++ one test passed in **36.55 seconds**, Rust one test passed in **9.71 seconds**.
The gate includes all sixteen exact semantic profiles, synthetic two-group AND,
control/Unicode negatives, IPv4 reference-model comparisons, lexical preservation,
Ada default-initialization rejection under both assertion policies, and
deliberate wrong-expectation detection. No repeated parent evidence run occurred.

Only after that gate, `StringProfile::AlternatingAscii` was added after all
Tasks 037–059 classifiers decline. All three backend validation and rendering
consumers were wired in the same change. Shared coverage/readiness already use
`string_profile`, so no backend-local capability classifier or separate coverage
switch was introduced. Helpers remain private/nested; no new preflight names.

Focused anti-overclaim tests pass: an exact differently named Notation profile
is recognized, counted renderable and generated by all three backends; changed,
missing or extra branches, altered facets and additional restriction groups
are rejected by classification, coverage and backend generation. The frozen
38-row core test confirms thirty admitted release rows and eight deferred rows
through the live classifier. Core tests seven passed, Task 059 classifier tests
three passed, workspace all-target check and Clippy passed.

Rust's existing checked constrained-String codec emitter is classifier-generic:
it encodes `as_str()` and decodes a JSON string through `Type::new`; no parallel
Task 060 codec has been added. Dedicated Task 060 compiled codec/mock-OWP tests
remain outstanding, as do service-readiness-specific negative assertions.

Integration invalidates historical tests asserting nineteen CURRENT unsupported
Strings and three CURRENT OrderOfBattle support blockers. Those current-state
regressions still require measured updates; historical evidence remains intact.
Full workspace tests and real AFTER coverage/message measurements have not yet
been run. This is not completion or review-gate evidence.

### Compiler campaign status and narrow Rust correction

The unfactored campaign completed with exit 101: Ada and C++ passed their
generated probes, then Rust rejected synthetic `Carrier_0`-style type names
under warnings denied and unnecessary outer parentheses around single-group
return expressions. Its result is not final factored-renderer evidence.

The synthetic names now use CamelCase `Carrier0` etc. Rust single-group
expressions omit unnecessary outer parentheses; multi-group expressions retain
the parentheses needed to preserve OR-within/AND-across semantics. No admission
or lexical matching rule changes. The corrected factored Rust probe passed
with warnings denied, including deliberately wrong expectation detection:
one test passed in 9.78 seconds, exit 0, at
`/tmp/task060-factored-rust-corrected.log`. The corrected factored C++ probe
passed strict C++17 flags and sabotage detection: one test passed in 39.97
seconds, exit 0, at `/tmp/task060-factored-cpp-corrected.log`.

At the single process inspection the factored Ada campaign was healthy. A
single bounded process-exit wait did not complete; it was left running.
Ada post-factoring success is not yet claimed. Capability integration remains
disabled until its compiler/lifecycle/sabotage result is known.

After the narrow Rust correction: core tests seven passed; the three unchanged
Task 059 classifier tests passed; Task 059 generated corpus again passed in Ada,
Rust and C++; workspace all-target check and warnings-denied Clippy passed;
affected core/backend all-target checks passed on Rust 1.95.0; CI split guard
and all 104 split regression checks passed. Formatting and diff checks pass.
The new explicit IPv4 lexical test pins leading-zero rejection and the exact
range/delimiter/control domain independently of any platform parser.
No parent inventory run was repeated and no real AFTER claims are made.

The earlier size table used underscored synthetic names. CamelCase test names
slightly reduce printed carrier sizes without changing semantic factoring;
final source-size evidence will use consistent final generated names.

### Semantic delimiter-product factoring

The shared core now recognizes a Cartesian product of identical delimiter-free
component unions from the structured bodies alone. It does not inspect names,
regex text or classifier identity. Every branch must have the same component
count, no component can admit the delimiter, and the set of distinct index
tuples must equal the full product cardinality. Missing combinations decline
the optimization. A focused regression drops one of the 625 IPv4 bodies and
proves decline; it compares original and factored semantic acceptance over
leading-zero, range, delimiter-count, empty-component, sign, whitespace and
Unicode-lookalike controls.

Each renderer emits five private/nested whole-component validators and a scanner
that validates every dot-separated slice with their union and requires exactly
four components. No validation heap allocation, numeric conversion, platform IP
parser or regex evaluator is introduced. Classification is unchanged.

Measured post-factoring generated carrier/body sizes (bytes / lines):

|Profile|Ada|Rust|C++|
|---|---:|---:|---:|
|Notation|2338 / 49|1316 / 38|2019 / 43|
|RecordOriginator|1947 / 49|1266 / 38|1667 / 43|
|MilitaryGrid 2.5|59317 / 1465|36316 / 1192|56355 / 1313|
|MilitaryGrid 2.6|55273 / 1365|33842 / 1111|52670 / 1225|
|IPv4|5862 / 141|3415 / 110|4919 / 123|
|NITF_ReleasingInstructions|21575 / 523|13116 / 414|18439 / 453|

Ada IPv4 decreases from 2,230,528 bytes / 51,767 lines to 5,862 bytes / 141
lines, approximately 380-fold smaller. Source-size tests pass in all three
backends and require this expanded family to stay below 25 KB as a regression
guard, not an admission rule. Task 059 generated corpus regressions still pass
in all three backends after the factoring change. Core tests: six passed.
Workspace all-target check and Clippy passed before the size-test addition;
formatting and diff checks pass.

The old unfactored standalone compiler campaign was healthy at the single
inspection and was left running. The post-factoring compiler campaign waits for
its process exit rather than starting a competing compile. Its log will be
`/tmp/task060-factored-probes.log` and result marker
`/tmp/task060-factored-probes.exit`. No post-factoring all-backend compiler pass
is claimed yet. Coverage/StringProfile/readiness remain disabled.

### Standalone renderer work in progress

The admission numbers remain **15 names per release / 16 unique profiles across
releases / 4 deferred names**, not 16 admitted names / 3 deferred names.
IPv6 is explicitly DEFERRED_COMPLEX; the three Unicode deferrals are unchanged.
Each release has 15 admitted normalized groups and 27 normalized expression
alternatives. The sixteen authored semantic profiles have sixteen groups and
724 expanded deterministic bodies. Their 18 distinct ordinal alphabet sets
include five already present in Task 059 and thirteen new sets; these inventory
counts are asserted in the core tests.

Task 059 sequence emitters have been extracted, without changing their emitted
text, and are reused by standalone Task 060 renderers. Existing Task 059 generated
corpus probes pass in all three backends. Full fixture byte-identity proof is
still outstanding. Branch helpers are private associated/static functions in
Rust/C++, and nested functions within Ada Create; no public/top-level helper
names have been introduced. These renderers are not connected to capability.

A shared semantic corpus generator covers every expanded body, lexical junk,
control/non-ASCII inputs, high-value profiles, and a synthetic two-group
intersection. Compiler probes include deliberate expectation corruption and
restoration. These tests are currently running; no three-backend pass is claimed.
An Ada test-source Character-versus-String literal error was diagnosed and
corrected before restarting the affected standalone probe campaign.

Initial unfactored Ada body-size diagnostics:

|Profile|Bytes|Lines|
|---|---:|---:|
|Notation|2338|49|
|MilitaryGrid 2.5|59317|1465|
|MilitaryGrid 2.6|55273|1365|
|IPv4|2230528|51767|
|NITF_ReleasingInstructions|21575|523|

IPv4 duplication is pathological and helper factoring is required before
finalizing rendering. These are diagnostic sizes, not final generated-size
evidence. StringProfile/coverage integration stays disabled until all backend
probes pass and rendering is suitable.

This evidence target checks both root hashes, exact unsupported name sets, effective facets, per-message reach, generated-support reach, all six OrderOfBattle parent verdicts, and the 12-cell parent coverage matrix. The separate Task 060 frontend grouping test proves preservation of same-level alternatives and successive restriction groups using an explicit finite-literal intersection oracle. Implementation, after measurements, backend corpus, codecs, byte identity, workspace/MSRV and CI delivery are NOT COMPLETE.

The final-source parent survey completed with exit 0: one test passed in
292.72 seconds, recorded at `/tmp/task060-parent-evidence-final.log`.
The separate pinned admission target completed with exit 0: two tests passed
in 11.54 seconds, recorded at `/tmp/task060-admission.log`. This short target
asserts the final boundary without unnecessarily repeating the message survey.

## Measured AFTER coverage (Task 059 exact parent comparison)

The single AFTER target passed; the following twelve cells preserve exact numerator counts. Denominators: 2.5 declarations 5557, fields 13160, messages 722; 2.6 declarations 5570, fields 13198, messages 725.

|Release|World|Backend|Kinds before→after|Declarations before→after|Field types before→after|Field occurrences before→after|Messages before→after|
|---|---|---|---:|---:|---:|---:|---:|
|2.5|ClosedSchemaSet|Ada|5537→5552|5515→5530|13160→13160|13160→13160|552→587|
|2.5|ClosedSchemaSet|Rust|5537→5552|5516→5531|13160→13160|13160→13160|555→590|
|2.5|ClosedSchemaSet|Cpp|5537→5552|5516→5531|13160→13160|13160→13160|555→590|
|2.5|OpenExtensions|Ada|5537→5552|5428→5443|13160→13160|13160→13160|526→540|
|2.5|OpenExtensions|Rust|5537→5552|5428→5443|13160→13160|13160→13160|526→540|
|2.5|OpenExtensions|Cpp|5537→5552|5428→5443|13160→13160|13160→13160|526→540|
|2.6|ClosedSchemaSet|Ada|5550→5565|5529→5544|13198→13198|13198→13198|553→588|
|2.6|ClosedSchemaSet|Rust|5550→5565|5530→5545|13198→13198|13198→13198|556→591|
|2.6|ClosedSchemaSet|Cpp|5550→5565|5530→5545|13198→13198|13198→13198|556→591|
|2.6|OpenExtensions|Ada|5550→5565|5442→5457|13198→13198|13198→13198|527→541|
|2.6|OpenExtensions|Rust|5550→5565|5442→5457|13198→13198|13198→13198|527→541|
|2.6|OpenExtensions|Cpp|5550→5565|5442→5457|13198→13198|13198→13198|527→541|

No numeric regression. The existing closed-world Ada difference remains one declaration and three message closures per release; exact unsafe-name/message attribution still requires the naming-context regression. Open-world backend counts agree. Whole-schema first blockers were not printed by this target and remain outstanding.

## OrderOfBattle measured model state

Both releases/all three backends: selected 55/55 (2.5), 56/56 (2.6); support 442/442; unsupported selected/support lists empty; backend/API blockers absent; production readiness analysis returns READY. This is MODEL evidence. CLI service-check/generate and generated compilation remain separate gates.

The single AFTER run completed with exit 0, one test passed in **185.73 seconds**
(`/tmp/task060-after-coverage.log`). An earlier commentary runtime was incorrect;
185.73 seconds is the actual libtest result. No measurement rerun was launched.

The generated-support-only synthetic regression passes in Ada/Rust/C++: the
selected abstract Base/Payload closure is fully renderable, while a concrete
descendant's String dependency appears only in the production projection's
generated-support set. An altered Notation literal keeps selected counts intact
but blocks final readiness and backend generation; the exact admitted row makes
support renderable and generation succeed. All three focused capability tests
pass. Workspace all-target Clippy and formatting/diff checks pass.

A single six-cell OrderOfBattle CLI campaign is running. Completed generation
cells are available in `/tmp/task060-oob-cli-results.json`; per-cell CLI logs
are outside the repository. Compiler probes are underway and no universal
CLI/generation/compiler success is claimed. The first Ada compile call exceeded
the tool client's 30-second wait; this is not recorded as compiler failure.

Subsequent completed CLI artifacts establish **service-check exit 0 and
service-generate exit 0 in all six release/backend cells**, recorded at
`/tmp/task060-oob-cli-results.json`. Strict C++17 model/API probes compile in
both releases (exit 0). Rust model/API compilation fails in both releases:
`OrderOfBattleML` derives PartialEq/Eq over `UnboundedVec<RecordDRLE, 0>`, while
the generated abstract-value `RecordDRLE` sum implements neither trait (E0369,
E0277). Exact diagnostics are in the per-release Rust compile logs. This is
a generated structural equality-derive defect, not a lexical union mismatch;
its remediation/regression and byte-identity implications remain unresolved.
Do not claim universal compiler-backed OrderOfBattle success or update the
current regression as though that compiler gate passed. The first Ada run
produced model/API `.ali` files but its exit capture was interrupted by the
client timeout; a recorded successful compile remains required.

The missing two-group codec test must not add a production classifier exception:
none of the pinned rows has two restriction-level groups. Production generation
correctly rejects such unobserved constraints. A test-only generated-carrier
construction mechanism is still needed to exercise group-AND codec behavior
without contradicting exact-profile admission. This remains outstanding.

## Final-source verification campaign (delivery still gated)

Source snapshot SHA-256: `670c104b3704c42e3164fb7a45e691ca445b87965abcf288bb8b385bbd1b29b1`.
The manifest `/tmp/task060-source-manifest.json` hashes tracked and untracked
non-documentation files by relative path/content; it includes production,
fixtures, tests, lockfile and workflows, not just HEAD. Original parent remains
`263df8be24439f368b64cfde56b9b0c70a93e226`. Production was frozen during
these campaigns. Rust 1.98.1, Cargo 1.98.1, GNAT 14.2.0 and GCC C++ 14.2.0.

### Final Rust OrderOfBattle verification

Both pinned releases were regenerated via production `service-generate` using
`--language rust --world closed-schema`, then compiled as the service API crate
root via `rustc --edition=2021 -Dwarnings --crate-type=lib` with runtime-api
linked. Exit 0 for both generation and compilation. Both emitted model/API
files are byte-identical to the previously compiled corrected outputs.
Both `RecordDRLE` and `OrderOfBattleML` derive **Debug, Clone**, neither equality
trait. `/tmp/task060-final-rust-results.json` records each result and derive.

|Release|Ada model/API|Rust model/API|C++ model/API|
|---|---|---|---|
|2.5|PASS, preserved explicit exit 0|PASS, newly executed exit 0|PASS, preserved exit 0|
|2.6|PASS, recovered explicit exit 0|PASS, newly executed exit 0|PASS, preserved exit 0|

The only later production changes were in Rust trait analysis, not Ada/C++
rendering or shared classifier inputs. Preserved Ada/C++ results are not
represented as freshly executed. Focused tests explicitly pin absent-only
optional/inherited/closed-sum storage preserving equality and reject the same
unclosed abstract value in the open world. Boolean/float/temporal compiler
cases remain separate (2 tests passed, `/tmp/task060-refinement-tests.log`).

### Final-source fixture byte comparison

The complete comparison ran once after world-threading settled, reusing the
release binary built from the archived exact parent and building AFTER from
this snapshot. Command: `python3 /tmp/task060-final-byte.py`; final log
`/tmp/task060-snapshot-byte.log`, exit artifact `.exit` = 0. It uses production
`generate` for Ada/Rust/C++ × closed-schema/open-extensions, hashing relative
output paths and complete bytes.

|Cells (185 XSD fixtures × 6)|Count|
|---|---:|
|Byte-identical success|557|
|Shared failure|547|
|New success|6|
|Changed success|0|
|Regression|0|

The six new successes are exactly
`tests/fixtures/service-generate/codec-alternating-ascii.xsd` across six scopes.
It is the sole new XSD fixture. There are no derive-related changed-success
fixture cells after correcting elided-field analysis. Real OrderOfBattle
derive corrections are independently established by its concrete temporal
storage path and compiled synthetic reproductions, not by counting fewer
traits as inherently correct.

### Separate real OrderOfBattle codec proof

Production `service-check --with-codec` is READY in Rust in both releases
(492/492 emitted declarations in 2.5; see per-release reports). Production
`service-generate --with-codec` succeeds; both generated model/API/codec
assemblies compile under `-Dwarnings` and execute a representative payload
with a concrete EmitterRecordMDT in the abstract RecordDRLE storage and
NotationType `UNKN` at CENOT_Identifier. Re-encoding preserves `UNKN`; invalid
`bad` and non-string integer input are rejected through codec decoding.
Records use the actual generated `$type` discriminator. This is Rust codec
evidence, not a codec claim for Ada/C++. Scratch run vectors do not replace
the remaining durable Deep-CI integration gate.

Results: `/tmp/task060-oob-codec-results.json`: compile/run exit 0 both
releases. Logs: `/tmp/task060-oob-{2.5,2.6}-codec-{generation,compile,run}.log`.
The first trial used an incorrect `type` member and was rejected; correcting
the vector to `$type` required no production change.

### Projected real-message impact versus whole-schema coverage

The preserved parent admission survey supplied the union of selected/support
reaching messages. A separately indexed production readiness comparison loaded
each schema once per implementation/release; it did not reload per operation.
BEFORE used archived exact parent source; AFTER used the frozen production
snapshot. Full logs `/tmp/task060-{parent-,}impact-{2.5,2.6}.log`, exit 0.

|Release/backend scope|Reaching|Newly READY|Already READY|TimeType first|IPv6 first|Unicode first|Topology failures|
|---|---:|---:|---:|---:|---:|---:|---:|
|2.5, Ada/Rust/C++ each|103|38|0|45|11|3|6|
|2.6, Ada/Rust/C++ each|103|38|0|45|11|3|6|

Newly READY names (same set in both releases and all backends):

`AccessAssessment`, `AccessAssessmentRequest`, `AccessAssessmentRequestStatus`, `AirRecord`, `COMINT_Activity`, `COMINT_Capability`, `COMINT_Command`, `DMPI`, `EA_Activity`, `EA_Capability`, `EA_Command`, `ESM_Activity`, `ESM_Capability`, `ESM_Command`, `ESM_SettingsCommand`, `EmitterRecord`, `Entity`, `EntityManagementRequest`, `FacilityRecord`, `LandRecord`, `LaunchObservation`, `MissileRecord`, `MultistaticEmitterData`, `OB_CorrelationRecord`, `ObservationMeasurementReport`, `ObservationReport`, `OrderOfBattle`, `ProductMetadata`, `SMTI_Command`, `SMTI_SettingsCommand`, `SeaSubsurfaceRecord`, `SeaSurfaceRecord`, `SignalReport`, `SpaceRecord`, `SystemEstimationRequestStatus`, `SystemManagementRequest`, `SystemStatus`, `UnitRecord`.

Exact reaching set and final verdicts are pinned in `tests/fixtures/string/task060-message-impact.tsv` (206 rows). Three Unicode-first services are AO_CapabilityStatus, AO_SettingsCommand, and SAR_SettingsCommand; remaining deferred Unicode names also occur behind earlier blockers. The actual distribution is not the historical TimeType count of 28 or the full-schema closure increase of 35.

Topology failures (identical before/after, all backends): `ActivityPlan`, `OrbitActivityPlan`, `ProductProcessingFunction`, `Response`, `RouteActivityPlan`, `SystemReadiness`. Their projected QueryPET → QueryType → QueryMatchType → QueryPET by-value cycle fails generation planning. No topology correction is included.

Ada full-schema Rust/C++-minus-Ada set is exactly Authorization, AuthorizationRequest, CommSupportActivity in both releases, unchanged from parent. The known QueryType companion/QueryType collision at QueryType_Kind remains. Projected topology and full-schema name context are separate; final per-message projected name attribution is recorded below (all 18 projected CLI checks pass).

### Smallest real vertical

SMTI_SettingsCommand is the deterministic minimum (69 total projected declarations in both releases) among all 38 newly READY messages. The previously unsupported FIPS_CountryCodeType is required by stored STANAG packing-plan concrete support. Normal service-check/service-generate and Ada/Rust/C++ model/API compile exit 0 for both releases; `/tmp/task060-smallest-results.json`. Rust production generated codec compilation and execution also exit 0 in both releases: ClassificationSystem `US` round-trips, `bad` and integer rejected. Its wire discriminator is STANAG_4607_PackingPlanType, deliberately not the Rust identifier. Durable JSON vectors are compiler/codec tests, not independent schema-conformance assertions.

### Validation continuation and invalidation assessment

Final production source did not change after the snapshot; subsequent changes were test harnesses, measured fixtures, current-state assertions and CI wiring. Full fixture byte and final model/API evidence remains valid since generation inputs and fixture XSD membership did not change. Workspace initially failed the group-AND harness due to incoherent rlib selection. That harness now uses an offline Cargo generated crate with a coherent dependency graph; it passes (19.24s), without any matcher/codec change. The current historical category-A regression now expects all 35 READY services, and its pinned target passes with unchanged execution markers. Fast includes synthetic tests through workspace test; pinned Task 060 evidence is wired into Deep with explicit marker/count checks (2 admission tests, 3 AFTER tests). CI-split guards/adversarial tests pass after wiring. Final workspace/Deep campaigns remain pending and must not be inferred from early/skipped tests.

The Ada gap attribution reuses the existing durable Task 058 naming regression:
unsafe declarations unique to Ada are QueryPET and QueryType; each of the
three differing message closures reaches QueryPET (not merely an arbitrary
unrelated full-schema declaration). Its full-schema preflight explicitly pins
the QueryType_Kind collision. The set is unchanged by Task 060. Current-state
closure totals in that regression are updated to 587/590 (2.5) and 588/591
(2.6), based on the preserved twelve-cell AFTER campaign; historical docs
remain unchanged. The existing test still independently exercises naming
and closure attribution; its pinned final run is pending.

Final snapshot maintenance: subsequent current-state tests change only
Task 058/059 exclusion counts from 19 to four and the measured closure totals;
no classifier/renderer/schema input changes. The workspace continuation was
started before these two test-only adjustments; their focused pinned tests
and current Clippy are separately required rather than pretending the earlier
workspace run used those assertions. CI-split now explicitly forbids Task 060
pinned targets in Fast and requires their Deep invocations/vertical markers;
adversarial missing-marker tests pass. No timeout was increased.

Marker formatting assessment: admission test now places a newline before each
required whole-line marker, because libtest prefixes the first println with
`test ...`. This changes no admission assertions or production inputs. Its
marker-valid execution is separately captured at
`/tmp/task060-admission-marker-final.{log,exit}`.

Projected naming comparison is now executed via normal CLI for all three gap
messages × both releases × all backends: all 18 service-check cells exit 0.
Thus QueryPET full-schema unsafe-name attribution does not imply these projected
services are blocked: their narrower context passes. Results and complete
reports: `/tmp/task060-naming-projected.json` and its referenced logs. No new
naming correction was introduced.

The full workspace continuation found an obsolete Task 058 negative expecting
NotationType to remain unsupported. It now rejects the unobserved NOPE neighbor
instead, preserving a meaningful fail-closed control without denying newly
admitted input. The six-test bounded ASCII coverage target passes. Durable
Deep compiler assembly initially had an incorrect runtime-api manifest path;
fixed to existing crates/runtime-api-rust, no production change. The failed
trials are retained in logs. Only the invalidated compiler/codec vertical is
rerun, not the completed 206-message impact test or the coverage analysis.
A final workspace run was launched with these test fixes, and marker checks
will combine the already executed unchanged tests with the corrected vertical.

Production-only manifest identity: `ed45c844bea9caa4831f34c1301413c200fc3d352dd4b931dea1ab1acc2d6aba`,
`/tmp/task060-production-manifest.json`. Every production source hash remains
identical to the initial final-world snapshot, checked explicitly after all
regression/harness changes. This establishes reuse of carrier, admission and
fixture-generation evidence rather than relabeling it as freshly executed.
The smallest closure is 62 selected + 7 support = 69, both releases.

Validation tooling: local frozen-source model generation release binary is
`/tmp/task060-target/release/ams-gra-codegen-oms`, BEFORE is
`/tmp/task060-parent-target/release/ams-gra-codegen-oms`. Pinned schema hashes
remain the roots listed in the baseline tables; source compilation uses Rust
1.98.1 locally and affected MSRV checks use Rust 1.95.0. All scratch artifacts
are outside the worktree. No commit/push/PR or remote CI is claimed yet.

Final non-documentation source identity, including all untracked production/tests
and CI/fixture files: `66c8af298e3983845968fcc4b4e7e6b3de61f084780a4a67233672dedb7c33cd`
(`/tmp/task060-final-source-manifest.json`). All changes from the production
snapshot are documented test/fixture/CI adjustments, with unchanged production
hashes. CI-split adversarial marker tests now include whole-line, missing marker
and exact-count Task 060 controls; 120 checks pass. Affected Rust 1.95.0
workspace all-target checks pass; two compiler-backed equality tests pass on
1.95.0, and the generated group-AND Cargo harness passes under that toolchain.
No MSRV result from an optional skipped real target is counted as Deep evidence.

Validation command ledger (final sources except explicitly assessed test-only
changes):

|Scope|Command / flags|Result / artifact|
|---|---|---|
|Format|cargo fmt --all -- --check|PASS|
|Workspace types|cargo check --workspace --all-targets|PASS, /tmp/task060-final-check.log|
|Workspace lints|cargo clippy --workspace --all-targets -- -D warnings|PASS, /tmp/task060-final-clippy.log|
|Synthetic final workspace|AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace|exit 0; 1,234 passed, 0 failed/ignored across 130 target summaries; /tmp/task060-workspace-final.{log,exit}|
|MSRV types|cargo +1.95.0 check --locked --workspace --all-targets|exit 0, /tmp/task060-msrv-last.{log,exit}|
|MSRV derive compilation|cargo +1.95.0 test --locked -p ams-gra-oms-backend-rust --test equality_closed_sum|2 passed, /tmp/task060-msrv-regressions.log|
|MSRV AND codec|cargo +1.95.0 test --locked -p ams-gra-oms-backend-rust --lib generated_group_and_carrier|1 passed, /tmp/task060-msrv-settled.log|
|CI separation|scripts/check-ci-split.sh; scripts/test-check-ci-split.sh|PASS, 120 adversarial checks, /tmp/task060-final-split.log|
|Diff whitespace|git diff --check|PASS|
|Pinned admission|cargo test --release -p ams-gra-codegen-oms --test uci_alternating_admission -- --nocapture --test-threads 1|2 passed, marker-valid, /tmp/task060-admission-marker-final.log|
|Pinned current support|cargo test --release -p ams-gra-codegen-oms --test uci_generated_support -- --nocapture --test-threads 1|2 passed, /tmp/task060-pinned-support-final.log|

Pinned commands use both AMS_GRA_UCI_2_5_ROOT/AMS_GRA_UCI_2_6_ROOT with
verified hashes, plus AMS_GRA_REQUIRE_GNAT=1 for compiler verticals. Local Cargo
artifacts use CARGO_TARGET_DIR=/tmp/task060-target (MSRV: /tmp/task060-msrv-final).
Final source identity and tool versions above apply; no optional-root skip
is counted as a pinned execution. Command exit files are durable; their absence
means pending, not success or failure.

Additional final control provenance: `/tmp/task060-msrv-controls.{log,exit}`
exit 0; Rust 1.95.0 deterministic-cycle test (one passed) and bounded-ASCII
negative-neighbor suite (six passed). Fast explicit execution checks now cover
AND codec, comparable/float/temporal derive compile, absent-only traits, checked
carrier codec, generated-support negative and mock-OWP delivery/rejection.
The untracked-file review found only the intended 20 Task 060 source/test/docs
and fixture files; no scratch logs, compiler outputs or generated code are
tracked. Historical parent evidence target is retained for provenance only,
not invoked on final pinned roots (its assertions deliberately describe parent).

## Completed local gates and delivery provenance

All five existing continuations were collected, not relaunched. Their source
inputs match `66c8af298e3983845968fcc4b4e7e6b3de61f084780a4a67233672dedb7c33cd`
exactly using the original tracked + untracked non-documentation manifest rules.
All production hashes also match the original world-aware production snapshot.
Documentation-only finalization does not invalidate them.

|Existing stage|State|Exit|Evidence and final summary|
|---|---|---:|---|
|workspace-final|completed|0|`/tmp/task060-workspace-final.{log,exit}`; 130 summaries, 1,234 passed, 0 failed, 0 ignored, 0 filtered|
|vertical-durable|completed|0|`/tmp/task060-vertical-durable.{log,exit}`; one target passed, 476.56s, all four release/service markers|
|historical-updated|completed|0|`/tmp/task060-historical-updated.{log,exit}`; Task 058 five tests/524.57s, Task 059 two tests/594.65s, required markers|
|confirm-ready|completed|0|`/tmp/task060-confirm-ready.{log,exit}`; 228 unique successful release/backend/message tuples|
|local-marker-check|completed|0|`/tmp/task060-local-marker-check.{log,exit}`; completed exit files, unchanged impact markers, corrected compiler markers and historical markers verified|

The original AFTER target's impact and coverage/OOB assertions were unchanged;
its compiler assembly trial failed on a test-only dependency path and was
replaced by the smallest invalidated target. The corrected vertical now has
an overall exit 0, not just intermediate markers. The successful coverage and
impact portions are preserved rather than making the failed combined exit a
false PASS. `/tmp/task060-final-audit.json` independently checks exact sets and
workspace totals; `/tmp/task060-local-marker-results.json` records completed
exit files and 228 CLI cells. All 228 tuples match precisely the 38-name sets
in both releases/all backends, without duplicate-line counting. Archived parent
readiness rows confirm NOT READY with a Task 060 declaration blocker for every
newly READY tuple. Final categories are disjoint and complete: 38 READY +
45 TimeType + 11 IPv6 + 3 deferred Unicode + 6 topology = 103, each release.
There are no already-READY or additional name/API/global failures in this
particular reaching set. The separate naming-context checks all pass (18/18).

The synthetic unsupported-generated-support negative remains in
`alternating_capability::generated_support_string_controls_final_readiness_not_selected_counts`;
its three-test target passed after current OrderOfBattle expectations changed.
Historical Task 056/058/059 documents were not modified. CI split is exactly
120 adversarial checks, not an older 104-count run. No timeout was increased;
local standalone Ada probe dominates the synthetic compiler portion, while
historical message surveys and the four-service durable vertical are the
measured slow Deep portions. Final exact-head hosted campaign timing remains
an independent gate, not inferred from local timing.

Validated final fixture matrix remains 185 XSDs × six scopes = 1,110 cells:
557 byte-identical successes, 547 shared failures, six new successes (only
`tests/fixtures/service-generate/codec-alternating-ascii.xsd` across Ada/Rust/C++
and both worlds), zero changed successes/regressions. Final review inspected
all 20 untracked files; no source manifest, scratch log, generated output,
compiler binary, object or ALI file is included.

### Live base reconciliation and review PR

Task 059 PR #60 is MERGED (2026-10-03T05:43:21Z) at
`a7aed23dd9f3852d20bbabf6b5b9aecd4e96bd87`. Live fetch confirms original benchmark
parent `263df8be24439f368b64cfde56b9b0c70a93e226` is an ancestor of main; main's
tree is byte-identical to that parent. No transplant/rebase was needed.
Task 060 implementation commit: `096ab10dcd5934ae3e96313e607488c0604d974a`.
Normal main reconciliation merge: `b44e57149e292afb42fc56b406cc322c5b966ea7`.
Every committed non-documentation file was checked against the validated
manifest after committing and after reconciliation: identity unchanged.
The main-relative diff has only the intended 40 Task 060/corrective files,
not duplicate Task 059 work. Task 059 branch/worktree was not modified.

Review PR **#61**, base **main**:
https://github.com/zackboll/ams-gra-codegen-oms/pull/61
Branch: `feature/060-alternating-ascii-string-profiles`. Open, non-draft,
unmerged, auto-merge disabled. No remaining PR #60 dependency. This final
provenance update is documentation-only; its resulting delivery HEAD is visible
in the PR and commit history (not self-embedded into its own content).

Hosted exact-head Fast and Deep CI are required independently. Workflow
pull_request triggers cover this main-target PR, including Deep because workflow
files changed. Do not substitute PR #60 CI or an obsolete Task 060 head.
Run IDs/results and final equality of local/remote/PR head are reported in the
review-gate PR evidence comment/final report once completed; a pending run is
not a pass. No auto-merge or merge is requested.

## PR #61 hosted-failure corrective

Previous head: `cdbf0950055194122094bb49766abfaabee28ca4`.
Fast run 37134770117 / job 111236999638 completed **failure**, step Generated
Rust OMS JSON codec (must execute). Deep run 37134770126 / real-uci job
111237001544 completed **failure**, step Real UCI constrained Binary inventory
(Task 053, must execute). Other Deep jobs could still run; this does not make
its failed real-uci job pending. Old runs remain failure evidence, not final CI.
Fast complete log `/tmp/task060-fast-failure.log`; Deep job metadata
`/tmp/task060-deep-failed-job.json` (GitHub withholds run logs while other jobs
are active), and pinned reproduction diagnostics are preserved separately.

### Exact execution-name defect

`cargo test -p ams-gra-oms-backend-rust --lib -- --list` registers
`task060_probes::generated_group_and_carrier_composes_with_production_service_codec`.
The previous workflow used the unqualified name both as guard expectation and
Cargo filter, while its helper adds `--exact`. Hosted output correctly reported
zero passed/49 filtered and the execution guard failed. Both occurrences now
use the fully qualified name. No test moved, public seam added, or execution
requirement weakened. All six new explicit Task 060 guard/filter pairs were
cross-checked against their actual registered test lists, then executed through
the actual extracted workflow helper: six separate exactly-one passing tests,
exit 0, `/tmp/task060-exact-workflow.{log,exit}`. The guard now detects loss of
module qualification; CI-split/adversarial suite is **121 checks**, superseding
the valid original 120-check measurement.

### Constrained-Binary current-state defect and reproduction

Before editing assertions, the exact committed target was run with BOTH pinned
roots and `--release --nocapture --test-threads 1`. It failed with exit 101 at
`uci_constrained_binary.rs:356`: actual first unsupported String **None**, stale
expected **Some(RecordOriginatorType)**. ProductMetadata's printed remaining
String list was **[]**. Log `/tmp/task060-binary-reproduce.log`, exit artifact
`/tmp/task060-binary-reproduce.exit`, 2 inventory tests passed/impact failed.
Normal production CLI evidence already records ProductMetadata READY in Ada,
Rust, C++ in both releases (181 selected declarations in 2.5, 183 in 2.6).
The corrected regression asserts the empty unsupported String list and exact
AlternatingAscii classification of both NotationType and RecordOriginatorType,
then compares all three actual readiness verdicts to A. It retains constrained
Binary lexical/provenance inventories, no-Binary-first-blocker assertions,
exact backend comparisons and Response cyclic-topology negatives.
Historical Task 053/059 measurements were not rewritten.

### Precise earlier-local provenance correction

`/tmp/task060-historical-updated.log` ran ONLY uci_bounded_ascii_string and
uci_structured_ascii with both roots, their markers and 5+2 passing tests.
It **did not execute uci_constrained_binary**. The GNAT-required synthetic
workspace run did include that target, but roots were absent: three tests
returned early, completed in 0.01s, and no required constrained-Binary inventory
or impact markers exist. Thus the genuine 1,234-test synthetic workspace result
is retained, but it never established pinned constrained-Binary execution.
The hosted failure does not contradict the separate Task 058/059 pinned runs.
Neither the earlier substring-filtered AND test success nor its compiled codec
proof validated the previous `--exact` workflow filter. Those proofs remain
valid while the actual invocation defect is now separately tested.

### Diagnostic-preserving shell correction

Narrowly affected Fast codec helper, Deep constrained-Binary capture, and new
Task 060 Deep captures now initialize status, capture output with `|| status=$?`,
print diagnostics on either outcome, and propagate the original status before
checking success evidence. set -euo pipefail, exact matching, one-test count and
all mandatory markers remain. New `scripts/test-task060-ci-wrappers.py` extracts
actual workflow shell and executes it: failure diagnostic visible with original
exit 23 (Fast) and 37 (Deep), zero matched tests rejected, success without
required markers rejected. Four checks pass; Fast invokes this regression.
No general CI redesign or timeout increase.

The initial corrected Binary shell block passed all 3 tests and unchanged
inventory/impact markers in both releases in 110.14s,
`/tmp/task060-binary-workflow.{log,exit}`. Clippy then identified a newly unused
old B-verdict helper; only that dead test closure was removed. A final-snapshot
Binary wrapper run captures the same unchanged assertions without that warning
at `/tmp/task060-binary-final-wrapper.{log,exit}`. No production code changed.

Corrective non-documentation identity:
`ccee06c4171e3b1a6f0ae671dcf358b34fefefc364ac27480b5403965afd5217`, manifest
`/tmp/task060-corrective-final-manifest.json`. The prior whole-source identity
is **not** claimed unchanged: affected files are the two workflows, constrained
Binary test, CI-split guard/adversarial script, and new wrapper regression.
Production inputs remain identical. Established model/compiler/codec evidence,
228 unique CLI confirmations and 1,110 fixture cells retain their original
provenance; these workflow/test fixes do not invalidate generation outputs.
Formatting, all-target workspace check, Clippy with warnings denied and diff
whitespace pass on the corrective sources. No capability survey or real vertical
was restarted. Fresh hosted final-head workflows remain a separate gate.

Final corrective Binary wrapper completed with explicit exit **0**, three tests
passed in **110.06s**. Its actual workflow block verified all four unchanged
UCI 2.5/2.6 inventory and message-impact markers plus the three-test summary.
`/tmp/task060-binary-final-wrapper.{log,exit}` contains printed complete output.
The final snapshot remained unchanged throughout this run.
