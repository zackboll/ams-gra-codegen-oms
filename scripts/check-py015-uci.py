#!/usr/bin/env python3
"""Authenticate frozen PY-015 measurements, independently of future live sources."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py015_measurement', ROOT / 'scripts/measure-py015-uci.py')
measurement = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measurement)
driver = measurement.driver
SOURCES = {
    'before': ('4f395938a700afd3a72f2b233c501031fe08ad1f',
               '96f0cda8c382f95cbf8246b36f8145a359470ee6',
               'cadf0c96ef587e5c1bfea82170213e32d43c7e8c1522493a84cc2b590e50c1c2'),
    'after': ('4ff5237344c274dd5f977e630c1a64222eb52042',
              'ba2fdd4c01ab9e9e711295dc23b573526d07312d',
              'b4412742664eb2920b8abb6edde5140ad3bc2ac3f07dc38bb1d5e124d2c9fa21'),
}
GENERATORS = {
    'before': '95d6f46448ff83c174c58c51ca97c1bac344f96d3e626edd748636057f813203',
    'after': 'ebdd86c56ca32d163dbf43f92ce37f607d8e793ab4544df9a56cdc7fbbce4f6b',
}
CLOSURE_TOOL = '810a773e270496cb56245762b87583f6fa51759259014f0f53220e70ac770dae'


def boundary(stage, version, message):
    if message == 'RDMA_InitializeSetup':
        return 'IPv4_AddressType: unsupported Python IR construct: unsupported constraints'
    if stage == 'before':
        return 'FGI_SourceOpenChoiceType: unsupported Python IR construct: Choice / List declarations'
    return 'WhitespaceVisibleString1024Type: unsupported Python IR construct: unsupported constraints'


def authenticate(source, stage):
    commit, tree, inventory = SOURCES[stage]
    driver.require((source['commit'], source['tree']) == (commit, tree), 'historical source identity changed')
    canonical = json.dumps(source['production_sha256'], sort_keys=True, separators=(',', ':')).encode()
    driver.require(hashlib.sha256(canonical).hexdigest() == inventory, 'historical production inventory changed')
    driver.require(source['generator_sha256'] == GENERATORS[stage]
                   and source['closure_tool_sha256'] == CLOSURE_TOOL, 'historical executable identity changed')
    driver.require((source['base'], source['base_tree']) ==
                   (measurement.BASE, measurement.BASE_TREE), 'wrong starting main')


def validate(before, after):
    expected = {(v, m, w, c) for v in driver.PINS
                for m in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream', 'Log', 'SurvivabilityRiskLevel']
                for w in ['closed-schema', 'open-extensions'] for c in [False, True]}
    stages, summaries = [], []
    for stage, value in [('before', before), ('after', after)]:
        authenticate(value['source'], stage)
        driver.require(value.get('completed') is True, 'incomplete measurement')
        driver.require(value['schema_includes'] == {
            v: dict(name=f'UCI_SecurityMarkings_v2_{v[-1]}_0.xsd', sha256=digest)
            for v, digest in measurement.INCLUDES.items()}, 'wrong include identity')
        rows = value['measurements']
        keys = [(r['version'], r['message'].split('}')[-1], r['world'], r['codec']) for r in rows]
        driver.require(len(rows) == 40 and set(keys) == expected, 'missing/duplicate selection')
        stages.append(dict(zip(keys, rows)))
        for key, row in zip(keys, rows):
            version, message, world, codec = key
            driver.require(row['message'] == '{' + driver.OAM + '}' + message, 'wrong message namespace')
            driver.require((row['revision'], row['root_sha256']) == driver.PINS[version], 'wrong UCI pin')
            closure = row['selected_type_qnames']
            driver.require(closure and len(closure) == len(set(closure)), 'empty/duplicate closure')
            driver.require(hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest()
                           == row['closure_sha256'], 'closure hash mismatch')
            blockers = [line.removeprefix('model boundary: ') for line in row['stdout'].splitlines()
                        if line.startswith('model boundary: ')]
            driver.require(blockers == ['{' + driver.OAM + '}' + boundary(stage, version, message)],
                           'changed historical boundary')
            for name in ['model', 'service api']:
                driver.require(f'{name} status: NOT READY\n' in row['stdout'], 'unqualified readiness')
                driver.require(f'{name} boundary: {blockers[0]}\n' in row['stdout'], 'mismatched boundary')
            if codec:
                driver.require('codec: not measured' in row['stdout'], 'untruthful codec readiness')
            driver.require(row['exit'] == row['generation']['exit'] == 1 and row['artifacts'] == {},
                           'non-atomic generation or forged exit')
            existing = row['preexisting_generation']
            driver.require(existing is not None and existing['exit'] == 1, 'missing existing-output failure')
            driver.require(existing['stdout'] == row['stdout'] and existing['stderr'] == row['stderr'],
                           'pre-existing readiness changed')
            for stream in ['stdout', 'stderr']:
                driver.require(row[stream] == row['generation'][stream], 'check/generate mismatch')
            summaries.append(dict(stage=stage, version=version, message=message, world=world,
                                  codec=codec, ready=False, first_blocker=blockers[0]))
    for key in expected:
        for field in ['selected_type_qnames', 'closure_sha256', 'contract_sha256']:
            driver.require(stages[0][key][field] == stages[1][key][field], 'selected closure/contract changed')
    return summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = validate(json.loads(args.before.read_text()), json.loads(args.after.read_text()))
    args.output.write_text(json.dumps(dict(completed=True, real_ready=0, selections=result), indent=2) + '\n')
    print('PY015 PINNED UCI DELTA: PASSED before=40 after=40 real_ready=0')


if __name__ == '__main__':
    main()