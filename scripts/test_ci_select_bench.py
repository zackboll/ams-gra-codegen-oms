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
from ci_bench import controller, policy
from ci_bench.worker import compiled_units


class Registration(unittest.TestCase):
    def setUp(self):
        self.inventory = ci_selection.inventory()
        self.workflows = {p.name: p.read_text() for p in
                          (ci_selection.ROOT / '.github/workflows').glob('*.yml')}

    def test_registered_manual_job_does_not_expand_main_or_pr(self):
        self.assertEqual(ci_policy.check(self.inventory, self.workflows), 40)
        self.assertEqual(self.inventory['manual_benchmark']['lane'], 3)
        for full in (False, True):
            selection = ci_selection.select([], self.inventory, full_main=full)
            self.assertNotIn(policy.JOB, selection['jobs'])
        self.assertEqual(self.inventory['lanes'], [f'oms-main-slot-{n}' for n in range(1, 6)])

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
            ('timeout-minutes: 180', 'timeout-minutes: 360'),
            ('python3 scripts/ci_bench/controller.py campaign', 'bash candidate/scripts/run.sh'),
            ('SOURCE_SHA: ${{ inputs.source_sha }}', 'TOKEN: ${{ secrets.TOKEN }}'),
            ('if-no-files-found: error', 'if-no-files-found: warn'),
            ('if: always()', 'if: success()'),
            ('source_sha:', 'source_ref:'),
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
        command = controller.docker_command('sha256:' + 'a' * 64, Path('/tmp/candidate'), 'test', 1, '14', 'original')
        for required in ('--network=none', '--read-only', '--cap-drop=ALL',
                         '--security-opt=no-new-privileges', '--user=10001:10001',
                         '--pids-limit=256', '--memory=6g', '--memory-swap=6g', '--cpus=1', '--cpuset-cpus=1'):
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


def rows():
    result = []
    for compiler, mode in controller.ORDER:
        report = dict(completed=True, compiler=compiler, mode=mode, jobs=1, instrumented=False,
                      generator_sha256='a' * 64, seed_sha256='b' * 64, dependency_identity_sha256=compiler * 32,
                      counts=dict(provider=3180 if mode == 'original' else 318,
                                  generated_support=150, bind=30, link=30, execute=30))
        result.append(dict(compiler=compiler, mode=mode, candidate_report=report,
                           cpu_seconds=10, cgroup_peak_memory_bytes=1024,
                           samples=[dict(external_busy_fraction=.01, steal_fraction=0, elapsed_seconds=1)]))
    return result


class Measurement(unittest.TestCase):
    def test_binder_metadata_not_counted_as_source_compilation(self):
        self.assertEqual(compiled_units(b'[Ada] unit.adb\n[Ada] programs.ads\n'
                         b'[C] memory.c\n[Ada] b__main.adb\n[Ada] main.ali\n'),
                         [b'unit.adb', b'programs.ads', b'memory.c'])

    def test_complete_two_compiler_abba(self):
        self.assertTrue(controller.validate_rows(rows()))

    def test_missing_order_identity_counts_trace_and_contention_fail(self):
        cases = [lambda r: r.pop(), lambda r: r.reverse(),
                 lambda r: r[0]['candidate_report'].update(counts={}),
                 lambda r: r[0]['candidate_report'].update(generator_sha256='c' * 64),
                 lambda r: r[0]['candidate_report'].update(seed_sha256='c' * 64),
                 lambda r: r[0]['candidate_report'].update(dependency_identity_sha256='c' * 64),
                 lambda r: r[0]['candidate_report'].update(instrumented=True),
                 lambda r: r[0]['candidate_report'].update(jobs=2),
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