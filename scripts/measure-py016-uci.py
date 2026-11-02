#!/usr/bin/env python3
"""Measure 40 actual pinned CLI selections; no predetermined readiness verdicts."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASE = '08ac2907dc3b2553085a638c8e1795c121ee1535'
BASE_TREE = '38f3b5d8ebd35f3d288b32bc7599583f6ec22626'
INCLUDES = {'2.5': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
            '2.6': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
spec = importlib.util.spec_from_file_location('py016_driver', ROOT / 'scripts/check-py006-uci.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', required=True, type=Path)
    parser.add_argument('--closure-tool', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--source-commit', required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    git = lambda *a: subprocess.check_output(['git', '-C', str(ROOT), *a])
    source_commit = git('rev-parse', args.source_commit).decode().strip()
    source_tree = git('rev-parse', args.source_commit + '^{tree}').decode().strip()
    production = git('ls-tree', '-r', '--name-only', source_commit,
                     'crates/backend-python/src').decode().splitlines()
    source = dict(commit=source_commit, tree=source_tree, base=BASE, base_tree=BASE_TREE,
                  generator_sha256=driver.digest(args.generator),
                  closure_tool_sha256=driver.digest(args.closure_tool),
                  production_sha256={p: hashlib.sha256(git('show', source_commit + ':' + p)).hexdigest()
                                     for p in production})
    rows = []
    includes = {}
    for version, (revision, digest) in driver.PINS.items():
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()
        driver.require(driver.digest(schema) == digest, 'pinned root hash mismatch')
        include = schema.with_name(f'UCI_SecurityMarkings_v2_{version[-1]}_0.xsd')
        driver.require(driver.digest(include) == INCLUDES[version], 'pinned include hash mismatch')
        includes[version] = dict(name=include.name, sha256=driver.digest(include))
        for message in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream', 'Log', 'SurvivabilityRiskLevel']:
            contract = args.output / f'{version}-{message}.yaml'
            contract.write_text(driver.contract(message, version))
            closure = json.loads(subprocess.check_output([str(args.closure_tool), str(schema), str(contract)]))
            driver.require(closure and len(closure) == len(set(closure)), 'invalid closure')
            closure_hash = hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest()
            for world in ['closed-schema', 'open-extensions']:
                for codec in [False, True]:
                    result = driver.check(args.generator, schema, contract, world, codec)
                    destination = args.output / f'{version}-{message}-{world}-{codec}'
                    generated = driver.check(args.generator, schema, contract, world, codec, destination)
                    artifacts = {str(p.relative_to(destination)): driver.digest(p)
                                 for p in sorted(destination.rglob('*')) if p.is_file()}
                    if result['exit']:
                        driver.require(generated['exit'] == result['exit'] and not destination.exists(),
                                       'unsupported selection did not reject atomically')
                        destination.mkdir()
                        sentinel = destination / 'sentinel.txt'
                        sentinel.write_bytes(b'pre-existing PY016 output\n')
                        existing = driver.check(args.generator, schema, contract, world, codec, destination)
                        driver.require(existing['exit'] == result['exit']
                                       and sentinel.read_bytes() == b'pre-existing PY016 output\n'
                                       and list(destination.iterdir()) == [sentinel],
                                       'unsupported selection modified pre-existing output')
                    else:
                        existing = None
                    rows.append(dict(version=version, revision=revision, root_sha256=digest,
                                     message='{' + driver.OAM + '}' + message,
                                     contract_sha256=driver.digest(contract), world=world, codec=codec,
                                     selected_type_qnames=closure, closure_sha256=closure_hash,
                                      generation=generated, preexisting_generation=existing,
                                      artifacts=artifacts, **result))
                    (args.output / 'partial.json').write_text(json.dumps(rows, indent=2) + '\n')
                    print(version, message, world, codec, result['exit'],
                          [line for line in result['stdout'].splitlines() if 'boundary:' in line], flush=True)
    (args.output / 'measurements.json').write_text(json.dumps(dict(
        completed=True, source=source, schema_includes=includes, measurements=rows), indent=2) + '\n')
    print('PY016 MEASUREMENTS COMPLETE: selections=40; not an interoperability claim')


if __name__ == '__main__':
    main()