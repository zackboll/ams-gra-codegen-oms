# PY-004 — Generated Python OMS JSON payload codecs

## Provenance and evidence gate

Fetched `origin/main` and PR heads before implementation, rechecking PR #100
immediately before creating `feature/py-004-python-oms-json-codecs`:

| Ref | Exact SHA |
| --- | --- |
| main | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| PR #98 | `93d02dba9bdc398453a29b5dd3b9956ae26793fb` |
| PR #99 | `05d2ac17ec352d6b3b6636d86050245a16dcc7f7` |
| PR #100 / PY-004 base | `697617f889a147661142cf244467c2ed6232c928` |

All three PRs were open, mergeable, with no recorded review approval and checks
pending. #100's Python, Rust and real-Sleet checks had passed. The branch stacks
on #100's exact head, not a merge or an invented approval; earlier branches are
not modified. Final candidate SHA and hosted status are recorded in the PR.

Reviewed Tasks 050, 051, 052, 073, 077 and PY-001–003. The Rust codec and its
vectors are implementation evidence, not an authority overriding the standard.
Fetched the official OMSC-SPC-013 Rev B DOCX from OMS standard commit
`726272bd0390982a759c91a9cf4e13b81c2b510b`, path
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`.
Verified SHA-256:
`b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
Read `word/document.xml` directly:

- §6.1.2: element members are bare for the exact OAM namespace, Clark-qualified
  otherwise; maxOccurs > 1 produces arrays; derived complex values use `$type`.
- §6.1.3: concrete complex values are objects. The optional repeated `B` example
  explicitly permits both `"B": []` and omission.
- §6.1.4: Boolean → Boolean; primitive decimal → number. XSD integers derive from
  decimal, so supported integral values are JSON numbers, never strings or bools.
  The special OAM `UniversallyUniqueIdentifierType` mapping is RFC 4122 and is
  **not** established by an unconstrained Python String carrier; it fails closed.
- Task 051's QName provenance, not host identifiers or a parent namespace guess,
  supplies each field's wire identity. Absent/empty member namespace fails closed.
- Tasks 073/077: unknown subtype possession is not validation authority; reject
  `$type` for this non-polymorphic subset. Reject duplicate decoded member strings
  before dict collapse, including nested unknown subtrees.

## Generation and readiness

```sh
cargo run --locked -p ams-gra-codegen-oms -- service-check \
  --schema tests/fixtures/python-codec/oam.xsd \
  --contract tests/fixtures/python-codec/service.yaml \
  --language python --world closed-schema --with-codec
cargo run --locked -p ams-gra-codegen-oms -- service-generate \
  --schema tests/fixtures/python-codec/oam.xsd \
  --contract tests/fixtures/python-codec/service.yaml \
  --language python --world closed-schema --with-codec --output /tmp/py-codec
```

The shared PY-003 render-backed path has an additive opt-in codec renderer hook.
Projection happens once, ServiceApiModel lowering happens once; model, API and
codec renderers all consume that exact selected schema/model. Codec measurement
occurs only after model/API succeed. Combined paths validate before any write.
Readiness and generation use the same actual codec renderer. There is no Python
addition to the legacy three-language coverage inventory or language tables.

Reports retain separate model/API/codec stages. An unqualified-member service
can be model/API READY but codec and aggregate NOT READY. Unsupported models
report codec not measured. Without `--with-codec`, PY-003 behavior and bytes are
unchanged. Zero OMS needs no codec and never emits `service_codec.py`.

Successful OMS generation emits `model.py`, `service_api.py`, `service_codec.py`.
The first two are byte-identical with and without the flag. Imports work both
standalone from the output directory and package-relative. Standard library only.

## Public Python API and supported mappings

For each **unique selected payload type**, exactly one pair is generated in
contract first-occurrence order (shared payloads are not duplicate functions):

```python
def encode_Payload(payload: model.Payload) -> str: ...
def decode_Payload(text: str) -> model.Payload: ...
```

`Payload` above is the existing Python backend's normalized, case-preserving
type symbol. Signatures name the actual existing model classes. The functions
operate on **payload bodies**, not global-element envelopes or routes.

| Construct | Wire and validation |
| --- | --- |
| Boolean | exact JSON Boolean / exact Python bool |
| signed/unsigned integer | integral JSON number / exact int, arbitrary precision; inclusive domains reused from model |
| unconstrained String | JSON string, no coercion; Unicode scalar strings, UTF-8 encodable |
| named scalar and alias chains | unwrap/rewrap every existing frozen wrapper, preserving exact types |
| enumeration | exact XSD wire value, not Python symbol or enum representation |
| concrete Record | object with explicit generated per-field mapping and schema order |
| required field | must exist, correct type; null rejected |
| optional non-nillable field | missing → None; present → validated value, never null |
| bounded repetitions | JSON array ↔ tuple; exact min/max and every item checked |
| nested Record / repeated Record | recursive generated type-specific mapping |

The renderer emits explicit encode/decode functions per declaration. No
`dataclasses.asdict`, host-field introspection, generic dataclass serializer,
handwritten per-message mapping, or model representation change is involved.

## Strict JSON and deterministic encoding

Decode accepts **str**, never an already-parsed dict or bytes. Standard-library
`json.loads` validates syntax/trailing content, `object_pairs_hook` rejects
duplicate strings **after escape decoding**, and numeric hooks reject floats,
exponents and non-standard constants. Integer tokens are accumulated in short
decimal chunks; no float conversion or process-global int-string limit change.
Integer-valued float/exponent spellings are deliberately not accepted by this
bounded integral codec. No claim is made to every legal decimal lexical form.

Unknown members and **all** `$type` occurrences are rejected at each Record.
Keys and String values reject lone surrogate code points; valid escaped surrogate
pairs and Unicode scalar text work. Both absence and `[]` decode to `()` for a
min-zero repetition; the encoder canonically omits empty tuples. This presence
distinction cannot be retained without changing the existing model. Optional
singletons preserve absence versus explicit empty String/Record presence.

Encode validates exact payload/model types, recursive fields, wrapper values,
primitive domains and tuple cardinalities. Members are emitted in schema order,
optional None and empty tuples omitted. Output uses compact separators and
`ensure_ascii=False`; standard-library string escaping is explicit. Integral
text formatting uses decimal chunks to preserve >4300-digit integers on 3.11+.
No NaN/infinity/float value can enter the generated wire tree. UTF-8 bytes are
`encode_Payload(value).encode('utf-8')`; the codec itself returns Unicode str.

`CodecError(ValueError)` documents malformed payload/type/domain failures with
schema-derived paths where available. Only JSON syntax and Unicode encoding
exceptions are translated at their narrow boundaries. Programming errors and
resource limits (e.g. recursion/memory exhaustion) are not caught as malformed
payloads. There is no full XSD validation or new security/resource sandbox.

## Explicit limits

Float32/64, decimal, binary (including lexical provenance), temporal types,
Choice, abstract/polymorphic values, unsupported structural inheritance,
nillability, unbounded repetition, unsupported String facets, OAM UUID special
mapping, unknown members/discriminators and unqualified element members remain
unsupported. The Python model capability is not expanded. Existing frontend
rejections (notably decimal) remain errors before readiness, not fake verdicts.

No transport, sockets, OWP, Sleet, reconnection, subscriptions, global envelopes
or dynamic schema/subtype authority. No pinned-UCI or broker qualification is
claimed for Python codecs.

## Qualification and parity

- Independently stored OAM and Clark-qualified non-OAM positive vectors, plus
  negative vector tables; non-OAM local form overrides an unqualified default.
- Both worlds, Python 3.11/3.13, normal/`-O`, standalone/package imports; generated
  files compile/import and resolved type hints name the original model classes.
- Exact expected wire text (not only round-trip), nested/repeated Records,
  schema ordering, empty arrays, optional omission/presence, i64/u64 boundaries,
  arbitrary precision beyond Python's default string-digit limit, Unicode,
  escaped duplicates, malformed/trailing JSON, nulls, unknown fields/$type,
  every shape/cardinality boundary and encoder exact-type revalidation.
- Normalized-IR alias chain/scalar-payload probe: the frontend does not expose
  alias construction, so the existing model facility is tested without expanding
  the frontend. Multiple exchange occurrences sharing a payload emit one pair.
- NOT READY codecs preserve absent and populated output destinations. Zero-OMS
  and PY-003 selected-service regressions remain active.
- Fast CI matrices Python 3.11 and 3.13, explicitly runs the codec CLI suite and
  backend probes, plus formatting, locked Clippy and builds.
- `check_codec_parity.py` compares exact #100 baseline and candidate without the
  codec flag: eight Python model/API runs and reports plus existing Ada/Rust/C++
  full/selected artifacts, readiness and coverage evidence. Bounded synthetic
  parity is separate from exhaustive real-UCI qualification.

Exact candidate SHA, totals, parity digest and hosted statuses are in the PR.