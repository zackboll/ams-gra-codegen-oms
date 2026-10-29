"""Unprivileged offline worker; never import candidate Python.

The host owns resource and contention accounting. The generator, generated
programs, GPR files and candidate dependency hooks remain untrusted execution.
"""
import functools
import hashlib
import http.server
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import shutil
import subprocess
import sys
import threading
import time


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def compiled_units(text):
    units = re.findall(rb'\[(?:Ada|C|Asm)\]\s+(\S+)', text)
    return [u for u in units if not u.startswith((b'b__', b'b~')) and not u.endswith(b'.ali')]


def main():
    compiler, mode = sys.argv[1:3]
    smoke = sys.argv[3:] == ['--smoke']
    if sys.argv[3:] and not smoke:
        raise ValueError('unexpected worker arguments')
    if compiler not in ('14', '16') or mode not in ('original', 'reuse'):
        raise ValueError('invalid worker arguments')
    state = Path('/work/state')
    shutil.copytree('/seed/' + compiler, state, symlinks=True)
    shutil.copytree('/usr/local/cargo', '/work/cargo', symlinks=True)
    settings = state / 'settings/settings.toml'
    settings.write_text(settings.read_text().replace('/seed/' + compiler, str(state)))
    # Both sides fetch identical verified archive bytes, without outside network.
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 8765), functools.partial(
        http.server.SimpleHTTPRequestHandler, directory='/seed'))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    os.environ.update(HOME=str(state / 'home'), CARGO_HOME='/work/cargo',
                      CARGO_TARGET_DIR='/work/target', CARGO_BUILD_JOBS='1',
                      PATH='/work/bin:' + os.environ['PATH'], PYTHONDONTWRITEBYTECODE='1',
                      TMPDIR='/work/tmp')
    Path('/work/tmp').mkdir()
    Path('/work/bin').mkdir()
    alr = Path('/work/bin/alr')
    alr.write_text('#!/bin/sh\nexec /opt/alire/bin/alr -s /work/state/settings "$@"\n')
    alr.chmod(0o755)
    subprocess.run(['cargo', '+1.95.0', 'build', '--offline', '--locked',
                    '-p', 'ams-gra-codegen-oms'], cwd='/source', check=True, timeout=600)
    generator = Path('/work/target/debug/ams-gra-codegen-oms')
    generator_hash = hashlib.sha256(generator.read_bytes()).hexdigest()
    sys.path.insert(0, '/trusted/scripts')
    from perf005_ci import validate_toolchain
    from perf005_provider import Pilot
    runner = load('benchmark_runner', '/trusted/scripts/check-task093-helper-demand.py')
    runner.ROOT = Path('/source')
    if smoke:
        runner.NAMES = runner.NAMES[:1]
    scratch = Path('/work/campaign')
    counts = dict(provider=0, generated_support=0, bind=0, link=0, execute=0)
    build_records = []

    def checked(command, cwd, stem, expected=0):
        building = command[:4] == ['alr', '-n', 'build', '--'] or (
            command[:5] == ['alr', '-n', 'exec', '--', 'gprbuild'])
        if building:
            pos = command.index('-cargs:Ada') if '-cargs:Ada' in command else len(command)
            command = command[:pos] + ['-j1'] + command[pos:]
        start = time.monotonic()
        log = stem.with_suffix('.log')
        with log.open('xb') as output:
            result = subprocess.run(command, cwd=cwd, stdin=subprocess.DEVNULL,
                                    stdout=output, stderr=subprocess.STDOUT)
        text = log.read_bytes()
        stem.with_suffix('.command.json').write_text(json.dumps(dict(
            command=command, exit=result.returncode, elapsed_seconds=time.monotonic() - start)) + '\n')
        if expected is not None and result.returncode != expected:
            raise RuntimeError(f'command failed: {stem.name}: {result.returncode}')
        if stem.name.endswith('-provider-identity'):
            validate_toolchain(text, compiler)
        if building and '-f' in command:
            # gprbuild may display its generated binder body as an Ada compile;
            # binder compilations are accounted in bind, not source inventory.
            units = compiled_units(text)
            provider = stem.name.startswith('provider-')
            wanted = 106 if provider else (111 if mode == 'original' else 5)
            if len(units) != wanted:
                print(text.decode(errors='replace'), flush=True)
                raise RuntimeError(f'compilation count differs: {stem.name}: {len(units)} != {wanted}')
            counts['provider'] += len(units) if provider else (106 if mode == 'original' else 0)
            counts['generated_support'] += 0 if provider else 5
            if not provider:
                if b'[gprbind]' not in text or b'[link]' not in text:
                    raise RuntimeError('bind/link evidence missing')
                counts['bind'] += 1
                counts['link'] += 1
            build_records.append(dict(cell=stem.name, units=[u.decode() for u in units]))
        if stem.name.endswith('-execute'):
            if text != b'TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n':
                raise RuntimeError('execution marker differs')
            counts['execute'] += 1
        return result.returncode, text

    pilot = None
    if mode == 'reuse':
        # Same already-main campaign-local provider mechanism, benchmark-only.
        # Do not import or change PERF-009's implementation/certificate.
        pilot = Pilot(scratch, compiler, checked, jobs=1)

    def adapted(command, cwd, stem, expected=0):
        if pilot and command[:5] == ['alr', '-n', 'build', '--', '-f']:
            return pilot.build(command, cwd, stem, expected)
        return checked(command, cwd, stem, expected)

    runner.checked = adapted
    sys.argv = ['check-task093-helper-demand.py', '--compiler', compiler, '--family', 'portable',
                '--scratch', str(scratch), '--generator', str(generator)]
    if smoke:
        sys.argv += ['--profiles', 'oam', '--policies', 'normal']
    started = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    runner.main()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    expected = dict(provider=3180 if mode == 'original' else 318,
                    generated_support=150, bind=30, link=30, execute=30)
    if smoke:
        expected = dict(provider=106 if mode == 'original' else 318,
                        generated_support=5, bind=1, link=1, execute=1)
    if counts != expected:
        raise RuntimeError('incomplete benchmark counts')
    # Independently resolve compiler/runtime/source identities on both sides.
    verifier = pilot or Pilot(scratch, compiler, checked, jobs=1)
    from perf005_provider import sources
    dependency_identities = []
    for consumer in sorted(scratch.glob('oam-*')) + sorted(scratch.glob('qualified-*')):
        if consumer.is_dir() and (consumer / 'consumer.gpr').is_file():
            identity = verifier.configuration(consumer, sources(consumer))
            dependency_identities.append(identity)
    if len(dependency_identities) != (1 if smoke else 10) or any(i != dependency_identities[0] for i in dependency_identities):
        raise RuntimeError('consumer dependency/compiler identities differ')
    result = json.loads((scratch / 'result.json').read_text())
    records = result['records']
    if not result['completed'] or result['red'] or len(records) != (1 if smoke else 30):
        raise RuntimeError('incomplete matrix')
    if hashlib.sha256(generator.read_bytes()).hexdigest() != generator_hash:
        raise RuntimeError('generator mutated during campaign')
    # Emission is bounded and is treated as candidate-reported evidence only.
    report = dict(completed=not smoke, smoke=smoke, compiler=compiler, mode=mode, instrumented=False, jobs=1,
                  generator_sha256=generator_hash,
                  seed_sha256=hashlib.sha256(Path('/seed/identity.json').read_bytes()).hexdigest(),
                  dependency_identity_sha256=hashlib.sha256(json.dumps(
                      dependency_identities[0], sort_keys=True).encode()).hexdigest(),
                  helper_wall_seconds=time.monotonic() - started,
                  helper_child_cpu_seconds=after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
                  largest_child_peak_rss_kib=after.ru_maxrss, counts=counts, builds=build_records,
                  records=records, candidate_reported=True)
    print('CI_BENCH_RESULT ' + json.dumps(report, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()