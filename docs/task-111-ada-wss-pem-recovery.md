# Task111 — verified in-memory WSS trust and same-client recovery

## Identity and scope

Freshly fetched `origin/main`: `ebb25de9c15f7dd26ce2ce319bc800191d7d9cfc`,
tree `735ab7537274d5851598f464359cea1c68a80130`. This includes merged
Task108–110, including Task108R1's production TLS pin and Task110's full UCI 2.6
multi-message qualification. Branch `feature/111-ada-wss-pem-recovery`, worktree
`/home/zboll/git/ams-gra-codegen-oms-task111`.

**No production changes.** Public signatures, generated Ada, codecs, schemas,
dependencies, other backends and historical consumers/scripts/receipts are
unchanged. The smoke router only adds Task111 Ada ownership and its offline
tests; no lane allocation, workflow or main-only Deep CI policy changes.
Because the router itself changes, the full PR conservatively selects all four
existing backend smokes.

The four final live cells ran from clean commit
`ab5c415e562b529ea49cbb56747ad701e2fb60e7`, tree
`b992b58814d8b644f1f98f25899cadf817017036`. Subsequent changes only correct/test
receipt validation and add reports. Receipts verify every recorded input blob
against that campaign commit. Final publication HEAD/tree and hosted smoke
status are reported separately in delivery/PR records to avoid self-reference.

## Immutable authorities

All five dependency checkouts were clean before and after each final cell.
Builds use committed archives and isolated GNATCOLL/provider objects, never
build in dependency source trees.

| Dependency | Commit |
| --- | --- |
| AHT | `827b0c17dd964e935f8e88912ce0611ee92b6f16` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |

The four existing schema hashes are checked in every cell and recorded in
receipts. UCI25 definitions/security: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` /
`4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b`.
UCI26 definitions/security: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` /
`ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88`.
The production generator was freshly built using `cargo build --locked --release`.
Python uses the existing `websockets==15.0.1`; Memcheck is 3.24.0, unpacked
outside the repository without system installation.

## Trust and lifecycle tests

`/home/zboll/git/ams-gra-codegen-oms-task111/tests/ada-wss-trust/trust_probe.adb`
uses the actual production `OMS_LACAL.Transports.WebSocket.Client`. A single
object lives across all 147 attempts. `Test_PEM.Read` reads exact file bytes
into an Ada string, passed as `CA_PEM` with empty `CA_Path`; the production TLS
API never receives a filename in the PEM argument. No insecure mode or system
trust-store installation is used.

The self-signed server trust anchor has **DNS:localhost only**, not an IP SAN.
The unrelated identity/key is independently generated as `unrelated.invalid`.
The hostname control uses `127.0.0.1`, reaching the same loopback listener
without relying on DNS aliases or a certificate that accidentally includes it.
Public fingerprints, SANs, trust source, URL and TLS version are in each receipt.
Private keys and full transcripts remain outside the repository.

| Compiler | OpenSSL | mbedTLS |
| --- | --- | --- |
| GNAT 14.2.0 | **PASS**, TLS 1.3, 147 attempts | **PASS**, TLS 1.2, 147 attempts |
| GNAT 16.1.0 | **PASS**, TLS 1.3, 147 attempts, first vertical | **PASS**, TLS 1.2, 147 attempts |

Every positive connection negotiates exactly `owp`, sends `INIT {}`, receives
`INFO {}`, performs normal WebSocket Close code 1000/echo, and observes EOF.
CA_Path runs both during warm-up and after all recovery cycles in every cell.

| Independent negative case | Observed result in all four cells |
| --- | --- |
| Unrelated CA PEM | TLS verification rejection, no application data |
| Malformed PEM | Configuration/parse rejection, TCP EOF before ClientHello |
| Correct CA, wrong hostname | TLS identity rejection, no application data |
| CA_Path plus CA_PEM | Configuration rejection, no network connection |
| PEM with `ws://` | Configuration rejection, no network connection |
| Server closes during handshake | Fails after observed TLS ClientHello, no upgrade/application |
| Verified TLS, invalid WS protocol in upgrade | TLS succeeds; non-`owp` upgrade rejected; EOF, no application |

The Ada probe catches only `Transport_Error` around Open; unexpected exceptions
are harness failures. The peer independently distinguishes TLS alert/EOF,
pre-TLS configuration failure and verified-TLS upgrade rejection. Exact backend
exception strings are evidence, not assertions. Failed Open leaves empty
`Protocol`, `Poll = Peer_Closed`, and `Send("INIT {}")` rejected. Cleanup is
checked **before Close** except the separately named explicit-Close control.
No plaintext fallback occurs: network trust attempts must begin with TLS
ClientHello; configuration controls cannot emit plaintext traffic.

After 12 warm-up attempts the baseline is **5 descriptors, 1 Linux task**
(the `/proc/self/fd` count includes the counting directory descriptor).
Every subsequent closed-state count equals baseline. Each independent negative
case is followed by a successful PEM exchange on the same object. Then 20
cycles execute both `untrusted -> valid -> valid` and
`untrusted -> Close -> Close -> valid -> valid`. No reconnect/replay is added.

The pinned AHT DNS helper uses a detached pthread, signals its result before
returning, and releases its own job reference. Instantaneous task counts can
race its final exit after very fast malformed-PEM failure. The probe records
initial/final counts and permits **at most 250 ms** to regain baseline, failing
on persistent growth. Two final GNAT14 observations resolved in approximately
4 and 12 ms. Descriptors must match immediately.

All final normal-run cases finished within **0.075 seconds**. Probe Open budget
is 8 seconds, application exchange 3 seconds, controller response 12 seconds,
peer I/O 5 seconds, process campaigns 300 seconds. Any timeout is a harness
failure, never an expected security rejection.

Both GNAT16 backend cells additionally repeated all 147 attempts under Memcheck:
**zero errors; zero definitely, indirectly or possibly lost bytes**, with
3,694 bytes/5 blocks still reachable at process exit in each backend. This is
observational qualification, not a formal proof against every allocation fault.

## Authentic UCI integration

Opt-in scratch adaptation replaces only the historical consumer's Open trust
argument and adds the exact-byte PEM reader. An offline inverse test requires
byte-for-byte recovery of each original consumer. Historical Task109/110
sources and receipts are untouched. The unchanged Task109 TLS relay forwards
bytes without manufacturing OWP data. Sleet, independent corpus, generated
codec hashes, payload expectations and broker authority remain unchanged.

| Cell | Full sequence | Outcome |
| --- | --- | --- |
| GNAT16/OpenSSL, UCI 2.5 | 62 steps | **PASS**, TLS 1.3 |
| GNAT16/OpenSSL, UCI 2.6 | 62 steps | **PASS**, TLS 1.3 |
| GNAT16/mbedTLS, UCI 2.6 | 62 steps | **PASS**, TLS 1.2 |

Every sequence: Log/RDMA/Survivability callbacks **17/19/19**, five expected
decode errors, 61 independent witness deliveries, 60 actual Ada WSS deliveries,
three subscriptions on one connection, zero consumer/broker exits and successful
cleanup. Broker rejection counts remain 39 (2.5) and 48 (2.6). Existing Task109
and Task110 verifiers independently passed; all generated-source pins matched.

## Other validation and limitations

* Unchanged Task108 GNAT16/OpenSSL and GNAT16/mbedTLS full safety campaigns:
  **PASS**, including each backend's 37 WSS regressions, framing/control,
  failed-handshake cleanup, bounded backpressure and real Sleet/CA_Path.
* Task111 helper/mutation tests: **10 passed**; smoke selector: **36 passed**.
* Unchanged Task109 helpers: **5 passed**; Task110 helpers: **12 passed**.
* Inherited Task101–106 discovery: **48 tests, 43 passed, 5 explicit opt-in
  disposable-UID ownership fixtures skipped**, not presented as passes.
* `git diff --check`: passed. No historical evidence modified.
* `Needs_Read` runtime coverage remains **not exercised**. Linux-only resource
  observation, local fixtures and the requested compiler/backend cells do not
  qualify certificate rotation, automatic reconnect, remote DNS failure, or
  every allocator fault. These are deliberately outside scope.

Initial harness diagnostics are recorded separately in `validation.json`:
malformed-DER timing classification, detached DNS exit measurement, and the
receipt's warm-up FD handling. None required a production correction. All four
final live cells completed from clean source; reporting fixes do not reinterpret
an unexpected transport failure as a pass.

## Reproduction and evidence

External records: `/home/zboll/task111-evidence`. Committed compact receipts:
`/home/zboll/git/ams-gra-codegen-oms-task111/docs/evidence/task111`.
Receipts include command/artifact hashes, dependency commit/tree identities,
public certificate identity, negative observations, resource counts, integration
results and Memcheck summaries. Run the external verifier with:

```sh
cd /home/zboll/git/ams-gra-codegen-oms-task111
PYTHONDONTWRITEBYTECODE=1 python3 /home/zboll/git/ams-gra-codegen-oms-task111/scripts/task111/receipt.py \
  --verify /home/zboll/git/ams-gra-codegen-oms-task111/docs/evidence/task111/final16-openssl.json
```

Each receipt requires its external directory. To reproduce a new clean-source
cell (use a **new** scratch directory), use existing exact Task096 setups and
the five pinned clean dependency checkouts:

```sh
/home/zboll/task110-evidence/venv/bin/python \
  /home/zboll/git/ams-gra-codegen-oms-task111/scripts/task111/qualify.py \
  --scratch /home/zboll/task111-evidence/reproduce16-openssl \
  --setup /home/zboll/task110-evidence/setup16 \
  --deps /home/zboll/task110-evidence/deps \
  --schema25 /home/zboll/task110-evidence/deps/uci25/OAC-STD-UCI_V2.5 \
  --schema26 /home/zboll/task110-evidence/deps/uci26.extracted \
  --generator /home/zboll/task111-evidence/target/release/ams-gra-codegen-oms \
  --compiler 16 --backend openssl --integration 25 26
```

Use compiler 14/setup14 for GNAT14, backend `mbedtls` for mbedTLS; only the
GNAT16/mbedTLS cell needs `--integration 26`. The optional `--memcheck` and
`--memcheck-lib` arguments identify the external executable and its runtime
directory. Exact executed commands, not just this example, are hash-recorded.