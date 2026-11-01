# PY-011 — Python NATO SpecialWords models and OMS JSON

## Scope and isolation

Fetched and independently compared `origin/main` with `git ls-remote`:
commit `070a9f8b8d40b7e8a48c093af655ca882034f987`, tree
`e26933a0bca500023ef4028c419f0a3b55e49d0e`. Created branch
`feature/py-011-nato-special-words` in
`/home/zboll/git/ams-gra-codegen-oms-py-011` and a clean detached baseline in
`/home/zboll/git/ams-gra-codegen-oms-py-011-baseline`.

Production changes are confined to the Python backend. The shared classifier,
normalized IR, frontend, other language backends, dependency pins, lockfile and
historical evidence are unchanged. No new schema semantics, regex interpreter,
case folding, trimming or normalization is introduced.

## Implementation

Only named primitive String declarations classified by the existing
`StringProfile::NatoSpecialWords` are admitted. Checked construction requires
exact `str`, total length 6–261, exact case-sensitive prefix `NATO:`, and a
1–256-character suffix containing only ASCII letters, hyphen and underscore.
Carriers use the existing frozen, slotted dataclass renderer. Named restriction
ancestry must retain the same proven fixed profile, with cycle/missing-base
checks. Unsupported aliases, other constrained strings, field-local facets,
Choices, Lists and abstract values remain unsupported.

The generated helper is demand-gated. Its use of `builtins.any` through a
private alias avoids introducing a schema-name reservation. The fixture
deliberately includes a declaration named `any`. Existing name collision checks
still reject unsafe names such as `str`.

OMS codecs reuse the existing checked-carrier ingress/egress helpers: exact
carrier types, checked reconstruction before encoding, missing/forged storage
rejection, exact lexical preservation, and member/index paths. A separate
non-OAM fixture mutation executes Clark-qualified diagnostic paths. Required,
optional, nested, inherited and bounded repeated fields use existing mechanisms.

## Versioned evidence and results

New evidence is under
`/home/zboll/git/ams-gra-codegen-oms-py-011/tests/fixtures/python-nato-special-words/evidence/v1`.
Raw logs, executables, generated sources, broker logs and numeric corpus are
retained in `/home/zboll/py011-evidence/v1`. `local-qualification.json` records
command templates, raw-log SHA-256 values, exit codes and reported counts.
`manifest.json` hashes the retained evidence files.

Tested interpreters: CPython **3.11.17** and **3.13.16**. Validation uses
`unittest`/explicit exceptions and remains active with `-O`.

| Qualification | Result |
|---|---|
| Python backend, all targets | 23 passed, 0 failed/ignored per interpreter |
| Python CLI/model/service/codec/header/inheritance/Float64/DateTime/VisibleAscii/NATO regressions | 31 passed, 0 failed/ignored per interpreter |
| New NATO generated probes | 24 model-only + 48 codec tests per interpreter; both worlds, normal/`-O`, standalone/package |
| Frozen Task 042 corpus | All 113 vectors, construction and both codec directions |
| Lexical adversaries | All 54 suffix characters; lengths 5/6/260/261/262; every Latin-1 character outside the alphabet plus Unicode lookalikes/surrogates; exact types and forged storage |
| Atomic failure | 14 XSD mutations × both worlds × model-only/codec, fresh and preexisting destinations |
| Normalized-IR adversaries | 14 mutations × both worlds, plus helper-demand test |
| NATO-free parity against verified main | 28 selections, 70 source files, byte-identical |
| Existing VisibleAscii-free parity | 20 selections, 50 source files, byte-identical |
| Full independent Float64 numeric regression | Each of four cells: 27,167 conversions + 20,474 encodings, 0 mismatches; four negative controls detected |
| Existing mock transport | 38 passed per interpreter across normal/`-O` |
| Existing header/inheritance/Float64/DateTime/VisibleAscii pinned-Sleet scripts | 10 passed per interpreter across normal/`-O` |
| Fast discovery | 5 passed in each of four interpreter/policy cells |
| UCI discovery | 3 passed in each of four cells, including original pinned NATO declaration probes |
| Sleet discovery | 3 passed in each of four cells, including generated NATO transport |
| Historical PY-007/008/009/010 evidence controls | 5/5/6/6 passed |
| Discovery regression | 4 passed |
| Python PR smoke | Passed; generated consumer executed |
| Formatting, scoped Clippy (`-D warnings`), diff whitespace | Passed |

Counts are test-execution counts, not a coverage percentage. Generated probe
tests loop over many lexical cases. The CLI harness captures model-only output
silently, so raw printed Python test counts do not include every executed probe.

### Authentic UCI readiness — BEFORE / AFTER

Root and security-markings include SHA-256 values are checked against existing
pins. Full readiness measurements use the production resolver's selected closure,
both generation worlds and model-only/codec-enabled modes: **24 BEFORE and 24
AFTER selections**, each with readiness and actual atomic-generation checks.
The closure and contract identities are unchanged across the implementation.

| Release | Message | BEFORE first blocker | AFTER first blocker | Readiness BEFORE → AFTER |
|---|---|---|---|---|
| 2.5 | PositionReport | `NATO_SpecialWordsType`: constrained String | `FGI_SourceOpenChoiceType`: Choice/List | NOT READY → NOT READY |
| 2.6 | PositionReport | `NATO_SpecialWordsType`: constrained String | `FGI_SourceOpenChoiceType`: Choice/List | NOT READY → NOT READY |
| 2.5 | RDMA_InitializeSetup | `IPv4_AddressType`: constrained String | Same | NOT READY → NOT READY |
| 2.6 | RDMA_InitializeSetup | `IPv4_AddressType`: constrained String | Same | NOT READY → NOT READY |
| 2.5 | SubsystemStream | `SubsystemStreamMDT`: unbounded `SubsystemStreamString` | Same | NOT READY → NOT READY |
| 2.6 | SubsystemStream | `HexBinaryType`: Binary primitive | Same | NOT READY → NOT READY |

Codec readiness is **not measured** when model generation blocks. No complete
real-UCI service became ready. Removing one blocker is not service readiness.

Separately, original `NATO_SpecialWordsType` declarations extracted without facet
changes from each authenticated security-markings include are generated and
executed against the corpus/full alphabet/boundaries. Each interpreter invocation
executes 32 generated profile tests across both releases/worlds/policies/import
styles. This is isolated declaration qualification, not real-UCI interoperability.

The BEFORE source is clean. The retained AFTER measurement truthfully records
`dirty=true` because it preceded the implementation commit; its complete
production SHA-256 map matches the final production sources. It is not presented
as clean-commit provenance. Initial slow debug measurement attempts were
interrupted and superseded by complete release-built measurements.

### Pinned-Sleet interoperability

Used the shared, clean Sleet checkout at commit
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`, with existing
`websockets==17.2`. The new test verifies the checkout identity and executes
actual CLI-generated model/API/codec code through LA-CAL sessions.

All four Python version/policy cells passed valid minimum, maximum, mixed-case
and full-alphabet delivery, exact lexical preservation, outbound forged-storage
rejection, recovery, subscription teardown and task-leak checks. An independent
explicit-alphabet oracle checks lexical examples without calling the generated
validator. All seven raw malformed values were observed as broker
`Invalid-Message` rejections in every cell. Attribution remains observation-based;
the test also handles/records generated-decoder rejection if a broker accepts a
malformed message. The retained observations describe what this pinned broker
actually did. This is a synthetic OAM fixture, not complete real-UCI service or
hardware interoperability.

## CI architecture and disposition

No workflow, selector, source-policy or artifact-gate changes. New tests join
the existing flat Python fast/UCI/Sleet discovery entrypoints with distinct
module names. The CI-RESET-001 PR smoke and complete main qualification,
including its five concurrent lanes, are preserved. Historical records and
historical test scripts are retained; they are not rewritten to describe a new
first blocker. No retired `pr-ci-required`, PR artifact gate or selector is restored.

At publication, disposition is **HOSTED_CI_PENDING** until the exact-head PR
smoke completes. Consult the PR's live checks for the final exact-head result;
this document does not claim hosted success before execution. No approval,
merge or auto-merge is authorized by this task.

## Reproduction

Run from `/home/zboll/git/ams-gra-codegen-oms-py-011`. Use fresh output directories
for scripts that require them. The local qualification JSON records executed log
names and hashes; paths below identify the retained inputs and binaries.

```sh
export PYTHONDONTWRITEBYTECODE=1 CARGO_TARGET_DIR=/home/zboll/py011-build
export AMS_GRA_UCI_2_5_ROOT=/tmp/perf-001-uci25/OAC-STD-UCI_V2.5/UCI_MessageDefinitions_v2_5_0.xsd
export AMS_GRA_UCI_2_6_ROOT=/tmp/perf-001-uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
export AMS_GRA_GENERATOR=/home/zboll/py011-build/debug/ams-gra-codegen-oms
export AMS_GRA_SLEET_BIN=/home/zboll/ci-select004-evidence/pinned/sleet/target/release/sleet
export PYTHONPATH=/home/zboll/git/ams-gra-codegen-oms-py-011/python/oms-lacal-runtime/src
for PYTHON in python3.11 python3.13; do
  export PYTHON
  cargo +1.95.0 test --locked -p ams-gra-oms-backend-python --all-targets
  cargo +1.95.0 test --locked -p ams-gra-codegen-oms \
    --test python_cli --test python_service --test python_codec \
    --test python_header_profiles --test python_inheritance --test python_float64 \
    --test python_datetime --test python_visible_ascii --test python_nato_special_words -- --nocapture
done
# PR smoke uses the default target directory for its generated consumer.
env -u CARGO_TARGET_DIR python3.11 scripts/pr_smoke.py run python
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy --locked -p ams-gra-oms-backend-python -p ams-gra-codegen-oms --all-targets -- -D warnings
git diff --check
python3.11 scripts/test-py011-uci.py
python3.11 -m unittest discover -s scripts -p test_python_qualification.py
python3.11 -m unittest discover -s scripts -p test_pr_smoke.py
for task in 007 008 009 010; do python3.11 scripts/test-py$task-uci.py; done
for version in 311 313; do
  export PYTHON=/home/zboll/ci-select004-evidence/python$version/bin/python
  bash scripts/check-py005-mock.sh
  for task in 006 007 008 009 010 011; do bash scripts/check-py$task-sleet.sh; done
  "$PYTHON" python/qualification/run.py fast
  "$PYTHON" -O python/qualification/run.py fast
  "$PYTHON" python/qualification/run.py sleet
  "$PYTHON" -O python/qualification/run.py sleet
done
# Full UCI discovery additionally requires the existing receipt directory and
# target/release/ams-gra-codegen-oms; see the recorded complete-uci-* logs.
export RUNNER_TEMP=/home/zboll/py011-evidence/v1/runner
mkdir -p "$RUNNER_TEMP/py006-uci"
for version in 311 313; do
  /home/zboll/ci-select004-evidence/python$version/bin/python python/qualification/run.py uci
  /home/zboll/ci-select004-evidence/python$version/bin/python -O python/qualification/run.py uci
done
```

`scripts/measure-py011-uci.py` records readiness/generation using `--generator`,
`--closure-tool`, `--source-root` and fresh `--output` arguments. The BEFORE
executables are in `/home/zboll/py011-baseline-build/release`; AFTER executables
are in `/home/zboll/py011-build/release`. Both use the existing
`examples/python_datetime_closure` resolver helper. Validate the resulting files
with `scripts/check-py011-uci.py --before ... --after ... --output ...`.

`scripts/check-py011-parity.py --before-generator ... --generator ... --output ...`
compares against the verified-main executable. Existing
`scripts/check-py010-parity.py` also passed unchanged. The independent numeric
command is `python3.11`/`python3.13`, optionally `-O`, followed by
`scripts/check-py008-numeric.py --full --generated
/home/zboll/py011-evidence/v1/numeric-generated --corpus
/home/zboll/py011-evidence/v1/numeric-corpus --output <fresh-result.json>`.
The corpus was generated by the unchanged
`tests/ada-float64-probe/oracle.py`.