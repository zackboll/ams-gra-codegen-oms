# Task 059 frozen pre-production semantic inventory

Source: verified pinned 2.5/2.6 normalized IR (Task 058 pinned inventory test output), starting main d4a043d324fb5580a717c4e267304ec2a3c80c2f. Every row is depth 1 <- xs:String, one pattern group, XML Schema dialect, whiteSpace explicit None / effective Preserve, numeric facets absent. No field-local facet or nillability on references. Existing inventory log also carries declared/effective references and source positions.

A: 27 admitted declaration rows per release (27 distinct exact expression+facet rows). B: 19 alternation/union rows per release. C: 0. The admitted classes form 25 distinct ASCII member sets: 7 reused from Task 058 and 18 genuinely new. All A are ASCII only. This is a **pre-implementation evidence freeze**, not a claim that these profiles are supported yet.

Deterministic matching evidence: fixed-width class segments need no partition decision. A sole bounded or `+` class consumes to end. In `FileNameType`, the first `+` class excludes `.`; its following literal dot marks the unique boundary. The final `+` class allows dots but consumes to end. `CounterSpaceCycleNumberType` has an optional trailing `T`/`E` after four fixed-width digits, so end-of-input selects zero or one. `AO_PRF_CodeType` is different: the optional literal `1` **overlaps** the following `[1-7]`. The actual `minLength=3`/`maxLength=4` and fixed-width remainder disambiguate it: at length three omit `1`; at length four consume `1`. A validator that greedily consumes `1` without checking total length first would be wrong. `NITF_IPON_IID2_SortieNumberType` has redundant fixed-width parentheses and no union.

|Release|Group|Local name|Exact normalized pattern groups/alternatives (Python repr)|length|minLength|maxLength|message reach|OOB support|Semantic sequence or exclusion|
|---|---|---|---|---:|---:|---:|---:|---|---|
|2.5|A|`AO_PRF_CodeType`|`[['XmlSchema:1?[1-7][1-8]{2}']]`|None|Some(3)|Some(4)|33|no|Class({1},OptionalOneByTotalLength(4)), Class(1-7,One), Class(1-8,Exact(2))|
|2.5|A|`CounterSpaceCycleNumberType`|`[['XmlSchema:[A-Z0-9]{2}-[0-9]{4}[TE]?']]`|None|Some(7)|Some(8)|3|no|Class(A-Z0-9,Exact(2)), Literal(-), Class(0-9,Exact(4)), Class(TE,OptionalOne)|
|2.5|A|`CounterSpaceSENO_Type`|`[['XmlSchema:[A-Z][IRS][0-9]{3}']]`|Some(5)|None|None|3|no|Class(A-Z,One), Class(IRS,One), Class(0-9,Exact(3))|
|2.5|A|`FileNameType`|`[['XmlSchema:[a-zA-Z0-9_\\\\-]+\\\\.[a-zA-Z0-9_\\\\.\\\\-]+']]`|None|Some(1)|Some(255)|30|no|Class(a-zA-Z0-9_\-,OneOrMore), Literal(.), Class(a-zA-Z0-9_\.\-,OneOrMore)|
|2.5|A|`IMO_NumberType`|`[['XmlSchema:IMO[0-9]{7}']]`|Some(10)|None|None|69|yes|Literal(IMO), Class(0-9,Exact(7))|
|2.5|A|`Link16_SpecificTypeModelType`|`[['XmlSchema:[A-Za-z0-9]{1,4}']]`|None|None|Some(4)|77|yes|Class(A-Za-z0-9,Bounded(1,4))|
|2.5|A|`Link16_TrackNumberType`|`[['XmlSchema:[A-HJ-NP-Z0-7]{2}[0-7]{3}']]`|Some(5)|None|None|31|yes|Class(A-HJ-NP-Z0-7,Exact(2)), Class(0-7,Exact(3))|
|2.5|A|`Link1_TrackNumberType`|`[['XmlSchema:[AEGHJKLM]{2}[0-7]{3}']]`|Some(5)|None|None|0|yes|Class(AEGHJKLM,Exact(2)), Class(0-7,Exact(3))|
|2.5|A|`MISP_ItemDesignatorType`|`[['XmlSchema:[a-zA-Z0-9 \\\\-_]{1,16}']]`|None|None|Some(16)|7|no|Class(a-zA-Z0-9 \ - _,Bounded(1,16))|
|2.5|A|`MeterUnitLetterType`|`[['XmlSchema:[m]']]`|Some(1)|None|None|0|no|Class(m,One)|
|2.5|A|`NIIRS_Type`|`[['XmlSchema:[0-9]\\\\.[0-9]']]`|Some(3)|None|None|7|no|Class(0-9,One), Literal(.), Class(0-9,One)|
|2.5|A|`NITF_ACFTB_SceneSourceType`|`[['XmlSchema:[0-9 ]']]`|Some(1)|None|None|0|no|Class(0-9 SPACE,One)|
|2.5|A|`NITF_AIMIDB_FlightNumberType`|`[['XmlSchema:[A-Z0-9][0-9]']]`|Some(2)|None|None|0|no|Class(A-Z0-9,One), Class(0-9,One)|
|2.5|A|`NITF_ClassificationAuthorityMethodType`|`[['XmlSchema:[ODM ]']]`|Some(1)|None|None|0|no|Class(ODM SPACE,One)|
|2.5|A|`NITF_ClassificationReasonType`|`[['XmlSchema:[A-H ]']]`|Some(1)|None|None|0|no|Class(A-H SPACE,One)|
|2.5|A|`NITF_DowngradeType`|`[['XmlSchema:[SCR ]']]`|Some(1)|None|None|0|no|Class(SCR SPACE,One)|
|2.5|A|`NITF_EXPLTB_SequenceNumberType`|`[['XmlSchema:[1-6 ]']]`|Some(1)|None|None|0|no|Class(1-6 SPACE,One)|
|2.5|A|`NITF_FileSecurityClassificationType`|`[['XmlSchema:[TSCRU]']]`|Some(1)|None|None|0|no|Class(TSCRU,One)|
|2.5|A|`NITF_IPON_IID2_SortieNumberType`|`[['XmlSchema:([A-Z0-9]{2})']]`|Some(2)|None|None|0|no|Class(A-Z0-9,Exact(2)) (fixed non-alternating parentheses)|
|2.5|A|`NumericStringLength1Type`|`[['XmlSchema:[0-9]']]`|Some(1)|None|None|0|no|Class(0-9,One)|
|2.5|A|`OB_AirDefenseAreaType`|`[['XmlSchema:[A-Z]{2}[0][0-9]{2}']]`|Some(5)|None|None|1|yes|Class(A-Z,Exact(2)), Class(0,One), Class(0-9,Exact(2))|
|2.5|A|`OB_EmitterSurrogateKeyType`|`[['XmlSchema:[a-zA-Z0-9]{5}[0-9]{9}']]`|None|None|Some(14)|1|yes|Class(a-zA-Z0-9,Exact(5)), Class(0-9,Exact(9))|
|2.5|A|`OB_O_SuffixType`|`[['XmlSchema:[A-Z]{2}[0-9]{3}']]`|Some(5)|None|None|71|yes|Class(A-Z,Exact(2)), Class(0-9,Exact(3))|
|2.5|A|`OctalValueType`|`[['XmlSchema:[0-7]+']]`|None|Some(1)|Some(16)|79|yes|Class(0-7,OneOrMore)|
|2.5|A|`ROME_IdentityType`|`[['XmlSchema:[a-zA-Z0-9]+']]`|None|Some(1)|Some(10)|2|no|Class(a-zA-Z0-9,OneOrMore)|
|2.5|A|`STANAG_4607_PacketSecurityClassificationType`|`[['XmlSchema:[1-5]']]`|Some(1)|None|None|0|no|Class(1-5,One)|
|2.5|A|`UnitIdentifierType`|`[['XmlSchema:[A-Z]{2}[ABCDEGJMNSX][A-Z]{2}[0-9]{5}']]`|Some(10)|None|None|3|yes|Class(A-Z,Exact(2)), Class(ABCDEGJMNSX,One), Class(A-Z,Exact(2)), Class(0-9,Exact(5))|
|2.5|B|`FIPS_CountryCodeType`|`[['XmlSchema:[A-Z]{2}&#124;[ ]{2}']]`|Some(2)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`IPv4_AddressType`|`[['XmlSchema:(([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])\\\\.){3}([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])']]`|None|Some(7)|Some(15)|11|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`IPv6_AddressType`|`[['XmlSchema:((:&#124;[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:&#124;[0-9a-fA-F]{0,4}))&#124;(((25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])\\\\.){3}(25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])))']]`|None|Some(2)|Some(45)|11|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`MilitaryGridType`|`[['XmlSchema:([1-9]&#124;[1-5][0-9]&#124;60)[C-HJ-NP-X]([A-HJ-NP-Z][A-HJ-NP-V]([0-9]{2}){0,5})?&#124;[ABYZ]([A-CF-HJ-LP-UX-Z][A-HJ-NP-Z]([0-9]{2}){0,5})?']]`|None|Some(14)|Some(15)|7|yes|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_AIMIDB_MissionNumberType`|`[['XmlSchema:((((U0)&#124;[A-Z]{2})[0-9]{2})&#124;UNKN)']]`|Some(4)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_CodewordsType`|`[['XmlSchema: {11}', 'XmlSchema:([A-Z]{2} ){1} {8}', 'XmlSchema:([A-Z]{2} ){2} {5}', 'XmlSchema:([A-Z]{2} ){3} {2}', 'XmlSchema:([A-Z]{2} ){3}[A-Z]{2}']]`|Some(11)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_DateAndTimeType`|`[['XmlSchema:(([12]\\\\d\\\\d\\\\d)((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01])([01][0-9]&#124;[2][0-3])([0-5][0-9]))', 'XmlSchema: {12}']]`|Some(12)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_DateType`|`[['XmlSchema:([12]\\\\d\\\\d\\\\d((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01]))', 'XmlSchema: {8}']]`|Some(8)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_DeclassificationExemptionType`|`[['XmlSchema:(X[1-8]( ){2})&#124;25X[1-9]&#124;[DNIO]&#124;( ){4}']]`|Some(4)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_DeclassificationType`|`[['XmlSchema:(DD&#124;DE&#124;GD&#124;GE&#124;O( )&#124;X( )&#124;( ){2})']]`|Some(2)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_IPON_IID2_ProgramCodeType`|`[['XmlSchema:([0-9][A-Z]&#124;[A-Z][0-9])']]`|Some(2)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_MSTGTA_TargetCategoryType`|`[['XmlSchema:[1-9][0-9]{4}&#124;[ ]{5}']]`|Some(5)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_MSTGTA_TargetLocationType`|`[['XmlSchema:([0-8]\\\\d([0-5]\\\\d){2}\\\\.\\\\d{2}(N&#124;S)(0\\\\d{2}&#124;1[0-7]\\\\d)([0-5]\\\\d){2}\\\\.\\\\d{2}(E&#124;W))', 'XmlSchema:([\\\\+\\\\-]{1}[0-8]\\\\d\\\\.\\\\d{6}[\\\\+\\\\-]{1}(0\\\\d{2}&#124;1[0-7]\\\\d)\\\\.\\\\d{6})']]`|Some(21)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_MSTGTA_TargetPriorityType`|`[['XmlSchema:[0-9][1-9][0-9]&#124;[0-9]{2}[1-9]&#124;[1-9][0-9]{2}']]`|Some(3)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_PATCHB_GravityType`|`[['XmlSchema:([3][1-3]\\\\.[0-9]{4})&#124;( {7})']]`|Some(7)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_ReleasingInstructionsType`|`[['XmlSchema: {20}', 'XmlSchema:([A-Z]{2} ){1} {17}', 'XmlSchema:([A-Z]{2} ){2} {14}', 'XmlSchema:([A-Z]{2} ){3} {11}', 'XmlSchema:([A-Z]{2} ){4} {8}', 'XmlSchema:([A-Z]{2} ){5} {5}', 'XmlSchema:([A-Z]{2} ){6} {2}', 'XmlSchema:([A-Z]{2} ){6}[A-Z]{2}']]`|Some(20)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NITF_UTC_TimeType`|`[['XmlSchema:(([01][0-9]&#124;[2][0-3])([0-5][0-9])([0-5][0-9])Z)', 'XmlSchema:( ){7}']]`|Some(7)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.5|B|`NotationType`|`[['XmlSchema:[A-Z0-9]{5}&#124;UNKN&#124;NONE']]`|None|Some(4)|Some(5)|79|yes|Alternation / multiple union alternatives; excluded|
|2.5|B|`RecordOriginatorType`|`[['XmlSchema:[A-Z][A-Z]&#124;[E]&#124;[\\\\-]']]`|None|Some(1)|Some(2)|72|yes|Alternation / multiple union alternatives; excluded|
|2.6|A|`AO_PRF_CodeType`|`[['XmlSchema:1?[1-7][1-8]{2}']]`|None|Some(3)|Some(4)|33|no|Class({1},OptionalOneByTotalLength(4)), Class(1-7,One), Class(1-8,Exact(2))|
|2.6|A|`CounterSpaceCycleNumberType`|`[['XmlSchema:[A-Z0-9]{2}-[0-9]{4}[TE]?']]`|None|Some(7)|Some(8)|3|no|Class(A-Z0-9,Exact(2)), Literal(-), Class(0-9,Exact(4)), Class(TE,OptionalOne)|
|2.6|A|`CounterSpaceSENO_Type`|`[['XmlSchema:[A-Z][IRS][0-9]{3}']]`|Some(5)|None|None|3|no|Class(A-Z,One), Class(IRS,One), Class(0-9,Exact(3))|
|2.6|A|`FileNameType`|`[['XmlSchema:[a-zA-Z0-9_\\\\-]+\\\\.[a-zA-Z0-9_\\\\.\\\\-]+']]`|None|Some(1)|Some(255)|30|no|Class(a-zA-Z0-9_\-,OneOrMore), Literal(.), Class(a-zA-Z0-9_\.\-,OneOrMore)|
|2.6|A|`IMO_NumberType`|`[['XmlSchema:IMO[0-9]{7}']]`|Some(10)|None|None|69|yes|Literal(IMO), Class(0-9,Exact(7))|
|2.6|A|`Link16_SpecificTypeModelType`|`[['XmlSchema:[A-Za-z0-9]{1,4}']]`|None|None|Some(4)|77|yes|Class(A-Za-z0-9,Bounded(1,4))|
|2.6|A|`Link16_TrackNumberType`|`[['XmlSchema:[A-HJ-NP-Z0-7]{2}[0-7]{3}']]`|Some(5)|None|None|31|yes|Class(A-HJ-NP-Z0-7,Exact(2)), Class(0-7,Exact(3))|
|2.6|A|`Link1_TrackNumberType`|`[['XmlSchema:[AEGHJKLM]{2}[0-7]{3}']]`|Some(5)|None|None|0|yes|Class(AEGHJKLM,Exact(2)), Class(0-7,Exact(3))|
|2.6|A|`MISP_ItemDesignatorType`|`[['XmlSchema:[a-zA-Z0-9 \\\\-_]{1,16}']]`|None|None|Some(16)|7|no|Class(a-zA-Z0-9 \ - _,Bounded(1,16))|
|2.6|A|`MeterUnitLetterType`|`[['XmlSchema:[m]']]`|Some(1)|None|None|0|no|Class(m,One)|
|2.6|A|`NIIRS_Type`|`[['XmlSchema:[0-9]\\\\.[0-9]']]`|Some(3)|None|None|7|no|Class(0-9,One), Literal(.), Class(0-9,One)|
|2.6|A|`NITF_ACFTB_SceneSourceType`|`[['XmlSchema:[0-9 ]']]`|Some(1)|None|None|0|no|Class(0-9 SPACE,One)|
|2.6|A|`NITF_AIMIDB_FlightNumberType`|`[['XmlSchema:[A-Z0-9][0-9]']]`|Some(2)|None|None|0|no|Class(A-Z0-9,One), Class(0-9,One)|
|2.6|A|`NITF_ClassificationAuthorityMethodType`|`[['XmlSchema:[ODM ]']]`|Some(1)|None|None|0|no|Class(ODM SPACE,One)|
|2.6|A|`NITF_ClassificationReasonType`|`[['XmlSchema:[A-H ]']]`|Some(1)|None|None|0|no|Class(A-H SPACE,One)|
|2.6|A|`NITF_DowngradeType`|`[['XmlSchema:[SCR ]']]`|Some(1)|None|None|0|no|Class(SCR SPACE,One)|
|2.6|A|`NITF_EXPLTB_SequenceNumberType`|`[['XmlSchema:[1-6 ]']]`|Some(1)|None|None|0|no|Class(1-6 SPACE,One)|
|2.6|A|`NITF_FileSecurityClassificationType`|`[['XmlSchema:[TSCRU]']]`|Some(1)|None|None|0|no|Class(TSCRU,One)|
|2.6|A|`NITF_IPON_IID2_SortieNumberType`|`[['XmlSchema:([A-Z0-9]{2})']]`|Some(2)|None|None|0|no|Class(A-Z0-9,Exact(2)) (fixed non-alternating parentheses)|
|2.6|A|`NumericStringLength1Type`|`[['XmlSchema:[0-9]']]`|Some(1)|None|None|0|no|Class(0-9,One)|
|2.6|A|`OB_AirDefenseAreaType`|`[['XmlSchema:[A-Z]{2}[0][0-9]{2}']]`|Some(5)|None|None|1|yes|Class(A-Z,Exact(2)), Class(0,One), Class(0-9,Exact(2))|
|2.6|A|`OB_EmitterSurrogateKeyType`|`[['XmlSchema:[a-zA-Z0-9]{5}[0-9]{9}']]`|None|None|Some(14)|1|yes|Class(a-zA-Z0-9,Exact(5)), Class(0-9,Exact(9))|
|2.6|A|`OB_O_SuffixType`|`[['XmlSchema:[A-Z]{2}[0-9]{3}']]`|Some(5)|None|None|71|yes|Class(A-Z,Exact(2)), Class(0-9,Exact(3))|
|2.6|A|`OctalValueType`|`[['XmlSchema:[0-7]+']]`|None|Some(1)|Some(16)|79|yes|Class(0-7,OneOrMore)|
|2.6|A|`ROME_IdentityType`|`[['XmlSchema:[a-zA-Z0-9]+']]`|None|Some(1)|Some(10)|2|no|Class(a-zA-Z0-9,OneOrMore)|
|2.6|A|`STANAG_4607_PacketSecurityClassificationType`|`[['XmlSchema:[1-5]']]`|Some(1)|None|None|0|no|Class(1-5,One)|
|2.6|A|`UnitIdentifierType`|`[['XmlSchema:[A-Z]{2}[ABCDEGJMNSX][A-Z]{2}[0-9]{5}']]`|Some(10)|None|None|3|yes|Class(A-Z,Exact(2)), Class(ABCDEGJMNSX,One), Class(A-Z,Exact(2)), Class(0-9,Exact(5))|
|2.6|B|`FIPS_CountryCodeType`|`[['XmlSchema:[A-Z]{2}&#124;[ ]{2}']]`|Some(2)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`IPv4_AddressType`|`[['XmlSchema:(([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])\\\\.){3}([0-9]&#124;[1-9][0-9]&#124;1[0-9][0-9]&#124;2[0-4][0-9]&#124;25[0-5])']]`|None|Some(7)|Some(15)|11|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`IPv6_AddressType`|`[['XmlSchema:((:&#124;[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:&#124;[0-9a-fA-F]{0,4}))&#124;(((25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])\\\\.){3}(25[0-5]&#124;2[0-4][0-9]&#124;[01]?[0-9]?[0-9])))']]`|None|Some(2)|Some(45)|11|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`MilitaryGridType`|`[['XmlSchema:(([1-9]&#124;[1-5][0-9]&#124;60)[C-HJ-NP-X][A-HJ-NP-Z][A-HJ-NP-V]&#124;[ABYZ][A-CF-HJ-LP-UX-Z][A-HJ-NP-Z])(([0-9]{2}){0,5})']]`|None|Some(3)|Some(15)|7|yes|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_AIMIDB_MissionNumberType`|`[['XmlSchema:((((U0)&#124;[A-Z]{2})[0-9]{2})&#124;UNKN)']]`|Some(4)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_CodewordsType`|`[['XmlSchema: {11}', 'XmlSchema:([A-Z]{2} ){1} {8}', 'XmlSchema:([A-Z]{2} ){2} {5}', 'XmlSchema:([A-Z]{2} ){3} {2}', 'XmlSchema:([A-Z]{2} ){3}[A-Z]{2}']]`|Some(11)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_DateAndTimeType`|`[['XmlSchema:(([12]\\\\d\\\\d\\\\d)((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01])([01][0-9]&#124;[2][0-3])([0-5][0-9]))', 'XmlSchema: {12}']]`|Some(12)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_DateType`|`[['XmlSchema:([12]\\\\d\\\\d\\\\d((0[1-9])&#124;(1[012]))(0[1-9]&#124;[12][0-9]&#124;3[01]))', 'XmlSchema: {8}']]`|Some(8)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_DeclassificationExemptionType`|`[['XmlSchema:(X[1-8]( ){2})&#124;25X[1-9]&#124;[DNIO]&#124;( ){4}']]`|Some(4)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_DeclassificationType`|`[['XmlSchema:(DD&#124;DE&#124;GD&#124;GE&#124;O( )&#124;X( )&#124;( ){2})']]`|Some(2)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_IPON_IID2_ProgramCodeType`|`[['XmlSchema:([0-9][A-Z]&#124;[A-Z][0-9])']]`|Some(2)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_MSTGTA_TargetCategoryType`|`[['XmlSchema:[1-9][0-9]{4}&#124;[ ]{5}']]`|Some(5)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_MSTGTA_TargetLocationType`|`[['XmlSchema:([0-8]\\\\d([0-5]\\\\d){2}\\\\.\\\\d{2}(N&#124;S)(0\\\\d{2}&#124;1[0-7]\\\\d)([0-5]\\\\d){2}\\\\.\\\\d{2}(E&#124;W))', 'XmlSchema:([\\\\+\\\\-]{1}[0-8]\\\\d\\\\.\\\\d{6}[\\\\+\\\\-]{1}(0\\\\d{2}&#124;1[0-7]\\\\d)\\\\.\\\\d{6})']]`|Some(21)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_MSTGTA_TargetPriorityType`|`[['XmlSchema:[0-9][1-9][0-9]&#124;[0-9]{2}[1-9]&#124;[1-9][0-9]{2}']]`|Some(3)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_PATCHB_GravityType`|`[['XmlSchema:([3][1-3]\\\\.[0-9]{4})&#124;( {7})']]`|Some(7)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_ReleasingInstructionsType`|`[['XmlSchema: {20}', 'XmlSchema:([A-Z]{2} ){1} {17}', 'XmlSchema:([A-Z]{2} ){2} {14}', 'XmlSchema:([A-Z]{2} ){3} {11}', 'XmlSchema:([A-Z]{2} ){4} {8}', 'XmlSchema:([A-Z]{2} ){5} {5}', 'XmlSchema:([A-Z]{2} ){6} {2}', 'XmlSchema:([A-Z]{2} ){6}[A-Z]{2}']]`|Some(20)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NITF_UTC_TimeType`|`[['XmlSchema:(([01][0-9]&#124;[2][0-3])([0-5][0-9])([0-5][0-9])Z)', 'XmlSchema:( ){7}']]`|Some(7)|None|None|0|no|Alternation / multiple union alternatives; excluded|
|2.6|B|`NotationType`|`[['XmlSchema:[A-Z0-9]{5}&#124;UNKN&#124;NONE']]`|None|Some(4)|Some(5)|79|yes|Alternation / multiple union alternatives; excluded|
|2.6|B|`RecordOriginatorType`|`[['XmlSchema:[A-Z][A-Z]&#124;[E]&#124;[\\\\-]']]`|None|Some(1)|Some(2)|72|yes|Alternation / multiple union alternatives; excluded|

Predicted remaining OrderOfBattle blockers by shape only: MilitaryGridType, NotationType, RecordOriginatorType. All other nine support blockers are in A. No profile requires backtracking. Semantic profiles are unique by expression and facet row; no name matching. Literal SPACE remains stored verbatim. Every accepted byte is ASCII (0x20..0x7e), so UTF-8 byte length equals XSD character count; control characters, DEL and non-ASCII fail. Actual length/minLength/maxLength remain independent of pattern repetition; absent minLength is not synthesized.

## Implementation and measured impact

The shared `StringProfile::StructuredAscii` classifier admits exactly the 27 pinned
expression/facet rows, independently of declaration name. `BoundedAscii` remains
a separate authoritative classifier. Segment kinds are literal or ordinal ASCII
class, with One, OptionalOne, OptionalOneByTotalLength(4), Exact, Bounded, and
OneOrMore repetition. The length-guided case is the pinned overlapping `1?`
prefix and its three-character suffix. Every segment consumes left to right and
the input must be exhausted; there is no regex engine or backtracking.

The length-guided operation was explicitly scope-approved during completion.
For `1?[1-7][1-8]{2}`, the mandatory suffix has length exactly three and
the optional literal has length zero or one. Thus the only possible lengths
are three (omit the prefix) and four (consume literal `1`). Once length selects
the prefix, `[1-7]` and the two `[1-8]` members have fixed positions. For example,
`178` and `188` must omit the prefix and are valid; `189` is invalid, not a
reason to retry another partition. `2778` cannot be a four-character form because its prefix
is not `1`. This is **length-guided deterministic segmentation**, not regex
backtracking. The classifier emits this operation only for this exact pinned
expression with minLength 3 / maxLength 4; similar unobserved expressions or
facets fail closed. No alternation, generic regex parser, arbitrary optional
ambiguity, or widening of the admitted set is introduced. Focused classification
tests and the shared generated Rust/C++/Ada corpus enforce these boundaries.
Completion sabotage probes changed only isolated generated copies to consume
the optional prefix greedily: the valid `178` case detected the mutation in
Rust (exit 101), C++ (exit 1), and Ada (exit 1). Repository production code was
not modified for these probes.

Current Task 053 regression: FileNameType is admitted, so ProductMetadata moves
past it to RecordOriginatorType in dependency-closure schema order. NotationType
remains excluded; the earlier unvalidated NotationType expectation was incorrect.
Historical Task 053 evidence is unchanged.

The actual `length`, `minLength`, `maxLength` facets are tested separately
from the pattern. In `[A-Za-z0-9]{1,4}` / `maxLength=4`, the pattern requires
at least one; the facet independently forbids more than four. No `minLength`
facet is invented. `[0-9]` / `length=1` enforces both one pattern member and
its actual length facet. No admitted alphabet contains controls, DEL or non-ASCII.
SPACE is stored unchanged only when a particular profile permits it.

Generated Rust/C++/Ada model probes compile and run the shared corpus; the
Rust generated codec accepts JSON strings only and calls `new`. A synthetic
mock-OWP round trip and rejection test pass. Existing Task 040 lifecycle
composition remains unchanged; the new synthetic fixture covers required,
optional, bounded and unbounded members.

Pinned 12-cell coverage, before (merged Task 058) to after:

| Release | World | Backend | Kinds | Declarations | Field types / occurrences | Message closures |
| --- | --- | --- | --- | --- | --- | --- |
| 2.5 | closed | Ada | 5510 → 5537 /5557 | 5488 → 5515 /5557 | 13160 = | 526 → 552 /722 |
| 2.5 | closed | Rust/C++ | 5510 → 5537 | 5489 → 5516 | 13160 = | 529 → 555 |
| 2.5 | open | all three | 5510 → 5537 | 5401 → 5428 | 13160 = | 504 → 526 |
| 2.6 | closed | Ada | 5523 → 5550 /5570 | 5502 → 5529 /5570 | 13198 = | 526 → 553 /725 |
| 2.6 | closed | Rust/C++ | 5523 → 5550 | 5503 → 5530 | 13198 = | 529 → 556 |
| 2.6 | open | all three | 5523 → 5550 | 5415 → 5442 | 13198 = | 504 → 527 |

All 12 backend/release/world cells were measured; the paired Rust/C++ and
three open-world rows are equal per backend. No field-type/occurrence changes.
The exact full-schema Ada/Rust three-message gap remains Authorization,
AuthorizationRequest, CommSupportActivity, due to QueryType_Kind; projected
service schemas are not affected by that full-schema naming collision.

OrderOfBattle: Task 058 baseline verified at 430/442, 12 unsupported, first
IMO_NumberType, 55 selected (2.5) or 56 (2.6). After Task 059 it has 439/442,
three unsupported, first MilitaryGridType in all six release/backend cells.
The exact remaining list is MilitaryGridType, NotationType, RecordOriginatorType;
all require union/alternation. The service remains NOT READY.

In each release 131 real message closures reach a new structured declaration.
Measured after-change single-message readiness is 23 (2.5) and 24 (2.6)
READY in all backends, with other messages still blocked or limited by
projection. A completion comparison ran each of these 47 release/message
selections against the preserved Task 058 binary: all were NOT READY before,
with an admitted Task 059 declaration as their first blocker. Thus the real
newly-READY counts are **23 (2.5) and 24 (2.6)**, not 131. Before comparison
used Rust service-check; after-change backend parity is asserted by the pinned
test. The only 2.6-specific newly-READY message is OrdersMetadata.

The common 23 are AO_Activity, AO_Capability, AO_Command, CounterSpaceActivity,
CounterSpaceReport, DamageAssessment, DamageEstimate, DamageReport,
DMPI_DesignationRequest, FileMetadata, MissionPlanMetricsReport,
ProductDownloadReport, ProductDownloadRequest, ProductDownloadRequestStatus,
ProductDownloadTaskStatus, ProductOrFileDisseminationRequestStatus,
ProductOrFileDisseminationTaskStatus, SAR_Capability, SMTI_Capability,
StrikeActivity, StrikeCapability, StrikeCommand, SystemDeploymentCapability.

The measured Task 058 first blockers for those newly-READY selections are:
AO_PRF_CodeType 6/6, CounterSpaceSENO_Type 1/1,
CounterSpaceCycleNumberType 1/2, Link16_SpecificTypeModelType 3/3,
FileNameType 8/8, NIIRS_Type 3/3, IMO_NumberType 1/1 (2.5/2.6).
These counts describe only the newly-READY subset, not all messages.

| After-change first blocker among reaching messages | 2.5 | 2.6 |
| --- | ---: | ---: |
| READY | 23 | 24 |
| NotationType | 66 | 67 |
| MilitaryGridType | 7 | 7 |
| IPv4_AddressType | 6 | 6 |
| RecordOriginatorType | 3 | 3 |
| generated-support FIPS_CountryCodeType | 3 | 3 |
| TimeType | 1 | 1 |
| USMTF_SerialNumberOfQualifierType | 1 | 1 |
| topology/projection | 20 | 20 |

The source-level per-message impact and the exact readiness results are logged
by the pinned Deep CI test.

### Real vertical and fixture byte identity

`FileMetadata` (UCI 2.5) was NOT READY on `FileNameType` on the merged
Task 058 binary; on this branch `service-check` reports READY in Ada, Rust,
and C++, with Rust `--with-codec` READY. It has 52 selected types and no
generated-support expansion. `service-generate` succeeds in all three
backends; the generated Rust model and C++ model/service headers compile
under strict flags, and the Ada model compiles with GNAT. The synthetic
`StructuredNotice` generated codec and mock-OWP exercise the same filename
shape, including multiple dot extensions and invalid text.

The merged Task 058 release binary was built at
`d4a043d324fb5580a717c4e267304ec2a3c80c2f` in an isolated detached
worktree. All **184** repository `.xsd` fixtures (including two new Task 059
ones) were compared against the Task 059 release binary in three backends and
both generation worlds (1104 cells): **545 byte-identical successes**, **547
shared failures**, **12 new successes** (the two new Task 059 fixtures), **zero
changed successes**, **zero regressions**. Existing Task 037/038/039/041/042,
044/046/053/054/056/057/058 successful outputs remained byte-identical.

Pinned UCI revisions: 2.5 `093610b7753944059360d3236770ab446d039556`,
root SHA-256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`;
2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`, root SHA-256
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.


## Completion validation

The scope-approved recovery changed only tests, the shared corpus and documentation;
production generation was unchanged, so the finalized byte-identity campaign remains
valid. Classifier tests: 3 passed. Generated Rust/C++/Ada corpus: one test per
backend passed, Ada under both assertion policies. The pinned constrained-Binary
message-impact regression passed for both releases and confirms ProductMetadata
has only RecordOriginatorType as its remaining unsupported String dependency.
The final pinned Task 059 target passed 2 tests against both verified roots.

`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`, with both pinned UCI roots set,
completed with exit 0: **1205 passed**, **0 ignored**, **0 failed**
across 122 libtest/doc-test result blocks. Completion logs are
`/tmp/task059-completion-focused.log` and `/tmp/task059-completion-workspace.log`.

Final fmt, workspace all-target check, workspace all-target Clippy with
`-D warnings`, affected runtime crates on Rust 1.95.0 with the committed lockfile,
CI split guard (104 regression checks), and `git diff --check` passed. The earlier
real generated-code Rust 1.95 run and real FileMetadata codec both recorded exit 0.
GNAT 14.2.0 compiled/executed the generated Ada corpus. The full-schema
QueryType_Kind gap remains explicitly asserted and was not fixed.

Task 058 whole-message baseline attribution is preserved, not re-created:
IMO_NumberType 39 (+1 support), AO_PRF_CodeType 31, TimeType 28 per release
in its 305 reaching-message population. Task 059 has zero remaining IMO/AO_PRF
blockers in its 131 reaching-message population; TimeType remains 1 in that
subset. These different populations must not be conflated. Final workspace
bounded-ASCII message-impact output supplies the current 305-message distribution.

Delivery requires successful Fast and Deep CI on the exact pushed PR head;
no merge or auto-merge is authorized. Remote run IDs and final SHA are recorded
in the PR review-gate report after those runs finish.
