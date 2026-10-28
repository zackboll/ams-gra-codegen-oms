#!/usr/bin/env python3
"""Strict current-service qualification plus immutable PY-006 profile checks.

No downloads. The factored PY-006 driver still defaults to its historical
comparison. This version selects a new, measured, task-owned service baseline;
it does not change either historical fixture or isolated profile oracle.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/fixtures/python-inheritance'
HISTORICAL = {
    'before.json': 'cd8c707460a3da6f3ef5eac8115324a2173440f4726fb553db961873a5782f05',
    'after.json': '7345f45f3cc0a69cd552e48d3c2f04d98aab4e77b24c902ab764057bf2f3f7dc',
}
spec = importlib.util.spec_from_file_location('py006_driver', ROOT / 'scripts/check-py006-uci.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def validate(rows, baseline):
    driver.require(len(rows) == len(baseline) == 24, 'missing/truncated campaign')
    expected = {(v, '{' + driver.OAM + '}' + m, w, c)
                for v in driver.PINS for m in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream']
                for w in ['closed-schema', 'open-extensions'] for c in [False, True]}
    seen = set()
    summaries = []
    for row in rows:
        key = (row['version'], row['message'], row['world'], row['with_codec'])
        driver.require(key in expected and key not in seen, 'duplicate/unknown selection')
        seen.add(key)
        matches = [b for b in baseline if (b['version'], b['message'], b['world'], b['codec']) == key]
        driver.require(len(matches) == 1, 'ambiguous current oracle')
        oracle = matches[0]
        revision, root_hash = driver.PINS[key[0]]
        driver.require(row['revision'] == revision and row['root_sha256'] == root_hash,
                       'source pin mismatch')
        driver.require(row['contract_sha256'] == oracle['contract_sha256'], 'contract identity changed')
        result = row['after']
        driver.require({k: result[k] for k in ['exit', 'stdout', 'stderr']} ==
                       {k: oracle[k] for k in ['exit', 'stdout', 'stderr']}, 'current service delta')
        # A future READY gain must get its own execution and independent vectors;
        # updating just a status/marker or frozen text can never qualify one here.
        driver.require(result['exit'] == 1 and result['stderr'] ==
                       'error: service selection is not renderable for python under ' + key[2] +
                       '; no files were generated\n', 'unqualified READY/process failure')
        text = result['stdout']
        stages = {}
        for label in ['model', 'service api']:
            verdicts = [line for line in text.splitlines() if line.startswith(label + ' status: ')]
            driver.require(verdicts == [label + ' status: NOT READY'], 'missing/ambiguous stage')
            blockers = [line.split(': ', 1)[1] for line in text.splitlines()
                        if line.startswith(label + ' boundary: ')]
            driver.require(len(blockers) == 1 and blockers[0], 'missing/ambiguous first blocker')
            stages[label] = dict(verdict='NOT READY', first_blocker=blockers[0])
        driver.require(text.count('\nstatus: NOT READY\n') == 1, 'missing final verdict')
        if key[3]:
            driver.require(text.count('codec: not measured') == 1, 'false codec qualification')
        stages['codec'] = dict(verdict='NOT MEASURED' if key[3] else 'NOT REQUESTED')
        closures = [line for line in text.splitlines() if line.startswith(
            ('selected oms messages: ', 'selected type closure: ', 'generated support types: '))]
        driver.require(len(closures) == 3, 'missing selected closure')
        summaries.append(dict(version=key[0], message=key[1], world=key[2], with_codec=key[3],
                              process_exit=result['exit'], stages=stages, selected_closure=closures,
                              root_sha256=root_hash, contract_sha256=row['contract_sha256']))
    driver.require(seen == expected, 'missing requested selection')
    return summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    for name, digest in HISTORICAL.items():
        driver.require(driver.digest(ROOT / 'tests/fixtures/python-header-profiles' / name) == digest,
                       'PY-006 historical evidence was modified')
    # This invokes every real service and every isolated profile. The latter
    # retains its exact artifact hashes and four must-execute literal probes.
    subprocess.run([sys.executable, str(ROOT / 'scripts/check-py006-uci.py'),
                    '--generator', str(args.generator), '--output', str(args.output),
                    '--service-baseline', str(FIXTURE / 'after.json')], check=True)
    result = json.loads((args.output / 'measurements.json').read_text())
    baseline = json.loads((FIXTURE / 'after.json').read_text())['measurements']
    summaries = validate(result['services'], baseline)
    driver.require(len(result['isolated_pinned_profiles']) == 8, 'missing isolated qualification')
    # Also execute the inherited Record fixture from the same production binary.
    for world in ['closed-schema', 'open-extensions']:
        dest = args.output.resolve() / ('inherited_' + world.replace('-', '_'))
        generated = driver.check(args.generator.resolve(), FIXTURE / 'oam.xsd',
                                 FIXTURE / 'service.yaml', world, True, dest)
        driver.require(generated['exit'] == 0, str(generated))
        for optimized in [False, True]:
            for style in ['standalone', 'package']:
                probe = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                       [str(FIXTURE / 'probe.py'), str(dest), style],
                                       capture_output=True, text=True)
                driver.require(probe.returncode == 0 and 'Ran 12 tests' in probe.stderr
                               and '\nOK\n' in probe.stderr, probe.stderr)
    summary = dict(services=summaries, historical_fixture_sha256=HISTORICAL,
                   source_baseline=json.loads((FIXTURE / 'after.json').read_text())['source'],
                   isolated_profiles=8, inherited_python_tests=96, real_ready=0)
    (args.output / 'qualification.json').write_text(json.dumps(summary, indent=2) + '\n')
    print('PY007 PINNED UCI AND INHERITANCE: PASSED services=24 isolated_profiles=8 inherited_tests=96 real_ready=0')


if __name__ == '__main__':
    main()