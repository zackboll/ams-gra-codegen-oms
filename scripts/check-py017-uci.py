#!/usr/bin/env python3
"""PY-017 successor: historical authentication, actual selections, executable RDMA.

PY-016's frozen 32/40 oracle is deliberately neither edited nor reinterpreted.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / 'tests/fixtures/python-network-addresses/evidence/v1'
AUTHENTICATION_SHA256 = '1729d4beaf56d7149acc4a1070f82221ca2e1765f134101245d2165d9e37b5ac'


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


measurement = module('py017_measurement', 'measure-py017-uci.py')
driver = measurement.driver
historical = module('py017_historical', 'check-py016-uci.py')


def authenticate(source, expected):
    # Compare frozen production inventories with independently reconstructed
    # immutable Git objects, never with future current source files.
    historical.authenticate(source, expected)


def frozen():
    authentication_file = EVIDENCE / 'source-authentication.json'
    driver.require(driver.digest(authentication_file) == AUTHENTICATION_SHA256,
                   'independent historical authentication changed')
    authentication = json.loads(authentication_file.read_text())
    values = [json.loads((EVIDENCE / (stage + '.json')).read_text()) for stage in ['before', 'after']]
    for stage, value in zip(['before', 'after'], values):
        authenticate(value['source'], authentication[stage])
    return values


def validate(before, after):
    expected = {(version, message, world, codec) for version in driver.PINS
                for message in ['Log', 'PositionReport', 'SubsystemStream', 'SurvivabilityRiskLevel', 'RDMA_InitializeSetup']
                for world in ['closed-schema', 'open-extensions'] for codec in [False, True]}
    stages = []
    summaries = []
    for stage, value in [('before', before), ('after', after)]:
        driver.require(value.get('completed') is True, 'incomplete measurement')
        driver.require(value['schema_includes'] == {
            version: dict(name=f'UCI_SecurityMarkings_v2_{version[-1]}_0.xsd', sha256=digest)
            for version, digest in measurement.INCLUDES.items()}, 'include identity changed')
        rows = value['measurements']
        keys = [(r['version'], r['message'].split('}')[-1], r['world'], r['codec']) for r in rows]
        driver.require(len(rows) == 40 and set(keys) == expected, 'missing/duplicate selections')
        stages.append(dict(zip(keys, rows)))
        for key, row in zip(keys, rows):
            version, message, world, codec = key
            driver.require(row['message'] == '{' + driver.OAM + '}' + message, 'wrong global QName')
            driver.require((row['revision'], row['root_sha256']) == driver.PINS[version], 'wrong release pin')
            driver.require(row['contract_sha256'] == hashlib.sha256(driver.contract(message, version).encode()).hexdigest(),
                           'changed original contract')
            closure = row['selected_type_qnames']
            driver.require(closure and len(closure) == len(set(closure)), 'empty/duplicate QName closure')
            driver.require(row['closure_sha256'] == hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest(),
                           'closure digest changed')
            # This is a frozen acceptance gate, not the measurement driver:
            # only install its READY oracle after the actual campaign passes.
            ready = stage == 'after' or message != 'RDMA_InitializeSetup'
            driver.require(row['exit'] == row['generation']['exit'] == (0 if ready else 1), 'unexpected process status')
            for label in ['model', 'service api'] + (['codec'] if codec and ready else []):
                driver.require(f'{label} status: {"READY" if ready else "NOT READY"}' in row['stdout'],
                               'missing actual readiness status')
            if codec and not ready:
                driver.require('codec: not measured (selected model or service API is NOT READY)' in row['stdout'],
                               'missing unmeasured codec boundary')
            if ready:
                driver.require(set(row['artifacts']) == {'model.py', 'service_api.py'} |
                               ({'service_codec.py'} if codec else set()), 'wrong artifact inventory')
                driver.require(all(len(digest) == 64 for digest in row['artifacts'].values()), 'invalid artifact digest')
            else:
                driver.require(not row['artifacts'] and row['preexisting_generation'] is not None,
                               'missing failure atomicity evidence')
                driver.require(row['preexisting_generation']['exit'] == 1, 'preexisting failure status')
                driver.require('IPv4_AddressType' in row['stdout'] and 'boundary:' in row['stdout'],
                               'missing actual first blocker')
            summaries.append(dict(stage=stage, key=key, ready=ready))
    for key in expected:
        for field in ['selected_type_qnames', 'closure_sha256', 'contract_sha256']:
            driver.require(stages[0][key][field] == stages[1][key][field], 'selection changed: ' + field)
        if key[1] != 'RDMA_InitializeSetup':
            driver.require(stages[0][key]['artifacts'] == stages[1][key]['artifacts'], 'existing READY bytes regressed')
    return summaries


def execute_rdma(generator, output):
    fixture = ROOT / 'tests/fixtures/python-network-addresses'
    records = []
    for release in ['2.5', '2.6']:
        schema = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
        driver.require(driver.digest(schema) == driver.PINS[release][1], 'pinned root mismatch')
        driver.require(driver.digest(schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')) ==
                       measurement.INCLUDES[release], 'pinned include mismatch')
        contract = ROOT / 'tests/fixtures/task103' / ('uci' + release.replace('.', '') + '-service.yaml')
        for world in ['closed-schema', 'open-extensions']:
            for codec in [False, True]:
                dest = output / ('rdma_' + release.replace('.', '') + '_' + world.replace('-', '_') + '_' + str(codec))
                result = driver.check(generator, schema, contract, world, codec, dest)
                driver.require(result['exit'] == 0, str(result))
                for optimized in [False, True]:
                    for style in ['standalone', 'package']:
                        command = [sys.executable] + (['-O'] if optimized else []) + [str(fixture / 'rdma_probe.py'),
                            str(dest), style, release.replace('.', ''), str(ROOT / 'tests/fixtures/service-generate' /
                            ('task092-real-rdma' + release.replace('.', '') + '.json'))]
                        process = subprocess.run(command, capture_output=True, text=True)
                        records.append(dict(release=release, world=world, codec=codec, optimized=optimized,
                                            style=style, command=command, exit=process.returncode,
                                            stdout=process.stdout, stderr=process.stderr,
                                            artifacts={p.name: driver.digest(p) for p in dest.glob('*.py')}))
                        (output / 'rdma-execution.json').write_text(json.dumps(records, indent=2) + '\n')
                        driver.require(process.returncode == 0 and 'Ran 3 tests' in process.stderr,
                                       process.stdout + process.stderr)
    print('PY017 AUTHENTIC GENERATED RDMA: PASSED releases=2 worlds=2 codecs=2 imports=2 policies=2')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--before', type=Path)
    parser.add_argument('--execute-only', action='store_true')
    args = parser.parse_args()
    # Preserve the complete historical test and authenticated frozen fixture.
    historical.validate(*historical.frozen())
    if args.execute_only:
        driver.require(args.generator is not None and args.output is not None, 'execution requires generator/output')
        args.output.mkdir(parents=True, exist_ok=False)
        execute_rdma(args.generator.resolve(), args.output.resolve())
        return
    frozen_before, frozen_after = frozen()
    validate(frozen_before, frozen_after)
    before = json.loads(args.before.read_text()) if args.before else frozen_before
    if args.generator:
        driver.require(args.output is not None, 'live qualification requires output')
        subprocess.run([sys.executable, str(ROOT / 'scripts/measure-py017-uci.py'), '--generator',
            str(args.generator.resolve()), '--closure-tool', str(args.generator.resolve().parent / 'examples/python_datetime_closure'),
            '--source-commit', 'HEAD', '--output', str(args.output.resolve())], check=True)
        after = json.loads((args.output / 'measurements.json').read_text())
        summaries = validate(before, after)
        for measured, oracle in zip(after['measurements'], frozen_after['measurements']):
            for field in ['exit', 'stdout', 'stderr', 'artifacts', 'closure_sha256', 'contract_sha256']:
                driver.require(measured[field] == oracle[field], 'live successor baseline changed: ' + field)
        execute_rdma(args.generator.resolve(), args.output.resolve())
        # Keep the predecessor's isolated-profile, inheritance, header and full
        # service execution. Do not call its obsolete live readiness oracle.
        for name in ['check-py009-uci.py', 'check-py010-uci.py']:
            parent = module('py017_' + name, name)
            parent.historical()
            output = args.output / name.removesuffix('.py')
            output.mkdir()
            parent.profiles(args.generator.resolve(), output)
        for world in ['closed-schema', 'open-extensions']:
            output = args.output / ('inheritance_' + world.replace('-', '_'))
            base = ROOT / 'tests/fixtures/python-inheritance'
            result = driver.check(args.generator.resolve(), base / 'oam.xsd', base / 'service.yaml', world, True, output)
            driver.require(result['exit'] == 0, str(result))
            for optimized in [False, True]:
                for style in ['standalone', 'package']:
                    process = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                        [str(base / 'probe.py'), str(output), style], capture_output=True, text=True)
                    driver.require(process.returncode == 0 and 'Ran 12 tests' in process.stderr, process.stdout + process.stderr)
        subprocess.run(['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms', '--test', 'python_header_profiles'],
                       cwd=ROOT, check=True, env={**os.environ, 'PYTHON': sys.executable})
        subprocess.run([sys.executable, str(ROOT / 'python/qualification/run.py'), 'uci'], check=True)
    else:
        after = frozen_after
        summaries = validate(before, after)
    print('PY017 PINNED UCI DELTA: PASSED before_ready=' + str(sum(r['ready'] for r in summaries if r['stage'] == 'before')) +
          '/40 after_ready=' + str(sum(r['ready'] for r in summaries if r['stage'] == 'after')) + '/40')


if __name__ == '__main__':
    main()