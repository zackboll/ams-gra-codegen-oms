# TASK113 acceptance receipts

Four receipts record clean executable source commit
`d5ac16fbded173ab06e5815601c51d980d3f2ab9`, tree
`60d96cfa0ea9f458102ae72dafff607d4d69f67d`:

* `/docs/evidence/task113/16-openssl.json`: UCI 2.6 and original UCI 2.5.
* `/docs/evidence/task113/16-mbedtls.json`: UCI 2.6.
* `/docs/evidence/task113/14-openssl.json`: UCI 2.6.
* `/docs/evidence/task113/14-mbedtls.json`: UCI 2.6.

All PASS. Each release records five epochs, exact actual wire commands/ACKs,
authentic document-frame hashes, per-epoch callbacks and terminal state,
independent broker registration/removal evidence, and restored FD/task counts.
Dependencies, their trees, schema hashes, unchanged generated-source hashes,
qualified input hashes, generator/broker binary hashes and command exits are
included. No private keys, schemas, full payloads or binaries are committed.

Full records remain outside Git at
`/home/zboll/task113-evidence/final{16,14}-{openssl,mbedtls}/`.
Each `artifacts.json` hashes commands, stdout/stderr, raw captures, observer
transcript, schemas, dependency archives and isolated builds. Receipts pin the
manifest and selected artifacts. The verifier in
`/scripts/task113/verify.py` rehashes all artifacts, reparses raw captures and
recomputes the receipt, rejecting modified inputs or evidence. Reproduction
commands are in `/docs/task-113-ada-wss-subscription-recovery.md`.

The final evidence/docs commit is separate from executable qualification inputs,
avoiding self-referential source identities. Final HEAD/tree and hosted PR smoke
are reported in the PR/final delivery. Pre-broker command interception is **not
tested**; missing client ACKs never establish remote non-processing.