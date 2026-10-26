"""Exercise the production gate with API-shaped fixtures and mutations."""
import copy
import datetime as dt
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import gate as g

HEAD = 'a' * 40
BASE = 'b' * 40
MAIN = 'c' * 40
SIGNAL = 11
DISPATCH = 12
RUN = 101
DRUN = 102


class FixtureAPI:
    def __init__(self):
        self.posts = []
        self.files = {}
        self.workflow_files = {}
        self.data = {
            f'repos/{g.SOURCE}': {'id': g.SOURCE_ID, 'full_name': g.SOURCE, 'default_branch': 'main'},
            f'repos/{g.DEST}': {'id': g.DEST_ID, 'full_name': g.DEST, 'default_branch': 'main',
                              'private': True, 'visibility': 'private'},
            f'repos/{g.SOURCE}/actions/workflows/oms-trusted-pr-signal.yml': {
                'id': SIGNAL, 'path': g.SIGNAL_PATH, 'state': 'active'},
            f'repos/{g.SOURCE}/actions/workflows/oms-ryzen-trusted-dispatch.yml': {
                'id': DISPATCH, 'path': g.DISPATCH_PATH, 'state': 'active'},
            f'repos/{g.SOURCE}/actions/runs/{RUN}': {
                'id': RUN, 'repository': {'id': g.SOURCE_ID}, 'head_repository': {'id': g.SOURCE_ID},
                'workflow_id': SIGNAL, 'path': g.SIGNAL_PATH, 'event': 'pull_request',
                'status': 'completed', 'conclusion': 'success', 'run_attempt': 1,
                'actor': {'id': g.OWNER_ID, 'login': 'display-name-not-authority'},
                'triggering_actor': {'id': g.OWNER_ID}, 'head_sha': HEAD, 'head_branch': 'feature',
                'created_at': dt.datetime.now(dt.timezone.utc).isoformat(),
                'pull_requests': [{'id': 20, 'number': 7, 'head': {
                    'sha': HEAD, 'ref': 'feature', 'repo': {'id': g.SOURCE_ID}},
                    'base': {'sha': BASE, 'ref': 'main', 'repo': {'id': g.SOURCE_ID}}}]},
            f'repos/{g.SOURCE}/pulls/7': {
                'id': 20, 'number': 7, 'state': 'open', 'user': {'id': g.OWNER_ID, 'login': 'zackboll'},
                'head': {'sha': HEAD, 'ref': 'feature', 'repo': {'id': g.SOURCE_ID, 'full_name': g.SOURCE}},
                'base': {'ref': 'main', 'repo': {'id': g.SOURCE_ID}}},
            f'repos/{g.SOURCE}/git/ref/heads/feature': {'object': {'sha': HEAD}},
            f'repos/{g.SOURCE}/commits/{HEAD}': {'sha': HEAD},
            f'repos/{g.SOURCE}/compare/{MAIN}...main': {'status': 'identical'},
            f'repos/{g.SOURCE}/actions/runs/{DRUN}': {
                'id': DRUN,
                'repository': {'id': g.SOURCE_ID}, 'head_repository': {'id': g.SOURCE_ID},
                'workflow_id': DISPATCH, 'path': g.DISPATCH_PATH, 'event': 'workflow_run',
                'head_branch': 'main', 'head_sha': MAIN, 'run_attempt': 1,
                'status': 'completed', 'conclusion': 'success',
                'actor': {'id': g.OWNER_ID}, 'triggering_actor': {'id': g.OWNER_ID}},
        }
        self.run = self.data[f'repos/{g.SOURCE}/actions/runs/{RUN}']
        self.pr = self.data[f'repos/{g.SOURCE}/pulls/7']
        self.dispatch = self.data[f'repos/{g.SOURCE}/actions/runs/{DRUN}']
        self.artifacts = [{'name': g.key(7, HEAD), 'id': 30, 'expired': False}]
        self.receipt = g.authorize_signal(self, RUN, SIGNAL, trusted_revision=MAIN)
        self.receipt.pop('stale')
        self.receipt.update(g.dispatch_identity(self, self.dispatch))

    def call(self, path, method='GET', data=None):
        if method == 'POST':
            self.posts.append((path, data))
            return None
        if path not in self.data:
            raise g.Denied('missing fixture API data')
        return copy.deepcopy(self.data[path])

    def file(self, repo, path, ref):
        content = self.files.get((repo, path, ref), b'same trusted signal bytes')
        if content is None:
            raise g.Denied('workflow file unavailable')
        return content

    def workflow_file(self, run):
        return self.workflow_files.get(run['id'], {
            'repositoryName': g.SOURCE, 'path': run['path'],
            'repositoryFileUrl': f'https://github.com/{g.SOURCE}/blob/{run["head_sha"]}/{run["path"]}'})

    def pages(self, path, field=None):
        if path.endswith('/statuses'):
            return []
        if path.endswith('/pulls'):
            return [{'id': self.pr['id'], 'number': self.pr['number']}]
        return copy.deepcopy(self.artifacts)

    def artifact_json(self, artifact_id):
        return copy.deepcopy(self.receipt)


class Authorization(unittest.TestCase):
    def setUp(self):
        self.api = FixtureAPI()

    def test_owner_same_repo_authorized_immutable_identity(self):
        auth = g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)
        self.assertEqual(auth['sha'], HEAD)
        self.assertEqual(auth['key'], g.key(7, HEAD))
        self.assertEqual(auth['pr'], 7)

    def test_valid_next_head_fresh_identity(self):
        new = 'd' * 40
        self.api.run['head_sha'] = new
        self.api.run['pull_requests'][0]['head']['sha'] = new
        self.api.pr['head']['sha'] = new
        self.api.data[f'repos/{g.SOURCE}/git/ref/heads/feature']['object']['sha'] = new
        self.api.data[f'repos/{g.SOURCE}/commits/{new}'] = {'sha': new}
        self.assertEqual(g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)['sha'], new)
        self.assertNotEqual(g.key(7, new), g.key(7, HEAD))

    def test_fork_owner_fork_and_external_creator_rejected(self):
        for owner, repo in [(9, 99), (g.OWNER_ID, 99), (9, g.SOURCE_ID)]:
            with self.subTest(owner=owner, repo=repo):
                api = FixtureAPI()
                api.pr['user']['id'] = owner
                api.pr['head']['repo']['id'] = repo
                api.pr['user']['login'] = 'zackboll'
                with self.assertRaises(g.Denied):
                    g.authorize_signal(api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_api_mutations_fail_closed(self):
        mutations = [
            ('repo', lambda a: a.data[f'repos/{g.SOURCE}'].update(id=99)),
            ('creator', lambda a: a.pr['user'].update(id=9)),
            ('closed', lambda a: a.pr.update(state='closed')),
            ('base', lambda a: a.pr['base'].update(ref='release')),
            ('stale', lambda a: a.pr['head'].update(sha='d' * 40)),
            ('workflow', lambda a: a.run.update(workflow_id=999)),
            ('path', lambda a: a.run.update(path='evil.yml')),
            ('event', lambda a: a.run.update(event='pull_request_target')),
            ('failure', lambda a: a.run.update(conclusion='failure')),
            ('incomplete', lambda a: a.run.update(status='in_progress')),
            ('actor', lambda a: a.run['actor'].update(id=9)),
            ('trigger', lambda a: a.run['triggering_actor'].update(id=9)),
            ('rerun', lambda a: a.run.update(run_attempt=2)),
            ('missingPR', lambda a: a.run.update(pull_requests=[])),
            ('ambiguousPR', lambda a: a.run['pull_requests'].append(copy.deepcopy(a.run['pull_requests'][0]))),
            ('badsha', lambda a: a.run['pull_requests'][0]['head'].update(sha='$(evil)')),
            ('association', lambda a: a.run['pull_requests'][0]['head']['repo'].update(id=99)),
            ('missingfield', lambda a: a.run.pop('triggering_actor')),
            ('expired', lambda a: a.run.update(created_at='2020-01-01T00:00:00Z')),
            ('branchadvanced', lambda a: a.data[f'repos/{g.SOURCE}/git/ref/heads/feature']['object'].update(sha=BASE)),
        ]
        for name, mutate in mutations:
            api = FixtureAPI()
            mutate(api)
            with self.subTest(name=name), self.assertRaises((g.Denied, KeyError)):
                g.authorize_signal(api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_unavailable_api_rejected(self):
        self.api.data.clear()
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_pr_modifies_signal_rejected(self):
        self.api.files[(g.SOURCE, g.SIGNAL_PATH, HEAD)] = b'evil signal'
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_same_commit_multiple_prs_rejected(self):
        actual = self.api.pages
        self.api.pages = lambda path, field=None: ([{'id': 20, 'number': 7}, {'id': 21, 'number': 8}]
                                                   if path.endswith('/pulls') else actual(path, field))
        with self.assertRaisesRegex(g.Denied, 'ambiguous commit-to-PR'):
            g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_verified_synthetic_merge_and_invalid_parents(self):
        merge = 'e' * 40
        self.api.run['head_sha'] = merge
        self.api.data[f'repos/{g.SOURCE}/commits/{merge}'] = {
            'sha': merge, 'parents': [{'sha': BASE}, {'sha': HEAD}]}
        self.assertEqual(g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)['sha'], HEAD)
        self.api.data[f'repos/{g.SOURCE}/commits/{merge}']['parents'].reverse()
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_numeric_and_sha_malformed(self):
        for value in [True, '7', -1, 0, None]:
            with self.subTest(value=value), self.assertRaises(g.Denied):
                g.integer(value)
        for value in ['', 'A' * 40, '../main', 'a' * 39, None]:
            with self.subTest(value=value), self.assertRaises(g.Denied):
                g.sha(value)

    def test_dispatch_receipt_bound_to_authoritative_source(self):
        request = {'key': g.key(7, HEAD), 'dispatcher_run': DRUN, 'signal_run': RUN}
        self.assertEqual(g.receipt(self.api, request, SIGNAL, DISPATCH)['sha'], HEAD)
        for changed in [{**request, 'sha': HEAD}, {**request, 'key': 'evil'},
                        {**request, 'dispatcher_run': '102'}, {},
                        {**request, 'signal_run': 999}]:
            with self.subTest(payload=changed), self.assertRaises((g.Denied, KeyError)):
                g.receipt(self.api, changed, SIGNAL, DISPATCH)

    def test_modified_receipt_and_missing_artifact_rejected(self):
        request = {'key': g.key(7, HEAD), 'dispatcher_run': DRUN, 'signal_run': RUN}
        self.api.receipt['sha'] = BASE
        with self.assertRaises(g.Denied):
            g.receipt(self.api, request, SIGNAL, DISPATCH)
        self.api.artifacts = []
        with self.assertRaises(g.Denied):
            g.receipt(self.api, request, SIGNAL, DISPATCH)

    def test_dispatch_provenance_mutations(self):
        for field, value in [('head_branch', 'feature'), ('event', 'push'), ('workflow_id', 999),
                             ('path', 'evil.yml'), ('conclusion', 'failure'), ('run_attempt', 2)]:
            api = FixtureAPI()
            api.dispatch[field] = value
            with self.subTest(field=field), self.assertRaises(g.Denied):
                g.verify_dispatch_run(api, DRUN, DISPATCH, terminal=True)
        self.api.data[f'repos/{g.SOURCE}/compare/{MAIN}...main'] = {'status': 'diverged'}
        with self.assertRaises(g.Denied):
            g.verify_dispatch_run(self.api, DRUN, DISPATCH)

    def test_fixture_status_api_is_exact_context_and_sha(self):
        auth = g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)
        for state in ['pending', 'success', 'failure', 'error']:
            g.publish(self.api, auth, state, f'https://github.com/{g.DEST}/actions/runs/1', 'fixture')
        self.assertEqual(len(self.api.posts), 4)
        self.assertTrue(all(p[0].endswith('/statuses/' + HEAD) and p[1]['context'] == 'ryzen/float32'
                            for p in self.api.posts))
        with self.assertRaises(g.Denied):
            g.publish(self.api, auth, 'success', 'https://evil.test', 'spoof')

    def test_older_head_requires_authenticated_effective_merge(self):
        merge = 'e' * 40
        self.api.files[(g.SOURCE, g.SIGNAL_PATH, HEAD)] = None
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)
        self.api.workflow_files[RUN] = {
            'repositoryName': g.SOURCE, 'path': g.SIGNAL_PATH,
            'repositoryFileUrl': f'https://github.com/{g.SOURCE}/blob/{merge}/{g.SIGNAL_PATH}'}
        self.api.data[f'repos/{g.SOURCE}/commits/{merge}'] = {
            'sha': merge, 'parents': [{'sha': BASE}, {'sha': HEAD}]}
        # REST still reports the PR HEAD, matching the observed GitHub run.
        auth = g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)
        self.assertEqual(self.api.run['head_sha'], HEAD)
        self.assertEqual(auth['signal_provenance']['merge_sha'], merge)
        for content in (None, b'run: malicious executable code'):
            self.api.files[(g.SOURCE, g.SIGNAL_PATH, merge)] = content
            with self.subTest(content=content), self.assertRaises(g.Denied):
                g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)
        self.api.files.pop((g.SOURCE, g.SIGNAL_PATH, merge))
        self.api.data[f'repos/{g.SOURCE}/commits/{merge}']['parents'].reverse()
        with self.assertRaisesRegex(g.Denied, 'effective merge parents'):
            g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_effective_workflow_provenance_mutations(self):
        valid = {'repositoryName': g.SOURCE, 'path': g.SIGNAL_PATH,
                 'repositoryFileUrl': f'https://github.com/{g.SOURCE}/blob/{HEAD}/{g.SIGNAL_PATH}'}
        for field, value in [('repositoryName', 'attacker/repo'), ('path', 'evil.yml'),
                             ('repositoryFileUrl', f'https://github.com/{g.SOURCE}/blob/main/{g.SIGNAL_PATH}'),
                             ('repositoryFileUrl', f'https://github.com/{g.SOURCE}/blob/{BASE}/{g.SIGNAL_PATH}'),
                             ('repositoryFileUrl', f'https://evil.test/{HEAD}/{g.SIGNAL_PATH}')]:
            self.api.workflow_files[RUN] = {**valid, field: value}
            with self.subTest(field=field, value=value), self.assertRaises(g.Denied):
                g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)

    def test_historical_receipt_ignores_new_main_workflow_bytes(self):
        request = {'key': g.key(7, HEAD), 'dispatcher_run': DRUN, 'signal_run': RUN}
        self.api.files[(g.SOURCE, g.SIGNAL_PATH, 'main')] = b'new trusted signal'
        self.api.files[(g.SOURCE, g.DISPATCH_PATH, 'main')] = b'new trusted dispatcher'
        self.api.data[f'repos/{g.SOURCE}/compare/{MAIN}...main']['status'] = 'ahead'
        auth = g.receipt(self.api, request, SIGNAL, DISPATCH)
        self.assertEqual(auth['signal_provenance']['baseline_sha'], MAIN)
        self.assertEqual(auth['receipt_version'], 2)
        self.api.files[(g.SOURCE, g.SIGNAL_PATH, MAIN)] = None
        with self.assertRaisesRegex(g.Denied, 'unavailable'):
            g.receipt(self.api, request, SIGNAL, DISPATCH)

    def test_receipt_provenance_fields_cannot_be_forged(self):
        request = {'key': g.key(7, HEAD), 'dispatcher_run': DRUN, 'signal_run': RUN}
        for field, value in [('receipt_version', 1), ('source_repository_id', 99),
                             ('signal_attempt', 2), ('dispatcher_sha', BASE),
                             ('signal_provenance', {}), ('dispatcher_workflow', {})]:
            api = FixtureAPI()
            api.receipt[field] = value
            with self.subTest(field=field), self.assertRaises(g.Denied):
                g.receipt(api, request, SIGNAL, DISPATCH)

    def test_private_execution_repository_visibility_fails_closed(self):
        g.repository(self.api, g.DEST, g.DEST_ID)
        for field, value in [('private', False), ('private', 'true'), ('private', None),
                             ('visibility', 'public'), ('visibility', 'internal'), ('visibility', None),
                             ('id', 99), ('full_name', 'attacker/repo'), ('default_branch', 'other')]:
            api = FixtureAPI()
            api.data[f'repos/{g.DEST}'][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(g.Denied):
                g.repository(api, g.DEST, g.DEST_ID)
        for field in ('private', 'visibility'):
            api = FixtureAPI()
            del api.data[f'repos/{g.DEST}'][field]
            with self.subTest(missing=field), self.assertRaises(g.Denied):
                g.repository(api, g.DEST, g.DEST_ID)

    def test_status_wrong_sha_rejected_before_post(self):
        auth = g.authorize_signal(self.api, RUN, SIGNAL, trusted_revision=MAIN)
        auth['sha'] = BASE
        with self.assertRaisesRegex(g.Denied, 'status SHA'):
            g.publish(self.api, auth, 'success', f'https://github.com/{g.DEST}/actions/runs/1', 'wrong SHA')
        self.assertEqual(self.api.posts, [])

    def test_dispatcher_cannot_use_pr_workflow_file(self):
        self.api.workflow_files[DRUN] = {
            'repositoryName': g.SOURCE, 'path': g.DISPATCH_PATH,
            'repositoryFileUrl': f'https://github.com/{g.SOURCE}/blob/{HEAD}/{g.DISPATCH_PATH}'}
        with self.assertRaisesRegex(g.Denied, 'trusted default revision'):
            g.verify_dispatch_run(self.api, DRUN, DISPATCH)

    def test_observed_github_signal_file_record(self):
        # Authenticated REST + GraphQL inspection of successful run 40108707517.
        # This is NOT a claim that the observed run used a synthetic merge.
        run = {'id': 40108707517, 'path': g.SIGNAL_PATH,
               'head_sha': '4c79fff41970a8ba7109df9bd185a6b166c0ffb0'}
        self.api.workflow_files[run['id']] = {
            'repositoryName': g.SOURCE, 'path': g.SIGNAL_PATH,
            'repositoryFileUrl': f'https://github.com/{g.SOURCE}/blob/{run["head_sha"]}/{g.SIGNAL_PATH}'}
        self.assertEqual(g.executed_workflow(self.api, run, g.SIGNAL_PATH), run['head_sha'])

    def test_graphql_workflow_file_bound_to_authenticated_run(self):
        run = {**self.api.run, 'check_suite_node_id': 'github-check-suite'}
        record = {'databaseId': RUN, 'runAttempt': 1, 'event': 'pull_request',
                  'workflow': {'databaseId': SIGNAL}, 'file': self.api.workflow_file(run)}
        for field, value in [(None, None), ('databaseId', 999), ('runAttempt', 2),
                             ('event', 'push'), ('workflow', {'databaseId': 99}), ('file', None)]:
            changed = copy.deepcopy(record)
            if field:
                changed[field] = value
            response = unittest.mock.MagicMock()
            response.__enter__.return_value.read.return_value = json.dumps(
                {'data': {'node': {'workflowRun': changed}}}).encode()
            with self.subTest(field=field), patch.object(g.urllib.request, 'urlopen', return_value=response):
                if field is None:
                    self.assertEqual(g.API('fixture').workflow_file(run), record['file'])
                elif field == 'file':
                    with self.assertRaises(g.Denied):
                        g.executed_workflow(g.API('fixture'), run, g.SIGNAL_PATH)
                else:
                    with self.assertRaises(g.Denied):
                        g.API('fixture').workflow_file(run)


if __name__ == '__main__':
    unittest.main()