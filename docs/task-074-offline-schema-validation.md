# Task 074 — Offline private-schema validation boundary

## 1. Executive qualification disposition

**OFFLINE_SCHEMA_VALIDATION_PARTIALLY_QUALIFIED**.

A pinned mature XSD 1.0 engine validates previously unknown concrete XML values
when supplied a complete synthetic private schema closure. Both real public UCI
2.5 and 2.6 schemas compile unchanged, and complete XML
`DataRecordListManagementRequest` messages with a synthetic private managed list
validate. The public-only closures reject that private type. Invalid inherited
request members and private restriction facets are rejected at document validation,
not accidentally at schema loading.

This is **XML-layer research evidence**, not production adoption, authenticated
deployment approval, OMS JSON validation, interoperability, or permission to publish.
No production dependency, runtime, generator, codec, world policy, readiness or CI
behavior changes. `NotClosedUnderOpenExtensions` remains intact. No opaque carrier
is introduced. The research entry point without an explicit in-process test bypass
fails at `authority-not-established`; that bypass is never approval evidence.

## 2. Starting repository and source identities

- Fetched starting `origin/main`: `96eaa52027bb5d8c2727082848e6f5adfde52edd`.
  Task 072 merged via PR #72 at this exact commit, confirmed by hosted status and
  ancestry. Its runtime ADR and merged Task 068 evidence were read.
- Task 073 PR #73 remained OPEN, unmerged at
  `7fd972e63fbbc175f280a8d930c35831cdd031fe`, inspected read-only. Its documentation
  still describes Task 072 as pending; that historical statement is not current
  repository state. No Task 073 cherry-pick, merge, rebase or worktree edit.
- Local/remote branch and all-state PR searches found no existing Task 074.
  Created `/home/zboll/git/ams-gra-codegen-oms-task074`, branch
  `research/074-offline-schema-validation`, directly from fetched main.
  Original Task 059 checkout was clean at `263df8be24439f368b64cfde56b9b0c70a93e226`;
  preserved it and all sibling worktrees, including prunable entries.
- UCI source: `https://gitlab.com/open-arsenal/uci/standard.git`.
  2.5 revision `093610b7753944059360d3236770ab446d039556`;
  2.6 revision `78eb61b6112c8bffa40820c33124b57787fc5bd9`.
  Rechecked cached repository revisions and root/security digests, then ran the
  final vertical directly against those cached release/extracted inputs.
- Official CAL: OMS repository revision
  `726272bd0390982a759c91a9cf4e13b81c2b510b`,
  `docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`,
  SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
  Rechecked revision/hash and inspected `word/document.xml` directly.
- Sleet source: `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` at
  `https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet`.
  Rechecked cached checkout HEAD; inspected
  `sleet/src/validator.rs::resolve_effective_complex_type_for_value`.
  Implementation evidence, not a normative private-schema authority.
- Historical wider source identities remain in
  `tests/fixtures/open-world/task068-input-manifest.txt`; no new certification of
  its entire schema-wide inventory is claimed.

## 3. Normative evidence versus proposed implementation policy

| Question | Authoritative requirement / scope |
|---|---|
| Concrete identity | CAL §6.1.2 requires `$type` when another complex type was expected, and valid derivation (XSD 1.0 Structures §3.4.6). OAM uses bare names; other namespaces use `{namespace}local`. XML `xsi:type` is a QName, not that JSON string syntax. |
| Full validity | CAL §6.1.5 says OMS JSON is valid iff its associated information items have valid XML Schema assessment. This includes inherited constraints and the full message, not merely subtype existence. |
| Mapping | CAL §6.1.5.1–3 prescribes document and member information items; §6.1.2–4 prescribes object/array/simple-value representation. A normative mapping exists, but this task implements and qualifies **none** of it. |
| Private extensions | UCI's abstract ManagedListBaseType documents ML extensions; merged ADR-0004/Task 027 records explicitly program-defined private extension points. The public file universe does not close the concrete-type universe. |
| Composition | XSD includes/imports and namespace/derivation rules determine assessment. Repository additive overlays are build-time inputs, same top-level namespace, duplicates rejected; supplying them does not imply Open closure. |
| Authorization | None of these validity rules authenticates deployment approval, grants publish authority, or establishes peer support for private types. Security markings are not an authorization implementation. |

Pinned Sleet looks up the asserted type in its supplied schema, rejects abstract or
unrelated candidates, and otherwise assesses known structure. That is not proof
of arbitrary private-schema support, complete XSD conformance, or approved custody.

The manifest, deterministic DFS closure, explicit QName allow-list, UTF-8-only
input, DTD/schema-hint rejection, finite budgets and owner gate below are **proposed
deployment policy**, not additional UCI/CAL requirements. CAL does not select a
signer, trust store, approver, manifest format or publication policy for us.

## 4. Validator alternatives and selection

| Alternative | Assessment |
|---|---|
| libxml2 XSD through lxml | Selected for bounded local research: mature native XSD 1.0 implementation, explicit resolver, structured diagnostics, installed offline, exact versions/binary hashes verified before every run. |
| xmllint | Same installed engine, but less convenient resolver isolation and structured test-stage checks. Not a second independent validator. |
| Xerces-C / Xerces-J | Mature XSD alternatives; would require separately pinned binaries, resolver configuration and qualification. Not installed/qualified here. |
| Saxon XSD validation | Potential XSD 1.1 option with edition/licensing/distribution requirements; no verified offline toolchain here. Not substituted. |
| Python xmlschema / custom validator | No verified pinned offline installation of the former; custom validation is not appropriate. Neither used. |
| Repository frontend / Sleet | Useful structure and codec controls, not substituted for a mature full XSD validator. |

Selected exact profile: Python **3.13.5**, lxml **5.4.0** (Debian `5.4.0-1`),
runtime/compiled libxml2 **2.9.14** (Debian package
`2.12.7+dfsg+really2.9.14-2.1+deb13u3`). The misleading package prefix is not the
engine version. Pins in `scripts/task074/bundle.py`:

| Executed component | SHA-256 |
|---|---|
| `etree.cpython-313-x86_64-linux-gnu.so` | `27d8b7e79e217e6946ad6871c13b6bdfbcf55a1a3f6b3ab590cb892fc3a5deca` |
| `libxml2.so.2.9.14` | `8f2643af68a9eba917929046806dabd1848867068d0eee0f6799288a8e886963` |

No download/install/floating dependency is used. A different build stops with
`toolchain-pin`, not an unverified fallback. These are local binary identity pins,
not a portable supply-chain lock, signed distribution or complete Python/OS TCB
attestation. Production must review security maintenance/CVEs and reproducible
distribution; this older engine is not endorsed for deployment by this task.

XSD 1.1 assertions/alternatives are outside this profile. The tested XSD 1.0
constructs include includes/imports, namespace-distinct same-name types, abstract
complex types, extension derivation, `xsi:type`, positive-integer and
pattern/length restriction facets, and finite recursive values. Both complete
public UCI schema closures compile. That is not a claim of exhaustive W3C-suite
conformance. Chameleon includes, redefine, catalogs, locationless imports and
substitution-group policy are not qualified; the wrapper refuses redefine and
requires explicit schema locations/exact namespace equality.

## 5. Proposed approved-bundle manifest and authority gate

`tests/fixtures/task074/manifest.json` is a deterministic executable example, with:

- bundle ID and version;
- root filename, ordered source/dependency closure, exact SHA-256 per file and
  expected target namespace;
- explicit expanded concrete QName allow-list;
- associated public UCI release (or `synthetic`);
- exact validator toolchain identities;
- approval provenance fields: owner, evidence, scope and status.

Real-run manifests, including composed root/overlay hashes, are frozen in
`tests/fixtures/task074/expected-results.json`. Order is first-visit DFS in schema
directive order, root first; extra/missing/reordered files fail. Repeated references
to the same file are deduplicated, not independent declarations. Separate immutable
release-specific bundles avoid mixing public UCI revisions; manifests may not
select a dependency by basename search or ambient namespace registry.

Proposed model: an explicitly deployment-owner-approved **immutable offline
snapshot** containing all schema bytes and a manifest. The deployment must supply
an authenticated binding of that snapshot to release, service/operation scope,
validity period and revocation/update rules. **No such authority was supplied.**
Fixture provenance deliberately says `UNESTABLISHED`, owner/evidence null. No CA,
signer, certificate chain, trust store or approval ceremony is invented. The
non-test entry point always refuses until a later reviewed implementation supplies
that authority gate; a non-null owner string would not suffice.

## 6. Integrity is not authenticity

Digest verification establishes exact byte identity with the supplied manifest.
An attacker able to replace both bytes and manifest can make hashes agree.
Authenticated owner approval is separate and absent. Likewise approval to assess
a schema is not approval to publish a payload under it. The QName policy rejection
of a schema-defined `Unapproved` type demonstrates validity versus policy, but its
fixture allow-list is test policy, not an authenticated deployment grant.

## 7. Dependency closure and snapshot resolution

The wrapper rejects absolute paths, parent traversal, URI/drive syntax, backslashes,
symlinks and resolved paths outside the bundle. It verifies hashes and namespaces
before compilation, checks every include/import edge and exact ordered closure,
and passes immutable verified bytes to a resolver under a synthetic
`file:///task074-approved/` base. All other resolver requests fail; there is no
ambient filesystem/network fallback. XSD compilation, not custom wrapper logic,
checks duplicate definitions and derivation validity. The wrapper does **not**
implement an XSD validator.

Test evidence: manifest omission fails at closure; absent file at missing-file;
modified content at digest; HTTP/file/traversal includes before resolution;
duplicate declarations at compiled schema diagnostics. Documents are tested only
after successful schema compilation. Mutating files after compilation does not
change the snapshot's validation result. Path inspection/read has a race on a
hostile writable filesystem; hash verification prevents acceptance of substituted
bytes but not all unintended reads. Production needs read-only immutable storage
or race-free open primitives and authenticated manifest delivery. This is retained
as a blocker, not hidden behind canonicalization.

## 8. Synthetic qualification matrix

Every negative asserts an exact wrapper stage or first libxml2 diagnostic.
All payload-invalidity cases use a successfully compiled complete schema.
`expected-results.json` records the exact executable results.

| Case | Expected result, observed |
|---|---|
| Known concrete | Valid XML |
| Approved private descendant | Valid XML with complete private import/include closure |
| Unknown / schema-defined unapproved | Unknown type `CVC_ELT_4_2` / policy `unapproved-type` |
| Missing private dependency / omitted manifest member | `missing-file` / `closure` |
| Incorrect namespace | Unknown expanded QName, `CVC_ELT_4_2` |
| Same local `Known` in public/private namespaces | Both valid in their proper namespaces; no local-name identity shortcut |
| Malformed / incompatible / abstract xsi:type | QName datatype / derivation / abstract-type diagnostic respectively |
| Invalid inherited Count | Positive-integer datatype diagnostic |
| Invalid private Secret | Restriction pattern diagnostic |
| Duplicate declaration | `SCHEMAP_REDEFINED_TYPE` at schema compilation |
| Modified bytes, unchanged manifest | `digest` |
| Traversal, external file/HTTP include, symlink | `path` / `symlink`; no schema loading fallback |
| Wrong manifest namespace/order/version digest | `namespace` / `closure-order` / `digest` |
| Schema-location override | `schema-location` before XSD assessment |
| DTD/external entity/entity expansion | `dtd` before assessment |
| Oversize / depth / recursive depth | `size` / `depth` / `depth` |
| Malformed UTF-8/XML, alternate encoding | `utf8` / `xml` / `encoding` |
| Finite recursive value | Valid XML |
| Direct non-bundle resolver request | `resolution`, HTTP and file |
| Post-compile disk mutation | Still validates snapshot |
| CPU / memory / timeout hostile worker | SIGKILL / MemoryError / killed and reaped |

Synthetic output repeated byte-identically. These fixtures are research-only;
their approval bypass cannot be used to release a value to typed handlers.
Final output contains 54 evidence rows: 51 outcome assertions, one resolver
transcript and two real-release closure manifests. Twelve outcome assertions
cover the real vertical (six per release); 39 cover synthetic/control cases.

## 9. Real UCI vertical

Both authoritative releases establish:

```text
DataRecordListManagementRequest : DataRecordListManagementRequestMT
  MessageData : DataRecordListManagementRequestMDT (extends RequestBaseType)
    AddRecord / DeleteRecord : ManagedListBaseType (abstract, optional 0..1)
      ForeignKeyMapML (concrete extension)
```

`ManagedListBaseType` has **no inherited value members**. Its annotation recommends
ML extensions with unbounded DRLE list entries. ForeignKeyMapML has optional
unbounded ForeignKeyPairListElement. Our `Task074SyntheticML` is intentionally a
minimal constraint probe, not a proposed operational managed-list definition or
standard UCI type. It adds required `Task074Code` through a separately included
synthetic restricted string definition. Additive same-namespace composition is
performed by a generated wrapper include, leaving authoritative bytes unchanged.

The known and synthetic complete **XML messages** assess against all public types,
including security/header/request members. The inherited negative changes
RequestBaseType.RequestState in the enclosing request; it does not invent an
inherited field on the empty managed-list base. The private negative changes the
extension Code. Both public-only private-type negatives fail with unresolved
`xsi:type`, and both incomplete private closures fail the explicit closure gate.
DeleteRecord's identical abstract position is structurally asserted; executable
payload qualification uses AddRecord, not a claimed separate DeleteRecord campaign.

| Release | Root SHA-256 | Security include SHA-256 |
|---|---|---|
| 2.5 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

Each executed closure is wrapper → public root → security include → private-fields.
UCI 2.6's separate Versioning schema is not an include/import of this root and is
not silently included as a dependency. Complete closure is defined by reachable
schema inputs, not every file in a release archive.

## 10. OMS JSON versus XML validation boundary

CAL does specify information-item mapping, contrary to treating arbitrary
JSON-to-XML conversion as sufficient. This task has not implemented or proven that
mapping across member QName expansion, `$type`, optional/repeated array shape,
particle ordering/choices, numeric lexical/value space, binary/temporal forms,
namespace attributes, duplicate JSON keys and unknown members. lxml receives
hand-authored XML, not OMS JSON. No unknown JSON-to-XML round-trip or full OMS JSON
validity claim is made. Existing checked generated JSON codec regressions retain
their own closed-schema boundary; they do not bridge arbitrary private JSON.

## 11. Security and resource qualification

| Control | Enforcer and executable evidence | Remaining boundary |
|---|---|---|
| Network schema retrieval | Wrapper safe paths, snapshot resolver, `no_network=True`; HTTP include and direct resolver denial | `unshare -n true` denied by host. No kernel network isolation/seccomp established; resolver transcript is not whole-process syscall tracing. |
| DTD/entities | Wrapper UTF-8/declaration gate, parser `load_dtd=False`, `resolve_entities=False`; DTD/XXE/expansion probes rejected | Not a general-purpose XML acceptance profile. |
| Schema hints | Wrapper rejects both schema-location attributes at all element depths | Does not accept hints even when benign. |
| Symlinks/path escape | Wrapper rejects symlink paths and lexical/canonical escapes; tests pass | Hostile filesystem race/read-only custody unresolved. |
| Conflicts/namespaces/versions | Hash/namespace/closure policy plus libxml2 duplicate diagnostics | No live update/revocation service or authenticated release binding. |
| Document bytes | Wrapper ≤64 KiB before parsing; oversize test | Schema snapshot individual file limit 16 MiB; no independently qualified aggregate admission budget. |
| Depth | Wrapper ≤64 after bounded non-huge parse; recursive/depth rejection | Post-parse depth check, not a streaming parser; OS worker limits contain parsing. |
| CPU / memory | OS RLIMIT_CPU=30s, RLIMIT_AS=768 MiB, core=0 on qualification worker | Hostile worker tests prove mechanics at 1 CPU second/256 MiB, not universal validator complexity or deployment concurrency safety. |
| Wall deadline | Parent subprocess timeout=60s, kill/reap on expiry; sleeping-worker probe at 0.1s | No production queue/concurrency/cancellation/descendant-process design. Worker itself spawns only resource-control probes. |
| Malformed data | UTF-8 wrapper and strict libxml2 parser; malformed encoding/XML tests | UTF-8-only profile, not arbitrary XML encodings. |

The full final qualification succeeds inside the stated worker limits. Resource
mechanics are executable evidence, but adversarial XSD complexity, aggregate schema
size, process-tree limits, schema compile cache pressure, throughput and deployment
concurrency are not fully qualified. Do not interpret passing samples as an
established general safe-production resource boundary.

## 12. Ada, Rust and C++ implications

- Rust: an external bounded validator worker could return XML-only assessment
  evidence, but no JSON conversion, trusted evidence ownership, typed conversion
  or public runtime seam is added. Avoid weakening checked generated codecs.
- C++: libxml2 native integration would require independent RAII/error/allocator,
  thread/concurrency and resource-isolation qualification; subprocess evidence is
  not an in-process safety contract.
- Ada/SPARK: Python/native validation is outside SPARK proof. A reviewed external
  process boundary and immutable bounded result protocol would be needed; no
  foreign binding, proof or open carrier is established.

All three existing additive private-overlay closed-sum workflows remain distinct
from unknown runtime values. No backend is granted OpenExtensions readiness.

## 13. Reproduction, validation and remaining gates

Run from `/home/zboll/git/ams-gra-codegen-oms-task074` on the exact pinned local
profile. Source acquisition is separate from offline qualification:

```sh
# Optional acquisition into new scratch paths (network only for acquisition):
bash scripts/fetch-pinned-uci-2.5.sh /home/zboll/task074-research/uci25
bash scripts/fetch-pinned-uci-2.6.sh /home/zboll/task074-research/uci26
# Synthetic offline run:
python3 scripts/task074/qualify.py
# Actual final real-source run (cached verified release repository/extraction):
python3 scripts/task074/qualify.py \
  --uci /tmp/task058-corrective/uci25/03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd \
  --uci /tmp/task058-corrective/uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
# JSON output must equal tests/fixtures/task074/expected-results.json.
```

The manifest generator is only a fixture helper; never treat locally computed
digests as owner approval. Output freezes exact source hashes, configuration,
validator versions and diagnostic results, without environment paths/timestamps.

Local Rust 1.95 commands (task-specific target directory, CARGO_INCREMENTAL=0):

```sh
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --workspace --all-targets
cargo +1.95.0 clippy --workspace --all-targets -- -D warnings
cargo +1.95.0 test -p ams-gra-oms-codegen-core -p ams-gra-oms-xsd-frontend --lib --tests
for filter in task028 task029; do
  cargo +1.95.0 test -p ams-gra-oms-backend-ada -p ams-gra-oms-backend-rust \
    -p ams-gra-oms-backend-cpp --lib "$filter"
done
cargo +1.95.0 test -p ams-gra-oms-runtime-rust-facade-tests \
  --test task068_open_contract --test generated_codec_qname --test task072_wire_boundary
cargo +1.95.0 test -p ams-gra-codegen-oms --test service_codec \
  --test service_generate --test task072_runtime_contract
python3 scripts/check-task068-fixtures.py
git diff --check
```

No multi-hour Deep campaign is required or claimed. Hosted CI on the pushed head
is a separate publication gate; this platform-specific opt-in Python pin is not
silently added to hosted CI or skipped as if qualified there.

The initial broader backend suite was stopped after an unrelated Task 060 Ada
compiler corpus ran for several minutes; no full-suite success is claimed for
that interrupted run. The focused commands above replace it and retain the
relevant Task 028/029 compiler/overlay and abstract-value/codec controls.
Final focused Rust result: **685 passed, zero failed, one existing opt-in real-UCI
Task 072 test ignored**. Task 068 fixture accounting and all six frozen readiness
sets passed. Workspace fmt/check/clippy and diff checks passed. The Task 074 real
XML probes are separate, executed evidence, not inferred from the ignored test.

Remaining production gates: authenticated deployment-owner approval/delivery;
schema/service operation authorization and peer acceptance; reviewed complete
CAL JSON mapping; maintained portable validator distribution/TCB provenance;
race-free read-only schema custody; aggregate/adversarial/concurrent resource and
OS network containment; transactional update/revocation and immutable assessment
ownership; language integration/proof obligations. None is waived by XML validity.

## 14. Proposed narrow next task and review gate

Obtain deployment/security-owner decisions for authenticated offline bundle
custody, validation scope and operation authorization, then qualify a bounded
CAL §6.1.5 JSON information-item mapping for this one vertical with explicit
loss/duplicate/namespace/facet tests. Keep production runtime and Open guard
unchanged until those reviewed gates have executable evidence. This is a proposal,
not authorization to begin Task 075. Commit/push non-draft PR, auto-merge disabled,
verify exact head and hosted status, stop for review; do not merge Task 074.