# Task 077 — opt-in duplicate-member rejection at Rust ingress

## Baseline and boundary

Starting source: `65a8f6c9a7d330ce6c915efcb6cf7ef45bc61658` (`origin/main`),
including reviewed Task 076 corrective head
`53d20cd0bc7387628d59533f6a326fe9bfcf303a`. No intervening main advancement
was present when the isolated feature worktree was created. Existing Task 077
branches and all-state PR search were empty. Baseline Fast and Deep checks
were in progress, not failed; no extra workflows were dispatched.

The actual boundary is CalClient's raw `ReceivedMessage.payload` → Worker SID
lookup → subscription dispatch closure → envelope parser →
`OmsJsonCodec::decode_payload` → typed handler. The earliest collapse is
`serde_json::from_str::<Value>` in envelope parsing, not the generated codec.
Historical Task 072/073 controls remain unchanged. The new public-runtime mock
test measures legacy last-member results from raw duplicate JSON; the generated
codec comparison measures known-last acceptance and unknown-last codec failure.

## API and compatibility

`DuplicateMemberPolicy::{LegacyLastWins, Reject}` and
`RuntimeConfig::{with_duplicate_member_policy, duplicate_member_policy}` are
additive. `new` defaults to `LegacyLastWins`; old builders remain valid.
The selected Copy policy is stored explicitly at connect and captured in each
subscription closure. It cannot change during that connected runtime's lifetime.
Independent connections can select different policies. There is no global,
environment-variable policy, per-message fallback, or change to INIT options,
retry configuration, config equality/clone/validation or codec signatures.

Compiling opt-in configuration example (also covered by the config unit test):

```rust
use ams_gra_oms_runtime_rust::{DuplicateMemberPolicy, RuntimeConfig};
use std::time::Duration;

let config = RuntimeConfig::new("ws://127.0.0.1:9000", "my-service", "002.5.0")
    .with_initial_connect_retry(3, Duration::from_millis(50), Duration::from_millis(100))
    .with_duplicate_member_policy(DuplicateMemberPolicy::Reject);
assert_eq!(config.duplicate_member_policy(), DuplicateMemberPolicy::Reject);
// Pass config and your existing codec provider to SleetRuntime::connect.
```

The public `unwrap_global_element` signature and legacy behavior are preserved.
Both paths share the same internal envelope-rule implementation. Reject is an
explicit compatibility choice: RFC 8259 recommends unique names, but duplicates
are not a universal JSON syntax prohibition. Legacy mode is not strict validation;
Reject is not complete CAL/XSD validity.

## Equality, errors, and dispatch

Each object has its own set of **decoded JSON strings**. Literal/escaped `x`
and `$type`/`\u0024type` collide, as do repeated envelope names and identical
duplicate values. Objects nested in arrays and unknown-member subtrees are
visited. Sibling objects and separate array elements may reuse names; repeated
array values are allowed. Case and Unicode normalization remain significant.
Schema-aware QName aliases are not resolved or equated.

On Reject, duplicates produce the dedicated `MessageDecodeError::DuplicateMember`
and existing `RuntimeEvent::SubscriptionDecodeError` with the correct SID and
expected global-element name. Neither codec nor handler runs. The subscription
stays active and subsequent valid traffic succeeds, including another SID.
No reconnect, replay, disconnect, worker scheduling or panic-containment change
occurs. Unknown SIDs and OWP control-frame parsing remain unchanged.

Precedence: the structural pass reports the first encountered syntax/depth
failure or duplicate. An observed duplicate wins before its value or remainder
is parsed, so duplicate rejection does **not** certify remainder syntax. If the
pass succeeds, complete input consumption is required, then existing Value,
envelope and codec errors retain their separate variants and order. No error
classification parses human-readable Serde diagnostics. Duplicate diagnostics
are constant-sized and include no member values or raw payload.

## Parsing and cost

Locked Serde `1.0.229`, serde_json `1.0.151`; direct Serde dependency uses the
already-resolved version. Only the runtime's lockfile dependency edge changes.
Official [Visitor](https://docs.rs/serde/1.0.229/serde/de/trait.Visitor.html) and
[MapAccess](https://docs.rs/serde/1.0.229/serde/de/trait.MapAccess.html) interfaces
were inspected. `DeserializeSeed`/`deserialize_any`, `next_key::<String>`, and
recursive map/sequence seeds implement the structural pass. No handwritten
grammar, regex parser, patched Serde, skipped subtrees or feature change is used.

Reject does two traversals of accepted text. The first stores decoded keys in
randomized hash sets for currently open objects (expected linear key processing,
not quadratic scans); sets are released on leaving each object. It retains no
subtree Values, scalar strings or extra complete payload copies. Additional live
memory is the decoded key bytes/hash-table overhead of active objects plus
library string scratch and depth-bounded visitor stack. Wide objects can still
cost substantial memory; this is not general DoS protection. Existing parser
recursion checks and transport limits remain enabled, with no new broad budgets.

The second pass is the unchanged Value parser. Numbers are not all converted
through f64 to build a Value. Parity controls cover i64/u64 boundaries,
`9007199254740993`, negative zero (also comparing serialized representations),
fractions/exponents, bool/null, Unicode and escapes. The generated codec mock
comparison confirms identical results/failures from duplicate-free Values.
No arbitrary precision or original numeric-token preservation is claimed.

## Evidence and qualification

Local toolchain: `rustc 1.95.0 (59807616e 2026-04-14)`,
commit `59807616e1fa2540724bfbac14d7976d7e4a3860`, x86_64 Linux,
LLVM 22.1.2. Task-specific build output: `/tmp/ams-task077-target`;
qualification logs: `/tmp/ams-task077`. Source under review is the normal feature
commit/PR head, not a modified main or rewritten historical campaign.

Fast controls need no Python, UCI fetch or live broker: config/default/clone/retry,
duplicates, nonduplicates/value parity, syntax/trailing/depth/precedence,
counter-based dispatch/recovery/independent policies, and generated-codec
comparisons. The exact-registration gate requires one executed nonignored test
and its marker for each control. Shell fake-cargo negatives cover zero matches,
failed listing, failed execution, missing marker and ignored-only results.
Fast CI runs both scripts; split guards require both and the focused live check.

Actual-broker evidence is separate: `task077_real_sleet_client_policy` uses the
already-supported synthetic OAM generated codec fixture, not new generator
support. Unmodified Sleet source:
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`; cached verified executable
`/home/zboll/task076-research/sleet-target/release/sleet`, SHA-256
`4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
The checkout's HEAD and clean tracked/untracked status were verified. No second
broker build or Task 076 34-case campaign was run.

Same-connection SUB then matched PUB establishes readiness separately for both
runtimes. A fresh test-only WebSocket publisher sends untouched raw scalar and
escaped discriminator duplicate texts. Default runtime generated-codec delivery
proves broker acceptance, not broker rejection. Reject produces the exact typed
event and unchanged codec/handler counters. Each case is followed by successful
valid delivery to both runtimes; ordinary unsubscribe/close succeeds. The focused
test passed locally (1 executed, 0 ignored). Offline execution ignores this test;
requested execution fails if the binary is missing. The existing single-build
real-Sleet harness explicitly runs it with `--ignored --exact` and checks its
execution count and marker. Deep CI retains that harness and includes the live
test's source path in its trigger filters.

Local and hosted qualification results and exact commit identities are recorded
in the PR/delivery report. Pending hosted checks are not counted as local evidence.

Local qualification passed: Rust 1.95 fmt, locked workspace all-target check and
Clippy (`-D warnings`); runtime 20 unit tests plus 1 compiling API doctest;
facade suite 98 libtest passes (including 12 existing environment-skip stubs:
6 older broker tests and 6 UCI-dependent tests), 2 ignored live campaigns
(Tasks 076/077); Task 072 synthetic guard 3 passes/1 pinned ignored;
Task 073 pinned vertical 1 ignored; generated-support binding 12 passes.
The facade run includes codec/QName, Task 072/073 wire/authority, fairness,
retry, dispatch and lifecycle tests. Task 077's gate executed all 6 controls;
Task 070's exact gate executed 10 controls. Wrapper negatives passed all 5
failure cases; split mutation controls passed 155 checks. The focused Task 077
live test separately executed successfully, so its offline ignored result is
not substituted for live evidence. Explicit live execution without a binary
was also verified to fail. No unrelated UCI/XML/backend campaign was repeated.

## Non-goals and preserved boundaries

No schema validation/private-schema authenticity, publication or forwarding
authorization, unknown-subtype support, expanded-QName collision detection,
raw lexical/byte preservation, numerical-precision remediation, event-queue
backpressure or comprehensive resource containment is provided. No registry,
opaque carrier, quarantine, broker modification or JSON-to-XSD runtime bridge.
`NotClosedUnderOpenExtensions`, GenerationWorld/readiness, generated models,
generator semantics, service traits, checked construction, pinned Sleet/wire
protocol and Tasks 074–076 implementations/frozen findings remain unchanged.
The sole production behavioral difference requires explicit Reject selection.