#!/usr/bin/env python3
"""Validate the versioned PY-011 readiness delta without changing old oracles."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py011_measurement', ROOT / 'scripts/measure-py011-uci.py')
measurement = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measurement)
driver = measurement.driver


def validate(before, after):
    expected = {(version, message, world, codec)
                for version in driver.PINS
                for message in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream']
                for world in ['closed-schema', 'open-extensions'] for codec in [False, True]}
    summaries = []
    for stage, value in [('before', before), ('after', after)]:
        driver.require(value.get('completed') is True, 'incomplete measurements')
        rows = value['measurements']
        keys = [(r['version'], r['message'].split('}')[-1], r['world'], r['codec']) for r in rows]
        driver.require(len(rows) == 24 and set(keys) == expected, 'missing/duplicate selection')
        for row, key in zip(rows, keys):
            version, message, _, codec = key
            revision, digest = driver.PINS[version]
            driver.require((row['revision'], row['root_sha256']) == (revision, digest), 'wrong pinned identity')
            closure = row['selected_type_qnames']
            driver.require(bool(closure) and len(closure) == len(set(closure)), 'empty/duplicate closure')
            driver.require(hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest()
                           == row['closure_sha256'], 'closure hash mismatch')
            if message == 'PositionReport':
                blocker = ('NATO_SpecialWordsType: unsupported Python IR construct: unsupported constraints'
                           if stage == 'before' else
                           'FGI_SourceOpenChoiceType: unsupported Python IR construct: Choice / List declarations')
            elif message == 'RDMA_InitializeSetup':
                blocker = 'IPv4_AddressType: unsupported Python IR construct: unsupported constraints'
            elif version == '2.5':
                blocker = 'SubsystemStreamMDT: unsupported Python IR construct: unbounded field SubsystemStreamString'
            else:
                blocker = 'HexBinaryType: unsupported Python IR construct: primitive Binary'
            for boundary in ['model', 'service api']:
                driver.require(f'{boundary} status: NOT READY\n' in row['stdout'], 'unqualified service readiness')
                driver.require(f'{boundary} boundary: {{{driver.OAM}}}{blocker}\n' in row['stdout'],
                               'unexpected first blocker')
            if codec:
                driver.require('codec: not measured' in row['stdout'], 'untruthful codec verdict')
            driver.require(row['exit'] == row['generation']['exit'] == 1 and row['artifacts'] == {},
                           'generation did not fail atomically')
            for field in ['stdout', 'stderr']:
                driver.require(row[field] == row['generation'][field], 'check/generate diagnostic mismatch')
            summaries.append(dict(stage=stage, version=version, message=message, world=key[2],
                                  codec=codec, ready=False, first_blocker=blocker))
    old = {(r['version'], r['message'], r['world'], r['codec']): r for r in before['measurements']}
    for row in after['measurements']:
        previous = old[(row['version'], row['message'], row['world'], row['codec'])]
        for field in ['selected_type_qnames', 'closure_sha256', 'contract_sha256']:
            driver.require(row[field] == previous[field], 'selection changed across implementation')
    return summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = validate(json.loads(args.before.read_text()), json.loads(args.after.read_text()))
    args.output.write_text(json.dumps(dict(completed=True, real_ready=0, selections=result), indent=2) + '\n')
    print('PY011 PINNED UCI DELTA: PASSED before=24 after=24 real_ready=0')


if __name__ == '__main__':
    main()