#!/usr/bin/env python3
"""Capture the 24 actual pinned current-service selections, without verdict guesses.

This measurement command is not a qualification gate. Review measured blockers
before freezing after.json, then run check-py008-uci.py against that evidence.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py006_measurement', ROOT / 'scripts/check-py006-uci.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    rows = []
    for version, (revision, digest) in driver.PINS.items():
        root = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()
        driver.require(driver.digest(root) == digest, 'pinned root hash mismatch')
        for message in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream']:
            contract = args.output / (version + '-' + message + '.yaml')
            contract.write_text(driver.contract(message, version))
            for world in ['closed-schema', 'open-extensions']:
                for codec in [False, True]:
                    result = driver.check(args.generator.resolve(), root, contract, world, codec)
                    rows.append(dict(version=version, revision=revision, root_sha256=digest,
                                     message='{' + driver.OAM + '}' + message,
                                     contract_sha256=driver.digest(contract), world=world,
                                     codec=codec, **result))
                    (args.output / 'partial.json').write_text(json.dumps(rows, indent=2) + '\n')
                    print(version, message, world, codec, result['exit'],
                          [line for line in result['stdout'].splitlines() if 'boundary:' in line], flush=True)
    source = dict(dependency='d76a36d2cb9d18bcc8b825b5134a65686f361deb',
                  dependency_tree='bb28f2e70e0c2b1c75ed19ff390f8c1df40773c3',
                  generator_sha256=driver.digest(args.generator),
                  production_sha256={str(p.relative_to(ROOT)): driver.digest(p)
                                     for p in sorted((ROOT / 'crates/backend-python/src').glob('*')) if p.is_file()})
    text = json.dumps(dict(completed=True, source=source, measurements=rows), indent=2) + '\n'
    replacements = [(str(args.output.resolve()), '<EVIDENCE>'),
                    (str(args.generator.resolve()), '<GENERATOR>'), (str(ROOT), '<REPO>')]
    for version in driver.PINS:
        replacements.append((str(Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()),
                             '<UCI' + version.replace('.', '') + '_ROOT>'))
    for old, new in replacements:
        text = text.replace(old, new)
    (args.output / 'measurements.json').write_text(text)
    print('PY008 MEASUREMENTS COMPLETE: selections=24 (not an interoperability claim)')


if __name__ == '__main__':
    main()