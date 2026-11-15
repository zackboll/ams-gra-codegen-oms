# R2D peer-closure subset provenance

The [acceptance document](../../../task-115-peer-closure-subset-acceptance.md)
defines the narrow contract. `acceptance-ledger.json` is a compact, path-scrubbed
index, not the raw external evidence or a newly executed runtime campaign.
All earlier TASK115 receipts remain unchanged and false for original acceptance.

## Reproduce from a complete checkout

Set `SETUP`, `DEPS`, `SCHEMA`, `CLIENT_SCRATCH`, and `WSS_SCRATCH` to absolute
paths appropriate to your machine. Follow the unchanged application README for
the exact prerequisites, pinned repositories, build, CLI/policy tests, and
real-Sleet campaign. Use a new scratch directory for each compiler/backend/UCI
cell and a Python environment with websockets 15.0.1. Do not rebuild in a frozen
qualification archive or mix provider object directories.

From the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/task115 -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_pr_smoke.py
python3 scripts/task115/build.py --release 26 --compiler 16 --backend openssl \
  --setup "$SETUP" --deps "$DEPS" --schema "$SCHEMA" \
  --generator "$GENERATOR" --scratch "$CLIENT_SCRATCH"
python3 scripts/task115/test_qualification.py --build "$CLIENT_SCRATCH"
python3 scripts/task115/qualify.py --build "$CLIENT_SCRATCH" \
  --evidence "$WSS_SCRATCH" --deps "$DEPS" --release 26
python3 scripts/task115/verify.py "$WSS_SCRATCH" --receipt "$NEW_RECEIPT"
```

The optional `--build` test writes policy-test outputs: run it only in your own
new scratch, never against a preserved build. Repeat with the five ledger cells
and the corresponding authentic release schema. This campaign command does not
reproduce the supplemental R2B/R2C/R2D pressure experiments by itself.

## Reverify preserved evidence without rerunning campaigns

Obtain the preserved `task115-r2a-evidence`, `task115-r2b-evidence`,
`task115-r2c-evidence`, and `task115-r2d-evidence` bundles from the qualification
custodian. Verify the ledger's R2D manifest digest first, then every entry in
that manifest and the nested preservation/provider/source manifests. These
bundle names and content hashes, rather than an author's local absolute paths,
identify the artifacts. Raw captures/private keys are deliberately not in Git;
this index does not imply they are publicly downloadable.

R2D `audit.py` is the reference reconciliation procedure: it checks the five
build manifests and 53 source-input hashes per cell; generated/build copies;
schema, executable, dependency, provider, and linked-library hashes; existing
campaign receipts using repository `scripts/task115/verify.py`; R2B supplemental
receipts using `verify_supplement.py`; and R2C receipts using `verify.py`.
Its original filesystem bindings are archival: when replaying, bind input roots
to the supplied bundles and redirect only its output root to new scratch. Never
overwrite historical results. R2D `verify_real_pressure.py` separately verifies
the final pressure run and its four mutation tests; R2C `test_verifier.py`
provides ten mutation tests. Exact script hashes are indexed in this ledger.
Tool relocation is not a new runtime qualification.

Publication reverified this procedure against the frozen original inputs, without
rerunning complete network campaigns. The historical R2D final-preservation
receipt is itself hash-verified; its old all-refs-unchanged assertion describes
that earlier audit, not the newly authorized publication branch/ref updates.
The compact totals must remain 30 / 55 / 5 / 20 / 60, with the one genuine-pressure
proof separate. Injected EAGAIN is not genuine kernel backpressure. There is no
kernel-blocked INIT/SUB, selector park/wakeup, or five-cell genuine-pressure claim.
