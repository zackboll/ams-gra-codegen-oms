# Sleet GitLab hexBinary dependency qualification

## Status: review preparation, not ready to merge

The upstream [MR !2](https://gitlab.com/open-arsenal/ams-gra/hello-world-sk/infra/sleet/-/merge_requests/2)
was open when checked. This preparation branch selects the reviewed fork
`https://gitlab.com/zackboll/sleet.git`, commit
`94796daf55ebd1134dce0925e6a204a05ad3423b`, tree
`47da595b9078cac1a9351e8290d1ae7f3976367f`.
The canonical repository is
`https://gitlab.com/open-arsenal/ams-gra/hello-world-sk/infra/sleet.git`.
Before merge, select the exact accepted canonical correction and rerun affected
qualification. Do not merge this candidate pin merely because hexBinary passes.

## Scope and identity

Fetched OMS base: `01694bae47c3277e7482aed96255e2f1d6c3d059`.
The source was built clean, unmodified, with its own Rust 1.95.0 toolchain and
`cargo build --release --locked -p sleet`. Main broker binary SHA-256:
`c163edba50725f9d95f70a339c003fc76ca41da2c5572a30b32c8c8662e83d8a`.
The Rust runner rejects wrong existing checkouts instead of silently checking
out a different broker. It verifies commit, tree and clean status before and
after build and prints toolchain, binary path and digest. C++ drivers also
record the actual built binary digest. Active Python and Ada identity verifiers
select the same candidate; archived receipts and frozen inventories are not
rewritten as if earlier runs used this dependency.

All three active Rust locks (workspace, standalone example, independent
OrdersMetadata consumer) change only sleet-client/sleet-types source identities;
all other package entries are byte-for-value unchanged.

## Required behavior

* Exact and ranged hexBinary lengths count octets, not hexadecimal characters.
* Authentic FileMetadata SHA-256 is 32 octets (64 hex digits), in both UCI 2.5
  and 2.6. Both raw OWP observation and independently inspected typed bytes are
  required; a successful socket write is insufficient evidence.
* 16-, 31-, and 33-octet hashes, malformed digits and odd lengths fail at the
  broker before delivery. Each rejection is followed by valid traffic on the
  original subscription, with no decoder error misattributed to broker rejection.
* Independent generated-codec and mock-OWP rejection tests remain mandatory.
  A validating broker need not forward invalid data to exercise the decoder.
* Existing unsubscribe, connection, worker and process cleanup assertions remain.

The selected source also changes handshake negotiation (offers `owp, OWP`,
requires an OWP response and adds typed `ClientError::Handshake`), Clark-name
handling, temporal lexical validation and numeric facets. OMS production error
mapping remains unchanged; the new handshake error is tested as non-retryable.
The standalone integration cases run separately so one failing family does not
hide FileMetadata boundary evidence or subsequent recovery/WSS attempts.

## Authentic input provenance

Both roots and their SecurityMarkings includes were checksum-verified, unchanged:

| UCI | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 |
|---|---|---|
| 2.5 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| 2.6 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |

## Qualification

The final PR receipt records exact OMS commit/tree and per-language results.
Raw local evidence is retained at `/home/zboll/sleet-gitlab-update-evidence`.

Confirmed upstream defect (not a CI #206 attribution): the candidate
rejects the existing valid `24:00:00.000Z` xs:time in
`DLZ.MessageData.DLZ_Data.TimeToRelease` with `FacetViolation`,
`lexicalRepresentation`. Existing DLZ fixtures and public codec acceptance are
not weakened to hide it. Standalone four/five-family transport therefore remains
blocked until upstream compatibility is resolved.

PR #206 was still open on the latest check; its worker-lifecycle observation
and build-budget repair are not folded into this dependency change. Distinguish
those independent source/environment issues from candidate broker rejection.

## Disposable PR #198 integration

PR #198 head `00b4d0ffec54b37f85fadd01a291989308df66c3` (tree
`f8fcdfa25fb5310f19fe12b67d44cb7ab13838be`) was combined with the fetched
OMS base and dependency changes in `/home/zboll/git/oms-sleet-pr198-integration`.
Its original branch is unchanged. The existing source-rebuild driver runs both
UCI releases under g++ and clang++ strict, ASan+UBSan and TSan configurations,
using this exact GitLab candidate, not the superseded GitHub correction.
This is integration evidence only, not approval or a merge of PR #198.

## Recorded qualification results

Evidence receipts retain the execution commit/tree; later configuration-only
and documentation commits are not retroactively substituted into earlier runs.

| Qualification | UCI 2.5 | UCI 2.6 |
|---|---|---|
| Rust authentic FileMetadata + independent Cargo consumer | PASS | PASS |
| Rust standalone 32-octet hash/broker policy/cleanup boundary | PASS | PASS |
| Rust real Log and RDMA | PASS | PASS |
| Rust synthetic codec, mock OWP, generated decoder negatives | PASS (40 tests across 11 binaries; release-independent) | same suite |
| Python registered Sleet (12 per policy), authentic Log/RDMA included | PASS normal/optimized | PASS normal/optimized |
| Python registered fast (43 per policy), owned Sleet (12) | PASS | PASS |
| Ada recovery GNAT 14/16, OpenSSL/mbedTLS | PASS all four configurations | PASS all four configurations |
| Ada authentic Log | FAIL valid XML whitespace around hexBinary | FAIL same valid lexical input |
| C++ authentic Log, GCC/Clang strict + GCC ASan/UBSan | PASS | PASS |
| C++ PositionReport and AMTI, both compilers strict/ASan+UBSan/TSan | PASS all six configurations | PASS all six configurations |
| Disposable PR198 authentic FileMetadata, all six configurations | PASS | PASS |

Rust runtime 20 unit tests + doctest, candidate client/types 12 + 56 tests,
workspace runtime/facade all-target release Clippy, standalone Clippy, formatting,
runner controls, and current Ada verifier adversaries pass. The optional
UID-ownership fixtures are explicitly unexecuted (four skips in 43 verifier
tests), not interoperability passes.

Additional candidate findings:

* Confirmed upstream defect: missing XML-whitespace normalization for hexBinary.
  Leading/trailing XML whitespace around authentic Log hexBinary (` \t0080ff\r\n`)
  is rejected lexically in both releases. Ada still requires valid acceptance;
  the test was not changed to accept rejection. Malformed hex never reaches
  the decoder; its exact wire-count verifier now expects only valid messages.
* Second-60 rejection is incompatibility with OMS's documented XSD 1.0 lexical
  profile, requiring upstream profile reconciliation, not the same defect claim
  as hour-24 rejection. Python PY009's existing
  `2000-02-29T23:59:60+14:00` is rejected, normal and optimized; the existing DLZ
  `23:59:60Z` fixture is also preserved. Neither failing input nor OMS's public
  Time/dateTime acceptance is narrowed to accommodate the candidate.
* The standalone Rust and Ada DLZ qualifications now require `not-a-timeZ`
  rejection by the broker with no routed malformed MSG, typed delivery or
  fabricated subscription-decoder error. They require valid recovery on the
  original subscriptions/session, followed by normal unsubscribe and cleanup.
  Independent generated codec and mock-transport negatives retain decoder
  rejection coverage. Full campaign execution remains limited by earlier valid
  midnight/second-60 rejection; corrected assertions not reached are not passes.

The C++ complete source-rebuild PR198 matrix finished with `failures=0` and
`unavailable configurations=0`. Disposable integration identity:
commit `2078668ba96674e06eb30b012e7806fac5ef926e`, tree
`1fc9d5c88aaf6c18890f28e507f24d7d2d752645`. Its rebuilt broker digest equals
the main candidate broker digest above. All 24 authentic transport executions
(two tests per release/configuration), codec consumers, boundary consumers and
five mutation controls were independently counted in its transcript.

A C++ AMTI run started while its shell file was being edited encountered a
shell parser error after all transport cells. It is superseded by the complete,
unchanged-source successful `cpp-amti-final` run, not reported as compatibility.
An Ada recovery run invalidated by source changes was likewise discarded and
rerun cleanly. Such invalidated/partial attempts remain in local evidence.

The PR #208 correction restores valid publication after every rejected Rust Log
document: independent raw MSG equality, typed payload and exact binary bytes on
the original subscription. Invalid-topic PUB remains client-side `InvalidInput`;
unauthorized-topic PUB and SUB remain broker-side errors. No production runtime
changes are needed. The OrdersMetadata runner-control fixture stubs the added
`rustc --version` diagnostic; the actual runner still reports its compiler and
fingerprints the broker, and injected exit 23/diagnostic must survive unchanged.

New correction evidence is recorded separately under
`/home/zboll/sleet-gitlab-update-evidence/correction-208`; historical execution
receipts are unchanged. Completed and incomplete correction runs are reported in
the [new correction receipt](sleet-gitlab-hex-length-correction-208.md), never
inferred from earlier hexBinary success. Restored Log (both releases), 20
independent codec/mock tests, two authentic DLZ codec tests and the runner controls
pass. Focused OrdersMetadata now has a completed failing result: 3 passed, 2
failed, zero ignored, because both broker tests still expect malformed
`not-a-dateZ` admission. That separate downstream expectation gap is not an
upstream defect. Its independent Cargo consumer passes separately, including
both releases, all-target check and Clippy. Full OrdersMetadata cleanup and the
full main-only qualification remain unfinished.
PR #206 is still independent and open; the original worktree is untouched.
