#!/usr/bin/env python3
"""PY-018 wrapper retaining PY-017 historical measurement bytes and projection IDs."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py018_measure_prior', ROOT / 'scripts/measure-py017-uci.py')
prior = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prior)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', required=True, type=Path)
    parser.add_argument('--projection-tool', required=True, type=Path)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    generator = args.generator.resolve()
    subprocess.run([sys.executable, str(ROOT / 'scripts/measure-py017-uci.py'),
        '--generator', str(generator), '--closure-tool', str(generator.parent / 'examples/python_datetime_closure'),
        '--source-commit', args.source_commit, '--output', str(args.output)], check=True)
    records = []
    for version in prior.driver.PINS:
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT'])
        for message in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream', 'Log',
                        'SurvivabilityRiskLevel', 'AO_CapabilityStatus']:
            contract = args.output / (version + '-' + message + '.yaml')
            contract.write_text(prior.driver.contract(message, version))
            identities = json.loads(subprocess.check_output([str(args.projection_tool.resolve()), str(schema), str(contract)]))
            records.append(dict(version=version, message=message, identities=identities,
                identities_sha256=hashlib.sha256(json.dumps(identities, sort_keys=True, separators=(',', ':')).encode()).hexdigest()))
            (args.output / 'projection-identities.json').write_text(json.dumps(records, indent=2) + '\n')
    print('PY018 ACTUAL SELECTED/SUPPORT PROJECTIONS COMPLETE releases=2 selections=6 worlds=2')


if __name__ == '__main__':
    main()