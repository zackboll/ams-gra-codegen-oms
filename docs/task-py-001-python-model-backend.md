# PY-001 — Initial Python model backend

## Scope and baseline

Starting `origin/main`, fetched before implementation:
`5f6800d505c6501638c322d20b23cfc36b09d7d6`.
Branch: `feature/py-001-python-model-backend`.

The new `ams-gra-oms-backend-python` crate implements the existing `Backend`
trait. Its `PythonBackend::generate` returns one standalone `model.py`, targeting
Python 3.11+, with only standard-library imports. The convenience `generate`
function returns the source string. **Both APIs require `GenerationWorld`; there
is no default or implicit closed-schema assumption.**

The backend consumes normalized `SchemaIr`. It calls shared
`plan_type_emissions(schema, world)`, including semantic validation, dependency
ordering and open-world guards. It does not parse XML, resolve service contracts,
or implement another dependency planner. Python-specific preflight and naming
live only in the new crate, without borrowing another language's naming policy.

No Python language option is added to the public CLI. Shared IR, backend coverage
inventories, readiness classifications, service resolution and Ada/Rust/C++
backends are unchanged. PY-002 must establish truthful Python-specific readiness
before CLI integration.

## Implemented subset

| Construct | Generated representation / enforcement |
| --- | --- |
| Boolean | Exact `bool`, never integer/string coercion |
| Unconstrained String | Exact `str`, spelling retained |
| Signed / unsigned integer | Exact `int`; `bool` explicitly rejected by exact-type check |
| Inclusive integral bounds | Shared `inclusive_integral_domain`; inclusive runtime checks |
| Unbounded signed / unsigned integer | Arbitrary-precision `int`; unsigned requires `>= 0` |
| Named primitive scalar | Frozen, slotted typed dataclass with checked `value` |
| Named scalar alias | Frozen wrapper around the declared primitive or named scalar; alias chains use the shared dependency order |
| String enumeration | `Enum` with `@unique`; every `.value` is the exact schema value |
| Concrete Record | Frozen, slotted, keyword-only typed dataclass |
| Required non-nillable field | Required constructor argument with exact-type check; `None` rejected |
| Optional non-nillable field | `T \| None`, default `None`; absence only, not XML nil |
| Bounded repetition | Immutable `tuple[T, ...]`; inclusive minimum/maximum length and every item checked |

Bounded integral facets use the existing supported signed `i64` / unsigned `u64`
domains. One-sided authored bounds, oversized bounded domains, exclusive bounds
and other unsupported facet combinations fail closed. Truly unconstrained
integers use Python's arbitrary precision rather than inventing a machine bound.
Direct primitive field constraints and named primitive constraints follow the
same integral classifier. Intrinsic frontend bounds such as `xs:byte` and
`xs:unsignedByte` are retained and enforced.

Repeated fields require a tuple; lists are rejected, not converted. A repetition
with minimum zero defaults to `()`, while a positive minimum requires an argument.
Singleton optional/required cardinalities remain singleton representations.
`TypeError` reports incorrect types and `ValueError` reports domain/cardinality
violations. Validators use explicit checks, not Python `assert`, and remain
active under `python -O`.

Normal assignment cannot invalidate a model: dataclasses are frozen and tuples
are immutable. This is not a sandbox against deliberate Python reflection,
`object.__setattr__`, manually calling `__new__`, or monkey-patching generated
classes. Models are not intended as a security isolation boundary.

### Naming and schema identity

Type and enum symbols retain ASCII case; field symbols lowercase ASCII letters.
Non-ASCII/non-identifier characters map to `_`; empty, non-letter-leading or
keyword symbols receive `schema_`. Normalization collisions fail closed rather
than receiving unstable suffixes. Dunder/sunder names cannot enter Python's
special namespaces. Referenced builtins, helper-local type collisions, enum API
names and class-body annotation shadowing are rejected by Python's own preflight.

Types expose `__schema_name__ = (namespace_uri, original_local_name)`.
Record fields carry original `schema_name` and `schema_namespace` in dataclass
field metadata. Enum values preserve their exact source strings, including
punctuation, escapes and Unicode. Metadata is future codec input, **not an OMS
JSON mapping implementation**. Source paths and timestamps are not emitted.

## Real Track example

The existing `crates/xsd-frontend/tests/fixtures/track.xsd` is loaded through the
existing frontend without changes. The checked-in generated example is
`crates/backend-python/tests/expected/track.py`.

From the repository root, generate without adding a CLI language option:

```sh
cargo run --locked -q -p ams-gra-oms-backend-python --example track > /tmp/model.py
cd /tmp
python3.11 - <<'PY'
from model import Track, Track_Id, Track_Quality

track = Track(id=Track_Id(1), quality=Track_Quality.Confirmed,
              callsign="EAGLE", sensor_ids=(12, 42))
assert track.id.value == 1
assert track.quality.value == "Confirmed"
assert Track(id=Track_Id(65535), quality=Track_Quality.Unknown).callsign is None

try:
    Track_Id(True)  # rejected despite bool subclassing int
except TypeError:
    pass
else:
    raise RuntimeError("expected rejection")
PY
```

`Track_Id` admits exactly `1..65535`. `Track_Quality` stores exactly `Unknown`,
`Tentative`, or `Confirmed`. `Track` requires typed `id` and `quality`, defaults
`callsign` to `None`, and allows zero through eight integer `sensor_ids`.

## Explicitly unsupported

Generation fails before returning artifacts for:

- abstract declarations and abstract structural value representations, including
  closed-world absent-only elisions; polymorphism and inheritance;
- abstract value references under `OpenExtensions` (the shared planner retains
  its authoritative open-world error); closed sums are not substituted;
- Choice, named List declarations, unbounded repeated fields, nillability;
- Decimal, Float32/64, Binary, DateTime, Time and Duration;
- patterned/length-constrained/whitespace-constrained strings, numeric patterns,
  exclusive bounds, partial inclusive ranges, and unsupported constraints on
  aliases, enum declarations, records or named field references;
- named restriction ancestry, aliases to records, recursive dependencies,
  unresolved references and invalid IR;
- multiple/absent schema namespaces and unsafe/colliding generated names.

Primitive base provenance on a matching primitive declaration, and String base
provenance on an enum, are supported; these are not structural inheritance.
Messages remain IR metadata, not generated envelopes or service APIs.
`generate_service_api` unconditionally returns the explicit error
`service API generation is unsupported in PY-001`, with no placeholder source.

There is **no OMS JSON codec readiness claim**, JSON codec, WebSocket client,
OWP transport, Sleet adapter, service façade or deployment support in this task.
Model validation must not be confused with wire-format conformance.

## Qualification

Tests require an available Python 3.11+ interpreter (`PYTHON` overrides `python3`);
they do not silently skip. Each generated module is compiled with `py_compile`,
imported in isolation, and has its type hints resolved. Eight Python unittest
methods run under both normal and optimized execution (16 executions per
interpreter), covering valid/invalid construction, numeric boundaries,
bool-versus-int, exact enums, required/optional fields, nested records, positive
and zero repeated minima/maxima, immutability, metadata and rejection of
polymorphic values.

Rust qualification comprises three backend unit tests and seven integration
tests, including deterministic golden source, relocation independence, explicit
world behavior, shared dependency planning, service API rejection, negative XSD
fixtures and unsupported IR/name matrices. Negative tests place a normal output
sink after the backend and verify its directory stays empty on every failure in
both worlds.

The positive `scalars.xsd` uses the frontend's supported `xs:unsignedInt`.
`xs:unsignedLong` is not supported by the existing frontend; the full `u64`
backend boundary is exercised through normalized IR, without expanding frontend
scope. Scalar aliases and arbitrary-precision unsigned integers are likewise
tested directly through the existing IR facilities.

Local evidence:

- Python 3.11.17 and 3.13.5: all 10 Rust tests and all 16 Python unittest
  executions per interpreter pass, including `-O` runs.
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo build --locked --workspace --all-targets`: pass.
- `bash scripts/check-ci-split.sh`: pass.
- `cargo test --locked --workspace`: pass, 1,438 passed, zero failed, eight
  existing ignored tests across 179 test/doc-test suites. Compiler-backed Ada,
  Rust and C++ tests executed; externally gated/pinned work is not inferred
  from ignored or environment-gated tests.
- `cargo +1.95.0 check --locked -p ams-gra-oms-backend-python --all-targets`:
  pass on the existing CI compatibility toolchain.

Fast CI adds a focused `python-model-backend` job with `actions/setup-python`
selecting Python 3.11. It executes all backend tests (including required Python
runtime probes), formatting, locked Clippy and a locked build. Existing jobs are
unchanged. Remote CI state is reported in PR delivery rather than presumed here.

### Existing backend/source/readiness parity

Built the recorded main SHA in a detached worktree and compared its CLI against
the candidate using identical absolute fixture paths:

```sh
python3 crates/backend-python/tests/check_existing_backends.py \
  /tmp/py001-baseline-target/debug/ams-gra-codegen-oms \
  target/debug/ams-gra-codegen-oms /tmp/py001-regression/evidence.json
```

Result: **PASS**. Ada/Rust/C++ Track source artifacts (ten files across six
generation runs), twelve established ready/unready service reports and two
coverage reports are identical across both generation worlds. Exit statuses,
stdout and stderr are compared for readiness. Canonical candidate evidence
SHA256 with this checkout's fixture paths:
`371b0223821c039ccbff57a1e5e1d99b06f010f5e820a10814a84bb9633d8ac2`.
The path-dependent digest is supporting evidence, not a portable golden test.
The comparison script is committed so the probe can be reproduced.

No existing backend, IR, shared readiness/coverage or service-resolution source
is modified. Existing golden/readiness tests are retained in workspace
qualification; this bounded fixture comparison is not a claim to have run the
separate pinned-UCI/Sleet Deep CI qualification.