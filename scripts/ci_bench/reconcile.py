"""Independent manual-run evidence consumer. No candidate code or extraction."""
import copy
import hashlib
import io
import json
import math
import os
import re
from pathlib import Path, PurePosixPath
import sys
import tarfile
import urllib.error
import urllib.request
import zipfile

# Direct execution starts with scripts/ on sys.path, like the required CI gate.
if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_artifact_provenance import resolve, timestamp, verify_final_job
from ci_bench import contract, controller

WORKFLOW = '.github/workflows/portable-provider-benchmark.yml'
UPLOADS = {'measurements': 'Upload mandatory measurements',
           'upload-receipt': 'Upload mandatory transport receipt'}
STEPS = ('Authorize exact main controller and candidate commit',
         'Build trusted pinned tool and dependency image',
         'Execute isolated compiler A B B A', 'Package bounded mandatory evidence',
         UPLOADS['measurements'], 'Account for actual packaging and upload step',
         UPLOADS['upload-receipt'], 'Enforce complete campaign and acceptance gates')


def api(path):
    request = urllib.request.Request('https://api.github.com/repos/' + controller.REPOSITORY + '/' + path,
        headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN'],
                 'Accept': 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28'})
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def pages(path, key):
    items = []
    for page in range(1, 101):
        data = api(path + f'?per_page=100&page={page}')
        items.extend(data[key])
        if len(items) == data['total_count']:
            return items
        if not data[key] or len(items) > data['total_count']:
            break
    raise ValueError('incomplete/changing/excessive API pagination')


def decode_archive(data, artifact):
    if len(data) > 128 * 1024**2 or artifact['digest'] != 'sha256:' + hashlib.sha256(data).hexdigest():
        raise ValueError('oversized or corrupt artifact archive')
    files = {}
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        if sum(i.file_size for i in archive.infolist()) > 128 * 1024**2:
            raise ValueError('oversized expanded artifact')
        for info in archive.infolist():
            path = PurePosixPath(info.filename)
            if (info.is_dir() or path.name != info.filename or info.filename in files
                    or info.filename in ('.', '..') or '\\' in info.filename
                    or (info.external_attr >> 16) & 0o170000 == 0o120000):
                raise ValueError('unsafe/duplicate artifact member')
            files[info.filename] = archive.read(info)
    return files


def download(artifact):
    # Never forward the API token to the signed archive storage host.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None
    request = urllib.request.Request('https://api.github.com/repos/' + controller.REPOSITORY
        + f'/actions/artifacts/{artifact["id"]}/zip',
        headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN']})
    try:
        response = urllib.request.build_opener(NoRedirect).open(request, timeout=60)
    except urllib.error.HTTPError as error:
        if error.code != 302:
            raise
        location = error.headers['Location']
        error.close()
        if not location.startswith('https://'):
            raise ValueError('unsafe artifact redirect')
        response = urllib.request.urlopen(location, timeout=60)
    with response:
        data = response.read(128 * 1024**2 + 1)
    return decode_archive(data, artifact)


def producer(artifact, job, kind, jobs, attempts, run_id, attempt):
    # Reuse the unchanged CI-RETRY resolver and its independent historical
    # execution intervals. Adapt only the benchmark's named publication step.
    def adapt(records):
        records = copy.deepcopy(records)
        for record in records:
            if record['name'] == job:
                for step in record['steps']:
                    if step['name'] == UPLOADS[kind]:
                        step['name'] = 'Upload mandatory source compiler and execution evidence'
        return records
    history = {n: dict(run=s['run'], jobs=adapt(s['jobs'])) for n, s in attempts.items()}
    origin = resolve(artifact, job, adapt(jobs), history, run_id, attempt)
    execution = next(j for j in attempts[int(origin['run_attempt'])]['jobs'] if j['id'] == origin['job_id'])
    names = [s['name'] for s in execution['steps']]
    if any(names.count(name) != 1 for name in STEPS):
        raise ValueError('missing/duplicate mandatory compiler step')
    return origin


def finite_positive(value):
    return isinstance(value, (int, float)) and math.isfinite(value) and value > 0


def verify_logs(data, rows):
    expected = {f'worker-{i}.log': rows[i] for i in range(4)}
    seen = set()
    with tarfile.open(fileobj=io.BytesIO(data), mode='r|gz') as archive:
        for member in archive:
            if (member.name not in expected or member.name in seen or not member.isfile()
                    or not 0 < member.size <= controller.LIMIT):
                raise ValueError('missing/unsafe/oversized worker logs')
            row = expected[member.name]
            seen.add(member.name)
            if row['worker_log'] != member.name:
                raise ValueError('worker log iteration identity mismatch')
            text = archive.extractfile(member).read(controller.LIMIT + 1).decode(errors='replace')
            emitted = [json.loads(line.removeprefix('CI_BENCH_RESULT ')) for line in text.splitlines()
                       if line.startswith('CI_BENCH_RESULT ')]
            if emitted != [row['candidate_report']]:
                raise ValueError('worker log/report counts and identities differ')
    if seen != set(expected):
        raise ValueError('missing worker logs')


def verify(artifacts, jobs, attempts, run_id, attempt, workflow, tree, workflow_digest, needs, loader):
    if set(needs) != set(contract.JOBS.values()) or any(n.get('result') != 'success' for n in needs.values()):
        raise ValueError('both compiler needs must succeed')
    current = attempts[int(attempt)]['run']
    if (current['head_sha'] != workflow or current['path'] != WORKFLOW
            or current['event'] != 'workflow_dispatch'
            or current['head_branch'] != 'main'
            or current['repository']['full_name'] != controller.REPOSITORY
            or current['repository']['id'] != current['head_repository']['id']):
        raise ValueError('unauthorized workflow identity')
    verify_final_job(jobs, current, contract.FINAL_JOB)
    combined, verified, executions, identities = [], {}, [], []
    for compiler, job_name in contract.JOBS.items():
        bundles, origins = {}, {}
        for kind in UPLOADS:
            name = f'ci-bench-001-gnat-{compiler}-{kind}'
            matches = [a for a in artifacts if a['name'] == name]
            if len(matches) != 1:
                raise ValueError('missing/ambiguous mandatory compiler artifact: ' + name)
            artifact = matches[0]
            origin = producer(artifact, job_name, kind, jobs, attempts, run_id, attempt)
            bundles[kind] = loader(artifact)
            origins[kind] = origin
        origin = origins['measurements']
        if any(origins['upload-receipt'][k] != origin[k] for k in ('run_id', 'run_attempt', 'job_id', 'job_name')):
            raise ValueError('measurement and receipt producers differ')
        files = bundles['measurements']
        packaging = json.loads(files['packaging.json'])
        if (set(packaging['files']) != set(files) - {'packaging.json'}
                or not {'result.json', 'identity.json', 'image.json', 'preflight.json',
                        'progress.json', 'current-contention.json', 'worker-logs.tar.gz'} <= set(packaging['files'])
                or any(hashlib.sha256(files[p]).hexdigest() != digest for p, digest in packaging['files'].items())
                or not finite_positive(packaging['wall_seconds'])
                or packaging['archive_bytes'] != len(files['worker-logs.tar.gz'])):
            raise ValueError('packaging inventory/digests/measurements invalid')
        if set(bundles['upload-receipt']) != {'upload-receipt.json'}:
            raise ValueError('unexpected receipt members')
        identity = json.loads(files['identity.json'])
        result = json.loads(files['result.json'])
        receipt = json.loads(bundles['upload-receipt']['upload-receipt.json'])
        expected = dict(source_commit=contract.SOURCE_COMMIT, source_tree=contract.SOURCE_TREE,
                        workflow_commit=workflow, workflow_tree=tree, workflow_sha256=workflow_digest,
                        compiler=compiler, job=job_name, job_id=str(origin['job_id']),
                        run_id=str(run_id), run_attempt=origin['run_attempt'])
        if any(identity.get(k) != v for k, v in expected.items()):
            raise ValueError('source/workflow/producer identity mismatch')
        if not re.fullmatch('[0-9a-f]{64}', identity['source_archive_sha256']):
            raise ValueError('missing source archive identity')
        snapshot = attempts[int(origin['run_attempt'])]
        job = next(j for j in snapshot['jobs'] if j['id'] == origin['job_id'])
        if (identity['runner_id'] != job['runner_id'] or identity['runner_name'] != job['runner_name']
                or not identity['runner_id'] or not identity['runner_name']):
            raise ValueError('same-runner producer identity mismatch')
        duration = (timestamp(job['completed_at']) - timestamp(job['started_at'])).total_seconds()
        if not 0 < duration <= contract.JOB_MINUTES * 60:
            raise ValueError('compiler hosted job limit exceeded')
        controller.validate_result(result, receipt, identity)
        verify_logs(files['worker-logs.tar.gz'], result['runs'])
        image = json.loads(files['image.json'])
        if (image['image_id'] != result['image_id']
                or not re.fullmatch('sha256:[0-9a-f]{64}', image['image_id'])
                or not finite_positive(image['setup_seconds']) or image['setup_seconds'] > contract.IMAGE_SECONDS
                or receipt['artifact_id'] != str(origin['artifact_id'])
                or 'sha256:' + receipt['artifact_digest'] != origin['artifact_digest']
                or not finite_positive(receipt['upload_step_envelope_seconds'])
                or receipt['uploaded_input_bytes'] != sum(map(len, files.values()))):
            raise ValueError('image or upload receipt accounting mismatch')
        receipt_step = next(s for s in job['steps'] if s['name'] == UPLOADS['upload-receipt'])
        receipt_upload_seconds = (timestamp(receipt_step['completed_at']) - timestamp(receipt_step['started_at'])).total_seconds()
        if (not finite_positive(identity['preparation_seconds']) or not finite_positive(receipt_upload_seconds)
                or identity['preparation_seconds'] + result['preflight']['elapsed_seconds'] > 300
                or packaging['wall_seconds'] + receipt['upload_step_envelope_seconds'] + receipt_upload_seconds > 600):
            raise ValueError('preparation/packaging/upload budget exceeded')
        if json.loads(files['preflight.json']) != dict(allocation=result['allocation'], sample=result['preflight']):
            raise ValueError('preflight evidence mismatch')
        if json.loads(files['progress.json']) != dict(identity=identity, runs=result['runs']):
            raise ValueError('progress evidence incomplete')
        if json.loads(files['current-contention.json']) != dict(iteration=3, samples=result['runs'][-1]['samples']):
            raise ValueError('final contention evidence mismatch')
        combined.extend(result['runs'])
        identities.append(identity)
        executions.append(job)
        verified[compiler] = dict(producers=origins, job_seconds=duration,
                                 runner_id=job['runner_id'], runner_name=job['runner_name'],
                                 image=image, packaging_seconds=packaging['wall_seconds'],
                                 preparation_seconds=identity['preparation_seconds'],
                                 receipt_upload_seconds=receipt_upload_seconds,
                                 upload_envelope_seconds=receipt['upload_step_envelope_seconds'],
                                 upload_steps=[s for s in job['steps'] if s['name'] in UPLOADS.values()])
    if (timestamp(executions[0]['completed_at']) > timestamp(executions[1]['started_at'])
            or timestamp(executions[1]['completed_at']) > timestamp(next(
                j for j in jobs if j['name'] == contract.FINAL_JOB)['started_at'])):
        raise ValueError('compiler executions overlap or are out of order')
    if identities[0]['source_archive_sha256'] != identities[1]['source_archive_sha256']:
        raise ValueError('compiler source archives differ')
    controller.validate_rows(combined)
    return dict(valid=True, completed=True, iterations=len(combined), performance_success=False,
                workflow_commit=workflow, workflow_tree=tree, workflow_sha256=workflow_digest,
                source_commit=contract.SOURCE_COMMIT, source_tree=contract.SOURCE_TREE,
                run_id=str(run_id), run_attempt=str(attempt), compilers=verified,
                totals={key: sum(row['candidate_report']['counts'][key] for row in combined)
                        for key in ('provider', 'generated_support', 'bind', 'link', 'execute')})


def main():
    output = controller.work() / 'reconciliation.json'
    controller.write(output, dict(valid=False, completed=False, disposition='RECONCILIATION_FAILED'))
    source, workflow = controller.authorize(os.environ)
    controller.validate_source_scope(source)
    if (os.environ.get('GITHUB_JOB') != contract.FINAL_JOB
            or controller.run(['git', '-C', str(controller.ROOT), 'rev-parse', 'HEAD']) != workflow):
        raise ValueError('trusted final controller identity mismatch')
    run_id, attempt = os.environ['GITHUB_RUN_ID'], os.environ['GITHUB_RUN_ATTEMPT']
    if not 1 <= int(attempt) <= 100:
        raise ValueError('invalid/excessive attempt history')
    attempts = {n: dict(run=api(f'actions/runs/{run_id}/attempts/{n}'),
                       jobs=pages(f'actions/runs/{run_id}/attempts/{n}/jobs', 'jobs'))
                for n in range(1, int(attempt) + 1)}
    result = verify(pages(f'actions/runs/{run_id}/artifacts', 'artifacts'), attempts[int(attempt)]['jobs'],
                    attempts, run_id, attempt, workflow,
                    controller.run(['git', '-C', str(controller.ROOT), 'rev-parse', 'HEAD^{tree}']),
                    hashlib.sha256((controller.ROOT / WORKFLOW).read_bytes()).hexdigest(),
                    json.loads(os.environ['BENCH_NEEDS']), download)
    controller.write(output, result)


if __name__ == '__main__':
    main()