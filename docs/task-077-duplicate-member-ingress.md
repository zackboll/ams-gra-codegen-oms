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
and its marker for each control. The corrected Task 077A shell mock checks
targeted registration, exit-status, marker and execution-count failures (see below).
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
Task 070's exact gate executed 10 controls. The original wrapper suite reported
5 passing negative cases, but Task 077A reproduced their masking defect; those
results do not establish the corrected protections. Split mutation controls
passed 155 checks. The focused Task 077
live test separately executed successfully, so its offline ignored result is
not substituted for live evidence. Explicit live execution without a binary
was also verified to fail. No unrelated UCI/XML/backend campaign was repeated.

### Task 077A — wrapper false-pass correction

Reviewed parent: `ac70eb5ce89686652f1664fb67136ca91ed22f3c`.
The real Fast gate remains byte-identical, blob
`fc25a82a012f58337f16a9c71684f0b39f946d8d`. Runtime sources, tests, manifests,
lockfile, workflows and CI-split protections are unchanged by this correction.

Before editing, the committed negative suite (blob
`104708db502b4a7b9eb6f380d5e75c08e1850dae`) exited **0** against the original
gate and disposable marker-only, count-only and both-assertions-removed mutants.
Its mock advertised only the config test. Even a fault-free mock run exited
**1** at invocation 3: the second registration listed only the config test.
Missing-marker and ignored-only faults against the corresponding weakened gates
also passed their intended invocation 2 and failed at unrelated invocation 3.
Thus the original suite's success was not proof of marker/count protection.

The revised driver uses `task077-mock-cargo.sh`, a small shell-only helper which
validates exact Cargo argv, package, lib/integration suite, phase and requested
name against an independently specified six-control inventory. Healthy listings
advertise the applicable suite's tests; exact executions emit the matching
marker, test name and passing summary. Unknown/malformed invocations are explicit
mock-contract failures, never credited as an intended negative. Fresh scenario
traces and fault logs are compared with an independent expected sequence. Faults
are injected only at a selected control/phase. No real gate file is mutated.

The healthy control uses this same infrastructure: exit **0**, all six
registrations and six exact executions in order (**12 Cargo invocations**), no
injection, and the final `TASK077 FAST: PASSED` marker. Fourteen targeted negatives
exercise seven categories at both control 0 (config) and control 5 (facade codec
comparison): missing registration, failed listing, failed execution, absent
marker, ignored-only summary, zero-executed summary, duplicate registration.
Listing faults stop at invocation **1/11**, without executing the target;
execution faults stop at **2/12**, after successful target registration. Exact
trace prefixes prohibit later invocations. Injection identity and absent final
success marker are required. Nonzero commands emit plausible success output but
return **7/9**, and those statuses must propagate. Marker faults alone omit the
marker; ignored/zero faults alone change the summary and keep the marker.

The no-argument CI entry point and final wrapper marker are preserved. Internal
`--check-only GATE` runs the same healthy/negative checker against a supplied
disposable gate. Each mutant's diff must contain only its selected exact
assertion line(s); all other bytes, including EOF, are preserved.

| Disposable mutant | Healthy path | Targeted faulty path (control 5) | Revised checker |
| --- | --- | --- | --- |
| Remove marker assertion only | 0, all 12 invocations | Missing marker incorrectly exits 0 | Nonzero at missing-marker control 0 |
| Remove count assertion only | 0, all 12 invocations | Ignored-only incorrectly exits 0 | Nonzero at ignored-only control 0 |
| Remove both assertions | 0, all 12 invocations | Missing marker and ignored-only each incorrectly exit 0 | Nonzero at missing-marker control 0 |

The outer mutation qualification requires these checker failures with the exact
regression diagnostic, not arbitrary failure. It demonstrates execution of each
supplied mutant, rather than merely searching for deleted source checks.

Per complete suite: **1 primary healthy + 14 attributed negatives**, **3 mutant
healthy paths + 4 direct faulty passes**, and **3 recursive checker runs**. The
recursive checkers execute 16 gate scenarios before the expected regression
failures, giving **38 total synthetic gate executions**. The complete suite ran
twice with byte-identical normalized output. All 14 primary rejection points and
all 3 mutation qualifications passed. These mock invocations execute no Rust.

Focused actual qualification separately runs the unchanged six-control Fast gate
with real Cargo `1.95.0 (f2d3ce0bd 2026-03-21)` and rustc `1.95.0
(59807616e 2026-04-14)`, locked dependencies and task-specific build output.
Fake Cargo PATH and injection variables are scoped to child processes and cleared
before the real run. Shell syntax, existing CI-split guard/155 mutation checks,
and `git diff --check` pass. No live Sleet, UCI, XML or broad backend campaign is
repeated for this shell correction. Corrective SHA and exact-head hosted status
are recorded in PR #77's follow-up report, separately from original-head evidence.

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