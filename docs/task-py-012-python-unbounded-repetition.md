# PY-012 — Python unbounded Record members and OMS JSON

## Isolation and source identities

All task work is in `/home/zboll/git/ams-gra-codegen-oms-py-012`, branch
`feature/py-012-unbounded-repetition`. Before implementation, fetched `main` and
compared it with `git ls-remote`: commit
`ab438eab77d0b202a033bec2b2fa99508bbebc8f`, tree
`02c0df930e7af0c8e487994ac25e5a69d1104a33`. This is the merge of PY-011 PR #158;
its branch, worktree, PR and historical evidence were not changed.

During qualification, verified a subsequent `main`, commit
`18e64c7c65a0c6926fd5cc7af04834c458c52310`, tree
`133d1515378e1bd98947bf97af92b62bf03399d5` (Task106), and incorporated it by an
ordinary merge. No rebase, force push, approval, merge or auto-merge of the task PR.
The complete committed AFTER qualification source is
`a062b1a96874fb77c69305ffcdca14a97c5e147f`, tree
`39ca54f00b80b3bb3a02df0b432435a01a854889`. Later task commits change only
qualification wiring, documentation and evidence; the evidence authenticates the
exact production source hashes and executable SHA-256, rather than pretending
that a document can contain its own final Git identity.

At the original delivery, fetched and observed newer main
`f1aa82782fd0b0f2b0ace1311d7f16afe8e11d7f`, tree
`c2b4581d2c383245c1675537256b0ef95e711349` (unrelated Rust RDMA qualification).
It was not merged into the original qualified source. See the R1 reconciliation
below for the subsequently incorporated main and separately rerun qualification.

Clean detached source worktrees live under the task's ignored `target` directory.
Raw logs, binaries and generated artifacts are retained separately in
`/home/zboll/py012-evidence/v1`; the Cargo target is `/home/zboll/py012-build`.
The initial incomplete debug baseline and initial failed test attempts are
retained there, not counted as passing qualification.

## Implementation and ownership

Production edits are confined to the Python backend's model and codec renderers.
They consume existing `OccurrenceShape::Unbounded { min }`; no new IR, frontend,
classifier, schema interpretation, primitive profile or dependency is introduced.
Lists, Choices, Binary and abstract-value polymorphism remain unsupported.

* Record members use `tuple[T, ...]`, with exact `tuple` checks (including
  rejection of tuple subclasses), exact normalized minimum, and per-element
  existing validation. Minimum zero defaults to `()`; positive minima have no
  default. There is no upper bound, truncation, sorting or coercion.
* Frozen, slotted, keyword-only dataclasses and `effective_record_fields` remain
  the representation and inheritance mechanisms. Immutable tuple storage may be
  shared with the caller; immutable nested Records/carriers retain existing
  ownership semantics. Direct DateTime values are checked and normalized into a
  new tuple by the existing lexical validator. This is not a promise of unlimited
  memory or recursion capacity.
* OMS codecs map tuples to ordered arrays and arrays to newly validated tuples.
  Omitted/empty zero-minimum members decode to `()` and empty optional tuples are
  canonically omitted on encode. Positive minima require present arrays.
* The existing sequence helper accepts `None` for an absent upper bound only in
  generated modules demanding unbounded Record members. Bounded checks retain
  their upper bound. Undemanded modules retain byte-identical source.
* Existing checked scalar/Record/carrier codecs validate both directions. Named
  DateTime carriers with forged missing storage receive a path-bearing codec
  error in the newly admitted surface. Wire names, Clark-qualified names,
  nested/inherited paths and each `[index]` are preserved. Strict parsing,
  duplicate/unknown members, `$type`, exact integers and Unicode policy are not
  relaxed. Explicit checks, not Python `assert`, implement new validation.

Four old rejection cases that asserted unbounded members were unsupported were
removed from their boundary lists. Other unsupported cases remain. PY-011's
historical source test now authenticates its complete recorded source-hash map
against an independent fixed digest, rather than freezing today's backend at
PY-011's bytes. This works in shallow main checkouts. All historical fixture and
evidence bytes remain unchanged.

## Qualification

CPython **3.11.17** and **3.13.16**; normal and `-O` execution. No missing-input
skips are accepted by the new tests. New fixtures use independently authored wire
trees and literal JSON, not only encoder-produced round trips.

| Check | Result |
|---|---|
| `cargo +1.95.0 fmt --all -- --check` | passed |
| `cargo +1.95.0 check --locked --workspace --all-targets` | passed |
| Python backend/CLI all-target Clippy, `-D warnings` | passed |
| Python backend all targets | 23 passed per interpreter, none failed/ignored |
| Ten Python CLI targets, including NATO and unbounded | 34 passed per interpreter, none failed/ignored |
| New model probe | 5 positive + 4 negative test methods per invocation |
| New codec probe, additional to model methods | 3 positive + 6 negative methods per invocation |
| Probe matrix | 2 worlds × 2 optimization policies × 2 import styles × model-only/codec-enabled |
| Aggregate generated probes per interpreter | 72 model-only + 144 codec-enabled methods, plus 4 qualified-path methods |
| Atomic unsupported selections | 5 mutations × 2 worlds × 2 codec selections; fresh and preexisting destinations |
| Determinism | repeated generation, cross-world bytes and model-only/codec model/API parity |
| Large traversal | 8,193 integers, exact ordered length/content in construction and both codec directions |
| Fast discovery | 7 tests, both interpreters and optimization policies |
| Pinned UCI discovery | 4 tests, both interpreters and optimization policies |
| Pinned Sleet discovery | 4 tests, both interpreters and optimization policies |
| Earlier mock and PY-006–PY-011 Sleet scripts | passed for both interpreters, including their normal/optimized probes |
| Historical PY-007–PY-011 evidence adversaries | passed; historical evidence unchanged |
| Existing successor UCI harnesses | PY-007/PY-008 passed; PY-009 passed; PY-010 passed for both interpreters, 416 isolated profile tests each |
| Unbounded-free Python parity | 32 selections, 80 Python sources byte-identical, including NATO |
| Existing VisibleAscii-free parity | 20 selections, 50 sources byte-identical |
| Ada/Rust/C++ parity | 6 generated selections, 12 readiness reports, 2 coverage reports identical |
| Python PR smoke | passed locally |
| Router/qualification-runner tests | 28 / 4 passed |
| Workflow syntax | existing verified local actionlint passed |

Subtests additionally cover first/middle/last invalid elements, strict tuple
containers, forged storage and values, integer limits, Boolean-as-integer,
constrained String and temporal carriers, enumerations, inherited and nested
Records, duplicates/order, missing/null/scalar containers, unknown fields and
invalid Unicode. Test-method counts above are not counts of individual subtests.

## Real UCI BEFORE / AFTER

24 BEFORE and 24 AFTER selections: two pinned schema releases, three unchanged
service contracts, both worlds and model-only/codec-enabled selection. Every
selection remains **NOT READY**, codec **NOT MEASURED** when requested, exit 1,
atomic generation with no output directory/artifacts. No real-UCI service
interoperability is claimed.

Each row below represents all four world/codec combinations at each stage.

| UCI | Service | BEFORE first blocker | AFTER first blocker |
|---|---|---|---|
| 2.5 | PositionReport | `FGI_SourceOpenChoiceType`, Choice/List | unchanged |
| 2.6 | PositionReport | `FGI_SourceOpenChoiceType`, Choice/List | unchanged |
| 2.5 | RDMA_InitializeSetup | `IPv4_AddressType`, unsupported constraints | unchanged |
| 2.6 | RDMA_InitializeSetup | `IPv4_AddressType`, unsupported constraints | unchanged |
| 2.5 | SubsystemStream | `SubsystemStreamMDT`, unbounded `SubsystemStreamString` | `SubsystemStreamMDT`, direct field `SubsystemStreamBinary`, primitive Binary |
| 2.6 | SubsystemStream | `HexBinaryType`, primitive Binary | unchanged |

The primary unbounded blocker disappears. Binary is deliberately not implemented.
PositionReport differs from pre-PY-011 history because PY-011 is already merged
in the exact starting main identity; neither measurement stops at NATO.

Pinned UCI roots: 2.5 revision `093610b7753944059360d3236770ab446d039556`, SHA-256
`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`; 2.6 revision
`78eb61b6112c8bffa40820c33124b57787fc5bd9`, SHA-256
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
SecurityMarkings include hashes are verified independently and recorded in both
stage documents. Selected QName closures and contract identities are unchanged:
60/50/40 types for 2.5 and 60/50/41 for 2.6. Exact closure hashes are in evidence.

An additional discovery test extracts the authenticated `SubsystemStreamString`
member and `VisibleString1024Type`, checks generated array codecs in both worlds
and policies, and exercises independent JSON. This is explicitly an **isolated
pinned member probe**, not an unchanged full UCI service or interoperability.

## Pinned-Sleet synthetic interoperability

Independent synthetic OAM schema `sleet.xsd` contains positive/zero-minimum
unbounded Strings and zero-minimum direct DateTime. It uses the unchanged Python
LA-CAL runtime and pinned Sleet commit
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`, clean checkout. Each test verifies those
identities. No broker/runtime source or dependency pin changes.

Four explicit executions (two interpreters × normal/optimized) deliver typed
messages with omitted/empty, singleton and multi-element arrays. Tests verify
duplicates, ordering, whitespace/Unicode String spelling and DateTime fractional
lexical preservation. Explicit raw `[]` also reaches the typed handler as `()`.

Per execution, eight malformed payloads are rejected at the broker (missing/empty
required array, null/scalar containers, bad elements, unknown member). They are
**not** counted as decoder executions. A ninth payload with repeated direct
DateTime `2023-02-29T00:00:00Z` is admitted by this broker; an instrumented decoder
counter and captured decoder input prove it reaches generated Python and raises
`CodecError`, surfaced as runtime `CodecFailure`, without typed handler delivery.
Every malformed case is followed by successful valid delivery. Unsubscribe,
disconnected runtimes, broker termination/wait and no new pending asyncio tasks
are asserted. Observations retain the attribution and actual decoder input.

## CI compatibility

No PR selector or artifact gate is introduced. The single required hosted
`PR smoke` remains; full qualification remains on main. R1 preserves main's
ten-lane scheduling. New tests join existing Fast/UCI/Sleet discovery entrypoints.

Main Deep Python commands previously still referenced PY-010 service verdicts.
Only their successor-baseline arguments now point at the new versioned PY-012
measurements. PY-010's runner gains the already-established optional
`--service-baseline` argument, retaining its historical default. The current
service results change but no historical oracle is rewritten. The earlier
profile and inheritance executions remain in those existing harnesses.

## Reproduction

```sh
cd /home/zboll/git/ams-gra-codegen-oms-py-012
export PYTHONDONTWRITEBYTECODE=1 CARGO_TARGET_DIR=/home/zboll/py012-build
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --locked --workspace --all-targets
cargo +1.95.0 clippy --locked -p ams-gra-oms-backend-python \
  -p ams-gra-codegen-oms --all-targets -- -D warnings
for PYTHON in python3.11 python3.13; do
  export PYTHON
  cargo +1.95.0 test --locked -p ams-gra-oms-backend-python --all-targets
  cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
    --test python_cli --test python_service --test python_codec \
    --test python_header_profiles --test python_inheritance --test python_float64 \
    --test python_datetime --test python_visible_ascii --test python_nato_special_words \
    --test python_unbounded_repetition -- --nocapture
done
env -u CARGO_TARGET_DIR python3.11 scripts/pr_smoke.py run python
python3.11 scripts/test-py012-uci.py
git diff --check
```

Pinned inputs and runtime environment are the same verified paths used by PY-011:

```sh
export AMS_GRA_UCI_2_5_ROOT=/tmp/perf-001-uci25/OAC-STD-UCI_V2.5/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/tmp/perf-001-uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
export AMS_GRA_GENERATOR=/home/zboll/py012-evidence/v1/qualified-bin/ams-gra-codegen-oms
export AMS_GRA_SLEET_BIN=/home/zboll/ci-select004-evidence/pinned/sleet/target/release/sleet
export PYTHONPATH=/home/zboll/git/ams-gra-codegen-oms-py-012/python/oms-lacal-runtime/src
export RUNNER_TEMP=/home/zboll/py012-evidence/v1/runner
mkdir -p "$RUNNER_TEMP/py006-uci"
for version in 311 313; do
  export PYTHON=/home/zboll/ci-select004-evidence/python$version/bin/python
  bash scripts/check-py012-sleet.sh
  for mode in fast uci sleet; do
    "$PYTHON" python/qualification/run.py "$mode"
    "$PYTHON" -O python/qualification/run.py "$mode"
  done
done
```

UCI discovery also requires the existing `target/release/ams-gra-codegen-oms`
path. Use fresh output directories for measurement/parity scripts. The new
`measure-py012-uci.py` takes the same generator/closure-tool/source-root/output
arguments as the established measurement, verifies include hashes, and records
the PY-012 base identity. `check-py012-uci.py` compares BEFORE/AFTER files and
checks the exact measured boundaries; `test-py012-uci.py` exercises adversaries.

## PY-012-R1 — historical authentication and main reconciliation

The reviewed head was `4c974ffb43cd63ac00e1a9c8bf2716b6eed39ad8`, tree
`29be7769d6147700731f01c486b464e4499339dd`. R1 incorporated verified main
`f2991de209b9fd7c7de1681e79411e75afd93784`, tree
`0e712ad5477b3e7d5ce88cdb820a04a97f1bb9f8`, by ordinary merge
`dff0d64c2f95a0a023ae8ae7e6f3c9740f1fa251`. This preserves main's Ada/Rust work,
ten-lane scheduling and PR smoke configuration; no retired selector or artifact
gate is restored.

The PY-012 historical test now authenticates the entire recorded
`production_sha256` map using canonical JSON (sorted keys, compact separators)
against independently fixed SHA-256
`5b9709087f851c80a80176065a0bb5ef112e2b4c4b157d40e2c916f258e5847e`.
It also pins the original clean AFTER commit/tree. Manifest authentication and
BEFORE/AFTER measurement validation remain in place. No historical JSON is
changed, and no current production bytes or historical Git objects are required
to validate history. Adversaries cover removed, added, modified and substituted
map entries, altered provenance, and unavailable current production sources.
This follows the existing PY-011 pattern without weakening current generation
or qualification checks.

The only R1 production source change is a fail-closed check in
`service_codec.rs`: the unbounded runtime substitution must find exactly one
expected bounded-sequence check, or generation returns an error. Unit tests
cover zero, one and duplicate matches. All other Python production sources and
all runtime templates are unchanged. Generated bytes for the valid template
are intended to be identical, not merely semantically equivalent; qualification
below records actual comparisons rather than relabeling earlier evidence.

R1 execution logs and new-source measurements are stored separately in
`/home/zboll/py012-r1-evidence/v1`. Original evidence under
`tests/fixtures/python-unbounded-repetition/evidence/v1` and raw original logs
under `/home/zboll/py012-evidence/v1` remain immutable historical results.
