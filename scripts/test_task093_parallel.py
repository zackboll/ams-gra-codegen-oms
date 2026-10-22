"""Fail-closed scheduler controls; also loaded by the existing CI wrapper gate."""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("parallel_runner", Path(__file__).with_name("check-task093-helper-demand.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class SchedulerControls(unittest.TestCase):
    def setUp(self):
        self.args = argparse.Namespace(compiler="14", family="float32", red=False,
                                       profiles=["oam", "qualified"],
                                       policies=["normal", "Check", "Ignore"],
                                       generator=Path("/bin/true"))

    def result(self, profile="oam", name="ArraySample"):
        return dict(completed=True, red=self.args.red, records=[
            dict(compiler="14.2.0", family="float32", profile=profile, name=name,
                 policy=policy, intentional_red=self.args.red,
                 compiler_exit=1 if self.args.red else 0)
            for policy in self.args.policies])

    def test_bounded_workers(self):
        for value in ["1", "2", "3", "4"]:
            self.assertEqual(runner.worker_count(value), int(value))
        for value in ["", "0", "5", "-1", "1.0", " 2", "02", "auto", "999999"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                runner.worker_count(value)

    def test_order_independent_aggregation_is_byte_identical(self):
        selections = [(profile, name) for profile in self.args.profiles for name in runner.NAMES]
        completed = [(selection, self.result(*selection)) for selection in selections]
        reference = runner.aggregate(self.args, selections, completed)
        self.assertEqual(len(reference), 30)
        self.assertEqual(json.dumps(reference), json.dumps(runner.aggregate(self.args, selections, list(reversed(completed)))))

    def test_missing_duplicate_and_unexpected_selection_fail(self):
        selections = [("oam", "ArraySample"), ("qualified", "ArraySample")]
        complete = [(selection, self.result(*selection)) for selection in selections]
        for items in [complete[:1], [complete[0], complete[0]],
                      [complete[0], (("oam", "other"), self.result())]]:
            with self.subTest(items=items), self.assertRaises(RuntimeError):
                runner.aggregate(self.args, selections, items)
        with self.assertRaises(RuntimeError):
            runner.aggregate(self.args, [selections[0]] * 2, [complete[0]] * 2)

    def test_malformed_incomplete_and_wrong_policy_fail(self):
        valid = self.result()
        mutations = [None, [], {}, {**valid, "completed": False},
                     {**valid, "completed": 1}, {**valid, "red": True},
                     {**valid, "records": valid["records"][:2]}]
        for key, value in [("compiler", "16.1.0"), ("family", "portable"),
                           ("policy", "Ignore"), ("name", "other"),
                           ("profile", "qualified"), ("compiler_exit", 1),
                           ("compiler_exit", False), ("intentional_red", True)]:
            bad = copy.deepcopy(valid)
            bad["records"][0][key] = value
            mutations.append(bad)
        bad = copy.deepcopy(valid)
        bad["records"][1] = bad["records"][0]
        mutations.append(bad)
        for result in mutations:
            with self.subTest(result=result), self.assertRaises(RuntimeError):
                runner.validate_case(result, self.args, "oam", "ArraySample")

    def test_intentional_red_requires_actual_failure(self):
        self.args.red = True
        runner.validate_case(self.result(), self.args, "oam", "ArraySample")
        bad = self.result()
        bad["records"][0]["compiler_exit"] = 0
        with self.assertRaises(RuntimeError):
            runner.validate_case(bad, self.args, "oam", "ArraySample")

    def test_scheduler_drains_workers_and_retains_failure_logs(self):
        # Real child exits, including abrupt failure and missing/malformed JSON.
        # These are scheduler controls, not substitutes for compiler campaigns.
        original_run = subprocess.run
        for fault in ["exit", "missing", "malformed", "duplicate"]:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                seed = root / "seed"
                (seed / "toolchains").mkdir(parents=True)
                script = '''
import json, pathlib, sys
args = sys.argv[1:]
scratch = pathlib.Path(args[args.index('--scratch') + 1]); scratch.mkdir()
profile = args[args.index('--profiles') + 1]
name = args[args.index('--case') + 1]
fault = FAULT
print('diagnostic', profile, name, flush=True)
if profile == 'oam' and name == 'ArraySample':
    if fault == 'exit': sys.exit(7)
    if fault == 'missing': sys.exit(0)
    if fault == 'malformed':
        (scratch / 'result.json').write_text('{'); sys.exit(0)
    name = 'wrong-identity'
records = [dict(compiler='14.2.0', family='float32', profile=profile, name=name,
               policy=p, intentional_red=False, compiler_exit=0)
           for p in ['normal', 'Check', 'Ignore']]
(scratch / 'result.json').write_text(json.dumps(dict(completed=True, red=False, records=records)))
'''.replace("FAULT", repr(fault))

                def child(command, **kwargs):
                    return original_run([sys.executable, "-c", script, *command[2:]], **kwargs)

                with patch.object(runner.subprocess, "run", side_effect=child), \
                     patch.object(runner, "verify_case_evidence"), self.assertRaises(RuntimeError):
                    runner.run_campaign(self.args, root, seed, 2)
                self.assertEqual(len(list(root.glob("*-worker.exit"))), 10)
                self.assertEqual(len(list(root.glob("*-worker.log"))), 10)
                self.assertIn("diagnostic oam ArraySample", (root / "oam-ArraySample-worker.log").read_text())
                self.assertFalse((root / "result.json").exists())

    def test_private_alire_metadata_and_toolchain_sentinel(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed = root / "seed"
            (seed / "toolchains").mkdir(parents=True)
            payload = root / "deployed-compiler"
            payload.mkdir()
            (seed / "toolchains/compiler").symlink_to(payload, target_is_directory=True)
            runner.clone_alire(seed, root / "one")
            runner.clone_alire(seed, root / "two")
            (root / "one/toolchains/must_reload").touch()
            # Seed toolchains may have been installed during preflight. Clone
            # their directory entries as symlinks, never copy binary payloads.
            installed = seed / "toolchains/new-compiler"
            installed.mkdir()
            (installed / "binary").write_bytes(b"compiler")
            runner.clone_alire(seed, root / "three")
            self.assertTrue((root / "three/toolchains/new-compiler").is_symlink())
            self.assertFalse((root / "two/toolchains/must_reload").exists())
            self.assertFalse((seed / "toolchains/must_reload").exists())
            self.assertEqual((root / "one/toolchains/compiler").resolve(), payload)
            self.assertIn(str(root / "one/cache"), (root / "one/settings.toml").read_text())
            self.assertNotEqual((root / "one/settings.toml").read_bytes(), (root / "two/settings.toml").read_bytes())

    def test_resource_sampler_includes_child_and_reports_cpu(self):
        measurement = runner.Measurement()
        subprocess.run([sys.executable, "-c", "import time; x=bytearray(16*1024*1024); time.sleep(0.4)"], check=True)
        result = measurement.finish()
        self.assertGreater(result["wall_seconds"], 0.4)
        self.assertGreaterEqual(result["cpu_seconds"], 0)
        self.assertGreater(result["sampled_peak_tree_rss_kib"], 16000)

    def test_completion_record_without_compiler_evidence_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            with patch.object(runner, "verify_sources"), self.assertRaises(OSError):
                runner.verify_case_evidence(Path(temporary), self.result()["records"])

    def test_ephemeral_alire_payload_links_are_hidden_from_existing_uploads(self):
        # upload-artifact@v4 follows directory symlinks but excludes hidden
        # directories by default. Keep compiler payload links out of evidence.
        source = Path(runner.__file__).read_text()
        self.assertIn('scratch / ".alire-seed"', source)
        self.assertIn('scratch / f".{label}-alire"', source)
        workflow = (runner.ROOT / ".github/workflows/ci.yml").read_text()
        self.assertNotIn("include-hidden-files: true", workflow)

    def test_provenance_detects_modified_or_missing_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.adb"
            source.write_bytes(b"original")
            digest = runner.hashlib.sha256(source.read_bytes()).hexdigest()
            (root / "sources.sha256").write_text(f"{digest}  {source}\n")
            runner.verify_sources(root)
            source.write_bytes(b"changed")
            with self.assertRaises(RuntimeError):
                runner.verify_sources(root)
            source.unlink()
            with self.assertRaises(OSError):
                runner.verify_sources(root)


class EquivalenceControls(unittest.TestCase):
    def test_comparator_rejects_source_marker_lock_and_outcome_differences(self):
        spec = importlib.util.spec_from_file_location("compare_controls", Path(__file__).with_name("compare-task093-helper-demand.py"))
        comparator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(comparator)
        left = {"source": b"Ada", "marker": b"COMPLETE", "lock": {"version": "14.2.0"},
                "outcome": (0, 0), "result.json": {"completed": True}}
        with patch.object(comparator, "evidence", side_effect=[left, copy.deepcopy(left)]):
            self.assertEqual(comparator.compare(Path("left"), Path("right")), len(left))
        for key in left:
            right = copy.deepcopy(left)
            right[key] = None
            with self.subTest(key=key), patch.object(comparator, "evidence", side_effect=[left, right]), self.assertRaises(RuntimeError):
                comparator.compare(Path("left"), Path("right"))
        with patch.object(comparator, "evidence", side_effect=[left, {}]), self.assertRaises(RuntimeError):
            comparator.compare(Path("left"), Path("right"))


if __name__ == "__main__":
    unittest.main()