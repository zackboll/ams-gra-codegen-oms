"""Deterministic wrapper and corpus checks; no broker build or authority bypass."""
import json
import subprocess
import unittest
import os
import signal
import tempfile
import sys
import ctypes
from pathlib import Path

from qualify import run_checked, read_artifact, REPO
from corpus import FIX
from bundle import digest


class DriverTests(unittest.TestCase):
    def fault(self, kind):
        # Caller-side watchdog and identity-bound finally cleanup independent of SUT.
        ctypes.CDLL(None).prctl(36,1,0,0,0)
        with tempfile.TemporaryDirectory(prefix='task076a-test-') as directory:
            process = subprocess.Popen([sys.executable,str(Path(__file__).with_name('fault_probe.py')),
                'scenario',kind,directory],stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                start_new_session=True,text=True)
            try:
                stdout,stderr=process.communicate(timeout=12)
                self.assertEqual(process.returncode,0,stdout+stderr)
                value=json.loads(stdout)
                self.assertTrue(value['ready'])
                self.assertTrue(value['sentinel_alive'])
            finally:
                if process.poll() is None:
                    os.killpg(process.pid,signal.SIGKILL)
                    process.communicate(timeout=3)
                for filename in ('ready.json','sentinel.json'):
                    ready=Path(directory)/filename
                    if ready.exists():
                        record=json.loads(ready.read_bytes())
                        pid=record.get('pid')
                        if pid and Path(f'/proc/{pid}/stat').exists():
                            self.assertEqual(Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()[19], record['start'])
                            self.assertEqual(os.getpgid(pid),record['pgid'])
                            os.killpg(record['pgid'],signal.SIGKILL)
                            try: os.waitpid(pid,0)
                            except ChildProcessError: pass

    def test_constructor_boundary_limits_wall(self): self.fault('stall-constructor')
    def test_preparation_wall(self): self.fault('stall-prepare')
    def test_mapping_wall(self): self.fault('stall-map')
    def test_reference_wall(self): self.fault('stall-reference')
    def test_worker_nonzero(self): self.fault('exit')
    def test_worker_signal(self): self.fault('signal')
    def test_worker_pin_failure(self): self.fault('pin')
    def test_worker_compilation_failure(self): self.fault('compilation')
    def test_cpu_limit(self): self.fault('cpu')
    def test_memory_limit(self): self.fault('memory')
    def test_resistant_independent_pipes(self): self.fault('independent')
    def test_resistant_captured_pipe(self): self.fault('pipe')
    def test_exit_during_grace(self): self.fault('race')
    def test_failed_leader_cleanup(self): self.fault('failed')
    def test_missing_marker_cleanup(self): self.fault('marker')
    def test_interrupted_leader_cleanup(self): self.fault('interrupt')
    def test_parent_interruption_cleanup(self): self.fault('parent-interrupt')
    def test_parent_term_cleanup(self): self.fault('parent-term')

    def test_successful_child(self):
        run_checked([sys.executable,'-c','print("TASK076 BROKER: EXECUTED")'],os.environ)

    def test_artifact_contract(self):
        expected=json.loads((REPO/'tests/fixtures/task076/expected-results.json').read_bytes())
        ids=[r['id'] for r in expected['results']]
        artifact=dict(token='current',campaign=expected['corpus'],results=expected['results'],
                      validator=expected['validator'])
        with tempfile.TemporaryDirectory() as directory:
            p=Path(directory)/'validator.json'
            with self.assertRaisesRegex(RuntimeError,'artifact invalid'):
                read_artifact(p,'current',ids)
            for text in ['{','not-json',json.dumps(dict(artifact,token='old')),
                         json.dumps(dict(artifact,results=artifact['results'][:-1])),
                         ' '* (2*1024*1024+1)]:
                p.write_text(text)
                with self.assertRaisesRegex(RuntimeError,'artifact invalid'):
                    read_artifact(p,'current',ids)
            p.write_text(json.dumps(artifact))
            self.assertEqual(read_artifact(p,'current',ids),artifact)

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