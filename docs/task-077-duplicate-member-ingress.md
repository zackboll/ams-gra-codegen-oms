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

## Task 077B — bounded complete real-UCI qualification

The reviewed parent is `cf132afc4d4cd45d0511912f5f1d3c18066357ce`.
At inspection, remote feature and PR #77 head matched it; current main was
`a9378b31608a38dd5eb0601e57a8cfd50ace76b0`. PR #77 was open, non-draft,
mergeable, with auto-merge disabled. Main had no branch protection, repository
rulesets or applicable branch rules/required checks; no aggregate check is
needed to preserve a required-check contract.

### Cancellation evidence

Deep run **38194131529**, attempt **1**, job **114633024377** (`real-uci`),
at the reviewed parent, started **2026-10-12T02:01:58Z** and completed
**06:02:13Z**: **4h00m15s**, configured limit **240 minutes**. Retrieved GitHub
check annotation: **"The job has exceeded the maximum execution time of
4h0m0s"**. This is a confirmed whole-job timeout, not an inferred assertion
failure, manual/concurrency cancellation, or infrastructure failure.

Task 060 ran from **05:44:06Z** to cancellation at **06:02:11Z**. The complete
job log shows `uci_alternating_admission` finished at **05:44:27Z**, both final
admission markers present, **2 passed / 0 failed / 0 ignored**. The runner
terminated `uci_alternating_after` and a child CLI during cleanup. Its output
was still captured in command substitution, so no last completed AFTER
subcommand can be identified. No independent assertion failure appears in
this log; that is not proof that AFTER passed. The cancelled campaign remains
**incomplete**. No old combined job was rerun.

All preceding qualification steps succeeded. Original-head Fast run
**38194131482**, `real-sleet` and `msrv-real-uci` succeeded; none certify the
corrected workflow revision.

### Before/after gate inventory

Before: all **17** rows belonged to `real-uci`. After: rows 1–7 remain in
`real-uci`, rows 8–17 belong to `real-uci-regressions`, in their original order.
Every row executes once, not once per job. The comparison checks entire step
dictionaries (including commands, markers and environment), not only names.

Common inputs: verified pinned **UCI 2.5 and 2.6** roots, except Task 071 uses
only 2.5. Both jobs use the existing **stable** Rust toolchain, runner GNAT and
C++ compiler; generated Rust probes use the same checkout and dependency cache.
Task 069 retains its existing debug profile; all other Cargo inventory payloads
retain release profile. Script-specific required-pinned and GNAT variables are
unchanged. Task 054's explicit `AMS_GRA_REQUIRE_GNAT: "1"` remains attached to
its step. Compiler verticals retain their direct GNAT/C++ and Rust commands.

| # / task | Exact script or Cargo command/filter | Execution/count and marker contract | New owner |
| --- | --- | --- | --- |
| 1 / 061 | `bash scripts/check-task061-pinned.sh` | 2 exact tests, 1 pass each; both inventory/impact and DLZ compiler/codec markers | `real-uci` |
| 2 / 071 | `bash scripts/check-task071-uci.sh` | 1 script; two route outputs compare equal, three TSV rows and exact TOML; REAL UCI ROUTES marker | `real-uci` |
| 3 / 069 | `bash scripts/check-task069-pinned.sh` | 1 ignored exact test explicitly executed; 1 passed/0 failed/0 ignored; REAL UCI SEMANTIC DIFF marker | `real-uci` |
| 4 / 062 | `bash scripts/check-task062-pinned.sh` | 1 exact listed test, 1 passed; 4 release/world markers, service impact and vertical script checks | `real-uci` |
| 5 / 063 | `bash scripts/check-task063-pinned.sh` | 2 exact tests, 1 pass each; both-release inventory/impact and smallest vertical markers; service impact script | `real-uci` |
| 6 / 065 | `bash scripts/check-task065-pinned.sh` | 1 exact listed test, 1 passed; 4 marker families per release, 33 projection rows and 2 zero gap counts per release | `real-uci` |
| 7 / 066 | `bash scripts/check-task066-pinned.sh` | 1 exact listed test, 1 passed; both authority markers and complete vertical script | `real-uci` |
| 8 / 061 | `python3 scripts/check-task061-service-impact.py target/release/ams-gra-codegen-oms` | 609 unique production service-check rows (444 release/backend/message tuples), all exit 0; final inventory marker | `real-uci-regressions` |
| 9 / 052 | `cargo test --release -p ams-gra-oms-xsd-frontend --test uci_binary_provenance --`; `--nocapture --test-threads 1` | 2 passed; both Binary provenance inventory markers | `real-uci-regressions` |
| 10 / 053 | `cargo test --release -p ams-gra-codegen-oms --test uci_constrained_binary --`; `--nocapture --test-threads 1` | 3 passed; both inventory and message-impact markers | `real-uci-regressions` |
| 11 / 054 | `cargo test --release -p ams-gra-codegen-oms --test uci_member_names --`; `--nocapture --test-threads 1` | 4 passed; both member inventory and whole-schema blocker markers plus category-A compiler marker | `real-uci-regressions` |
| 12 / 056 | `cargo test --release -p ams-gra-codegen-oms --test uci_generated_support task056_real_uci --`; `--nocapture --test-threads 1` | 2 passed; both OrderOfBattle parity and category-A parity markers | `real-uci-regressions` |
| 13 / 064 | `cargo test --release -p ams-gra-codegen-oms --test uci_generated_support -- --list`; `cargo test --release -p ams-gra-codegen-oms --test uci_generated_support "$name" --`; `name=task064_real_uci_generated_support_binding; execution --exact --nocapture --test-threads 1` | 1 exact listed binding test, 1 passed; both binding markers | `real-uci-regressions` |
| 14 / 057 | `cargo test --release -p ams-gra-codegen-oms --test uci_duration --`; `--nocapture --test-threads 1` | 4 passed; both inventory and message-impact markers plus category-A compiler marker | `real-uci-regressions` |
| 15 / 058 | `cargo test --release -p ams-gra-codegen-oms --test uci_bounded_ascii_string --`; `--nocapture --test-threads 1` | 5 passed; both inventory and message-impact markers plus newly-READY compiler/codec marker | `real-uci-regressions` |
| 16 / 059 | `cargo test --release -p ams-gra-codegen-oms --test uci_structured_ascii --`; `--nocapture --test-threads 1` | 2 passed; both structured inventory and message-impact markers | `real-uci-regressions` |
| 17 / 060 | `cargo test --release -p ams-gra-codegen-oms --test uci_alternating_admission -- --nocapture --test-threads 1`; `cargo test --release -p ams-gra-codegen-oms --test uci_alternating_after -- --nocapture --test-threads 1` | Admission: 2 passed and both exact baseline=19/admitted=15/Unicode=3/complex=1 markers; AFTER: 3 passed and all 6 projected-impact/OrderOfBattle/SMTI compiler-codec markers | `real-uci-regressions` |

Exact script-target expansion (same package `ams-gra-codegen-oms` and
`--release`, unless noted):

- Task 061: `--test uci_time_zulu task061_pinned_time_inventory_and_impact`
  and `task061_smallest_real_service_compilers_and_time_codec`, each
  `-- --exact --nocapture`.
- Task 062: `--test uci_ipv6_address -- --list`, then
  `-- --exact task062_pinned_ipv6_inventory_coverage_and_impact --nocapture`;
  `check-task062-service-impact.py` and `check-task062-vertical.py` consume
  the locally built release CLI.
- Task 063: `--test uci_patterned_integral` with exact filters
  `task063_pinned_inventory_coverage_and_service_impact` and
  `task063_smallest_real_service_compiler_codec_vertical`, each
  `-- --exact --nocapture`; `check-task063-service-impact.py` consumes the CLI.
- Task 065: `--test ada_companion_naming -- --list`, then
  `-- --exact task065_pinned_naming_evidence --nocapture`.
- Task 066: `--test unicode_string_authority -- --list`, then
  `-- --exact task066_pinned_unicode_authority_inventory --nocapture`;
  `check-task066-vertical.py` consumes the CLI.
- Task 069: debug `--test schema_diff_pinned -- --ignored --exact
  task069_real_uci_semantic_diff --nocapture`.
- Task 071: release CLI build and the unchanged PositionReport route fixture,
  `service-routes` twice plus `--format sleet-toml` (no Cargo test target).

The per-row marker inventory includes **every** inherited `grep` assertion;
the payload comparison and normalized guard digests cover their exact strings,
anchors, pass counts and command-exit propagation. Script bodies and all their
additional markers/counts remain unchanged. No coverage campaign or service
confirmation is replicated by repeating runner setup.

Each independent job checks out the same intended revision, installs stable
Rust, installs/verifies GNAT and fetches each release once through unchanged
revision/SHA-256 scripts. Root variables are exported through `GITHUB_ENV`.
Runner-local paths are under `RUNNER_TEMP`; Task 061 confirmations export
`TMPDIR="$RUNNER_TEMP"`. Job B explicitly checks both roots, seeds the locked
workspace dependencies with `cargo fetch --locked`, and builds its own input:

```sh
cargo build --release --locked -p ams-gra-codegen-oms
```

That executable is required **before** the unchanged Task 061 Python invocation.
No binary, artifact, environment or test result is borrowed from Job A. Other
moved Cargo targets build their own required CLI/test executables. No broker
build or runtime campaign is added to these schema-only jobs.

Both jobs have **240-minute** budgets and no `needs` dependency. Observed first
portion including setup was **98m57s**; observed second portion **141m18s**
before cancellation is an incomplete duration, not a prediction. Splitting adds
one runner's checkout/toolchain/GNAT/dependency/CLI setup and pinned downloads;
it does **not** claim lower total CPU consumption. It shortens the combined
critical path and provides separate bounded budgets.

Fetch accounting is now exact per owner: real-uci **1/1**, regressions **1/1**,
MSRV **1/0**, Sleet **1/0** (2.5/2.6). It counts executable invocations, not
comments or PR paths. Missing fetches cannot compensate for duplicate fetches
in another job. All previous Fast/Deep, marker, Task 077 and Sleet guards remain.

`scripts/check-task077b-ci-layout.py` is the only additional path: a stdlib-only,
task-specific structural helper invoked by the existing shell checker and test
suite. It pins normalized inherited qualification command/check digests (not
test results), enforcing exact once-only ownership and intact Task 060 commands,
exit propagation, counts and markers. Comments/indentation are ignored. These
payload hashes intentionally require review if qualification commands change;
this is not a general YAML framework. The helper's disposable mutations require
their specific diagnostic, rather than arbitrary unrelated checker failure.

### Focused local versus hosted evidence

Local: actionlint **1.7.7** passes Fast and Deep YAML; shell syntax and
`git diff --check` pass. Existing CI-split suite: **155** checks before;
updated suite: **209** checks (**54** additional: one disposable healthy and
53 attributed negatives). Coverage includes both missing/duplicate release
fetches per job, missing Job B/build/dependency preparation, deletion/duplication
of every moved gate, either Task 060 command and every marker/count assertion,
disabled jobs/Task 060, soft failures, missing budgets and serialization.
The before/after inventory confirms **17 = 7 + 10** identical step dictionaries;
MSRV/Sleet and workflow-level settings compare equal.

Task 077A wrapper and mutation suite passes (healthy 6 registrations/executions,
14 primary negatives, 3 mutants / 4 faulty passes detected). The unchanged real
Task 077 Fast gate passes all six controls with locked Cargo in fresh task-owned
build output. Fake Cargo is not used for that run. Local stable Cargo/rustc were **1.98.1**;
the existing toolchain choice is unchanged.

Job B inputs were fetched with both original pin-verification scripts in
`/tmp/ams-task077b/scratch`; GNAT **14.2.0** and G++ **14.2.0** verified. The
explicit locked release build from this worktree passes in its fresh target
root; CLI `--help` succeeds. This does not rerun the 609-row sweep locally.
Complete Task 060 local result and exact-head hosted run IDs are recorded below
when available, separately from original-run evidence.

No production sources, runtime APIs/defaults, schema generation, dependencies,
Cargo.lock, pinned sources or qualification scripts/targets were changed.
DuplicateMemberPolicy, typed rejection, codec/handler ordering, subscription
recovery, GenerationWorld/readiness and NotClosedUnderOpenExtensions are intact.

Fresh complete Task 060 local execution: **PASS**, 2026-10-12
**23:32:11Z–23:49:09Z**, exact extracted workflow shell step, wall time
**16m57.458s**. Admission: **2 passed / 0 failed / 0 ignored**, **11.81s**;
AFTER: **3 passed / 0 failed / 0 ignored**, **1004.63s**. Both admission
markers, both projected-impact markers and all four release/service
compiler-codec markers were checked by the original shell assertions and passed.
Full output: `/tmp/ams-task077b/task060.log`. This is separate local evidence,
not complete hosted certification of the new four-job workflow.

At commit time, corrected hosted checks have not yet run. The single normal
push will automatically trigger Fast and Deep; exact corrective SHA, head
parity, new run IDs/status and mergeability are reported in PR #77's follow-up
comment. Pending hosted certification is not merge readiness. No duplicate
workflow dispatch, merge or new task is performed.
