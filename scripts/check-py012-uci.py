#!/usr/bin/env python3
"""Verify PY-012's measured pinned-service delta; never infer interoperability."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAIN = 'ab438eab77d0b202a033bec2b2fa99508bbebc8f'
MAIN_TREE = '02c0df930e7af0c8e487994ac25e5a69d1104a33'
spec = importlib.util.spec_from_file_location('py012_driver', ROOT / 'scripts/check-py006-uci.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def validate(before, after):
    expected = {(v, m, w, c) for v in driver.PINS
                for m in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream']
                for w in ['closed-schema', 'open-extensions'] for c in [False, True]}
    driver.require(before['source']['commit'] == MAIN and before['source']['tree'] == MAIN_TREE
                   and before['source']['dirty'] is False, 'wrong baseline source')
    stages, summaries = [], []
    for stage, value in [('before', before), ('after', after)]:
        driver.require(value.get('completed') is True, 'incomplete measurements')
        rows = value['measurements']
        keys = [(r['version'], r['message'].split('}')[-1], r['world'], r['codec']) for r in rows]
        driver.require(len(rows) == 24 and set(keys) == expected, 'missing/duplicate selection')
        stages.append(dict(zip(keys, rows)))
        for row, key in zip(rows, keys):
            version, message, world, codec = key
            driver.require((row['revision'], row['root_sha256']) == driver.PINS[version], 'wrong pin')
            closure = row['selected_type_qnames']
            driver.require(bool(closure) and len(closure) == len(set(closure)), 'empty/duplicate closure')
            driver.require(hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest()
                           == row['closure_sha256'], 'closure hash mismatch')
            if message == 'PositionReport':
                blocker = 'FGI_SourceOpenChoiceType: unsupported Python IR construct: Choice / List declarations'
            elif message == 'RDMA_InitializeSetup':
                blocker = 'IPv4_AddressType: unsupported Python IR construct: unsupported constraints'
            elif version == '2.6':
                blocker = 'HexBinaryType: unsupported Python IR construct: primitive Binary'
            elif stage == 'before':
                blocker = 'SubsystemStreamMDT: unsupported Python IR construct: unbounded field SubsystemStreamString'
            else:
                blocker = ('SubsystemStreamMDT: unsupported Python IR construct: field SubsystemStreamBinary: '
                           'unsupported Python IR construct: primitive Binary')
            for boundary in ['model', 'service api']:
                driver.require(f'{boundary} status: NOT READY\n' in row['stdout'], 'unqualified readiness')
                driver.require(f'{boundary} boundary: {{{driver.OAM}}}{blocker}\n' in row['stdout'],
                               'unexpected first blocker')
            if codec:
                driver.require('codec: not measured' in row['stdout'], 'untruthful codec verdict')
            driver.require(row['exit'] == row['generation']['exit'] == 1 and row['artifacts'] == {},
                           'generation did not fail atomically')
            for field in ['stdout', 'stderr']:
                driver.require(row[field] == row['generation'][field], 'check/generate diagnostic mismatch')
            summaries.append(dict(stage=stage, version=version, message=message, world=world,
                                  codec=codec, ready=False, first_blocker=blocker))
    for key in expected:
        for field in ['selected_type_qnames', 'closure_sha256', 'contract_sha256']:
            driver.require(stages[0][key][field] == stages[1][key][field], 'selection changed')
    return summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = validate(json.loads(args.before.read_text()), json.loads(args.after.read_text()))
    args.output.write_text(json.dumps(dict(completed=True, real_ready=0, verified_main=MAIN,
                                         verified_main_tree=MAIN_TREE, selections=result), indent=2) + '\n')
    print('PY012 PINNED UCI DELTA: PASSED before=24 after=24 real_ready=0')


if __name__ == '__main__':
    main()