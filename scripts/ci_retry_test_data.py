"""Synthetic API metadata ONLY for verifier tests, never qualification evidence."""
import copy


def job_name(name):
    if name.startswith('task103-'):
        return 'deep-qualification / ada-real-uci-rdma-sleet' + f' ({name[-2:]})'
    if name.startswith('task102-'):
        return 'deep-qualification / ada-real-uci-log-sleet' + f' ({name[-2:]})'
    if name.startswith('task101-'):
        return 'deep-qualification / ada-real-uci-dlz-sleet' + f' ({name[-2:]})'
    if name.startswith('perf007-'):
        return 'deep-qualification / ' + name.removeprefix('perf007-')
    return ('deep-qualification / ada-websocket-real-sleet' if '-sleet-' in name
            else 'ada-websocket-transport') + f' ({name[-2:]})'


def metadata(artifacts, current=2, producers=None):
    producers = producers or {a['name']: current for a in artifacts}
    attempts = {}
    jobs = []
    for number in range(1, current + 1):
        run = dict(id=123, run_attempt=number, head_sha='c'*40, event='pull_request',
                   path='.github/workflows/ci.yml', check_suite_id=456,
                   run_started_at=f'2026-10-27T{number:02d}:00:00Z',
                   repository=dict(id=10), head_repository=dict(id=10),
                   pull_requests=[dict(number=7, head=dict(sha='c'*40), base=dict(sha='d'*40))])
        attempts[number] = dict(run=run, jobs=[])
        for index, artifact in enumerate(artifacts):
            producer = producers[artifact['name']]
            if producer > number:
                continue
            prefix = f'2026-10-27T{producer:02d}'
            publication = ('Preserve mandatory shard evidence and failure diagnostics'
                           if artifact['name'].startswith('perf007-') else
                           'Upload mandatory source compiler and execution evidence')
            job = dict(id=number*100 + index, run_id=123, run_attempt=number,
                       head_sha='c'*40, name=job_name(artifact['name']),
                       started_at=prefix+':01:00Z', completed_at=prefix+':10:00Z',
                       status='completed', conclusion='success',
                       steps=[dict(name=publication, number=1, status='completed', conclusion='success',
                                   started_at=prefix+':08:00Z', completed_at=prefix+':09:00Z')])
            attempts[number]['jobs'].append(job)
    for index, artifact in enumerate(artifacts):
        artifact.update(id=index+1, expired=False, size_in_bytes=1, digest='sha256:'+'f'*64,
                        created_at=f'2026-10-27T{producers[artifact["name"]]:02d}:08:30Z',
                        workflow_run=dict(id=123, head_sha='c'*40, repository_id=10, head_repository_id=10))
    jobs = copy.deepcopy(attempts[current]['jobs'])
    return jobs, attempts