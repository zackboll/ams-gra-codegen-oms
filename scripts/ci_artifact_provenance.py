"""Resolve actual executions, not GitHub's copied partial-rerun job records.

Attempt job snapshots can relabel an old successful execution with a new ID
and run_attempt. Only the historical snapshot whose run interval contains the
execution is a producer. Embedded artifact claims never select that attempt.
"""
from datetime import datetime
import re


def timestamp(value):
    if not isinstance(value, str) or not value.endswith('Z'):
        raise ValueError('missing/invalid authoritative timestamp')
    return datetime.fromisoformat(value.replace('Z', '+00:00'))


def execution(job):
    # IDs/check URLs/run_attempt are changed by GitHub for copied executions.
    # Every execution-bearing field, including complete step metadata, must agree.
    return {key: job[key] for key in (
        'name', 'run_id', 'head_sha', 'status', 'conclusion', 'started_at', 'completed_at', 'steps')}


def resolve(artifact, job_name, jobs, attempts, run_id, current_attempt):
    """Return independently verified producer identity for one required artifact."""
    current = int(current_attempt)
    if current < 1 or current > 100 or set(attempts) != set(range(1, current + 1)):
        raise ValueError('missing/excessive workflow attempt history')
    latest = attempts[current]['run']
    if str(latest['id']) != str(run_id) or latest['run_attempt'] != current:
        raise ValueError('current workflow run/attempt mismatch')
    origin = artifact.get('workflow_run', {})
    if (str(origin.get('id')) != str(run_id) or origin.get('head_sha') != latest['head_sha']
            or origin.get('repository_id') != latest['repository']['id']
            or origin.get('head_repository_id') != latest['head_repository']['id']):
        raise ValueError('cross-run/head/repository artifact')
    if (artifact.get('expired') is not False or artifact.get('size_in_bytes', 0) <= 0
            or not isinstance(artifact.get('id'), int)
            or not isinstance(artifact.get('digest'), str)
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', artifact['digest'])):
        raise ValueError('expired/empty/unsigned artifact metadata')
    created = timestamp(artifact['created_at'])
    matches = [j for j in jobs if j['name'] == job_name]
    if len(matches) != 1:
        raise ValueError('missing/duplicate latest producer job: ' + job_name)
    selected = matches[0]
    if selected['run_attempt'] != current:
        raise ValueError('latest producer snapshot is not current attempt')
    if selected['status'] != 'completed' or selected['conclusion'] != 'success':
        raise ValueError('artifact producing job not successful')
    if not selected['steps'] or any(s.get('status') != 'completed' or s.get('conclusion') != 'success'
                                    for s in selected['steps']):
        raise ValueError('producer has missing/skipped/failed execution steps')
    producers = []
    last_start = None
    for number in range(1, current + 1):
        snapshot = attempts[number]
        run = snapshot['run']
        if (str(run['id']) != str(run_id) or run['run_attempt'] != number
                or any(run[key] != latest[key] for key in (
                    'head_sha', 'event', 'path', 'check_suite_id', 'repository', 'head_repository', 'pull_requests'))):
            raise ValueError('altered workflow attempt source/proposed merge provenance')
        start = timestamp(run['run_started_at'])
        if last_start is not None and start <= last_start:
            raise ValueError('unordered workflow attempts')
        last_start = start
        end = timestamp(attempts[number+1]['run']['run_started_at']) if number < current else None
        historical = [j for j in snapshot['jobs'] if j['name'] == job_name]
        if len(historical) > 1:
            raise ValueError('duplicate historical producer job')
        for job in historical:
            if (str(job['run_id']) != str(run_id) or job['head_sha'] != latest['head_sha']
                    or job['run_attempt'] != number):
                raise ValueError('altered historical producer metadata')
            began = timestamp(job['started_at'])
            # Ignore copied executions whose timestamps precede this attempt.
            if began < start:
                continue
            finished = timestamp(job['completed_at'])
            if end is not None and finished >= end:
                raise ValueError('producer execution overlaps workflow attempts')
            if (began > finished or not began <= created <= finished
                    or execution(job) != execution(selected)):
                continue
            if job['status'] != 'completed' or job['conclusion'] != 'success':
                raise ValueError('failed historical producer')
            # Bind publication to successful upload execution, not just a broad
            # job lifetime; reject missing/skipped/forged publication metadata.
            publication = ('Preserve mandatory shard evidence and failure diagnostics'
                           if artifact['name'].startswith('perf007-') else
                           'Preserve Task101 mandatory source and execution evidence'
                           if artifact['name'].startswith('task101-') else
                           'Upload mandatory source compiler and execution evidence')
            uploads = [s for s in job['steps'] if s.get('name') == publication
                       and s.get('conclusion') == 'success'
                       and s.get('status') == 'completed'
                       and timestamp(s['started_at']) <= created <= timestamp(s['completed_at'])]
            if len(uploads) != 1:
                raise ValueError('artifact creation not bound to a successful producer step')
            producers.append(dict(run_id=str(run_id), run_attempt=str(number),
                                  job_id=job['id'], job_name=job_name,
                                  artifact_id=artifact['id'], artifact_digest=artifact['digest'],
                                  created_at=artifact['created_at']))
    if len(producers) != 1:
        raise ValueError('missing/ambiguous/stale artifact producer execution: ' + job_name)
    return producers[0]


def verify_final_job(jobs, run, job_name):
    """The gate itself must really execute in the CURRENT attempt, never copied."""
    matches = [j for j in jobs if j['name'] == job_name]
    if len(matches) != 1:
        raise ValueError('missing/duplicate final gate identity')
    job = matches[0]
    if (job['run_id'] != run['id'] or job['run_attempt'] != run['run_attempt']
            or job['head_sha'] != run['head_sha'] or job['status'] != 'in_progress'
            or timestamp(job['started_at']) < timestamp(run['run_started_at'])):
        raise ValueError('final gate is not executing in current workflow attempt')