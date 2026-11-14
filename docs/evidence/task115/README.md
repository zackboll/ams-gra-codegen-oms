# TASK115 receipts — partial implementation, BLOCKED_WITH_EVIDENCE

The four UCI2.6 cells and representative UCI2.5 cell pass the six real WSS
standalone executable cases. They do **not** establish full TASK115 acceptance:
each receipt explicitly sets `task115_acceptance: false` and records the real
stalled-TLS deadline overrun. Retryable loss is restricted to unambiguous peer
closure; ambiguous transport exceptions remain terminal.

Implementation source: `760e4a429d043386fd8236793dc049b775aed2e2`, tree
`ff28539a723a8508a21697a30072a331e67a2b22`. Builds report clean source.
These receipts bind exact compiler/backend/release, source, pinned dependency
commits/trees, schema hashes, generated hashes, executable hash, artifact-manifest
hash, independent broker owners, resource samples and deadline probe results.
`input_manifest_sha256` hashes the sorted-key JSON of the full input hash map in
the external build/qualification records. It is not a claim of a later HEAD run.

Full records are outside Git at `/home/zboll/task115-evidence/final*-wss` and
the corresponding build directories. Their `artifacts.json` covers raw captures,
logs, command JSON and results but excludes private keys. Reproduce verification
with `/scripts/task115/verify.py`; the external full receipts additionally contain
the complete input map. Seven external mutation trials reject evidence corruption.

No schema source, generated UCI model/codec, certificate key or binary is committed.
See `/docs/task-115-ada-uci-standalone-reconnect.md` and the application README
for exact commands, scope and limitations. Hosted CI is separate from local WSS
qualification. Do not infer strict deadline compliance from six passing cases.