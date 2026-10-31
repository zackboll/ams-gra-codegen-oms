#!/usr/bin/env python3
"""CI-SELECT-003: mandatory, fail-closed Python-owned qualification discovery.

Only fixed entry points are workflow-owned. New Python feature tests live in the
three flat test directories, so adding one does not require editing CI YAML.
Neither the runner nor unknown helpers are granted narrow selection ownership.
"""
import json
import os
from pathlib import Path
import re
import sys
import unittest

# Discovery must not create an unregistered __pycache__ directory.
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
QUALIFICATION = Path(__file__).resolve().parent
MODES = ('fast', 'uci', 'sleet')
TEST_NAME = re.compile(r'test_[a-z0-9_]+\.py\Z')


def _flatten(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from _flatten(item)
        else:
            yield item


def discover(mode, qualification=QUALIFICATION):
    if mode not in MODES:
        raise ValueError('unregistered Python qualification mode')
    directory = qualification / mode
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f'absent or redirected Python qualification directory: {mode}')
    sources = sorted(directory.iterdir())
    if not sources:
        raise ValueError(f'no Python qualification test sources: {mode}')
    for source in sources:
        if (source.is_symlink() or not source.is_file() or
                TEST_NAME.fullmatch(source.name) is None):
            raise ValueError(f'unregistered Python qualification source: {source.name}')
    loader = unittest.TestLoader()
    suite = loader.discover(start_dir=str(directory), pattern='test_*.py',
                            top_level_dir=str(directory))
    if loader.errors:
        raise ValueError('Python qualification discovery errors: ' + repr(loader.errors))
    tests = list(_flatten(suite))
    actual = {item.id().split('.')[0] for item in tests}
    expected = {path.stem for path in sources}
    identities = [item.id() for item in tests]
    if not tests or actual != expected or len(identities) != len(set(identities)):
        raise ValueError('incomplete/duplicate Python qualification test discovery')
    return suite, sources, identities


def require_inputs(mode):
    if mode == 'uci':
        for name in ('AMS_GRA_UCI_2_5_ROOT', 'AMS_GRA_UCI_2_6_ROOT'):
            value = os.environ.get(name)
            if not value or not Path(value).is_file():
                raise ValueError('missing pinned UCI root: ' + name)
        if not (ROOT / 'target/release/ams-gra-codegen-oms').is_file():
            raise ValueError('missing qualified release generator')
    elif mode == 'sleet':
        value = os.environ.get('AMS_GRA_SLEET_BIN')
        if not value or not Path(value).is_file():
            raise ValueError('missing shared pinned-Sleet binary')


def main(argv):
    if len(argv) != 1 or argv[0] not in MODES:
        print('usage: python/qualification/run.py {fast|uci|sleet}', file=sys.stderr)
        return 2
    mode = argv[0]
    try:
        if sys.version_info < (3, 11):
            raise ValueError('Python 3.11+ required')
        require_inputs(mode)
        suite, files, identities = discover(mode)
    except ValueError as error:
        print(f'CI_SELECT_003_INVALID: {error}', file=sys.stderr)
        return 1
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful() or result.testsRun != len(identities):
        print('CI_SELECT_003_QUALIFICATION_FAILED', file=sys.stderr)
        return 1
    if mode == 'uci' and os.environ.get('RUNNER_TEMP'):
        receipt = Path(os.environ['RUNNER_TEMP']) / 'py006-uci' / 'ci-select003-python.json'
        if not receipt.parent.is_dir():
            print('CI_SELECT_003_INVALID: missing existing pinned-UCI evidence directory', file=sys.stderr)
            return 1
        receipt.write_text(json.dumps({'schema_version': 1, 'mode': mode,
                                      'tests_run': result.testsRun,
                                      'sources': [f.name for f in files],
                                      'test_ids': identities}, indent=2) + '\n')
    print(f'CI_SELECT_003_{mode.upper()}_COMPLETE tests={result.testsRun} files={len(files)}',
          flush=True)
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
