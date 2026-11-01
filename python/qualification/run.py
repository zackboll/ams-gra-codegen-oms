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
import tempfile
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
    fixture_root = directory / 'fixtures'
    if fixture_root.exists() or fixture_root.is_symlink():
        sys.path.insert(0, str(ROOT / 'scripts'))
        from owned_qualification import python_fixtures
        python_fixtures(ROOT)
        sources = [source for source in sources if source != fixture_root]
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
    if fixture_root.exists():
        from owned_qualification import python_ids
        declared = sorted(identity for source in sources for identity in python_ids(source))
        if sorted(identities) != declared:
            raise ValueError('Python discovery filtered actual source test inventory')
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


class QualificationResult(unittest.TextTestResult):
    """Account for actual starts and successes, not just suite registration."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.executed_ids = []
        self.passed_ids = []

    def startTest(self, test):
        self.executed_ids.append(test.id())
        super().startTest(test)

    def addSuccess(self, test):
        self.passed_ids.append(test.id())
        super().addSuccess(test)


def execution_status(result, identities):
    counts = dict(tests_registered=len(identities), tests_run=result.testsRun,
                  tests_passed=len(result.passed_ids), failures=len(result.failures),
                  errors=len(result.errors), skipped=len(result.skipped),
                  expected_failures=len(result.expectedFailures),
                  unexpected_successes=len(result.unexpectedSuccesses))
    complete = (result.wasSuccessful()
                and result.testsRun == len(identities)
                and not result.failures and not result.errors
                and not result.skipped and not result.expectedFailures
                and not result.unexpectedSuccesses
                and sorted(result.executed_ids) == sorted(identities)
                and sorted(result.passed_ids) == sorted(identities))
    reasons = [name for name in ('failures', 'errors', 'skipped', 'expected_failures',
                                'unexpected_successes') if counts[name]]
    if not result.wasSuccessful():
        reasons.append('unittest unsuccessful')
    if result.testsRun != len(identities):
        reasons.append('executed count differs from registration')
    if sorted(result.executed_ids) != sorted(identities):
        reasons.append('missing, duplicate, or unregistered executed test identities')
    if sorted(result.passed_ids) != sorted(identities):
        reasons.append('not every registered identity passed exactly once')
    return dict(status='success' if complete else 'failed', reasons=reasons, **counts,
                executed_test_ids=result.executed_ids, passed_test_ids=result.passed_ids,
                skipped_test_ids=[test.id() for test, _ in result.skipped],
                expected_failure_test_ids=[test.id() for test, _ in result.expectedFailures],
                unexpected_success_test_ids=[test.id() for test in result.unexpectedSuccesses],
                failed_test_ids=[test.id() for test, _ in result.failures],
                error_test_ids=[test.id() for test, _ in result.errors])


def prepare_receipt(mode):
    if mode != 'uci':
        return None
    temporary = os.environ.get('RUNNER_TEMP')
    if not temporary:
        raise ValueError('missing RUNNER_TEMP for mandatory pinned-UCI receipt')
    receipt = Path(temporary) / 'py006-uci' / 'ci-select003-python.json'
    # A failed rerun must never leave an earlier success in the uploaded bundle,
    # even when input validation or test discovery fails before execution.
    receipt.unlink(missing_ok=True)
    if not receipt.parent.is_dir():
        raise ValueError('missing existing pinned-UCI evidence directory')
    return receipt


def write_receipt(receipt, document):
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', dir=receipt.parent,
                                         prefix='.ci-select003-', delete=False) as output:
            temporary = Path(output.name)
            output.write(json.dumps(document, indent=2) + '\n')
        temporary.replace(receipt)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main(argv):
    if len(argv) != 1 or argv[0] not in MODES:
        print('usage: python/qualification/run.py {fast|uci|sleet}', file=sys.stderr)
        return 2
    mode = argv[0]
    try:
        receipt = prepare_receipt(mode)
        if sys.version_info < (3, 11):
            raise ValueError('Python 3.11+ required')
        require_inputs(mode)
        suite, files, identities = discover(mode)
    except (OSError, ValueError) as error:
        print(f'CI_SELECT_003_INVALID: {error}; tests_run=0 tests_registered=unknown',
              file=sys.stderr)
        return 1
    result = unittest.TextTestRunner(verbosity=2, resultclass=QualificationResult).run(suite)
    execution = execution_status(result, identities)
    if execution['status'] != 'success':
        print('CI_SELECT_003_QUALIFICATION_FAILED ' + json.dumps(execution, sort_keys=True),
              file=sys.stderr)
        return 1
    if receipt is not None:
        try:
            write_receipt(receipt, dict(schema_version=2, mode=mode, **execution,
                                       sources=[f.name for f in files], test_ids=identities))
        except OSError as error:
            print(f'CI_SELECT_003_RECEIPT_FAILED: {error}; ' +
                  json.dumps(execution, sort_keys=True), file=sys.stderr)
            return 1
    print(f'CI_SELECT_003_{mode.upper()}_COMPLETE tests={result.testsRun} files={len(files)}',
          flush=True)
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
