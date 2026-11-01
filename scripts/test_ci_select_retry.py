"""CI-RETRY-001 fail-closed historical producer reconciliation controls."""
import copy
import hashlib
import io
import importlib.util
import json
import unittest
import zipfile
from unittest.mock import patch

from ci_artifact_provenance import resolve, verify_final_job
from ci_retry_test_data import metadata, job_name
from test_ci_select import gate
from test_ci_select_task100 import Task100Evidence, bundle as task100_bundle
from test_real_uci_evidence import bundle as shard_bundle, change_json
import ci_selection


class HistoricalReplaySelection(unittest.TestCase):
    def test_only_complete_historical_v1_can_be_projected(self):
        spec = importlib.util.spec_from_file_location('retry_replay',
            ci_selection.ROOT / 'scripts/replay-ci-retry-001.py')
        replay = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(replay)
        policy = ci_selection.inventory()
        manifest = ci_selection.select([], policy, full_reason='historical fixture')
        for field in ('deep_jobs', 'deep_selected_cells', 'deep_skipped_cells'):
            manifest.pop(field)
        manifest['version'] = 1
        original = copy.deepcopy(manifest)
        result = replay.replay_selection(manifest, policy)
        ci_selection.validate_deep(result, policy)
        self.assertEqual(result['deep_selected_cells'], 24)
        self.assertEqual(result['deep_skipped_cells'], 0)
        self.assertEqual(manifest, original)
        for mutation in (dict(version=2), dict(mode='selective-pr'), dict(deep=False),
                         dict(baseline=False), dict(jobs={}), dict(cells=[]),
                         dict(selected_cells=19), dict(skipped_cells=1)):
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                replay.replay_selection(dict(manifest, **mutation), policy)


NAMES = [f'task100-ada-websocket-{kind}-{compiler}' for kind in ('transport', 'sleet')
         for compiler in ('14', '16')] + ['perf007-' + shard for shard in gate.SHARDS]


class RetryEvidence(unittest.TestCase):
    def setUp(self):
        Task100Evidence.setUp(self)  # Fixture-only license digests; no live evidence.

    def fixture(self, current, producers):
        artifacts = [dict(name=name) for name in NAMES]
        jobs, attempts = metadata(artifacts, current, producers)
        files = {}
        for artifact in artifacts:
            name = artifact['name']
            if name.startswith('perf007-'):
                data = shard_bundle(name.removeprefix('perf007-'))
                change_json(data, 'identity.json', attempt=str(producers[name]))
            else:
                data = task100_bundle(name[-2:], '-sleet-' in name)
                identity = json.loads(data['integration-identity.json'])
                identity['run_attempt'] = str(producers[name])
                data['integration-identity.json'] = json.dumps(identity).encode()
            files[name] = data
        return artifacts, jobs, attempts, files

    def reconcile(self, fixture, current=2):
        artifacts, jobs, attempts, files = fixture
        loader = lambda a: files[a['name']]
        task100 = gate.reconcile_task100(artifacts, dict(jobs={'ada-websocket-transport': True}, deep=True,
                                        deep_jobs=['ada-websocket-real-sleet']),
                                        'a'*40, 'b'*40, '123', str(current), loader,
                                        jobs=jobs, attempts=attempts)
        primary = gate.reconcile_primary(artifacts, 'a'*40, 'b'*40, '123', str(current), loader,
                                        jobs=jobs, attempts=attempts)
        return {**task100, **primary}

    def test_normal_failed_only_full_and_mixed_reruns(self):
        cases = [(1, {n: 1 for n in NAMES}), (2, {n: 1 for n in NAMES}),
                 (2, {n: 2 for n in NAMES}),
                 (3, {n: i % 3 + 1 for i, n in enumerate(NAMES)})]
        for current, producers in cases:
            with self.subTest(current=current, producers=producers):
                result = self.reconcile(self.fixture(current, producers), current)
                self.assertEqual(len(result), 7)
                self.assertCountEqual([r['producer']['run_attempt'] for r in result.values()],
                                      [str(a) for a in producers.values()])

    def test_wrong_embedded_attempt_never_selects_producer(self):
        for name in NAMES:
            fixture = self.fixture(2, {n: 1 for n in NAMES})
            data = fixture[3][name]
            if name.startswith('perf007-'):
                change_json(data, 'identity.json', attempt='2')
            else:
                identity = json.loads(data['integration-identity.json'])
                identity['run_attempt'] = '2'
                data['integration-identity.json'] = json.dumps(identity).encode()
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.reconcile(fixture)

    def test_missing_corrupt_cross_source_compiler_shard_incomplete_bundles(self):
        for name in NAMES:
            for mutation in ('missing-artifact', 'duplicate-artifact', 'missing-bundle', 'corrupt', 'wrong-sha',
                             'wrong-tree', 'wrong-run', 'wrong-variant', 'missing-result'):
                fixture = self.fixture(2, {n: 1 for n in NAMES})
                artifacts, jobs, attempts, files = fixture
                artifact = next(a for a in artifacts if a['name'] == name)
                data = files[name]
                path = 'identity.json' if name.startswith('perf007-') else 'integration-identity.json'
                if mutation == 'missing-artifact': artifacts.remove(artifact)
                elif mutation == 'duplicate-artifact': artifacts.append(copy.deepcopy(artifact))
                elif mutation == 'missing-bundle': data.clear()
                elif mutation == 'corrupt': data[path] = b'not JSON'
                elif mutation == 'missing-result':
                    del data['completion.json' if name.startswith('perf007-') else 'campaign/result.json']
                else:
                    identity = json.loads(data[path])
                    key = {'wrong-sha': 'sha', 'wrong-tree': 'tree', 'wrong-run': 'run_id',
                           'wrong-variant': 'shard' if name.startswith('perf007-') else 'compiler'}[mutation]
                    identity[key] = 'wrong'
                    data[path] = json.dumps(identity).encode()
                with self.subTest(name=name, mutation=mutation), self.assertRaises((ValueError, KeyError)):
                    self.reconcile(fixture)

    def test_missing_and_altered_authoritative_metadata_rejected(self):
        mutations = [
            lambda a,j,h: a.update(expired=True),
            lambda a,j,h: a.update(size_in_bytes=0),
            lambda a,j,h: a.update(digest=None),
            lambda a,j,h: a['workflow_run'].update(id=999),
            lambda a,j,h: a['workflow_run'].update(head_sha='x'*40),
            lambda a,j,h: a['workflow_run'].update(repository_id=99),
            lambda a,j,h: a.update(created_at='2026-10-27T01:00:00Z'),
            lambda a,j,h: a.update(created_at='2026-10-27T01:09:30Z'),
            lambda a,j,h: a.update(created_at='2026-10-27T02:08:30Z'),
            lambda a,j,h: j.update(conclusion='failure'),
            lambda a,j,h: j.update(run_attempt=1),
            lambda a,j,h: j['steps'][0].update(conclusion='skipped'),
            lambda a,j,h: h.pop(1),
            lambda a,j,h: h[1]['jobs'].clear(),
            lambda a,j,h: h[1]['jobs'].append(copy.deepcopy(h[1]['jobs'][0])),
            lambda a,j,h: h[1]['jobs'][0].update(conclusion='failure'),
            lambda a,j,h: h[1]['jobs'][0].update(run_attempt=2),
            lambda a,j,h: h[1]['jobs'][0].update(run_id=999),
            lambda a,j,h: h[1]['jobs'][0].update(head_sha='x'*40),
            lambda a,j,h: h[1]['jobs'][0].update(started_at='2026-10-27T02:01:00Z'),
            lambda a,j,h: h[1]['jobs'][0]['steps'][0].update(completed_at='2026-10-27T01:08:00Z'),
            lambda a,j,h: h[1]['run'].update(head_sha='x'*40),
            lambda a,j,h: h[1]['run'].update(id=999),
            lambda a,j,h: h[1]['run'].update(run_attempt=2),
            lambda a,j,h: h[1]['run'].update(path='.github/workflows/wrong.yml'),
            lambda a,j,h: h[1]['run'].update(check_suite_id=999),
            lambda a,j,h: h[1]['run']['pull_requests'][0]['base'].update(sha='x'*40),
        ]
        for kind in ('task100-ada-websocket-transport-14', 'perf007-real-uci-schema-ipv6'):
            for index, mutate in enumerate(mutations):
                artifacts = [dict(name=kind)]
                jobs, attempts = metadata(artifacts, 2, {kind: 1})
                mutate(artifacts[0], jobs[0], attempts)
                with self.subTest(kind=kind, mutation=index), self.assertRaises((ValueError, KeyError)):
                    resolve(artifacts[0], job_name(kind), jobs, attempts, '123', '2')

    def test_superseded_artifact_not_reused_after_new_successful_execution(self):
        artifacts = [dict(name=NAMES[0])]
        jobs, attempts = metadata(artifacts, 2, {NAMES[0]: 2})
        artifacts[0]['created_at'] = '2026-10-27T01:08:30Z'
        with self.assertRaises(ValueError):
            resolve(artifacts[0], job_name(NAMES[0]), jobs, attempts, '123', '2')


class GateIdentity(unittest.TestCase):
    def test_gate_must_execute_in_current_attempt(self):
        _, history = metadata([], 2)
        run = history[2]['run']
        job = dict(name='pr-ci-required', id=55, run_id=123, run_attempt=2, head_sha='c'*40,
                   status='in_progress', started_at='2026-10-27T02:20:00Z')
        verify_final_job([job], run, 'pr-ci-required')
        for key, value in [('run_attempt', 1), ('run_id', 999), ('head_sha', 'wrong'),
                           ('status', 'completed'), ('started_at', '2026-10-27T01:20:00Z')]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                verify_final_job([dict(job, **{key: value})], run, 'pr-ci-required')
        for jobs in ([], [job, copy.deepcopy(job)]):
            with self.assertRaises(ValueError):
                verify_final_job(jobs, run, 'pr-ci-required')

    def test_archive_digest_checked_before_bundle_contents(self):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, 'w') as archive:
            archive.writestr('identity.json', '{}')
        data = stream.getvalue()
        artifact = dict(id=1, digest='sha256:' + hashlib.sha256(data).hexdigest())
        with patch.dict(gate.os.environ, GITHUB_REPOSITORY='fixture/repo', GH_TOKEN='fixture'), \
                patch.object(gate.urllib.request, 'build_opener') as opener:
            opener.return_value.open.side_effect = lambda *a, **k: io.BytesIO(data)
            self.assertEqual(gate.download_bundle(artifact), {'identity.json': b'{}'})
            for digest in (None, 'sha256:'+'0'*64):
                with self.assertRaisesRegex(ValueError, 'archive digest'):
                    gate.download_bundle(dict(artifact, digest=digest))

    def test_history_fetch_uses_every_attempt_and_current_snapshot(self):
        with patch.object(gate, 'api', side_effect=lambda path: path), \
                patch.object(gate, 'pages', side_effect=lambda path, key: path):
            result = gate.evidence('123', '3')
        self.assertEqual(set(result['attempts']), {1, 2, 3})
        self.assertEqual(result['jobs'], 'actions/runs/123/attempts/3/jobs')
        for attempt in ('0', '101'):
            with self.assertRaises(ValueError):
                gate.evidence('123', attempt)

    def test_final_result_not_published_when_provenance_or_bundle_gate_fails(self):
        import os
        from pathlib import Path
        import tempfile
        manifest = dict(identities=dict(merge='a'*40, head='c'*40), deep=True,
                        deep_jobs=sorted(gate.SHARDS), selected_cells=34, skipped_cells=0)
        _, attempts = metadata([], 2)
        run = attempts[2]['run']
        env = dict(SELECTION_MANIFEST=json.dumps(manifest), NEEDS_JSON='{}',
                   GITHUB_RUN_ID='123', GITHUB_RUN_ATTEMPT='2', GITHUB_SHA='a'*40,
                   GITHUB_EVENT_NAME='pull_request', GITHUB_JOB='pr-ci-required')
        for target in ('verify_records', 'verify_final_job', 'reconcile_task100', 'reconcile_primary'):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory, \
                    patch.dict(gate.os.environ, env), patch.object(gate, 'from_environment', return_value=manifest), \
                    patch.object(gate, 'inventory', return_value={}), patch.object(gate, 'api', return_value=run), \
                    patch.object(gate, 'verify_source'), patch.object(gate, 'validate'), \
                    patch.object(gate, 'verify_records'), \
                    patch.object(gate, 'git', return_value=b'b'*40), \
                    patch.object(gate, 'evidence', return_value=dict(jobs=[], attempts=attempts, artifacts=[])), \
                    patch.object(gate, 'verify_final_job'), patch.object(gate, 'reconcile_task100', return_value={}), \
                    patch.object(gate, 'reconcile_primary', return_value={}), \
                    patch.object(gate, target, side_effect=ValueError('mandatory gate failed')):
                before = os.getcwd()
                try:
                    os.chdir(directory)
                    with self.assertRaises(ValueError):
                        gate.main()
                    self.assertFalse(Path('ci-required-result.json').exists())
                finally:
                    os.chdir(before)

    def test_pagination_cannot_hide_duplicates_truncation_or_changed_count(self):
        for pages in ([dict(total_count=2, jobs=[dict(id=1)]), dict(total_count=2, jobs=[])],
                      [dict(total_count=0, jobs=[dict(id=1)])]):
            with patch.object(gate, 'api', side_effect=pages), self.assertRaises(ValueError):
                gate.pages('actions/runs/123/attempts/1/jobs', 'jobs')


if __name__ == '__main__':
    unittest.main()