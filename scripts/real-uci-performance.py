#!/usr/bin/env python3
"""Compare complete GitHub API snapshots; never call projections measurements.

Run with original run/jobs, candidate run/jobs and downloaded shard directories.
Queue metrics are eligibility-to-first-setup, including lane/provisioning delay;
GitHub does not expose a separable pure scheduler wait duration.
"""
import argparse
from datetime import datetime
import json
from pathlib import Path

from real_uci_evidence import SHARDS, verify_bundle


def stamp(value):
    return datetime.fromisoformat(value.replace('Z', '+00:00')).timestamp()


def interval(job):
    if job['status'] != 'completed' or job['conclusion'] != 'success' or not job['steps']:
        raise ValueError('incomplete/failed hosted job: ' + job['name'])
    if any(s['conclusion'] != 'success' for s in job['steps']):
        raise ValueError('failed/skipped hosted step: ' + job['name'])
    start = stamp(job['steps'][0]['started_at'])
    end = stamp(job['completed_at'])
    if end <= start:
        raise ValueError('invalid execution timestamps')
    return start, end


def workflow(run, jobs, primary_names):
    if run['status'] != 'completed' or run['conclusion'] != 'success':
        raise ValueError('workflow not successfully complete')
    rows = []
    eligibility = stamp(run['run_started_at'])
    # The reusable Deep caller is eligible only after changes completes.
    if any(j['name'].startswith('deep-qualification / ') for j in jobs):
        changes = [j for j in jobs if j['name'] == 'changes']
        if len(changes) != 1:
            raise ValueError('missing reusable caller prerequisite')
        eligibility = interval(changes[0])[1]
        deep = [j for j in jobs if j['name'].startswith('deep-qualification / ')]
    else:
        deep = jobs
    for job in jobs:
        start, end = interval(job)
        rows.append(dict(name=job['name'], execution_seconds=end-start,
                         eligibility_to_setup_seconds=start-eligibility if job in deep else None))
    primary = [j for j in deep if j['name'].removeprefix('deep-qualification / ') in primary_names]
    if (len(primary) != len(primary_names) or
            {j['name'].removeprefix('deep-qualification / ') for j in primary} != set(primary_names)):
        raise ValueError('missing/duplicate primary hosted job')
    durations = [interval(j)[1]-interval(j)[0] for j in primary]
    return dict(run_id=run['id'], source_sha=run['head_sha'],
        primary_execution_critical_path_seconds=max(durations),
        primary_runner_minutes=sum(durations)/60,
        deep_max_execution_seconds=max(interval(j)[1]-interval(j)[0] for j in deep),
        deep_eligible_to_completion_seconds=max(interval(j)[1] for j in deep)-eligibility,
        deep_runner_minutes=sum(interval(j)[1]-interval(j)[0] for j in deep)/60,
        workflow_elapsed_seconds=stamp(run['updated_at'])-stamp(run['run_started_at']),
        complete_workflow_runner_minutes=sum(r['execution_seconds'] for r in rows)/60, jobs=rows)


def compare(before, after):
    old = before['primary_execution_critical_path_seconds']
    new = after['primary_execution_critical_path_seconds']
    return dict(baseline=before, candidate=after,
        primary_execution_reduction_percent=100*(old-new)/old,
        primary_execution_speedup=old/new,
        workflow_elapsed_improvement_percent=100*(before['workflow_elapsed_seconds']-after['workflow_elapsed_seconds'])/before['workflow_elapsed_seconds'],
        primary_runner_minute_change=after['primary_runner_minutes']-before['primary_runner_minutes'],
        deep_runner_minute_change=after['deep_runner_minutes']-before['deep_runner_minutes'],
        target_met=100*(old-new)/old >= 35,
        warning='Historical standalone Deep versus combined CI workflow elapsed is not like-for-like; also compare the parent reusable run. Queue includes lane wait and provisioning.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('baseline_run', 'baseline_jobs', 'candidate_run', 'candidate_jobs', 'evidence'):
        parser.add_argument(name, type=Path)
    parser.add_argument('--source-sha', required=True)
    parser.add_argument('--source-tree', required=True)
    args = parser.parse_args()
    load = lambda p: json.loads(p.read_bytes())
    old_run, new_run = load(args.baseline_run), load(args.candidate_run)
    before = workflow(old_run, load(args.baseline_jobs)['jobs'], {'real-uci'})
    after = workflow(new_run, load(args.candidate_jobs)['jobs'], set(SHARDS))
    shards = {}
    for shard in SHARDS:
        directory = args.evidence / ('perf007-'+shard)
        files = {p.relative_to(directory).as_posix(): p.read_bytes() for p in directory.rglob('*') if p.is_file()}
        shards[shard] = verify_bundle(files, shard, args.source_sha, args.source_tree,
                                      new_run['id'], new_run['run_attempt'])
    result = compare(before, after)
    result['verified_shards'] = shards
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()