"""Explicit manual benchmark inventory extension; no wildcard registration."""
import hashlib
from pathlib import Path

WORKFLOW = 'portable-provider-benchmark.yml'
JOB = 'portable-provider-benchmark'


def check(policy, workflows, root):
    record = policy.get('manual_benchmark')
    if not isinstance(record, dict) or set(record) != {
            'workflow', 'job_id', 'lane', 'events', 'source_policy', 'workflow_sha256', 'sources_sha256'}:
        raise ValueError('missing/unrecognized manual benchmark inventory')
    if (record['workflow'], record['job_id'], record['lane'], record['events'], record['source_policy']) != (
            WORKFLOW, JOB, 3, ['workflow_dispatch'], 'current main controller; exact untrusted repository commit'):
        raise ValueError('manual benchmark authorization changed')
    text = workflows.get(WORKFLOW, '')
    # Exact normalized trusted definition: any event/permission/runner/checkout,
    # input interpolation, lane, timeout, upload or container command edit fails.
    from ci_policy import normalized
    digest = hashlib.sha256(normalized(text).encode()).hexdigest()
    if digest != record['workflow_sha256'] or digest != WORKFLOW_SHA256:
        raise ValueError('manual benchmark workflow trust contract changed')
    expected_paths = {f'scripts/ci_bench/{name}' for name in (
        'Dockerfile', 'controller.py', 'worker.py', 'provision.py', 'contract.py')}
    if set(record['sources_sha256']) != expected_paths:
        raise ValueError('manual benchmark source inventory incomplete')
    for path, expected in record['sources_sha256'].items():
        if hashlib.sha256((Path(root) / path).read_bytes()).hexdigest() != expected:
            raise ValueError('manual benchmark controller source changed: ' + path)


# Recomputed only as part of an explicitly reviewed CI-BENCH workflow change.
WORKFLOW_SHA256 = '5d4933809b9d4a0c7bdbde9211482dd6de2c79cc7faec1f3d8e6d5a64f2bb61b'