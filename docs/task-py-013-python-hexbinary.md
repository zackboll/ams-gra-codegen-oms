# PY-013 — Python Binary models and OMS JSON hexBinary

## Scope and source identities

Worktree: `/home/zboll/git/ams-gra-codegen-oms-py-013`; branch:
`feature/py-013-hexbinary`. Fetched main before beginning; no existing PY-013
branch or PR existed. Starting main was commit
`a28cb1d7add3e1c171a31f11462e676330e75b5d`, tree
`0c79b9d279da230431bfaad93b211bd75e0a7c22`.

Incorporated concurrent main by ordinary merge, not rebase: main commit
`f14aa593561eda9ae5db4cc1e9522a7337403b8f`, tree
`5712209a094e47341c8e5b94966266332e815314`; merge commit
`50bb9a7f10967bef39f96c90d0020d0976dccd8b`. Its Rust changes and workflow
updates are preserved. No sibling branches, worktrees, PRs or evidence changed.

The measured AFTER production implementation is commit
`26c167c20a8ebb3fc8233b3f165deb1a14cb82b2`, tree
`dde0502f15f20d8563995c4b2bff6f3d6b3f69e9`. Subsequent qualification additions
do not alter its Python production bytes. Frozen inventories are independently
authenticated in `scripts/check-py013-uci.py`, rather than compared with current
production or requiring old Git objects. The authenticated generator SHA-256s:

* BEFORE: `c688216aa5b269a9510004664970d9cc8d8fbb1ac6519cac719c14326cc56098`
* AFTER: `fceabd6a86c9baa5c8fc59d8d186a572fea777f36523fd27dcf6503d9f048b53`

Raw execution logs, failed attempts, binaries and generated artifacts are retained
separately in `/home/zboll/py013-evidence/v1`; builds use
`/home/zboll/py013-build`. The checked-in versioned evidence is
`tests/fixtures/python-hexbinary/evidence/v1`. Historical PY-006 through PY-012
evidence is unchanged. No dependency pins or shared classifier, IR, frontend,
Ada/Rust/C++ backend sources are modified by this task.

## Implementation

* Direct Binary fields use exact immutable `bytes`, not strings or integer
  arrays. Named declarations are frozen, slotted dataclasses with a checked
  `value: bytes`. Subclasses, bytearray, memoryview, lists and strings fail;
  validation contains no assertions and remains active under `python -O`.
* `binary_length_domain` classifies effective declaration constraints. Length
  counts octets. Named restriction chains use the already normalized domains;
  no schema-name inference or replacement classifier. Field-local length,
  patterns, numeric facets, invalid whitespace, contradictions, aliases and
  abstract values remain unsupported.
* Codec provenance is checked before rendering using
  `declaration_binary_encoding` and `resolve_binary_encoding`. Unknown or
  Base64 Binary is model-representable but has no supported codec. No frontend
  Base64 support is added.
* One JSON string represents one Binary value. Encoding revalidates exact bytes
  and named octet lengths (including forged/missing storage), then produces two
  uppercase ASCII hex digits per octet. Empty bytes encode as `""`.
* Decoding requires a string, strips only surrounding SPACE/TAB/CR/LF, and
  rejects internal whitespace, odd digit count and non-ASCII hexadecimal digits
  before calling the standard parser. This implements fixed XML whitespace
  collapse plus the hexBinary grammar, not Unicode whitespace normalization.
  Errors are `CodecError` with schema-qualified member and array-index paths.
* Existing Record, effective inheritance and tuple occurrence machinery is
  reused. Repeated Binary is `tuple[bytes, ...]` (or tuples of named carriers),
  independently validating outer occurrences and inner octet counts. Zero-minimum
  tuple defaults, positive minima, order and omission rules remain PY-012's.
* Helpers are private and demand-gated. `bytes` is reserved only when Binary is
  demanded, for both type and field names. No new globally resolved builtins
  were introduced beyond this protected name.

## Independent generated-code qualification

`crates/cli/tests/python_hexbinary.rs` drives unchanged production CLI selection,
check and generation in both worlds, model-only and codec-enabled selection,
both import styles, and normal/optimized execution. Its authored vector covers
all 256 octets, empty/one/multiple octets, mixed-case input and uppercase output,
XML whitespace boundaries, malformed Unicode/ASCII/odd forms, exact/min/max/
interval/zero domains, nested restriction chains, frozen storage, subclasses,
forged carriers, optional/bounded/unbounded members, first/middle/last index
errors and qualified nested/inherited paths. It checks determinism and atomic
failure with absent and pre-existing output directories.

`crates/backend-python/tests/hexbinary.rs` adds synthetic unknown/Base64 provenance
fail-closed controls and unsupported effective-domain, alias and field-facet
controls. The existing backend/CLI controls that historically rejected every
Binary value now distinguish admitted Binary from still-unsupported facets.

CPython 3.11 and 3.13 backend all-target regressions passed (27 tests each).
Eleven CLI regression targets passed (36 tests per interpreter), including all
previous scalar/temporal/numeric/PY-011/PY-012 targets. Generated probes execute
normal and optimized policies. An initial 3.13 run failed with `ENOSPC` writing
pre-existing Float64 bytecode under `/tmp`; a task-owned `TMPDIR` rerun passed.
Initial old Binary rejection expectations failed and were corrected; those logs
are retained, not counted as successful tests.

Binary-free byte parity against the preserved starting-main CLI passes for 40
selections / 100 generated Python sources. This includes PY-012's unbounded
schema and independently authored Sleet schema. No Binary helper appears in
these outputs. The earlier 32-selection / 80-source parity is also retained.

## Actual pinned UCI BEFORE / AFTER

Both authenticated roots and security-marking includes use the existing pins:

* UCI 2.5: `093610b7753944059360d3236770ab446d039556`
* UCI 2.6: `78eb61b6112c8bffa40820c33124b57787fc5bd9`

`scripts/measure-py013-uci.py` measures the actual selected closures through the
unchanged production CLI; no support filter or guessed next blocker. Each row
below represents both worlds and model-only/codec-enabled selection (four
selections). There are **32 BEFORE and 32 AFTER**. Exact qualified selected type
identities, closure/contract hashes, check/generation stdout/stderr/exit and
artifact inventories are recorded in the versioned JSON.

| Release | Service | BEFORE first blocker | AFTER first blocker |
|---|---|---|---|
| 2.5 | PositionReport | `FGI_SourceOpenChoiceType`: Choice/List | unchanged |
| 2.6 | PositionReport | `FGI_SourceOpenChoiceType`: Choice/List | unchanged |
| 2.5 | RDMA_InitializeSetup | `IPv4_AddressType`: constraints | unchanged |
| 2.6 | RDMA_InitializeSetup | `IPv4_AddressType`: constraints | unchanged |
| 2.5 | SubsystemStream | direct `SubsystemStreamBinary`: Binary | `FGI_SourceOpenChoiceType`: Choice/List |
| 2.6 | SubsystemStream | `HexBinaryType`: Binary | `FGI_SourceOpenChoiceType`: Choice/List |
| 2.5 | Log | `LogMDT.ServiceUpTime`: Duration | unchanged |
| 2.6 | Log | `DurationType`: Duration | unchanged |

All actual selected models/APIs remain NOT READY. Codecs are **not measured**
when model/API are not ready. Every generation exits 1 without creating output;
closures/contracts remain identical across BEFORE/AFTER. Removing the two
Binary blockers is not full-service readiness or real-provider interoperability.

The isolated authentic UCI test executes the unmodified direct 2.5 and named 2.6
`SubsystemStreamBinary` declarations, applicable `HexBinaryType`, 2.6
`IFF_RegisterType -> HexBinaryType -> xs:hexBinary` (seven octets), and both
releases' `AA_CodeType`, `BDS_AddressType`, `SHA_2_256_HashType`. Both worlds,
import styles, interpreters and optimization policies passed. These extractions
are explicitly not executions of complete UCI services.

## Pinned Sleet interoperability and limits

Pinned revision: `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`, clean checkout. Synthetic OAM
`BinaryNotice` runs through generated model/API/codec and existing Python LA-CAL
runtime. Both interpreters and optimization policies passed. Empty/nonempty
payloads, 0x80/0xFF, uppercase output, lowercase inbound, optional/repeated
values, typed delivery, recovery, unsubscribe, teardown and no leaked asyncio
tasks are checked.

Each execution observed five generated-decoder malformed rejections and one
malformed-payload broker rejection, followed by successful recovery. The broker
also rejected a Python-valid two-octet carrier `"80FF"` while accepting lexical
`"80"` for its length=2 facet; Python then rejected the latter as one octet.
This confirms the pinned broker's lexical-character-count limitation. Python
validation is not weakened. The full original broker logs and generated modules
are retained separately. No complete authentic service became READY, so there
is no claim of full-UCI generated-service interoperability.

## CI and delivery

New tests join existing Python Fast/UCI/Sleet discovery. There are no new workflow
lanes, selectors, gates or replacement PR matrices; merged ten-lane main policy
is preserved. Formatting, locked workspace all-target check, focused
warning-denied Clippy, local Python PR smoke and `git diff --check` passed.

Existing Deep CI PY-007 through PY-010 profile campaigns now explicitly select
`evidence/v1/legacy-services.json`: the exact 24 non-Log rows projected from the
authenticated PY-013 AFTER measurement. This updates the current-service oracle
without changing any historical fixture or historical test default. The main
32-row evidence remains authoritative. Projection equality is tested. The
existing Sleet harness exports its just-built generator path for PY-012/PY-013
discovery; no transport implementation or broker source is changed.

The first direct UCI discovery attempt rejected before execution because
`RUNNER_TEMP` was missing. It is retained separately; source-bound receipt
qualification supplies the required environment. Final source-bound receipts
and hosted exact-head PR smoke are reported in the PR handoff rather than
claimed against this document's self-referential final commit.