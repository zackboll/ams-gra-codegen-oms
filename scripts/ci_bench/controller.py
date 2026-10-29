"""CI-BENCH-001 trusted main controller (stdlib; never import source candidate).

No candidate-controlled strings become shell commands, environment files,
artifact paths or mounts. Feature code runs in a disposable offline container.
"""
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import threading
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
REPOSITORY = 'zackboll/ams-gra-codegen-oms'
ORDER = [(c, m) for c in ('14', '16') for m in ('original', 'reuse', 'reuse', 'original')]
LIMIT = 16 * 1024 * 1024


def sha(value):
    if not re.fullmatch('[0-9a-f]{40}', value):
        raise ValueError('exact lowercase 40-character commit SHA required')
    return value


def run(command, **kwargs):
    return subprocess.check_output(command, timeout=kwargs.pop('timeout', 120), **kwargs).decode().strip()


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')


def work():
    return Path(os.environ['RUNNER_TEMP']) / 'ci-bench-001'


def authorize(env):
    if (env.get('GITHUB_EVENT_NAME'), env.get('GITHUB_REF'), env.get('GITHUB_REPOSITORY'),
            env.get('RUNNER_ENVIRONMENT')) != ('workflow_dispatch', 'refs/heads/main', REPOSITORY, 'github-hosted'):
        raise ValueError('manual main execution on GitHub-hosted runner required')
    return sha(env['SOURCE_SHA']), sha(env['EXPECTED_WORKFLOW_SHA'])


def prepare():
    base = work()
    base.mkdir(exist_ok=False)
    write(base / 'public/result.json', dict(valid=False, completed=False, disposition='PREPARING'))
    source, workflow = authorize(os.environ)
    if run(['git', '-C', str(ROOT), 'rev-parse', 'HEAD']) != workflow:
        raise ValueError('trusted controller checkout mismatch')
    # A branch-selected workflow must not masquerade as an authorized main run.
    main = run(['git', 'ls-remote', 'https://github.com/' + REPOSITORY + '.git', 'refs/heads/main']).split()[0]
    if main != workflow:
        raise ValueError('dispatch must execute current exact main workflow')
    # Public API, deliberately no token. Validate repository object identity.
    with urllib.request.urlopen('https://api.github.com/repos/' + REPOSITORY + '/commits/' + source, timeout=30) as response:
        metadata = json.load(response)
    if metadata['sha'] != source:
        raise ValueError('repository commit identity mismatch')
    tree = sha(metadata['commit']['tree']['sha'])
    repo = base / 'repository'
    run(['git', 'init', str(repo)])
    run(['git', '-C', str(repo), 'fetch', '--depth=1', 'https://github.com/' + REPOSITORY + '.git', source])
    if run(['git', '-C', str(repo), 'rev-parse', 'FETCH_HEAD', 'FETCH_HEAD^{tree}']).splitlines() != [source, tree]:
        raise ValueError('fetched commit/tree mismatch')
    entries = run(['git', '-C', str(repo), 'ls-tree', '-r', source]).splitlines()
    if any(line.startswith(('120000 ', '160000 ')) for line in entries):
        raise ValueError('candidate symlinks/submodules not authorized')
    archive = base / 'source.tar'
    with archive.open('wb') as output:
        subprocess.run(['git', '-C', str(repo), 'archive', source], stdout=output, check=True, timeout=120)
    destination = base / 'source'
    destination.mkdir()
    if archive.stat().st_size > 128 * 1024 * 1024:
        raise ValueError('candidate source exceeds 128 MiB')
    with tarfile.open(archive) as source_tar:
        source_tar.extractall(destination, filter='data')
    # Only dependencies already in trusted main's lock are provisioned offline.
    if (destination / 'Cargo.lock').read_bytes() != (ROOT / 'Cargo.lock').read_bytes():
        raise ValueError('candidate dependency lock not authorized by main')
    if not (destination / 'Cargo.toml').is_file():
        raise ValueError('candidate lacks generator workspace')
    write(base / 'identity.json', dict(source_commit=source, source_tree=tree, workflow_commit=workflow,
          workflow_tree=run(['git', '-C', str(ROOT), 'rev-parse', 'HEAD^{tree}']),
          source_archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
          run_id=os.environ['GITHUB_RUN_ID'], run_attempt=os.environ['GITHUB_RUN_ATTEMPT']))


def image():
    base = work()
    subprocess.run(['docker', 'build', '--pull', '--tag', 'ci-bench-001:trusted',
                    '--file', str(ROOT / 'scripts/ci_bench/Dockerfile'), str(ROOT)],
                   check=True, timeout=1800)
    image_id = run(['docker', 'image', 'inspect', '--format', '{{.Id}}', 'ci-bench-001:trusted'])
    write(base / 'image.json', dict(image_id=image_id))


def cpu_stat(text):
    result = {}
    for line in text.splitlines():
        fields = line.split()
        if re.fullmatch(r'cpu\d+', fields[0]):
            values = list(map(int, fields[1:9]))
            result[int(fields[0][3:])] = dict(total=sum(values), idle=values[3] + values[4], steal=values[7])
    return result


def cpu_set(text):
    result = set()
    for part in text.strip().split(','):
        ends = list(map(int, part.split('-')))
        result.update(range(ends[0], ends[-1] + 1))
    return result


def allocation():
    allowed = os.sched_getaffinity(0)
    for cpu in sorted(allowed):
        siblings = cpu_set(Path(f'/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list').read_text())
        if siblings <= allowed:
            return cpu, sorted(siblings)
    raise ValueError('complete guest SMT topology unavailable')


def sample_delta(before, after, cpus, workload_usec=0, elapsed=1, maximum_gap=3):
    total = sum(after[c]['total'] - before[c]['total'] for c in cpus)
    idle = sum(after[c]['idle'] - before[c]['idle'] for c in cpus)
    steal = sum(after[c]['steal'] - before[c]['steal'] for c in cpus)
    workload = workload_usec / 1e6 * os.sysconf('SC_CLK_TCK')
    if total <= 0 or elapsed > maximum_gap or elapsed <= 0:
        raise ValueError('missing or gapped CPU samples')
    return dict(external_busy_fraction=max(0, total - idle - workload) / (total / len(cpus)),
                steal_fraction=steal / total, elapsed_seconds=elapsed)


def docker_command(image_id, source, name, cpu, compiler, mode):
    return ['docker', 'create', '--name', name, '--network=none', '--read-only',
            '--cap-drop=ALL', '--security-opt=no-new-privileges', '--user=10001:10001',
            '--pids-limit=256', '--memory=6g', '--memory-swap=6g', '--cpus=1',
            '--cpuset-cpus=' + str(cpu), '--ulimit', 'nofile=1024:1024',
            '--ulimit', 'fsize=2147483648:2147483648',
            '--tmpfs', '/work:rw,exec,nosuid,nodev,size=4g,uid=10001,gid=10001',
            '--tmpfs', '/tmp:rw,noexec,nosuid,nodev,size=256m,uid=10001,gid=10001',
            '--mount', f'type=bind,source={source},target=/source,readonly',
            image_id, 'python3', '/trusted/scripts/ci_bench/worker.py', compiler, mode]


def drain(stream, path):
    """Drain without runner command interpretation; retain at most 16 MiB."""
    count = 0
    with path.open('wb') as output:
        while chunk := stream.read(65536):
            output.write(chunk[:max(0, LIMIT - count)])
            count += len(chunk)
    return count


def campaign():
    base = work()
    identity = json.loads((base / 'identity.json').read_text())
    image_id = json.loads((base / 'image.json').read_text())['image_id']
    cpu, siblings = allocation()
    before = cpu_stat(Path('/proc/stat').read_text())
    time.sleep(10)
    preflight = sample_delta(before, cpu_stat(Path('/proc/stat').read_text()), siblings,
                             elapsed=10, maximum_gap=11)
    write(base / 'public/preflight.json', dict(allocation=dict(cpu=cpu, siblings=siblings), sample=preflight))
    # The ten-second preflight is intentionally one interval, not a lost sample.
    if preflight['external_busy_fraction'] > .05 or preflight['steal_fraction'] > .01:
        raise ValueError('preflight contention gate failed')
    rows = []
    for index, (compiler, mode) in enumerate(ORDER):
        name = f'ci-bench-001-{index}'
        run(docker_command(image_id, base / 'source', name, cpu, compiler, mode))
        samples = []
        started = time.monotonic()
        log = base / f'worker-{index}.log'
        process = subprocess.Popen(['docker', 'start', '--attach', name], stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT)
        reader = threading.Thread(target=drain, args=(process.stdout, log))
        reader.start()
        try:
            # Cgroup counters are host-owned, outside the candidate's namespace.
            pid = 0
            for _ in range(50):
                pid = int(run(['docker', 'inspect', '--format', '{{.State.Pid}}', name]))
                if pid:
                    break
                time.sleep(.1)
            cgroup_path = Path(f'/proc/{pid}/cgroup').read_text().strip().split('::', 1)[1]
            cgroup = Path('/sys/fs/cgroup') / cgroup_path.lstrip('/')
            previous = cpu_stat(Path('/proc/stat').read_text())
            usage = int(dict(line.split() for line in (cgroup / 'cpu.stat').read_text().splitlines())['usage_usec'])
            last = time.monotonic()
            peak = 0
            while process.poll() is None:
                time.sleep(1)
                now = time.monotonic()
                current = cpu_stat(Path('/proc/stat').read_text())
                try:
                    stats = dict(line.split() for line in (cgroup / 'cpu.stat').read_text().splitlines())
                    current_usage = int(stats['usage_usec'])
                    peak = max(peak, int((cgroup / 'memory.peak').read_text()))
                except FileNotFoundError:
                    break
                item = sample_delta(previous, current, siblings, current_usage - usage, now - last)
                item.update(at_seconds=now - started, usage_usec=current_usage,
                            throttled_usec=int(stats.get('throttled_usec', '0')))
                samples.append(item)
                write(base / 'public/current-contention.json', dict(iteration=index, samples=samples))
                previous, usage, last = current, current_usage, now
                if now - started > 1800 or log.stat().st_size >= LIMIT:
                    raise ValueError('per-iteration time/output limit reached')
            process.wait(timeout=10)
            reader.join(timeout=10)
            text = log.read_text(errors='replace')
            emitted = [line.removeprefix('CI_BENCH_RESULT ') for line in text.splitlines()
                       if line.startswith('CI_BENCH_RESULT ')]
            if process.returncode or len(emitted) != 1 or not samples:
                raise ValueError('worker failed or measurement missing')
            if time.monotonic() - last > 3 or samples[0]['at_seconds'] > 3:
                raise ValueError('CPU monitoring start/end gap')
            report = json.loads(emitted[0])
            row = dict(compiler=compiler, mode=mode, wall_seconds=time.monotonic() - started,
                       cpu_seconds=usage / 1e6, cgroup_peak_memory_bytes=peak,
                       samples=samples, candidate_report=report, worker_log=str(log.name))
            rows.append(row)
            write(base / 'public/progress.json', dict(identity=identity, runs=rows))
        finally:
            subprocess.run(['docker', 'rm', '--force', name], capture_output=True, timeout=30, check=False)
            process.wait(timeout=30)
            reader.join(timeout=30)
    valid = validate_rows(rows)
    write(base / 'public/result.json', dict(completed=True, valid=valid, identity=identity,
          image_id=image_id, allocation=dict(cpu=cpu, monitored_smt_siblings=siblings, workers=1),
          preflight=preflight, runs=rows, performance_success=False,
          isolation_scope='guest-visible CPUs/SMT and steal; hypervisor co-tenancy is not observable',
          evidence_trust='host resource counters; candidate-reported workload counts are not attestations'))


def validate_rows(rows):
    if [(r['compiler'], r['mode']) for r in rows] != ORDER:
        raise ValueError('complete two-compiler ABBA required')
    generators, seeds = set(), set()
    dependencies = {c: set() for c in ('14', '16')}
    for row in rows:
        report = row['candidate_report']
        expected = dict(provider=3180 if row['mode'] == 'original' else 318,
                        generated_support=150, bind=30, link=30, execute=30)
        if (report.get('completed') is not True or report.get('counts') != expected
                or report.get('instrumented') is not False or report.get('jobs') != 1
                or report.get('compiler') != row['compiler'] or report.get('mode') != row['mode']):
            raise ValueError('candidate counts/identity/workload gate failed')
        for key in ('generator_sha256', 'seed_sha256', 'dependency_identity_sha256'):
            if not re.fullmatch('[0-9a-f]{64}', report.get(key, '')):
                raise ValueError('missing generator/seed identity')
        generators.add(report['generator_sha256'])
        seeds.add(report['seed_sha256'])
        dependencies[row['compiler']].add(report['dependency_identity_sha256'])
        if not row['samples'] or any(not all(math.isfinite(s[k]) for k in (
                                     'external_busy_fraction', 'steal_fraction', 'elapsed_seconds'))
                                     or s['external_busy_fraction'] > .05 or s['steal_fraction'] > .01
                                     or s['elapsed_seconds'] > 3 or s['elapsed_seconds'] <= 0
                                     or s['external_busy_fraction'] < 0 or s['steal_fraction'] < 0
                                     for s in row['samples']):
            raise ValueError('throughout-campaign contention gate failed')
        if row['cpu_seconds'] <= 0 or row['cgroup_peak_memory_bytes'] <= 0:
            raise ValueError('resource accounting missing')
    if len(generators) != 1 or len(seeds) != 1 or any(len(s) != 1 for s in dependencies.values()):
        raise ValueError('A/B generator or dependency identity differs')
    return True


def package():
    base = work()
    public = base / 'public'
    public.mkdir(parents=True, exist_ok=True)
    start = time.monotonic()
    if not (public / 'result.json').exists():
        write(public / 'result.json', dict(valid=False, completed=False, disposition='SETUP_FAILED'))
    # Only controller-owned bounded files; never recursively follow candidate paths.
    with tarfile.open(public / 'worker-logs.tar.gz', 'w:gz', compresslevel=1) as archive:
        for path in sorted(base.glob('worker-*.log')):
            if path.is_symlink() or path.stat().st_size > LIMIT:
                raise ValueError('unsafe/oversized log')
            archive.add(path, arcname=path.name, recursive=False)
    for name in ('identity.json', 'image.json'):
        if (base / name).exists():
            shutil.copyfile(base / name, public / name)
    write(public / 'packaging.json', dict(wall_seconds=time.monotonic() - start,
          archive_bytes=(public / 'worker-logs.tar.gz').stat().st_size,
          files={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in public.iterdir() if p.is_file()}))
    write(base / 'upload-start.json', dict(monotonic=time.monotonic()))


def receipt():
    base = work()
    start = json.loads((base / 'upload-start.json').read_text())['monotonic']
    write(base / 'upload-receipt.json', dict(outcome=os.environ.get('MEASUREMENTS_OUTCOME'),
          artifact_id=os.environ.get('MEASUREMENTS_ID'), artifact_digest=os.environ.get('MEASUREMENTS_DIGEST'),
          upload_step_envelope_seconds=time.monotonic() - start,
          uploaded_input_bytes=sum(p.stat().st_size for p in (base / 'public').iterdir() if p.is_file()),
          method='monotonic envelope includes inter-step scheduling; exact step/job duration from Jobs API',
          receipt_upload='measure after completion from Jobs API; cannot self-measure final upload'))


def gate():
    base = work()
    result = json.loads((base / 'public/result.json').read_text())
    transport = json.loads((base / 'upload-receipt.json').read_text())
    if (result.get('valid') is not True or transport.get('outcome') != 'success'
            or not transport.get('artifact_id') or not transport.get('artifact_digest')):
        raise ValueError('benchmark invalid/incomplete or mandatory upload failed')
    validate_rows(result['runs'])


if __name__ == '__main__':
    commands = dict(prepare=prepare, image=image, campaign=campaign, package=package, receipt=receipt, gate=gate)
    if len(sys.argv) != 2 or sys.argv[1] not in commands:
        raise SystemExit('expected prepare|image|campaign|package|receipt|gate')
    try:
        commands[sys.argv[1]]()
    except Exception as error:
        if sys.argv[1] in ('prepare', 'image', 'campaign'):
            write(work() / 'public/result.json', dict(valid=False, completed=False,
                  disposition='IDENTITY_CONTENTION_OR_EXECUTION_GATE_FAILED',
                  phase=sys.argv[1], error=str(error)))
        raise