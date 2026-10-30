"""Synthetic API/artifact fixtures only; never benchmark acceptance evidence."""
import copy
import hashlib
import io
import json
import tarfile
import unittest
import zipfile

from ci_bench import contract, reconcile
from test_ci_select_bench import rows


def encoded(value):
    return json.dumps(value, sort_keys=True).encode()


class Reconciliation(unittest.TestCase):
    def fixture(self, current=1, producers=None):
        producers = producers or {'14': 1, '16': 1}
        attempts, artifacts, bundles, needs = {}, [], {}, {}
        for number in range(1, current + 1):
            run = dict(id=123, run_attempt=number, head_sha='a' * 40,
                       event='workflow_dispatch', path=reconcile.WORKFLOW, head_branch='main',
                       check_suite_id=456, repository=dict(id=10, full_name=reconcile.controller.REPOSITORY),
                       head_repository=dict(id=10), pull_requests=[],
                       run_started_at=f'2026-10-{number:02d}T00:00:00Z')
            jobs = []
            for compiler, job_name in contract.JOBS.items():
                origin = producers[compiler]
                if origin > number:
                    continue
                prefix = f'2026-10-{origin:02d}T0{1 if compiler == "14" else 2}'
                steps = [dict(name=name, number=index + 1, status='completed', conclusion='success',
                              started_at=prefix + f':{index + 1:02d}:00Z',
                              completed_at=prefix + f':{index + 1:02d}:59Z')
                         for index, name in enumerate(reconcile.STEPS)]
                jobs.append(dict(id=number * 100 + int(compiler), run_id=123, run_attempt=number,
                                 name=job_name, head_sha='a' * 40, status='completed', conclusion='success',
                                 started_at=prefix + ':00:00Z', completed_at=prefix + ':10:00Z',
                                 runner_id=int(compiler), runner_name='runner-' + compiler, steps=steps))
            attempts[number] = dict(run=run, jobs=jobs)
        jobs = attempts[current]['jobs']
        jobs.append(dict(id=999, run_id=123, run_attempt=current, name=contract.FINAL_JOB,
                         head_sha='a' * 40, status='in_progress',
                         started_at=f'2026-10-{current:02d}T03:00:00Z'))
        all_rows = rows()
        for index, (compiler, job_name) in enumerate(contract.JOBS.items()):
            origin = producers[compiler]
            producer_job = next(j for j in attempts[origin]['jobs'] if j['name'] == job_name)
            identity = dict(source_commit=contract.SOURCE_COMMIT, source_tree=contract.SOURCE_TREE,
                            workflow_commit='a' * 40, workflow_tree='b' * 40, workflow_sha256='c' * 64,
                            compiler=compiler, job=job_name, job_id=str(producer_job['id']),
                            run_id='123', run_attempt=str(origin), source_archive_sha256='d' * 64,
                            runner_id=producer_job['runner_id'], runner_name=producer_job['runner_name'])
            identity['preparation_seconds'] = 5
            runs = all_rows[index * 4:(index + 1) * 4]
            result = dict(valid=True, completed=True, identity=identity, runs=runs,
                          budget=contract.budget(), allocation=runs[0]['allocation'], performance_success=False,
                          host_resources=dict(available_memory_bytes=8 * 1024**3, free_disk_bytes=10 * 1024**3),
                          image_id='sha256:' + 'e' * 64,
                          preflight=dict(external_busy_fraction=.01, steal_fraction=0, elapsed_seconds=10))
            log_archive = io.BytesIO()
            with tarfile.open(fileobj=log_archive, mode='w:gz') as archive:
                for row in runs:
                    log = b'CI_BENCH_RESULT ' + encoded(row['candidate_report']) + b'\n'
                    info = tarfile.TarInfo(row['worker_log']); info.size = len(log)
                    archive.addfile(info, io.BytesIO(log))
            files = {'identity.json': encoded(identity), 'result.json': encoded(result),
                     'image.json': encoded(dict(image_id=result['image_id'], setup_seconds=100)),
                     'preflight.json': encoded(dict(allocation=result['allocation'], sample=result['preflight'])),
                     'progress.json': encoded(dict(identity=identity, runs=runs)),
                     'current-contention.json': encoded(dict(iteration=3, samples=runs[-1]['samples'])),
                     'worker-logs.tar.gz': log_archive.getvalue()}
            files['packaging.json'] = encoded(dict(wall_seconds=1, archive_bytes=len(files['worker-logs.tar.gz']),
                files={p: hashlib.sha256(data).hexdigest() for p, data in files.items()}))
            receipt = dict(outcome='success', artifact_id=str(index * 2 + 1), artifact_digest='f' * 64,
                           run_id='123', run_attempt=str(origin), job=job_name, job_id=identity['job_id'],
                           compiler=compiler, upload_step_envelope_seconds=2,
                           uploaded_input_bytes=sum(map(len, files.values())))
            for kind, bundle in [('measurements', files), ('upload-receipt', {'upload-receipt.json': encoded(receipt)})]:
                name = f'ci-bench-001-gnat-{compiler}-{kind}'
                step = next(s for s in producer_job['steps'] if s['name'] == reconcile.UPLOADS[kind])
                artifacts.append(dict(id=index * 2 + (1 if kind == 'measurements' else 2), name=name,
                                      expired=False, size_in_bytes=1, digest='sha256:' + 'f' * 64,
                                      created_at=step['started_at'].replace(':00Z', ':30Z'),
                                      workflow_run=dict(id=123, head_sha='a' * 40,
                                                        repository_id=10, head_repository_id=10)))
                bundles[name] = bundle
            needs[job_name] = dict(result='success')
        return dict(artifacts=artifacts, jobs=jobs, attempts=attempts, run_id='123', attempt=str(current),
                    workflow='a' * 40, tree='b' * 40, workflow_digest='c' * 64, needs=needs,
                    loader=lambda artifact: bundles[artifact['name']]), bundles

    def verify(self, fixture):
        return reconcile.verify(**fixture[0])

    def edit(self, bundles, compiler, path, change):
        bundle = bundles[f'ci-bench-001-gnat-{compiler}-measurements']
        value = json.loads(bundle[path])
        change(value)
        bundle[path] = encoded(value)
        packaging = json.loads(bundle['packaging.json'])
        packaging['files'] = {p: hashlib.sha256(data).hexdigest() for p, data in bundle.items() if p != 'packaging.json'}
        bundle['packaging.json'] = encoded(packaging)
        receipt_bundle = bundles[f'ci-bench-001-gnat-{compiler}-upload-receipt']
        receipt = json.loads(receipt_bundle['upload-receipt.json'])
        receipt['uploaded_input_bytes'] = sum(map(len, bundle.values()))
        receipt_bundle['upload-receipt.json'] = encoded(receipt)

    def test_complete_and_independently_resolved_retry_producers(self):
        for current, origins in [(1, {'14': 1, '16': 1}), (2, {'14': 1, '16': 1}),
                                 (2, {'14': 1, '16': 2}), (2, {'14': 2, '16': 2})]:
            with self.subTest(current=current, origins=origins):
                result = self.verify(self.fixture(current, origins))
                self.assertEqual(result['iterations'], 8)
                self.assertEqual(result['totals'], dict(provider=13992, generated_support=1200,
                                                       bind=240, link=240, execute=240))
                self.assertFalse(result['performance_success'])
                for compiler in contract.COMPILERS:
                    self.assertEqual(result['compilers'][compiler]['producers']['measurements']['run_attempt'],
                                     str(origins[compiler]))

    def test_needs_jobs_attempts_and_artifact_provenance_fail_closed(self):
        mutations = [lambda f: f['needs'].pop(contract.JOBS['16']),
                     lambda f: f['needs'][contract.JOBS['14']].update(result='skipped'),
                     lambda f: f['jobs'][0].update(conclusion='failure'),
                     lambda f: f['jobs'][0]['steps'][0].update(conclusion='skipped'),
                     lambda f: f['jobs'][0]['steps'].pop(),
                     lambda f: f['jobs'][0].update(run_attempt=2),
                     lambda f: f['attempts'][1]['run'].update(event='pull_request'),
                     lambda f: f['attempts'][1]['run'].update(head_branch='feature'),
                     lambda f: f['artifacts'].pop(),
                     lambda f: f['artifacts'].append(copy.deepcopy(f['artifacts'][0])),
                     lambda f: f['artifacts'][0].update(expired=True),
                     lambda f: f['artifacts'][0].update(size_in_bytes=0),
                     lambda f: f['artifacts'][0].update(digest=None),
                     lambda f: f['artifacts'][0]['workflow_run'].update(id=999),
                     lambda f: f['artifacts'][0]['workflow_run'].update(head_sha='0' * 40),
                     lambda f: f['artifacts'][0].update(created_at='2026-10-01T01:09:00Z'),
                     lambda f: f['jobs'][-1].update(started_at='2026-09-30T03:00:00Z')]
        for index, mutate in enumerate(mutations):
            fixture = self.fixture()
            mutate(fixture[0])
            with self.subTest(index=index), self.assertRaises((ValueError, KeyError)):
                self.verify(fixture)

    def test_identity_and_receipt_claims_do_not_select_producers(self):
        for path in ('identity.json', 'upload-receipt.json'):
            for key, value in [('run_attempt', '2'), ('job_id', '999'), ('run_id', '122'),
                               ('compiler', '16'), ('job', contract.JOBS['16'])]:
                fixture, bundles = self.fixture()
                kind = 'measurements' if path == 'identity.json' else 'upload-receipt'
                bundle = bundles['ci-bench-001-gnat-14-' + kind]
                claim = json.loads(bundle[path]); claim[key] = value; bundle[path] = encoded(claim)
                with self.subTest(path=path, key=key), self.assertRaises(ValueError):
                    reconcile.verify(**fixture)

    def test_workload_resource_scope_and_packaging_negatives(self):
        mutations = [lambda r: r.update(completed=False), lambda r: r['runs'].pop(),
                     lambda r: r['runs'].reverse(), lambda r: r.update(performance_success=True),
                     lambda r: r.update(budget={}),
                     lambda r: r['host_resources'].update(available_memory_bytes=0),
                     lambda r: r['runs'][0]['candidate_report']['counts'].update(execute=29),
                     lambda r: r['runs'][0]['candidate_report']['builds'][0].update(bind=False),
                     lambda r: r['runs'][0]['candidate_report']['builds'][0].update(link=False),
                     lambda r: r['runs'][0]['candidate_report'].update(source_tree='0' * 40),
                     lambda r: r['runs'][0]['candidate_report'].update(instrumented=True),
                     lambda r: r['runs'][0]['final_cgroup'].update(oom_kill=1),
                     lambda r: r['runs'][0]['final_cgroup'].update(memory_peak=0),
                     lambda r: r['runs'][0]['cgroup_limits'].update({'cpu.max': '100000 100000'}),
                     lambda r: r['runs'][0].update(samples=[]),
                     lambda r: r['runs'][0]['samples'][0].update(external_busy_fraction=.06),
                     lambda r: r['runs'][0]['samples'][0].update(steal_fraction=.02),
                     lambda r: r['runs'][0]['samples'][0].update(elapsed_seconds=4)]
        for index, mutate in enumerate(mutations):
            fixture = self.fixture()
            self.edit(fixture[1], '14', 'result.json', mutate)
            with self.subTest(index=index), self.assertRaises((ValueError, KeyError)):
                self.verify(fixture)
        for path, key, value in [('identity.json', 'workflow_tree', '0' * 40),
                                  ('identity.json', 'workflow_sha256', '0' * 64),
                                  ('identity.json', 'runner_id', 999),
                                  ('identity.json', 'source_archive_sha256', '0' * 64),
                                  ('image.json', 'setup_seconds', 1801)]:
            fixture = self.fixture()
            self.edit(fixture[1], '14', path, lambda r: r.update({key: value}))
            with self.subTest(path=path, key=key), self.assertRaises(ValueError):
                self.verify(fixture)
        fixture = self.fixture()
        fixture[1]['ci-bench-001-gnat-14-measurements']['result.json'] += b' '
        with self.assertRaisesRegex(ValueError, 'digests'):
            self.verify(fixture)

    def test_receipt_upload_success_digest_and_counts_are_mandatory(self):
        for key, value in [('artifact_id', '999'), ('artifact_digest', '0' * 64),
                           ('uploaded_input_bytes', 0), ('outcome', 'failure'),
                           ('upload_step_envelope_seconds', 601), ('upload_step_envelope_seconds', float('nan'))]:
            fixture = self.fixture()
            bundle = fixture[1]['ci-bench-001-gnat-14-upload-receipt']
            receipt = json.loads(bundle['upload-receipt.json']); receipt[key] = value
            bundle['upload-receipt.json'] = encoded(receipt)
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                self.verify(fixture)
        fixture = self.fixture()
        self.edit(fixture[1], '14', 'packaging.json', lambda p: p.update(wall_seconds=601))
        with self.assertRaises(ValueError):
            self.verify(fixture)
        fixture = self.fixture()
        files = fixture[1]['ci-bench-001-gnat-14-measurements']
        files.pop('worker-logs.tar.gz')
        with self.assertRaises(ValueError):
            self.verify(fixture)

    def test_same_runner_within_compiler_not_between_compilers(self):
        # Log/report agreement is checked independently; differing seed hashes
        # across compilers are permitted by the row validator, not within ABBA.
        data = rows()
        for row in data[4:]:
            row['candidate_report']['seed_sha256'] = '1' * 64
        self.assertTrue(reconcile.controller.validate_rows(data))
        data[4]['candidate_report']['seed_sha256'] = '2' * 64
        with self.assertRaises(ValueError):
            reconcile.controller.validate_rows(data)

    def test_complete_digest_checked_zip_transport(self):
        fixture, bundles = self.fixture()
        archives = {}
        for artifact in fixture['artifacts']:
            name = artifact['name']
            bundle = bundles[name]
            if name.endswith('upload-receipt'):
                measurement = name.replace('upload-receipt', 'measurements')
                receipt = json.loads(bundle['upload-receipt.json'])
                receipt['artifact_digest'] = hashlib.sha256(archives[measurement]).hexdigest()
                bundle['upload-receipt.json'] = encoded(receipt)
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, 'w') as archive:
                for path, data in bundle.items():
                    archive.writestr(path, data)
            archives[name] = stream.getvalue()
            artifact['digest'] = 'sha256:' + hashlib.sha256(archives[name]).hexdigest()
            artifact['size_in_bytes'] = len(archives[name])
        fixture['loader'] = lambda a: reconcile.decode_archive(archives[a['name']], a)
        self.assertEqual(reconcile.verify(**fixture)['iterations'], 8)
        name = fixture['artifacts'][0]['name']
        archives[name] += b'corruption'
        with self.assertRaisesRegex(ValueError, 'corrupt'):
            reconcile.verify(**fixture)

    def test_cross_attempt_reverse_execution_and_copied_producer_forgery(self):
        fixture = self.fixture(2, {'14': 2, '16': 1})
        with self.assertRaisesRegex(ValueError, 'overlap|order'):
            self.verify(fixture)
        fixture = self.fixture(2)
        # Latest copied IDs/attempts must never replace the historical producer.
        bundle = fixture[1]['ci-bench-001-gnat-14-upload-receipt']
        receipt = json.loads(bundle['upload-receipt.json'])
        receipt.update(run_attempt='2', job_id='214'); bundle['upload-receipt.json'] = encoded(receipt)
        with self.assertRaises(ValueError):
            self.verify(fixture)

    def test_archives_are_digest_checked_bounded_and_never_extracted(self):
        for names in [('result.json',), ('../result.json',), ('/result.json',), ('a\\b',),
                      ('result.json', 'result.json')]:
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, 'w') as archive:
                for name in names:
                    archive.writestr(name, b'{}')
            data = stream.getvalue()
            artifact = dict(digest='sha256:' + hashlib.sha256(data).hexdigest())
            if names == ('result.json',):
                self.assertEqual(reconcile.decode_archive(data, artifact), {'result.json': b'{}'})
                with self.assertRaises(ValueError):
                    reconcile.decode_archive(data + b'corrupt', artifact)
            else:
                with self.subTest(names=names), self.assertRaises(ValueError):
                    reconcile.decode_archive(data, artifact)


if __name__ == '__main__':
    unittest.main()