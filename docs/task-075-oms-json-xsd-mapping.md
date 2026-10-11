# Task 075 — Bounded OMS JSON-to-XSD mapping

## Disposition and authority boundary

**OMS_JSON_XSD_MAPPING_PARTIALLY_QUALIFIED**. Executable research mapping and
independent pinned XSD assessment succeed on complete
`DataRecordListManagementRequest` messages for UCI 2.5 and 2.6, known
`ForeignKeyMapML`, and Task 074's **synthetic** `Task074SyntheticML` in a complete
research bundle. This is not production unknown-subtype handling: the private
definition is absent from the public schema but present in the supplied verified
assessment closure. Public-only input rejects at mapping, without inventing an
unknown type's content from the base. Incomplete closure fails before mapping.

No deployment approval is required to run these experiments. Approval, operation
authorization, publication authority and peer interoperability remain independent
production gates. No opaque carrier, relay, registration or public validation API
is introduced. `NotClosedUnderOpenExtensions`, GenerationWorld, readiness,
generated codecs/models, frontend/IR/backend, ingress/dispatch/lifecycle and Sleet
authorization remain unchanged. Task 072–074 historical evidence is untouched.

Partial, rather than universal CAL qualification, is intentional: the validation
tables do not explicitly specify conversion of `$type` into an XML attribute
(Table 6.1-2 says empty attributes/in-scope namespaces), while §6.1.2 explicitly
requires concrete identity and XSD derivation. This experiment realizes that
identity as namespace-bound `xsi:type`, with XSD checking the resulting complete
instance. It does **not** claim a literal attribute-free reconstruction of Table
6.1-2, nor resolve that textual tension by silently dropping the discriminator.
Likewise exponent-form JSON numbers for decimal-family fields are unsupported,
not declared universally invalid: Table 6.1-3 preserves token characters and XSD
decimal lexical space excludes exponent spelling. Broader CAL interpretations
need independent review, not deployment-owner permission to perform research.

## Immutable baseline and sources

Fetched `origin/main` and starting base:
`6dcdd2d18f068c07521a12fe84e3239c44cf43f0`, exactly the supplied SHA.
Tasks 072–074 are merged in that ancestry. All-state Task 075 PR search was empty;
no existing Task 075 branch. Created branch
`research/075-oms-json-xsd-mapping` directly from fetched main in
`/home/zboll/git/ams-gra-codegen-oms-task075`. Original clean Task 059 checkout
at `263df8be24439f368b64cfde56b9b0c70a93e226`, sibling/prunable worktrees,
Task 074 reconciliation worktree and shared caches were preserved. Available disk
was 829 GiB before compilation; scratch/build output is exclusively under
`/home/zboll/task075-research`.

| Source | Immutable identity |
|---|---|
| OMS official source | `726272bd0390982a759c91a9cf4e13b81c2b510b` at `https://gitlab.com/open-arsenal/oms/standard.git` |
| Official CAL | `docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`; SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7` |
| UCI 2.5 source | `093610b7753944059360d3236770ab446d039556` at `https://gitlab.com/open-arsenal/uci/standard.git` |
| UCI 2.6 source | `78eb61b6112c8bffa40820c33124b57787fc5bd9`, same repository |
| 2.5 root / security SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` / `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 root / security SHA-256 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` / `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| Validator | unchanged `scripts/task074/bundle.py` at starting base; lxml 5.4.0 / libxml2 2.9.14 |
| Validator binary SHA-256 | etree `27d8b7e79e217e6946ad6871c13b6bdfbcf55a1a3f6b3ab590cb892fc3a5deca`; libxml2 `8f2643af68a9eba917929046806dabd1848867068d0eee0f6799288a8e886963` |

The official revision/hash was rechecked and `word/document.xml` read directly,
not substituted with prior summaries. Paragraph references below are one-based
indices of **all** `w:p` descendants, including table cells, in document order;
text is concatenated from `w:t`. This gives reproducible evidence anchors without
relying on Word page pagination. Mapper/qualification/validator source digests,
full ordered closure manifests, raw JSON bytes (UTF-8 text), raw digests, XML and
XML digests are frozen in `tests/fixtures/task075/evidence.json`. The PR head is
the immutable repository identity for the new implementation, not a
self-referential SHA embedded in its own commit.

## Profile frozen before implementation

The selected vertical requires global element identity, flattened MessageType
security/header content, flattened RequestBaseType request content, ID_Type
extension records, scalar optional AddRecord/DeleteRecord, abstract
ManagedListBaseType assertions, ForeignKeyMapML optional unbounded DRLE entries,
and the security OwnerProducerChoiceType. OwnerProducer itself is **required
unbounded** and is an array even with one entry. ManagedListBaseType has no
inherited value members; RequestState is the enclosing inherited-field control.

Supported mapping forms:

* Explicit expected expanded global element, exactly one envelope member;
  bare names identify only the exact OAM URI, Clark names preserve other namespaces.
* ASCII NCNames, qualified local elements, named types, element-only/empty complex
  types, sequence/choice groups with maxOccurs 1, complex **extension** chains.
* Named simple restriction chains down to string/normalizedString/token,
  dateTime/date/time/duration/anyURI, boolean, decimal and integer-family builtins.
  Enum/facet checking remains XSD-owned; UUID strings use their schema restriction.
* Scalar versus array dictated by maxOccurs, optional absence and zero entries
  only when minOccurs allows; repetition order preserved, choice membership checked.
* `$type` resolved in the complete snapshot, concrete and extension-related;
  properly bound QName in XML, never a Clark attribute value.
* Numbers retain exact RFC 8259 tokens; ordinary decimal/integer lexical spellings
  map unchanged. String, number, boolean, object, array and null stay distinct.
* XML 1.0 representable strings including CR preserved by lxml serialization
  (`&#13;` on the wire), checked by reparsing and comparing character information.

Deterministically unsupported: recursive value/type paths, repeated groups,
all/wildcard/ref particles, anonymous types, unqualified local members, mixed or
simple complex content, complex restriction, list/union, float/double/binary and
other out-of-profile builtins, non-ASCII NCNames, element default/fixed/nillable
policies, and decimal exponent forms. These are **unsupported mapping**, not a
claim of invalid OMS JSON. Out-of-profile declarations elsewhere in UCI do not
block selected mapping: declaration indexing is broad but semantics are examined
only on traversed paths. This index is mapping metadata, not a new XSD processor.
XSD owns facets and final derivation constraints, including restrictions not
reimplemented here. No all-UCI completeness claim.

## Conformance table

Implementation references below are in `scripts/task075/mapper.py`; executable
named probes are in `scripts/task075/qualify.py`. Expected trees/characters are
hand-authored independently of the mapper; Task 074 XML is a separate reference.

| CAL rule and exact evidence | Required shape | Mapped information items | Implementation | Positive | Negative | Limitation |
|---|---|---|---|---|---|---|
| §6.1.1 P528–535; Table 6.1-1 P799–817 | One-member object, expected global QName | One document element, UTF-8 XML 1.0 | `map`, `qname` | both `known complete`, closed vector | wrong/multiple/nonobject envelope | expected service identity supplied externally |
| §6.1.2 P572–578; §6.1.5.2 P820–826 | Sequence members any order, names expanded | Schema-order namespace/local children | `qname`, `particle`, `member_name` | `complete permuted`, `permuted members` | unknown member, expanded duplicate | ASCII qualified names only |
| §6.1.2 P580–583; Table 6.1-2 P838–842 | Repeated element is array even for one | Ordered repeated sibling elements | `particle` | arrays 0/1/2, real repeated 0/1/2 | array-required/scalar-required | repeated groups excluded; occurrence checked before XSD |
| §6.1.2 P573 | Exactly one chosen branch when required | Selected particle's children | `particle_keys`, `particle` | choice A/B, real security choice | choice conflict, security choice conflict | no arbitrary repeated group choice |
| §6.1.2 P584–588, XSD Structures §3.4.6 | Actual complex definition and valid derivation | Namespace-bound `xsi:type` | `definition`, `derived`, `value` | known/private, same local private identity | missing/abstract/unrelated/unresolved type | extension ancestry precheck; XSD final assessment; validation-table tension above |
| §6.1.3; extension example P689–710 | One flattened object | Base children followed by extension children | `content`, `value` | complete request, synthetic inherited Count | invalid inherited Count/RequestState | ManagedListBaseType has no fabricated member |
| §6.1.4 examples P761–787; §6.1.5.4 P855–857, Table 6.1-3 P858–866 | Boolean tokens, decimal-family number tokens, other supported simple types strings | Exact token/decoded character content | `primitive`, `simple`, `Number` | large integer, exact values, real UUID/enums | numeric string/boolean, string number, fraction integer XSD | exponent unsupported; floats excluded |
| §6.1.5 P797, Table 6.1-2 P840, XSD Structures §3.9.4 | Complete content-model-consistent document | Independent XSD assessment | `assess`, unchanged Task 074 Bundle | 2.5/2.6 private complete | intended enum/pattern XSD invalidity | research-only result, no authorization |
| Research policy, NOT normative CAL | Raw UTF-8 finite unique names and finite output | Original bytes/digest retained | `raw_parse`, limits, bounded worker | policy boundaries | duplicate escaped `$type`, byte/depth/member/array/number/output limits | byte/depth before parsing; counts after allocation bounded by input and worker |
| XML 1.0 representability policy plus CAL Table 6.1-3 | Decoded XML-compatible characters | CR, Unicode and escapes preserved | `simple`, lxml serializer | exact values and preservation | NUL/lone surrogate | XML 1.0 only |

## Pipeline, snapshot custody and resources

`raw bytes -> duplicate-aware json.loads -> envelope/QName/shape -> schema metadata
-> complete lxml XML -> Task 074 pinned offline XSD -> research-only evidence`.
No call to production `unwrap_global_element`; no binary float conversion and no
handwritten JSON grammar. Python's proven parser provides object-pairs and exact
number-token hooks. Escaped names are decoded before duplicate checking. Expanded
QName ambiguity is checked separately. Null, unexpected members or alternatives
are never discarded to manufacture validity. Empty strings are character-empty
simple values; empty object is allowed only for empty concrete content; empty
arrays produce zero siblings only where optional. Missing required content is a
shape failure. Fractional integer spelling reaches XSD and fails lexical space.

Metadata is parsed exclusively from `Bundle.files`, after Task 074 digest/closure
and binary-pin checks. No mutable ambient rereads, catalogs, payload schema
locations, network resolution or extra dependencies are introduced. One public
and one private bundle per release are compiled once and reused across probes.
The metadata and schema engine therefore see identical bytes. Manifest generation
is only fixture preparation, not an approval authority. Default Bundle authority
rejection is asserted; `research=True` is confined to qualification code and is
explicitly non-authorizing.

Finite limits: 65,536 input bytes; depth 32; 2,048 total members; 256 array entries;
128 characters per number token; 65,536 serialized XML bytes. A string-aware
bracket preflight checks depth before json.loads. Byte bounds are preallocation;
member/array counting uses parser hooks/postwalk, backed by input size. Output
text has a conservative six-times UTF-8 budget before serialization plus a final
serialized-byte check. This can reject some XML below the maximum and is an
explicit research resource policy, not CAL invalidity. Namespace/element metadata
overhead is bounded by the verified schema and worker, not a streaming serializer.
Worker limits reuse Task 074: 30 CPU seconds, 768 MiB address space, no core dumps,
60-second wall timeout. Worker nonzero/timeout are separately asserted resource
outcomes, never substitutes for a valid payload rejection. This is not aggregate
concurrency certification or OS network isolation evidence.

Outcome taxonomy: `syntax`, `policy`, `envelope`, `shape`, `unsupported`,
`mapping`, Task 074 bundle failure (`closure`, `digest`, `missing-file`, toolchain),
`xsd:invalid:*`, `assessment-policy`, `resource`, and
`research-schema-valid-not-authorized`. A bundle/compiler crash cannot satisfy a
payload-invalidity probe; every negative asserts its exact stage/reason.

## Complete-message and differential evidence

Both releases have identical intended outcomes:

| Probe | UCI 2.5 | UCI 2.6 |
|---|---|---|
| Known complete + permuted members | research schema valid | research schema valid |
| Real security incompatible choice | shape:choice-conflict | shape:choice-conflict |
| Known repeated DRLE arrays 0/1/2, ordered UUID assertions | valid | valid |
| Known DeleteRecord and empty concrete managed list | valid | valid |
| Synthetic private complete; private DeleteRecord | valid | valid |
| Same private JSON, public-only closure | mapping:unresolved-type | mapping:unresolved-type |
| Private dependency omitted | closure before assessment | closure before assessment |
| Invalid inherited RequestState | XSD enumeration constraint | XSD enumeration constraint |
| Invalid Task074Code | XSD pattern constraint | XSD pattern constraint |

The hand-authored raw `message.json` maps to information items equal to Task 074's
independent complete XML for each release. Field values, member permutations,
AddRecord/DeleteRecord and concrete subtype changes use the same mapper, not
fixture-specific output templates. Repeated entries are nonempty real
ForeignKeyPairDRLE objects; UUID order is independently asserted. The private
overlay is Task 074's labeled synthetic extension, not an authoritative UCI type.

Closed generated-codec differential uses actual
`generated_codec_qname::record::task051_complete_document_combines_runtime_and_codec_keys`
and its exact `{urn:test}MessageB/Count=7` vector. The Python control maps that
same document to a hand-authored expected tree and independently XSD-assesses it.
The existing generated test executes decode/encode through `OmsJsonCodec`, not a
handwritten codec. Prefix spelling/JSON object order are not comparison targets.

`scripts/task075/codec-control.sh` regenerates Task 068's **generation-time**
same-namespace PrivateB overlay using the production CLI, executes checked
decode/encode through public `OmsJsonCodec`, and rejects PrivateC and mismatched
PublicA. The Python campaign assesses the identical private vector and compares
it to an independent expected XML tree. This is generation-time knowledge, not
successful production runtime handling of unknown subtypes. The standalone
generated executable has two unused-code warnings; workspace Clippy is clean.
No backend multi-namespace feature or full-real-message generator campaign was
forced to obtain this control.

## Execution and reproducibility

Run from `/home/zboll/git/ams-gra-codegen-oms-task075`:

```sh
python3 scripts/task075/qualify.py
python3 scripts/task075/qualify.py --real \
  --uci /home/zboll/git/.task064-scratch/schemas/uci25/UCI_MessageDefinitions_v2_5_0.xsd \
  --uci /home/zboll/git/.task064-scratch/schemas/uci26/UCI_MessageDefinitions_v2_6_0.xsd
TASK075_SCRATCH=/home/zboll/task075-research bash scripts/task075/codec-control.sh
```

The real command requires exactly both pinned roots: missing root, missing
release or unavailable pinned validator fails nonzero, never a success stub.
Fresh inputs can be acquired with the existing pinned fetch scripts into
task-specific storage; paths above are read-only cached verified snapshots.
No alternate lxml installation is silently accepted. Python pin evidence is
machine-specific and separate from Rust CI.

Rust 1.95 validation used `CARGO_TARGET_DIR=/home/zboll/task075-research/target`
and `CARGO_INCREMENTAL=0`: workspace fmt, all-target check, all-target Clippy
with `-D warnings`; focused Task 068, 072, 073 wire-boundary/QName codec,
service_codec/service_generate/runtime_contract tests; Task 028/029 three-backend
closed/open/private-overlay controls. No Rust files/dependencies changed.
Task 068 fixture accounting and six frozen readiness sets passed. Applicable
Python compilation, shell syntax and `git diff --check` are checked separately.
Final exact counts and head statuses are recorded in the PR/final report.

Final local results: **101 campaign probes** (73 synthetic/resource/differential
controls plus 14 complete-message probes per release), all passed; 33 successful
research assessments, five policy-boundary acceptance controls and 63 intended
negative outcomes. The synthetic-only command separately passed all 73 probes.
A second full campaign was byte-identical to frozen evidence. Two additional
CLI invocations with missing required real inputs failed nonzero as required.
Rust focused suites passed **123 tests**, zero failed, one existing real-UCI
opt-in test ignored (99 boundary/codec/service tests plus 24 Task 028/029 backend
controls). The standalone private-codec executable passed independently.
Formatting, workspace all-target check/Clippy, Python compilation, shell syntax,
Task 068 fixture accounting and diff whitespace checks passed.

Source SHA-256: mapper
`c83616a88b8dcdba9df553362933c4039516c3ad9c5db968d9071b2c9be788be`;
campaign `37b12f792d172bf0df1e768fa80fbec31f25559a96a8477ac4d4472b1791050e`;
unchanged validator `06a2959481330eb844c971f08bbb1dd0fd2c04d17d668238ae73c1cca09a889d`.

New Python campaigns and shell differential are **local opt-in only**, not hosted
CI requirements. Existing hosted Rust workflows trigger normally; a green Rust
workflow is not certification of an unexecuted Python campaign. No hosted CI
changes, duplicate dispatches, unrelated multi-hour UCI/Sleet campaigns or full
schema readiness reruns. The existing opt-in real-UCI Rust contract test remains
ignored in the focused synthetic Rust run; the executed Python real vertical is
separate evidence, not inferred from that ignored test.

Remaining mapping gaps: the explicitly unsupported constructs above, literal
CAL validation-table `$type` interpretation, general numeric forms, non-ASCII
names, exhaustive optional content beyond the tested vertical and independent
cross-engine/peer information-item comparison. Separate deployment blockers:
authenticated bundle ownership/custody, operation authorization, maintained
validator TCB distribution, concurrent resource isolation, revocation/transactional
policy checks and peer agreement. Neither set is waived by research schema validity.

Commit normally, push and open a non-draft PR with auto-merge disabled; verify
tested local HEAD = remote HEAD = PR head and record any main advancement.
Stop for independent review. Do not merge or begin Task 076.

## Task 075A — independent-review corrections

Prepared on reviewed parent `669d370704a8765688b6a88819cd5b13ba1eb741`,
mapper Git blob `2dec3db32de22e52edd62b2412954db04505139c`.
Fetched main remains `6dcdd2d18f068c07521a12fe84e3239c44cf43f0`; no merge,
rebase or history rewrite. PR #75 was verified open, non-draft and unmerged,
with auto-merge disabled. No reviewer archive or inline reproduction comments
were available locally/on the PR; supplied minimal findings were ported into
the unchanged pinned Bundle harness, not an alternate in-memory validator.

### Root causes and before/after

`tests/fixtures/task075/review-before.json` records successfully compiled,
complete digest-bound fixture manifests, independently assessed XML, raw JSON
and original reviewed-mapper outcomes. These diagnostic reproductions were
rechecked using the reviewed source against the final fixture digests. The
known TypeError was caught **only** in scratch diagnostic code and recorded as
an exception, never credited as successful assessment. Python's set iteration
can reverse the operands in the exception text, so its recorded diagnostic
identity is `TypeError:mixed-absent-namespace-sort`.

| Finding | Original pinned outcome | Corrected pinned outcome |
|---|---|---|
| Unused no-namespace import beside qualified Record | TypeError during namespace sorting | research-schema-valid-not-authorized |
| Metadata-only Record with comments/PIs/annotations | unsupported:complex-content | research-schema-valid-not-authorized |
| Required choice with optional A and required B, empty object | shape:choice-missing despite independently valid empty XML | research-schema-valid-not-authorized |
| Genuinely required A/B choice, empty object (control) | shape:choice-missing; reference XML invalid | unchanged shape:choice-missing |

1. Namespace preparation mixed `None` with URI strings before sorting. Absent
   namespaces are now removed **only from prefix allocation**, before sorting;
   `{}` declarations remain in the complete declaration index. The same-local
   no-namespace Record/Record type cannot overwrite qualified Record identity.
   Selecting empty-namespace JSON still gives the existing explicit QName shape
   rejection; unsupported no-namespace members are not promoted to OAM. The
   selected namespace profile is not broadened.
2. Structural loops and `len()` checks counted inert XML nodes. Shared
   `structural_children` now ignores comments, processing instructions and
   xs:annotation consistently in indexing, complexType/complexContent/extension
   cardinality and particle traversal. Genuine unsupported XSD nodes remain
   visible. Hashed schema bytes are never stripped or rewritten by the mapper.
3. Choice absence checked only its own minOccurs. `Mapper.emptiable` now
   recognizes empty content for bounded existing element/sequence/choice forms:
   zero minOccurs, all empty sequence children, or an empty choice alternative.
   It inspects all children, caps nesting at the existing depth limit and rejects
   unsupported/repeated group forms. No element/default is invented; populated
   alternatives still use existing selection, occurrence and conflict checks.

**Nullable choices are supported**, including a nested sequence/choice empty
case, within this existing bounded profile. XSD Structures Second Edition
§3.9.6 `Particle Emptiable` states minOccurs is zero **or** the group's minimum
effective total range is zero. §3.8.6 `Effective Total Range (all and sequence)`
and `Effective Total Range (choice)` supply sum/minimum rules. This is consistent
with CAL §6.1.2 P572–573 and Table 6.1-2 P840: construct children in a
particle-valid order; choosing an empty-capable branch need not create a child.
Pinned independent empty-reference assessment supplies executable evidence;
this helper is not a replacement validator or a general content-model search.

### New regression accounting and corrected-source qualification

New fixtures: `review-base.xsd`, `review-unused.xsd`,
`review-no-namespace.xsd`, `review-metadata.xsd`. The metadata variant covers
sequence/choice comments, comments between complexType children, comments/PIs
around complexContent and extension, and legal annotation boundaries. Wildcard
particles still return `unsupported:particle` even when optional.

New normal-entry-point probe families: `review {base,unused,metadata} bundle`,
`review {base,unused,metadata} Record Text+Extra`,
`review {base,metadata} Base Text`, `{Nullable,Nested,Optional} empty`,
`Required empty`, `{Nullable,Required,Nested} {A,B,A+B}`, `Unsupported empty`,
their independent `reference` assessments, and `review selected no namespace
input`. Baseline/unused/metadata trees are compared prefix-independently.
Successful cases are compared to hand-reviewed reference information items;
empty required/conflicting alternatives have independently invalid references.

**70 new outcome probes**: three bundle compilation outcomes, 33 independent
reference XML assessments and 34 mapper outcomes. Tree equivalence and distinct
declaration-index checks are auxiliary assertions, not extra outcome probes.
No original case was removed or changed: an explicit evidence comparison
verified all original **101 results and every original evidence entry** unchanged.
Only review results/evidence and the two changed source digests were added.

Corrected synthetic campaign: **143 probes passed**. Corrected full campaign:
**171 probes passed**, including the unchanged 14 complete-message probes per
release. Repeated full output was byte-identical to the refreshed frozen evidence.
Both UCI releases retain known/private AddRecord/DeleteRecord successes,
ordered repeated DRLE controls, security-choice/occurrence negatives, intended
RequestState/Task074Code XSD rejection, public-only type rejection and incomplete
closure rejection. Default authority and both toolchain negatives execute.
Both missing-required-real-input CLI controls again failed nonzero.

Generated closed-codec control and regenerated generation-time private-overlay
control passed. Rust 1.95 workspace formatting and focused Task 068/072/073
wire/guard/QName tests passed: **17 tests**, zero failed, zero ignored in these
selected suites. No Rust sources/dependencies changed, so no new workspace
all-target/Clippy campaign was required or claimed; the original-head checks
above remain historical. Python compilation, shell syntax and diff checks passed.
No broad backend, full-schema or Sleet compiler campaign was repeated.

Corrected mapper SHA-256:
`417cdead53e03f051c1e309221ea393448683e4e9da20650f3da4296a63bd173`.
Corrected campaign SHA-256:
`47b5be93071500d95ad5c00359385c12ef5864ede14f122073b0f903a97a3095`.
Validator/source/toolchain pins and UCI source identities above are unchanged;
the pinned environment was available and no qualification was blocked.

Disposition remains **OMS_JSON_XSD_MAPPING_PARTIALLY_QUALIFIED**. The documented
CAL `$type`/`xsi:type` question, unsupported forms and production-adoption gates
remain unresolved. No production behavior, safety guard, authority bypass or CI
requirement changed. New-head hosted Rust CI is a separate status; it does not
execute the local pinned Python campaign. Exact corrective commit/head parity
and hosted run identity belong in the PR/final report. Stop for independent
review, leave PR #75 open/non-draft/unmerged, and do not begin Task 076.