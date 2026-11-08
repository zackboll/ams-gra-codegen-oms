# PY-018 checked Unicode 3.1 strings

Python admits exactly the shared `StringProfile::Unicode` languages:

| Existing UCI declaration | Unicode code points |
| --- | ---: |
| `NITF_DateAndTimeType` | 12 |
| `NITF_DateType` | 8 |
| `NITF_MSTGTA_TargetLocationType` | 21 |

Admission is effective-facet classification, never declaration spelling. Every
named restriction edge must retain the same profile. Extra facets, changed
alternative ordering, incompatible/unresolved/cyclic ancestry, abstract forms,
and field-local constrained primitives remain unsupported. Ordinary neighboring
String profiles do not become supported.

The renderer derives its Nd ranges from `UNICODE31_ND` and each ordered token
sequence from `UnicodeStringProfile::branches()`. The tiny Python runtime uses
only exact `str`, code-point length, explicit character membership and fixed Nd
range membership. It does not use host Unicode classification, regexes, calendar
or coordinate interpretation. U+1369 and U+1D7CE are admitted at Nd positions;
U+1E950 is not. Explicit ASCII positions stay ASCII. Date and DateAndTime retain
their all-space alternatives. No text is normalized, trimmed, or converted.

Carriers are frozen, slotted dataclasses with required values and explicit
validation exceptions, also under `python -O`. Schema-specific recursive
revalidation is emitted even without Choices. OMS JSON codecs reuse the checked
named-string ingress/egress helpers: exact carrier classes, checked constructors,
ordinary JSON strings, strict parsing and qualified/indexed paths. Forged storage
is rejected without changing it. No transport protocol changes are needed.

## Independent qualification

`tests/fixtures/python-unicode31/cases.py` retains the unchanged Task066 authored
corpus and independently enumerates month/day/hour/minute boundaries and Unicode
classification witnesses. Invalid UTF-8 has no Python `str` representation;
surrogates are independently tested as Python-only invalid strings. The Rust CLI
integration test compares all **3,099** independent string expectations to
`UnicodeStringProfile::accepts`, then executes generated Python under both worlds,
both import styles and both optimization policies. It also checks atomic failure,
renamed declarations, extra facets and unsupported neighbors. Normalized-IR tests
cover ancestry and abstract forms directly.

`scripts/check-py018-uci.py` verifies the exact three normalized profiles against
unchanged pinned UCI 2.5/2.6 roots and includes, extracts their real declarations
without changing facets, and executes real checked models/codecs inside a
clearly labelled synthetic envelope without Choices. This is isolated real-UCI
profile qualification, **not full-service UCI coverage**.

Actual `AO_CapabilityStatus` service readiness is measured separately, before and
after, including selected/support projection identities. The existing Python
abstract-value boundary at `NITF_PackingPlanPET` prevents a full-service claim;
the task does not widen this unrelated boundary.

`scripts/check-py018-compatibility.py` is the separate acceptance gate for the
40-cell PY-017 success campaign. Historical evidence and its original delta are
not rewritten. Live stages must retain exact artifacts, pins, selected closure
and diagnostic identities. `scripts/test-py018-uci.py` supplies negative controls
for both gates. The existing parity script separately compares 56 selections
and 140 Python sources against the freshly fetched base.

Fast, UCI and Sleet registrations use the existing discovery and source-bound
receipt mechanisms. The pinned Sleet test reuses the existing broker/runtime,
checks actual publish/receive and independent raw wire expectations, malformed
input recovery, unsubscribe/callback silence and teardown. Decoder-entry
instrumentation distinguishes broker rejection from generated-decoder rejection.
The broker's host classification is not the generated model's authority: the
observed broker rejects U+1369 before decoder entry and delivers U+1E950 to the
generated decoder, which correctly rejects it.

## Evidence and reproduction

The starting main is `1c48bcbff22535ef92b870d7a12ef6be7fdbf90b`, tree
`7059845473ad84876760a38995c838341a04bd92`. Versioned PY-018 evidence is separate
from PY-017. Qualification records source identities, file hashes, actual exits
and artifact inventories rather than inferring success from readiness.

Useful focused commands (in addition to the existing owned qualification):

```sh
cargo test --locked -p ams-gra-oms-backend-python
cargo test --locked -p ams-gra-codegen-oms --test python_unicode_strings
cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms --examples
python scripts/test-py018-uci.py
python scripts/check-py018-uci.py --generator target/release/ams-gra-codegen-oms \
  --before-generator /absolute/path/to/base/generator --output /external/new/evidence
```

Set the established pinned root/broker environment variables, use real Python
3.11/3.13 and both normal/optimized execution. No hosted runner, CI lane, source
pin, shared IR or peer backend semantics are changed.