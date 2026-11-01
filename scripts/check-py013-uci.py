#!/usr/bin/env python3
"""Authenticate the measured PY-013 delta without depending on current sources."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py013_measurement', ROOT / 'scripts/measure-py013-uci.py')
measurement = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measurement)
driver = measurement.driver
# Independently pinned canonical inventories, not manifest-provided expectations.
# Like PY-012-R1, these identify historical bytes, never current production files.
SOURCES = {
    'before': ('a28cb1d7add3e1c171a31f11462e676330e75b5d',
               '0c79b9d279da230431bfaad93b211bd75e0a7c22',
               '5c0492be958f4b8fd6e371c16cbd8621df105418b2eefae2556c58db4ffad400'),
    'after': ('26c167c20a8ebb3fc8233b3f165deb1a14cb82b2',
              'dde0502f15f20d8563995c4b2bff6f3d6b3f69e9',
              'eb9d180fe90c8fd5207d66b517af15299d56e6b6eb06cbe5f1c1475387d59d9e'),
}
GENERATORS = {
    'before': 'c688216aa5b269a9510004664970d9cc8d8fbb1ac6519cac719c14326cc56098',
    'after': 'fceabd6a86c9baa5c8fc59d8d186a572fea777f36523fd27dcf6503d9f048b53',
}
CLOSURE_TOOL = '1cb305284e8d97471c99f8c65df5d6029d0b00b01fb058a526eeae43aa49cf9a'


def authenticate(source, stage):
    commit, tree, inventory = SOURCES[stage]
    driver.require((source['commit'], source['tree']) == (commit, tree), 'historical source identity changed')
    canonical = json.dumps(source['production_sha256'], sort_keys=True, separators=(',', ':')).encode()
    driver.require(hashlib.sha256(canonical).hexdigest() == inventory, 'historical production inventory changed')
    driver.require(source['generator_sha256'] == GENERATORS[stage]
                   and source['closure_tool_sha256'] == CLOSURE_TOOL, 'historical executable identity changed')
    driver.require((source['base'], source['base_tree']) ==
                   (measurement.BASE, measurement.BASE_TREE), 'wrong starting main')


def boundary(stage, version, message):
    if message == 'PositionReport' or (message == 'SubsystemStream' and stage == 'after'):
        return 'FGI_SourceOpenChoiceType: unsupported Python IR construct: Choice / List declarations'
    if message == 'RDMA_InitializeSetup':
        return 'IPv4_AddressType: unsupported Python IR construct: unsupported constraints'
    if message == 'Log':
        return ('LogMDT: unsupported Python IR construct: field ServiceUpTime: '
                'unsupported Python IR construct: primitive Duration' if version == '2.5' else
                'DurationType: unsupported Python IR construct: primitive Duration')
    return ('SubsystemStreamMDT: unsupported Python IR construct: field SubsystemStreamBinary: '
            'unsupported Python IR construct: primitive Binary' if version == '2.5' else
            'HexBinaryType: unsupported Python IR construct: primitive Binary')


def validate(before, after):
    expected = {(v, m, w, c) for v in driver.PINS
                for m in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream', 'Log']
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
        driver.require(len(rows) == 32 and set(keys) == expected, 'missing/duplicate selection')
        stages.append(dict(zip(keys, rows)))
        for key, row in zip(keys, rows):
            version, message, world, codec = key
            driver.require(row['message'] == '{' + driver.OAM + '}' + message, 'wrong message namespace')
            driver.require((row['revision'], row['root_sha256']) == driver.PINS[version], 'wrong UCI pin')
            closure = row['selected_type_qnames']
            driver.require(closure and len(closure) == len(set(closure)), 'empty/duplicate closure')
            driver.require(hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest()
                           == row['closure_sha256'], 'closure hash mismatch')
            blocker = boundary(stage, version, message)
            for name in ['model', 'service api']:
                driver.require(f'{name} status: NOT READY\n' in row['stdout'], 'unqualified readiness')
                driver.require(f'{name} boundary: {{{driver.OAM}}}{blocker}\n' in row['stdout'], 'unexpected blocker')
            if codec:
                driver.require('codec: not measured' in row['stdout'], 'untruthful codec readiness')
            driver.require(row['exit'] == row['generation']['exit'] == 1 and row['artifacts'] == {},
                           'non-atomic generation or forged exit')
            for stream in ['stdout', 'stderr']:
                driver.require(row[stream] == row['generation'][stream], 'check/generate mismatch')
            summaries.append(dict(stage=stage, version=version, message=message, world=world,
                                  codec=codec, ready=False, first_blocker=blocker))
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
    print('PY013 PINNED UCI DELTA: PASSED before=32 after=32 real_ready=0')


if __name__ == '__main__':
    main()