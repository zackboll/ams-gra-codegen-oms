"""Additive Task102 ownership; inherited inventories retain their own pins."""
import copy
import hashlib
import json

JOB = 'ada-real-uci-log-sleet'


def project_rules(policy):
    rules = copy.deepcopy(policy['rules'])
    owned = [r for r in rules if r['category'] == 'ada-real-uci-log']
    if len(owned) != 1 or owned[0] != {
        'category': 'ada-real-uci-log',
        'paths': ['scripts/task102/**', 'scripts/task102_evidence.py', 'scripts/test_task102*.py',
                  'tests/ada-real-uci-log/**', 'tests/fixtures/task102/**'],
        'jobs': ['rust', 'ada-json-guard', 'ada-hexbinary-codec', 'ada-lacal-runtime-core', 'ada-websocket-transport'],
        'deep': True, 'deep_jobs': [JOB],
        'rationale': 'Real UCI Log campaign, fixtures, consumer and evidence verifier',
        'deep_rationale': 'Task102 actual generated Log transport under both compiler families',
    }:
        raise ValueError('Task102 qualification ownership changed')
    rules.remove(owned[0])
    for rule in rules:
        required = 'ada-real-uci-dlz-sleet' in rule['deep_jobs'] and rule['category'] != 'ada-real-uci-dlz'
        if (JOB in rule['deep_jobs']) != required:
            raise ValueError('Task102 dependency coverage changed')
        if required:
            rule['deep_jobs'].remove(JOB)
    return rules


def check_job(policy, normalized):
    records = [r for r in policy['jobs'] if r['job_id'] == JOB]
    if len(records) != 1 or records[0]['workflow'] != 'deep-ci.yml':
        raise ValueError('Task102 independent Deep job missing/duplicate')
    record = records[0]
    if [(c['variant'], c['lane'], c['artifacts']) for c in record['cells']] != [
        ({'compiler': c}, lane, ['task102-ada-real-uci-log-' + c]) for c, lane in [('14', 5), ('16', 2)]
    ]:
        raise ValueError('Task102 compiler/lane/artifact inventory changed')
    if (hashlib.sha256(record['expected_header'].encode()).hexdigest() != '122dc75469ad9c16745821f257e0e0f131201f3324ec1ab89e8b7eba02eb2a7a'
            or hashlib.sha256(normalized(record['original_steps_yaml']).encode()).hexdigest() != '2b24e53977c465526aef0fce0edd60c587b25f18bb9ad43ab3973fad81ac1f84'):
        raise ValueError('Task102 mandatory execution/publication contract changed')
    if (hashlib.sha256(json.dumps(record['original_steps'], sort_keys=True).encode()).hexdigest() != 'b944880c7a2136878656ac6378e5ee503df4034fd9fff2cccde59f5017f3f678'
            or hashlib.sha256(json.dumps(record['original_job'], sort_keys=True).encode()).hexdigest() != '2f5ee937f83b53b1faa7315f5db0187550f379fa48190febceaecffa50daddde'):
        raise ValueError('Task102 aggregate step contract changed')