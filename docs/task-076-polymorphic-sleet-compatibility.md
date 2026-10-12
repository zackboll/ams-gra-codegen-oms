# Task 076 — Polymorphic discriminator / pinned Sleet qualification

## Disposition and isolation

**POLYMORPHIC_COMPATIBILITY_PARTIALLY_CHARACTERIZED**. The executable bounded
matrix establishes useful agreements and incompatibilities, not universal CAL
conformance or authorization for production open-world support.

Immutable starting main: `15aa0aa431732de6efbc1eb4412ba94fa9158a33`.
`bf7b0b2cd8378a950d446ebb10ea57d4cf0977ed` is an ancestor. Fetch found no
advancement beyond the supplied SHA. Local/remote branches and all-state PRs were
inspected; no Task 076 effort existed. Worktree:
`/home/zboll/git/ams-gra-codegen-oms-task076`, branch
`research/076-polymorphic-sleet-compatibility`. Original checkout, siblings,
historical evidence and caches were preserved. Builds, broker checkout, copied
UCI closures, configuration and raw logs reside in `/home/zboll/task076-research`.
Baseline exact-main CI and Deep CI were in progress, not failed; no campaigns
were dispatched. Publication HEAD/parity and hosted status are in the PR/final
handoff, not a self-referential source digest here.

## Four separate interpretation conclusions

The official pinned DOCX was reopened as `word/document.xml` and its SHA-256
verified. `tests/fixtures/task076/cal-anchors.json` retains actual paragraphs and
tables, with reproducible one-based direct-body paragraph/table anchors (not page
numbers or unofficial Markdown). Source is OMS
`726272bd0390982a759c91a9cf4e13b81c2b510b`, official
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`,
SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.

**A — CAL explicitly says:** §6.1.2, body anchors 405–409, valid derivation
references XSD Type Derivation OK (Complex); another type in an expected-type
context carries `$type` naming the concrete complex type, bare only for the exact
OAM namespace, Clark form otherwise. §6.1.3 and examples (anchors 495–535) show
concrete identity and combined derived content. §6.1.4 supplies primitive-value
mapping, not a license to bypass inherited constraints. §6.1.5 anchor 618 makes
validity depend on assessment of the associated complete document. Actual Table
6.1-2, anchor 632, specifies empty attributes, namespace attributes and in-scope
namespaces, and maps object members as children. It does **not explicitly prescribe
namespace-bound `xsi:type`** or explain a `$type` exception to that table.

**B — XSD requires for our chosen XML representation:** namespace-bound QName
resolution of `xsi:type`, an existing nonabstract type, valid derivation from the
original declaration and compliance with its substitution restrictions. W3C XSD
1.0 Structures Second Edition (2004),
<https://www.w3.org/TR/2004/REC-xmlschema-1-20041028/structures.html>:
`#cvc-elt` §3.3.4 clauses 4.2/4.3 and `#cvc-type` §3.4.4;
`#cos-ct-derived-ok` §3.4.6; `#cvc-elt`/`#cos-ct-derived-ok` distinguish
element disallowed substitutions (`block`) from schema derivation prohibition
(`final`). The blocked fixture compiles unchanged before either instance is
assessed. Declared-concrete control validates; derived instance does not. No
abstract declaration, `block`, or authoritative schema was rewritten to obtain
agreement.

**C — research adapter:** unchanged corrected Task 075 mapper implements a
namespace-bound `xsi:type` *assessment-adapter representation*, justified by
carrying exact concrete identity into a standard processor that assesses the
original element declaration and complete content. This is not an explicitly
prescribed CAL reconstruction. All successful mapper information items are
compared against separately specified XML, prefix-independently including exact
expanded concrete QNames. References assess complete documents, not private
subtrees. The `erased-diagnostic` case removes discriminator identity and is
exactly a discriminator-erased diagnostic control, **not** a faithful “literal
CAL mapping.” It fails on the unchanged abstract declaration.

**D — pinned Sleet:** actual unmodified server accepts/rejects as tabulated below.
Agreement with lxml is implementation agreement, not an authoritative amendment.
The pinned parser and validator are not an XSD 1.0 processor.

**Source-anchored clarification question (not submitted externally):** How should
§6.1.2's concrete `$type` identity (anchors 405–409) participate in §6.1.5's
complete-document assessment when Table 6.1-2 (anchor 632) specifies empty
attributes/in-scope namespaces and maps each object member to children? Is an
exception translating `$type` to resolved type-definition information intended,
and how should element `block` restrictions be applied? This remains normative,
separate from deployment custody/authorization questions.

## Executable shared corpus and provenance

`scripts/task076/corpus.py` imports the corrected mapper; no second mapper.
`scripts/task076/qualify.py` requires both hash-verified UCI releases, pinned clean
Sleet checkout, binary and pinned Python validator. It writes the same fixture
paths/digests to a broker campaign. Rust reads those exact UTF-8 bytes and sends
them directly after the OWP PUB prefix—**no parse/reserialize before sending**.
Duplicate raw evidence thus survives. Stable case ID, raw/reference hashes,
declared/asserted identities, bundle manifest hash and complete dependency hashes
are frozen in `tests/fixtures/task076/expected-results.json`.

Sleet source: `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` at
<https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet>.
Task-owned clean clone from cached pinned Cargo source, built **once** with
`cargo build --release --locked -p sleet`, its pinned Rust **1.95.0**.
Binary SHA-256: `4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
Lockfile SHA-256: `744f8610779d47502092d08e2cfe3e59b72deeb36199042b990db36195f090ac`.
No downloaded binary, changed broker source or floating probe dependency.

Validator remains Task 074's exact machine pin: lxml **5.4.0**, libxml2 **2.9.14**;
etree SHA-256 `27d8b7e79e217e6946ad6871c13b6bdfbcf55a1a3f6b3ab590cb892fc3a5deca`,
libxml SHA-256 `8f2643af68a9eba917929046806dabd1848867068d0eee0f6799288a8e886963`.
Authority/provenance is explicitly research-only and UNESTABLISHED. Pins/resolver
protections and all Task 075 outcomes are untouched.

### Actual server path and synchronization

Inspected `sleet/src/main.rs` startup schema/version validation,
`schema/parser.rs::parse` and include traversal, `server.rs::process_init`,
`process_sub`, `process_pub`, and `validator.rs::validate_oms_json` /
`resolve_effective_complex_type_for_value`. PUB authorizes, parses with
`serde_json`, invokes the validator, then routes the original raw JSON. Validator
uses stored-name maps, checks abstractness and `is_derived_from`, flattens
inherited members, checks unknown/missing members and facets. Parser handles
includes but skips imports and does not retain element `block`.

Loopback-only config: task076 service, fixed test UUID ending 0076, allowed topic
`task076`, no message route restriction, appropriate `000.1.0` / `002.5.0` /
`002.6.0` schema version, schema_path equal to the verified bundle root. A server
is reused per bundle; each case uses its own connection. WebSocket explicitly
offers `owp`; INFO establishes INIT, verbose SUB `+OK` establishes subscription;
matched positive PUB requires both `+OK` and inspected MSG before case PUB. The
blocked-element control uses the declared concrete type. Successful case PUB
requires both acknowledgement and correlated MSG, with semantic content equality
including exact `$type` strings. Explicit `-ERR Invalid-Message` establishes
message-validation rejection. Neither timeout nor absent delivery counts as
rejection. Per-case deadlines and a process-group deadline bound execution.
RAII kills/reaps servers; wrapper terminates the entire child process group on
timeout. Raw logs/config are retained in task scratch; volatile ports/timestamps
are excluded from frozen assertions.

Initial development attempts exposed harness bugs (missing `owp` handshake and
incorrect MSG token split); they were stopped, task-owned processes cleaned up,
and corrected. These attempts are not payload evidence. No infrastructure pass
or broker rejection is inferred from those failures.

## Synthetic matrix (20 shared cases)

`V` = research-valid, not authorized; `D` = actual inspected delivery; `R` =
explicit broker message-validation error. Detailed reason strings and payloads
are in frozen JSON. Rejections at different stages are not equivalent assessment.

| Case | Mapper/XSD | Independent XML | Sleet |
|---|---|---|---|
| concrete-implicit | V | valid | D |
| concrete-explicit | V | valid | D |
| known abstract descendant | V | valid | D |
| private complete bundle | V | valid | D |
| private public-only | mapping unresolved | unknown xsi:type | R |
| missing on abstract | mapping missing | abstract invalid | R |
| unknown | mapping unresolved | unknown xsi:type | R |
| abstract | mapping abstract | abstract invalid | R |
| unrelated | mapping unrelated | derivation invalid | R |
| malformed discriminator | QName shape rejection | not representable | R |
| non-string discriminator | QName-token shape rejection | not representable | R |
| invalid inherited Count | number-token shape rejection | datatype invalid | R |
| invalid private Secret | XSD pattern invalid | pattern invalid | R |
| unexpected content | member shape rejection | content invalid | R |
| same-local foreign Known | V, exact private QName | valid | R: InvalidPolymorphicType |
| extension-blocked instance | XSD `CVC_ELT_4_3` | same instance invalid | **D** |
| blocked declared-type control | V | valid | D |
| duplicate `$type` | input policy duplicate-name | no unique reference | **D**, last Known |
| escaped duplicate `$type` | input policy duplicate-name | no unique reference | **D**, last Known |
| discriminator-erased diagnostic | mapping missing | abstract invalid | R |

The same-local foreign case is not rewritten to a bare discriminator to conceal
namespace loss. The bundle's public Known is a separate positive control. XSD
imports the private Known; Sleet skips that import. Reported rejection is at
message validation after successful startup/control, **not** a successfully
loaded private namespace closure. Sleet root loading does not establish import
closure completeness; loader limitation is established independently from source
and the QName probe. Cross-namespace acceptance is not qualified.

Duplicate acceptance is a parser/input-policy difference, not automatically a CAL
conformance defect. Blocked-instance delivery is a measured discrepancy with XSD
assessment under the documented adapter, not proof of universal CAL behavior.

## Both-release complete-message vertical (14 shared cases)

Authoritative roots, unchanged:

| Release | source revision | root SHA-256 | security dependency SHA-256 |
|---|---|---|---|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

Both source releases are from <https://gitlab.com/open-arsenal/uci/standard.git>.
Cached root bytes independently rehashed; full traversal records dependency
digests. Task 074's overlay/field closure is reused without modification; it
declares **synthetic** `Task074SyntheticML`, not an authoritative UCI declaration.
Sleet's include traversal inherits public schema version; both private closures
actually load and pass matched known controls. No fallback to another release.

| Complete DataRecordListManagementRequest / ManagedListBaseType | 2.5 | 2.6 |
|---|---|---|
| ForeignKeyMapML AddRecord | mapper V / XML valid / D | same |
| ForeignKeyMapML DeleteRecord | mapper V / XML valid / D | same |
| Task074SyntheticML AddRecord, complete private closure | mapper V / XML valid / D | same |
| Task074SyntheticML DeleteRecord, complete private closure | mapper V / XML valid / D | same |
| identical private AddRecord, public-only | mapping unresolved / XML unknown type / R | same |
| invalid enclosing RequestState | XSD enumeration / XML enumeration / R | same |
| invalid Task074Code | XSD pattern / XML pattern / R | same |

These are full documents against their original global declarations. The same
complete closure is supplied to all paths; no private-subtree shortcut. Original
Task 075 partial qualification is preserved, not retroactively upgraded.

## Counts, controls and validation

Final matrix: **34 mapper outcomes** (14 valid, 20 rejected), **30 independent
reference XML outcomes** (14 valid, 16 invalid; four unrepresentable/ambiguous:
malformed, non-string and duplicate pair), **34 actual Sleet outcomes** (16
delivered, 18 explicitly rejected). No unsupported or unexecuted shared cases;
four XML representations intentionally unavailable, not claimed executed.
Each Sleet case has a matched delivered positive control. No schema-compilation
failure, timeout, authorization failure or infrastructure-inconclusive result is
counted as payload rejection. Six bundle compilations precede assessment.
The frozen records, not aggregated prose, are the exact accounting authority.

Controls executed locally: Task 075 **143** synthetic regression probes unchanged;
generated closed codec through the same pinned binary (Task 050, one test);
generation-time private-overlay codec regenerated and executed (Task 075 control,
PrivateB round trip and unknown/mismatched types refused). Task 068/072/073 and
QName boundary suites: **17 passed**, zero failed. Driver tests cover missing
inputs, failed child and missing execution marker plus stable IDs and duplicate
bytes. Python compilation, shell syntax, Rust formatting, workspace all-target
check and Clippy warnings-denied are checked locally. No production Rust or
dependencies changed; no unrelated full-schema or backend campaign repeated.

### Reproduce (explicit real command cannot skip missing inputs)

From `/home/zboll/git/ams-gra-codegen-oms-task076`:

```sh
PYTHONPYCACHEPREFIX=/home/zboll/task076-research/pycache \
CARGO_TARGET_DIR=/home/zboll/task076-research/target CARGO_INCREMENTAL=0 \
python3 scripts/task076/qualify.py \
  --scratch /home/zboll/task076-research \
  --sleet-checkout /home/zboll/task076-research/sleet \
  --binary /home/zboll/task076-research/sleet-target/release/sleet \
  --uci /home/zboll/git/.task064-scratch/schemas/uci25/UCI_MessageDefinitions_v2_5_0.xsd \
  --uci /home/zboll/git/.task064-scratch/schemas/uci26/UCI_MessageDefinitions_v2_6_0.xsd
python3 scripts/task076/test_driver.py
```

Binary digests can be build-environment-dependent: frozen evidence is explicitly
for the pinned local binary, not a promise of reproducible binaries on all hosts.
For a different verified build, compare provenance separately, never silently
normalize the binary identity. The Rust broker test is explicitly ignored in
ordinary workspace runs; the qualification driver uses `--ignored --exact` and
requires its execution marker. Hosted Rust coverage does not execute/attest the
opt-in machine-pinned Python/XSD campaign. No CI gate or timeout was changed and
no second broker build or duplicate Deep campaign was added.

## Bounded recommendation / independent review gate

Retain closed-schema generation-time overlays for supported same-OAM deployments;
use this matrix only as bounded interoperability evidence. Before claiming wider
compatibility, seek clarification of Table 6.1-2's discriminator treatment and
independently qualify namespace closure and blocked-substitution enforcement in
the intended peer version. Do not loosen the mapper/codec to match Sleet.
Normative clarification, peer implementation limits and deployment-owner schema
custody/authorization are distinct. None blocks isolated research or grants
production open-world authority.

Production ingress/dispatch/lifecycle, generated models/public codec APIs,
GenerationWorld/readiness, `NotClosedUnderOpenExtensions`, Sleet source/policy,
Task 074 protections and Task 075 behavior/history are unchanged. No opaque
carrier, general mapper, trust registry, validator service or production API.
Non-draft PR, auto-merge disabled, independent review required. Do not merge or
begin Task 077.

## Task 076A — corrective bounded qualification (separate from original run)

Reviewed parent `35ec513e5ee4b8705ec3346caa645e156d165d92`; reviewed driver blob
`932b1dc46251b307342c2f546f9fad4464b0ed31`. PR #76 was open, non-draft,
unmerged, auto-merge disabled; local/remote/PR heads matched. Main remained
`15aa0aa431732de6efbc1eb4412ba94fa9158a33`. No sibling, production, mapper,
validator, Rust test, dependency or CI change.

Both review findings were reproduced **before editing**, under an independent
15-second outer watchdog with finally cleanup and a test subreaper. Injected
Bundle construction reached the old driver PID before broker supervision, with
CPU/address space `(-1,-1)` and core `(0,-1)`. Separately, a same-group descendant
ignored TERM and used independent output handles: the old wrapper returned
`broker process timeout; observation inconclusive` while that descendant remained
live (state S). Both reproductions were synthetic fault injection, not pinned
qualification. Volatile PID-bearing records are retained only in task scratch;
normalized observations are in `tests/fixtures/task076/containment-evidence.json`.

### Corrected phase boundaries

`qualify.py` is now a standard-library-only supervisor. It never imports the
mapper, corpus preparation or native XML modules. It validates invocation and
broker provenance, creates a **fresh invocation-owned directory**, then launches
`validator_worker.py` in an owned session. The worker sets CPU **30 seconds**,
address space **768 MiB**, core **zero**, **before native XML imports**. Its
parent-enforced **60-second wall deadline** includes imports, pin checks, closure
preparation/copy/parsing, all six Bundle compilations, all mapping and reference
assessment, and information-item comparisons. The full 34/30 campaign fits this
unchanged profile, without partitioning or a larger budget.

Zero worker exit **and** a completion marker **and** a size-bounded structured
artifact bound to the current invocation token are required. Expected case order,
complete bundle set, outcomes, normalized corpus and pins are checked before the
broker starts. Missing/malformed/truncated/oversized/partial/stale artifacts fail.
Exceptions/signals/allocation/pin/schema failures are **validator phase failures**,
not invalid-payload outcomes. No final qualification marker precedes completion.
Logs survive failure in the fresh invocation directory. Broker Cargo execution
has its separate **300-second wall deadline**, without XML resource limits.
Thus validator plus broker work has a finite 360-second deadline budget, plus
bounded group cleanup (0.2-second grace and at most two 3-second final waits per
phase) and small bounded artifact/provenance operations.
Fresh broker.json paths cannot reuse an earlier run's success. Broker marker,
exit, artifact size, exact case identities and historical observations are checked.

### Owned process cleanup guarantee and limitations

`supervisor.py` requires Linux `/proc`, `waitid(WNOWAIT)` and
`PR_SET_CHILD_SUBREAPER`; unsupported process-control capabilities fail explicitly.
Workloads are sequential, launched in new sessions/groups. The leader stays
unreaped while group signals are possible, reserving PID/PGID identity and
avoiding signaling a reused group. TERM is sent, a bounded 0.2-second grace period
elapses, and KILL is sent **even when the leader exited or closed its pipes**.
Direct child waits and adopted descendant reaping each have bounded deadlines.
`/proc` start identities distinguish tracked descendants; live and zombie members
are separately visible, and an incomplete cleanup raises a separate failure
alongside the original reason. No waitpid assumption about arbitrary grandchildren.

The same cleanup runs on normal success, nonzero/signal exit, missing marker,
timeout and ordinary parent SIGINT/SIGTERM. Repeated ordinary interruption is
deferred during cleanup. Captured output uses nonblocking bounded-size reads,
not an unbounded communicate/wait. Rust server RAII is unchanged; forced broker
termination does not rely on destructors. No broad process-name kill is used.
The unrelated sentinel remains alive. This guarantee is for the tested **owned
same-session/process-group topology**, not processes that deliberately detach,
SIGKILL of the supervisor, a general sandbox, or operational services.

### Corrective tests and unchanged semantic evidence

**25 driver tests passed**: the original five plus **20 corrective containment/
contract controls**. Fault scenarios have explicit readiness records/pipe
handshakes, caller-side outer deadlines, an independent scenario alarm and finally
cleanup. Limits are observed at the actual worker's injected dependency boundary.
Executable CPU/memory probes use explicit test-only smaller limits after recording
the production defaults. Injected adapters are solely supervision tests and never
used by the real campaign. Artifact contract controls include missing, truncated,
malformed, stale, partial, oversized and successful complete results. Race control
exits during TERM grace; independent-pipe and captured-pipe descendants ignore TERM;
failed/missing-marker/interrupted paths leave no live owned descendants.

Corrected real campaign and deterministic repeat: **34 mapper outcomes (14 valid,
20 rejected), 30 XML assessments (14 valid, 16 invalid), 34 pinned Sleet outcomes
(16 delivered, 18 explicit rejection)**. Every broker case still has its matched
delivered control. All corpus/raw/reference digests, six bundles/dependency
identities, results, payload evidence and broker/validator pins are unchanged;
only driver/helper source identities and explicitly new supervision metadata are
refreshed. The original run is **not retroactively described as bounded**.
Both UCI releases and all measured block/duplicate/QName disagreements are intact.
Disposition stays **POLYMORPHIC_COMPATIBILITY_PARTIALLY_CHARACTERIZED**; the CAL
interpretation remains unresolved.

Task 075's **143** synthetic probes, the **17** guard/QName controls, actual
generated closed codec through the reused pinned binary, and regenerated private
overlay control passed again. Python compilation, relevant shell syntax, Rust
1.95 formatting and diff checks passed. No Rust source/dependency changed, so no
new all-target/Clippy claim is needed for this Python-only correction. Hosted
Rust CI is separately reported at the corrective head; it does not execute these
Python containment or pinned local differential campaigns. Source/build/toolchain
identities above remain unchanged. Scratch: `/home/zboll/task076a-research`;
same qualification CLI, with that scratch path, is the corrected reproduction.

Corrective commit and exact head parity belong to the PR handoff. **Independent
review required**, not merge authorization. No PR merge or Task 077.