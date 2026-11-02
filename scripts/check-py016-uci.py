#!/usr/bin/env python3
"""PY-016 successor service gate; immutable historical defaults remain unchanged."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / 'tests/fixtures/python-whitespace-visible/evidence/v1'
AUTHENTICATION_SHA256 = 'f86b3035f989d4fb561a1efe860bd30e4930a32296ed8316ecea1b9ca7600092'


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


measurement = module('py016_measurement', 'measure-py016-uci.py')
driver = measurement.driver


def authenticate(source, expected):
    # Independent versioned source inventory oracle, not future live sources
    # and not merely an author-recomputable evidence file manifest.
    for field in ['commit', 'tree', 'generator_sha256', 'closure_tool_sha256', 'base', 'base_tree']:
        driver.require(source[field] == expected[field], 'historical source identity changed: ' + field)
    canonical = json.dumps(source['production_sha256'], sort_keys=True, separators=(',', ':')).encode()
    driver.require(hashlib.sha256(canonical).hexdigest() == expected['production_inventory_sha256'],
                   'historical production inventory changed')


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
            ready = stage == 'after' and message != 'RDMA_InitializeSetup'
            driver.require(row['exit'] == row['generation']['exit'] == (0 if ready else 1), 'unexpected process status')
            text = row['stdout']
            for label in ['model', 'service api']:
                driver.require(f'{label} status: {"READY" if ready else "NOT READY"}\n' in text,
                               'incorrect stage verdict')
            if ready:
                driver.require('model boundary:' not in text and 'service api boundary:' not in text, 'READY with blocker')
                if codec:
                    driver.require('codec status: READY\n' in text, 'unqualified codec')
                driver.require(set(row['artifacts']) == {'model.py', 'service_api.py'} | ({'service_codec.py'} if codec else set()),
                               'wrong generated artifact inventory')
                driver.require(all(len(digest) == 64 for digest in row['artifacts'].values()), 'invalid artifact hash')
                driver.require(row['preexisting_generation'] is None, 'unexpected rejection evidence for READY')
            else:
                blocker = ('IPv4_AddressType' if message == 'RDMA_InitializeSetup' else 'WhitespaceVisibleString1024Type')
                for label in ['model', 'service api']:
                    driver.require(f'{label} boundary: {{{driver.OAM}}}{blocker}: unsupported Python IR construct: unsupported constraints\n' in text,
                                   'unexpected measured blocker')
                if codec:
                    driver.require('codec: not measured' in text, 'false codec qualification')
                driver.require(row['artifacts'] == {}, 'partial rejected artifacts')
                existing = row['preexisting_generation']
                driver.require(existing and existing['exit'] == 1, 'missing pre-existing atomic check')
                for field in ['stdout', 'stderr']:
                    driver.require(existing[field] == row[field], 'pre-existing output changed readiness')
            summaries.append(dict(stage=stage, version=version, message=message, world=world, codec=codec, ready=ready))
    for key in expected:
        for field in ['selected_type_qnames', 'closure_sha256', 'contract_sha256']:
            driver.require(stages[0][key][field] == stages[1][key][field], 'closure/contract changed across implementation')
    return summaries


def frozen():
    before, after = [json.loads((EVIDENCE / (stage + '.json')).read_text()) for stage in ['before', 'after']]
    authentication_file = EVIDENCE / 'source-authentication.json'
    driver.require(driver.digest(authentication_file) == AUTHENTICATION_SHA256,
                   'independent historical authentication changed')
    authentication = json.loads(authentication_file.read_text())
    for stage, value in [('before', before), ('after', after)]:
        authenticate(value['source'], authentication[stage])
    return before, after


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    before, after = frozen()
    summaries = validate(before, after)
    if args.generator:
        driver.require(args.output is not None, 'live qualification requires output')
        subprocess.run([sys.executable, str(ROOT / 'scripts/measure-py016-uci.py'),
                        '--generator', str(args.generator.resolve()), '--closure-tool',
                        str(args.generator.resolve().parent / 'examples/python_datetime_closure'),
                        '--source-commit', 'HEAD', '--output', str(args.output.resolve())], check=True)
        live = json.loads((args.output / 'measurements.json').read_text())
        validate(before, live)
        for measured, oracle in zip(live['measurements'], after['measurements']):
            for field in ['exit', 'stdout', 'stderr', 'artifacts', 'closure_sha256', 'contract_sha256']:
                driver.require(measured[field] == oracle[field], 'live successor baseline changed: ' + field)
        # Retain prior isolated-profile execution as well as new full services.
        for name in ['check-py009-uci.py', 'check-py010-uci.py']:
            parent = module('py016_' + name, name)
            parent.historical()
            output = args.output / name.removesuffix('.py')
            output.mkdir()
            parent.profiles(args.generator.resolve(), output)
        # Preserve PY-007's executable inheritance and original header-profile
        # checks rather than just deleting those executions from current CI.
        for fixture, count in [('python-inheritance', 12)]:
            for world in ['closed-schema', 'open-extensions']:
                output = args.output / (fixture.replace('-', '_') + '_' + world.replace('-', '_'))
                base = ROOT / 'tests/fixtures' / fixture
                schema = base / 'oam.xsd'
                if not schema.exists():
                    schema = base / 'root.xsd'
                result = driver.check(args.generator.resolve(), schema, base / 'service.yaml', world, True, output)
                driver.require(result['exit'] == 0, str(result))
                for optimized in [False, True]:
                    for style in ['standalone', 'package']:
                        result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                            [str(base / 'probe.py'), str(output), style], capture_output=True, text=True)
                        driver.require(result.returncode == 0 and f'Ran {count} tests' in result.stderr,
                                       result.stdout + result.stderr)
        subprocess.run(['cargo', 'test', '--locked', '-p', 'ams-gra-codegen-oms',
                        '--test', 'python_header_profiles'], cwd=ROOT, check=True,
                       env={**os.environ, 'PYTHON': sys.executable})
        subprocess.run([sys.executable, str(ROOT / 'python/qualification/run.py'), 'uci'], check=True)
    print('PY016 PINNED UCI DELTA: PASSED before=40 after=40 ready_selections=' +
          str(sum(row['ready'] for row in summaries)) + '; executable qualification required separately')


if __name__ == '__main__':
    main()