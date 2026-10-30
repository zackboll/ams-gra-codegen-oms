"""Trusted image provisioning; immutable index, versioned archive origins."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request

INDEX = "842d7165e3e1e644f4d03563e7a4f35c05415af4"


def main():
    seed = Path('/seed')
    seed.mkdir()
    archive = seed / 'gnatcoll.zip'
    with urllib.request.urlopen('https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v26.0.0.zip', timeout=120) as response:
        archive.write_bytes(response.read())
    expected = 'cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b'
    if hashlib.sha512(archive.read_bytes()).hexdigest() != expected:
        raise RuntimeError('GNATCOLL archive origin differs')
    for compiler in ('14', '16'):
        root = seed / compiler
        root.mkdir()
        home = root / 'home'
        home.mkdir()
        settings = root / 'settings'
        settings.mkdir()
        (settings / 'settings.toml').write_text(
            f'[cache]\ndir = "{root}/cache"\n[index]\nauto_update = 0\n'
            '[toolchain]\nassistant = false\n')
        env = dict(os.environ, HOME=str(home))
        alr = ['alr', '-s', str(settings), '-n']
        subprocess.run(alr + ['index', '--add',
                        'git+https://github.com/alire-project/alire-index#' + INDEX, '--name', 'community'],
                       env=env, check=True, timeout=300)
        repo = settings / 'indexes/community/repo'
        if subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip() != INDEX:
            raise RuntimeError('index identity differs')
        for name in ('gnatcoll', 'gnatcoll_minimal'):
            path = repo / f'index/gn/{name}/{name}-26.0.0.toml'
            text = path.read_text()
            url = 'https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v26.0.0.zip'
            if text.count(url) != 1:
                raise RuntimeError('unexpected archive URL')
            path.write_text(text.replace(url, 'http://127.0.0.1:8765/gnatcoll.zip'))
        probe = root / 'probe'
        probe.mkdir()
        name, version = ('gnat_external', '14.2.0') if compiler == '14' else ('gnat_native', '16.1.0')
        (probe / 'alire.toml').write_text(
            'name = "ci_bench_seed"\nversion = "0.1.0-dev"\ndescription = "Trusted seed"\n'
            'licenses = "MIT"\n[[depends-on]]\n'
            f'{name} = "={version}"\ngprbuild = "=26.0.1"\n')
        subprocess.run(alr + ['--force', 'toolchain', '--select', '--local',
                             f'{name}={version}', 'gprbuild=26.0.1'], cwd=probe, env=env,
                       check=True, timeout=600)
        subprocess.run(alr + ['exec', '--', 'gnat', '--version'], cwd=probe, env=env, check=True)
        shutil.rmtree(probe)
    files = {str(p.relative_to(seed)): hashlib.sha256(p.read_bytes()).hexdigest()
             for p in sorted(seed.rglob('*')) if p.is_file() and not p.is_symlink()}
    (seed / 'identity.json').write_text(json.dumps(dict(index=INDEX, files=files), sort_keys=True) + '\n')


if __name__ == '__main__':
    main()