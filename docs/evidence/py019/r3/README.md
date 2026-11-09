# PY-019 R3 local acceptance bundle

`manifest.json` is the exact SHA-256 inventory of `payload/`. Verify it with
`scripts/py019_evidence.py::verify_files`. The payload includes independently
authored candidate JSON, actual broker/decoder observations, all twelve discovered
matrix receipts/logs, before/after service measurements and closure identities,
the complete historical comparison, regression logs, and source identities.

These are explicitly **local uncommitted-source** receipts. They bind actual
feature input hashes and do not impersonate hosted clean-source owned receipts.
Commit/PR/check identities are publication metadata and must be reported
separately. No approval or merge recommendation is implied.

The Sleet negative is schema-valid but outside the documented concrete-position
Python codec subset. It is not mislabeled malformed abstract data. All original
broker and envelope negatives remain separately attributed. Each transport cell
also verifies unsubscribe, silence, connection/broker shutdown and no remaining
async tasks.

Historical campaigns retained 24 verified R2 rows and executed 16 missing rows
per stage. Original argv/output paths are intentionally retained for those rows.
R0/R1/R2 failure evidence remains under `/home/zboll/py019-evidence`; prior
manifest hashes are recorded here without changing the historical bundles.

Six pre-existing opt-in CLI tests were ignored by the ordinary full CLI command;
their execution is not claimed. No test was newly ignored or suppressed.

The complete original external R3 evidence, including generated artifacts and
broker logs, is retained at `/home/zboll/py019-evidence/r3` and receives a separate
final manifest after publication metadata is recorded.