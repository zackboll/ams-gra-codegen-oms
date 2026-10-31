#!/usr/bin/env python3
"""Source-bound receipts around the unchanged CI-SELECT-003 execution contract."""
import importlib.util
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True
from owned_qualification import ROOT, digest, python_fixtures, python_ids
from task101_evidence import PINS, INCLUDES, REV, TREE


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ('fast', 'uci', 'sleet'):
        raise ValueError('expected fixed Python qualification mode')
    mode = sys.argv[1]
    version = '.'.join(map(str, sys.version_info[:2]))
    policy = 'optimized' if sys.flags.optimize == 1 else 'normal'
    output = Path(os.environ['RUNNER_TEMP']) / ('ci004-python-' + mode) / (version + '-' + policy)
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / 'result.json'
    receipt.unlink(missing_ok=True)
    if version not in ('3.11', '3.13'):
        raise ValueError('real Python 3.11 or 3.13 required')
    if sys.flags.optimize not in (0, 1):
        raise ValueError('unsupported optimization policy')
    spec = importlib.util.spec_from_file_location('owned_python_runner', ROOT / 'python/qualification/run.py')
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    runner.require_inputs(mode)
    source = subprocess.check_output(['git', 'rev-parse', 'HEAD', 'HEAD^{tree}'], cwd=ROOT, text=True).splitlines()
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT):
        raise ValueError('dirty Python qualification source')
    if source[0] != os.environ.get('GITHUB_SHA', source[0]):
        raise ValueError('Python workflow source mismatch')
    generator = ROOT / 'target' / ('release' if mode == 'uci' else 'debug') / 'ams-gra-codegen-oms'
    identity = dict(sha=source[0], tree=source[1], run_id=os.environ.get('GITHUB_RUN_ID', 'local'),
                    run_attempt=os.environ.get('GITHUB_RUN_ATTEMPT', '1'), mode=mode,
                    python=version, python_version=sys.version, optimization=policy,
                    interpreter_sha256=digest(Path(sys.executable).read_bytes()),
                    generator_sha256=digest(generator.read_bytes()))
    if mode in ('uci', 'sleet'):
        releases = ('25', '26') if mode == 'uci' else ('25',)
        for release in releases:
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release[-1] + '_ROOT'])
            if (digest(schema.read_bytes()) != PINS[release] or
                    digest((schema.parent / f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd').read_bytes()) != INCLUDES[release]):
                raise ValueError('wrong Python pinned UCI source/include')
        identity['uci_roots'] = {r: PINS[r] for r in releases}
        identity['uci_includes'] = {r: INCLUDES[r] for r in releases}
    if mode == 'sleet':
        binary = Path(os.environ['AMS_GRA_SLEET_BIN']).resolve()
        checkout = binary.parents[2]
        if (subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD', 'HEAD^{tree}'],
                                    text=True).splitlines() != [REV, TREE] or
                subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain'])):
            raise ValueError('wrong Python pinned Sleet checkout')
        identity.update(sleet_revision=REV, sleet_tree=TREE, sleet_binary_sha256=digest(binary.read_bytes()))
    fixtures = [r for r in python_fixtures() if r['mode'] == mode]
    suite, sources, identities = runner.discover(mode)
    expected = sorted(t for path in sources for t in python_ids(path))
    if sorted(identities) != expected:
        raise ValueError('filtered Python source inventory')
    hashes = {p.relative_to(ROOT).as_posix(): digest(p.read_bytes()) for p in sources}
    for item in fixtures:
        hashes.update(item['hashes'])
    log = io.StringIO()
    with contextlib.redirect_stdout(log), contextlib.redirect_stderr(log):
        result = unittest.TextTestRunner(stream=log, verbosity=2, resultclass=runner.QualificationResult).run(suite)
    (output / 'execution.log').write_text(log.getvalue())
    print(log.getvalue(), end='')
    execution = runner.execution_status(result, expected)
    if execution['status'] != 'success':
        raise ValueError('incomplete Python execution: ' + json.dumps(execution))
    if any(digest((ROOT / p).read_bytes()) != h for p, h in hashes.items()):
        raise ValueError('Python inputs changed during execution')
    runner.write_receipt(receipt, dict(version=1, identity=identity, hashes=hashes,
        fixtures=fixtures, test_ids=expected, execution=execution,
        log_sha256=digest((output / 'execution.log').read_bytes())))
    print('CI004_PYTHON_COMPLETE', mode, version, policy, len(expected), flush=True)


if __name__ == '__main__':
    main()