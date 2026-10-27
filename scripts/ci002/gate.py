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
RECEIPT_VERSION = 2


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

    def workflow_file(self, run):
        # WorkflowRun.file is GitHub's executed workflow-file record. REST
        # head_sha and a *current* refs/pull/N/merge are not this evidence.
        query = '''query($suite: ID!) {
          node(id: $suite) { ... on CheckSuite { workflowRun {
            databaseId runAttempt event
            workflow { databaseId }
            file { path repositoryName repositoryFileUrl }
          } } }
        }'''
        request = urllib.request.Request('https://api.github.com/graphql',
            data=json.dumps({'query': query, 'variables': {'suite': run['check_suite_node_id']}}).encode(),
            headers={'Authorization': 'Bearer ' + self.token, 'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                content = response.read(65537)
            require(len(content) <= 65536, 'oversized workflow provenance response')
            data = json.loads(content)
            require(not data.get('errors'), 'workflow provenance API error')
            record = data['data']['node']['workflowRun']
            require(record['databaseId'] == run['id'] and record['runAttempt'] == run['run_attempt']
                    and record['event'] == run['event']
                    and record['workflow']['databaseId'] == run['workflow_id'],
                    'workflow provenance run mismatch')
            return record['file']
        except (urllib.error.URLError, ValueError, KeyError, TypeError):
            raise Denied('executed workflow provenance unavailable') from None

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
    if name == DEST or expected == DEST_ID:
        require(name == DEST and expected == DEST_ID and data.get('private') is True
                and data.get('visibility') == 'private', 'execution repository must remain private')
    return data


def workflow(api, name, path, expected_id):
    data = api.call(f'repos/{name}/actions/workflows/{path.rsplit("/", 1)[-1]}')
    require(data['id'] == integer(expected_id) and data['path'] == path
            and data['state'] == 'active', 'workflow identity mismatch')
    return data


def executed_workflow(api, run, path):
    record = api.workflow_file(run)
    require(isinstance(record, dict), 'executed workflow record unavailable')
    require(record['repositoryName'] == SOURCE and record['path'] == path,
            'executed workflow repository/path mismatch')
    prefix = f'https://github.com/{SOURCE}/blob/'
    url = record['repositoryFileUrl']
    require(isinstance(url, str) and url.startswith(prefix), 'invalid executed workflow URL')
    revision, separator, actual_path = url[len(prefix):].partition('/')
    require(separator and actual_path == path, 'executed workflow path mismatch')
    return sha(revision)


def workflow_identity(content):
    require(isinstance(content, bytes) and 0 < len(content) <= 65536, 'missing/oversized workflow file')
    blob = b'blob ' + str(len(content)).encode() + b'\0' + content
    return {'blob_sha': hashlib.sha1(blob).hexdigest(), 'sha256': digest(content)}


def signal_provenance(api, run, association, trusted_revision):
    # The baseline comes only from the independently verified default-branch
    # dispatcher run, never the payload or mutable main's workflow bytes.
    baseline = sha(trusted_revision)
    head, base = sha(association['head']['sha']), sha(association['base']['sha'])
    revision = executed_workflow(api, run, SIGNAL_PATH)
    merge = None
    if revision != head:
        commit = api.call(f'repos/{SOURCE}/commits/{revision}')
        require(commit['sha'] == revision and [p['sha'] for p in commit['parents']] == [base, head],
                'unverified effective merge parents')
        merge = revision
    # No absent-HEAD exception: an older branch is allowed only if GitHub's
    # executed-file record identifies a verified merge with trusted bytes.
    actual = api.file(SOURCE, SIGNAL_PATH, revision)
    trusted = api.file(SOURCE, SIGNAL_PATH, baseline)
    require(actual == trusted, 'PR modified effective signal workflow')
    return {'workflow_sha': revision, 'baseline_sha': baseline, 'base_sha': base,
            'head_sha': head, 'merge_sha': merge, **workflow_identity(actual)}


def authorize_signal(api, run_id, signal_id, current=True, *, trusted_revision):
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
    provenance = signal_provenance(api, run, association, trusted_revision)
    return {'key': key(pr['number'], head), 'pr': pr['number'], 'sha': head,
            'receipt_version': RECEIPT_VERSION, 'source_repository_id': SOURCE_ID,
            'source_repository': SOURCE, 'pr_id': pr['id'], 'signal_path': SIGNAL_PATH,
            'signal_run': run_id, 'signal_attempt': run['run_attempt'], 'signal_id': signal_id,
            'signal_provenance': provenance, 'stale': stale}


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
    require(run['id'] == run_id and executed_workflow(api, run, DISPATCH_PATH) == run['head_sha'],
            'dispatcher workflow is not its trusted default revision')
    require((run['status'] == 'completed' and (run['conclusion'] == 'success' or diagnostic))
            or (not terminal and run['status'] == 'in_progress'), 'dispatcher not successful/active')
    return run


def dispatch_identity(api, run):
    return {'dispatcher_run': run['id'], 'dispatcher_attempt': run['run_attempt'],
            'dispatcher_id': run['workflow_id'], 'dispatcher_path': DISPATCH_PATH,
            'dispatcher_sha': run['head_sha'],
            'dispatcher_workflow': workflow_identity(api.file(SOURCE, DISPATCH_PATH, run['head_sha']))}


def receipt(api, payload, signal_id, dispatch_id, current=True, terminal=False, diagnostic=False):
    require(isinstance(payload, dict) and set(payload) == {'dispatcher_run', 'signal_run', 'key'},
            'malformed dispatch payload')
    integer(payload['dispatcher_run'])
    integer(payload['signal_run'])
    require(isinstance(payload['key'], str) and len(payload['key']) < 160, 'invalid key')
    dispatch = verify_dispatch_run(api, payload['dispatcher_run'], dispatch_id, terminal, diagnostic)
    auth = authorize_signal(api, payload['signal_run'], signal_id, current,
                            trusted_revision=dispatch['head_sha'])
    auth.update(dispatch_identity(api, dispatch))
    require(payload['key'] == auth['key'], 'payload key changed')
    artifacts = api.pages(f'repos/{SOURCE}/actions/runs/{payload["dispatcher_run"]}/artifacts', 'artifacts')
    matches = [a for a in artifacts if a['name'] == auth['key'] and not a['expired']]
    require(len(matches) == 1, 'missing/ambiguous dispatch receipt')
    data = api.artifact_json(matches[0]['id'])
    expected = {k: v for k, v in auth.items() if k != 'stale'}
    require(data == expected, 'receipt identity mismatch')
    return auth


def publish(api, auth, state, url, description):
    require(state in {'pending', 'success', 'failure', 'error'}, 'invalid status')
    require(auth['receipt_version'] == RECEIPT_VERSION
            and auth['sha'] == auth['signal_provenance']['head_sha']
            and auth['key'] == key(auth['pr'], auth['sha']), 'status SHA differs from authorization')
    require(url.startswith('https://github.com/' + DEST + '/actions/runs/'), 'invalid result URL')
    api.call(f'repos/{SOURCE}/statuses/{sha(auth["sha"])}', 'POST', {
        'state': state, 'context': CONTEXT, 'target_url': url, 'description': description[:140]})


def digest(data):
    return hashlib.sha256(data).hexdigest()