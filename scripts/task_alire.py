"""Private settings for real Task092/093; retry verified archive transport only."""
from pathlib import Path
import os
import shlex
import sys


def configure(scratch, checked, cwd):
    scratch = scratch.resolve()
    settings = scratch / 'alire-settings'
    cache = scratch / 'alire-cache'
    downloader = Path(__file__).resolve().with_name('pinned_download.py')
    # The Deep pack's existing constrained Task102 control already supplies a
    # private -s wrapper over the verified worker installation. Do not add a
    # second -s (Alire rejects duplicates), re-download or mutate its seed.
    # Verify that inherited isolation and the same pinned hook remain exact.
    if os.environ.get('ADA_DEEP_REAL_ALR') and os.environ.get('AMS_GRA_ADA_BUILD_JOBS') == '1':
        alr = ['alr', '-n']
        setup = Path(os.environ['HOME']).resolve().parent
        for name, expected in [('cache.dir', str(setup / 'cache')),
                               ('dependencies.shared', 'false')]:
            actual = checked(alr + ['settings', '--global', '--get', name], cwd,
                             scratch / (name + '.log')).decode().strip()
            if actual.lower() != expected.lower():
                raise RuntimeError('inherited private Alire setting differs: ' + name)
        hook = checked(alr + ['settings', '--global', '--get', 'origins.archive.download_cmd'],
                       cwd, scratch / 'origins.archive.download_cmd.log').decode().strip()
        command = shlex.split(hook)
        if (len(command) != 4 or Path(command[0]).name not in ('python3', 'python3.11', 'python3.13')
                or command[1:] != [str(downloader), '${URL}', '${DEST}']):
            raise RuntimeError('inherited verified Alire download hook differs')
        return alr
    if any(any(c.isspace() for c in str(path)) for path in (sys.executable, downloader, scratch)):
        raise RuntimeError('Alire download command requires whitespace-free paths')
    settings.mkdir()
    cache.mkdir()
    alr = ['alr', '-s', str(settings), '-n']
    for name, value in [
        ('cache.dir', str(cache)),
        ('toolchain.dir', str(cache / 'toolchains')),
        ('dependencies.shared', 'false'),
        ('origins.archive.download_cmd', f'{sys.executable} {downloader} ${{URL}} ${{DEST}}'),
    ]:
        checked(alr + ['settings', '--global', '--set', name, value], cwd,
                scratch / (name + '.log'))
    return alr