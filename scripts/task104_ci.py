"""Task104 additive ownership, projected onto unchanged Task103 contracts."""
import copy
import hashlib
import json

JOB = 'ada-real-uci-survivability-sleet'
RULE = {
    'category': 'ada-real-uci-survivability',
    'paths': ['scripts/task104/**', 'scripts/task104_evidence.py', 'scripts/task104_ci.py',
              'scripts/test_ci_select_task104.py', 'tests/ada-real-uci-survivability/**', 'tests/fixtures/task104/**'],
    'jobs': ['rust', 'ada-json-guard', 'ada-repeated-choice-codec', 'ada-lacal-runtime-core', 'ada-websocket-transport'],
    'deep': True, 'deep_jobs': [JOB],
    'rationale': 'Real UCI Survivability campaign, fixtures, consumer and evidence verifier',
    'deep_rationale': 'Task104 actual generated Survivability transport under both compiler families',
}


def project(policy, normalized):
    records = [r for r in policy['jobs'] if r['job_id'] == JOB]
    if len(records) != 1 or records[0]['workflow'] != 'deep-ci.yml':
        raise ValueError('Task104 independent Deep job missing/duplicate')
    record = records[0]
    if [(c['variant'], c['lane'], c['artifacts']) for c in record['cells']] != [
            ({'compiler': c}, lane, ['task104-ada-real-uci-survivability-' + c])
            for c, lane in [('14', 4), ('16', 1)]]:
        raise ValueError('Task104 compiler/lane/artifact inventory changed')
    for value, expected in [
        (record['expected_header'], 'd8c54e19337239fa120009abbb9a75f5bfdc8ac34b3202e78f3410e2f3e202e8'),
        (normalized(record['original_steps_yaml']), '763e22435c1869e3dd637b883ef76f10a05e94016b95025bc0d3a7b273511ad3'),
        (json.dumps(record['original_steps'], sort_keys=True), '40f092d7e0fb11e94fd06d7efaee3e8fa93f277bd61ba4ceaa0a023fdbe81570'),
        (json.dumps(record['original_job'], sort_keys=True), 'f057d71ccd40a1c41fcfc42cb839ebdc4e996ac3b7785bcba495c869d80555cf'),
    ]:
        if hashlib.sha256(value.encode()).hexdigest() != expected:
            raise ValueError('Task104 mandatory execution/publication contract changed')
    projected = copy.deepcopy(policy)
    owned = [r for r in projected['rules'] if r['category'] == RULE['category']]
    if owned != [RULE]:
        raise ValueError('Task104 qualification ownership changed')
    projected['rules'].remove(owned[0])
    for rule in projected['rules']:
        required = 'ada-real-uci-rdma-sleet' in rule['deep_jobs'] and rule['category'] != 'ada-real-uci-rdma'
        if (JOB in rule['deep_jobs']) != required:
            raise ValueError('Task104 dependency coverage changed')
        if required:
            rule['deep_jobs'].remove(JOB)
    return projected