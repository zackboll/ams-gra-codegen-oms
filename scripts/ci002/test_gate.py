"""Exercise the production gate with API-shaped fixtures and mutations."""
import copy
import datetime as dt
import json
from pathlib import Path
import unittest

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
        self.data = {
            f'repos/{g.SOURCE}': {'id': g.SOURCE_ID, 'full_name': g.SOURCE, 'default_branch': 'main'},
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
        self.receipt = {'key': g.key(7, HEAD), 'pr': 7, 'sha': HEAD,
                        'signal_run': RUN, 'signal_id': SIGNAL, 'dispatcher_run': DRUN}

    def call(self, path, method='GET', data=None):
        if method == 'POST':
            self.posts.append((path, data))
            return None
        if path not in self.data:
            raise g.Denied('missing fixture API data')
        return copy.deepcopy(self.data[path])

    def file(self, repo, path, ref):
        return self.files.get((repo, path, ref), b'same trusted signal bytes')

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
        auth = g.authorize_signal(self.api, RUN, SIGNAL)
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
        self.assertEqual(g.authorize_signal(self.api, RUN, SIGNAL)['sha'], new)
        self.assertNotEqual(g.key(7, new), g.key(7, HEAD))

    def test_fork_owner_fork_and_external_creator_rejected(self):
        for owner, repo in [(9, 99), (g.OWNER_ID, 99), (9, g.SOURCE_ID)]:
            with self.subTest(owner=owner, repo=repo):
                api = FixtureAPI()
                api.pr['user']['id'] = owner
                api.pr['head']['repo']['id'] = repo
                api.pr['user']['login'] = 'zackboll'
                with self.assertRaises(g.Denied):
                    g.authorize_signal(api, RUN, SIGNAL)

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
                g.authorize_signal(api, RUN, SIGNAL)

    def test_unavailable_api_rejected(self):
        self.api.data.clear()
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL)

    def test_pr_modifies_signal_rejected(self):
        self.api.files[(g.SOURCE, g.SIGNAL_PATH, HEAD)] = b'evil signal'
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL)

    def test_same_commit_multiple_prs_rejected(self):
        actual = self.api.pages
        self.api.pages = lambda path, field=None: ([{'id': 20, 'number': 7}, {'id': 21, 'number': 8}]
                                                   if path.endswith('/pulls') else actual(path, field))
        with self.assertRaisesRegex(g.Denied, 'ambiguous commit-to-PR'):
            g.authorize_signal(self.api, RUN, SIGNAL)

    def test_verified_synthetic_merge_and_invalid_parents(self):
        merge = 'e' * 40
        self.api.run['head_sha'] = merge
        self.api.data[f'repos/{g.SOURCE}/commits/{merge}'] = {
            'sha': merge, 'parents': [{'sha': BASE}, {'sha': HEAD}]}
        self.assertEqual(g.authorize_signal(self.api, RUN, SIGNAL)['sha'], HEAD)
        self.api.data[f'repos/{g.SOURCE}/commits/{merge}']['parents'].reverse()
        with self.assertRaises(g.Denied):
            g.authorize_signal(self.api, RUN, SIGNAL)

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
        auth = g.authorize_signal(self.api, RUN, SIGNAL)
        for state in ['pending', 'success', 'failure', 'error']:
            g.publish(self.api, auth, state, f'https://github.com/{g.DEST}/actions/runs/1', 'fixture')
        self.assertEqual(len(self.api.posts), 4)
        self.assertTrue(all(p[0].endswith('/statuses/' + HEAD) and p[1]['context'] == 'ryzen/float32'
                            for p in self.api.posts))
        with self.assertRaises(g.Denied):
            g.publish(self.api, auth, 'success', 'https://evil.test', 'spoof')


if __name__ == '__main__':
    unittest.main()