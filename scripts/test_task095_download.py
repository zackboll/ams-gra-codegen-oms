"""Task095 archive-only transport recovery: bounded and fail closed."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import task095_download as download

spec = importlib.util.spec_from_file_location('task095', Path(__file__).with_name('check-task095.py'))
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)


class Recovery(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.destination = self.root / 'provider.zip'
        self.payload = b'controlled verified archive'
        self.responses = []
        self.calls = []

    def curl(self, command, stdout, stderr):
        self.calls.append(command)
        self.assertEqual(command[0], 'curl')
        self.assertEqual(stderr, subprocess.STDOUT)
        part = Path(command[command.index('--output') + 1])
        self.assertFalse(self.destination.exists())
        self.assertEqual(part.parent, self.destination.parent)
        # A failed transfer leaves bytes; they must never survive or be published.
        code, message = self.responses.pop(0)
        part.write_bytes(self.payload if code == 0 else b'incomplete archive')
        stdout.write(message)
        return subprocess.CompletedProcess(command, code)

    def run_download(self, url=download.URL):
        with patch.object(download.subprocess, 'run', side_effect=self.curl), \
                patch.object(download.time, 'sleep') as sleep, \
                patch.object(download, 'SHA512', hashlib.sha512(self.payload).hexdigest()):
            download.download(url, self.destination, self.root)
        return sleep

    def test_transient_timeout_then_verified_atomic_success(self):
        self.responses = [(28, b'curl: (28) Failed to connect'), (0, b'downloaded')]
        sleep = self.run_download()
        sleep.assert_called_once_with(2)
        self.assertEqual(self.destination.read_bytes(), self.payload)
        self.assertEqual(len(self.calls), 2)
        self.assertTrue(all('--max-time' in command for command in self.calls))
        self.assertEqual(len(list(self.root.glob('*.command.json'))), 2)
        self.assertEqual(sorted(p.read_text() for p in self.root.glob('*.exit')), ['0\n', '28\n'])
        self.assertTrue(any(b'Failed to connect' in p.read_bytes() for p in self.root.glob('*.log')))
        self.assertFalse(list(self.root.glob('*.part')))

    def test_persistent_timeout_exhausts_exact_budget_and_capped_delays(self):
        self.responses = [(28, b'timeout')] * 3
        with patch.object(download.subprocess, 'run', side_effect=self.curl), \
                patch.object(download.time, 'sleep') as sleep:
            with self.assertRaisesRegex(RuntimeError, 'attempts=3/3'):
                download.download(download.URL, self.destination, self.root)
        self.assertEqual([c.args[0] for c in sleep.call_args_list], [2, 4])
        self.assertEqual(len(self.calls), 3)
        self.assertEqual(len(list(self.root.glob('*.exit'))), 3)
        self.assertFalse(self.destination.exists())
        self.assertFalse(list(self.root.glob('*.part')))

    def test_corrupt_success_payload_is_fatal_not_retried(self):
        self.responses = [(0, b'HTTP success but wrong bytes')]
        with patch.object(download.subprocess, 'run', side_effect=self.curl), \
                patch.object(download.time, 'sleep') as sleep:
            with self.assertRaisesRegex(RuntimeError, 'archive hash mismatch'):
                download.download(download.URL, self.destination, self.root)
        sleep.assert_not_called()
        self.assertEqual(len(self.calls), 1)
        self.assertFalse(self.destination.exists())
        self.assertFalse(list(self.root.glob('*.part')))

    def test_http_and_nontransient_failures_are_distinguished(self):
        for code in (408, 429, 500, 502, 503, 504):
            self.assertIsNotNone(download.transient(22, f'curl: (22) The requested URL returned error: {code}'.encode()))
        for code, message in ((22, b'error: 404'), (60, b'certificate failure'), (0, b'error: 503')):
            self.assertIsNone(download.transient(code, message))
        self.responses = [(60, b'certificate failure')]
        with self.assertRaisesRegex(RuntimeError, 'attempts=1/3'):
            self.run_download()
        self.assertEqual(len(self.calls), 1)

    def test_transient_http_recovers_then_still_checks_payload(self):
        self.responses = [(22, b'curl: (22) The requested URL returned error: 503'),
                          (0, b'complete')]
        sleep = self.run_download()
        sleep.assert_called_once_with(2)
        self.assertEqual(len(self.calls), 2)
        self.assertEqual(self.destination.read_bytes(), self.payload)
        decisions = [p.read_text() for p in self.root.glob('*.decision.log')]
        self.assertTrue(any('transient HTTP response' in text for text in decisions))

    def test_other_origins_get_no_retry_or_unverified_substitution(self):
        self.responses = [(28, b'timeout')]
        with self.assertRaisesRegex(RuntimeError, 'attempts=1/1'):
            self.run_download('https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v27.0.0.zip')
        self.assertEqual(len(self.calls), 1)
        self.assertTrue(self.calls[0][-1].endswith('/v27.0.0.zip'))
        self.assertNotIn('--max-time', self.calls[0])

    def test_existing_destination_not_accepted_or_overwritten(self):
        self.destination.write_bytes(b'partial')
        with self.assertRaisesRegex(RuntimeError, 'existing archive destination'):
            self.run_download()
        self.assertEqual(self.destination.read_bytes(), b'partial')
        self.assertFalse(self.calls)

    def test_wrong_provider_version_hash_and_source_remain_fatal(self):
        alire = self.root / 'alire'
        alire.mkdir()
        archive = 'sha512:' + download.SHA512
        for version, digest in (('27.0.0', archive), ('26.0.0', 'sha512:bad')):
            text = ''.join(f'[[solution.state]]\n[solution.state.release]\nname = "{name}"\nversion = "{version}"\n[solution.state.release.origin]\nhashes = ["{digest}"]\n' for name in ('gnatcoll', 'gnatcoll_minimal'))
            (alire / 'alire.lock').write_text(text)
            with self.assertRaisesRegex(RuntimeError, 'wrong provider'):
                qualification.verify_provider(self.root)
        (alire / 'alire.lock').write_text(text.replace('sha512:bad', archive))
        (alire / 'gnatcoll-json.adb').write_bytes(b'corrupted provider source')
        with self.assertRaisesRegex(RuntimeError, 'provider source hash mismatch'):
            qualification.verify_provider(self.root)


if __name__ == '__main__':
    unittest.main()