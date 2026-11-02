# PY-016 — Python whitespace-visible String carriers

## Scope and representation

The Python model and OMS JSON codec use the existing language-neutral
`string_profile` proof. No admission depends on a declaration name or release.
Only these whole semantic tuples are admitted:

| Policy | Minimum | Maximum |
| --- | ---: | ---: |
| Collapse | 0 | 1024 |
| Collapse | 0 | 4096 |
| Preserve | 0 | 4096 |
| Preserve | 1 | 1024 |
| Preserve | 1 | 4096 |

Named declarations become frozen, slotted dataclasses with a required `value:
str` constructor argument. Only exact `str` values are accepted. Optional absence
is `None`; an empty constructed carrier is a present value. Existing String
representations and direct field-local constrained String rejection are unchanged.

A demand-emitted private validator implements XML Schema whitespace collapse in
linear time using only SPACE, TAB, LF and CR. Length is checked **after** collapse,
with no raw-input maximum. Preserve retains every character and space, including
LF and CR, but rejects TAB. Both policies admit only printable ASCII, LF and CR
after normalization; neither performs Unicode normalization or case conversion.
Runtime checks do not depend on Python assertions or optimization policy.

Each named restriction edge requires a classified parent and child with contained
bounds and the same normalization policy. Effective-facet mutations, unresolved
bases, cycles, and policy-changing edges fail closed. The recursive Choice table
revalidates storage without repairing forged noncanonical Collapse carriers.

The codec maps carriers to plain JSON strings. Decode calls the checked named
constructor. Encode requires the exact generated class, accessible exact-string
storage, valid characters/bounds, and canonical Collapse storage. Model failures
become path-qualified `CodecError` values, including repeated and Clark-qualified
Choice paths. Strict JSON parsing and wire identities are unchanged.

## Authentic release differences

| Original declaration | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| WhitespaceVisibleString1024Type | Collapse, 0..1024 | Preserve, 1..1024 |
| WhitespaceVisibleString4096Type | Collapse, 0..4096 | Preserve, 1..4096 |
| QueryString4096Type | Preserve, 0..4096 | Preserve, 1..4096 |

The isolated-declaration qualification copies the original declarations and
facets from authenticated MessageDefinitions/SecurityMarkings inputs. It is
explicitly separate from complete-service execution.

## Qualification organization

* `crates/cli/tests/python_whitespace_visible.rs`: actual CLI admission, atomic
  rejection, deterministic artifacts, both worlds, model-only/codec, import
  styles, optimization policies, all admitted profiles and negative neighbors.
* `crates/backend-python/tests/whitespace_visible.rs`: independent normalized-IR
  ancestry and effective-facet adversaries, without relying on frontend rejection.
* `tests/fixtures/python-whitespace-visible/probe.py`: executable boundaries,
  exhaustive byte character corpus, Unicode/control rejection, long raw inputs,
  exact types, immutable storage, nested/repeated Choices, forged carriers and
  strict inbound/outbound paths.
* Existing Fast/UCI/Sleet source discovery registers the new task tests without
  adding lanes or matrices. All source-bound receipts require clean immutable
  source and actual execution under Python 3.11/3.13, normal and `-O`.
* `test_whitespace_visible_services.py` generates from the **unchanged full**
  pinned schemas, then executes Log, PositionReport, SubsystemStream and
  SurvivabilityRiskLevel using independently authored JSON goldens, checked
  headers/security and repeated OwnerProducer Choices.
* Pinned-Sleet tests cover the synthetic model and full unchanged UCI Log for
  both releases/worlds, instrument decoder calls, distinguish broker rejection
  from runtime/decoder rejection, recover after malformed input, unsubscribe,
  and check for leaked tasks. Sleet and `websockets==17.2` are unchanged.

Versioned evidence lives under
`tests/fixtures/python-whitespace-visible/evidence/v1`. Historical PY-006 through
PY-015 fixtures and default historical oracles remain unchanged. The main UCI
workflow calls the executable PY-016 successor instead of historical gates that
deliberately reject any READY service. The successor retains prior profile and
inheritance execution as well as the new complete-service tests.

The evidence distinguishes generator-production source identity from subsequent
test/evidence delivery commits. Final-source receipts bind the actual clean
qualification revision, tree, interpreter, generator and source inventories;
they must not be described as receipts for a later unexecuted revision.