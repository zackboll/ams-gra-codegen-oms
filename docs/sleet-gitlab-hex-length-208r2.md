# PR #208 R2 — downstream reconciliation

Review preparation only: **do not merge**. This correction does not establish
canonical dependency acceptance or complete main-only qualification.

## Reconciliation

Original PR head: `ac333a98eab35c590ff61f7e45f248015499c9a4`.
Freshly fetched main: `c6ddb747d22ec3c9092a87755465c63a52ebad84`.
Ordinary merge: `5028629133aea6ad9ec46595d7fe2f0e2fe2c0db`, tree
`56a0ad4120508cac8b2a948577179377b7a506cc`.

The sole textual conflict was `scripts/test_ada_deep_pack.py`: retain both
the candidate-pin workflow expectation and main's exact build/admission/timing
expectations. No whole-side resolution was used. The automatic runner merge was
compared with both parents. Main's schema-session generation optimization,
source/root/contract invalidation, seven mandatory-family flags, worker metadata
negative controls, immediate post-join assertions and repeated joins remain.
Candidate commit/tree/clean-source verification, actual Cargo broker build,
compiler reporting and binary fingerprinting remain. No production validator,
authentic schema, dependency pin, or valid compatibility fixture is changed.

The inventory regression now compares the candidate runner's complete published
inventory, including its split standalone tests, rather than demanding the
superseded broker URL. The 2,000-join test is a separate mandatory standalone
case, not unreachable behind the mixed-family DLZ failure. The required runner
count increases from six to seven; no original family or marker is removed.

## OrdersMetadata correction and focused results

Both releases retain `not-a-dateZ`. Require OWP `Invalid-Message` and the exact
observed detail: `FacetViolation` at `OrdersMetadata.MessageHeader.Timestamp`,
facet `lexicalRepresentation`, value not a valid `xs:dateTime`. No generic facet
or alternate decoder-boundary acceptance is sufficient. Require no observer MSG,
callback or decoder event; after each negative, the original subscription must
receive a valid typed publication and the observer must receive identical JSON.
The direct codec test still rejects every negative independently and requires a
Timestamp diagnostic. Unsubscribe and teardown barriers are unchanged.

Before: five authentic tests, **3 passed / 2 failed**, one live failure per
release at the obsolete decoder-boundary expectation.
After: **5 passed / 0 failed / 0 ignored**, plus **1 passing independent consumer
driver**, executing both releases' tests/check/Clippy. A second runner execution
after `cargo clean --release -p sleet` also passes; verbose output witnesses
`rustc --crate-name sleet`, not merely an existing executable.

The four authentic MessageDefinitions/SecurityMarkings SHA-256 pins match the
table in `rust-oms-012-real-orders-metadata.md`.

Selected, unchanged Sleet commit: `94796daf55ebd1134dce0925e6a204a05ad3423b`;
tree: `47da595b9078cac1a9351e8290d1ae7f3976367f`; clean source, own Rust 1.95.0
toolchain and locked dependencies. Rebuilt broker SHA-256:
`14d49d5989940b7513cfdceaf7eb857aaafa6faedea7da503f827e3179fb1f5c`.

Other completed local checks at source delivery:

- Authentic Log: four tests; lifecycle: seven tests; both pass without ignores.
- Authentic FileMetadata: five tests plus independent consumer pass, both
  releases, hash-present/hash-absent, broker negatives, recovery and cleanup.
- Synthetic/generated/mock-OWP regression: 40 passes across 11 test binaries.
- Schema-session parity and genuine schema/contract change controls: three pass.
- Workspace release all-target Clippy `-D warnings` and format checks pass.
- Orders runner: eight controls pass, including injected exit 23, compiler and
  digest reporting, missing artifact, wrong commit/tree, dirty source, missing
  markers, skips and wrong test counts. Lifecycle controls: four pass, including
  the eight executable worker metadata controls.
- Ada CI/deep packs: 20/22 pass. Task101–106 verifier controls: 48 run, 43 pass,
  five explicitly opt-in Podman ownership fixtures skipped. Python registered/
  owned discovery, main-lane, Task077B CI-layout mutation, dependency installer,
  timing and Alire-wrapper controls pass.
- Smoke router: 40 pass. Its offline-command assertions normalize only the
  checkout prefix so a directory named `sleet` cannot cause a false failure;
  forbidden campaigns remain forbidden.

Broader reruns and exact-head hosted results are reported in the current PR
description, not inferred from these focused successes. Local evidence is under
`/home/zboll/sleet-gitlab-update-evidence/208r2/`. Initial failed controls and
incomplete attempts are retained there alongside corrected reruns.

## Remaining blockers

The standalone mixed-family campaigns still fail on the candidate's valid DLZ
midnight incompatibility; their failures must not be described as green lifecycle
or FileMetadata campaigns. Independent boundaries and joins run separately.
The ordinary runner remains fail-closed at a failing mandatory family; subsequent
families are not thereby qualified. Full main-only qualification remains pending.

Unchanged upstream blockers:

- `24:00:00.000Z`: valid XML Schema midnight rejected.
- HexBinary leading/trailing XML whitespace: required normalization absent.
- `2000-02-29T23:59:60+14:00`: incompatible with OMS's documented XSD 1.0 profile.

Upstream Sleet acceptance, a verified exact canonical pin, compatibility fixes
and remaining mandatory qualification are required before merge readiness.
No upstream project or PR #198 branch is changed. Leave PR #208 open, non-draft
and unmerged for independent review.