# TASK112 acceptance evidence

Four receipts describe clean-source acceptance at
`3c1aacd409ffc5eeb8e5da27643959478534c392` /
`5c713c424f7cbac0f9e8a978f43499589a704de6`:

* `/docs/evidence/task112/16-openssl.json` — UCI 2.6 and original UCI 2.5.
* `/docs/evidence/task112/16-mbedtls.json` — UCI 2.6.
* `/docs/evidence/task112/14-openssl.json` — UCI 2.6.
* `/docs/evidence/task112/14-mbedtls.json` — UCI 2.6.

All cells PASS. Each release contains eight epochs, actual captured protocol
commands/responses, document-frame hashes, callback observations, TLS shutdown
classification, broker connection ownership, and identical `fd=8 tasks=3`
cleanup measurements. Source/dependency commit/tree identities, schema and
unchanged generated hashes, and all command exits are included. No keys,
schemas, full payload captures or build products are committed.

Full independently checkable execution records remain outside Git under
`/home/zboll/task112-evidence/accept{16,14}-{openssl,mbedtls}/`. Each contains
`artifacts.json` with SHA256 for every full artifact, including exact command
arrays/cwd, stdout/stderr, raw wire streams, observer traffic, generated
sources, dependency archives, isolated builds and executables. The receipt
pins that manifest and the relevant raw artifacts. Run the verifier documented
in `/docs/task-112-ada-wss-session-recovery.md` against those records to
recompute the receipt; it rejects changed source inputs or artifacts.

The final evidence/docs-only commit is deliberately separate from executable
qualification inputs, avoiding a self-referential final-commit receipt.
Hosted PR smoke and exact final HEAD/tree are reported in the delivery/PR;
they are not falsely claimed to have been part of the earlier local campaign.