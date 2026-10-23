# PY-003 — Python selected-service readiness and typed API

## Provenance and scope

Fetched and verified before implementation:

| Ref | Exact SHA |
| --- | --- |
| `origin/main` | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| PR #98 head | `93d02dba9bdc398453a29b5dd3b9956ae26793fb` |
| PR #99 head | `05d2ac17ec352d6b3b6636d86050245a16dcc7f7` |

Both dependencies were OPEN, MERGEABLE, with pending checks and no recorded
review approval. Therefore this branch stacks on PR #99's exact current head,
not a claimed approved head; neither dependency branch is modified. The PR base
is `feature/py-002-python-cli-integration`. Final hosted status and candidate
SHA belong in the delivery PR, not a prediction in this document.

Reviewed Tasks 030–032 (shared plan, projection, readiness), 047 (lowering,
payload binding, naming and artifact preflight), 048 (operation mapping and
message identity), 049 (adapter/transport separation), and 050 (codec boundary).

## Authority and readiness

```text
ServicePlan + SchemaIr + GenerationWorld
    -> project_service_generation_schema
    -> build_service_api_model (once)
    -> actual PythonBackend model + service API renderers
    -> validate combined artifact paths
    -> check: report only / generate: write already-rendered files
```

The additive core `analyze_rendered_service_readiness` accepts a `Backend` and
uses its real `name()` identity and real renderers, rather than adding Python
to the independently qualified `BackendLanguage::ALL` reporting inventory.
Ada/Rust/C++ coverage ordering and readiness implementations are untouched.
Python capability and naming rules have exactly one implementation, in its
backend. This path trades inexpensive in-memory rendering for a verdict that
cannot diverge from generation. No file is written by readiness.

Only the shared selected projection reaches Python checks. Unselected Choice,
unsupported primitives, and unrelated declarations cannot block a supported
selection. Projection/binding integrity defects remain errors; unsupported
world/emission shapes become deterministic NOT READY boundaries. Python's
PY-001 model subset is unchanged, including its fail-closed restrictions on
abstract values, constraints, inheritance, nillability and unbounded fields.

Reports distinguish `model status`, `service api status`, optional codec status,
and the aggregate `status`. Blockers are deterministic first failures from the
real backend, not a speculative feature matrix or borrowed language verdict.

## Generated API

Python 3.11+ and standard library only. A supported selection emits `model.py`
and `service_api.py`; zero OMS exchanges emit **only** `service_api.py`.
Both standalone imports from the output directory and package-relative imports
are supported, with no fallback that hides a genuine model import error.

```sh
cargo run --locked -p ams-gra-codegen-oms -- service-check \
  --schema tests/fixtures/python-service/root.xsd \
  --contract tests/fixtures/python-service/service.yaml \
  --language python --world closed-schema
cargo run --locked -p ams-gra-codegen-oms -- service-generate \
  --schema tests/fixtures/python-service/root.xsd \
  --contract tests/fixtures/python-service/service.yaml \
  --language python --world closed-schema --output /tmp/python-service
python3.11 -m py_compile /tmp/python-service/model.py /tmp/python-service/service_api.py
```

Public root constants: `SERVICE_NAME`, `SERVICE_VERSION`, `SERVICE_KIND`.
Each `function_<normalized-id>` class carries `ID`, original `NAME`, and ordered
`EXCHANGES`; `FUNCTIONS` preserves contract function order. Each nested
`exchange_<normalized-id>` class retains `ID`, `KIND`, `DIRECTION`, `MANDATE`.
OMS endpoints additionally carry `TOPIC`, `MESSAGE_NAMESPACE`, `MESSAGE_NAME`,
`SUBSCRIPTION_GROUP`, and `Payload` bound to the actual generated model class.
Repeated OMS occurrences are never deduplicated into one endpoint.

IDs use the existing Python ASCII substitution/lowercase/keyword policy, with
fixed `function_` / `exchange_` prefixes. Scope-local collisions fail before
output; no suffixes silently change identity. Function display names are never
used as identifiers. Model symbols are qualified through private `_model`, so
they cannot hide wrapper symbols. Fixed names and generated scope prefixes are
disjoint. `model.py` and `service_api.py` have distinct fixed artifact paths.

OMS endpoints expose precisely the operation already decided in the shared
`ServiceApiModel`: Output → `publish`; Input → `subscribe`. Adapters are
structural generic `PublishAdapter[Payload, Result]` and
`SubscribeAdapter[Payload, Result]` protocols. Methods are keyword-only:

- Publish: `message_namespace`, `message_name`, `topic`, `payload`.
- Subscribe: the same route plus `subscription_group`, `handler` (no payload).

Endpoint signatures name the concrete model payload. Return values and
exceptions pass through unchanged. Python annotations do not themselves enforce
runtime type checking. The model retains its existing explicit runtime checks.
The facade never calls the handler, creates a subscription handle, opens a
socket, serializes, or interprets an OWP frame.

## Codec and failure boundaries

Historical PY-003 codec boundary below; [PY-004](task-py-004-python-oms-json-codecs.md)
adds qualified, opt-in payload codecs without changing model/API output.

There is **no Python OMS JSON codec**. For any OMS selection, `--with-codec`
sets aggregate status and codec status to NOT READY and emits no files, even
if model and API status are READY. Zero-OMS `--with-codec` remains successful,
reports no codec required, and produces byte-identical wrapper source.

All files render in memory and combined paths validate before filesystem writes.
Capability, naming, world, binding, and codec failures leave both absent and
populated destinations unchanged. As with existing backends this does not promise
transactional rollback for unrelated I/O failures during the filesystem writer.

## Qualification

`crates/cli/tests/python_service.rs` executes the production binary and the
Python unittest probe normally and under `-O`, standalone and package imports,
in both worlds. It covers compilation, resolved type hints, routing/endpoint
metadata, repeated occurrences, same-payload distinct message identities,
mock adapter forwarding/returns/exceptions, zero OMS, selected versus unselected
unsupported types, normalized function/exchange/model collisions, abstract
values, codec rejection, no-write failures and deterministic output.
Python must execute; missing interpreters are failures, not skips.

The focused hosted job explicitly uses Python 3.11 and runs both Python CLI
suites plus backend tests, formatting, locked Clippy and builds. The existing
workspace jobs retain their GNAT and transport qualification gates.

`crates/backend-python/tests/check_service_parity.py` compares exact baseline
and candidate artifacts/reports: six full models, twelve selected model/wrapper
runs (including zero OMS), twelve ready/unready reports, and two coverage
reports across Ada/Rust/C++ and both worlds. This is bounded fixture parity,
not exhaustive pinned-UCI coverage. Exact execution totals and evidence digests
are recorded in the delivery PR.