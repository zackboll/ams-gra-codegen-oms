#!/usr/bin/env python3
"""Fail closed on selection, per-matrix authoritative outcomes and evidence."""
import json
import os
from pathlib import Path
import urllib.request
import urllib.error
import io
import zipfile
import hashlib
from ci_artifact_provenance import resolve, verify_final_job
from real_uci_evidence import SHARDS, ORIGINAL, verify_bundle
from task100_evidence import verify_bundle as verify_task100_bundle
from task101_evidence import JOB as TASK101_JOB, verify_bundle as verify_task101_bundle
from task102_evidence import JOB as TASK102_JOB, verify_bundle as verify_task102_bundle
from task103_evidence import JOB as TASK103_JOB, verify_bundle as verify_task103_bundle
from task105_evidence import JOB as TASK105_JOB, verify_bundle as verify_task105_bundle
from task104_evidence import JOB as TASK104_JOB, verify_bundle as verify_task104_bundle
from ci_selection import ROOT, complete_diff, deep_jobs, from_environment, git, inventory, ordinary_jobs, validate_deep


def validate(manifest, expected, needs, jobs, artifacts, policy):
    if manifest != expected:
        raise ValueError("selection differs from independently recomputed complete diff")
    validate_deep(manifest, policy)
    required = ordinary_jobs(policy)
    for prerequisite in ("changes", "baseline"):
        if needs.get(prerequisite, {}).get("result") != "success":
            raise ValueError("missing/failed mandatory " + prerequisite)
    for job_id, record in required.items():
        selected = manifest["jobs"][job_id]
        result = needs.get(job_id, {}).get("result")
        if not selected:
            if result != "skipped":
                raise ValueError("unselected job not explicitly skipped: " + job_id)
            continue
        if result != "success":
            raise ValueError("selected job not successful: " + job_id + ": " + str(result))
        if any(not manifest["jobs"].get(p) for p in record["prerequisites"]):
            raise ValueError("missing prerequisite: " + job_id)
        validate_cells(record, jobs, artifacts)
    if manifest["deep"]:
        if needs.get("deep-qualification", {}).get("result") != "success":
            raise ValueError("missing/failed/cancelled Deep CI prerequisite")
    elif needs.get("deep-qualification", {}).get("result") != "skipped":
        raise ValueError("unselected Deep CI not explicitly skipped")
    for job_id, record in deep_jobs(policy).items():
        if job_id in manifest['deep_jobs']:
            validate_cells(record, jobs, artifacts, prefix="deep-qualification / ")
        else:
            validate_skipped_deep(record, jobs, artifacts, caller_selected=manifest['deep'])
    known = {'deep-qualification / ' + c['name'] for r in deep_jobs(policy).values() for c in r['cells']}
    known.update('deep-qualification / ' + j for j in deep_jobs(policy))
    known.update(collapsed_matrix_name(r) for j, r in deep_jobs(policy).items()
                 if j not in manifest['deep_jobs'] and len(r['cells']) > 1)
    if any(j['name'].startswith('deep-qualification / ') and j['name'] not in known for j in jobs):
        raise ValueError('unregistered authoritative Deep job')


def collapsed_matrix_name(record):
    # Hosted job-level skips retain the literal, unexpanded matrix expression.
    # Derive only the reviewed inventory axis, never accept an arbitrary suffix.
    axis, = record['cells'][0]['variant']
    return 'deep-qualification / ' + record['job_id'] + ' (${{ matrix.' + axis + ' }})'


def validate_skipped_deep(record, jobs, artifacts, caller_selected):
    """Job-level if precedes matrix expansion: GitHub emits one collapsed skip.

    Reject expanded execution even if the reusable caller's aggregate is green.
    With the caller skipped GitHub may expose only that caller, not its children.
    These representations must also be confirmed against retained hosted jobs.
    """
    prefix = 'deep-qualification / '
    names = {prefix + record['job_id'], *(prefix + c['name'] for c in record['cells'])}
    if len(record['cells']) > 1:
        names.add(collapsed_matrix_name(record))
    matches = [j for j in jobs if j['name'] in names]
    if caller_selected and len(matches) != 1:
        raise ValueError('missing/ambiguous skipped Deep job: ' + record['job_id'])
    if not caller_selected and len(matches) > 1:
        raise ValueError('ambiguous skipped Deep caller child: ' + record['job_id'])
    for job in matches:
        if (job['status'] != 'completed' or job['conclusion'] != 'skipped'
                or job.get('steps') or job.get('runner_id', 0) not in (0, None)):
            raise ValueError('unselected Deep job executed qualification: ' + record['job_id'])
    owned = {n for c in record['cells'] for n in c['artifacts']}
    if any(a['name'] in owned for a in artifacts):
        raise ValueError('unexpected unselected Deep artifact: ' + record['job_id'])


def validate_cells(record, jobs, artifacts, prefix=""):
    for cell in record["cells"]:
        matches = [j for j in jobs if j["name"] == prefix + cell["name"]]
        if len(matches) != 1:
            raise ValueError("missing/ambiguous authoritative matrix cell: " + cell["name"])
        job = matches[0]
        if job["status"] != "completed" or job["conclusion"] != "success":
            raise ValueError("cell not successful: " + cell["name"])
        # A successful job with skipped qualification steps is NOT qualification.
        steps = job.get("steps", [])
        mandatory = record.get('integration_steps', record['original_steps'])
        if record['job_id'].startswith('ada-websocket-'):
            mandatory = [*mandatory, dict(name='Record Task100 integration run identity')]
        if len(steps) < len(mandatory) or any(s["conclusion"] != "success" for s in steps):
            raise ValueError("missing/skipped/failed qualification step: " + cell["name"])
        for step in mandatory:
            if 'name' in step and sum(s.get('name') == step['name'] for s in steps) != 1:
                raise ValueError('missing mandatory named step: ' + step['name'])
        for name in cell["artifacts"]:
            evidence = [a for a in artifacts if a["name"] == name]
            if (len(evidence) != 1 or evidence[0]['expired'] or evidence[0]['size_in_bytes'] <= 0
                    or evidence[0]['created_at'] < job['started_at']):
                raise ValueError("missing/expired/stale required evidence: " + name)


def api(path):
    url = "https://api.github.com/repos/" + os.environ["GITHUB_REPOSITORY"] + "/" + path
    request = urllib.request.Request(url, headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"],
        "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"})
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def pages(path, key):
    items = []
    for page in range(1, 1001):
        data = api(path + ("&" if "?" in path else "?") + f"per_page=100&page={page}")
        items.extend(data[key])
        if len(items) == data["total_count"]:
            return items
        if not data[key] or len(items) > data["total_count"]:
            break
    raise ValueError("incomplete or changing API pagination: " + path)


def evidence(run_id, attempt):
    if not 1 <= int(attempt) <= 100:
        raise ValueError('excessive/invalid workflow attempt')
    attempts = {n: dict(run=api(f'actions/runs/{run_id}/attempts/{n}'),
                       jobs=pages(f'actions/runs/{run_id}/attempts/{n}/jobs', 'jobs'))
                for n in range(1, int(attempt) + 1)}
    return dict(jobs=attempts[int(attempt)]['jobs'], attempts=attempts,
                artifacts=pages(f"actions/runs/{run_id}/artifacts", "artifacts"))


def verify_records(records, execution):
    """Bind every API record to the authoritative run, attempt and head source."""
    jobs = records['jobs']
    artifacts = records['artifacts']
    if len({j['id'] for j in jobs}) != len(jobs) or len({a['id'] for a in artifacts}) != len(artifacts):
        raise ValueError('duplicate authoritative job/artifact identity')
    for job in jobs:
        if (str(job['run_id']) != str(execution['id'])
                or str(job['run_attempt']) != str(execution['run_attempt'])
                or job['head_sha'] != execution['head_sha']):
            raise ValueError('authoritative job source/run/attempt mismatch')
    for artifact in artifacts:
        source = artifact.get('workflow_run', {})
        if str(source.get('id')) != str(execution['id']) or source.get('head_sha') != execution['head_sha']:
            raise ValueError('authoritative artifact source/run mismatch')


def download_bundle(artifact):
    """Read data only. Never extract paths or execute untrusted artifact code."""
    url = ('https://api.github.com/repos/' + os.environ['GITHUB_REPOSITORY']
           + f'/actions/artifacts/{artifact["id"]}/zip')
    request = urllib.request.Request(url, headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN']})
    # GitHub redirects the archive request to a time-limited signed blob URL.
    # Do not forward the API credential to that host.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None
    try:
        response = urllib.request.build_opener(NoRedirect).open(request, timeout=60)
    except urllib.error.HTTPError as error:
        if error.code != 302:
            raise
        location = error.headers['Location']
        error.close()
        if not location.startswith('https://'):
            raise ValueError('unsafe evidence redirect')
        response = urllib.request.urlopen(location, timeout=60)
    with response:
        data = response.read(512 * 1024 * 1024 + 1)
    if len(data) > 512 * 1024 * 1024:
        raise ValueError('oversized evidence archive')
    if artifact.get('digest') != 'sha256:' + hashlib.sha256(data).hexdigest():
        raise ValueError('artifact archive digest mismatch')
    files = {}
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        if sum(i.file_size for i in archive.infolist()) > 1024 * 1024 * 1024:
            raise ValueError('oversized expanded evidence')
        for entry in archive.infolist():
            if entry.is_dir():
                continue
            name = entry.filename
            if name in files or name.startswith('/') or '..' in name.split('/'):
                raise ValueError('duplicate/unsafe evidence path')
            files[name] = archive.read(entry)
    return files


def reconcile_primary(artifacts, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    completed = []
    summaries = {}
    for shard in SHARDS:
        matches = [a for a in artifacts if a['name'] == 'perf007-' + shard]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate shard artifact: ' + shard)
        producer = resolve(matches[0], 'deep-qualification / ' + shard, jobs, attempts, run_id, attempt)
        summary = verify_bundle(loader(matches[0]), shard, sha, tree, run_id, producer['run_attempt'])
        summary = dict(summary, producer=producer)
        summaries[shard] = summary
        completed.extend(summary['tasks'])
    if sorted(completed) != sorted(ORIGINAL) or len(set(completed)) != len(ORIGINAL):
        raise ValueError('original qualification != disjoint union of runtime evidence')
    return summaries


def verify_source(manifest, policy, run, sha, event_name):
    identities = manifest['identities']
    if identities['merge'] != sha or git(ROOT, 'rev-parse', 'HEAD').decode().strip() != sha:
        raise ValueError('final gate checkout/source mismatch')
    git(ROOT, 'merge-base', '--is-ancestor', policy['incorporated_main'], sha)
    git(ROOT, 'merge-base', '--is-ancestor', policy['task100_incorporated_main'], sha)
    git(ROOT, 'merge-base', '--is-ancestor', policy['reconciliation_main'], sha)
    git(ROOT, 'merge-base', '--is-ancestor', policy['ci_select_002_main'], sha)
    if event_name == 'pull_request':
        complete_diff(ROOT, identities['base'], identities['head'], sha)
        git(ROOT, 'merge-base', '--is-ancestor', policy['incorporated_main'], identities['base'])
        git(ROOT, 'merge-base', '--is-ancestor', policy['task100_incorporated_main'], identities['base'])
        git(ROOT, 'merge-base', '--is-ancestor', policy['reconciliation_main'], identities['base'])
        git(ROOT, 'merge-base', '--is-ancestor', policy['ci_select_002_main'], identities['base'])
    if run['event'] != event_name or run['head_sha'] != (identities['head'] if event_name == 'pull_request' else sha):
        raise ValueError('authoritative workflow run source mismatch')


def reconcile_task100(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    summaries = {}
    kinds = []
    if manifest['jobs']['ada-websocket-transport']:
        kinds.append(('transport', False))
    if 'ada-websocket-real-sleet' in manifest['deep_jobs']:
        kinds.append(('sleet', True))
    for kind, real_sleet in kinds:
        for compiler in ('14', '16'):
            name = f'task100-ada-websocket-{kind}-{compiler}'
            matches = [a for a in artifacts if a['name'] == name]
            if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
                raise ValueError('missing/duplicate Task100 artifact: ' + name)
            job_name = ('deep-qualification / ada-websocket-real-sleet' if real_sleet
                        else 'ada-websocket-transport') + f' ({compiler})'
            producer = resolve(matches[0], job_name, jobs, attempts, run_id, attempt)
            summaries[name] = dict(verify_task100_bundle(loader(matches[0]), compiler, real_sleet,
                                   sha, tree, run_id, producer['run_attempt']), producer=producer)
    return summaries


def reconcile_task101(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    summaries = {}
    if TASK101_JOB not in manifest['deep_jobs']:
        return summaries
    for compiler in ('14', '16'):
        name = 'task101-ada-real-uci-dlz-' + compiler
        matches = [a for a in artifacts if a['name'] == name]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate/expired Task101 artifact: ' + name)
        producer = resolve(matches[0], f'deep-qualification / {TASK101_JOB} ({compiler})',
                           jobs, attempts, run_id, attempt)
        summaries[name] = dict(verify_task101_bundle(loader(matches[0]), compiler, sha, tree,
                              run_id, producer['run_attempt']), producer=producer)
    return summaries


def reconcile_task102(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    summaries = {}
    if TASK102_JOB not in manifest['deep_jobs']:
        return summaries
    for compiler in ('14', '16'):
        name = 'task102-ada-real-uci-log-' + compiler
        matches = [a for a in artifacts if a['name'] == name]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate/expired Task102 artifact: ' + name)
        producer = resolve(matches[0], f'deep-qualification / {TASK102_JOB} ({compiler})',
                           jobs, attempts, run_id, attempt)
        summaries[name] = dict(verify_task102_bundle(loader(matches[0]), compiler, sha, tree,
                              run_id, producer['run_attempt']), producer=producer)
    return summaries


def reconcile_task103(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    summaries = {}
    if TASK103_JOB not in manifest['deep_jobs']:
        return summaries
    for compiler in ('14', '16'):
        name = 'task103-ada-real-uci-rdma-' + compiler
        matches = [a for a in artifacts if a['name'] == name]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate/expired Task103 artifact: ' + name)
        producer = resolve(matches[0], f'deep-qualification / {TASK103_JOB} ({compiler})',
                           jobs, attempts, run_id, attempt)
        summaries[name] = dict(verify_task103_bundle(loader(matches[0]), compiler, sha, tree,
                              run_id, producer['run_attempt']), producer=producer)
    return summaries


def reconcile_task104(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    summaries = {}
    if TASK104_JOB not in manifest['deep_jobs']:
        return summaries
    for compiler in ('14', '16'):
        name = 'task104-ada-real-uci-survivability-' + compiler
        matches = [a for a in artifacts if a['name'] == name]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate/expired Task104 artifact: ' + name)
        producer = resolve(matches[0], f'deep-qualification / {TASK104_JOB} ({compiler})',
                           jobs, attempts, run_id, attempt)
        summaries[name] = dict(verify_task104_bundle(loader(matches[0]), compiler, sha, tree,
                              run_id, producer['run_attempt']), producer=producer)
    return summaries


def reconcile_task105(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle, *, jobs, attempts):
    summaries = {}
    if TASK105_JOB not in manifest['deep_jobs']:
        return summaries
    for compiler in ('14', '16'):
        name = 'task105-ada-survivability-26-' + compiler
        matches = [a for a in artifacts if a['name'] == name]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate/expired Task105 artifact: ' + name)
        producer = resolve(matches[0], f'deep-qualification / {TASK105_JOB} ({compiler})',
                           jobs, attempts, run_id, attempt)
        summaries[name] = dict(verify_task105_bundle(loader(matches[0]), compiler, sha, tree,
                              run_id, producer['run_attempt']), producer=producer)
    return summaries


def main():
    manifest = json.loads(os.environ["SELECTION_MANIFEST"])
    expected = from_environment()
    run = os.environ["GITHUB_RUN_ID"]
    policy = inventory()
    execution = api(f'actions/runs/{run}/attempts/{os.environ["GITHUB_RUN_ATTEMPT"]}')
    if str(execution['id']) != run or str(execution['run_attempt']) != os.environ['GITHUB_RUN_ATTEMPT']:
        raise ValueError('authoritative run/attempt mismatch')
    verify_source(expected, policy, execution, os.environ['GITHUB_SHA'], os.environ['GITHUB_EVENT_NAME'])
    own = evidence(run, os.environ["GITHUB_RUN_ATTEMPT"])
    verify_records(own, execution)
    verify_final_job(own['jobs'], execution, os.environ['GITHUB_JOB'])
    for snapshot in own['attempts'].values():
        verify_source(expected, policy, snapshot['run'], os.environ['GITHUB_SHA'], os.environ['GITHUB_EVENT_NAME'])
    validate(manifest, expected, json.loads(os.environ["NEEDS_JSON"]), own["jobs"], own["artifacts"], policy)
    primary = {}
    task100 = reconcile_task100(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    task101 = reconcile_task101(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    task102 = reconcile_task102(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    task103 = reconcile_task103(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    task104 = reconcile_task104(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    task105 = reconcile_task105(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    if set(SHARDS) <= set(manifest['deep_jobs']):
        primary = reconcile_primary(own['artifacts'], os.environ['GITHUB_SHA'],
                                    git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                                    run, os.environ['GITHUB_RUN_ATTEMPT'], jobs=own['jobs'], attempts=own['attempts'])
    report = dict(version=manifest['version'], identities=manifest["identities"], selected_cells=manifest["selected_cells"],
                  skipped_cells=manifest["skipped_cells"], deep=manifest["deep"],
                  deep_jobs=manifest['deep_jobs'], deep_selected_cells=manifest['deep_selected_cells'],
                  deep_skipped_cells=manifest['deep_skipped_cells'], conclusion="success",
                  run_id=run, gate_attempt=os.environ['GITHUB_RUN_ATTEMPT'],
                  real_uci_shards=primary, task100=task100, task101=task101, task102=task102, task103=task103, task104=task104, task105=task105)
    Path("ci-required-result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("PR_CI_REQUIRED_COMPLETE", json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()