#!/usr/bin/env python3
"""Separate PY-018 gate: retain all 40 historical successes and artifact hashes.

Historical PY-017 evidence and its original before/after oracle are immutable.
Live stages must agree with its successful snapshot AND with each other.
"""
import argparse
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py018_compat_prior', ROOT / 'scripts/check-py017-uci.py')
prior = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prior)


def validate(before, after):
    frozen_before, frozen_after = prior.frozen()
    prior.validate(frozen_before, frozen_after)
    expected = {(r['version'], r['message'], r['world'], r['codec']): r for r in frozen_after['measurements']}
    stages = []
    for value in [before, after]:
        prior.driver.require(value.get('completed') is True, 'incomplete live compatibility campaign')
        rows = value['measurements']
        actual = {(r['version'], r['message'], r['world'], r['codec']): r for r in rows}
        prior.driver.require(len(rows) == 40 and set(actual) == set(expected), 'wrong live selections')
        prior.driver.require(value['schema_includes'] == frozen_after['schema_includes'], 'wrong live include pins')
        for key, oracle in expected.items():
            row = actual[key]
            for field in ['revision', 'root_sha256', 'contract_sha256', 'selected_type_qnames',
                          'closure_sha256', 'exit', 'stdout', 'stderr', 'artifacts']:
                prior.driver.require(row[field] == oracle[field], 'historical success changed: ' + field)
            prior.driver.require(row['exit'] == row['generation']['exit'] == 0, 'READY regression')
            prior.driver.require(row['generation']['stdout'] == oracle['generation']['stdout']
                                 and row['generation']['stderr'] == oracle['generation']['stderr'],
                                 'generation diagnostic changed')
        stages.append(actual)
    for key in expected:
        prior.driver.require(stages[0][key]['artifacts'] == stages[1][key]['artifacts'], 'before/after hash drift')
    return dict(before_ready=40, after_ready=40, selections=40,
                artifacts=sum(len(r['artifacts']) for r in expected.values()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', required=True, type=Path)
    parser.add_argument('--after', required=True, type=Path)
    args = parser.parse_args()
    result = validate(json.loads(args.before.read_text()), json.loads(args.after.read_text()))
    print('PY018 HISTORICAL COMPATIBILITY PASSED ' + json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()