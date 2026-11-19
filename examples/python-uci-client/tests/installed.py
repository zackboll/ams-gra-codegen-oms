"""PY-OMS-023 installed-application harness (test support only; never packaged).

Everything happens in external temporary storage; the repository is only read:

1. The runtime (python/oms-lacal-runtime) and this application are copied to a
   staging directory and built there with `pip wheel` (the setuptools backend
   of each pyproject.toml). The runtime's own pinned requirement (websockets)
   is downloaded as a wheel. The three files form a local wheelhouse.
2. A fresh virtual environment of the CURRENT interpreter is created and the
   application is installed from that wheelhouse strictly offline: --no-index,
   --only-binary, no pip configuration, and HTTP(S) proxies pointing at a
   closed port, so any network access would fail.
3. Every previously generated binding set (format 1/2/3, UCI 2.5/2.6) is
   copied to a separate deployment directory; the installed client is only
   ever given those copies.

The installed console script runs with a minimal environment (no
PYTHONPATH/PYTHONHOME, no user site, private HOME) from a working directory
outside the repository holding decoy `python_uci_client.py` and `oms_lacal.py`
modules; importing either decoy exits with DECOY_STATUS.
"""
import email
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zipfile

from support import APP, ROOT, bindings_directory

DISTRIBUTION = 'ams-gra-oms-python-uci-client'
VERSION = '0.1.0'
COMMAND = 'python-uci-client'
WHEEL = 'ams_gra_oms_python_uci_client-0.1.0-py3-none-any.whl'
DIST_INFO = 'ams_gra_oms_python_uci_client-0.1.0.dist-info'
RUNTIME = ROOT / 'python/oms-lacal-runtime'
RUNTIME_WHEEL = 'ams_gra_oms_lacal_runtime-0.1.0-py3-none-any.whl'
RUNTIME_DIST_INFO = 'ams_gra_oms_lacal_runtime-0.1.0.dist-info'
# The complete application wheel: the module and its metadata, nothing else.
WHEEL_CONTENTS = frozenset({'python_uci_client.py', *(
    f'{DIST_INFO}/{name}' for name in ('METADATA', 'WHEEL', 'entry_points.txt',
                                       'top_level.txt', 'RECORD'))})
# Deployment key -> (release, with_log, with_survivability, manifest format).
BINDING_SETS = {release + suffix: (release, log, survivability, form)
                for release in ('2.5', '2.6')
                for suffix, log, survivability, form in (('', False, False, 1),
                                                         ('-log', True, False, 2),
                                                         ('-log-survivability', True, True, 3))}
MODE_FLAGS = {1: [], 2: ['--with-log'], 3: ['--with-log', '--with-survivability']}
DECOY_STATUS = 97
DECOY = ("import sys\nsys.stderr.write('PY-OMS-023 DECOY IMPORTED: ' + __name__ + '\\n')\n"
         f"raise SystemExit({DECOY_STATUS})\n")
# What `python -m venv` itself seeds (3.11 also seeds setuptools).
VENV_SEED = frozenset({'pip', 'setuptools'})
CLOSED_PORT_PROXY = 'http://127.0.0.1:9'


def deployment_key(release, with_log=False, with_survivability=False):
    return release + ('-log' if with_log else '') + ('-survivability' if with_survivability else '')


def wheel_metadata(path, dist_info):
    with zipfile.ZipFile(path) as archive:
        return email.message_from_bytes(archive.read(f'{dist_info}/METADATA'))


class Installation:
    """One wheelhouse, one fresh offline-installed environment, one deployment."""

    def __init__(self):
        self.holder = tempfile.TemporaryDirectory(prefix='py-oms-023-installed-')
        self.root = Path(self.holder.name).resolve()
        if self.root.is_relative_to(ROOT.resolve()):
            self.holder.cleanup()
            raise RuntimeError('installed-application storage must be outside the repository')
        self.stage, self.wheelhouse = self.root / 'stage', self.root / 'wheelhouse'
        self.venv = self.root / 'venv'
        self.bin = self.venv / 'bin'
        self.deploy, self.cwd, self.home = self.root / 'deploy', self.root / 'cwd', self.root / 'home'
        self.optimize = bool(sys.flags.optimize)
        self.launches = 0  # installed console-script launches through support.run_cli

    def setup(self):
        try:
            for directory in (self.stage, self.wheelhouse, self.deploy, self.cwd, self.home):
                directory.mkdir()
            for name in ('python_uci_client.py', 'oms_lacal.py'):
                (self.cwd / name).write_text(DECOY)
            self._build()
            self._install()
            for key, (release, log, survivability, _) in BINDING_SETS.items():
                shutil.copytree(bindings_directory(release, with_log=log,
                                                   with_survivability=survivability), self.deploy / key)
        except BaseException:
            self.cleanup()
            raise
        return self

    def cleanup(self):
        self.holder.cleanup()

    def _step(self, argv, env=None):
        argv = [str(item) for item in argv]
        result = subprocess.run(argv, capture_output=True, text=True, timeout=900, env=env,
                                cwd=self.root)
        if result.returncode != 0:
            raise RuntimeError(f'step failed: {argv}\n{result.stdout}{result.stderr}')
        return result

    def _build(self):
        runtime, application = self.stage / 'oms-lacal-runtime', self.stage / 'python-uci-client'
        runtime.mkdir()
        application.mkdir()
        for name in ('pyproject.toml', 'README.md'):
            shutil.copy2(RUNTIME / name, runtime / name)
        shutil.copytree(RUNTIME / 'src', runtime / 'src',
                        ignore=shutil.ignore_patterns('__pycache__', '*.py[cod]', '*.egg-info'))
        for name in ('pyproject.toml', 'README.md', 'python_uci_client.py'):
            shutil.copy2(APP / name, application / name)
        self._step([sys.executable, '-m', 'pip', 'wheel', '--no-deps', '--disable-pip-version-check',
                    '--wheel-dir', self.wheelhouse, runtime, application])
        # The runtime's own pins (websockets==17.2), read from its built metadata.
        self.runtime_requires = wheel_metadata(self.wheelhouse / RUNTIME_WHEEL,
                                               RUNTIME_DIST_INFO).get_all('Requires-Dist')
        self._step([sys.executable, '-m', 'pip', 'download', '--no-deps', '--only-binary=:all:',
                    '--disable-pip-version-check', '--dest', self.wheelhouse, *self.runtime_requires])


    def offline_environment(self):
        return self.environment({
            'PIP_NO_INDEX': '1', 'PIP_CONFIG_FILE': os.devnull,
            'PIP_DISABLE_PIP_VERSION_CHECK': '1', 'PIP_NO_CACHE_DIR': '1',
            'http_proxy': CLOSED_PORT_PROXY, 'https_proxy': CLOSED_PORT_PROXY,
            'HTTP_PROXY': CLOSED_PORT_PROXY, 'HTTPS_PROXY': CLOSED_PORT_PROXY}, optimize=False)

    def _install(self):
        self._step([sys.executable, '-m', 'venv', self.venv])
        offline = self.offline_environment()
        self.install_output = self._step(
            [self.bin / 'python', '-m', 'pip', 'install', '--no-index', '--only-binary=:all:',
             '--find-links', self.wheelhouse, f'{DISTRIBUTION}=={VERSION}'], env=offline).stdout
        self.packages = {item['name'].lower(): item['version'] for item in json.loads(self._step(
            [self.bin / 'python', '-m', 'pip', 'list', '--format=json'], env=offline).stdout)}
        self.pip_check = self._step([self.bin / 'python', '-m', 'pip', 'check'], env=offline).stdout
        self.purelib = Path(self.python(
            'import sysconfig; print(sysconfig.get_path("purelib"))').stdout.strip())

    def deployed(self, release, with_log=False, with_survivability=False):
        """Drop-in for support.bindings_directory: the DEPLOYED copy."""
        return self.deploy / deployment_key(release, with_log, with_survivability)

    def environment(self, extra=None, optimize=None):
        """Minimal operator environment: no PYTHONPATH/PYTHONHOME, no user site."""
        env = {name: os.environ[name] for name in ('LANG', 'LC_ALL', 'TMPDIR') if name in os.environ}
        env.update(PATH=os.pathsep.join((str(self.bin), os.defpath)), HOME=str(self.home),
                   PYTHONNOUSERSITE='1')
        if self.optimize if optimize is None else optimize:
            env['PYTHONOPTIMIZE'] = '1'
        env.update(extra or {})
        return env

    def command(self):
        """The installed console script (support.run_cli launcher protocol)."""
        self.launches += 1
        return [str(self.bin / COMMAND)]

    def run(self, *arguments, extra=None, optimize=None, timeout=60):
        """Run the installed console script synchronously from the decoy directory."""
        return subprocess.run([str(self.bin / COMMAND), *map(str, arguments)], capture_output=True,
                              text=True, timeout=timeout, cwd=self.cwd,
                              env=self.environment(extra, optimize))

    def python(self, code, extra=None, optimize=None):
        """The environment's interpreter with -P: the decoy directory is never on sys.path."""
        return subprocess.run([str(self.bin / 'python'), '-P', '-c', code], capture_output=True,
                              text=True, timeout=60, cwd=self.cwd, check=True,
                              env=self.environment(extra, optimize))

