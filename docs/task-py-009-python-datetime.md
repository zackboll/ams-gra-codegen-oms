# PY-009 — checked Python DateTime models and OMS JSON codecs

## Isolation and dependency authority

Freshly fetched merged main: `e186c9dd7c700eb56e142a22309256f3de9a0141`,
tree `7074b9ded51f0aa23458c4d52c26b581dfafc143`. GitHub confirms PR #117
merged as `b6c6c9c5b1fabcd0716c1fd1183cc411d8841872` and PR #119 merged as
`3f69cb0e5b0801cf01b5cbde9018bb7a3b43fb9b`; both pass
`git merge-base --is-ancestor` against that main. Branch
`feature/py-009-python-datetime` was created from that exact main in
`/home/zboll/git/ams-gra-codegen-oms-py-009`. A separate clean detached baseline
worktree is `/home/zboll/git/ams-gra-codegen-oms-py-009-baseline`. Other worktrees,
PRs and historical qualification artifacts are not modified.

Task-owned raw logs, exit files, binaries, generated output and failure histories
are retained under `/home/zboll/py009-evidence`; build targets are
`/home/zboll/py009-build` and `/home/zboll/py009-baseline-build`. Precommit local
evidence is not clean exact-head hosted evidence. Final commit/tree, PR URL,
remote-head parity and hosted results are recorded in the delivery handoff;
a commit cannot embed its own identity.

## Implementation contract

Admission uses the existing `temporal_profile` and `direct_temporal_profile` in
`crates/codegen-core/src/temporal.rs`. Nothing classifies by UCI/type names.
Named DateTime requires exactly `TemporalProfile::DateTimeZulu`: one effective
XML Schema pattern alternative `.+Z` and no other facets. Every named primitive
restriction ancestor must retain the same proven profile; missing, cyclic,
incompatible or unconstrained ancestry fails closed. Named carriers are frozen,
slotted dataclasses with `value: str`, normalized in `__post_init__`.

Direct unconstrained DateTime requires `DirectTemporalProfile::DateTime` and
uses exact Python `str` with generated validation and normalized storage.
Concrete Records use the unchanged shared `effective_record_fields` projection,
including abstract ancestry-only bases, optional members, nested concrete
Records and bounded tuples. There is no Python inheritance/polymorphism policy.

The demand-gated standard-library helper validates XML Schema 1.0 lexical
spelling without `datetime`, timestamp conversion or fixed-width years/fractions:

- Gregorian month/day/leap-year validity, including negative years;
- four-or-more year digits, no year zero, no leading zero for extended years;
- hours 00–24, minutes 00–59, seconds 00–60, arbitrary fractional precision;
- hour 24 requires zero minute, second and fraction;
- lexical second 60 follows the established Task036 contract, without IERS or
  calendar-position restrictions or normalization onto another minute;
- direct values admit omitted timezone, literal `Z`, and signed offsets through
  ±14:00 (14 requires minute 00); named values require literal `Z`;
- only XML whitespace (#x20/#x9/#xA/#xD) collapses. Unicode whitespace, internal
  spaces, Unicode digits, suffixes, malformed widths and delimiters are rejected.

Validated normalized lexical spelling is stored and serialized verbatim, including
trailing fractional zeros, hour 24, leap seconds and `-00:00`. There is no temporal
arithmetic, canonicalization, ordering or value-space equality. Ordinary dataclass
equality compares stored lexical representations. Checks use exceptions, never
assertions, and remain active under `-O`.

OMS JSON encodes DateTime as strings. The same model validator runs on decode and
encode, including forged/mutated carrier values and direct Record members.
Expected invalid input becomes `CodecError` with qualified member/index paths;
typed handlers are not called on invalid input. Record order, OAM bare keys,
non-OAM Clark keys, duplicate rejection, Float64/integer handling and runtime
message-size limits are unchanged. DateTime-free generated sources remain
byte-identical. No OWP, runtime implementation, Sleet, transport or adapter
interface is changed.

Still unsupported: named unconstrained DateTime; other temporal patterns/facets;
field-local temporal constraints; unproven constraint ancestry; DateTime aliases;
named/direct Time and Duration; Binary, Decimal, Float32, unsupported Choices,
Lists and abstract-value polymorphism. Existing boundaries are not broadened.

## Independent generated-code qualification

`crates/cli/tests/python_datetime.rs` executes actual CLI-generated code, not a
handwritten model. It covers both worlds, codec/no-codec artifact selection,
deterministic regeneration, standalone/package imports, flattened inheritance,
optional/nested/repeated members, exact types, frozen/slotted storage, normalized
spelling, outbound forged values and strict JSON paths. Unsupported schemas
reject before writing files and preserve pre-existing output sentinels.

`tests/fixtures/python-datetime/lexical.json` is independently frozen from the
XML Schema 1.0 contract. Both unchanged Task036/046 corpora additionally run
against generated Python; Rust/C++/Ada temporal regressions execute those same
historical cases. A 5,001-digit year and 5,000-digit fraction exercise Python's
integer-string limit without changing process-global policy. Independent
positive/negative JSON goldens cover wire types and member/index paths. The
unchanged Task090 named/direct Clark-key JSON goldens are decoded and reencoded
by production-generated Python codecs, including wrong-namespace rejection.

Python 3.11.17 and 3.13.16 CLI regression runs pass **26 Rust integration tests
per version**, including 8 generated Python DateTime unittest cases in each of
8 world/optimization/import cells (**64 per version**). Codec-free generated
models/API imports execute two further cases in each of those eight cells
(**16 per version**). Task090 goldens also execute in normal/optimized modes.
Both versions' Python backend all-targets suites pass. Local default Python
3.13.5 also passed the initial focused campaign; it does not substitute for the
explicit 3.13.16 execution.

The inherited direct-DateTime rejection in `python_codec.rs` is deliberately
retained as an executed positive/direct-invalid-calendar control for this new
capability, not removed, ignored or filtered. Unsupported Time/Duration/etc.
remain the original negative controls. Initial compile API errors, stale boundary
expectations and CI-wrapper migration failures remain in raw evidence; later
passes do not rewrite those failed attempts.

## Real pinned Sleet

The existing shared broker at
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` is reused without changes. Its
checkout is clean; runtime uses pinned `websockets==17.2`.
`scripts/check-py009-sleet.sh` requires exactly one executed integration test per
normal/optimized policy, a completion marker, and success (no skips). Python
3.11.17 and 3.13.16 each pass both policies.

Actual OWP publish/subscribe delivers a generated Payload containing named,
direct, inherited, nested, repeated and optional DateTime values. The handler
receives the exact generated types and preserved long fractional spelling.
Outbound forged carriers are rejected, valid delivery recovers, unsubscribe
deactivates/removes the subscription, runtime contexts close, broker terminates,
and no `oms-lacal-*` task remains.

**Measured broker limitation:** pinned Sleet admits
`2023-02-29T00:00:00Z`. The test requires actual generated subscriber decoding
to reject it with `CodecFailure` before typed handler delivery, then verifies
subsequent valid delivery. It does not falsely claim broker calendar validation
or modify the broker to satisfy an assumed verdict. The initial expectation of
`ServerRejection` failed and is retained in the second-attempt logs; the observed
behavior is explicitly tested. This is a synthetic DateTime vertical, not full
real-UCI interoperability. Existing PY006/007/008 Sleet campaigns and all 19 mock
runtime cases also pass on both versions in normal/optimized modes.

## Pinned UCI BEFORE/AFTER measurements

Source revisions and root hashes remain the existing qualified pins:

| Release | Revision | Root SHA-256 |
| --- | --- | --- |
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

`scripts/measure-py009-uci.py` runs both `service-check` and `service-generate`
for all **24 release/message/world/codec selections** against the same exact
PY006 service contracts. BEFORE uses a clean Rust 1.95.0 release build of the
recorded main. AFTER uses the actual candidate executable. The evidence-only
`python_datetime_closure` example calls the unchanged shared service resolver;
it records the full ordered QName closure and its SHA-256. BEFORE/AFTER closures
are exactly equal, not inferred from counts. Command arrays, exit/stdout/stderr,
contract/root hashes and artifact maps are retained in task-owned frozen
`before.json` and `after.json`. No PY008 fixture is overwritten.

| Release | Service | Closure | BEFORE first blocker | AFTER attributable first blocker |
| --- | --- | ---: | --- | --- |
| 2.5 | PositionReport | 60 | primitive DateTime | VisibleString256Type: unsupported constraints |
| 2.5 | RDMA_InitializeSetup | 50 | primitive DateTime | IPv4_AddressType: unsupported constraints |
| 2.5 | SubsystemStream | 40 | primitive DateTime | VisibleString1024Type: unsupported constraints |
| 2.6 | PositionReport | 60 | unsupported constraints (historically unattributed) | VisibleString256Type: unsupported constraints |
| 2.6 | RDMA_InitializeSetup | 50 | unsupported constraints (historically unattributed) | IPv4_AddressType: unsupported constraints |
| 2.6 | SubsystemStream | 41 | primitive Binary | HexBinaryType: primitive Binary |

Every row applies to both explicit worlds and both codec selections. BEFORE and
AFTER model readiness and service API readiness are **NOT READY**, process exit
**1**; codec is **NOT MEASURED** when requested, otherwise **NOT REQUESTED**.
Generation also exits 1, atomically, with no output directory and an empty
artifact hash map. There are no successful real-service generated artifacts to
hash. This removes the measured UCI 2.5 DateTime blocker but does not establish
complete generation or service readiness.

`scripts/check-py009-uci.py` independently reruns the 24 selections against the
frozen evidence, checks exact closure/generation identities, and extracts the
actual pinned DateTimeType declaration (including UCI 2.6's included security
schema). It executes generated codecs in both worlds/import styles/optimization
policies. These isolated primitive probes are explicitly not real-service
round trips. `scripts/test-py009-uci.py` rejects truncation, duplication, identity,
closure, diagnostic, artifact and edited-oracle READY corruption. Existing
PY006/PY007/PY008 historical assertions and profile controls are preserved.

## Regression and CI ownership

Rust **1.95.0** locked workspace check and denied-warning Clippy pass. A GNAT-
required locked workspace campaign passes **1,498 tests**, zero failed, eight
pre-existing ignored across 191 summaries. Later focused additions have their
own explicit reruns; this count is the retained campaign's actual count.
The full inherited Float64 oracle passes on both Python versions in normal and
optimized execution: **27,167 conversions, 20,474 encodings, zero mismatches**
per cell. DateTime-free model/API/codec parity passes 12 selections/30 sources;
legacy Ada/Rust/C++ parity passes six generation runs, twelve readiness reports
and two coverage reports. No legacy renderer or shared temporal classifier is
modified.

Fast's existing 3.11/3.13 Python job must run `python_datetime` and PY009 evidence
negative controls. Deep's existing real-UCI regression job must build the closure
helper and run PY009 actual measurements/profiles, retaining evidence under its
existing mandatory `py006-uci` artifact. Deep's existing shared Sleet interpreter
loop must execute PY009 for every interpreter alongside PY006/007/008.

`scripts/py009_ci.py` projects only these exact mandatory additions and the
explicit successor-baseline arguments back onto immutable PY008 payloads. The
original policy then independently checks the inherited payload bytes, headers,
matrices, five lanes, artifact requirements, final gate and rerun rules. Dedicated
negative tests reject deleted/disabled/filtered commands, missing closure builds,
lost Sleet execution or reverted successor baseline. New surfaces select both
Python cells and their Deep owners; infrastructure edits select full validation.
No runner, security, final-gate or artifact provenance policy is relaxed.

The split adversarial harness passes **556 checks**. Selector tests pass
**133**, inherited security controls **30**, UCI artifact/
performance controls **18**. Pinned actionlint **1.7.12** passes. Documentation,
format and whitespace checks are mandatory. Hosted exact-final-head Fast/Deep,
required artifacts and `pr-ci-required` remain independent acceptance gates;
local success is not a substitute. **HOSTED_CI_PENDING** until they all pass;
any unresolved mandatory failure is **CORRECTIVE_WORK_REQUIRED**. Do not label
the PR ready for independent review while either condition holds. No merge,
auto-merge, approval, rebase or force push is authorized.

## Reproduction

Run from `/home/zboll/git/ams-gra-codegen-oms-py-009` (or the checkout's absolute
path). Use fresh task-owned output directories; keep the baseline executable
immutable. Supply the roots returned by the unchanged pinned fetch scripts.

```sh
cd /home/zboll/git/ams-gra-codegen-oms-py-009
export CARGO_TARGET_DIR=/home/zboll/py009-build
export PYTHONDONTWRITEBYTECODE=1
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
    --test python_float64 --test python_datetime -- --nocapture
done
cargo +1.95.0 build --locked --release -p ams-gra-codegen-oms \
  --bin ams-gra-codegen-oms --example python_datetime_closure
export AMS_GRA_UCI_2_5_ROOT=/tmp/perf-001-uci25/OAC-STD-UCI_V2.5/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/tmp/perf-001-uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
python3 scripts/test-py009-uci.py
for interpreter in python3.11 python3.13; do
  "$interpreter" scripts/check-py009-uci.py \
    --generator /home/zboll/py009-build/release/ams-gra-codegen-oms \
    --output "/home/zboll/py009-evidence/reproduce-$interpreter"
done
python3 scripts/check-py008-uci.py \
  --generator /home/zboll/py009-build/release/ams-gra-codegen-oms \
  --output /home/zboll/py009-evidence/reproduce-inherited \
  --service-baseline tests/fixtures/python-datetime/after.json
export AMS_GRA_GENERATOR=/home/zboll/py009-build/release/ams-gra-codegen-oms
export AMS_GRA_SLEET_BIN=/home/zboll/py006-deps/sleet/target/release/sleet
export PYTHONPATH=/home/zboll/git/ams-gra-codegen-oms-py-009/python/oms-lacal-runtime/src
for PYTHON in /home/zboll/py006-deps/py311/bin/python /home/zboll/py006-deps/py313/bin/python; do
  export PYTHON
  bash scripts/check-py005-mock.sh
  bash scripts/check-py009-sleet.sh
done
python3 -m unittest discover -s scripts -p 'test_ci_select*.py'
python3 -m unittest discover -s scripts/ci002 -p 'test_*.py'
bash scripts/test-check-ci-split.sh
ACTIONLINT=/home/zboll/git/ams-gra-codegen-oms-perf-009-evidence/tools/actionlint \
  bash scripts/check-ci-actionlint.sh
python3 scripts/ci-doc-policy.py
git diff --check
```

For a fresh broker/UCI environment use the existing
`scripts/run-real-sleet-test.sh` and pinned fetch setup from Deep CI; never replace
the pins with latest versions. Numeric reproduction uses the unchanged
`tests/ada-float64-probe/oracle.py` and `scripts/check-py008-numeric.py --full`
in all four version/optimization cells. BEFORE reproduction uses
`scripts/measure-py009-uci.py --generator` pointing at the clean main release
executable, `--source-root` pointing at the clean baseline worktree,
`--closure-tool` pointing at the built evidence example and
`--output` a new evidence directory. Do not overwrite historical fixtures to
make a changed result pass.
