# Task 093 — Ada OMS JSON hexBinary codecs

## C1 correction and qualification (current work)

C1 evidence is isolated under `/home/zboll/task093-scratch/c1`; all original
failed schemas and logs above that directory remain untouched. Starting HEAD
and both sibling PR HEADs were freshly verified unchanged. No sibling work
was integrated, reset, rebased, or modified.

### Stage-specific corrections

`parse_local_element` rejects child declarations before resolving `type`.
The original inline restriction therefore tests **frontend** rejection, not
Binary model/codec readiness. The runner now requires exit **1**, the exact
relevant `unsupported XSD construct: xs:anonymous element type` diagnostic,
and no output directory from both `service-check` and `service-generate`.
It records stage, command array, cwd, exit, expected diagnostic and actual log.

The separate backend IR test
`task093_field_local_binary_facets_fail_at_model_preflight` constructs
required and optional direct HexBinary `Data` members with local `length=4`
and requires the exact error
`unsupported Ada IR construct: field constraints on Data`. A preliminary
test-only `u32`/`u64` assignment compile error is retained in
`ir-field-facets.log`; the corrected test passes in
`ir-field-facets-corrected.log`. No production change was needed.

Both original malformed Choice schemas are retained as frontend tests.
Additional syntactically valid named Choice declarations now reach codec
readiness and specifically reject direct and named Binary alternatives.
A named Binary pattern restriction rejects at the existing frontend pattern
boundary; a separate core IR test verifies `Binary pattern constraints`.
Unknown lexical provenance and manually supplied Base64 provenance remain
independent core controls; frontend Base64 rejection is not substituted for
them. Invalid cardinality, repeated direct/named Binary and field-local
constraints remain negative controls. No frontend behavior was changed.

The synthetic runner has ten generator controls, each executed through both
CLI actions (20 negative invocations per run). Executable decoder negatives
remain inside the original 33 codec checks; model coverage remains six checks.
Qualified non-OAM names run as a separate full fixture variant.

### Compiler and real-source evidence

`complete-{14,16}{,-qualified}/` passed all Normal/Check/Ignore cells,
each executing the 33-check codec and six-check model consumers. Final
manifest-producing reruns use `certified-*` evidence directories.

`real-2.{5,6}-{14,16}/` passed all three policies with 14 checks per cell.
The real runner verifies the unchanged root SHA-256, extracts the actual
`LogMDT.Data` member, `FileMetadataMDT.SHA_2_Hash` member, and the complete
`SHA_2_256_HashType` restriction (32 octets). UCI 2.6 additionally retains
the complete `HexBinaryType` declaration. A synthetic `BinaryProbe` envelope
allows independent qualification without weakening the full Log selection.
`provenance.json` preserves exact fragments and fragment hashes; source
definitions, facet values, restriction ancestry and optionality are unchanged.
It also inventories `SubsystemStreamMDT.SubsystemStreamBinary` and
`ProductMetadataMDT.SHA_2_Hash`. Typed execution checks exact octets, high-bit
bytes, empty/present/absent data, canonical uppercase, lowercase and XML
whitespace input, malformed hex, and 31/33-octet hash rejection. Only expected
`Codec_Error` is caught. This is **extracted real-schema Binary qualification,
not full Log service qualification or real-provider acceptance**.

Fresh exact Task 091 baseline/current Log measurements:

| Selection | Selected types | Emitted declarations | Codec baseline → Task 093 | First blocker baseline → Task 093 |
|---|---:|---:|---|---|
| Log 2.5 | 42 | 41 | 39/41 → 40/41 | LogMDT.Data Binary → OwnerProducer repeated Choice |
| Log 2.6 | 44 | 43 | 41/43 → 42/43 | HexBinaryType Binary → OwnerProducer repeated Choice |

Both remain NOT READY. Exact logs are `log25-{baseline,current}.log` and
`log26-{baseline,current}.log`. Task 092 was not used to remove the blocker.

### Pins and preservation

Fresh OMS source checkout at `726272bd0390982a759c91a9cf4e13b81c2b510b`
reproduces DOCX SHA-256
`b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
Fresh W3C Second Edition HTML reproduces
`980de872aa2c50013d5202176eb85eef3f183a7df635fdcb6b4aa34c1c12eb72`.
Both UCI roots reproduce the previously recorded hashes.

`byte-preservation.json` records **44 equal outcomes: 43 successful
generations and one matching unsupported rejection**. Successful outputs
cover Rust models/APIs/codecs, C++ models/APIs, existing Ada model/API output,
Binary-free Ada codecs, direct and combined DateTime, Duration and combined
temporal codecs. Successful Binary-free Ada codecs explicitly contain no
`Encode_Hex_Binary` helper. Synthetic and extracted real Binary model/API
bytes also match with and without `--with-codec`. Final source hashes are
recorded in `final-source-hashes.json`; generated hashes and locks accompany
each compiler campaign.

### CI integrity localization

The new Deep job is appended after all inherited jobs. The inherited mutation
test originally appends a purported extra fetch at EOF. With artifact metadata
at EOF, that text is not an executable run command, so the guard correctly
does not count it. `integrity-final-mutations.log` preserves the failure.
The narrow correction duplicates the real fetch line **inside its owning
`real-uci` job**, asserting the mutation changes that block. No guard was
weakened, check count reduced, inherited job reordered or Task 092 fix copied.
This may overlap future sibling CI-test changes and requires reconciliation
if Task 092 subsequently lands. Corrected full self-test evidence is separate.

### Publication gate status

During C1, PR #92 merged as
`624364c4f8f6212edee3afc04e331c4064202659`. Fresh fetch confirmed the original
Task 091 HEAD is an ancestor and its tree is byte-identical to final main.
Task 093 integrated this history-preservingly with `git merge --ff-only
origin/main`, preserving every uncommitted change. No source or fixture bytes
changed, so the ongoing qualification remains applicable to the integrated
tree. Publication, if qualified, must target **main** rather than the former
feature parent. PR #93 was still open and has not been integrated.

Full inherited campaigns, final workspace completion, final integrity reruns,
documentation disposition and publication are still being evaluated. Until
their results are recorded, the historical `QUALIFICATION_FAILED` disposition
below is not replaced by a ready-for-review claim. No hosted CI result or PR
identity is implied by local executable success.

### C1 mandatory local completion

All **18 inherited campaigns** completed exit zero: Tasks 087A/B, 088,
089/089R, 090/090R and 091/091R under GNAT 14.2.0 and 16.1.0, each campaign
executing Normal/Check/Ignore. `inherited-results.json` records exact driver
commands/exits and each campaign retains compiler/build/consumer logs.
All eight final Task 093 campaigns in `compiler-matrix.json` passed: two
compilers times the OAM/non-OAM synthetic variants and two real UCI releases.

Rust 1.95 locked workspace all-target fmt/check/Clippy/tests all exited zero.
Workspace tests report **1,415 passed, zero failed, eight pre-existing ignored
external-environment tests**, across 162 summaries; ignored tests are not
claimed as executed qualification. The Ada backend and core readiness suites
are included. Separate CLI codec suite: 18/18; guard wrapper: 8/8.
Final warnings-denied Clippy also passed after adding all focused IR tests.
Python/JSON/XML/workflow syntax and `git diff --check` pass. CI split guard,
layout checker, and corrected mutation suite (209 checks) pass.

The production generator was rebuilt after the final test additions; its
binary remains identical to the qualified generator, SHA-256
`54cbcb6dbf177087b4a116a38b0c07fcb4f3b4eec10c7e05acb8726d2c9a7866`.
Final source-manifest SHA-256:
`3c6e36e12512f69f89457838fea5a6c1fee912d1dedc33a8f2fb0d71c6ee6b4b`.
Each compiler campaign preserves generated source hashes, exact command
arrays/exits, GNAT/GPRBUILD identities and dependency locks. Alire is 2.1.1,
GPRBUILD package 26.0.1 (executable 26.0.0), GNATCOLL/minimal 26.0.0.

Semantic review confirms only resolved HexBinary enables codecs; no model,
Rust, C++, IR or frontend behavior changed. Length construction remains inside
the existing checked constructor; only its `Constraint_Error` is translated.
The encoding bound precedes multiplication; decoding trims trailing XML
whitespace before incrementing the leading index, then uses bounded pair
offsets and byte arithmetic (0..255). The JSON string length bounds the decoded
vector capacity. No nibble array or additional emission plan is introduced.
Helper names are reserved case-insensitively. Full Log remains unsupported
because repeated Choice is outside this branch. Sleet lexical-character length
behavior is not emulated; no transport, hardware or provider acceptance is
claimed.

Mandatory local qualification is complete. Publication targets main because
PR #92 merged; exact-head hosted CI remains required. No automatic merge is
authorized. Hosted identities and HEAD parity will be recorded in the PR and
final report rather than predicted here.

## Original attempt disposition: QUALIFICATION_FAILED (historical)

The following sections preserve the original stopped attempt. C1 results and
current publication status are recorded above; historical failures are not
silently replaced by later passing evidence.

## Verified baseline and isolation

- Main: `13a28bf69c5e5e63833ffdb0fdeea701bbe2fd25`.
- Starting/dependency HEAD: `3eb92115e2466a151c2bad9b9ff87b2ab5ba2c72`.
- PR #92 remains open at that HEAD.
- PR #93 remains open at `09a1b2e630acfda7e971bbe2524f3763a54e6b50`.
- Branch: `feature/093-ada-hexbinary-codec`, created directly from PR #92.
- Worktree: `/home/zboll/git/ams-gra-codegen-oms-task093`.
- Evidence: `/home/zboll/task093-scratch`.
- No sibling branch, worktree, evidence, or PR was modified.

## Contract and implementation

The reviewed Task 052/053 research maps semantic octets to JSON strings:
empty is `""`, `00 A1 FF` is `"00A1FF"`. XML Schema Part 2 Second Edition
3.2.15.1 admits both hexadecimal cases; 3.2.15.2 canonicalizes uppercase.
Sections 4.3.1–4.3.3 count octets. Section 4.3.6 collapses XML whitespace:
surrounding XML whitespace is accepted, internal whitespace is invalid.
OMSC-SPC-013 Rev B 6.1.3/6.1.4 supplies the String representation.
Fresh normative document hash reverification remains outstanding.

The draft changes preserve the model: direct Binary uses
`Binary_Vectors.Vector` of `Interfaces.Unsigned_8`; unconstrained named
Binary uses its public record `Value` component; constrained named Binary
uses checked `Create`/`Value`. Only constructor `Constraint_Error` is mapped
to `Codec_Error`, after lexical extraction. Codec length checks are not
duplicated. The decoder trims XML whitespace and decodes pairs directly,
without an intermediate nibble array. Encoding checks the doubled String
length before allocation. New helper names are reserved in collision preflight.
Helpers are inserted only when projected operations reference Binary.

Readiness requires resolved HexBinary provenance and supported named length
facets. Unknown provenance, Base64, direct field facets, repeated Binary,
direct/named Binary Choice alternatives, unsupported aliases/lists,
abstract values and unsupported facets remain excluded. Named declarations
are checked in their own emission context. No model API changes were made.

## Executed evidence

The initial synthetic codec runner completed all six cells:

| Compiler | Normal | Check | Ignore |
|---|---|---|---|
| GNAT 14.2.0 | 33 checks pass | 33 checks pass | 33 checks pass |
| GNAT 16.1.0 | 33 checks pass | 33 checks pass | 33 checks pass |

Evidence: `gnat14/` and `gnat16-v2/`. These runs precede the added model
consumer and generator negative controls; they do **not** certify the final
runner. The later `gnat14-v3/` and `gnat16-v3/` runs completed Normal and Check
with codec count 33 and model count 6, but were interrupted during Ignore by
the command execution timeout. Do not count those as completed campaigns.

Typed tests cover vector lengths, `00`, `80`, `FF`, empty values, optional
presence/absence, unconstrained/constrained named values, restriction ancestry,
nested and inherited Records, mixed-case input and uppercase re-encoding.
Negative tests catch only `Codec_Error`, checking no successfully returned
typed payload is published on failure. Cases include invalid digits, odd
counts, internal whitespace, non-string kinds, named lengths, unknown/missing
members and failures in both earlier and later nested Records.

The final runner attempt `gnat16-v4/` failed at `field-facets`: the anonymous
field-type fixture fails atomically in the frontend with
`unsupported XSD construct: xs:anonymous element type`, but the runner
requires a diagnostic containing `Binary`. Exact command, exit 1, and actual
diagnostic are preserved in `field-facets.command.json` and `field-facets.log`.
This is a runner expectation failure, not evidence of accepting the schema.
Hand-built IR direct-field facet rejection passes the focused core test.
The final runner has not passed and must be corrected and fully rerun.

Other completed gates:

- Rust 1.95 locked workspace all-target check.
- Warnings-denied workspace all-target Clippy (before the final added tests).
- Core service codec tests and two focused Task 093 readiness tests.
- CLI service codec suite: 18 passed, including zero-OMS/atomic-output controls.
- Formatting and `git diff --check`.
- Python/JSON/XML/YAML syntax.
- CI split guard, layout check, and mutation self-test (209 checks).
- Task 087A/B on GNAT 14 and 16, Task 088 on GNAT 14.
- Task 091R on GNAT 16, all policies, 14 controls.

Workspace tests, Task 088 GNAT 16 and remaining inherited campaigns were
interrupted at the stop disposition; no full-suite pass is claimed.
`stopped-processes.json` identifies only Task 093-owned process trees stopped.

## Byte preservation

`byte-comparison.json` records 30 equal baseline/current command outcomes and
generated-file hash maps across Rust model/API/codec, C++ model/API, Ada
model/API and Binary-free Ada codecs, including Duration and combined temporal
output. Some unsupported outcomes generate no files; this is not 30 successful
code-generation selections. Full DateTime selection coverage and explicit
Binary-free helper absence controls remain outstanding. Synthetic Binary
model/API bytes match with and without `--with-codec`.

## Real UCI measurements (partial)

Both unchanged UCI roots were fetched through the existing pinned scripts:
2.5 root hash `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`,
2.6 root hash `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

| Log selection | Selected types | Codec declarations | First blocker | Status |
|---|---:|---:|---|---|
| 2.5 current | 42 | 40/41 | SecurityInformationType.OwnerProducer repeated Choice | NOT READY |
| 2.6 current | 44 | 42/43 | SecurityInformationType.OwnerProducer repeated Choice | NOT READY |

Current logs: `log25-current-v2.log`, `log26-current-v2.log`.
Exact baseline measurements were interrupted and remain outstanding. Historical
Task 091R 2.5 measurement is 39/41 with `LogMDT.Data` as blocker; it is not
substituted for a fresh measurement. Additional real Binary provenance
inspection and extracted real-schema executable qualification are incomplete.
No complete real-UCI service execution or provider acceptance is claimed.
Pinned Sleet's known lexical-character-versus-octet length defect must not
change the Ada contract. No hardware/transport interoperability was executed.

## Remaining mandatory work

Correct and rerun the mandatory runner; qualify qualified non-OAM names;
complete all core context/facet controls, isolated model-only execution,
normative pins, fresh before/after UCI measurements, extracted real Binary
execution, all inherited campaigns and workspace tests, and comprehensive
byte/helper controls. The Fast CI job is drafted but has not run hosted.
No Deep CI job was added without established real-source qualification.
No commit, push, hosted run identity, PR, final Task 093 SHA or publication
milestone is available. HEAD remains the starting SHA with local changes.