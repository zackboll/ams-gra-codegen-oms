# TASK108R2 — corrected AHT WSS requalification and main reconciliation

## Source identities and integration

The existing worktree and branch are preserved:
`/home/zboll/git/ams-gra-codegen-oms-task108r1`,
`feature/108-ada-wss-aht-fork-r1`, existing OMS PR #183.

| Identity | SHA |
| --- | --- |
| Reviewed original OMS HEAD | `3089370a0289f419a1e76493703cb0369c191266` |
| Original OMS tree | `4e81daac7d2988ac732f39c2cb23d59d838cec0f` |
| Fetched and incorporated main | `65dd473ea2deb8ea0472383e7e245ea7069eb753` |
| Ordinary merge commit | `c63e6d41129438db8c11e9ef72ca6ca49b05ea12` |
| Corrected-pin tested OMS HEAD | `740441b587aa8b20f70ecfee90b6ab133e51a631` |
| Corrected-pin tested OMS tree | `6473cded7c155272c7c7ff2a1573f2fae5e5cb09` |
| Corrected AHT HEAD | `827b0c17dd964e935f8e88912ce0611ee92b6f16` |
| Corrected AHT tree | `d137b924a7b0fa8b71c1ee8c582749e97294335b` |
| Unchanged websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Unchanged Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Unchanged Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |

The final publication commit adds only this report, captured evidence, and the
README dependency correction. Its exact HEAD/tree are reported on PR #183 and
in the task's final report, avoiding a self-referential commit identity here.
Local execution is attributed to the tested HEAD/tree above, not falsely to
the later documentation/evidence commit.

`origin/main` was fetched and merged with `--no-ff`; no conflicts occurred.
Main's Ada packing inventory, allocation, selection, provisioning and workflow
changes were retained. A byte comparison against current main establishes that
both CI workflow files differ **only** by the pre-existing WSS `libssl-dev`
additions. No new lane, concurrency rule, selector policy or full-main campaign
was introduced. The Python changes imported by the merge are main's existing
changes, not new task edits; the PR diff against main contains no Python-backend,
Rust, C++, shared-IR or generated-model/codec changes.

No OMS production Ada source changed from the reviewed R1 implementation.
`Send`, `Poll`, `Close`, verified trust, bounded output queues, frame ordering,
ownership and backpressure behavior are retained. No AHT or Sleet source was
changed. No upstream AHT PR was submitted or modified.

## R2 changes and scope audit

Only 16 executable pin/validator files changed in the corrected-pin commit:

* `scripts/task100/qualify.py` through `scripts/task106/qualify.py`, plus
  `scripts/task108/qualify.py`: immutable corrected AHT revision.
* `scripts/task100_evidence.py`, `scripts/task103_evidence.py` through
  `scripts/task106_evidence.py`: matching fail-closed expected dependency identity.
* `scripts/test_task103_evidence.py`, `scripts/test_task104_evidence.py`,
  `scripts/test_task105_evidence.py`: matching synthetic test receipt identities.

Each file was checked against the merge parent: the sole change is replacing
the old AHT SHA with the corrected SHA. No unrelated dependency version changed.
The README's active setup pin is also corrected. Historical R1 evidence and
its original report remain byte-for-byte unchanged; their old SHA is evidence
of what R1 actually tested, not an active provisioning pin.

All original 50 PR files were reviewed and individually classified in the
retained `scope-review.json` inside `docs/evidence/task108r2/validation.json`.
No safe file removal was found:

* Runtime source/spec and GPR import provide verified WSS with one AHT owner.
* Existing transport drivers need the AHT checkout on GPR_PROJECT_PATH after
  importing its library project; reverting them breaks inherited consumers.
* Validators and their tests must identify the dependency live drivers use.
* Existing CI consumers now link AHT's TLS library and require libssl-dev.
* WSS fixtures and the TLS terminator provide real security/control/backpressure
  regression evidence, not mock-only capability claims.
* R1 raw receipts, failure history, GPR closure and input hashes remain necessary
  provenance. Removing them to reduce file count would weaken the submission.

No unrelated historical script was changed merely for consistency. New R2
evidence retains command/stream/exit and source-manifest artifacts, excluding
private keys, provider caches and binaries. Complete raw build/source trees
remain at `/home/zboll/task108r2-scratch`.

## Corrected-AHT focused WSS qualification

| Compiler | TLS backend | Focused campaign | Real generated Ada/Sleet |
| --- | --- | --- | --- |
| GNAT 14.2.0 | OpenSSL | PASS | PASS |
| GNAT 14.2.0 | mbedTLS | PASS | PASS |
| GNAT 16.1.0 | OpenSSL | PASS | PASS |
| GNAT 16.1.0 | mbedTLS | PASS | PASS |

Each cell executes the existing TASK108R1 driver without redesign:

* Six direct API cases: trusted TLS/idle, deaf peer, natural backpressure and
  exact resumed-byte accounting, untrusted certificate, hostname mismatch,
  and unauthenticated truncation rejection.
* Thirteen Poll/security cases: partial TLS input, batched records, idle Poll,
  maximum Ping/Pong, Close, forced EAGAIN and three-byte ciphertext sends,
  ordered two-Pong/Close burst, deaf send failure, protocol/trust/identity/
  truncation rejection, and failed-handshake cleanup.
* Engine-generated control output tested directly and through **actual OMS
  Poll with zero application plaintext**.
* Thirty-seven inherited transport/generated-consumer cases over verified WSS,
  including bad handshakes/frames, bounds, UTF-8, output failure, peer Close,
  30 resource lifecycles and exact zero FD/task growth.
* Real pinned unmodified Sleet, through the existing test-only TLS terminator,
  with generated typed Ada handlers. All four print
  `TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1`.

Natural backpressure resumes 1,802,240 bytes with position-pattern checking and
SHA-256 `eeeb364451b2f57090ad96249089abf48367f782566d3859094d49b0334e2d8b`.
Trusted localhost TLS validates the original DNS identity. No successful OMS
test uses an insecure connection. TLS-generated control output and staged
ciphertext produce no observed regression with corrected AHT.

The GNAT 16/OpenSSL accepted campaign is `wss-16-openssl-rerun`. The initial
batch wrapper timed out and interrupted one incomplete campaign; its partial
artifacts are not success evidence. Fresh complete rerun passed. Two inherited
campaign first attempts used system Python without websockets; reruns use the
already-provisioned `websockets==15.0.1` virtual environment. These environment/
launch failures remain explicitly retained, not hidden or treated as defects.

### Reproduction

For each compiler `C` in `14 16` and backend `B` in `openssl mbedtls`, use a fresh
scratch path with this existing driver:

```sh
python3 -B /home/zboll/git/ams-gra-codegen-oms-task108r1/scripts/task108/qualify.py \
  --scratch /home/zboll/task108r2-scratch/new-wss-16-openssl \
  --setup /home/zboll/task106-scratch/setup16 --compiler 16 --backend openssl \
  --aht /home/zboll/git/aht-tls-003 \
  --codec /home/zboll/task106-scratch/deps/websocket-ada \
  --generator /home/zboll/task106-scratch/target/release/ams-gra-codegen-oms \
  --sleet /home/zboll/task106-scratch/sleet
```

The dependency checkout stays clean; committed source archives are built in
isolated writable trees. Source manifests and generator hash are retained.
The generator is the existing TASK106 binary, not a claimed new exact-HEAD
generator build; no generator source changed in this task. GNATCOLL 26, builder,
codec and Sleet versions remain unchanged. Platform artifacts record OpenSSL
3.5.7, mbedTLS 3.6.7 source provisioning, and exact tool hashes.

## Inherited Ada compatibility and validation

The full inherited Task100 contract was rerun rather than relying solely on a
representative cell:

| Campaign | Compiler | Result |
| --- | --- | --- |
| Task100 full plaintext | 14.2, 16.1 | 24 cells/compiler; 1,044 scenarios/compiler; 6 Sleet sessions/compiler |
| Task101 authentic DLZ, UCI 2.5/2.6 | 14.2 | 12 cells, real pinned Sleet |
| Task102 authentic Log, UCI 2.5/2.6 | 16.1 | 12 cells, real pinned Sleet; constrained-binary observations retained |
| Task103 authentic RDMA, UCI 2.5/2.6 | 16.1 | 12 cells, real pinned Sleet |
| Task104 authentic Survivability, UCI 2.5 | 14.2 | 6 cells, real pinned Sleet; separate 2.6 static inventory |
| Task105 authentic Survivability, UCI 2.6 | 14.2 | 6 cells, real pinned Sleet |
| Task106 heterogeneous Log/RDMA/Survivability | 16.1 | 6 cells, one real Sleet connection per cell; independent codec negatives |
| Task107 generated heterogeneous dispatch | 14.2, 16.1 | 6 cells/compiler, 64 checks/cell, 2 directionality rejections/compiler |

The applicable unchanged evidence validators were run against fresh live data
with explicit **local** identities (`run_id=local-task108r2`), not fabricated
GitHub run receipts. All Task100–106 validators pass, including source parity,
dependency pins/cleanliness, compiler/toolchain identities, original model/codec
parity, required negative compile outcomes, handler recovery and cleanup.

Additional local results:

* Changed-path router/executor regression tests: 34 PASS.
* Evidence tests: 52 run, 48 PASS, 4 documented external-fixture SKIPs.
* Task101 contract tests: 5 run, 4 PASS, 1 external-fixture SKIP.
* Main Ada packing tests: 19 PASS; main lane-allocation tests: 2 PASS.
* Existing `scripts/check-ci-actionlint.sh`: PASS with main's precise existing
  queue:max compatibility filter; no new suppression or workflow policy change.
* Exact changed-path route is conservatively all backends because workflows
  changed. No local unrelated language campaign was launched. Hosted PR smoke
  follows the existing routing policy.
* Dependency commit/tree verified locally and through GitHub commit API.
* `git diff --check`: PASS.

## AHT read-only upstream minimality assessment

The assessment lives **outside** the OMS PR source tree:
`/home/zboll/task108r2-scratch/aht-upstream-minimality.md`.
Its SHA-256 and exact source identities are in the R2 evidence manifest.

**Conclusion: `MINOR_SCOPE_REDUCTION_RECOMMENDED`.**

All five requested production files were reviewed hunk-by-hunk and classified.
No UNRELATED production feature was found. Staged ciphertext, staged-first
ordering, pending accounting, bounded acceptance, fail-closed poisoning and the
corrected Held/Fail_Output lock cleanup are REQUIRED/SAFETY. All three TLS output
APIs are used by OMS; numeric Pending_Output also enforces its output bound.

Blocking Read/Write signatures, deadlines, close protocol, trust, SNI and
handshake configuration are preserved, but **allocation-path identity is not**:
shared blocking drain and Close now stage even blocking-only output. A smaller
behavioral change could drain an existing staged prefix first, then retain the
old direct blocking engine-extraction path. Do not remove staged accounting,
mixed-call ordering, allocation-failure cleanup or fault tests to reduce lines.

The assessment recommends the smallest test-only case 18 correction: replace
single-shot/fixed-50-ms delivery accounting with a deadline-bounded repeated
Drain_Peer helper at both byte-count comparisons; keep exact count and sequence
assertions. It also identifies a misleading mbedTLS WANT_WRITE callback comment.
No fork patch is made here. Output is appropriately stacked on the exact receive
PR HEAD and isolated from that receive delta; a later upstream contribution must
use the correct accepted base and be reviewed independently.

## Limits and publication status

**Needs_Read runtime coverage remains NOT TESTED** on either backend. No real
fixture reached it. Source handling retains Count=0 and bounded nonrecursive
progress; that is not a runtime coverage claim. Also not claimed: CA_PEM
integration, system-store/public Internet trust success, IPv6 URL support,
other OSes, arbitrary renegotiation, or authentic real-UCI WSS (the WSS Sleet
schema is the existing generated Task100 fixture).

No new AHT full suite, macOS, allocation-fault injection or sanitizer run is
claimed; the AHT safety audit is source-backed plus downstream execution. No
unrelated Rust/Python/C++ or full-main local/dispatch campaign was launched.

PR #183 remains the publication target. Exact-final-HEAD hosted PR smoke and
trusted signal are checked after pushing and reported separately, accurately
distinguishing pending checks from completed signals. A trusted lifecycle signal
is not itself a claim of completed full product qualification. No self-approval,
merge, auto-merge, force push or upstream AHT submission is authorized or done.