"""Private settings for real Task092/093; retry verified archive transport only."""
from pathlib import Path
import sys


def configure(scratch, checked, cwd):
    scratch = scratch.resolve()
    settings = scratch / 'alire-settings'
    cache = scratch / 'alire-cache'
    downloader = Path(__file__).resolve().with_name('pinned_download.py')
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