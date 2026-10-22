#!/usr/bin/env python3
"""Scheduler-only fault injection; never substitutes for pinned production checks."""
from contextlib import redirect_stderr, redirect_stdout
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("task062_scheduler", REPO / "scripts/check-task062-service-impact.py")
scheduler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scheduler)

# Real subprocesses read the actual per-selection contracts, exercise both streams,
# and delay selected jobs to force completion out of scheduling order.
FAKE = '''#!/usr/bin/env python3
import os
from pathlib import Path
import signal
import sys
import time
args = sys.argv
contract = Path(args[args.index('--contract') + 1])
text = contract.read_text()
index = int(contract.parent.name)
mode = os.environ.get('TASK062_FAKE_MODE', '')
if index == 0:
    time.sleep(.15)
if mode == 'timeout':
    time.sleep(2)
if mode == 'signal' and index == 3:
    os.kill(os.getpid(), signal.SIGTERM)
if mode == 'failure' and index == 3:
    print('injected diagnostic', file=sys.stderr, flush=True)
    sys.exit(23)
if 'name: task062' not in text or 'message: ' not in text:
    sys.exit(24)
language = args[args.index('--language') + 1]
world = args[args.index('--world') + 1]
report = (f'service contract valid\\nservice: task062\\nlanguage: {language}\\n'
          f'generation world: {world}\\n\\nselected oms messages: 1\\n'
          'renderable selected oms messages: 1\\nselected type closure: 56\\n'
          'renderable selected types: 56\\nstatus: READY\\n')
if index == 3:
    if mode == 'malformed': report = 'READY\\n'
    if mode == 'not-ready': report = report.replace('status: READY', 'status: NOT READY')
    if mode == 'truncated': report = report.removesuffix('status: READY\\n')
    if mode == 'wrong-language': report = report.replace('language: ada', 'language: rust')
    if mode == 'counts': report = report.replace('renderable selected types: 56', 'renderable selected types: 55')
    if mode == 'extra-output': report += 'unexpected\\n'
sys.stdout.write(report)
print('deterministic diagnostic', file=sys.stderr)
'''


class SchedulerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="task062-scheduler-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.fake = self.root / "cli"
        self.fake.write_text(FAKE)
        self.fake.chmod(0o755)
        schema = self.root / "schema.xsd"
        schema.write_text("scheduler-only dummy schema")
        self.env = patch.dict(os.environ, {
            "AMS_GRA_UCI_2_5_ROOT": str(schema), "AMS_GRA_UCI_2_6_ROOT": str(schema),
            "TMPDIR": str(self.root), "TASK062_FAKE_MODE": ""})
        self.env.start()
        self.addCleanup(self.env.stop)
        self.counter = 0

    def prepare(self):
        self.counter += 1
        scratch = self.root / f"campaign-{self.counter}"
        scratch.mkdir()
        return scheduler.prepare(self.fake, scheduler.selections(), scratch), scratch

    def run_campaign(self, workers=4, mode=""):
        items, scratch = self.prepare()
        os.environ["TASK062_FAKE_MODE"] = mode
        output = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")
        diagnostics = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")
        with redirect_stdout(output), redirect_stderr(diagnostics):
            scheduler.campaign(items, workers, scratch)
        output.seek(0)
        return items, scratch, output.read()

    def test_delayed_workers_order_isolation_and_equivalence(self):
        items, serial, serial_output = self.run_campaign(1)
        _, parallel, output = self.run_campaign(4)
        _, two, _ = self.run_campaign(2)
        manifest = json.loads((parallel / "manifest.json").read_text())
        self.assertTrue(manifest["complete"])
        self.assertEqual(len({item['command'][5] for item in items}), 54)
        self.assertEqual(len({item['log_path'] for item in items}), 54)
        positions = [output.index(item["identity"]) for item in items]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(output.count(scheduler.MARKER), 1)
        self.assertEqual(serial_output.count(scheduler.MARKER), 1)
        with redirect_stdout(io.StringIO()):
            scheduler.compare(serial / "manifest.json", parallel / "manifest.json")
            scheduler.compare(serial / "manifest.json", two / "manifest.json")
        for index, item in enumerate(items):
            self.assertEqual(Path(item['command'][5]).read_text(), scheduler.contract_text(item['identity']))
            self.assertEqual(item['index'], index)

    def test_invalid_workers(self):
        for value in ("", "0", "-1", "5", "100", "1.0", "two", " 2", "2 ", "01"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                scheduler.worker_count(value)
        for value in ("1", "2", "3", "4"):
            self.assertEqual(scheduler.worker_count(value), int(value))

    def test_cli_rejects_invalid_configuration_before_launch(self):
        for value in ('0', '-1', '5', 'invalid', ''):
            result = subprocess.run(
                [sys.executable, str(REPO / 'scripts/check-task062-service-impact.py'), str(self.fake)],
                env=dict(os.environ, AMS_GRA_TASK062_WORKERS=value), capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'AMS_GRA_TASK062_WORKERS', result.stderr)
            self.assertNotIn(scheduler.MARKER.encode(), result.stdout)
        self.assertFalse(list(self.root.glob('task062-service-impact-*')))

    def test_fixture_negative_controls(self):
        rows = scheduler.selections()
        for mutation in (rows[:-1], rows + [rows[0]], rows[:-1] + [rows[0]],
                         [rows[0].replace('2.5', '9.9')] + rows[1:],
                         [rows[0].replace('ClosedSchemaSet', 'typo')] + rows[1:]):
            with self.subTest(mutation=mutation[:1]), self.assertRaises(ValueError):
                scheduler.validate_selections(mutation)

    def test_failed_process_and_malformed_reports_fail_campaign(self):
        for mode in ("failure", "signal", "malformed", "not-ready", "truncated",
                     "wrong-language", "counts", "extra-output"):
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                self.run_campaign(mode=mode)
            scratch = self.root / f"campaign-{self.counter}"
            manifest = json.loads((scratch / "manifest.json").read_text())
            self.assertFalse(manifest["complete"])
            self.assertEqual(len(manifest["results"]), 54)
            self.assertTrue((scratch / "03/diagnostic.stderr").exists())
            if mode == "failure":
                self.assertEqual(manifest["results"][3]["exit_status"], 23)
                self.assertIn(b"injected diagnostic", (scratch / "03/diagnostic.stderr").read_bytes())

    def test_timeout_and_missing_executable(self):
        items, _ = self.prepare()
        with patch.dict(os.environ, TASK062_FAKE_MODE="timeout"), patch.object(scheduler, "TIMEOUT_SECONDS", .05):
            result = scheduler.execute(items[0])
        self.assertIsNone(result["exit_status"])
        self.assertIn("timed out", result["error"])
        items[0]['command'][0] = str(self.root / 'missing-cli')
        result = scheduler.execute(items[0])
        self.assertIn('FileNotFoundError', result['error'])

    def test_missing_duplicate_extra_wrong_and_reordered_results(self):
        items, scratch, _ = self.run_campaign()
        results = json.loads((scratch / "manifest.json").read_text())["results"]
        mutations = [results[:-1], results + [results[0]], results[:-1] + [results[0]],
                     list(reversed(results))]
        wrong = copy.deepcopy(results)
        wrong[0]['identity'] = 'wrong identity'
        mutations.append(wrong)
        for mutation in mutations:
            with self.assertRaises(ValueError):
                scheduler.validate_results(items, mutation)
        for key, value in (("command", []), ("exit_status", None), ("duration_seconds", float('nan'))):
            mutation = copy.deepcopy(results)
            mutation[0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                scheduler.validate_results(items, mutation)

    def test_missing_and_malformed_completion_logs(self):
        items, scratch, _ = self.run_campaign()
        results = json.loads((scratch / "manifest.json").read_text())["results"]
        log = Path(results[0]['log_path'])
        for content in ('{}', '{broken'):
            log.write_text(content)
            with self.assertRaises(ValueError):
                scheduler.validate_results(items, results)
        log.unlink()
        with self.assertRaises(ValueError):
            scheduler.validate_results(items, results)

    def test_worker_exception_never_passes(self):
        with patch.object(scheduler, 'execute', side_effect=RuntimeError('injected worker exception')):
            with self.assertRaises(ValueError):
                self.run_campaign()
        manifest = json.loads((self.root / f'campaign-{self.counter}/manifest.json').read_text())
        self.assertFalse(manifest['complete'])

    def test_concurrency_is_bounded(self):
        import threading
        import time
        original = scheduler.execute
        for workers in (1, 2, 4):
            active = peak = 0
            lock = threading.Lock()

            def tracked(item):
                nonlocal active, peak
                with lock:
                    active += 1
                    peak = max(peak, active)
                try:
                    time.sleep(.01)
                    return original(item)
                finally:
                    with lock:
                        active -= 1

            with patch.object(scheduler, 'execute', tracked):
                self.run_campaign(workers)
            self.assertEqual(active, 0)
            self.assertEqual(peak, workers)

    def test_comparison_rejects_command_contract_identity_and_count_changes(self):
        _, serial, _ = self.run_campaign(1)
        _, parallel, _ = self.run_campaign(4)
        path = parallel / 'manifest.json'
        original = json.loads(path.read_text())
        mutations = []
        command = copy.deepcopy(original)
        for key in ('expected', 'results'):
            command[key][0]['command'][3] = '/wrong/schema.xsd'
        scheduler.save_json(command['results'][0]['log_path'], command['results'][0])
        path.write_text(json.dumps(command))
        with self.assertRaises(ValueError), redirect_stdout(io.StringIO()):
            scheduler.compare(serial / 'manifest.json', path)
        scheduler.save_json(original['results'][0]['log_path'], original['results'][0])
        for key, value in (('identity', 'wrong'), ('contract', 'wrong'), ('exit_status', 23)):
            mutation = copy.deepcopy(original)
            mutation['results'][0][key] = value
            mutations.append(mutation)
        mutation = copy.deepcopy(original)
        mutation['results'].pop()
        mutations.append(mutation)
        for mutation in mutations:
            path.write_text(json.dumps(mutation))
            with self.assertRaises(ValueError), redirect_stdout(io.StringIO()):
                scheduler.compare(serial / 'manifest.json', path)

    def test_comparison_rejects_missing_or_changed_diagnostics(self):
        _, serial, _ = self.run_campaign(1)
        _, parallel, _ = self.run_campaign(4)
        manifest_path = parallel / 'manifest.json'
        manifest = json.loads(manifest_path.read_text())
        log = Path(manifest['results'][0]['log_path']).with_suffix('.stderr')
        log.write_bytes(b'changed diagnostic\n')
        with self.assertRaises(ValueError), redirect_stdout(io.StringIO()):
            scheduler.compare(serial / 'manifest.json', manifest_path)
        log.unlink()
        with self.assertRaises(ValueError):
            scheduler.compare(serial / 'manifest.json', manifest_path)
        manifest['complete'] = False
        manifest_path.write_text(json.dumps(manifest))
        with self.assertRaises(ValueError):
            scheduler.compare(serial / 'manifest.json', manifest_path)

    def test_benchmark_observes_thread_launched_fake_processes(self):
        destination = self.root / 'measurement'
        result = subprocess.run(
            [sys.executable, str(REPO / 'scripts/benchmark-task062-service-impact.py'),
             str(REPO / 'scripts/check-task062-service-impact.py'), str(self.fake), str(destination)],
            env=dict(os.environ, AMS_GRA_TASK062_WORKERS='4'), capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        metrics = json.loads((destination / 'metrics.json').read_text())
        commands = json.loads((destination / 'commands.json').read_text())
        self.assertEqual(metrics['checks'], 54)
        self.assertEqual(metrics['successes'], 54)
        self.assertEqual(len({record['identity'] for record in commands}), 54)
        self.assertGreater(metrics['peak_tree_rss_bytes_sampled'], 0)
        self.assertGreater(len(metrics['sampled_process_io']), 1)


if __name__ == '__main__':
    unittest.main(verbosity=2)
