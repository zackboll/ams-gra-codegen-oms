# TASK116 compact qualification receipts

Four clean-source GNAT14.2/GNAT16.1 x OpenSSL/mbedTLS cells each pass 19 real-TLS
deadline cases. Source HEAD `865bb824db0f84033fffaa5c9fdf40808baa5144`, tree
`ce2b7443d2419ea69dd862807aaa17b678cb4f42`. The API/production implementation
is unchanged from `0b5e5f28273c25d9ed72ddd2c151011ad64151c3`; the subsequent
test-only commit distinguishes valid short-budget exhaustion before ClientHello.

Each cell receipt binds compiler/backend, dependency commits/trees, input hashes,
executable hash, full artifact-manifest hash, elapsed times, connection counts and
post-finalization resource samples. Full captures, generated scratch, certificates,
private keys, binaries and command/output records remain outside Git under
`/home/zboll/task116-evidence/accept*`. Artifact manifests exclude private keys.

`compatibility.json` records representative unchanged TASK108, TASK111 and
TASK112–114 runs and hashes their external results/verifier receipts. `controls.json`
records the original failing reproduction, disposable TASK115 integration,
five rejecting evidence mutations and implementation hosted smoke. Discarded
dirty/source-changing/short-stage attempts are explicitly not acceptance.

Reverify a cell:

```sh
python3 /home/zboll/git/ams-gra-codegen-oms-task116/scripts/task116/verify.py \
  /home/zboll/task116-evidence/accept16-openssl
```

The 150 ms external tolerance covers process/scheduler overhead, not an extra
two-second budget. Setup allocation is conservative and may fail early. This is
network-wait deadline enforcement, not preemption of CPU/filesystem work or
cancellation of pinned AHT's detached OS DNS resolver. No dependency changes,
insecure mode, new task, publication replay or retryability redesign is claimed.
See `/docs/task-116-ada-wss-caller-deadline.md` for exact semantics and limitations.