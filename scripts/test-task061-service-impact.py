#!/usr/bin/env python3
"""Fail-closed transport controls; production evaluation lives in Rust tests."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess

spec = importlib.util.spec_from_file_location("driver", Path(__file__).with_name("check-task061-service-impact.py"))
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


class Controls(unittest.TestCase):
    def setUp(self):
        self.requests = [dict(id=str(i), contract=f"/tmp/c{i}.yaml", language="rust",
                              world="closed-schema", with_codec=False, ada_codec_runtime=None) for i in range(2)]
        self.report = dict(format="service-check-batch-results-v1", schema="/schema", schema_version="002.5.0",
                           schema_loads=1, completed=2,
                           results=[dict(request=r, exit_code=0, stdout="report", stderr="") for r in self.requests])

    def validate(self, report):
        return driver.validate_batch(report, self.requests, "/schema", "2.5")

    def test_complete_and_failed_results(self):
        self.assertEqual(len(self.validate(self.report)), 2)
        driver.validate_exit(0, self.report["results"])
        with self.assertRaises(RuntimeError):
            driver.validate_exit(1, self.report["results"])
        self.report["results"][0]["exit_code"] = 1
        self.assertEqual(self.validate(self.report)[0]["exit_code"], 1)
        driver.validate_exit(1, self.report["results"])
        for status in [0, 2, -9]:
            with self.assertRaises(RuntimeError):
                driver.validate_exit(status, self.report["results"])

    def test_wrong_pinned_schema(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "wrong.xsd"
            root.write_text('<schema version="002.5.0"/>')
            with self.assertRaisesRegex(RuntimeError, "wrong pinned schema identity"):
                driver.verify_roots({"2.5": str(root)})

    def test_mutations_fail(self):
        mutations = [
            lambda r: r.update(completed=1),
            lambda r: r.update(schema="/wrong"),
            lambda r: r.update(schema_version="002.6.0"),
            lambda r: r.update(schema_loads=0),
            lambda r: r.update(completed=True),
            lambda r: r["results"].pop(),
            lambda r: r["results"].append(copy.deepcopy(r["results"][0])),
            lambda r: r["results"].__setitem__(1, copy.deepcopy(r["results"][0])),
            lambda r: r["results"][0]["request"].update(id="extra"),
            lambda r: r["results"][0]["request"].update(language="ada"),
            lambda r: r["results"][0].update(exit_code=True),
            lambda r: r["results"].reverse(),
        ]
        for mutate in mutations:
            report = copy.deepcopy(self.report)
            mutate(report)
            with self.assertRaises(RuntimeError):
                self.validate(report)

    def test_partial_and_duplicate_json(self):
        for text in ["", "{", json.dumps(self.report)[:-1], '{"a":1,"a":2}']:
            with self.assertRaises((ValueError, RuntimeError)):
                json.loads(text, object_pairs_hook=driver.unique_object)

    def test_partial_process_and_masked_failure_are_not_passes(self):
        # Fault injection tests transport only, never readiness qualification.
        rows = [("2.5", "ClosedSchemaSet", "Rust", "A"),
                ("2.5", "ClosedSchemaSet", "Rust", "B")]
        self.report["results"][0]["exit_code"] = 1
        for payload in [b'{', json.dumps(self.report).encode()]:
            with tempfile.TemporaryDirectory() as tmp:
                scratch = Path(tmp)
                result = subprocess.CompletedProcess([], 0, payload, b"injected process diagnostic")
                with patch.object(driver.subprocess, "run", return_value=result):
                    with self.assertRaises(RuntimeError):
                        driver.execute(Path("/not-a-readiness-mock"), rows, self.requests,
                                       {"2.5": "/schema"}, scratch, "batch", 1)
                for req in self.requests:
                    self.assertTrue((scratch / "batch" / (req["id"] + ".log")).is_file())
                self.assertFalse((scratch / "batch" / "results.json").exists())

    def test_worker_bounds_and_full_inventory(self):
        for text in ["0", "33", "-1", "1.5", "", " 4", "٤"]:
            with self.assertRaises(RuntimeError):
                driver.workers(text)
        for value in (1, 2, 4, 8, 32):
            self.assertEqual(driver.workers(str(value)), value)
        self.assertEqual(len(driver.load_rows()), 609)


if __name__ == "__main__":
    unittest.main()