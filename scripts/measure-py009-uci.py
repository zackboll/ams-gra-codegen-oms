#!/usr/bin/env python3
"""Measure actual CLI readiness AND atomic generation for all 24 pinned selections.

Measurement is not qualification. Review the results before freezing a new oracle.
Supply a release-built python_datetime_closure example from the same resolver.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py006_driver', ROOT / 'scripts/check-py006-uci.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def measure(generator, closure_tool, output, source_root=ROOT):
    output.mkdir(parents=True, exist_ok=False)
    rows = []
    for version, (revision, digest) in driver.PINS.items():
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()
        driver.require(driver.digest(schema) == digest, 'pinned root hash mismatch')
        for message in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream']:
            contract = output / (version + '-' + message + '.yaml')
            contract.write_text(driver.contract(message, version))
            closure_process = subprocess.run([str(closure_tool), str(schema), str(contract)],
                                             capture_output=True, text=True, check=True)
            closure = json.loads(closure_process.stdout)
            driver.require(closure and len(closure) == len(set(closure)), 'invalid closure')
            closure_hash = hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest()
            for world in ['closed-schema', 'open-extensions']:
                for codec in [False, True]:
                    result = driver.check(generator, schema, contract, world, codec)
                    destination = output / f'{version}-{message}-{world}-{codec}'
                    generated = driver.check(generator, schema, contract, world, codec, destination)
                    artifacts = {str(p.relative_to(destination)): driver.digest(p)
                                 for p in sorted(destination.rglob('*')) if p.is_file()}
                    if result['exit']:
                        driver.require(generated['exit'] == result['exit'] and not destination.exists(),
                                       'unsupported schema did not reject atomically')
                    rows.append(dict(version=version, revision=revision, root_sha256=digest,
                                     message='{' + driver.OAM + '}' + message,
                                     contract_sha256=driver.digest(contract), world=world, codec=codec,
                                     selected_type_qnames=closure, closure_sha256=closure_hash,
                                     generation=generated, artifacts=artifacts, **result))
                    (output / 'partial.json').write_text(json.dumps(rows, indent=2) + '\n')
                    print(version, message, world, codec, result['exit'],
                          [line for line in result['stdout'].splitlines() if 'boundary:' in line], flush=True)
    source = dict(base='e186c9dd7c700eb56e142a22309256f3de9a0141',
                  base_tree='7074b9ded51f0aa23458c4d52c26b581dfafc143',
                  generator_sha256=driver.digest(generator), closure_tool_sha256=driver.digest(closure_tool),
                  production_sha256={str(p.relative_to(source_root)): driver.digest(p)
                                     for p in sorted((source_root / 'crates/backend-python/src').glob('*')) if p.is_file()})
    value = dict(completed=True, source=source, measurements=rows)
    text = json.dumps(value, indent=2) + '\n'
    for old, new in [(str(output.resolve()), '<EVIDENCE>'), (str(generator), '<GENERATOR>'),
                     (str(ROOT), '<REPO>')]:
        text = text.replace(old, new)
    for version in driver.PINS:
        text = text.replace(str(Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()),
                            '<UCI' + version.replace('.', '') + '_ROOT>')
    (output / 'measurements.json').write_text(text)
    return json.loads(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--closure-tool', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT,
                        help='Source checkout used to build this executable (baseline worktree for BEFORE)')
    args = parser.parse_args()
    measure(args.generator.resolve(), args.closure_tool.resolve(), args.output.resolve(), args.source_root.resolve())
    print('PY009 MEASUREMENTS COMPLETE: selections=24 (not an interoperability claim)')


if __name__ == '__main__':
    main()
