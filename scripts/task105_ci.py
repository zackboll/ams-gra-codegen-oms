"""Task105 additive ownership, projected onto unchanged Task104 contracts."""
import copy
import hashlib
import json

JOB = 'ada-real-uci-survivability-26-sleet'
RULE = {'category': 'ada-real-uci-survivability26', 'paths': ['scripts/task105/**', 'scripts/task105_evidence.py', 'scripts/task105_ci.py', 'scripts/test_ci_select_task105.py', 'tests/ada-real-uci-survivability26/**', 'tests/fixtures/task105/**'], 'jobs': ['rust', 'ada-json-guard', 'ada-repeated-choice-codec', 'ada-lacal-runtime-core', 'ada-websocket-transport'], 'deep': True, 'deep_jobs': ['ada-real-uci-survivability-26-sleet'], 'rationale': 'Authentic UCI 2.6 Survivability qualification and source-bound evidence', 'deep_rationale': 'Task105 independent generated UCI 2.6 transport under both compiler families'}


def project(policy, normalized):
    records = [r for r in policy['jobs'] if r['job_id'] == JOB]
    if len(records) != 1 or records[0]['workflow'] != 'deep-ci.yml':
        raise ValueError('Task105 independent Deep job missing/duplicate')
    record = records[0]
    if [(c['variant'], c['lane'], c['artifacts']) for c in record['cells']] != [
            ({'compiler': c}, lane, ['task105-ada-survivability-26-' + c])
            for c, lane in [('14', 1), ('16', 4)]]:
        raise ValueError('Task105 compiler/lane/artifact inventory changed')
    for value, expected in [
        (record['expected_header'], 'a71cbd016cea63e677fbcac2dd56ce3f335f80bcf61c35550dc7c3dd5f2a8c8b'),
        (normalized(record['original_steps_yaml']), 'fcc9a9dff9bb5d306f15acc3a7b84b87bebb75c2cc0e35d7889706e694e9bda4'),
        (json.dumps(record['original_steps'], sort_keys=True), '3d7f7fccc3d93df5c68ca315aa125386df4a200177b5640197a3d1dd5009e1e9'),
        (json.dumps(record['original_job'], sort_keys=True), '6df9289546c30b5eb435d9d053dcd1682148fdd074e8641f81d52b8e0e513e5e'),
    ]:
        if hashlib.sha256(value.encode()).hexdigest() != expected:
            raise ValueError('Task105 mandatory execution/publication contract changed')
    projected = copy.deepcopy(policy)
    owned = [r for r in projected['rules'] if r['category'] == RULE['category']]
    if owned != [RULE]:
        raise ValueError('Task105 qualification ownership changed')
    projected['rules'].remove(owned[0])
    for rule in projected['rules']:
        required = 'ada-real-uci-survivability-sleet' in rule['deep_jobs'] and rule['category'] != 'ada-real-uci-survivability'
        if (JOB in rule['deep_jobs']) != required:
            raise ValueError('Task105 dependency coverage changed')
        if required:
            rule['deep_jobs'].remove(JOB)
    return projected