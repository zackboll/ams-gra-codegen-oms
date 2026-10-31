"""CI-BENCH-001 registration and fail-closed security/measurement mutations."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import ci_policy
import ci_selection
from ci_bench import controller, policy, contract
from ci_bench.worker import compiled_units


class Registration(unittest.TestCase):
    def setUp(self):
        self.inventory = ci_selection.inventory()
        self.workflows = {p.name: p.read_text() for p in
                          (ci_selection.ROOT / '.github/workflows').glob('*.yml')}

    def test_registered_manual_job_does_not_expand_main_or_pr(self):
        self.assertEqual(ci_policy.check(self.inventory, self.workflows), 46)
        self.assertEqual(self.inventory['manual_benchmark']['lane'], 3)
        for full in (False, True):
            selection = ci_selection.select([], self.inventory, full_main=full)
            for job in policy.JOBS:
                self.assertNotIn(job, selection['jobs'])
        self.assertEqual(self.inventory['lanes'], [f'oms-main-slot-{n}' for n in range(1, 6)])

    def test_v2_deep_selection_and_manual_topology_remain_independent(self):
        registered = ci_selection.deep_jobs(self.inventory)
        self.assertEqual(sum(len(r['cells']) for r in registered.values()), 18)
        cases = [('README.md', set()),
                 ('tests/ada-websocket/transport_probe.adb', {'ada-websocket-real-sleet', 'ada-real-uci-dlz-sleet', 'ada-real-uci-log-sleet', 'ada-real-uci-rdma-sleet'}),
                 ('scripts/ci_bench/reconcile.py', set(registered)),
                 ('.github/workflows/portable-provider-benchmark.yml', set(registered))]
        for path, wanted in cases:
            change = dict(status='M', paths=[path], old_mode='100644', new_mode='100644')
            manifest = ci_selection.select([change], self.inventory)
            ci_selection.validate_deep(manifest, self.inventory)
            with self.subTest(path=path):
                self.assertEqual(manifest['version'], 2)
                self.assertEqual(manifest['deep_jobs'], sorted(wanted))
                self.assertEqual(manifest['deep_selected_cells'] + manifest['deep_skipped_cells'], 18)
                self.assertFalse(set(policy.JOBS) & (set(manifest['jobs']) | set(manifest['deep_jobs'])))
        manifest = ci_selection.select([], self.inventory, full_main=True)
        self.assertEqual(manifest['selected_cells'], 20)
        self.assertEqual(manifest['deep_selected_cells'], 18)
        self.assertEqual(manifest['deep_skipped_cells'], 0)
        for mutation in (dict(version=1), dict(deep_selected_cells=11),
                         dict(deep_jobs=sorted([*registered, contract.FINAL_JOB]))):
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                ci_selection.validate_deep(dict(manifest, **mutation), self.inventory)

    def test_registration_removal_and_extra_workflow_rejected(self):
        del self.inventory['manual_benchmark']
        with self.assertRaisesRegex(ValueError, 'manual benchmark inventory'):
            ci_policy.check(self.inventory, self.workflows)
        self.inventory = ci_selection.inventory()
        self.workflows['unmapped-benchmark.yml'] = self.workflows[policy.WORKFLOW]
        with self.assertRaisesRegex(ValueError, 'unmapped workflow'):
            ci_policy.check(self.inventory, self.workflows)

    def test_workflow_security_mutations(self):
        original = self.workflows[policy.WORKFLOW]
        cases = [
            ('  workflow_dispatch:', '  pull_request:\n  workflow_dispatch:'),
            ('  workflow_dispatch:', '  push:\n  workflow_dispatch:'),
            ('  workflow_dispatch:', '  workflow_call:\n  workflow_dispatch:'),
            ('contents: read', 'contents: write'),
            ('contents: read', 'contents: read\n  id-token: write'),
            ('runs-on: ubuntu-24.04', 'runs-on: self-hosted'),
            ('group: oms-main-slot-3', 'group: oms-main-slot-6'),
            ('queue: max', 'queue: single'),
            ('queue: max', 'queue: max\n      cancel-in-progress: true'),
            ("github.ref == 'refs/heads/main'", 'true'),
            ('persist-credentials: false', 'persist-credentials: true'),
            ('ref: ${{ github.sha }}', 'ref: ${{ inputs.source_sha }}'),
            ('timeout-minutes: 360', 'timeout-minutes: 720'),
            ('python3 scripts/ci_bench/controller.py campaign', 'bash candidate/scripts/run.sh'),
            ('SOURCE_SHA: ${{ inputs.source_sha }}', 'TOKEN: ${{ secrets.TOKEN }}'),
            ('if-no-files-found: error', 'if-no-files-found: warn'),
            ('if: always()', 'if: success()'),
            ('source_sha:', 'source_ref:'),
            ('needs: [benchmark-gnat-14]', 'needs: []'),
            ('needs: [benchmark-gnat-14, benchmark-gnat-16]', 'needs: [benchmark-gnat-14]'),
            ("BENCH_COMPILER: '16'", "BENCH_COMPILER: '14'"),
            ('actions: read', 'actions: write'),
            ('timeout-minutes: 15', 'timeout-minutes: 360'),
            ('ci-bench-001-gnat-16-measurements', 'ci-bench-001-gnat-14-measurements'),
            ('python3 scripts/ci_bench/reconcile.py', 'true'),
            ('BENCH_NEEDS: ${{ toJSON(needs) }}', "BENCH_NEEDS: '{}'"),
            ('      - name: Enforce complete campaign and acceptance gates', '      - continue-on-error: true\n        name: Enforce complete campaign and acceptance gates'),
        ]
        for before, after in cases:
            with self.subTest(before=before):
                self.assertIn(before, original)
                self.workflows[policy.WORKFLOW] = original.replace(before, after, 1)
                with self.assertRaisesRegex(ValueError, 'workflow trust contract'):
                    ci_policy.check(self.inventory, self.workflows)

    def test_inventory_cannot_bless_candidate_workflow(self):
        self.inventory['manual_benchmark']['workflow_sha256'] = '0' * 64
        with self.assertRaises(ValueError):
            ci_policy.check(self.inventory, self.workflows)

    def test_missing_controller_inventory_rejected(self):
        self.inventory['manual_benchmark']['sources_sha256'].pop('scripts/ci_bench/controller.py')
        with self.assertRaisesRegex(ValueError, 'source inventory incomplete'):
            ci_policy.check(self.inventory, self.workflows)

    def test_invalid_source_policy_or_lane_rejected(self):
        for key, value in [('lane', 6), ('events', ['push']), ('source_policy', 'feature workflow')]:
            inventory = copy.deepcopy(self.inventory)
            inventory['manual_benchmark'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                ci_policy.check(inventory, self.workflows)

    def test_manual_topology_registration_is_explicit(self):
        for job in policy.JOBS:
            inventory = copy.deepcopy(self.inventory)
            del inventory['manual_benchmark']['jobs'][job]
            with self.subTest(job=job), self.assertRaises(ValueError):
                ci_policy.check(inventory, self.workflows)
        for job, key, value in [(contract.JOBS['16'], 'needs', []),
                                (contract.FINAL_JOB, 'needs', [contract.JOBS['14']]),
                                (contract.JOBS['14'], 'timeout_minutes', 720)]:
            inventory = copy.deepcopy(self.inventory)
            inventory['manual_benchmark']['jobs'][job][key] = value
            with self.subTest(job=job, key=key), self.assertRaises(ValueError):
                ci_policy.check(inventory, self.workflows)

    def test_every_manual_job_lock_and_authorization_is_mandatory(self):
        header, jobs = ci_policy.blocks(self.workflows[policy.WORKFLOW])
        self.assertEqual(set(jobs), set(policy.JOBS))
        for job in jobs:
            for before, after in [('group: oms-main-slot-3', 'group: oms-main-slot-4'),
                                  ('queue: max', 'queue: max\n      cancel-in-progress: true'),
                                  ("github.ref == 'refs/heads/main'", 'true'),
                                  ('persist-credentials: false', 'persist-credentials: true')]:
                changed = dict(jobs)
                self.assertIn(before, changed[job])
                changed[job] = changed[job].replace(before, after, 1)
                workflows = dict(self.workflows)
                workflows[policy.WORKFLOW] = header + ''.join('  ' + j + ':\n' + body for j, body in changed.items())
                with self.subTest(job=job, before=before), self.assertRaises(ValueError):
                    ci_policy.check(self.inventory, workflows)


class Authorization(unittest.TestCase):
    def setUp(self):
        self.env = dict(GITHUB_EVENT_NAME='workflow_dispatch', GITHUB_REF='refs/heads/main',
                        GITHUB_REPOSITORY=controller.REPOSITORY, RUNNER_ENVIRONMENT='github-hosted',
                        SOURCE_SHA='1' * 40, EXPECTED_WORKFLOW_SHA='2' * 40)

    def test_exact_main_hosted_dispatch(self):
        self.assertEqual(controller.authorize(self.env), ('1' * 40, '2' * 40))

    def test_event_branch_repository_runner_and_sha_mutations(self):
        for key, values in dict(GITHUB_EVENT_NAME=['push', 'pull_request', 'workflow_call'],
                                GITHUB_REF=['refs/heads/feature', 'refs/pull/1/merge'],
                                GITHUB_REPOSITORY=['attacker/fork'], RUNNER_ENVIRONMENT=['self-hosted'],
                                SOURCE_SHA=['main', '--help', 'A' * 40, 'a' * 39, 'a' * 40 + '\n', '$(id)'],
                                EXPECTED_WORKFLOW_SHA=['main']).items():
            for value in values:
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    controller.authorize(dict(self.env, **{key: value}))

    def test_offline_unprivileged_bounded_container(self):
        command = controller.docker_command('sha256:' + 'a' * 64, Path('/tmp/candidate'), 'ci-bench-001-0', [0, 1], '14', 'original')
        for required in ('--network=none', '--read-only', '--cap-drop=ALL',
                         '--security-opt=no-new-privileges', '--user=10001:10001',
                          '--pids-limit=256', '--memory=6g', '--memory-swap=6g', '--cpus=2', '--cpuset-cpus=0,1'):
            self.assertIn(required, command)
        mounts = [command[i+1] for i, arg in enumerate(command) if arg == '--mount']
        self.assertEqual(mounts, ['type=bind,source=/tmp/candidate,target=/source,readonly'])
        self.assertNotIn('--privileged', command)
        self.assertNotIn('--env', command)
        self.assertNotIn('/var/run/docker.sock', ' '.join(command))
        self.assertNotIn('GITHUB', ' '.join(command))

    def test_output_is_bounded_and_not_workflow_interpreted(self):
        import io
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'worker.log'
            payload = b'::error::untrusted\n' + b'x' * (controller.LIMIT + 1)
            controller.drain(io.BytesIO(payload), path)
            self.assertEqual(path.stat().st_size, controller.LIMIT)

    def test_compiler_can_only_run_in_its_registered_job(self):
        for compiler in contract.COMPILERS:
            with patch.dict(os.environ, BENCH_COMPILER=compiler, GITHUB_JOB=contract.JOBS[compiler]):
                self.assertEqual(controller.compiler_job(), compiler)
            for wrong in [contract.FINAL_JOB, contract.JOBS['16' if compiler == '14' else '14'], 'unknown']:
                with patch.dict(os.environ, BENCH_COMPILER=compiler, GITHUB_JOB=wrong), self.assertRaises(ValueError):
                    controller.compiler_job()


def rows():
    result = []
    for compiler, mode in controller.ORDER:
        records = [dict(profile=p, name=n, policy=a, compiler='14.2.0' if compiler == '14' else '16.1.0',
                        family='portable', intentional_red=False, compiler_exit=0)
                   for p in ('oam', 'qualified') for n in (
                       'ArraySample', 'Array_Encode_Hex_Binary', 'Encode_Hex_Binary',
                       'Array_encode_hex_binary', 'Array_Decode_Hex_Binary')
                   for a in ('normal', 'Check', 'Ignore')]
        builds = []
        for record in records:
            ns = 'task093' if record['profile'] == 'qualified' else 'oam'
            units = ['helper_demand_consumer.adb', 'service_codec.adb', 'programs.ads',
                     f'programs-{ns}.ads', 'ams_gra_oms_json_guard.adb']
            if mode == 'original':
                units += [f'provider-{n}.adb' for n in range(106)]
            builds.append(dict(cell='{profile}-{name}-{policy}-build'.format(**record), units=units,
                               bind=True, link=True, source_sha256='d' * 64,
                               execute_marker='TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n'))
        if mode == 'reuse':
            builds += [dict(cell=f'provider-{a}-build', units=[f'provider-{n}.adb' for n in range(106)])
                       for a in ('normal', 'Check', 'Ignore')]
        report = dict(completed=True, compiler=compiler, mode=mode, jobs=2, instrumented=False,
                      source_commit=contract.SOURCE_COMMIT, source_tree=contract.SOURCE_TREE,
                      measurement_scope=contract.SCOPE, records=records, builds=builds,
                      helper_wall_seconds=.5, helper_child_cpu_seconds=1,
                      generator_build_seconds=.1, setup_seconds=.2,
                      generator_sha256='a' * 64, seed_sha256='b' * 64, dependency_identity_sha256=compiler * 32,
                      counts=dict(provider=3180 if mode == 'original' else 318,
                                  generated_support=150, bind=30, link=30, execute=30))
        topology = {0: dict(siblings=[0, 1], core=0, package=0),
                    1: dict(siblings=[0, 1], core=0, package=0)}
        result.append(dict(compiler=compiler, mode=mode, candidate_report=report, wall_seconds=1,
                           allocation=controller.validate_topology(topology, [0, 1], [0, 1]),
                           oom=False, exit_code=0,
                           cpu_seconds=10, cgroup_peak_memory_bytes=1024,
                           samples=[dict(at_seconds=1, usage_usec=10000000, throttled_usec=0,
                                         external_busy_fraction=.01, steal_fraction=0, elapsed_seconds=1)]))
    for index, row in enumerate(result):
        row['iteration'] = index % 4
        row['worker_log'] = f'worker-{index % 4}.log'
        row.update(cgroup_limits={'cpu.max': '200000 100000', 'cpuset.cpus.effective': '0,1',
                                 'memory.max': str(contract.MEMORY_BYTES), 'memory.swap.max': '0', 'pids.max': '256'},
                   final_cgroup=dict(usage_usec=row['samples'][-1]['usage_usec'],
                                     throttled_usec=row['samples'][-1]['throttled_usec'],
                                     memory_peak=row['cgroup_peak_memory_bytes'], oom=0, oom_kill=0))
    return result


class Measurement(unittest.TestCase):
    def test_budget_does_not_credit_provider_speedup(self):
        self.assertEqual(sum(contract.budget().values()), 360 * 60)
        self.assertEqual(contract.budget()['campaigns'], 4 * contract.ITERATION_SECONDS)
        self.assertEqual(contract.ITERATION_SECONDS, 4500)
        self.assertEqual(contract.FINAL_MINUTES, 15)
        for child, harness in [(2026, 564), (2641, 630)]:
            self.assertLess((child + harness) * 1.25 + 120, contract.ITERATION_SECONDS)
        self.assertEqual(contract.WORKERS, 2)

    def test_scope_cannot_accept_perf005_or_another_candidate(self):
        controller.validate_source_scope(contract.SOURCE_COMMIT, contract.SOURCE_TREE)
        for source, tree in [('a' * 40, contract.SOURCE_TREE), (contract.SOURCE_COMMIT, 'b' * 40)]:
            with self.assertRaisesRegex(ValueError, 'BENCHMARK_SCOPE_INSUFFICIENT'):
                controller.validate_source_scope(source, tree)

    def test_topology_and_smt_sibling_negatives(self):
        topology = {0: dict(siblings=[0, 2], core=0, package=0),
                    2: dict(siblings=[0, 2], core=0, package=0),
                    1: dict(siblings=[1, 3], core=1, package=0),
                    3: dict(siblings=[1, 3], core=1, package=0)}
        assigned = controller.validate_topology(topology, [0, 1], [0, 1, 2, 3])
        self.assertEqual(assigned['monitored_smt_siblings'], [0, 1, 2, 3])
        self.assertEqual(assigned['guest_core_count'], 2)
        self.assertFalse(assigned['physical_isolation_proven'])
        assigned = controller.validate_topology(topology, [0, 2], [0, 1, 2, 3])
        self.assertEqual(assigned['guest_core_count'], 1)
        for selected, allowed in [([0], [0, 1, 2, 3]), ([0, 1], [0, 1]), ([0, 0], [0, 1, 2, 3])]:
            with self.assertRaises(ValueError):
                controller.validate_topology(topology, selected, allowed)
        topology[2]['siblings'] = [2]
        with self.assertRaises(ValueError):
            controller.validate_topology(topology, [0, 1], [0, 1, 2, 3])

    def test_actual_cgroup_mismatch_and_missing_counters(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = {'cpu.max': '200000 100000', 'cpuset.cpus.effective': '0-1',
                     'memory.max': str(6 * 1024**3), 'memory.swap.max': '0', 'pids.max': '256'}
            for name, value in files.items():
                (root / name).write_text(value)
            controller.validate_cgroup_limits(root, [0, 1])
            for name, wrong in [('cpu.max', '100000 100000'), ('cpu.max', 'max 100000'),
                                ('cpuset.cpus.effective', '0'), ('memory.max', 'max'),
                                ('memory.swap.max', '100'), ('pids.max', 'max')]:
                (root / name).write_text(wrong)
                with self.subTest(name=name), self.assertRaises(ValueError):
                    controller.validate_cgroup_limits(root, [0, 1])
                (root / name).write_text(files[name])
            (root / 'cpu.max').unlink()
            with self.assertRaises(FileNotFoundError):
                controller.validate_cgroup_limits(root, [0, 1])

    def test_docker_arguments_cannot_be_candidate_controlled(self):
        for image, name, cpus, compiler in [('evil', 'ci-bench-001-0', [0, 1], '14'),
                                           ('sha256:' + 'a' * 64, '--privileged', [0, 1], '14'),
                                           ('sha256:' + 'a' * 64, 'ci-bench-001-0', [0], '14'),
                                           ('sha256:' + 'a' * 64, 'ci-bench-001-0', [0, 1], '--env')]:
            with self.assertRaises(ValueError):
                controller.docker_command(image, Path('/tmp/source'), name, cpus, compiler, 'reuse')

    def test_host_headroom_required(self):
        controller.validate_host_resources(8 * 1024**3, 10 * 1024**3)
        for memory, disk in [(7 * 1024**3, 10 * 1024**3), (8 * 1024**3, 9 * 1024**3)]:
            with self.assertRaises(ValueError):
                controller.validate_host_resources(memory, disk)

    def test_candidate_modules_only_inside_worker(self):
        host = (ci_selection.ROOT / 'scripts/ci_bench/controller.py').read_text()
        self.assertNotIn('importlib', host)
        self.assertNotIn('sys.path.insert', host)
        worker = (ci_selection.ROOT / 'scripts/ci_bench/worker.py').read_text()
        self.assertIn("sys.path.insert(0, '/source/scripts')", worker)
        self.assertIn('PortablePilot(scratch, compiler, checked, jobs=contract.WORKERS, traced=False)', worker)

    def test_upload_receipts_source_attempt_and_preflight_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, RUNNER_TEMP=directory,
                GITHUB_RUN_ID='123', GITHUB_RUN_ATTEMPT='2', RECEIPT_OUTCOME='success',
                GITHUB_JOB=contract.JOBS['14'], BENCH_COMPILER='14'):
            base = controller.work()
            identity = dict(source_commit=contract.SOURCE_COMMIT, source_tree=contract.SOURCE_TREE,
                            run_id='123', run_attempt='2', job=contract.JOBS['14'], job_id='789', compiler='14')
            runs = rows()[:4]
            result = dict(valid=True, completed=True, identity=identity, runs=runs,
                          budget=contract.budget(), allocation=runs[0]['allocation'], performance_success=False,
                          host_resources=dict(available_memory_bytes=8 * 1024**3, free_disk_bytes=10 * 1024**3),
                          preflight=dict(external_busy_fraction=.01, steal_fraction=0, elapsed_seconds=10))
            transport = dict(outcome='success', artifact_id='456', artifact_digest='a' * 64,
                             run_id='123', run_attempt='2', job=contract.JOBS['14'], job_id='789', compiler='14')
            controller.write(base / 'identity.json', identity)
            controller.write(base / 'public/result.json', result)
            controller.write(base / 'upload-receipt.json', transport)
            controller.gate()
            for key, value in [('artifact_id', ''), ('artifact_digest', 'bad'), ('run_id', '122'),
                               ('run_attempt', '1'), ('outcome', 'failure')]:
                controller.write(base / 'upload-receipt.json', dict(transport, **{key: value}))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    controller.gate()
            controller.write(base / 'upload-receipt.json', transport)
            with patch.dict(os.environ, RECEIPT_OUTCOME='failure'), self.assertRaises(ValueError):
                controller.gate()
            result['preflight']['external_busy_fraction'] = .06
            controller.write(base / 'public/result.json', result)
            with self.assertRaises(ValueError):
                controller.gate()

    def test_adversarial_cells_scope_resources_and_sampling(self):
        mutations = [lambda r: r[0]['candidate_report'].update(measurement_scope='PERF-005'),
                     lambda r: r[0]['candidate_report'].update(source_commit='a' * 40),
                     lambda r: r[0]['candidate_report']['records'].pop(),
                     lambda r: r[0]['candidate_report']['records'].__setitem__(0, r[0]['candidate_report']['records'][1]),
                     lambda r: r[0]['candidate_report']['builds'][0].update(bind=False),
                     lambda r: r[0]['candidate_report']['builds'][0].update(link=False),
                     lambda r: r[0]['candidate_report']['builds'][0].update(execute_marker='checks= 6'),
                     lambda r: r[0]['candidate_report']['builds'][0].update(source_sha256='missing'),
                     lambda r: r[0]['candidate_report']['builds'][0].update(source_sha256='e' * 64),
                     lambda r: r[0]['candidate_report']['builds'][0]['units'].pop(),
                     lambda r: r[0].update(oom=True), lambda r: r[0].update(exit_code=124),
                     lambda r: r[0].update(cpu_seconds=float('nan')),
                     lambda r: r[0].update(cpu_seconds=9),
                     lambda r: r[0]['candidate_report'].update(generator_build_seconds=0),
                     lambda r: r[0].update(wall_seconds=contract.ITERATION_SECONDS + 4),
                     lambda r: r[0].update(wall_seconds=5),
                     lambda r: r[0]['samples'][0].update(at_seconds=4),
                     lambda r: r[0]['samples'][0].update(throttled_usec=10001),
                     lambda r: r[0]['samples'][0].update(usage_usec=-1),
                     lambda r: r[0]['allocation'].update(guest_core_count=2),
                     lambda r: r[0]['allocation'].update(workers=1)]
        for index, mutate in enumerate(mutations):
            value = rows()
            mutate(value)
            with self.subTest(index=index), self.assertRaises(ValueError):
                controller.validate_rows(value)

    def test_binder_metadata_not_counted_as_source_compilation(self):
        self.assertEqual(compiled_units(b'[Ada] unit.adb\n[Ada] programs.ads\n'
                         b'[C] memory.c\n[Ada] b__main.adb\n[Ada] main.ali\n'),
                         [b'unit.adb', b'programs.ads', b'memory.c'])

    def test_complete_two_compiler_abba(self):
        self.assertTrue(controller.validate_rows(rows()))
        self.assertTrue(controller.validate_rows(rows()[:4], '14'))
        self.assertTrue(controller.validate_rows(rows()[4:], '16'))
        for data, compiler in [(rows(), '14'), (rows()[:4], '16'), (rows()[:3], '14')]:
            with self.assertRaises(ValueError):
                controller.validate_rows(data, compiler)

    def test_missing_order_identity_counts_trace_and_contention_fail(self):
        cases = [lambda r: r.pop(), lambda r: r.reverse(),
                 lambda r: r[0]['candidate_report'].update(counts={}),
                 lambda r: r[0]['candidate_report'].update(generator_sha256='c' * 64),
                 lambda r: r[0]['candidate_report'].update(seed_sha256='c' * 64),
                 lambda r: r[0]['candidate_report'].update(dependency_identity_sha256='c' * 64),
                 lambda r: r[0]['candidate_report'].update(instrumented=True),
                 lambda r: r[0]['candidate_report'].update(jobs=1),
                 lambda r: r[0]['candidate_report'].update(completed=False),
                 lambda r: r[0].update(samples=[]), lambda r: r[0].update(cpu_seconds=0),
                 lambda r: r[0].update(cgroup_peak_memory_bytes=0),
                 lambda r: r[0]['samples'][0].update(external_busy_fraction=.06),
                 lambda r: r[0]['samples'][0].update(steal_fraction=.02),
                 lambda r: r[0]['samples'][0].update(steal_fraction=float('nan')),
                 lambda r: r[0]['samples'][0].update(elapsed_seconds=4)]
        for index, mutate in enumerate(cases):
            value = rows()
            mutate(value)
            with self.subTest(index=index), self.assertRaises(ValueError):
                controller.validate_rows(value)

    def test_cpu_accounting_subtracts_only_cgroup_work(self):
        before = controller.cpu_stat('cpu0 0 0 0 100 0 0 0 0\ncpu1 0 0 0 100 0 0 0 0')
        after = controller.cpu_stat('cpu0 100 0 0 100 0 0 0 0\ncpu1 0 0 0 200 0 0 0 0')
        with patch.object(os, 'sysconf', return_value=100):
            sample = controller.sample_delta(before, after, [0, 1], 1000000)
        self.assertEqual(sample['external_busy_fraction'], 0)
        self.assertEqual(controller.sample_delta(before, after, [0, 1])['external_busy_fraction'], 1)
        with self.assertRaises(ValueError):
            controller.sample_delta(before, after, [0, 1], elapsed=4)
        self.assertEqual(controller.cpu_set('0-2,4'), {0, 1, 2, 4})

    def test_failure_packaging_and_upload_failure_cannot_pass(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, RUNNER_TEMP=directory):
            controller.package()
            base = controller.work()
            self.assertFalse(json.loads((base / 'public/result.json').read_text())['valid'])
            with patch.dict(os.environ, MEASUREMENTS_OUTCOME='failure'):
                controller.receipt()
            with self.assertRaises(ValueError):
                controller.gate()


if __name__ == '__main__':
    unittest.main()