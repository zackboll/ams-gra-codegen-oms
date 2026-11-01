# PY-010 — checked Python visible-ASCII Strings and OMS JSON codecs

## Isolation and authority

Fetched `origin/main` at commit `a44ce80f278c0a64c604a1b800679e02f8687cdf`,
tree `845a29e78466fa5ad2397cff8c475cf298027deb`. GitHub confirms PR #130
MERGED at that commit; its PY-009 implementation `63d03b4` and merge commit
both pass `git merge-base --is-ancestor` against main. Created branch
`feature/py-010-visible-ascii-strings` in
`/home/zboll/git/ams-gra-codegen-oms-py-010` and clean detached baseline
`/home/zboll/git/ams-gra-codegen-oms-py-010-baseline`. Other worktrees and
Ada/PERF evidence are untouched. No rebase, force push or merge is performed.

The unchanged shared classifier in
`/home/zboll/git/ams-gra-codegen-oms-py-010/crates/codegen-core/src/string_profile.rs`
is the admission authority, not UCI names. Reviewed Task 039's frozen corpus,
existing PY-006 and PY-009 implementations and qualification evidence. No
frontend, normalized IR, shared classifier, dependencies or lockfile changes.

## Exact implementation contract

Only named primitive String declarations classified as `VisibleAscii` gain
support. Inventory: `(1,20)`, `(1,32)`, `(1,64)`, `(1,81)`, `(1,128)`,
`(1,256)`, `(1,480)`, `(1,512)`, `(1,1024)`, `(2,4)`.

Each has one XML Schema `[ -~]{M,N}` expression, matching effective min/max
facets and no extra constraints. **Authority detail:** intrinsic `xs:string`
whiteSpace is preserve when the explicit facet is absent. The shared classifier
rejects explicit preserve as an extra facet as well as collapse/replace. This
task follows that existing authority exactly; it does not broaden it.
`PatternDialect` currently has only the `XmlSchema` variant, so there is no
second normalized dialect to synthesize in backend tests.

Carriers are frozen, slotted dataclasses holding exact `str`. The private
demand-gated helper checks inclusive lengths and the literal character interval
`' ' <= char <= '~'`. SPACE is significant anywhere, including all-space values.
Controls, DEL, NUL, non-ASCII, surrogates, wrong types and str subclasses fail.
There is no regex interpreter, implicit conversion, trimming, normalization or
assert-based validation. Named restriction ancestry is walked with cycle,
missing-base, profile and compatible-domain checks. Existing IR validation
also requires inherited pattern preservation; distinct conjunctive patterns
remain unproven rather than being simplified.

The codec reuses the established `_header_value`/`_header_text` structure,
demanded by either old header profiles or VisibleAscii. It requires exact carrier
types, revalidates through the checked constructor on encode and decode,
preserves stored spelling and qualified member/index paths, and rejects forged
or mutated storage. Strict parsing can reject floats or invalid Unicode at `$`
before schema traversal; member-local validation still carries exact paths.
Record inheritance, nesting, optional values and bounded tuples reuse the
existing concrete mechanisms. Unconstrained direct Strings are unchanged.

No support is added for WhitespaceVisible, NatoSpecialWords, general bounded,
structured or alternating ASCII, IPv4/IPv6, Unicode profiles, other constrained
Strings, unsupported aliases, field-local restrictions, Choices, Lists or
abstract values. DateTime, Float64, UUID and schema-version semantics and
VisibleAscii-free output are unchanged.

## Qualification and retained evidence

Task-owned raw logs, exit files, binaries and generated sources are retained
under `/home/zboll/py010-evidence`; isolated build directories are
`/home/zboll/py010-build` and `/home/zboll/py010-baseline-build`.
Versioned BEFORE/AFTER measurements and integrity manifest are in
`/home/zboll/git/ams-gra-codegen-oms-py-010/tests/fixtures/python-visible-ascii`.
BEFORE uses the clean starting-main executable. Precommit AFTER explicitly
records `dirty: true`, production source hashes and executable hash; it is not
misrepresented as clean exact-head hosted evidence. Final commit/tree and
hosted status belong in the handoff because a commit cannot embed itself.

The actual CLI harness exercises both explicit worlds, codec/no-codec modes,
standalone/package imports, normal/optimized Python, nested/inherited Records,
optional members, bounded tuples, deterministic regeneration, name collisions
and atomic failures preserving existing output. The model probe executes all
**60 frozen Task 039 cases**, all-ten-profile independent boundary tests, every
ASCII code point plus Unicode/surrogate adversaries. The codec probe applies
the shared corpus in both directions and exercises forged values and paths.
The CLI suite runs three model and six codec unittest methods in each of eight
world/policy/import cells: **72 method executions per interpreter**.

The pinned profile helper loads actual UCI IR and calls `string_profile` to
inventory all **13 declarations / ten unique pairs per release**. The gate
extracts those exact XSD declarations with their named ancestors, then CLI
generates and executes them in both worlds/policies/import styles: **416
unittest method executions per interpreter**, separate from service readiness.
The 20-selection PY-009 source parity inventory covers ordinary Record,
header, inheritance, Float64 and DateTime fixtures: **50 generated source hashes**.

Pinned Sleet revision: `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`; local checkout is clean.
The existing shared broker build and pinned `websockets==17.2` are reused.
Actual typed OWP delivery succeeds at 1 and 256 characters and with leading,
trailing and repeated spaces. Nine malformed inputs (TAB/CR/LF/NUL/U+001F/DEL,
non-ASCII, empty and overlong) yielded observed broker `Invalid-Message`
rejections on the initial campaign. The generated decoder independently rejects
all nine locally; **these events do not prove malformed delivery reached the
subscriber decoder**. Outbound forged carriers fail codec validation; valid
delivery recovers after every malformed publish. Unsubscribe, close and task
teardown are checked. Optional retained artifacts include generated sources,
per-case observations and broker logs, in both Python versions/policies.

Existing PY-006/007/008/009 broker tests, PY-005 actual network mocks, prior
generated-code regressions, the full independent Float64 numerical oracle,
legacy Ada/Rust/C++ parity, historical-evidence controls, selection/security,
artifact provenance negatives, split mutations and pinned actionlint remain
mandatory. Exact execution results and unresolved gates are reported in the
handoff; neither running tests nor hosted checks are presumed successful.

## Measured UCI BEFORE / AFTER

All entries below apply identically to both worlds and codec/no-codec selections
(24 total). Exact stdout/stderr, exits, contracts, root/closure hashes and
artifact inventories are retained in the versioned JSON. Model and API remain
**NOT READY**, exit **1**, zero generated artifacts. Codec-requested runs say
**not measured**; codec-free runs do not request codecs.

| UCI | Service | BEFORE first blocker | AFTER first blocker |
| --- | --- | --- | --- |
| 2.5 | PositionReport | VisibleString256Type constraints | NATO_SpecialWordsType constraints |
| 2.5 | RDMA_InitializeSetup | IPv4_AddressType constraints | unchanged |
| 2.5 | SubsystemStream | VisibleString1024Type constraints | SubsystemStreamMDT: unbounded SubsystemStreamString |
| 2.6 | PositionReport | VisibleString256Type constraints | NATO_SpecialWordsType constraints |
| 2.6 | RDMA_InitializeSetup | IPv4_AddressType constraints | unchanged |
| 2.6 | SubsystemStream | HexBinaryType: Binary | unchanged |

Pinned revisions remain `093610b7753944059360d3236770ab446d039556` (2.5)
and `78eb61b6112c8bffa40820c33124b57787fc5bd9` (2.6). Frozen PY-006 through
PY-009 evidence is not edited. Prior current-service gates receive an explicit
versioned successor baseline while their historical/profile checks remain.

## CI ownership and disposition

PY-010 is additive to existing Python Fast 3.11/3.13 cells, reusable Deep
`real-uci-regressions` and the shared `real-sleet` owner. No independent workflow
is introduced. Mandatory command placement, baseline, broker loop and artifact
negative controls project additions onto the previous immutable CI contracts.
Five-lane policy, selection, source identity, partial-rerun protections and
`pr-ci-required` are retained. Broker evidence uses `py010-sleet`; UCI evidence
remains nested in the established `py006-uci` artifact.

Until exact-final-head Fast, selected Deep and artifact/provenance final gates
pass, disposition is **HOSTED_CI_PENDING**. Any unresolved required failure means
**CORRECTIVE_WORK_REQUIRED**. Only all completed gates justify
**READY_FOR_INDEPENDENT_REVIEW**. Stop before merging; do not approve or enable
auto-merge.

## Reproduction

Run from `/home/zboll/git/ams-gra-codegen-oms-py-010`; output paths below must
be fresh. Preserve existing raw evidence rather than overwrite it.

```sh
export CARGO_TARGET_DIR=/home/zboll/py010-build PYTHONDONTWRITEBYTECODE=1
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --locked --workspace --all-targets
cargo +1.95.0 clippy --locked --workspace --all-targets -- -D warnings
AMS_GRA_REQUIRE_GNAT=1 cargo +1.95.0 test --locked --workspace
for PYTHON in python3.11 python3.13; do
  export PYTHON
  cargo +1.95.0 test --locked -p ams-gra-oms-backend-python --all-targets
  cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
    --test python_cli --test python_service --test python_codec \
    --test python_header_profiles --test python_inheritance \
    --test python_float64 --test python_datetime --test python_visible_ascii -- --nocapture
done
cargo +1.95.0 build --locked --release -p ams-gra-codegen-oms \
  --bin ams-gra-codegen-oms --example python_datetime_closure \
  --example python_visible_ascii_profiles
export AMS_GRA_UCI_2_5_ROOT=/tmp/perf-001-uci25/OAC-STD-UCI_V2.5/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/tmp/perf-001-uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
python3 scripts/test-py010-uci.py
for interpreter in python3.11 python3.13; do
  "$interpreter" scripts/check-py010-uci.py \
    --generator /home/zboll/py010-build/release/ams-gra-codegen-oms \
    --output "/home/zboll/py010-evidence/reproduce-$interpreter"
done
python3 scripts/check-py010-parity.py \
  --generator /home/zboll/py010-build/release/ams-gra-codegen-oms \
  --output /home/zboll/py010-evidence/reproduce-parity
export AMS_GRA_GENERATOR=/home/zboll/py010-build/release/ams-gra-codegen-oms
export AMS_GRA_SLEET_BIN=/home/zboll/py006-deps/sleet/target/release/sleet
export PYTHONPATH=/home/zboll/git/ams-gra-codegen-oms-py-010/python/oms-lacal-runtime/src
for PYTHON in /home/zboll/py006-deps/py311/bin/python /home/zboll/py006-deps/py313/bin/python; do
  export PYTHON
  bash scripts/check-py005-mock.sh
  bash scripts/check-py009-sleet.sh
  bash scripts/check-py010-sleet.sh
done
python3 -m unittest discover -s scripts -p 'test_ci_select*.py'
python3 -m unittest discover -s scripts/ci002 -p 'test_*.py'
bash scripts/test-check-ci-split.sh
ACTIONLINT=/home/zboll/git/ams-gra-codegen-oms-perf-009-evidence/tools/actionlint \
  bash scripts/check-ci-actionlint.sh
python3 scripts/ci-doc-policy.py
git diff --check
```

The full numeric campaign uses the unchanged
`/home/zboll/git/ams-gra-codegen-oms-py-010/tests/ada-float64-probe/oracle.py`
and `scripts/check-py008-numeric.py --full` on CLI-generated Float64 sources in
all four Python version/policy cells. BEFORE reproduction points
`scripts/measure-py010-uci.py` at the clean baseline executable and source root.
Fresh dependencies use the existing pinned UCI fetch and shared Sleet harness;
never replace pins with latest versions or rewrite historical NOT READY text.