"""Deterministic wrapper and corpus checks; no broker build or authority bypass."""
import json
import subprocess
import unittest
from pathlib import Path

from qualify import run_checked
from corpus import FIX
from bundle import digest


class DriverTests(unittest.TestCase):
    def test_failed_process(self):
        with self.assertRaisesRegex(RuntimeError,'probe failed'):
            run_checked(['python3','-c','raise SystemExit(3)'], {})

    def test_missing_marker(self):
        with self.assertRaisesRegex(RuntimeError,'missing execution marker'):
            run_checked(['python3','-c','print("0 tests passed")'], {})

    def test_missing_inputs(self):
        r = subprocess.run(['python3',str(Path(__file__).with_name('qualify.py'))],capture_output=True)
        self.assertNotEqual(r.returncode,0)
        self.assertIn(b'required',r.stderr)

    def test_duplicate_bytes_preserved(self):
        for cid in ['duplicate','duplicate-escaped']:
            raw = (FIX/(cid+'.json')).read_bytes()
            self.assertEqual(json.loads(raw)['Message']['$type'],'Known')
            self.assertNotEqual(raw,json.dumps(json.loads(raw)).encode())
        self.assertNotEqual(digest((FIX/'duplicate.json').read_bytes()),digest((FIX/'duplicate-escaped.json').read_bytes()))

    def test_unique_case_ids(self):
        rows=json.loads((FIX/'synthetic.json').read_bytes())
        self.assertEqual(len(rows),len({r['id'] for r in rows}))
        for r in rows:
            self.assertTrue((FIX/(r['id']+'.json')).is_file())


if __name__=='__main__':
    unittest.main()