"""Fail-closed GitHub provenance checks. Never import code from an OMS PR.

Mirrored into the private executor; changes require coordinated review/tests.
Only stdlib is used. API objects and downloaded JSON are untrusted data.
"""
import base64
import datetime as dt
import hashlib
import io
import json
import re
import urllib.error
import urllib.parse
import urllib.request
import zipfile

SOURCE = 'zackboll/ams-gra-codegen-oms'
SOURCE_ID = 1373902528
DEST = 'zackboll/ci-runner-pilot'
DEST_ID = 1430048464
OWNER_ID = 4860920
SIGNAL_PATH = '.github/workflows/oms-trusted-pr-signal.yml'
DISPATCH_PATH = '.github/workflows/oms-ryzen-trusted-dispatch.yml'
EXECUTOR_PATH = '.github/workflows/oms-trusted-float32.yml'
SHA = re.compile(r'^[0-9a-f]{40}$')
CONTEXT = 'ryzen/float32'


class Denied(Exception):
    pass


def require(condition, reason):
    if not condition:
        raise Denied(reason)


def integer(value):
    require(type(value) is int and value > 0, 'invalid numeric identity')
    return value


def sha(value):
    require(isinstance(value, str) and SHA.fullmatch(value), 'invalid SHA')
    return value


def key(pr, head):
    return f'oms-{SOURCE_ID}-pr-{integer(pr)}-{sha(head)}-float32-v1'


class API:
    def __init__(self, token):
        require(bool(token), 'missing API credential')
        self.token = token

    def call(self, path, method='GET', data=None, raw=False):
        require(path.startswith('repos/') or path == 'user', 'unexpected API path')
        body = None if data is None else json.dumps(data).encode()
        request = urllib.request.Request('https://api.github.com/' + path, data=body,
            method=method, headers={'Authorization': 'Bearer ' + self.token,
                'Accept': 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28',
                'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                content = response.read(8 * 1024 * 1024 + 1)
            require(len(content) <= 8 * 1024 * 1024, 'API response too large')
            return content if raw else (json.loads(content) if content else None)
        except (urllib.error.URLError, ValueError) as error:
            # Do not include token-bearing request objects, headers or bodies.
            raise Denied(f'GitHub API unavailable/invalid ({type(error).__name__})') from None

    def pages(self, path, field=None):
        result = []
        for page in range(1, 101):
            data = self.call(f'{path}{"&" if "?" in path else "?"}per_page=100&page={page}')
            items = data[field] if field else data
            require(isinstance(items, list), 'invalid API page')
            result.extend(items)
            if len(items) < 100:
                return result
        raise Denied('pagination limit; refusing incomplete metadata')

    def file(self, repo, path, ref):
        data = self.call(f'repos/{repo}/contents/{path}?ref={urllib.parse.quote(ref, safe="")}')
        require(data.get('type') == 'file' and data.get('encoding') == 'base64', 'missing file')
        return base64.b64decode(data['content'], validate=False)

    def artifact_json(self, artifact_id):
        # Follow the authenticated API redirect without forwarding credentials
        # to the signed blob URL. No extraction, execution, or unsafe ZIP paths.
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self, req, fp, code, msg, headers, newurl):
                return None
        request = urllib.request.Request(
            f'https://api.github.com/repos/{SOURCE}/actions/artifacts/{integer(artifact_id)}/zip',
            headers={'Authorization': 'Bearer ' + self.token})
        try:
            urllib.request.build_opener(NoRedirect).open(request, timeout=30)
        except urllib.error.HTTPError as response:
            require(response.code == 302, 'artifact redirect unavailable')
            location = response.headers.get('Location', '')
        else:
            raise Denied('missing artifact redirect')
        require(urllib.parse.urlparse(location).scheme == 'https', 'invalid artifact URL')
        with urllib.request.urlopen(location, timeout=30) as response:
            content = response.read(65537)
        require(len(content) <= 65536, 'oversized receipt archive')
        with zipfile.ZipFile(io.BytesIO(content)) as archive:
            files = archive.infolist()
            require(len(files) == 1 and files[0].filename == 'request.json'
                    and files[0].file_size <= 8192, 'invalid receipt archive')
            return json.loads(archive.read(files[0]))


def repository(api, name, expected):
    data = api.call(f'repos/{name}')
    require(data['id'] == expected and data['full_name'] == name
            and data['default_branch'] == 'main', 'repository identity mismatch')
    return data


def workflow(api, name, path, expected_id):
    data = api.call(f'repos/{name}/actions/workflows/{path.rsplit("/", 1)[-1]}')
    require(data['id'] == integer(expected_id) and data['path'] == path
            and data['state'] == 'active', 'workflow identity mismatch')
    return data


def authorize_signal(api, run_id, signal_id, current=True):
    repository(api, SOURCE, SOURCE_ID)
    workflow(api, SOURCE, SIGNAL_PATH, signal_id)
    run = api.call(f'repos/{SOURCE}/actions/runs/{integer(run_id)}')
    require(run['id'] == run_id and run['repository']['id'] == SOURCE_ID
            and run['head_repository']['id'] == SOURCE_ID, 'signal repository mismatch')
    require(run['workflow_id'] == signal_id and run['path'] == SIGNAL_PATH
            and run['event'] == 'pull_request' and run['status'] == 'completed'
            and run['conclusion'] == 'success' and run['run_attempt'] == 1,
            'signal workflow/event/conclusion mismatch')
    require(run['actor']['id'] == OWNER_ID and run['triggering_actor']['id'] == OWNER_ID,
            'signal actor not owner')
    associations = run['pull_requests']
    require(len(associations) == 1, 'ambiguous/missing PR association')
    association = associations[0]
    pr = api.call(f'repos/{SOURCE}/pulls/{integer(association["number"])}')
    require(pr['number'] == association['number'] and pr['id'] == association['id'], 'PR identity mismatch')
    require(pr['user']['id'] == OWNER_ID and pr['head']['repo']['id'] == SOURCE_ID
            and pr['head']['repo']['full_name'] == SOURCE
            and pr['base']['repo']['id'] == SOURCE_ID and pr['base']['ref'] == 'main',
            'PR creator/source/base denied')
    head = sha(association['head']['sha'])
    require(association['head']['repo']['id'] == SOURCE_ID
            and association['base']['repo']['id'] == SOURCE_ID
            and association['base']['ref'] == 'main'
            and association['head']['ref'] == run['head_branch'], 'association mismatch')
    # GitHub run.head_sha normally is the PR head, while check suites can use
    # the synthetic merge SHA. Accept that SHA only with verified parents.
    if run['head_sha'] != head:
        merge = api.call(f'repos/{SOURCE}/commits/{sha(run["head_sha"])}')
        require(merge['sha'] == run['head_sha'] and len(merge['parents']) == 2
                and merge['parents'][1]['sha'] == head
                and merge['parents'][0]['sha'] == association['base']['sha'], 'unverified merge SHA')
    stale = pr['state'] != 'open' or pr['head']['sha'] != head
    if current:
        require(not stale, 'closed or superseded PR')
        age = dt.datetime.now(dt.timezone.utc) - dt.datetime.fromisoformat(run['created_at'].replace('Z', '+00:00'))
        require(0 <= age.total_seconds() <= 86400, 'expired/future signal; replay denied')
        branch = api.call(f'repos/{SOURCE}/git/ref/heads/' + urllib.parse.quote(pr['head']['ref'], safe=''))
        require(branch['object']['sha'] == head, 'source branch advanced')
    commit = api.call(f'repos/{SOURCE}/commits/{head}')
    require(commit['sha'] == head, 'source commit unavailable')
    linked = api.pages(f'repos/{SOURCE}/commits/{head}/pulls')
    require(len(linked) == 1 and linked[0]['id'] == pr['id']
            and linked[0]['number'] == pr['number'], 'ambiguous commit-to-PR association')
    # Signal content cannot be altered in the PR. Read as bytes; never evaluate
    # source YAML. Newly edited signal requires default-branch review first.
    require(api.file(SOURCE, SIGNAL_PATH, head) == api.file(SOURCE, SIGNAL_PATH, 'main'),
            'PR modified signal workflow')
    return {'key': key(pr['number'], head), 'pr': pr['number'], 'sha': head,
            'signal_run': run_id, 'signal_id': signal_id, 'stale': stale}


def verify_dispatch_run(api, run_id, dispatch_id, terminal=False, diagnostic=False):
    workflow(api, SOURCE, DISPATCH_PATH, dispatch_id)
    run = api.call(f'repos/{SOURCE}/actions/runs/{integer(run_id)}')
    ancestry = api.call(f'repos/{SOURCE}/compare/{sha(run["head_sha"])}...main')
    require(run['repository']['id'] == SOURCE_ID and run['head_repository']['id'] == SOURCE_ID
            and run['workflow_id'] == dispatch_id and run['path'] == DISPATCH_PATH
            and run['event'] == 'workflow_run' and run['head_branch'] == 'main'
            and ancestry['status'] in {'ahead', 'identical'} and run['run_attempt'] == 1,
            'untrusted dispatcher provenance/default revision')
    require(run['actor']['id'] == OWNER_ID and run['triggering_actor']['id'] == OWNER_ID,
            'dispatcher actor not owner')
    require((run['status'] == 'completed' and (run['conclusion'] == 'success' or diagnostic))
            or (not terminal and run['status'] == 'in_progress'), 'dispatcher not successful/active')
    return run


def receipt(api, payload, signal_id, dispatch_id, current=True, terminal=False, diagnostic=False):
    require(isinstance(payload, dict) and set(payload) == {'dispatcher_run', 'signal_run', 'key'},
            'malformed dispatch payload')
    integer(payload['dispatcher_run'])
    integer(payload['signal_run'])
    require(isinstance(payload['key'], str) and len(payload['key']) < 160, 'invalid key')
    verify_dispatch_run(api, payload['dispatcher_run'], dispatch_id, terminal, diagnostic)
    auth = authorize_signal(api, payload['signal_run'], signal_id, current)
    require(payload['key'] == auth['key'], 'payload key changed')
    artifacts = api.pages(f'repos/{SOURCE}/actions/runs/{payload["dispatcher_run"]}/artifacts', 'artifacts')
    matches = [a for a in artifacts if a['name'] == auth['key'] and not a['expired']]
    require(len(matches) == 1, 'missing/ambiguous dispatch receipt')
    data = api.artifact_json(matches[0]['id'])
    expected = {k: v for k, v in auth.items() if k != 'stale'}
    expected['dispatcher_run'] = payload['dispatcher_run']
    require(data == expected, 'receipt identity mismatch')
    return auth


def publish(api, auth, state, url, description):
    require(state in {'pending', 'success', 'failure', 'error'}, 'invalid status')
    require(url.startswith('https://github.com/' + DEST + '/actions/runs/'), 'invalid result URL')
    api.call(f'repos/{SOURCE}/statuses/{sha(auth["sha"])}', 'POST', {
        'state': state, 'context': CONTEXT, 'target_url': url, 'description': description[:140]})


def digest(data):
    return hashlib.sha256(data).hexdigest()