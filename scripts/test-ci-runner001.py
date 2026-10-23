#!/usr/bin/env python3
"""Manual-only routing contract and measurement-shell regression tests.

These are regression checks on the intended workflow, NOT a security boundary
against a PR replacing this workflow or these tests. Isolation is external.
"""
import itertools
import ast
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SELF_HOSTED = ['self-hosted', 'linux', 'x64', 'ryzen7945hx-test', 'oms-ci-trusted']
ROUTE = ("fromJSON(github.event_name == 'workflow_dispatch' && "
         "inputs.ryzen_qualification == true && "
         "vars.OMS_RYZEN_MANUAL_APPROVED == 'true' && "
         "'[\"self-hosted\",\"linux\",\"x64\",\"ryzen7945hx-test\",\"oms-ci-trusted\"]' "
         "|| '[\"ubuntu-24.04\"]')")


def float_job(text):
    return re.search(r'^  ada-float32-codec:\n(.*?)(?=^  [\w-]+:|\Z)',
                     text, re.M | re.S).group(1)


def route(text, event, requested, approved):
    expression = re.search(r'^    runs-on: \$\{\{ (.*?) \}\}$',
                           float_job(text), re.M).group(1)
    if expression != ROUTE:
        raise ValueError('Manual-only runner routing contract changed')
    # Evaluate the ACTUAL expression after substituting context values. Test
    # all event/input/variable combinations, not an independent routing model.
    expression = expression.replace('github.event_name', repr(event))
    expression = expression.replace('inputs.ryzen_qualification', repr(requested))
    expression = expression.replace('vars.OMS_RYZEN_MANUAL_APPROVED', repr(approved))
    expression = expression.replace(' == true', ' == True')
    expression = expression.replace(' && ', ' and ').replace(' || ', ' or ')
    return eval(expression, {'__builtins__': {}, 'fromJSON': json.loads})


class RunnerRouting(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = (ROOT / '.github/workflows/ci.yml').read_text()
        cls.job = float_job(cls.text)

    def test_event_input_approval_matrix(self):
        cases = itertools.product(
            ['pull_request', 'pull_request_target', 'push', 'schedule', 'workflow_dispatch'],
            [False, True, '', None], ['', 'false', 'true', 'TRUE'])
        for event, requested, approved in cases:
            expected = (SELF_HOSTED if event == 'workflow_dispatch'
                        and requested is True and approved == 'true' else ['ubuntu-24.04'])
            with self.subTest(event=event, requested=requested, approved=approved):
                self.assertEqual(route(self.text, event, requested, approved), expected)

    def test_fork_same_repo_and_untrusted_dispatch_defaults_stay_hosted(self):
        for origin in ['fork', 'same-repository']:
            with self.subTest(origin=origin):
                self.assertEqual(route(self.text, 'pull_request', True, 'true'), ['ubuntu-24.04'])
        self.assertEqual(route(self.text, 'workflow_dispatch', True, ''), ['ubuntu-24.04'])
        self.assertRegex(self.text, r'type: boolean\n        default: false')
        self.assertNotIn('pull_request_target:', self.text)
        self.assertIn('permissions:\n  contents: read\n', self.text)
        self.assertNotIn('secrets.', self.job)
        self.assertIn('persist-credentials: false', self.job)

    def test_routing_mutations_are_rejected(self):
        for clause in ["github.event_name == 'workflow_dispatch' && ",
                       'inputs.ryzen_qualification == true && ',
                       "vars.OMS_RYZEN_MANUAL_APPROVED == 'true' && "]:
            with self.subTest(clause=clause), self.assertRaises(ValueError):
                route(self.text.replace(clause, '', 1), 'pull_request', True, 'true')

    def test_only_one_job_has_self_hosted_selection(self):
        jobs = re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)',
                          self.text.split('jobs:\n', 1)[1], re.M | re.S)
        for name, body in jobs:
            with self.subTest(job=name):
                if name == 'ada-float32-codec':
                    self.assertIn('self-hosted', body)
                else:
                    self.assertNotIn('self-hosted', body)
                    self.assertRegex(body, r'(?m)^    runs-on: ubuntu-(24\.04|latest)$')
        deep = (ROOT / '.github/workflows/deep-ci.yml').read_text()
        self.assertNotIn('self-hosted', deep)

    def test_complete_workload_and_measurement_retained(self):
        for required in [
                'debian@sha256:bcf83fd8af414eda0c0a99d25a4e61dedd88e7aac7f7bc519d04555ff0c40ec5',
                'dtolnay/rust-toolchain@1.95.0', 'timeout-minutes: 90',
                "CARGO_BUILD_JOBS: '4'", 'shell: bash scripts/ci-runner001-measure.sh {0}',
                "OMS_CI_ADA_BUILD_JOBS: '4'",
                'test "$SELECTED_RUNNER" = ryzen7945hx-01',
                'test "$(git -c safe.directory="$GITHUB_WORKSPACE" rev-parse HEAD)" = "$EXPECTED_SHA"',
                'scripts/check-task095.py', 'scripts/check-task095-rounding.py',
                'scripts/check-task093-mixed.py', 'scripts/check-task093-helper-demand.py',
                'for compiler in 14 16; do', 'for profile in oam qualified; do',
                'if: always()', 'name: task095-platform-qualified-float32',
                '/tmp/task095-evidence/task093-c5-portable-14/**',
                '/tmp/task095-evidence/ci-runner001/**', 'if-no-files-found: error']:
            with self.subTest(required=required):
                self.assertIn(required, self.job)

    def test_ada_build_commands_use_bounded_worker_setting(self):
        for name in ['check-task095.py', 'check-task093-mixed.py',
                     'check-task093-helper-demand.py']:
            source = (ROOT / 'scripts' / name).read_text()
            tree = ast.parse(source)
            commands = [node for node in ast.walk(tree) if isinstance(node, ast.List)
                        and len(node.elts) >= 5
                        and all(isinstance(item, ast.Constant) for item in node.elts[:5])
                        and [item.value for item in node.elts[:5]] == ['alr', '-n', 'build', '--', '-f']]
            with self.subTest(script=name):
                self.assertEqual(len(commands), 1)
                expression = ast.Expression(commands[0])
                for jobs in [1, 4]:
                    command = eval(compile(expression, name, 'eval'),
                                   {'__builtins__': {}, 'build_jobs': jobs})
                    self.assertEqual(command[-1], f'-j{jobs}')
                self.assertIn('if not 1 <= build_jobs <= 4:', source)
                self.assertIn('os.environ.get("OMS_CI_ADA_BUILD_JOBS", "1")', source)

    def test_invalid_workers_fail_before_creating_workload(self):
        with tempfile.TemporaryDirectory(prefix='ci-runner001-workers-') as temporary:
            for name in ['check-task095.py', 'check-task093-mixed.py',
                         'check-task093-helper-demand.py']:
                for workers in ['0', '5']:
                    scratch = Path(temporary) / f'{name}-{workers}'
                    command = [sys.executable, str(ROOT / 'scripts' / name),
                               '--compiler', '14', '--generator', '/does-not-exist',
                               '--scratch', str(scratch)]
                    if name == 'check-task095.py':
                        command += ['--profile', 'oam']
                    if name == 'check-task093-helper-demand.py':
                        command += ['--family', 'portable']
                    result = subprocess.run(command, capture_output=True, text=True,
                                            env={**os.environ, 'OMS_CI_ADA_BUILD_JOBS': workers})
                    with self.subTest(script=name, workers=workers):
                        self.assertEqual(result.returncode, 2, result.stderr)
                        self.assertIn('must be between 1 and 4', result.stderr)
                        self.assertFalse(scratch.exists())


class MeasurementShell(unittest.TestCase):
    def test_success_failure_and_pipefail_with_metrics(self):
        with tempfile.TemporaryDirectory(prefix='ci-runner001-test-') as temporary:
            root = Path(temporary)
            wrapper = root / 'measure.sh'
            source = (ROOT / 'scripts/ci-runner001-measure.sh').read_text()
            wrapper.write_text(source.replace('evidence=/tmp/task095-evidence/ci-runner001',
                                             f'evidence={root}/evidence'))
            for command, expected in [('printf smoke-ok', 0), ('exit 7', 7),
                                      ('false | true', 1), ('false; echo unreachable', 1)]:
                script = root / 'step.sh'
                script.write_text(command + '\n')
                result = subprocess.run(['bash', str(wrapper), str(script)],
                                        capture_output=True, text=True, env=os.environ)
                with self.subTest(command=command):
                    self.assertEqual(result.returncode, expected, result.stderr)
                    self.assertIn(f'exit_status={expected}', result.stdout)
                    self.assertNotIn('unreachable', result.stdout)
            records = list((root / 'evidence').glob('step-*'))
            self.assertEqual(len(records), 4)
            for record in records:
                for name in ['command.sh', 'started.txt', 'completed.txt',
                             'memory-before.txt', 'memory-after.txt', 'resources.txt']:
                    self.assertGreater((record / name).stat().st_size, 0)
                self.assertIn('MemAvailable:', (record / 'memory-before.txt').read_text())
                self.assertIn('SwapFree:', (record / 'memory-after.txt').read_text())
                self.assertIn('max_process_rss_kib=', (record / 'resources.txt').read_text())


if __name__ == '__main__':
    unittest.main()