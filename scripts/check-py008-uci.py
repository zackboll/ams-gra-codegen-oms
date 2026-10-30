#!/usr/bin/env python3
"""Versioned PY-008 current-service gate, preserving PY-006/007 history.

Reuse the existing source-qualified service driver, exact profile artifacts and
targeted generated inherited Record execution. No second service resolver, no
downloads, and no READY qualification by editing frozen status text.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/fixtures/python-float64'
HISTORICAL = {
    'python-header-profiles/before.json': 'cd8c707460a3da6f3ef5eac8115324a2173440f4726fb553db961873a5782f05',
    'python-header-profiles/after.json': '7345f45f3cc0a69cd552e48d3c2f04d98aab4e77b24c902ab764057bf2f3f7dc',
    'python-inheritance/before.json': 'f9b56c017187d84e75c182a83cd5ebfebb7d568e596b15167aa953c1e5726233',
    'python-inheritance/after.json': 'b80985470ec6757eed65db1683cca08c5f24f3b191edf4ca04cb2d24d5e5d25c',
}
spec = importlib.util.spec_from_file_location('py007_qualification', ROOT / 'scripts/check-py007-uci.py')
parent = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parent)


def validate(rows, baseline):
    # Parent enforces complete unique identities, exact pins/contracts, exact
    # stdout/stderr/exit, stage/first-blocker presence and no unqualified READY.
    summaries = parent.validate(rows, baseline)
    for row in rows:
        message = row['message'].split('}', 1)[1]
        expected_contract = hashlib.sha256(parent.driver.contract(message, row['version']).encode()).hexdigest()
        parent.driver.require(row['contract_sha256'] == expected_contract,
                              'service contract does not match the exact qualified selection')
    return summaries


def historical():
    for name, digest in HISTORICAL.items():
        parent.driver.require(parent.driver.digest(ROOT / 'tests/fixtures' / name) == digest,
                              'historical evidence modified: ' + name)
    baseline = json.loads((ROOT / 'tests/fixtures/python-inheritance/after.json').read_text())['measurements']
    rows = [dict(version=b['version'], revision=b['revision'], root_sha256=b['root_sha256'],
                 message=b['message'], world=b['world'], with_codec=b['codec'],
                 contract_sha256=b['contract_sha256'],
                 after={key: b[key] for key in ['exit', 'stdout', 'stderr']}) for b in baseline]
    parent.driver.require(len(parent.validate(rows, baseline)) == 24, 'historical PY-007 assertions missing')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--existing-output', action='store_true',
                        help='Validate the immediately preceding versioned parent campaign without repeating it')
    parser.add_argument('--service-baseline', type=Path,
                        help='Measured successor baseline; historical defaults remain immutable')
    args = parser.parse_args()
    baseline_path = args.service_baseline or FIXTURE / 'after.json'
    historical()
    if not args.existing_output:
        subprocess.run([sys.executable, str(ROOT / 'scripts/check-py007-uci.py'),
                        '--generator', str(args.generator), '--output', str(args.output),
                        '--service-baseline', str(baseline_path)], check=True)
    result = json.loads((args.output / 'measurements.json').read_text())
    baseline = json.loads(baseline_path.read_text())
    parent.driver.require(baseline.get('completed') is True, 'incomplete frozen campaign')
    summaries = validate(result['services'], baseline['measurements'])
    inherited = json.loads((args.output / 'qualification.json').read_text())
    parent.driver.require(inherited['inherited_python_tests'] == 96
                          and inherited['isolated_profiles'] == 8 and inherited['real_ready'] == 0,
                          'missing inherited/profile execution')
    parent.driver.require(inherited['current_generator_sha256'] == parent.driver.digest(args.generator),
                          'current execution generator identity mismatch')
    # Frozen BEFORE is a new measurement from the verified parent executable,
    # never a rewritten historical result. Require its exact PY-007 verdicts.
    before = json.loads((FIXTURE / 'before.json').read_text())
    parent.driver.require(before.get('completed') is True, 'incomplete BEFORE campaign')
    parent.driver.require(before['source'] == dict(
        dependency='d76a36d2cb9d18bcc8b825b5134a65686f361deb',
        tree='bb28f2e70e0c2b1c75ed19ff390f8c1df40773c3'), 'BEFORE dependency identity mismatch')
    parent.validate(before['services'], json.loads(
        (ROOT / 'tests/fixtures/python-inheritance/after.json').read_text())['measurements'])
    summary = dict(completed=True, version='PY-008', services=summaries,
                   historical_sha256=HISTORICAL, inherited_tests=96, isolated_profile_executions=32,
                   real_ready=0, interoperability='NOT QUALIFIED: complete services remain NOT READY')
    (args.output / 'py008-qualification.json').write_text(json.dumps(summary, indent=2) + '\n')
    print('PY008 PINNED UCI: PASSED services=24 inherited_tests=96 profile_executions=32 real_ready=0')


if __name__ == '__main__':
    main()
