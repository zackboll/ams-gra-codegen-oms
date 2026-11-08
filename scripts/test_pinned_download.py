"""Controlled transport and integrity failures; no network in these controls."""
import hashlib
import gzip
import io
from pathlib import Path
import subprocess
import tempfile
import tarfile
import unittest
from unittest.mock import patch
import zipfile

import pinned_download as download


class PinnedDownload(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.destination = Path(self.tmp.name) / 'tool.zip'
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, 'w') as archive:
            archive.writestr('bin/tool', 'verified tool')
        self.data = stream.getvalue()
        pin = ('https://github.com/official/release',
               'https://api.github.com/repos/official/releases/assets/1',
               'sha256', hashlib.sha256(self.data).hexdigest(), 'zip')
        self.pins = patch.dict(download.PINS, test=pin)
        self.pins.start()
        self.addCleanup(self.pins.stop)

    def transport(self, results, data=None):
        results = iter(results)

        def run(command, **kwargs):
            self.assertIn('--proto', command)
            self.assertIn('--proto-redir', command)
            self.assertNotIn('--insecure', command)
            self.assertIn('--max-time', command)
            code = next(results)
            Path(command[command.index('--output') + 1]).write_bytes(
                (self.data if data is None else data) if code == 0 else b'partial')
            return subprocess.CompletedProcess(command, code, '',
                'curl: (22) The requested URL returned error: 504\n' if code else '')
        return run

    def invoke(self, results, data=None):
        with patch.object(download.subprocess, 'run', side_effect=self.transport(results, data)) as run, \
                patch.object(download.time, 'sleep') as sleep:
            download.download('test', self.destination)
        self.assertEqual(self.destination.read_bytes(), self.data)
        self.assertEqual(list(self.destination.parent.glob('.pinned-*')), [])
        return run, sleep

    def test_primary_success(self):
        run, sleep = self.invoke([0])
        self.assertEqual(run.call_count, 1)
        sleep.assert_not_called()

    def test_primary_504_then_retry(self):
        run, sleep = self.invoke([22, 0])
        self.assertEqual(run.call_count, 2)
        sleep.assert_called_once_with(2)

    def test_primary_exhausted_then_official_asset_recovery(self):
        run, sleep = self.invoke([22, 22, 22, 0])
        self.assertEqual(run.call_count, 4)
        self.assertEqual(sleep.call_count, 2)
        command = run.call_args.args[0]
        self.assertEqual(command[-1], download.PINS['test'][1])
        self.assertIn('Accept: application/octet-stream', command)

    def test_all_retrieval_attempts_exhausted(self):
        with patch.object(download.subprocess, 'run', side_effect=self.transport([22] * 6)) as run, \
                patch.object(download.time, 'sleep') as sleep:
            with self.assertRaisesRegex(RuntimeError, 'all pinned retrieval attempts exhausted'):
                download.download('test', self.destination)
        self.assertEqual(run.call_count, 6)
        self.assertEqual(sleep.call_count, 4)
        self.assertEqual(list(self.destination.parent.iterdir()), [])

    def test_wrong_checksum_fatal_without_fallback(self):
        with patch.object(download.subprocess, 'run', side_effect=self.transport([0], b'wrong')) as run:
            with self.assertRaisesRegex(RuntimeError, 'checksum mismatch'):
                download.download('test', self.destination)
        self.assertEqual(run.call_count, 1)
        self.assertEqual(list(self.destination.parent.iterdir()), [])

    def test_truncated_archive_even_if_digest_matches(self):
        truncated = self.data[:30]
        pin = list(download.PINS['test'])
        pin[3] = hashlib.sha256(truncated).hexdigest()
        with patch.dict(download.PINS, test=tuple(pin)), \
                patch.object(download.subprocess, 'run', side_effect=self.transport([0], truncated)):
            with self.assertRaises(zipfile.BadZipFile):
                download.download('test', self.destination)
        self.assertFalse(self.destination.exists())

    def test_truncated_gzip_footer_even_if_digest_matches(self):
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode='w') as archive:
            member = tarfile.TarInfo('tool')
            member.size = 1
            archive.addfile(member, io.BytesIO(b'x'))
        truncated = gzip.compress(stream.getvalue())[:-4]
        pin = list(download.PINS['test'])
        pin[3:] = [hashlib.sha256(truncated).hexdigest(), 'tar.gz']
        with patch.dict(download.PINS, test=tuple(pin)), \
                patch.object(download.subprocess, 'run', side_effect=self.transport([0], truncated)):
            with self.assertRaises(EOFError):
                download.download('test', self.destination)
        self.assertFalse(self.destination.exists())

    def test_existing_wrong_bytes_rejected_without_network(self):
        self.destination.write_bytes(b'old or corrupt cache')
        with patch.object(download.subprocess, 'run') as run:
            with self.assertRaisesRegex(RuntimeError, 'checksum mismatch'):
                download.download('test', self.destination)
        run.assert_not_called()

    def test_existing_verified_cache(self):
        self.destination.write_bytes(self.data)
        with patch.object(download.subprocess, 'run') as run:
            download.download('test', self.destination)
        run.assert_not_called()

    def test_transient_classification_and_nontransient_fail_closed(self):
        for status in (502, 503, 504):
            self.assertTrue(download.transient(22, f'error: {status}'))
        for code in (5, 6, 7, 18, 28, 52, 55, 56):
            self.assertTrue(download.transient(code, ''))
        for status in (401, 403, 404):
            self.assertFalse(download.transient(22, f'error: {status}'))
        self.assertFalse(download.transient(60, 'certificate failure'))
        with patch.object(download.subprocess, 'run', return_value=
                          subprocess.CompletedProcess([], 60, '', 'certificate failure')) as run:
            with self.assertRaisesRegex(RuntimeError, 'non-transient'):
                download.download('test', self.destination)
        self.assertEqual(run.call_count, 1)

    def test_exact_tool_versions(self):
        for name, correct, wrong in [('alire', 'alr 2.1.1\n', 'alr 2.1.2\n'),
                                      ('actionlint', '1.7.12\ninstalled by go\n', '1.7.13\n')]:
            with self.subTest(tool=name), patch.object(download.subprocess, 'check_output', return_value=correct):
                download.verify_version(name, '/verified/tool')
            for output in (wrong, '', 'prefix ' + correct):
                with self.subTest(tool=name, output=output), \
                        patch.object(download.subprocess, 'check_output', return_value=output), \
                        self.assertRaisesRegex(RuntimeError, 'wrong pinned tool version'):
                    download.verify_version(name, '/verified/tool')

    def test_unknown_origin_rejected(self):
        helper = Path(download.__file__)
        result = subprocess.run(['python3', str(helper), 'https://untrusted.invalid/tool.zip',
                                 str(self.destination)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('unknown or ambiguous', result.stderr)


if __name__ == '__main__':
    unittest.main()