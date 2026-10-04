# Task 062 — exact pinned IPv6 constrained-String profile

## Isolation and immutable BEFORE identity

Task 062 began in the clean linked worktree
`/home/zboll/git/ams-gra-codegen-oms-task062`, on
`feature/062-ipv6-address-profile`. No previous Task 062 worktree, branch or PR
existed. Origin was fetched before creation. PR #62 was OPEN, non-draft,
unmerged, with auto-merge disabled, head
`add348ce867daf4253a88510a65a48a3ed23f218` and base
`main@0800e97c71d8bf3edcef40ef1dc4ef5200b76deb`.

**Selected parent and immutable BEFORE benchmark:**
`add348ce867daf4253a88510a65a48a3ed23f218`.
No parent advancement required inspection. Later parent advancement must not
change this benchmark. At publication neither task had merge authorization.
Task 061 subsequently merged independently; see the corrective integration
section below. Task 062 remains unauthorized to merge. Task 061's worktree,
branch, PR, CI and collectors were not modified by this task.

Task-specific artifacts are outside the worktree at
`/home/zboll/git/ams-gra-codegen-oms-task062-artifacts/`: `target/` is
`CARGO_TARGET_DIR`, `tmp/` is `TMPDIR`, and separate `probes/`, `schemas/`,
`logs/`, `manifests/` and `scratch/` directories isolate all other work.
No shared target cleaning or shared Git configuration changes are permitted.

## Evidence gate status

**FROZEN before production changes. At freeze, production capability was unchanged.**
The evidence-only target
`uci_ipv6_address::task062_pinned_ipv6_inventory_coverage_and_impact`
loads both pinned roots, compares complete normalized effective constraints
without declaration-name classification, inventories authored/effective
members, and measures selected and generated-support reach in both worlds.
All 12 coverage cells and complete relevant service verdicts finished in
268.80 seconds: exactly one intended test, zero failures, command and wrapper
exit 0, all four release/world completion markers. The normalized evidence
is frozen in `tests/fixtures/string/task062-before.tsv` (SHA-256
`b68e5165c854403cd3494a9fb96ef3294131209caae76cd31a37f8ef60c8f610`).
Raw diagnostics and durable exits remain in `logs/before-inventory.*`.
Missing completion markers are not a pass.

One initial wrapper launch failed before test execution because `/usr/bin/time`
is not installed. Its exit 127 and diagnostics were retained. The corrected
wrapper uses Bash's `time`; no duplicate campaign or watcher was started.

## Pinned authority and raw/normalized cross-check

Public schema source: `https://gitlab.com/open-arsenal/uci/standard.git`.

| Release | Pinned tag commit | Root SHA-256 | Declaration line |
|---|---|---|---:|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | 124930 |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | 125396 |

Existing schema caches were read only, their checkout HEADs and root digests
verified, then their schema directories copied into Task-062-specific paths.
The copies were independently rehashed. Neither extraction nor checkout was
performed on reused roots. Schema identity records are in `schemas/identities.json`.

Both raw roots contain one declaration named `IPv6_AddressType`, directly
restricted from `xs:string` (restriction lines 124934 / 125400), with exactly
the following pattern text:

```text
((:|[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:|[0-9a-fA-F]{0,4}))|(((25[0-5]|2[0-4][0-9]|[01]?[0-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9]?[0-9])))
```

Normalized profile: `PrimitiveKind::String`; one restriction-level pattern
group; one alternative; `PatternDialect::XmlSchema`; `minLength=2`;
`maxLength=45`; no `length`, numeric bounds, or explicit whitespace facet.
The effective whitespace policy is intrinsic `xs:string` **preserve**.
These complete facets, not a name, are the admission boundary.

## Language-neutral factored semantic derivation (pre-rendering)

Write `H` for 0..4 ASCII hexadecimal characters, `C` for a literal colon,
`D` for the exact decimal octet alternatives, and `V` for `D.D.D.D`.
The complete expression is:

```text
(C | H) C (H C){0,5} ((H C)? (C | H) | V)
D = 25[0-5] | 2[0-4][0-9] | [01]?[0-9]?[0-9]
```

There is no nonempty-hex requirement in this expression. The length facets
independently reject the one-character `:` even though the pattern admits it.
No RFC group-count, compression-count or numeric-address rule may be added.

A bounded deterministic scanner can examine the two possible prefix endpoints
(`H C`, and `C C`), then at each of at most six repetition counts test the
factored suffix. Each `H C` endpoint is determined by the first non-hex character;
there is no cartesian expansion of hex groups or decimal alternatives. The
hex suffix tests `H`, `C`, `H C H`, or `H C C`. The embedded-IPv4 suffix tests
four nonempty octets separated by exactly three dots. Each octet is 1..3
ASCII digits whose value is at most 255: exhaustive bounded lexical evidence
must establish equivalence with the authored alternatives, including leading
zero forms. Matcher helpers remain private/nested in generated carriers.

Only bounded offsets are advanced. An Ada input character index is calculated
only after proving `offset < Text'Length`; endpoint offsets are not turned
into one-past-end absolute indices. For slices ending at `Positive'Last`,
use `Text'First + offset`, never an intermediate addition of a one-past-end
endpoint. Null strings fail the facets before any indexing.

## Why a host IPv6 parser is not the oracle

This is the exact pinned XML Schema lexical language, **not RFC IPv6 parsing**.
No production validator or expected-result oracle may use `Ipv6Addr`,
`inet_pton`, networking libraries or a general regex engine. The constructor
must store accepted input unchanged: no canonicalization or 128-bit conversion.

The expression algebra independently derives these surprising cases:

| Spelling | Expected | Expression reason |
|---|---|---|
| `1:2`, `1:`, `:1` | accept | `H C H`, including empty terminal/initial H |
| `:::` | accept | empty H pieces or the `C C` prefix |
| `:::::::::` | accept | `C C`, five empty `H C` repeats, suffix `H C C` |
| `::::::::::` | reject | exceeds maximum structural colon count |
| `1::2::3` | accept | multiple empty H pieces are permitted |
| `:001.009.099.199` | accept | empty-H prefix and exact octet alternatives |
| `1:255.255.255.255` | accept | no RFC group-count rule is imposed |
| `:` | reject | independent `minLength=2` |
| `1:2:3:4:5:6:7:8:` | reject | suffix cannot contain those extra nonempty groups |
| `[::1]`, `fe80::1%eth0` | reject | brackets/percent are outside all pieces |

An evidence-only libxml2 2.9.14 diagnostic corroborated these rows but also
incorrectly accepted `::12345`. W3C XML Schema 1.0, Appendix F explicitly says
expressions are "implicitly anchor[ed] ... at the head and tail" and asserts
"only strings in L(R) are valid literals". Simple diagnostic control patterns
`[0-9]{1,4}` and `A.*Z` reject overlong/substr-only matches correctly; the
complex IPv6 expression exposes an engine discrepancy. This diagnostic is
not an acceptance oracle. The semantic model retains bounded, full-literal
matching, and `::12345` is rejected. Authority copies are in
`scratch/xsd10.raw` / `scratch/xsd10.txt`.

This diagnostic adds no repository or generated-code dependency. The final
shared compiler corpus must derive expectations independently from these
semantic partitions, not invoke the production matcher for expected values.

## Remaining required evidence

### Frozen BEFORE inventory and reach

Both releases have **one exact-profile declaration**, **three authored member
references**, **six effective references** (three inherited). The authored
references are `IPv6_AddressRoutingType.IPv6_Address` (required),
`IPv6_SettingsType.StaticIP_Address` (required), and
`IPv6_SettingsType.PreferredDNS_Server` (optional). Inherited required
references occur in `IPv6_AddressRoutingPortType`, `IPv6_ConnectionType`, and
`IPv6_EndpointType`. There are no directly authored repeated/Choice IPv6
members, aliases, lists, or additional restriction descendants. Ancestor
containers still introduce repeated and Choice composition in message closure.

Both releases' closed-world reaching services are **11**:
`IO_PortCommand`, `IO_PortStatus`, `PO_Activity`, `PO_Capability`,
`PO_CapabilityStatus`, `PO_Command`, `PO_SettingsCommand`,
`ProductOrFileDisseminationDestination`, `RDMA_Initialize`,
`RDMA_InitializeSetup`, `Task`. Ten select IPv6 semantically; only
`ProductOrFileDisseminationDestination` reaches it exclusively as generated
support. All are NOT READY in all three backends. IPv6 is the selected first
blocker for the ten; for the support-only service selected closure is already
renderable, but IPv6 support prevents READY. This reproduces the 11-member
Time partition independently rather than assuming its completeness.

Open world: four successfully projected reaching services (`IO_PortCommand`,
`IO_PortStatus`, `RDMA_Initialize`, `RDMA_InitializeSetup`), all IPv6-blocked.
Projection also fails on semantically IPv6-reaching `Response` and `Task` in
2.5, and on `PO_Activity`, `PO_Capability`, `PO_CapabilityStatus`, `PO_Command`,
`PO_SettingsCommand`, `Response`, `Task` in 2.6. These are abstract-value
topology failures, not absent IPv6 reach. Support reach is unavailable where
projection fails. Full projection failure distribution is frozen in the TSV.

The smallest projected blocked service is `RDMA_InitializeSetup`, **50 selected
+ 0 support** declarations in both releases/worlds. `Task` and all PO services
also have the three deferred Unicode declarations as support blockers:
`NITF_DateAndTimeType`, `NITF_DateType`, `NITF_MSTGTA_TargetLocationType`.
Removing IPv6 must not imply those services become READY.

| Release | World | Backend | Kinds | Full declarations | Field types / occurrences | Message closures |
|---|---|---|---:|---:|---:|---:|
|2.5|closed|Ada|5553/5557|5531/5557|13160/13160|632/722|
|2.5|closed|Rust/C++|5553/5557|5532/5557|13160/13160|664/722|
|2.5|open|Ada/Rust/C++|5553/5557|5444/5557|13160/13160|566/722|
|2.6|closed|Ada|5566/5570|5545/5570|13198/13198|637/725|
|2.6|closed|Rust/C++|5566/5570|5546/5570|13198/13198|669/725|
|2.6|open|Ada/Rust/C++|5566/5570|5458/5570|13198/13198|570/725|

## Semantic/compiler and lifecycle evidence

The factored scanner matches **26,946 independently expected cases**, using a
separate separator-count oracle, authored boundary rows, every 1..3-digit
decimal spelling in each of four octet positions, and systematic hex width,
case and empty/nonempty partitions. Expected values never call the production
matcher. The scanner mirrors `(C|H)C`, 0..5 `HC` repeats and the factored suffix.
No complete cartesian branch is emitted. Exactly 45 characters is accepted for
`ffff:ffff:ffff:ffff:ffff:ffff:255.255.255.255`.

| Synthetic `Address` carrier | Bytes | Lines |
|---|---:|---:|
| Rust | 2423 | 79 |
| C++ | 2871 | 72 |
| Ada body (visible/private carrier uses existing lifecycle) | 4077 | 95 |

Production generation of a synthetic single `AddressType` schema (including
namespace/unit boilerplate, imports, and Ada public/private spec + body) measures:
**Ada 6,243 bytes / 150 lines; Rust 2,811 / 93; C++ 3,621 / 103**. A separate
production size guard requires fewer than 16,000 bytes / 350 lines, and schema
declarations using the private scanner helper names pass shared name preflight.

Guards require fewer than 12,000 bytes and 250 lines per scanner/carrier.
Rust `rustc --edition=2024 -Dwarnings` and C++17
`-Wall -Wextra -Werror -pedantic` compile and run the entire corpus. Ada compiles
under `-gnat2022 -gnatwe -O0 -gnato`, under default, assertions-enabled and
Assertion_Policy Ignore; every case is checked at 1-based, shifted and
Positive'Last-ending positions (including high null strings). Constructor
rejections must have the exact lexical diagnostic, not incidental overflow.

Rust private String/new/as_str/no Default; C++ private String/create/value,
no public default and copy-only validated lifecycle; Ada private Text/Create/
Value and rejecting component default all retain input exactly. Probes exercise
copies, assignment and repeated storage. Each backend receives one planted
wrong expectation, fails execution, is restored, and passes again. Passing
compiler sources and corpus remain under Task-062 TMPDIR for audit.

## Capability and Rust OMS JSON

`StringProfile::Ipv6Address` is offered **after** all prior profiles. Complete
ConstraintSet equality includes the exact pattern group, one alternative,
length facets, intrinsic whitespace and absence of other facets. Neighbor
tests reject min/max changes, altered expressions, extra alternatives/groups,
explicit preserve/replace/collapse, `length`, and unconstrained String admission.
`PatternDialect` currently has only `XmlSchema`; a wrong dialect is not
representable in normalized IR. Non-String primitives do not classify.
A renamed `AddressType` fixture admits identically; a declaration actually
named `IPv6_AddressType` with an added alternative blocks all backends.
Selected and support-only fail-closed readiness are tested separately.

OMS authority was independently read from `open-arsenal/oms` commit
`726272bd0390982a759c91a9cf4e13b81c2b510b`: OMSC-SPC-013 Rev B SHA-256
`b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
Section 6.1.4 case 5 says **"Otherwise string"**; xs:string meets none of cases
1..4. Section 6.1.5.4 maps string characters without quotation marks into the
lexical value. Read-only source inspection and extracted text are recorded in
`scratch/oms-mapping.txt`. Existing checked-String codec emission encodes
`as_str()` and decodes only `dec_str` followed by `new`. Codec readiness now
also consults the shared String classifier: unsupported constraints cannot
be reported codec-ready independently of the model gate.

Compiled codec tests preserve mixed hex case, repeated compression-like text,
and zero-prefixed embedded IPv4 in required/optional/repeated positions. Invalid
lexicals and non-string JSON reject. Mock OWP receives/publishes valid unchanged
text and reports invalid lexical/non-string inputs as codec events, with no
typed-handler delivery. Local exact Fast gates pass.

## Measured AFTER and real impact

The one loaded-schema campaign completed **one exact test, exit 0, 248.20s**,
with all four release/world markers. All cells are frozen in
`tests/fixtures/string/task062-after.tsv`; BEFORE remains unchanged.
The final assertion-bearing campaign (including exact member counts and frozen
AFTER row checks) also passed **one exact test, exit 0, 259.55s**, all four
markers. Full-schema unique gain tuples are frozen separately in
`task062-full-closure-gains.tsv` (**82 tuples**), avoiding confusion with the
54 projected-service gains.

| Release | World | Backend | Kinds | Full declarations | Field types / occurrences | Message closures | Closure gain |
|---|---|---|---:|---:|---:|---:|---:|
|2.5|closed|Ada|5554/5557|5532/5557|13160/13160|641/722|9|
|2.5|closed|Rust|5554/5557|5533/5557|13160/13160|674/722|10|
|2.5|closed|C++|5554/5557|5533/5557|13160/13160|674/722|10|
|2.5|open|Ada|5554/5557|5445/5557|13160/13160|570/722|4|
|2.5|open|Rust|5554/5557|5445/5557|13160/13160|570/722|4|
|2.5|open|C++|5554/5557|5445/5557|13160/13160|570/722|4|
|2.6|closed|Ada|5567/5570|5546/5570|13198/13198|646/725|9|
|2.6|closed|Rust|5567/5570|5547/5570|13198/13198|679/725|10|
|2.6|closed|C++|5567/5570|5547/5570|13198/13198|679/725|10|
|2.6|open|Ada|5567/5570|5459/5570|13198/13198|574/725|4|
|2.6|open|Rust|5567/5570|5459/5570|13198/13198|574/725|4|
|2.6|open|C++|5567/5570|5459/5570|13198/13198|574/725|4|

Every cell gains **one declaration kind and one fully renderable declaration**;
field types/occurrences and all denominators are unchanged.

Both releases/all backends newly READY closed: **IO_PortCommand, IO_PortStatus,
RDMA_Initialize, RDMA_InitializeSetup, ProductOrFileDisseminationDestination**.
The first four also newly READY open. No reaching service was already READY.
Exact unique frozen sets: **54 (release, world, backend, message)**, **30
(release, backend, message)** in `task062-new-ready.tsv`. All 54 are independently
confirmed by production `service-check`, exit 0 and final execution marker;
`logs/service-impact.log` records every tuple. Six closed-world services remain
Unicode-support-blocked (`Task` and the five PO services). Open-world topology
failures remain unchanged. Full-schema closure gains are not projected gains.

The minimum READY vertical by selected+support count and name tie-break is
**RDMA_InitializeSetup (50+0)**. Both releases pass `service-check`,
`service-generate`, strict Ada model/body/API compilation, Rust API+codec build
and lexical round trip/rejection, and C++ model/API compilation. Rust real
fixtures include repeated and optional network endpoint Choices holding mixed
hex case, repeated empty groups and embedded IPv4 spelling. Six compiler markers,
two real-codec markers and exit 0 are retained in `logs/vertical.log` / `.exit`.

## Ada naming and historical fixture context

Rust/C++ closed-full-schema-minus-Ada grows from **32 to 33** per release. The
one added message is **Task**; its unsafe declaration is **QueryPET**, owing to
the existing QueryType / QueryType companion collision at QueryType_Kind.
This is not fixed. Projected Task has no Ada backend name blocker but is NOT
READY because all three deferred Unicode support declarations remain.
The inherited naming attribution test is extended, and the exact 33-message
sets are frozen in `task062-ada-full-schema-gap.tsv`. Current expectations use
new Task062 TSVs; historical Task060/061 figures and fixture files are untouched.
The exact two-release naming attribution test completed one test, exit 0,
and reports `QueryPET` as Task's sole unsafe closure declaration in each release.
The inherited projected-message impact/naming regression completed one test,
exit 0, **386.26s**, both release markers; all earlier admitted alternation
profiles keep their admission decisions.

All **188 repository XSD fixtures × 3 backends × 2 worlds = 1,128 cells** were
freshly generated with immutable-parent and child CLIs on identical child
fixture inputs: **575 identical successes, 547 shared failures, 6 new successes,
0 changed successes, 0 regressions**. The new cells are exactly
`tests/fixtures/service-generate/codec-ipv6.xsd` in every backend/world. No
unrelated successful output changes. Parent CLI was freshly built from a Git
archive of the immutable benchmark in a separate target; fixture digests and
results are retained in `scratch/fixture-results.json`. These are Task062's
figures, not replacements for Task061 historical evidence.

## Validation and delivery status

Local synthetic gates, pinned BEFORE/AFTER, 54 CLI confirmations, real RDMA
vertical and fixture identity have passed. All inherited pinned regression
gates passed: naming attribution, OOB readiness/coverage, projected impact,
OrderOfBattle + SMTI_SettingsCommand both-release compiler/codecs (**444.69s**),
and DLZ both-release compiler/codecs (**119.19s**). The Ada high-bound regression,
TimeZulu, structured ASCII and alternating ASCII compiler/lifecycle gates pass.
Local fmt, workspace all-target check, warnings-denied Clippy, Rust 1.95 runtime
and generated-facade all-target checks, diff check, CI split (**129 checks**),
inherited wrapper checks (**47 / 4**) and Task062 wrapper checks (**6**) pass.
Generated IPv6 Rust also compiles/runs its exact corpus under Rust 1.95.
Local tool versions: Rust 1.98.1, Rust 1.95.0, GNAT 14.2.0, GCC C++ 14.2.0.

Failure provenance is retained, not replaced by a success marker: initial
wrapper exit 127 before execution; initial incomplete integration compile
diagnostics; fail-closed codec readiness reproduction; stale Task060 IPv6
deferral assertion corrected while retaining the historical rows. A complete
workspace invocation passed **1,260 tests, zero test failures**, then failed
rustdoc with missing-crate diagnostics after another toolchain build had reused
the target during its execution. This invocation is NOT a successful workspace
gate. The serial full workspace rerun subsequently completed with exit 0
(`logs/workspace-final.log` and `logs/workspace-final.exit`); Rust 1.95 uses a separate
Task062 `msrv-target` directory. No shared target was cleaned. Source manifests
and command-specific durable exits are under the Task062 artifact root.

The initial PR publication was stacked on Task061 while PR #62 was open.
The corrective below reconciles onto its independently merged main tree without
moving the immutable benchmark. Fresh hosted conclusions and actual checkout
identities must be recorded before final readiness is claimed. No merge
authorization, and no auto-merge.

## Inherited Task058 Deep-CI corrective and main reconciliation

### Preserved old stacked failure

The reviewed Task062 head was
`8765b455235197eb533ba3d89c8a0522cf81dd60`, stacked on the immutable
BEFORE `add348ce867daf4253a88510a65a48a3ed23f218`.

| Gate | Run | Attempt | Conclusion |
|---|---:|---:|---|
| Old stacked Fast | 37173941536 | 1 | SUCCESS |
| Old stacked Deep | 37173941535 | 1 | FAILURE |

Deep's real-uci job `111352510175` completed the **Task062 exact IPv6
inventory/impact/real vertical step SUCCESS** before the inherited Task058
failure. Task061 Time and 609 unique service confirmations and Tasks052–057
also passed; Tasks059/060 were skipped after Task058 failed. The other Deep
jobs, msrv-real-uci `111352510070` and real-sleet `111352510176`, succeeded.

The actual first panic was in `task058_closed_schema_ada_gap_evidence` at
`crates/cli/tests/uci_bounded_ascii_string.rs:373:13` on that old source:

```text
assertion `left == right` failed
  left: 641
 right: 632
```

**641 is correct:** Task062 independently measured a +9 UCI2.5 closed Ada
renderable-closure gain over 632. Rust/C++ gained +10 (664→674); UCI2.6 gained
637→646 Ada and 669→679 peers. This is an inherited stale current-state
assertion, not an IPv6 compiler, lexical matcher, or codec failure.

Complete attempt-1 log archives, per-run/job JSON and original diagnostics were
recovered once into the external artifact directory
`logs/hosted-old-corrective/`. The old collector is retired, not restarted;
neither old workflow was rerun. The old synthetic checkout was
`c876e726197db5bccdd2205dad59ce59d8931e52`, distinct from the feature head and
stacked base. Terminal old evidence does not certify the corrected main-base PR.

### Narrow corrective ownership

Task058 no longer asserts moving global closure totals or compares the entire
live gap with Task061's historical TSV. Replacing 632/664/637/669 with
641/674/646/679 would merely cause another legitimate capability task to break
the inherited regression. Instead its existing test retains:

- equal current Rust/C++ renderable-closure sets and Ada as their subset;
- `Authorization`, `AuthorizationRequest`, and `CommSupportActivity` in the
  peer-minus-Ada gap, without claiming they are the entire current gap;
- exactly `QueryPET` and `QueryType` as Ada-only unsafe declarations, not unsafe
  in Rust/C++;
- the exact full-schema `QueryType companion` / `QueryType` collision at
  `QueryType_Kind`, with peer preflight safe;
- for each historical witness, sole relevant unsafe closure declaration
  `QueryPET`, the established bounded-ASCII members
  `AlphanumericDashSpaceUnderscoreStringLength15Type` and `EmptyType`, absent
  `QueryType`, safe projected Ada naming and production projected service
  readiness in all backends.

There are still **five Task058 tests**. The Deep workflow changes only comments:
its inherited inventory now retains three unsupported Unicode neighbours after
IPv6 admission. Commands, count requirements, markers, ordering, timeout,
triggers and concurrency are unchanged.

`tests/fixtures/temporal/task061-ada-full-schema-gap.tsv` remains historical and
unchanged. Exact Task062 current-state coverage remains owned by
`task062_pinned_ipv6_inventory_coverage_and_impact` (all 12 cells). Current
33-message naming remains independently enforced by
`task061_pinned_ada_full_schema_naming_attribution` and the exact-set assertion
in `task060_real_projected_message_impact_and_naming` against
`tests/fixtures/string/task062-ada-full-schema-gap.tsv`. No historical
Task058/061 measurement is rewritten as a Task062 measurement.

The full Task062 production diff was reread: admission is exact, name-free
ConstraintSet equality; independent 2..45 length facets and full authored XSD
lexical matching are enforced; embedded IPv4 implements precisely the authored
decimal alternatives; all carriers preserve spelling; Ada forms absolute
indices only at proven in-range offsets, never one-past-Positive'Last; shared
codec readiness and generated checked construction remain aligned;
unsupported String neighbours fail closed. There is no production blocker and
**no production change or rollback** in this corrective. Source-size guards
remain part of the requested Fast validation.

### Integration identity and fresh gate

Focused corrective validation used independently rehashed Task062 copies of
both pinned roots. `task058_closed_schema_ada_gap_evidence` completed exactly
one release test, exit 0, both release markers (**176.60s**).
`task061_pinned_ada_full_schema_naming_attribution` then completed exactly one
release test, exit 0, both release markers (**118.20s**); its fresh 33-message
output for each release was also compared exactly with the Task062 gap TSV.
The requested `check-task062-fast.sh`, Task062 CI wrapper tests, CI split and
adversarial split tests, fmt, all-target workspace check, warnings-denied
all-target Clippy and diff check all completed exit 0. Durable commands, logs,
exits and marker verification are in `logs/corrective/`.

The successful old exact-head pinned Task062 12-cell impact/vertical campaign
is retained by source/dependency identity, rather than rerun locally for this
test-only correction. `manifests/corrective-production-identity.json` records
67 unchanged production, build, corpus and Task062 fixture inputs. The fresh
hosted Deep campaign will execute the full current evidence again.

Live main was verified at `cf9ebc518f49e58b1731417186c836f3a41e0e74`, the
Task061 merge with parents `0800e97c71d8bf3edcef40ef1dc4ef5200b76deb` and
`add348ce867daf4253a88510a65a48a3ed23f218`. Its tree
`f0451bd2e04da6f298fab754b23afe1ad390e721` exactly equals the immutable BEFORE
tree. Normal merge reconciliation, not rebase/cherry-pick/force-push, is used.
The immutable Task062 benchmark remains
`add348ce867daf4253a88510a65a48a3ed23f218`; the separate final integration base
is `cf9ebc518f49e58b1731417186c836f3a41e0e74`.

Fresh automatically triggered Fast/Deep on the final feature head and main base
are required. Their results are **pending, not claimed successful** until the
single head/base-keyed collector preserves terminal logs, actual synthetic
checkout SHA and parent relationship, all Deep jobs and execution markers.
PR #63 must stay open, non-draft, unmerged, auto-merge disabled. Task063/064
and their worktrees are outside this corrective.