#!/usr/bin/env python3
"""Job-local mutation regression, including overlapping independent setup."""
import importlib.util
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("layout", ROOT / "scripts/check-task077b-ci-layout.py")
layout = importlib.util.module_from_spec(spec)
spec.loader.exec_module(layout)


class MutationOwnership(unittest.TestCase):
    def test_exact_evidence_exceptions(self):
        from real_uci_evidence import evidence_steps
        original = (ROOT / '.github/workflows/deep-ci.yml').read_text()
        independent = '''  independent-evidence-regression:
    runs-on: ubuntu-latest
    steps:
      - name: Overlapping independent setup
        run: |
          cargo fetch --locked
          cargo build --release --locked -p ams-gra-codegen-oms
'''
        negatives = positives = 0
        for text in (original, original.replace('jobs:\n', 'jobs:\n' + independent, 1)):
            with tempfile.TemporaryDirectory() as tmp:
                path = pathlib.Path(tmp) / 'deep.yml'
                path.write_text(text)
                self.assertEqual(layout.check(path), [])
                positives += 1
                for job in (*layout.PRIMARY_SHARDS, 'real-uci-regressions'):
                    match = re.search(r'^  ' + re.escape(job) + r':\n.*?(?=^  [\w-]+:|\Z)',
                                      text, re.M | re.S)
                    body = match[0]
                    evidence = evidence_steps(job) if job in layout.PRIMARY_SHARDS else layout.EVIDENCE
                    self.assertEqual(body.count(evidence), 1)
                    conditional = ('      - name: Reconcile complete shard evidence\n'
                                   '        if : false\n        run: echo unauthorized\n')
                    variants = [
                        ('before', conditional + evidence, 'enabled hard gate: '),
                        ('after', evidence + conditional, 'enabled hard gate: '),
                        ('duplicate', evidence + evidence, 'exact evidence exception: '),
                        ('action', evidence.replace('actions/upload-artifact@v4',
                                                    'actions/upload-artifact@v4.6.2'),
                         'exact evidence exception: '),
                        ('spaced condition', evidence.replace('if:', 'if :', 1),
                         'exact evidence exception: '),
                        ('quoted condition', evidence.replace('if:', "'if':", 1),
                         'exact evidence exception: '),
                        ('optional evidence', evidence.replace('        if: always()',
                            '        continue-on-error : true\n        if: always()', 1),
                         'exact evidence exception: '),
                        ('extra property', evidence.rstrip() + '\n        continue-on-error: true\n',
                         'exact evidence exception: '),
                    ]
                    if job in layout.PRIMARY_SHARDS:
                        variants.append(('command', evidence.replace(
                            'real_uci_evidence.py finish', 'real_uci_evidence.py init'),
                            'exact evidence exception: '))
                    for label, replacement, diagnostic in variants:
                        with self.subTest(job=job, mutation=label):
                            changed = body.replace(evidence, replacement, 1)
                            self.assertNotEqual(changed, body)
                            path.write_text(text[:match.start()] + changed + text[match.end():])
                            self.assertIn('Task077B ' + diagnostic + job, layout.check(path))
                            negatives += 1
                    # Add conditions to an existing mandatory qualification,
                    # not the evidence or immutable source-verification steps.
                    name = next(name for name, owner, _ in layout.GATES if owner == job)
                    target = '      - name: ' + name + '\n'
                    self.assertEqual(body.count(target), 1)
                    for field in ('if : false', "'if': false", 'continue-on-error : true'):
                        changed = body.replace(target, target + '        ' + field + '\n', 1)
                        path.write_text(text[:match.start()] + changed + text[match.end():])
                        self.assertIn('Task077B enabled hard gate: ' + job, layout.check(path))
                        negatives += 1
        self.assertEqual(positives, 2)
        self.assertEqual(negatives, 94)

    def test_optimization_environment_is_owner_scoped(self):
        original = (ROOT / '.github/workflows/deep-ci.yml').read_text()
        expected = "    env:\n      CARGO_PROFILE_DEV_BUILD_OVERRIDE_OPT_LEVEL: '3'\n"
        self.assertEqual(layout.OPTIMIZATION_JOBS, ('real-sleet', 'msrv-real-uci'))
        self.assertEqual(layout.OPTIMIZATION_ENV, expected)
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / 'deep.yml'
            path.write_text(original)
            self.assertEqual(layout.check(path), [])
            for job in layout.SOURCE_JOBS:
                with self.subTest(job=job):
                    match = re.search(r'^  ' + re.escape(job) + r':\n.*?(?=^  [\w-]+:|\Z)',
                                      original, re.M | re.S)
                    body = match[0]
                    if job in ('real-sleet', 'msrv-real-uci'):
                        header = body.split('    steps:\n', 1)[0]
                        environments = re.findall(r'^    env:\n.*?(?=^    \S|\Z)',
                                                  header, re.M | re.S)
                        self.assertEqual(len(environments), 1)
                        changed = body.replace(environments[0], expected, 1)
                    else:
                        changed = body.replace('    runs-on:', expected + '    runs-on:', 1)
                        self.assertNotEqual(changed, body)
                    path.write_text(original[:match.start()] + changed + original[match.end():])
                    if job in ('real-sleet', 'msrv-real-uci'):
                        self.assertEqual(layout.check(path), [])
                    else:
                        self.assertIn('Task077B mandatory source identity: ' + job,
                                      layout.check(path))

    def test_job_environment_mutation_preserves_steps(self):
        original = (ROOT / '.github/workflows/deep-ci.yml').read_text()
        for job in ('real-sleet', 'msrv-real-uci'):
            body = re.search(r'^  ' + re.escape(job) + r':\n.*?(?=^  [\w-]+:|\Z)',
                             original, re.M | re.S)[0]
            header, steps = body.split('    steps:\n', 1)
            if job == 'real-sleet':
                self.assertIn('        env:\n', steps)
            self.assertEqual(len(re.findall(r'^    env:\n', header, re.M)), 1)
            for spelling in ('{} :', '{}   :', "'{}':", '"{}":', "'{}' :", '"{}"   :'):
                with self.subTest(job=job, spelling=spelling):
                    changed = layout.mutate_job_environment(body, spelling)
                    changed_header, changed_steps = changed.split('    steps:\n', 1)
                    self.assertEqual(changed_steps, steps)
                    self.assertEqual(changed_header, header.replace(
                        '\n    env:\n', '\n    ' + spelling.format('env')
                        + '\n      GITHUB_SHA: main\n', 1))
            for invalid_header in (header.replace('    env:\n', '    other:\n'),
                                   header + '    env:\n      EXTRA: value\n',
                                   header.replace('    env:\n', '     env:\n')):
                with self.subTest(job=job), self.assertRaisesRegex(AssertionError, 'job environment'):
                    layout.mutate_job_environment(invalid_header + '    steps:\n' + steps, '{} :')

    def test_protected_block_key_spellings(self):
        for key in ('if', 'continue-on-error', 'env', 'defaults', 'needs'):
            for spelling in (key + ':', key + ' :', key + '   :',
                             "'" + key + "':", '"' + key + '":',
                             "'" + key + "' :", '"' + key + '"   :'):
                for prefix in ('    ', '        ', '      - ', ''):
                    with self.subTest(key=key, spelling=spelling, prefix=prefix):
                        self.assertTrue(layout.has_key(prefix + spelling + ' false', key))
            for text in ('    # ' + key + ' : false',
                         '    not-' + key + ' : false',
                         '    name: ' + key + ' : false',
                         '    ' + key + '-suffix: false'):
                self.assertFalse(layout.has_key(text, key), text)

    def test_step_environment_spelling_does_not_disable_qualification(self):
        # GITHUB_* defaults cannot be overwritten by env assignments (GitHub
        # variables reference). This is not an if/continue-on-error bypass:
        # canonical env: is accepted too. Do not ban legitimate step env maps
        # merely to turn every exploratory trial into a negative mutation.
        original = (ROOT / '.github/workflows/deep-ci.yml').read_text()
        for job, name in (
                ('real-uci-regressions-alternating', layout.GATES[-1][0]),
                ('real-sleet', 'Real UCI 2.5 codecs (Tasks 050/052, must execute)')):
            match = re.search(r'^  ' + re.escape(job) + r':\n.*?(?=^  [\w-]+:|\Z)',
                              original, re.M | re.S)
            target = '      - name: ' + name + '\n'
            self.assertEqual(match[0].count(target), 1)
            for spelling in ('{}:', '{} :', '{}   :', "'{}':", '"{}":',
                             "'{}' :", '"{}"   :'):
                changed_body = match[0].replace(target, target + '        '
                    + spelling.format('env') + '\n          GITHUB_SHA: main\n', 1)
                changed = original[:match.start()] + changed_body + original[match.end():]
                with self.subTest(job=job, spelling=spelling), tempfile.TemporaryDirectory() as tmp:
                    path = pathlib.Path(tmp) / 'deep.yml'
                    path.write_text(changed)
                    self.assertEqual(layout.check(path), [])
                    self.assertEqual(changed[:match.start()], original[:match.start()])
                    self.assertTrue(changed.endswith(original[match.end():]))

    def test_only_exact_evidence_upload_is_exempt(self):
        original = (ROOT / ".github/workflows/deep-ci.yml").read_text()
        for changed in [
                original.replace('if: always()', 'if: false'),
                original.replace('name: Preserve Task 061 per-selection evidence', 'name: Some gate'),
                original.replace('name: Preserve Task 061 per-selection evidence\n',
                                 'name: Preserve Task 061 per-selection evidence\n        continue-on-error: true\n'),
                original.replace('name: task061-service-check\n', 'name: foreign-artifact\n')]:
            with tempfile.TemporaryDirectory() as tmp:
                path = pathlib.Path(tmp) / 'deep.yml'
                path.write_text(changed)
                self.assertIn('Task077B enabled hard gate: real-uci-regressions', layout.check(path))

    def test_actual_task092_and_overlapping_predecessor(self):
        original = (ROOT / ".github/workflows/deep-ci.yml").read_text()
        self.assertLess(original.index("  ada-repeated-choice-real-uci:"),
                        original.index("  real-uci-temporal-naming:"))
        independent = '''  independent-layout-regression:
    runs-on: ubuntu-latest
    steps:
      - name: Independent setup with overlapping commands
        run: |
          UCI25="$(scripts/fetch-pinned-uci-2.5.sh "$RUNNER_TEMP/independent25")"
          UCI26="$(scripts/fetch-pinned-uci-2.6.sh "$RUNNER_TEMP/independent26")"
          cargo build --release --locked -p ams-gra-codegen-oms
          cargo fetch --locked
'''
        for text in [original, original.replace("jobs:\n", "jobs:\n" + independent, 1)]:
            with self.subTest(prepended=text != original), tempfile.TemporaryDirectory(
                    prefix="task077b-owner-") as tmp:
                path = pathlib.Path(tmp) / "deep.yml"
                path.write_text(text)
                self.assertEqual(layout.check(path), [])
                actual_run = subprocess.run
                observed = []
                outcomes = []

                def inspect_mutation(command, **kwargs):
                    mutated = pathlib.Path(command[2]).read_text()
                    jobs = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)",
                                           text, re.M | re.S))
                    changed = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)",
                                              mutated, re.M | re.S))
                    for job in jobs.keys() - set(layout.SOURCE_JOBS):
                        self.assertEqual(changed.get(job), jobs[job], job)
                    changed_jobs = {job for job in jobs.keys() | changed.keys()
                                    if changed.get(job) != jobs.get(job)}
                    # The inherited separated-Task060 fixture intentionally
                    # changes two owners; every other job mutation is local.
                    self.assertLessEqual(len(changed_jobs), 2)
                    observed.append(mutated)
                    result = actual_run(command, **kwargs)
                    outcomes.append(result.returncode)
                    return result

                # The expanded sharding self-test still runs the real
                # guard and checks each attributed diagnostic and non-no-op.
                from unittest.mock import patch
                with patch.object(layout.subprocess, "run", side_effect=inspect_mutation):
                    layout.self_test(str(path))
                # Every primary shard now receives the same source/setup
                # mutations as each independent regression runner.
                # 373 inherited negatives + 876 equivalent-key negatives
                # + nine MSRV optimization-environment negatives;
                # the original and five semantics-preserving controls.
                self.assertEqual(len(observed), 1264)
                self.assertEqual(outcomes.count(0), 6)
                self.assertEqual(sum(code != 0 for code in outcomes), 1258)
                self.assertEqual(observed.count(text), 1)  # Healthy case only.

    def test_source_gate_executes_fail_closed(self):
        # Execute the exact inline workflow command in a disposable repository:
        # event type changes must not relax the immutable-SHA assertion.
        with tempfile.TemporaryDirectory(prefix='deep-ci-source-') as tmp:
            root = pathlib.Path(tmp)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / 'source.txt').write_text('disposable identity fixture\n')
            subprocess.run(['git', '-C', str(root), 'add', 'source.txt'], check=True)
            subprocess.run(['git', '-C', str(root), '-c', 'user.name=Source Gate Test',
                            '-c', 'user.email=source-gate@example.invalid', 'commit', '-qm',
                            'Disposable source identity'], check=True)
            actual = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'],
                                             text=True).strip()
            tree = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD^{tree}'],
                                           text=True).strip()
            for event in ('pull_request', 'push', 'schedule', 'workflow_dispatch'):
                for expected, success in ((actual, True), ('0' * 40, False)):
                    with self.subTest(event=event, matching=success):
                        env = dict(os.environ, GITHUB_SHA=expected, GITHUB_EVENT_NAME=event)
                        result = subprocess.run(['bash', '-c', layout.SOURCE_COMMAND], cwd=root,
                                                env=env, capture_output=True, text=True)
                        self.assertEqual(result.returncode == 0, success, result.stderr)
                        if success:
                            self.assertEqual(result.stdout.splitlines(), [
                                'DEEP_CI_SOURCE_SHA=' + actual, 'DEEP_CI_SOURCE_TREE=' + tree,
                                'DEEP_CI_EVENT=' + event])
                        else:
                            self.assertNotIn('DEEP_CI_SOURCE_SHA=', result.stdout)
            result = subprocess.run(['bash', '-c', layout.SOURCE_COMMAND], cwd=root.parent,
                                    env=dict(os.environ, GITHUB_SHA=actual, GITHUB_EVENT_NAME='push'),
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            # Git resolving HEAD but failing tree lookup must also abort.
            tools = root / 'tools'
            tools.mkdir()
            git = tools / 'git'
            git.write_text('#!/bin/sh\nif [ "$2" = HEAD ]; then echo "' + actual
                           + '"; else exit 23; fi\n')
            git.chmod(0o755)
            for command in (git.read_text(), git.read_text().replace('exit 23', 'exit 0')):
                git.write_text(command)
                result = subprocess.run(['bash', '-c', layout.SOURCE_COMMAND], cwd=root,
                                        env=dict(os.environ, PATH=str(tools) + ':' + os.environ['PATH'],
                                                 GITHUB_SHA=actual, GITHUB_EVENT_NAME='push'),
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn('DEEP_CI_SOURCE_SHA=', result.stdout)


if __name__ == "__main__":
    unittest.main()