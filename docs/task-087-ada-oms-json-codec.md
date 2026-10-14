# Task 087 — incomplete production codec work

**Original Task 087 disposition: NOT_READY.** This increment establishes and locally tests the
reusable guard only. It does not implement generated Ada codecs, change Ada
codec readiness, or qualify a generated codec executable. It is not a claim
of complete OMS JSON conformance or runtime readiness.

## Isolation

Freshly fetched starting `origin/main`:
`89702ef09c5966b54dc43222da5823e8abd05fb2` (merged PR #86).
Branch: `feature/087-ada-oms-json-codec`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task087`.
Evidence/build root: `/tmp/task087`.
Initial main working tree was clean; no Task 087 branch or PR existed and the
open-PR query returned none. Sibling worktrees, Task 086 evidence and UCI caches
were not removed or modified.

## Reusable boundary

`ada/oms-json-support` contains the one validation implementation, package
`AMS_GRA_OMS_JSON_Guard`. The former probe package is now a package renaming,
not a second validator. The standalone probe imports the support GPR and pins
the support crate locally. Its historical unguarded provider control remains.
Task 086's historical documentation is unchanged.

The provider remains GNATCOLL / GNATCOLL minimal **26.0.0**, upstream revision
`8e270f83719d4284782a965edd80246be15b949b`. The inspected upstream and cached
Alire `gnatcoll-json.adb` have identical SHA-256
`7e07277731a44ea08716fd61a6462ebe7a7ce0722adc6dc787cbbaafd3099063`.
The support README documents license, exact dependencies, Ada 2022 compiler
requirements, source-project consumption and GPR/Alire setup.

The outer `Constraint_Error` translation has been removed completely.
GNATCOLL's result-returning reader reports grammar failures through
`Read_Result`; its integer conversion already translates overflow in a
narrow local handler. Explicit guard rejections raise `Validation_Error`.
Unexpected indexing/arithmetic/programming failures are not concealed.
Unchecked floating overflow is not yet qualified; no float codec is supported.

Ingress measures the **1,048,576-byte** limit before validation/provider
allocation. An iterative pre-parser check limits simultaneous structural
openings to **128**, ignoring delimiters in strings. GNATCOLL still owns JSON
grammar. Tree traversal retains maximum depth 128 and 100,000 nodes.
No complete memory-containment claim is made. Caller-owned input allocation,
provider allocation overhead and output serialization allocation are not
measured or wholly contained.

Unicode validation and outgoing key/string validation retain the researched
behavior. Duplicate decoded keys remain **last-value-wins**. Signed integers
retain the range `-9223372036854775808 .. 9223372036854775807`; `-0` preserves
value, not spelling. Fraction/exponent support is not integer-codec evidence.

## Local execution evidence

Actual compiler versions recorded using each resolved Alire environment:
GNAT **14.2.0** and **16.1.0**, Alire **2.1.1**.
Separate scratch support copies avoid mixed-compiler object directories.
Both compiler environments built and executed the full original **199-case**
guard corpus plus four added checks under normal, explicit Check, and explicit
Ignore assertion policies. All six runs passed.

The original executable-output prefix has SHA-256
`ba61ad1bf1cd12253655cb6495104874f5a61bf6f7f4b06fe8a84f154cfcf64e`, exactly the
Task 086 qualification hash. The complete new direct executable output is
identical across all six runs, SHA-256
`67a60d687e5fa1ad660a9b0322a4f9588f0107925e6abd4fe15353f8777c3036`.
This includes the Unicode corpus, both unguarded defect reproductions,
signed overflow rejections, integer precision, and escaped-key duplicate
evidence. Added tests accept the exact byte/depth limits and reject one beyond
each limit while checking the specific `Validation_Error` message.

Initial build/test failures were corrected: a local `Positive` procedure hid
the predefined subtype; compiler consumers initially shared support object
files; and the first scanner edit omitted the intended nesting branch. Those
failed runs are preserved in scratch where not superseded by repeated build
logs and are not treated as successful qualification.

Passed local workspace gates:

* `cargo +1.95.0 fmt --all -- --check`
* `cargo +1.95.0 check --workspace --all-targets --locked`
* `cargo +1.95.0 clippy --workspace --all-targets --locked -- -D warnings`
* `git diff --check`

## Model representation review and remaining gates

Source inspection found named Boolean derived types, named signed integer
range types, unsigned-64 subtypes, direct String unbounded storage, checked
constrained-String `Create`/`Value` carriers, enum literals mapped by the
shared generated-name helper, flattened effective inherited Record fields,
`Optional_String`, and field-specific discriminated optional wrappers.
Safe reconstruction requires complete aggregates with constructed present
values, never default initialization of private carriers.

The actual proposed synthetic model has **not** been generated/inspected, so
these source observations are not sufficient evidence to write the renderer.
Direct field-local facets and unsigned-domain conversion need separate,
fail-closed capability decisions. Full unsigned-64, Choice, repeated values,
abstract values, Float/Decimal, temporal values, Binary, nillable values,
unqualified members, Alias/List, and unresolved references remain excluded
from the proposed first slice.

Outstanding implementation/qualification gates:

1. Independently authored minimal XSD/contract/expected JSON and generated
   model API inspection.
2. Shared Ada-specific capability decision using projected emission order,
   generated support, naming preflight and exact domain checks.
3. Production Ada renderer, public typed payload codec pairs, artifact
   collision checks, and CLI integration/counts. **No codec API exists yet.**
4. Generated executable positive/negative corpus with both compilers and all
   assertion policies, including checked constructors and member error paths.
5. Byte-identity matrix, zero-OMS and unsupported-output atomicity evidence.
   Generator/model/backend sources and original goldens are unchanged in this
   increment, but that is not a substitute for the requested execution matrix.
6. Verified pinned UCI roots/SecurityMarkings and RDMA readiness inventory.
   No new real-UCI measurements or real-UCI codec success is claimed.
7. Mandatory hosted provider provisioning and generated codec execution.
   No workflow changes or hosted codec qualification exist yet.
8. Final review, complete feature push, non-draft PR, exact local/remote/PR
   head parity and exact-head CI status. No ready-for-review PR is claimed.

Model readiness remains distinct from codec readiness. The production Ada
codec verdict still uses its original LanguageNotImplemented boundary. No
runtime/LA-CAL/WebSocket changes, Rust codec changes, C++ changes or dependency
upgrades were made. No merge, deployment or Task 088 work was performed.

## Task 087A — independently mergeable support-library delivery

**Task 087A delivers the reusable JSON support boundary only.** Production
Ada OMS JSON codec generation remains unimplemented. `service-check --language
ada --with-codec` continues to report NOT READY for OMS-message services; no
`service_codec.ads` or `service_codec.adb` artifacts are produced. Task 087B
will consume this independently reviewed support library after it merges.
The original Task 087 generator gates above are not Task 087A requirements.

### Commit chain and strict scope

The immutable starting main remains
`89702ef09c5966b54dc43222da5823e8abd05fb2`. On fetching origin for Task 087A,
main had not advanced. The existing support commit
`30c514fb6f6a585a0d6ee9b0a5f9e8027c0989ea` was verified as HEAD and preserved
without reset, amendment or rebase. The Task 087A commit is its normal additive
child (`Qualify reusable Ada JSON guard in hosted CI`); exact published child
SHA and PR head parity are recorded in the PR and delivery report, avoiding a
self-referential commit hash in this document.

Only the support library, probe integration, this document, two focused
qualification scripts and the isolated Fast CI job are changed. Cargo
manifests/lock, codegen-core, backends, CLI, schemas, runtime and Deep CI are
unchanged. Original Task 086A records and `/tmp/task087` evidence are preserved.

### Library contract and audit

Library version: `ams_gra_oms_json=0.1.0-dev`. Public API:

```ada
Validation_Error : exception;
function Read (Text : String) return GNATCOLL.JSON.JSON_Value;
function Write (Value : GNATCOLL.JSON.JSON_Value) return String;
```

The manifest fixes GNATCOLL and GNATCOLL minimal to **26.0.0**. Qualified
upstream source remains `8e270f83719d4284782a965edd80246be15b949b`. The wrapper
checks the resolved provider archive SHA-512 against the Task 086 evidence;
the provider is neither patched nor upgraded. Support sources are MIT;
GNATCOLL is `GPL-3.0-or-later WITH GCC-exception-3.1`. The README describes the
provider's GPL and GCC Runtime Library Exception texts and distribution
boundary without claiming the exception replaces source-license obligations.

Comparison with the reviewed guard confirmed unchanged UTF-8 lead/continuation
and scalar-range validation, surrogate pairing, control/escape checks,
pre-read validation, pre-write key/string validation, and post-write lexical
validation. Accepted Unicode is not normalized or substituted. Integer tokens
retain the exact signed-64-bit boundary; overflowing tokens never fall back to
floating point. `-0` normalizes as a value. Fraction/exponent and floating
overflow qualification is outside this integer slice. Duplicates retain the
provider's last-value-wins, including escaped-equivalent names.

Resource limits remain 1,048,576 input bytes before provider read, 128
simultaneous structural openings before parsing, and traversal depth 128 /
100,000 nodes. This is not complete memory containment. The byte scanner uses
offsets bounded by remaining String length, checks each lookahead before
indexing, and advances only after validating the consumed width. Incoming
offsets are additionally bounded by the byte limit. The 16-bit accumulator
never exceeds 65,535. Nesting is checked before incrementing and decremented
only above zero. Traversal rejects before the node-limit increment; depth
advances only from an admitted depth, so at most 129 reaches the next check.
No assertion policy controls these bounds. Unexpected internal exceptions
are observable, not converted into malformed-input success.

The probe's former catch-all `Observe` handler was removed: unexpected
provider exceptions now exit nonzero, while both authored unguarded defect
reproductions remain. The original output does not change because that
handler was never triggered in the qualified corpus. All validation is still
in one production implementation via `package JSON_Guard renames ...`.

### Reproducible local and hosted qualification

From the repository root, with Alire 2.1.1 and Python 3.11+:

```sh
python3 scripts/test-task087a-wrapper.py
python3 scripts/check-task087a.py --compiler 14 --scratch /tmp/task087a-new14
python3 scripts/check-task087a.py --compiler 16 --scratch /tmp/task087a-new16
```

Scratch destinations must be new. Run campaigns sequentially with a shared
Alire cache. The wrapper selects exact compiler and GPRBUILD releases locally,
checks solution metadata/provider hash, prints actual compiler versions,
records each subprocess exit and output, builds an independent path-dependent
consumer, and executes normal/Check/Ignore campaigns. It does not run Rust or
copy support Ada sources into the consumer. A library-only `alr -n build`
also succeeds using the README's scratch-copy commands.

Toolchains: GNAT external **14.2.0** and native **16.1.0**, Alire **2.1.1**,
GPRBUILD Alire release **26.0.1** (binary identifies **26.0.0, 2026-04-15**).
Alire's compatibility heuristic needs explicit force-selection for the local
external-14/GPRBUILD pair; no failures are suppressed. Hosted native-16 needs
no override. Both clean consumers execute their exact PASS marker with exit 0.

Frozen original 199-case prefix SHA-256:
`ba61ad1bf1cd12253655cb6495104874f5a61bf6f7f4b06fe8a84f154cfcf64e`.
Complete 199-case-plus-four-boundary output SHA-256:
`67a60d687e5fa1ad660a9b0322a4f9588f0107925e6abd4fe15353f8777c3036`.
The wrapper requires both hashes and exactly one of each actual completion
marker. A bare PASS word is not evidence. The eight wrapper controls reject
nonzero command status, zero cases, incorrect counts, missing completion,
skips, invented PASS output, unexpected exceptions and wrong dependency
versions/archive hashes. Fake probe commands are actual subprocesses.

Fast CI adds mandatory job **`ada-json-guard`**, Ubuntu 24.04, 30-minute
budget, separate from the existing Rust job. It downloads Alire 2.1.1's
x86-64 Linux archive and verifies SHA-256
`09c66bcd8c35dd4b97b72c3d9b76e44caa6964a2db35aba069f396f00f1f64c7`.
The archive installation was reproduced locally. Alire resolves explicitly
selected GNAT native 16.1.0 and GPRBUILD 26.0.1 with archive integrity checks;
the wrapper verifies the resolved exact versions. The job runs all three
policies, not just normal and Ignore, plus the clean consumer. It uploads
logs and the exact solution lock as evidence even on failure. Missing tools,
resolution/build failures, nonzero executable status, wrong hashes, absent
markers or skipped cases fail the job; there is no allowed-to-skip path.
No real-UCI fetch, Sleet build, duplicate workflow dispatch or runtime campaign
is added. Existing split guard and its 209 adversarial checks pass unchanged.

An initial local native-16 campaign overlapped a library-only forced build
using the same Alire cache and failed linking against a concurrently rebuilt
archive. It was not certified. Qualification was repeated sequentially; the
failed log is preserved. This is why shared-cache campaigns must be serialized.

Final recorded local results (all command exit codes **0**):

| Gate | Result |
| --- | --- |
| GNAT 14.2.0 normal / Check / Ignore | 199 corpus cases + 4 boundary checks in each run; both frozen hashes match |
| GNAT 16.1.0 normal / Check / Ignore | 199 corpus cases + 4 boundary checks in each run; both frozen hashes match |
| Independent path-dependent consumer, each compiler | Builds and executes exact consumer PASS marker |
| Independent support project build | Passes with native 16.1.0 |
| Verified downloaded Alire binary, hosted command reproduction | All three native-16 policies and consumer pass |
| Qualification wrapper controls | 8 tests pass, including required adverse cases |
| Rust 1.95.0 fmt / locked workspace all-target check / warnings-denied Clippy | Pass |
| Ada backend + codegen-core regression command | 34 completed target summaries; 587 passed, 0 failed, 0 ignored |
| Existing CLI service codec regressions | 11 passed, 0 failed, 0 ignored |
| Fast/Deep split and adversarial split controls | Pass; 209 controls |
| Workflow YAML parse and protected-file scope checks | Pass |

The original unfinished Ada/core log was found completed (same 587 passing
tests), but had no separately recorded launcher exit status. A full rerun
records exit 0 in `/tmp/task087/task087a-ada-core-tests.exit` and its final
summaries in the adjacent log; only that rerun is certified here. Final support
logs/locks are in `/tmp/task087/task087a-qualified14` and
`/tmp/task087/task087a-certified16`, with additional hosted-command reproduction
under `/tmp/task087/task087a-hosted-reproduction`.

Hosted CI status is reported independently at publication; local success does
not assert hosted success. The branch is submitted for independent review,
not merged or deployed. No Task 087B or Task 088 work is included.