"""Task103-only CI ownership projected back onto immutable Task102 policy."""
import copy
import hashlib
import json

JOB = 'ada-real-uci-rdma-sleet'
RULE = {
    'category': 'ada-real-uci-rdma',
    'paths': ['scripts/task103/**', 'scripts/task103_evidence.py', 'scripts/task103_ci.py',
              'scripts/test_ci_select_task103.py', 'tests/ada-real-uci-rdma/**', 'tests/fixtures/task103/**'],
    'jobs': ['rust', 'ada-json-guard', 'ada-repeated-choice-codec',
             'ada-lacal-runtime-core', 'ada-websocket-transport'],
    'deep': True, 'deep_jobs': [JOB],
    'rationale': 'Real UCI RDMA campaign, fixtures, consumer and evidence verifier',
    'deep_rationale': 'Task103 actual generated RDMA transport under both compiler families',
}


def project(policy, normalized):
    records = [r for r in policy['jobs'] if r['job_id'] == JOB]
    if len(records) != 1 or records[0]['workflow'] != 'deep-ci.yml':
        raise ValueError('Task103 independent Deep job missing/duplicate')
    record = records[0]
    if [(c['variant'], c['lane'], c['artifacts']) for c in record['cells']] != [
            ({'compiler': c}, lane, ['task103-ada-real-uci-rdma-' + c])
            for c, lane in [('14', 1), ('16', 3)]]:
        raise ValueError('Task103 compiler/lane/artifact inventory changed')
    for value, expected in [
        (record['expected_header'], 'f0d696cf9571c69db1938de9523d3e35a1847da011adabd5f9497e5f0d607043'),
        (normalized(record['original_steps_yaml']), 'a097befb5b1de81c6f6c75b488b56bb8759b5c80b2f946d8d2989b1d88e06bf0'),
        (json.dumps(record['original_steps'], sort_keys=True), '1e915e9a1d51b11e34745ab80ac1a7bfe5d64436ed626a1050c1a92f14811c68'),
        (json.dumps(record['original_job'], sort_keys=True), '584a8a901dc87eabb8c28c12cf81f8b4cb12d08d85c07f39b1e2d8b71b54ae9a'),
    ]:
        if hashlib.sha256(value.encode()).hexdigest() != expected:
            raise ValueError('Task103 mandatory execution/publication contract changed')
    projected = copy.deepcopy(policy)
    owned = [r for r in projected['rules'] if r['category'] == RULE['category']]
    if owned != [RULE]:
        raise ValueError('Task103 qualification ownership changed')
    projected['rules'].remove(owned[0])
    for rule in projected['rules']:
        required = 'ada-real-uci-dlz-sleet' in rule['deep_jobs'] and rule['category'] != 'ada-real-uci-dlz'
        if (JOB in rule['deep_jobs']) != required:
            raise ValueError('Task103 dependency coverage changed')
        if required:
            rule['deep_jobs'].remove(JOB)
    return projected