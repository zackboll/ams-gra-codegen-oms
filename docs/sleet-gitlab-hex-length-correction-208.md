# PR #208 downstream qualification correction receipt

This is a new receipt, not a replacement for historical executions. Review and
adoption remain separate: PR #208 stays open, non-draft and unmerged.

## Executed source

* Reviewed head: `7b15692a76815046b374eaf87a00ca669ec9edc9`.
* Correction commit: `8de80922377c8f7c5857cad13d60a4be8f37e4c4`.
* Correction tree: `7af5fa1fadba478e6fa934b1577a610ace3be2f6`.
* Sleet unchanged: `https://gitlab.com/zackboll/sleet.git`, commit
  `94796daf55ebd1134dce0925e6a204a05ad3423b`, tree
  `47da595b9078cac1a9351e8290d1ae7f3976367f`.

Tests ran on the correction inputs; recorded input hashes were checked against
the committed bytes. Subsequent receipt/documentation changes do not substitute
their commit identity into those executions. No production runtime, schema,
authentic fixture, dependency manifest or lockfile changed in this correction.

The Log and Ada executions used broker SHA-256
`c163edba50725f9d95f70a339c003fc76ca41da2c5572a30b32c8c8662e83d8a`.
The focused OrdersMetadata runner rebuilt the same clean candidate with its own
Rust 1.95.0 toolchain/lockfile, inheriting
`CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3`; its reported binary SHA-256
is `14d49d5989940b7513cfdceaf7eb857aaafa6faedea7da503f827e3179fb1f5c`.
The changed build artifact digest is recorded, not hidden or treated as a new
dependency pin. The separate OrdersMetadata consumer used that rebuilt binary.

## Restored and reconciled coverage

* Rust Log: all 12 rejected documents per release retain broker rejection. Each
  is followed by valid publication, raw MSG equality, and inspected typed payload
  plus exact `00 80 FF` bytes on the original subscription. Invalid-topic PUB
  is client-side `InvalidInput`; unauthorized-topic PUB/SUB are broker errors.
  Unsubscribe, no callback after unsubscribe and cleanup remain mandatory.
* Standalone Rust DLZ: malformed time must fail at the broker, without malformed
  raw MSG, typed delivery or fabricated decoder error. Valid barrier/recovery
  messages must remain on the original subscriptions/session, without reconnect,
  then terminate normally and clean up. These later assertions are not claimed
  executed when the valid initial DLZ publication fails.
* Ada DLZ: the wire driver independently subscribes to witness no malformed MSG;
  the typed consumer requires zero decode errors, original active SID, valid
  recovery and unchanged handler/witness/cleanup obligations. Current verifier
  counts are 11 broker rejections, zero decoder rejections, 11 primary handlers,
  one witness and 12 valid wire deliveries for a successful complete cell.
  Verifier adversaries reject fabricated decoder errors, extra wire/typed
  deliveries and missing negative/recovery observations. Current input hashes
  are reconciled additively; historical hashes/receipts are untouched.
* OrdersMetadata control fixture now stubs `rustc --version`; the real runner
  still prints compiler identity and fingerprints the broker. All intended
  fail-closed decisions, including exit 23 and its diagnostic, are exercised.

## Focused executions

Raw evidence and commands: `/home/zboll/sleet-gitlab-update-evidence/correction-208`.
The machine-readable `correction-receipt.json` records input and transcript
SHA-256 values, exact exits and separate broker artifacts.

| Execution | Result | Limits |
|---|---|---|
| Rust authentic Log, UCI 2.5/2.6 | 4 passed, zero ignored | 24 negative/recovery pairs and all PUB/SUB probes executed |
| Rust generated hex codec / time codec / mock OWP | 7 + 1 + 12 passed, zero ignored | Independent decoder rejection and existing valid temporal/whitespace inputs preserved |
| Rust authentic DLZ codec, both releases | 2 passed, zero ignored, 3 filtered | Broker tests intentionally not part of this codec-only invocation |
| OrdersMetadata runner controls | 6 passed, no skips | Includes preserved exit 23 and exact diagnostic |
| Ada Task101 verifier controls | 4 passed, 1 skipped | Optional disposable UID fixture not enabled; not a transport pass |
| Ada deep-pack controls | 20 passed, no skips | Current hashes plus additive source reconciliation |
| Standalone Rust all-target check, both releases | PASS | Final source compiles |
| Standalone Rust campaign, both releases | FAIL, exit 101, 0 passed / 1 failed / 0 ignored / 5 filtered each | Initial valid DLZ MSG absent; raw observer times out before corrected malformed-time section |
| Ada GNAT16 Normal/O0 DLZ, both releases | FAIL, exit 1 each | Correct `not-a-timeZ` rejection and no-MSG witness reached; unchanged valid recovery then rejected at `24:00:00.000Z`; no completed campaign/cleanup claimed |
| Independent Ada DLZ codec, both releases | PASS, 26 cases each | 10 accepted, 16 rejected, retaining midnight and second-60 lexical profile |
| Focused OrdersMetadata runner | FAIL, exit 101: 3 passed, 2 failed, zero ignored | Valid typed publish/independent receipt pass both releases; obsolete `not-a-dateZ` broker-admission expectation fails both; later cleanup not reached |
| Separate OrdersMetadata Cargo consumer | PASS, 1 outer test, zero ignored | Both releases; test, all-target check and Clippy succeed |
| Formatting, Python syntax, whitespace checks | PASS | Workspace and standalone formatting |

The runner command was
`AMS_GRA_RUST_ORDERS_METADATA_ONLY=1 bash /home/zboll/git/oms-sleet-gitlab-update/scripts/run-real-sleet-test.sh /home/zboll/git/sleet-gitlab-hex-length`.
Its first failing binary prevents Cargo from running the consumer, so the latter
was explicitly run separately with `--test real_orders_metadata_consumer`.
The outstanding OrdersMetadata result is now known, not a still-running pass:
it needs a separate malformed-date boundary reconciliation. This is not another
upstream defect and is not silently fixed by weakening its input.

Initial disposable Ada setup attempts failed due to omitted dependency exports,
then an input-only rather than bidirectional generated contract. Those attempts
are retained as failures; corrected setup compiled the actual revised consumer
and reached the candidate rejection reported above. Earlier standalone attempts
precede a final assertion refinement; both final-source campaigns were rerun and
remain failures. No aborted/running attempt is counted as passing.

## Compatibility and adoption disposition

* **Confirmed upstream defects:** hour-24 rejection and missing hexBinary
  XML-whitespace normalization. Existing failing acceptance inputs remain.
* **Profile incompatibility:** second-60 rejection conflicts with OMS's documented
  XSD 1.0 lexical profile; upstream profile reconciliation is required. Keep
  `23:59:60Z` and `2000-02-29T23:59:60+14:00` and public OMS behavior unchanged.
* Fresh isolated diagnostics reproduce malformed-time, hour-24 and second-60
  rejections for both releases. The isolated second-60 diagnostic changes only
  its local probe's hour-24 field to avoid masking; repository fixtures and
  full acceptance campaigns are unchanged. It is not a campaign pass.
* Upstream MR !2 still reports `opened`, selected SHA unchanged. Accepted
  canonical dependency revision and affected reruns remain mandatory.
* Full temporal, OrdersMetadata and main-only qualification remain incomplete.
  Unaffected Python/C++ matrices were not rerun or relabeled for this correction.
  PR #206's repair and PR #198's published branch are untouched and separate.

## Hosted status at correction-code publication

GitHub confirms PR #208 head `8de80922377c8f7c5857cad13d60a4be8f37e4c4`.
No check runs or Actions runs are attached to that head. GitHub reports
`mergeable=false`, `mergeable_state=dirty` (conflict with the advanced base);
pull-request workflows do not run on a conflicting merge. Earlier green checks
belong to reviewed head `7b15692`, not this correction. No rebase, force push or
unrelated conflict resolution was performed. The receipt-only final head and its
hosted status are recorded separately in the PR description and local receipt.