#!/usr/bin/env python3
"""Fail closed on selection, per-matrix authoritative outcomes and evidence."""
import json
import os
from pathlib import Path
import urllib.request
import urllib.error
import io
import zipfile
from real_uci_evidence import SHARDS, ORIGINAL, verify_bundle
from task100_evidence import verify_bundle as verify_task100_bundle
from ci_selection import ROOT, complete_diff, from_environment, git, inventory, ordinary_jobs


def validate(manifest, expected, needs, jobs, artifacts, policy):
    if manifest != expected:
        raise ValueError("selection differs from independently recomputed complete diff")
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
        for record in policy["jobs"]:
            if record["workflow"] == "deep-ci.yml":
                validate_cells(record, jobs, artifacts, prefix="deep-qualification / ")
    elif needs.get("deep-qualification", {}).get("result") != "skipped":
        raise ValueError("unselected Deep CI not explicitly skipped")


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
            evidence = [a for a in artifacts if a["name"] == name and not a["expired"]
                        and a["size_in_bytes"] > 0 and a["created_at"] >= job["started_at"]]
            if len(evidence) != 1:
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
    return dict(jobs=pages(f"actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs"),
                artifacts=pages(f"actions/runs/{run_id}/artifacts", "artifacts"))


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


def reconcile_primary(artifacts, sha, tree, run_id, attempt, loader=download_bundle):
    completed = []
    summaries = {}
    for shard in SHARDS:
        matches = [a for a in artifacts if a['name'] == 'perf007-' + shard]
        if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
            raise ValueError('missing/duplicate shard artifact: ' + shard)
        summary = verify_bundle(loader(matches[0]), shard, sha, tree, run_id, attempt)
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
    if event_name == 'pull_request':
        complete_diff(ROOT, identities['base'], identities['head'], sha)
        git(ROOT, 'merge-base', '--is-ancestor', policy['incorporated_main'], identities['base'])
        git(ROOT, 'merge-base', '--is-ancestor', policy['task100_incorporated_main'], identities['base'])
        git(ROOT, 'merge-base', '--is-ancestor', policy['reconciliation_main'], identities['base'])
    if run['event'] != event_name or run['head_sha'] != (identities['head'] if event_name == 'pull_request' else sha):
        raise ValueError('authoritative workflow run source mismatch')


def reconcile_task100(artifacts, manifest, sha, tree, run_id, attempt, loader=download_bundle):
    summaries = {}
    kinds = []
    if manifest['jobs']['ada-websocket-transport']:
        kinds.append(('transport', False))
    if manifest['deep']:
        kinds.append(('sleet', True))
    for kind, real_sleet in kinds:
        for compiler in ('14', '16'):
            name = f'task100-ada-websocket-{kind}-{compiler}'
            matches = [a for a in artifacts if a['name'] == name]
            if len(matches) != 1 or matches[0]['expired'] or matches[0]['size_in_bytes'] <= 0:
                raise ValueError('missing/duplicate Task100 artifact: ' + name)
            summaries[name] = verify_task100_bundle(loader(matches[0]), compiler, real_sleet,
                                                    sha, tree, run_id, attempt)
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
    validate(manifest, expected, json.loads(os.environ["NEEDS_JSON"]), own["jobs"], own["artifacts"], policy)
    primary = {}
    task100 = reconcile_task100(own['artifacts'], manifest, os.environ['GITHUB_SHA'],
                               git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                               run, os.environ['GITHUB_RUN_ATTEMPT'])
    if manifest['deep']:
        primary = reconcile_primary(own['artifacts'], os.environ['GITHUB_SHA'],
                                    git(ROOT, 'rev-parse', 'HEAD^{tree}').decode().strip(),
                                    run, os.environ['GITHUB_RUN_ATTEMPT'])
    report = dict(identities=manifest["identities"], selected_cells=manifest["selected_cells"],
                  skipped_cells=manifest["skipped_cells"], deep=manifest["deep"], conclusion="success",
                  real_uci_shards=primary, task100=task100)
    Path("ci-required-result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("PR_CI_REQUIRED_COMPLETE", json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()