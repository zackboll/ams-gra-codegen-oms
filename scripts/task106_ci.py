"""Task106-only inventory projection onto the exact inherited Task105 policy."""
import copy
import hashlib
import json

JOB = 'ada-real-uci-multimessage-sleet'
RULE = {'category': 'ada-real-uci-multimessage', 'paths': ['scripts/task106/**', 'scripts/task106_evidence.py', 'scripts/task106_ci.py', 'scripts/test_ci_select_task106.py', 'tests/ada-real-uci-multimessage/**', 'tests/fixtures/task106/**'], 'jobs': ['rust', 'ada-json-guard', 'ada-repeated-choice-codec', 'ada-lacal-runtime-core', 'ada-websocket-transport'], 'deep': True, 'deep_jobs': [JOB], 'rationale': 'Task106 heterogeneous UCI 2.5 generated Ada qualification', 'deep_rationale': 'Three generated payload types on one pinned-Sleet connection under both compilers'}


def project(policy, normalized):
    records = [r for r in policy['jobs'] if r['job_id'] == JOB]
    if len(records) != 1 or records[0]['workflow'] != 'deep-ci.yml':
        raise ValueError('Task106 independent Deep job missing/duplicate')
    record = records[0]
    if [(c['variant'], c['lane'], c['artifacts']) for c in record['cells']] != [
            ({'compiler': compiler}, lane, ['task106-ada-multimessage-' + compiler])
            for compiler, lane in [('14', 2), ('16', 4)]]:
        raise ValueError('Task106 compiler/lane/artifact inventory changed')
    for value, expected in [
        (record['expected_header'], 'c901d37715e136758d01f93e5866a82895113c97f8056ed265f8bfff17c46612'),
        (normalized(record['original_steps_yaml']), '2c114ae7e5095400af37ddc23935d5d21b5ac46a11f084bb04649045d9d03c67'),
        (json.dumps(record['original_steps'], sort_keys=True), '262fda78e1e2f92bef7ef08760bb8aa084e56ae98a8d51b27c992b1af6ae34ae'),
        (json.dumps(record['original_job'], sort_keys=True), 'e7811d224eda0970bb55e3255db649f57216bd35a5c7c4f247bd63432f7ffc01'),
    ]:
        if hashlib.sha256(value.encode()).hexdigest() != expected:
            raise ValueError('Task106 mandatory execution/publication contract changed')
    projected = copy.deepcopy(policy)
    owned = [r for r in projected['rules'] if r['category'] == RULE['category']]
    if owned != [RULE]:
        raise ValueError('Task106 qualification ownership changed')
    projected['rules'].remove(owned[0])
    for rule in projected['rules']:
        required = bool(set(rule['deep_jobs']) & {
            'ada-real-uci-log-sleet', 'ada-real-uci-rdma-sleet', 'ada-real-uci-survivability-sleet'})
        if (JOB in rule['deep_jobs']) != required:
            raise ValueError('Task106 shared source/oracle dependency coverage changed')
        if required:
            rule['deep_jobs'].remove(JOB)
    return projected