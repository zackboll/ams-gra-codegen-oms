"""APT fallback retains signed official sources, bounds execution and fails closed."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    'install_rust_ci', Path(__file__).with_name('install-rust-ci-dependencies.py'))
APT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(APT)


class AptDependencies(unittest.TestCase):
    def test_fallback_deb822_and_legacy_preserve_keyring_and_suites(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = root / 'sources.list.d'
            sources.mkdir()
            output = root / 'fallback'
            output.mkdir()
            (sources / 'ubuntu.sources').write_text(
                'Types: deb\nURIs: http://azure.archive.ubuntu.com/ubuntu/\n'
                'Suites: noble noble-updates noble-security\nComponents: main universe\n'
                'Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n')
            (root / 'sources.list').write_text(
                'deb http://azure.archive.ubuntu.com/ubuntu noble main\n'
                'deb https://untrusted.example/ubuntu noble main\n')
            original = (sources / 'ubuntu.sources').read_bytes()
            APT.fallback_sources(root, output)
            text = '\n'.join(p.read_text() for p in output.iterdir())
            self.assertIn('https://archive.ubuntu.com/ubuntu/', text)
            self.assertIn('noble-security', text)
            self.assertIn('Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg', text)
            self.assertNotIn('untrusted.example', text)
            self.assertNotIn('trusted=yes', text)
            self.assertEqual(original, (sources / 'ubuntu.sources').read_bytes())

    def test_unknown_sources_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(RuntimeError):
                APT.fallback_sources(Path(directory), Path(directory))

    def test_timeout_and_failure_do_not_become_success(self):
        with patch.object(APT.subprocess, 'Popen') as popen, patch.object(APT.os, 'killpg') as kill:
            popen.return_value.wait.side_effect = [subprocess.TimeoutExpired('apt-get', 180), 0]
            self.assertEqual(APT.execute(['apt-get', 'update'], 180), 124)
            kill.assert_called_once()
            self.assertTrue(popen.call_args.kwargs['start_new_session'])
        with patch.object(APT, 'execute', return_value=100) as execute:
            self.assertFalse(APT.install(APT.OPTIONS))
            self.assertEqual(execute.call_count, 1)

    def test_install_requires_both_packages_and_bounded_update_and_install(self):
        with patch.object(APT, 'execute', return_value=0) as execute:
            self.assertTrue(APT.install(APT.OPTIONS))
            calls = execute.call_args_list
            self.assertEqual(calls[0].args[1], 180)
            self.assertEqual(calls[1].args[1], 300)
            self.assertEqual(calls[1].args[0][-2:], ['gnat', 'strace'])
            self.assertIn('APT::Update::Error-Mode=any', calls[0].args[0])


if __name__ == '__main__':
    unittest.main()