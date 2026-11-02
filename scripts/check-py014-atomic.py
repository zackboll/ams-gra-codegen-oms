#!/usr/bin/env python3
"""Actual pre-existing-output rejection for every pinned PY-014 selection."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py014_atomic_driver', ROOT / 'scripts/check-py006-uci.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--measurements', type=Path, required=True)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    measured = json.loads(args.measurements.read_text())
    driver.require(measured['completed'] and len(measured['measurements']) == 40, 'incomplete measurements')
    driver.require(driver.digest(args.generator) == measured['source']['generator_sha256'], 'wrong executable')
    args.output.mkdir(parents=True, exist_ok=False)

    def execute(row):
        version = row['version']
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()
        driver.require(driver.digest(schema) == row['root_sha256'], 'wrong pinned schema')
        message = row['message'].split('}')[-1]
        label = f'{version}-{message}-{row["world"]}-{row["codec"]}'
        contract = args.output / (label + '.yaml')
        contract.write_text(driver.contract(message, version))
        driver.require(driver.digest(contract) == row['contract_sha256'], 'changed contract')
        destination = args.output / label
        destination.mkdir()
        sentinel = destination / 'sentinel.txt'
        sentinel.write_bytes(b'PY014 existing output must remain unchanged\n')
        before = {sentinel.name: driver.digest(sentinel)}
        result = driver.check(args.generator, schema, contract, row['world'], row['codec'], destination)
        after = {p.relative_to(destination).as_posix(): driver.digest(p)
                 for p in destination.rglob('*') if p.is_file()}
        driver.require(result['exit'] == row['exit'] == 1 and before == after, 'non-atomic failure')
        driver.require(result['stdout'] == row['stdout'] and result['stderr'] == row['stderr'], 'readiness changed')
        return dict(version=version, message=row['message'], world=row['world'], codec=row['codec'],
                    closure_sha256=row['closure_sha256'], contract_sha256=row['contract_sha256'],
                    before=before, after=after, result=result)

    with ThreadPoolExecutor(max_workers=4) as workers:
        rows = list(workers.map(execute, measured['measurements']))
    (args.output / 'atomic.json').write_text(json.dumps(dict(completed=True,
        source=measured['source'], measurements_sha256=hashlib.sha256(args.measurements.read_bytes()).hexdigest(),
        selections=rows), indent=2) + '\n')
    print('PY014 PINNED PRE-EXISTING OUTPUT ATOMICITY: PASSED selections=40')


if __name__ == '__main__':
    main()