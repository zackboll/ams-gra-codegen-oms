"""Explicit manual benchmark inventory extension; no wildcard registration."""
import hashlib
from pathlib import Path
from . import contract

WORKFLOW = 'portable-provider-benchmark.yml'
JOBS = (*contract.JOBS.values(), contract.FINAL_JOB)


def check(policy, workflows, root):
    record = policy.get('manual_benchmark')
    if not isinstance(record, dict) or set(record) != {
            'workflow', 'jobs', 'lane', 'events', 'source_policy', 'workflow_sha256', 'sources_sha256'}:
        raise ValueError('missing/unrecognized manual benchmark inventory')
    topology = {
        contract.JOBS['14']: dict(compiler='14', needs=[], timeout_minutes=360),
        contract.JOBS['16']: dict(compiler='16', needs=[contract.JOBS['14']], timeout_minutes=360),
        contract.FINAL_JOB: dict(needs=list(contract.JOBS.values()), timeout_minutes=15)}
    if (record['workflow'], record['jobs'], record['lane'], record['events'], record['source_policy']) != (
            WORKFLOW, topology, 3, ['workflow_dispatch'], 'current main controller; exact untrusted repository commit'):
        raise ValueError('manual benchmark authorization changed')
    text = workflows.get(WORKFLOW, '')
    # Exact normalized trusted definition: any event/permission/runner/checkout,
    # input interpolation, lane, timeout, upload or container command edit fails.
    from workflow_text import normalized
    digest = hashlib.sha256(normalized(text).encode()).hexdigest()
    if digest != record['workflow_sha256'] or digest != WORKFLOW_SHA256:
        raise ValueError('manual benchmark workflow trust contract changed')
    expected_paths = {f'scripts/ci_bench/{name}' for name in (
        'Dockerfile', 'controller.py', 'worker.py', 'provision.py', 'contract.py', 'reconcile.py')}
    expected_paths.add('scripts/ci_artifact_provenance.py')
    if set(record['sources_sha256']) != expected_paths:
        raise ValueError('manual benchmark source inventory incomplete')
    for path, expected in record['sources_sha256'].items():
        if hashlib.sha256((Path(root) / path).read_bytes()).hexdigest() != expected:
            raise ValueError('manual benchmark controller source changed: ' + path)


# Recomputed only as part of an explicitly reviewed CI-BENCH workflow change.
WORKFLOW_SHA256 = 'f07d07683698e768d6031f16b6f0e1998015fc9a491a4e259a77c8df2d130acf'