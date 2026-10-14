# Task 087 — incomplete production codec work

**Disposition: NOT_READY.** This increment establishes and locally tests the
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