# PY-002 — Python model generation through the public CLI

## Verified dependency and scope

PR #98 was open and unmerged when this task began. Its verified head was
`93d02dba9bdc398453a29b5dd3b9956ae26793fb`, exactly one commit ahead of
`origin/main` (`5f6800d505c6501638c322d20b23cfc36b09d7d6`), which was also
their merge-base. Branch `feature/py-002-python-cli-integration` starts at that
head and is stacked on `feature/py-001-python-model-backend`. The dependency
branch is not modified. Neither PR should be auto-merged.

The CLI now depends on `ams-gra-oms-backend-python` and selects `PythonBackend`
for ordinary, full-schema `generate`. There is no second renderer, preflight,
or Python runtime implementation in the CLI.

```sh
ams-gra-codegen-oms generate \
  --schema crates/xsd-frontend/tests/fixtures/track.xsd \
  --language python --world closed-schema --output generated/python
python3 -m py_compile generated/python/model.py
```

Python 3.11+ is required to use the standalone generated `model.py`; its imports
are standard-library only. See the [PY-001 subset](task-py-001-python-model-backend.md)
for supported types, runtime validation, and unsupported schema constructs.

`closed-schema` and `open-extensions` remain required, exact spellings, with no
default or abbreviation. The CLI forwards the chosen `GenerationWorld` directly
to the backend. In particular, accepting the open world does not make abstract
values renderable: the shared open-world guard still rejects them. Python also
rejects closed abstract-value representations rather than weakening their
meaning. The established additive `--overlay` path is unchanged.

## Deliberate service boundary

Python is not added to `BackendLanguage` or `BackendLanguage::ALL`. Shared
coverage, schema impact, and authoritative service readiness continue to cover
the independently qualified Ada/Rust/C++ backends only.

The CLI's conversion to a service backend is fallible. Python produces exit 1
and the explicit diagnostic:

```text
Python service commands are unsupported: service wrapper generation and authoritative Python readiness analysis are not implemented; no files were generated
```

Both `service-check` and `service-generate` reject Python before loading service
inputs, projecting a schema, performing readiness analysis, writing reports, or
touching output files. This also applies to empty/non-OMS selections and
`--with-codec`. Stdout is empty: no borrowed READY or NOT READY verdict is
fabricated. Required options and other usage validation still apply first
(exit 2), as they do for every language.

## Failure atomicity

Ordinary generation retains the existing sequence: load and validate the full
schema, render all files in memory, validate generated paths, then write files.
Python backend preflight rejects unsupported schemas before the filesystem
writer is called. A new output directory remains absent; existing files remain
unchanged. This is schema-rejection atomicity, not a new claim of transactional
rollback for unrelated filesystem failures during writing.

## Qualification

`crates/cli/tests/python_cli.rs` executes the production binary and checks:

- Track generation in both worlds, exact parity with `PythonBackend`, repeated
  runs into the same directory, and relocation-independent source bytes.
- Actual module compilation, import, resolved type hints, and the complete
  PY-001 Track runtime corpus, normally and under `-O` in both worlds (16 Python
  unittest executions per interpreter). Python is required, never skipped.
- Valid XSD inputs containing Choice, nillability, unsupported constraints, and
  abstract values; failures leave absent and populated destinations untouched.
- Invalid language/world spellings and missing world are usage errors without
  output.
- Both Python service commands, both worlds, with/without codec, selected and
  empty service contracts, and missing contract paths: explicit rejection,
  empty stdout, no new files, and no overwriting existing output.
- Truthful help and the unchanged three-backend coverage inventory.

The existing Python CI job now runs these CLI tests in addition to backend
tests on Python 3.11, and includes the CLI in locked Clippy and build checks.
Other workflow jobs are unchanged. The workflow edit also triggers the existing
Deep CI path filter; its results must be reported, not inferred from local tests.

For baseline comparisons, build the CLI from each recorded baseline SHA and
run `crates/backend-python/tests/check_existing_backends.py` against the
candidate, using identical absolute fixture paths. It checks six Ada/Rust/C++
Track generation runs, twelve ready/unready reports (including status/stdout/
stderr), and two coverage reports, across both worlds. Both the verified PY-001
head and original main baseline passed with canonical candidate evidence SHA256
`371b0223821c039ccbff57a1e5e1d99b06f010f5e820a10814a84bb9633d8ac2`.
This is bounded fixture parity, not a claim of exhaustive pinned-UCI coverage.

Exact final commit SHAs, local test totals, remote CI status, and any remaining
blockers are recorded in the delivery PR rather than predicting them here.