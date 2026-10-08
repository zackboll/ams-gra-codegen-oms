#!/usr/bin/env bash
# Reuse the root already fetched by Deep real-uci; no downloading/generation.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${AMS_GRA_UCI_2_5_ROOT:?already fetched pinned UCI root required}"
python3 - "$AMS_GRA_UCI_2_5_ROOT" <<'PY'
import hashlib, pathlib, sys
assert hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest() == 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27'
PY
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cargo build --release -p ams-gra-codegen-oms
cli="${CARGO_TARGET_DIR:-target}/release/ams-gra-codegen-oms"
contract=tests/fixtures/service-generate/position-report-loop.yaml
"$cli" service-routes --schema "$AMS_GRA_UCI_2_5_ROOT" --contract "$contract" > "$scratch/first.tsv"
"$cli" service-routes --schema "$AMS_GRA_UCI_2_5_ROOT" --contract "$contract" > "$scratch/second.tsv"
cmp "$scratch/first.tsv" "$scratch/second.tsv"
"$cli" service-routes --schema "$AMS_GRA_UCI_2_5_ROOT" --contract "$contract" --format sleet-toml --service-id route-smoke --service-uuid 550e8400-e29b-41d4-a716-446655440071 > "$scratch/service.toml"
python3 - "$scratch" <<'PY'
import pathlib, sys, tomllib
p = pathlib.Path(sys.argv[1])
rows = [r.split('\t') for r in (p/'first.tsv').read_text().splitlines()]
assert len(rows) == 3
assert [(r[2], r[3]) for r in rows[1:]] == [('input', 'subscribe'), ('output', 'publish')]
assert {(r[6], r[7]) for r in rows[1:]} == {('https://www.vdl.afrl.af.mil/programs/oam', 'PositionReport')}
assert [r[5] for r in rows[1:]] == ['mission.position-report'] * 2
assert tomllib.loads((p/'service.toml').read_text()) == dict(service_id='route-smoke', service_uuid='550e8400-e29b-41d4-a716-446655440071', allowed_topics=['mission.position-report'], topic_bindings=[dict(topic='mission.position-report', allowed_messages=['PositionReport'])])
PY
echo 'TASK071 REAL UCI ROUTES: PASSED'