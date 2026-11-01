"""Reconstruct Python registrations without importing or executing task code."""
from owned_qualification import ROOT, digest, document, python_fixtures, python_ids
from task101_evidence import PINS, INCLUDES, REV, TREE


def verify_bundle(files, mode, versions, sha, tree, run_id, attempt, root=ROOT):
    sources = sorted((root / 'python/qualification' / mode).glob('test_*.py'))
    fixtures = [r for r in python_fixtures(root) if r['mode'] == mode]
    hashes = {p.relative_to(root).as_posix(): digest(p.read_bytes()) for p in sources}
    for item in fixtures:
        hashes.update(item['hashes'])
    tests = sorted(t for source in sources for t in python_ids(source))
    expected_paths = set()
    for version in versions:
        for policy in ('normal', 'optimized'):
            prefix = version + '-' + policy + '/'
            expected_paths.update([prefix + 'result.json', prefix + 'execution.log'])
            result = document(files[prefix + 'result.json'])
            identity = result['identity']
            wanted = dict(sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt),
                          mode=mode, python=version, optimization=policy)
            if any(identity.get(k) != v for k, v in wanted.items()):
                raise ValueError('wrong Python source/run/attempt/interpreter/owner')
            if not identity.get('python_version', '').startswith(version + '.'):
                raise ValueError('wrong actual Python version')
            import re
            if any(not re.fullmatch('[0-9a-f]{64}', identity.get(k, ''))
                   for k in ('interpreter_sha256', 'generator_sha256')):
                raise ValueError('missing Python generator/interpreter identity')
            if mode in ('uci', 'sleet'):
                releases = ('25', '26') if mode == 'uci' else ('25',)
                if (identity.get('uci_roots') != {r: PINS[r] for r in releases} or
                        identity.get('uci_includes') != {r: INCLUDES[r] for r in releases}):
                    raise ValueError('wrong Python pinned UCI identity')
            if mode == 'sleet' and (identity.get('sleet_revision') != REV or identity.get('sleet_tree') != TREE or
                    not re.fullmatch('[0-9a-f]{64}', identity.get('sleet_binary_sha256', ''))):
                raise ValueError('wrong Python pinned Sleet identity')
            if (result.get('version') != 1 or result['hashes'] != hashes or result['fixtures'] != fixtures or
                    result['test_ids'] != tests or digest(files[prefix + 'execution.log']) != result['log_sha256']):
                raise ValueError('incomplete/forged Python registration/log evidence')
            execution = result['execution']
            if (execution.get('status') != 'success' or execution.get('reasons') != [] or
                    any(execution.get(k) != len(tests) for k in ('tests_registered', 'tests_run', 'tests_passed')) or
                    any(execution.get(k) != 0 for k in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes')) or
                    sorted(execution.get('executed_test_ids', [])) != tests or
                    sorted(execution.get('passed_test_ids', [])) != tests):
                raise ValueError('skipped/suppressed/failed Python qualification')
    if set(files) != expected_paths:
        raise ValueError('missing/unexpected Python execution inventory')
    return dict(mode=mode, versions=versions, policies=['normal', 'optimized'], tests=len(tests))