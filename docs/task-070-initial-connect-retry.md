# Task 070 — Safe bounded initial LA-CAL connection retry

Immutable BEFORE: `42a8f369fe4a7f733fe1b0f802f575c3ec4a7fb0` (actual fetched
origin/main). Branch `feature/070-initial-connect-retry`; independent worktree
`/home/zboll/git/ams-gra-codegen-oms-task070`. No sibling worktree changes.

## Scope and API

Before Task070, the worker made one timed CalClient connection, reported
readiness once, and was joined before a connect error returned. Task070 adds
only opt-in **initial TCP ConnectionRefused retry** inside that worker.
Task049 historical evidence remains unchanged.

```rust
use ams_gra_oms_runtime_rust::RuntimeConfig;
use std::time::Duration;

let config = RuntimeConfig::new("ws://127.0.0.1:9000", "svc", "000.1.0")
    .with_initial_connect_retry(
        3,
        Duration::from_millis(50),
        Duration::from_millis(200),
    );
```

ConnectRetryPolicy is exposed via RuntimeConfig::initial_connect_retry(), with
max_attempts(), initial_delay(), and max_delay() getters. The default is exactly
**one attempt, no retry, zero delay**. Explicit one attempt never sleeps.
Schema, service ID, OWP versions, non-verbose INIT, and the default 10-second
timeout are unchanged.

max_attempts includes the first, is u8, and is validated as 1..=8
(MAX_INITIAL_CONNECT_ATTEMPTS). With retries, initial delay must be positive;
max delay must always be >= initial delay. All timer durations must be
representable by platform Instant, as must the checked sum of the campaign's
timeout and delay budget. Invalid settings return InvalidConfig
before spawning a worker; no panic or silent repair.

After failed attempt n: `min(initial_delay * 2^(n-1), max_delay)`.
Checked doubling caps overflow at max delay, never wraps. No sleep follows the
final failure; success immediately leaves the loop. Tokio time is used, not
blocking thread sleep. No jitter or RNG dependency; a future jitter policy may
help many clients starting simultaneously.

The timeout is **per attempt**, covering WebSocket/INIT/INFO. The maximum
configured waiting budget is `max_attempts * connect_timeout + sum(retry delays)`
(plus scheduling overhead), not a single timeout for the campaign. There are
at most eight attempts and seven sleeps. The original final error returns
unchanged on exhaustion.

## Typed classification evidence

The actual manifest/lock/checkout pin inspected is
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` (the request omitted the final b).
sleet-client/src/lib.rs validates input before connecting and wraps tungstenite
errors as ClientError::Transport. Locked tokio-tungstenite 0.29 maps
TcpStream::connect failures to tungstenite Error::Io before WebSocket upgrade.
The typed direct source of that variant is std::io::Error.

The predicate requires RuntimeError::Client(ClientError::Transport(...)), whose
direct source downcasts to std::io::Error with **exactly ConnectionRefused**.
No strings or broad transport/network category are accepted. ClientError itself
has an empty Error implementation; the predicate inspects Transport directly.
RuntimeError::source still returns the original typed ClientError, preserving
the transport and actual IO error for explicit matching.

Nonretryable: InvalidInput (invalid service ID), Server (INIT -ERR),
ProtocolParse, UnexpectedServerOp, HTTP/WebSocket handshake rejection, DNS
failure, TLS failure/unsupported TLS, non-refused IO, ConnectTimeout,
WorkerSpawn, WorkerPanicked, and every other runtime error. A generic Transport
does not prove INIT was never delivered. Failed INIT is never retried.

## Lifecycle and non-replay

One OS thread and one current-thread Tokio runtime serve the campaign. One
successful CalClient remains; readiness is sent once, and failure joins the
worker before returning. No command/subscription state exists during initial
connection. Worker::run and its unbiased select are unchanged.

After successful INIT/INFO the policy is inactive. Connection loss emits
ConnectionClosed, stops the worker, and operations return WorkerStopped.
There is **no reconnect, resubscription, SID restoration, publish replay,
offline queue, acknowledgement tracking, or PUB/SUB/UNSUB retry**. Old handles
cannot bind to a new connection. The caller may explicitly create a new
runtime. Initial connection retry cannot duplicate a publish.

## Test evidence and limits

Six exact runtime tests cover validation; default/explicit one attempt; three
total attempts and no final sleep; exponential progression, cap and overflow;
success on attempts 1/2/3; typed classification with actual pinned-client
invalid input, INIT Server, ProtocolParse and unexpected operation errors,
HTTP rejection, unsupported TLS, timeout, worker spawn/panic, and seven
representative non-refused IO kinds (even with misleading refusal text).

The real loopback test receives an actual TCP refusal, then binds the peer in
the private delay seam before continuing: no sleep-and-hope. Attempt two
completes INIT/INFO, then drops the client and checks no extra connection.
Network waits have five-second bounds.

Four exact facade controls cover terminal refusal preserving typed IO and
leaving no worker; INIT rejection with one INIT/no extra connection;
established-session loss without replay/reconnection and stale handle
rejection; and one generated typed PUB/one INIT/close/no extra connection/no
Linux worker remaining. Worker counts are serialized. Mock extra-connection
observation is opt-in, preserving Task049 mock behavior by default.

**Limit:** refused-then-accepted coordination uses the private initial-phase
seam, not the public worker plus generated endpoint in one scenario. Generated
publish is verified separately through the public runtime with retry enabled.
No timing-dependent end-to-end coverage is claimed.

Fast registers and executes all ten tests exactly, prints diagnostics and
checks exit status before markers, then prints
`TASK070 INITIAL CONNECT RETRY: PASSED`. No real UCI or Sleet server is used.
Deep real-uci (240 minutes), real-sleet, msrv-real-uci remain unchanged.
Local and hosted delivery results are reported separately; implementation does
not imply hosted certification.

Local validation passed: workspace fmt/check/Clippy (-D warnings), both runtime
packages' tests, GNAT-required workspace tests (1304 passed, zero failed), all
ten exact Task070 gates, CI split guard and its 140 adversarial checks,
git diff --check, and all three locked Rust 1.95.0 checks. Fake-Cargo controls
also rejected command failure, missing registration, and zero-test execution.
Cargo.lock adds only an existing pinned sleet-client test dependency edge;
no package or version changes. Runtime API, generators, Codegen Core, SchemaIr,
worker fairness, error representation and Deep CI are unchanged.