#!/usr/bin/env python3
"""Compare job-local cache output to real full-schema Log generation, never fixtures."""
import argparse
import importlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from unittest.mock import patch

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'python/oms-lacal-runtime/tests'))
from full_uci_generation import digest, generate, inventory

spec = importlib.util.spec_from_file_location('py016_measure', ROOT / 'scripts/measure-py016-uci.py')
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--generator', type=Path, default=ROOT / 'target/debug/ams-gra-codegen-oms')
    args = parser.parse_args()
    output = args.output.resolve()
    require(not output.is_relative_to(ROOT), 'evidence must be outside source')
    output.mkdir(parents=True, exist_ok=False)
    cache = output / 'cache'
    cache.mkdir()
    generator = args.generator.resolve()
    source = subprocess.check_output(['git', 'rev-parse', 'HEAD', 'HEAD^{tree}'],
                                     cwd=ROOT, text=True).splitlines()
    require(not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT), 'dirty source')
    rows = []
    for release in ('2.5', '2.6'):
        schema = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
        include = schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')
        require(digest(schema) == measure.driver.PINS[release][1], 'wrong pinned root')
        require(digest(include) == measure.INCLUDES[release], 'wrong pinned include')
        for world in ('closed-schema', 'open-extensions'):
            selection = output / (release + '-' + world)
            selection.mkdir()
            contract = selection / 'service.yaml'
            contract.write_text(measure.driver.contract('Log', release))
            package = 'full_log_' + release.replace('.', '') + '_' + world.replace('-', '_')
            results = {}
            generation_calls = {}
            for mode in ('original', 'miss', 'hit'):
                destination = selection / mode / package
                destination.parent.mkdir()
                with patch.dict(os.environ, AMS_GRA_PY016_GENERATION_CACHE=(
                        '' if mode == 'original' else str(cache))):
                    with patch('subprocess.run', wraps=subprocess.run) as calls:
                        results[mode] = generate(generator, schema, contract, release, world, destination)
                    generation_calls[mode] = sum(call.args[0][0] == str(generator)
                                                 for call in calls.call_args_list)
            original = inventory(selection / 'original' / package)
            require(set(original) == {'model.py', 'service_api.py', 'service_codec.py'},
                    'unexpected actual production package layout')
            require(generation_calls == dict(original=1, miss=1, hit=0), 'wrong real generation count')
            require([results[m]['cache'] for m in ('original', 'miss', 'hit')] ==
                    ['disabled', 'miss', 'hit'], 'wrong cache state transitions')
            for mode in ('miss', 'hit'):
                require(inventory(selection / mode / package) == original,
                        'cached sources differ from original production generation')
                require(results[mode]['files'] == original, 'manifest inventory differs from production')
            phase = time.monotonic()
            sys.path.insert(0, str(selection / 'hit'))
            try:
                namespace = importlib.import_module(package)
                require(namespace.__file__ is None, 'cached package is not a namespace package')
                modules = [importlib.import_module(package + '.' + name)
                           for name in ('model', 'service_api', 'service_codec')]
                require(all(Path(module.__file__).parent == selection / 'hit' / package
                            for module in modules), 'import used another generated selection')
            finally:
                sys.path.pop(0)
                for name in list(sys.modules):
                    if name == package or name.startswith(package + '.'):
                        del sys.modules[name]
            row = dict(release=release, world=world, files=original, generation_calls=generation_calls,
                       imports_seconds=time.monotonic() - phase, results=results)
            rows.append(row)
            (output / 'partial.json').write_text(json.dumps(rows, indent=2) + '\n')
            print('PY016_REAL_CACHE_PARITY', release, world, json.dumps(generation_calls), flush=True)
    require(subprocess.check_output(['git', 'rev-parse', 'HEAD', 'HEAD^{tree}'],
                                    cwd=ROOT, text=True).splitlines() == source, 'source changed')
    require(not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT), 'source dirtied')
    (output / 'result.json').write_text(json.dumps(dict(source=source,
        generator_sha256=digest(generator), selections=rows, completed=True), indent=2) + '\n')
    print('PY016_REAL_CACHE_PARITY_COMPLETE selections=4 real_generations=8 cache_hits=4', flush=True)


if __name__ == '__main__':
    main()