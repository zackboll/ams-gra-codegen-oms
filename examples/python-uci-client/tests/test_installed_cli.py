"""PY-OMS-023: the installed python-uci-client wheel, independent of the checkout.

Builds the runtime and application wheels into an external wheelhouse,
installs them offline into a fresh environment of this interpreter and runs
the INSTALLED `python-uci-client` console script (never python_uci_client.py
from the repository) from a directory outside the repository, against binding
sets copied to a separate deployment directory. No broker is needed here; see
test_installed_real_sleet.py for real pinned Sleet.
"""
import configparser
import json
import shutil
import subprocess
import unittest
import zipfile

from support import APP, ROOT, cleanup_bindings, client, free_port
import installed
from installed import BINDING_SETS, DECOY_STATUS, MODE_FLAGS

INSTALLATION = None
REPOSITORY_STATE = None


def repository_state():
    """Ignored files (build/, dist/, *.egg-info, caches) in the application and
    runtime trees: what an in-tree build or bytecode compilation would leave."""
    status = subprocess.run(['git', 'status', '--porcelain', '--ignored', '--untracked-files=all', '--',
                             str(APP), str(installed.RUNTIME)], cwd=ROOT, capture_output=True,
                            text=True, check=True).stdout
    return sorted(line for line in status.splitlines() if line.startswith('!! '))


def setUpModule():
    global INSTALLATION, REPOSITORY_STATE
    REPOSITORY_STATE = repository_state()
    INSTALLATION = installed.Installation().setup()


def tearDownModule():
    try:
        if INSTALLATION is not None:
            INSTALLATION.cleanup()
    finally:
        cleanup_bindings()


def closed_line(form):
    names = [message for message, _, _ in client.MANIFEST_FAMILIES[form]]
    suffix = ''.join(f' {name}=0/0' for name in names) if form > 1 else ''
    return 'CLOSED received=0 rejected=0 callback-failures=0' + suffix


def arguments(url, release, directory, form, *extra):
    return ['--url', url, '--service-id', 'python-uci-client', '--schema-release', release,
            '--bindings', directory, *MODE_FLAGS[form], *extra]


def unused_url():
    """A loopback port with nothing listening: connection refused (exit 4)."""
    return f'ws://127.0.0.1:{free_port()}'


class InstalledCli(unittest.TestCase):
    def assert_not_decoy(self, result):
        self.assertNotEqual(result.returncode, DECOY_STATUS, result.stderr)
        self.assertNotIn('DECOY', result.stdout + result.stderr)

    def test_wheel_contents_and_metadata(self):
        wheelhouse = INSTALLATION.wheelhouse
        names = sorted(path.name for path in wheelhouse.iterdir())
        self.assertEqual(len(names), 3, names)
        self.assertIn(installed.WHEEL, names)
        self.assertIn(installed.RUNTIME_WHEEL, names)
        self.assertTrue(any(name.startswith('websockets-17.2-') for name in names), names)
        with zipfile.ZipFile(wheelhouse / installed.WHEEL) as archive:
            members = set(archive.namelist())
            # Exactly the application module and its metadata: no generated code,
            # contracts, schemas, generation helper, certificates or tests.
            self.assertEqual(members, installed.WHEEL_CONTENTS)
            self.assertEqual(archive.read('python_uci_client.py'),
                             (APP / 'python_uci_client.py').read_bytes())  # packaged unmodified
            wheel = archive.read(f'{installed.DIST_INFO}/WHEEL').decode()
            entry_points = archive.read(f'{installed.DIST_INFO}/entry_points.txt').decode()
            top_level = archive.read(f'{installed.DIST_INFO}/top_level.txt').decode().split()
        self.assertIn('Root-Is-Purelib: true', wheel)
        self.assertIn('Tag: py3-none-any', wheel)
        self.assertEqual(top_level, ['python_uci_client'])
        points = configparser.ConfigParser()
        points.read_string(entry_points)
        self.assertEqual(points.sections(), ['console_scripts'])
        self.assertEqual(dict(points['console_scripts']),
                         {'python-uci-client': 'python_uci_client:main'})
        metadata = installed.wheel_metadata(wheelhouse / installed.WHEEL, installed.DIST_INFO)
        self.assertEqual(metadata['Name'], installed.DISTRIBUTION)
        self.assertEqual(metadata['Version'], installed.VERSION)
        self.assertEqual(metadata['Requires-Python'], '>=3.11')
        self.assertEqual(metadata.get_all('Requires-Dist'), ['ams-gra-oms-lacal-runtime==0.1.0'])
        self.assertEqual(metadata['License'], 'Apache-2.0')
        self.assertIn('Private :: Do Not Upload', metadata.get_all('Classifier'))
        # The existing runtime package, unchanged: its own single pin.
        self.assertEqual(INSTALLATION.runtime_requires, ['websockets==17.2'])
        # Nothing was built, cached or generated inside the repository by this
        # suite (an earlier `pip install ./python/oms-lacal-runtime` may already
        # have left ignored setuptools output there; it must not change).
        self.assertEqual(repository_state(), REPOSITORY_STATE)
        print(f'PY-OMS-023 WHEEL: {installed.WHEEL} members={len(members)} PASSED', flush=True)

    def test_offline_installation_into_fresh_environment(self):
        self.assertNotIn('Downloading', INSTALLATION.install_output)
        self.assertRegex(INSTALLATION.install_output,
                         r'Successfully installed .*ams-gra-oms-python-uci-client-0\.1\.0')
        packages = dict(INSTALLATION.packages)
        self.assertIn('pip', packages)
        for seed in installed.VENV_SEED:
            packages.pop(seed, None)
        self.assertEqual(packages, {'ams-gra-oms-python-uci-client': '0.1.0',
                                    'ams-gra-oms-lacal-runtime': '0.1.0', 'websockets': '17.2'})
        self.assertIn('No broken requirements found', INSTALLATION.pip_check)
        self.assertRegex((INSTALLATION.venv / 'pyvenv.cfg').read_text(),
                         r'(?m)^include-system-site-packages = false$')
        # Control: without the wheelhouse the same offline configuration cannot
        # resolve the application, so the installation above used only local wheels.
        result = subprocess.run([str(INSTALLATION.bin / 'python'), '-m', 'pip', 'download', '--no-deps',
                                 '--no-index', '--dest', str(INSTALLATION.root / 'control'),
                                 f'{installed.DISTRIBUTION}=={installed.VERSION}'],
                                capture_output=True, text=True, timeout=120, cwd=INSTALLATION.cwd,
                                env=INSTALLATION.offline_environment())
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn('No matching distribution found', result.stderr)
        print(f'PY-OMS-023 OFFLINE INSTALL: PASSED packages={sorted(INSTALLATION.packages)}', flush=True)

    def test_import_origin_is_the_installed_environment(self):
        probe = INSTALLATION.python(
            'import json, sys, python_uci_client, oms_lacal, websockets\n'
            'print(json.dumps(dict(client=python_uci_client.__file__, runtime=oms_lacal.__file__,'
            ' websockets=websockets.__file__, path=sys.path, prefix=sys.prefix,'
            ' user_site=sys.flags.no_user_site, optimize=sys.flags.optimize)))')
        origin = json.loads(probe.stdout)
        for key in ('client', 'runtime', 'websockets'):
            path = installed.Path(origin[key]).resolve()
            self.assertTrue(path.is_relative_to(INSTALLATION.purelib), (key, path))
            self.assertFalse(path.is_relative_to(ROOT.resolve()), (key, path))
        self.assertEqual(installed.Path(origin['client']).name, 'python_uci_client.py')
        self.assertEqual(installed.Path(origin['prefix']).resolve(), INSTALLATION.venv.resolve())
        self.assertEqual(origin['user_site'], 1)
        self.assertFalse(any(entry and installed.Path(entry).resolve().is_relative_to(ROOT.resolve())
                             for entry in origin['path']), origin['path'])
        # The decoys are armed: an interpreter that DID search the working
        # directory would import them and exit 97 (the console script never does).
        armed = subprocess.run([str(INSTALLATION.bin / 'python'), '-c', 'import python_uci_client'],
                               capture_output=True, text=True, timeout=60, cwd=INSTALLATION.cwd,
                               env=INSTALLATION.environment())
        self.assertEqual(armed.returncode, DECOY_STATUS, armed.stderr)
        self.assertIn('PY-OMS-023 DECOY IMPORTED: python_uci_client', armed.stderr)
        # The installed console script is a generated launcher for main().
        script = (INSTALLATION.bin / installed.COMMAND).read_text()
        self.assertTrue(script.startswith(f'#!{INSTALLATION.bin / "python"}'), script)
        self.assertIn('from python_uci_client import main', script)
        print(f'PY-OMS-023 IMPORT ORIGIN: {origin["client"]}', flush=True)

    def test_help_and_invalid_arguments(self):
        result = INSTALLATION.run('--help')
        self.assert_not_decoy(result)
        self.assertEqual(result.returncode, client.EXIT_OK, result.stderr)
        self.assertEqual(result.stdout, client.HELP)  # identical to the source module's help
        self.assertEqual(result.stderr, '')
        deployed = INSTALLATION.deployed('2.6', True, True)
        base = arguments(unused_url(), '2.6', deployed, 3)
        cases = {
            'no arguments': [],
            'unknown flag': base + ['--verbose'],
            'help combined': base + ['--help'],
            'missing --bindings': base[:6],
            'bad release': arguments(unused_url(), '2.7', deployed, 3),
            'survivability without log': ['--url', unused_url(), '--service-id', 'python-uci-client',
                                          '--schema-release', '2.6', '--bindings', deployed,
                                          '--with-survivability'],
            'duplicate switch': base + ['--with-log'],
            'zero duration': base + ['--duration-seconds', '0'],
            'non-canonical max': base + ['--max-messages', '07'],
            'http url': ['--url', 'http://127.0.0.1'] + base[2:],
            'ca-file on ws': base + ['--ca-file', str(deployed / 'model.py')],
            'unreadable ca-file': ['--url', 'wss://localhost'] + base[2:] + ['--ca-file', '/nonexistent.pem'],
        }
        for name, argv in cases.items():
            with self.subTest(case=name):
                result = INSTALLATION.run(*argv)
                self.assert_not_decoy(result)
                self.assertEqual(result.returncode, client.EXIT_USAGE, result.stderr)
                self.assertEqual(result.stdout, '')
                self.assertTrue(result.stderr.startswith('python-uci-client: error: '), result.stderr)
                self.assertTrue(result.stderr.endswith('Try: python-uci-client --help\n'), result.stderr)
        print(f'PY-OMS-023 INSTALLED HELP/USAGE: PASSED usage-cases={len(cases)}', flush=True)

    def test_imports_nothing_from_the_checkout(self):
        # -X importtime lists every module import of one complete installed
        # session (bindings loaded, connection attempted). The decoy working
        # directory would exit 97 if it were ever searched.
        result = INSTALLATION.run(*arguments(unused_url(), '2.6',
                                             INSTALLATION.deployed('2.6', True, True), 3),
                                  extra={'PYTHONVERBOSE': '1'})
        self.assert_not_decoy(result)
        self.assertEqual(result.returncode, client.EXIT_CONNECT, result.stderr[-2000:])
        root = str(ROOT.resolve())
        loaded = [line for line in result.stderr.splitlines()
                  if line.startswith(('# code object from ', 'code object from '))]
        self.assertTrue(loaded, 'verbose import trace is empty')
        self.assertFalse([line for line in loaded if root in line])
        # Bytecode is cached next to the installed sources (site-packages/__pycache__).
        purelib = str(INSTALLATION.purelib) + '/'
        for module in ('/python_uci_client.', '/oms_lacal/'):
            self.assertTrue(any(purelib in line and module in line for line in loaded),
                            (module, loaded[-20:]))
        self.assertNotIn(root, result.stderr)
        self.assertNotIn(root, result.stdout)
        print(f'PY-OMS-023 IMPORT AUDIT: PASSED modules-from-files={len(loaded)} repository=0', flush=True)


    def test_every_deployed_binding_set_loads_and_runs(self):
        # Formats 1/2/3 x UCI 2.5/2.6 from the deployment copies: verified, loaded
        # and executed by the installed client; nothing listens, so each session
        # reaches the connection attempt (exit 4) and closes with zero counts.
        for key, (release, _, _, form) in BINDING_SETS.items():
            with self.subTest(bindings=key):
                deployed = INSTALLATION.deployed(*BINDING_SETS[key][:3])
                manifest = json.loads((deployed / 'python-uci-client-build.json').read_text())
                self.assertEqual((manifest['format'], manifest['release']), (form, release))
                result = INSTALLATION.run(*arguments(unused_url(), release, deployed, form))
                self.assert_not_decoy(result)
                self.assertEqual(result.returncode, client.EXIT_CONNECT, result.stderr)
                self.assertEqual(result.stdout.splitlines(), [closed_line(form)])
                self.assertEqual(result.stderr, 'CONNECT FAILED (TransportFailure: connection failed)\n')
        print(f'PY-OMS-023 DEPLOYED BINDINGS: PASSED sets={sorted(BINDING_SETS)}', flush=True)

    def test_modified_wrong_release_and_wrong_mode_bindings_exit_3(self):
        cases = []
        for key, (release, _, _, form) in BINDING_SETS.items():
            deployed = INSTALLATION.deployed(*BINDING_SETS[key][:3])
            other = '2.6' if release == '2.5' else '2.5'
            cases.append((f'{key} as {other}', arguments(unused_url(), other, deployed, form),
                          f"generated for UCI '{release}', not --schema-release {other}"))
            for mode in {1, 2, 3} - {form}:
                cases.append((f'{key} in mode {mode}', arguments(unused_url(), release, deployed, mode),
                              f'bindings in {deployed} '))
        tampered = INSTALLATION.root / 'tampered'
        shutil.copytree(INSTALLATION.deployed('2.6', True, True), tampered)
        with (tampered / 'service_codec.py').open('a') as handle:
            handle.write('\n# modified after generation\n')
        cases.append(('modified codec', arguments(unused_url(), '2.6', tampered, 3),
                      'generated service_codec.py was modified after generation; regenerate'))
        cases.append(('missing directory', arguments(unused_url(), '2.6', INSTALLATION.root / 'absent', 3),
                      'has no python-uci-client-build.json; run generate_bindings.py first'))
        for name, argv, expected in cases:
            with self.subTest(case=name):
                result = INSTALLATION.run(*argv)
                self.assert_not_decoy(result)
                self.assertEqual(result.returncode, client.EXIT_BINDINGS, result.stderr)
                self.assertEqual(result.stdout, '')
                self.assertTrue(result.stderr.startswith('python-uci-client: error: '), result.stderr)
                self.assertIn(expected, result.stderr)
        print(f'PY-OMS-023 BINDING REFUSALS: PASSED cases={len(cases)}', flush=True)

    def test_optimized_installed_executable(self):
        # PYTHONOPTIMIZE=1 qualifies the installed launcher in optimized mode.
        probe = INSTALLATION.python('import sys, python_uci_client; print(sys.flags.optimize, '
                                    'python_uci_client.__cached__)', optimize=True)
        flag, cached = probe.stdout.split()
        self.assertEqual(flag, '1')
        self.assertTrue(cached.endswith('.opt-1.pyc'), cached)
        self.assertTrue(installed.Path(cached).is_relative_to(INSTALLATION.purelib), cached)
        self.assertEqual(INSTALLATION.run('--help', optimize=True).stdout, client.HELP)
        result = INSTALLATION.run(*arguments(unused_url(), '2.5', INSTALLATION.deployed('2.5'), 1),
                                  optimize=True)
        self.assertEqual(result.returncode, client.EXIT_CONNECT, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [closed_line(1)])
        result = INSTALLATION.run(*arguments(unused_url(), '2.6', INSTALLATION.deployed('2.5'), 1),
                                  optimize=True)
        self.assertEqual(result.returncode, client.EXIT_BINDINGS, result.stderr)
        print('PY-OMS-023 PYTHONOPTIMIZE=1: PASSED', flush=True)


if __name__ == '__main__':
    unittest.main()

