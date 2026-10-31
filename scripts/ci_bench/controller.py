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

try:
    from . import contract, cpu
except ImportError:
    import contract, cpu

ROOT = Path(__file__).resolve().parents[2]
REPOSITORY = 'zackboll/ams-gra-codegen-oms'
ORDER = [(c, m) for c in contract.COMPILERS for m in contract.MODES]
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


def compiler_job():
    compiler = os.environ.get('BENCH_COMPILER')
    if compiler not in contract.COMPILERS or os.environ.get('GITHUB_JOB') != contract.JOBS[compiler]:
        raise ValueError('exact registered compiler job required')
    return compiler


def authorize(env):
    if (env.get('GITHUB_EVENT_NAME'), env.get('GITHUB_REF'), env.get('GITHUB_REPOSITORY'),
            env.get('RUNNER_ENVIRONMENT')) != ('workflow_dispatch', 'refs/heads/main', REPOSITORY, 'github-hosted'):
        raise ValueError('manual main execution on GitHub-hosted runner required')
    return sha(env['SOURCE_SHA']), sha(env['EXPECTED_WORKFLOW_SHA'])


def prepare():
    started = time.monotonic()
    base = work()
    base.mkdir(exist_ok=False)
    write(base / 'public/result.json', dict(valid=False, completed=False, disposition='PREPARING'))
    source, workflow = authorize(os.environ)
    compiler = compiler_job()
    validate_source_scope(source)
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
    validate_source_scope(source, tree)
    repo = base / 'repository'
    run(['git', 'init', str(repo)])
    run(['git', '-C', str(repo), 'fetch', '--depth=1', 'https://github.com/' + REPOSITORY + '.git', source])
    if run(['git', '-C', str(repo), 'rev-parse', 'FETCH_HEAD', 'FETCH_HEAD^{tree}']).splitlines() != [source, tree]:
        raise ValueError('fetched commit/tree mismatch')
    entries = run(['git', '-C', str(repo), 'ls-tree', '-r', '-z', source]).split('\0')
    if any(line.startswith(('120000 ', '160000 ')) for line in entries):
        raise ValueError('candidate symlinks/submodules not authorized')
    if any('.git' in line.split('\t', 1)[-1].lower().split('/') for line in entries if line):
        raise ValueError('nested Git metadata not authorized')
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
    url = ('https://api.github.com/repos/' + REPOSITORY + '/actions/runs/' +
           os.environ['GITHUB_RUN_ID'] + '/attempts/' + os.environ['GITHUB_RUN_ATTEMPT'] + '/jobs?per_page=100')
    with urllib.request.urlopen(url, timeout=30) as response:
        jobs = json.load(response)
    matches = [j for j in jobs['jobs'] if j['name'] == contract.JOBS[compiler]
               and j['status'] == 'in_progress' and str(j['run_attempt']) == os.environ['GITHUB_RUN_ATTEMPT']]
    if jobs['total_count'] > 100 or len(matches) != 1:
        raise ValueError('missing/ambiguous authoritative compiler producer')
    producer = matches[0]
    write(base / 'identity.json', dict(source_commit=source, source_tree=tree, workflow_commit=workflow,
          workflow_tree=run(['git', '-C', str(ROOT), 'rev-parse', 'HEAD^{tree}']),
          workflow_sha256=hashlib.sha256((ROOT / '.github/workflows/portable-provider-benchmark.yml').read_bytes()).hexdigest(),
          compiler=compiler, job=contract.JOBS[compiler], job_id=str(producer['id']),
          runner_id=producer['runner_id'], runner_name=producer['runner_name'],
          preparation_seconds=time.monotonic() - started,
          source_archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
          run_id=os.environ['GITHUB_RUN_ID'], run_attempt=os.environ['GITHUB_RUN_ATTEMPT']))


def image():
    base = work()
    started = time.monotonic()
    subprocess.run(['docker', 'build', '--pull', '--tag', 'ci-bench-001:trusted',
                    '--file', str(ROOT / 'scripts/ci_bench/Dockerfile'), str(ROOT)],
                   check=True, timeout=contract.IMAGE_SECONDS)
    image_id = run(['docker', 'image', 'inspect', '--format', '{{.Id}}', 'ci-bench-001:trusted'])
    write(base / 'image.json', dict(image_id=image_id, setup_seconds=time.monotonic() - started))


def validate_source_scope(source, tree=None):
    if source != contract.SOURCE_COMMIT or (tree is not None and tree != contract.SOURCE_TREE):
        raise ValueError('BENCHMARK_SCOPE_INSUFFICIENT: exact reviewed PERF-009 commit/tree required')


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


def validate_topology(topology, selected, allowed):
    if len(selected) != contract.WORKERS or len(set(selected)) != contract.WORKERS or not set(selected) <= set(allowed):
        raise ValueError('CPU quota/cpuset/worker mismatch')
    monitored = set()
    for cpu in selected:
        if cpu not in topology:
            raise ValueError('missing guest CPU topology')
        item = topology[cpu]
        siblings = set(item['siblings'])
        if not siblings or cpu not in siblings or not siblings <= set(allowed):
            raise ValueError('incomplete guest SMT topology')
        for sibling in siblings:
            if sibling not in topology:
                raise ValueError('missing SMT sibling topology')
            other = topology[sibling]
            if (set(other['siblings']) != siblings or other['core'] != item['core']
                    or other['package'] != item['package'] or item['core'] < 0 or item['package'] < 0):
                raise ValueError('inconsistent guest SMT topology')
        monitored.update(siblings)
    cores = {(topology[c]['package'], topology[c]['core']) for c in selected}
    return dict(cpus=selected, monitored_smt_siblings=sorted(monitored), workers=contract.WORKERS,
                guest_core_count=len(cores), topology={str(c): topology[c] for c in sorted(monitored)},
                physical_isolation_proven=False)


def allocation():
    allowed = os.sched_getaffinity(0)
    topology = {}
    for cpu in sorted(allowed):
        root = Path(f'/sys/devices/system/cpu/cpu{cpu}/topology')
        topology[cpu] = dict(siblings=sorted(cpu_set((root / 'thread_siblings_list').read_text())),
                             core=int((root / 'core_id').read_text()),
                             package=int((root / 'physical_package_id').read_text()))
    # Prefer two distinct guest cores; a complete two-thread guest core is
    # also allowed, explicitly labeled as one guest core, never two physical cores.
    pairs = [(a, b) for a in sorted(allowed) for b in sorted(allowed) if a < b]
    pairs.sort(key=lambda p: topology[p[0]]['core'] == topology[p[1]]['core'])
    for pair in pairs:
        try:
            return validate_topology(topology, list(pair), allowed)
        except ValueError:
            continue
    raise ValueError('qualified two-vCPU allocation unavailable')


def validate_host_resources(mem_available, disk_available):
    if mem_available < 8 * 1024**3 or disk_available < 10 * 1024**3:
        raise ValueError('insufficient runner memory/disk headroom')


def sample_delta(before, after, cpus, workload_usec=0, elapsed=1, maximum_gap=3):
    total = sum(after[c]['total'] - before[c]['total'] for c in cpus)
    idle = sum(after[c]['idle'] - before[c]['idle'] for c in cpus)
    steal = sum(after[c]['steal'] - before[c]['steal'] for c in cpus)
    workload = workload_usec / 1e6 * os.sysconf('SC_CLK_TCK')
    if total <= 0 or elapsed > maximum_gap or elapsed <= 0 or workload_usec < 0 or steal < 0 or idle < 0:
        raise ValueError('missing or gapped CPU samples')
    return dict(external_busy_fraction=max(0, total - idle - workload) / (total / len(cpus)),
                steal_fraction=steal / total, elapsed_seconds=elapsed)


def docker_command(image_id, source, name, cpus, compiler, mode):
    if (not re.fullmatch(r'sha256:[0-9a-f]{64}', image_id)
            or not re.fullmatch(r'ci-bench-001-\d+', name)
            or compiler not in ('14', '16') or mode not in ('original', 'reuse')
            or len(set(cpus)) != contract.WORKERS or any(type(c) is not int or c < 0 for c in cpus)):
        raise ValueError('invalid trusted Docker arguments or CPU allocation')
    return ['docker', 'create', '--interactive', '--name', name, '--network=none', '--read-only', '--log-driver=none',
            '--cap-drop=ALL', '--security-opt=no-new-privileges', '--user=10001:10001',
            '--pids-limit=256', '--memory=6g', '--memory-swap=6g', '--cpus=2',
            '--cpuset-cpus=' + ','.join(map(str, cpus)), '--ulimit', 'nofile=1024:1024',
            '--ulimit', 'fsize=2147483648:2147483648',
            '--tmpfs', '/work:rw,exec,nosuid,nodev,size=4g,uid=10001,gid=10001',
            '--tmpfs', '/tmp:rw,noexec,nosuid,nodev,size=256m,uid=10001,gid=10001',
            '--mount', f'type=bind,source={source},target=/source,readonly',
            image_id, 'python3', '/trusted/scripts/ci_bench/supervisor.py', compiler, mode]


def drain(stream, path, ready=None):
    """Drain without runner command interpretation; retain at most 16 MiB."""
    count = 0
    with path.open('wb') as output:
        pending = b''
        while chunk := stream.read1(65536) if hasattr(stream, 'read1') else stream.read(65536):
            output.write(chunk[:max(0, LIMIT - count)])
            output.flush()
            count += len(chunk)
            pending = (pending + chunk)[-LIMIT:]
            if ready is not None and b'\nCI_BENCH_WORKER_EXITED\n' in pending:
                ready.set()
    return count


def cgroup_stats(cgroup):
    stats = {k: int(v) for k, v in (line.split() for line in (cgroup / 'cpu.stat').read_text().splitlines())}
    events = {k: int(v) for k, v in (line.split() for line in (cgroup / 'memory.events').read_text().splitlines())}
    return dict(usage_usec=stats['usage_usec'], throttled_usec=stats['throttled_usec'],
                memory_peak=int((cgroup / 'memory.peak').read_text()), oom=events['oom'],
                oom_kill=events['oom_kill'])


def validate_cgroup_limits(cgroup, cpus):
    values = {key: (cgroup / key).read_text().strip() for key in (
        'cpu.max', 'cpuset.cpus.effective', 'memory.max', 'memory.swap.max', 'pids.max')}
    validate_limit_values(values, cpus)
    return values


def validate_limit_values(values, cpus):
    quota, period = values['cpu.max'].split()
    if (quota == 'max' or int(quota) != contract.WORKERS * int(period)
            or int(period) <= 0 or cpu_set(values['cpuset.cpus.effective']) != set(cpus)
            or int(values['memory.max']) != contract.MEMORY_BYTES
            or int(values['memory.swap.max']) != 0 or int(values['pids.max']) != 256):
        raise ValueError('actual cgroup CPU/memory/process allocation mismatch')


def validate_samples(samples, wall):
    previous_at, previous_usage, previous_throttle = 0, 0, 0
    if not samples or not 0 < wall <= contract.ITERATION_SECONDS + 3:
        raise ValueError('missing samples or iteration timeout')
    for sample in samples:
        keys = ('at_seconds', 'usage_usec', 'throttled_usec', 'external_busy_fraction',
                'steal_fraction', 'elapsed_seconds')
        if any(not math.isfinite(sample[k]) or sample[k] < 0 for k in keys):
            raise ValueError('invalid cgroup/contention sample')
        delta = sample['at_seconds'] - previous_at
        if (not 0 < delta <= 3 or abs(delta - sample['elapsed_seconds']) > .05
                or sample['usage_usec'] < previous_usage or sample['throttled_usec'] < previous_throttle
                or sample['external_busy_fraction'] > .05 or sample['steal_fraction'] > .01
                or (sample['throttled_usec'] - previous_throttle) / (delta * 1e6)
                > contract.MAX_THROTTLED_FRACTION):
            raise ValueError('gapped/throttled/contended campaign')
        previous_at, previous_usage, previous_throttle = (sample['at_seconds'], sample['usage_usec'],
                                                         sample['throttled_usec'])
    if not 0 <= wall - previous_at <= 3:
        raise ValueError('missing final cgroup samples')


def campaign():
    base = work()
    compiler = compiler_job()
    identity = json.loads((base / 'identity.json').read_text())
    image_id = json.loads((base / 'image.json').read_text())['image_id']
    assigned = allocation()
    cpus, siblings = assigned['cpus'], assigned['monitored_smt_siblings']
    meminfo = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    host_resources = dict(available_memory_bytes=int(meminfo['MemAvailable'].split()[0]) * 1024,
                          free_disk_bytes=shutil.disk_usage(base).free)
    validate_host_resources(host_resources['available_memory_bytes'], host_resources['free_disk_bytes'])
    preflight_journal = cpu.Journal(base / 'public/cpu-preflight.jsonl', cpu.context(identity, assigned, -1))
    preflight_journal.event('coverage_start')
    before = preflight_journal.snapshot(0, state='preflight')
    time.sleep(10)
    after = preflight_journal.snapshot(1, state='preflight')
    preflight_journal.event('coverage_end')
    header = dict(preflight_journal.header, coverage_start_ns=before['host']['begin_ns'])
    preflight = cpu.calculate(before, after, header, host_only=True)
    cpu.replay(preflight_journal.records, identity, assigned, -1, [preflight])
    preflight_journal.close()
    write(base / 'public/preflight.json', dict(allocation=assigned, sample=preflight))
    # The ten-second preflight is intentionally one interval, not a lost sample.
    if preflight['external_busy_fraction'] > .05 or preflight['steal_fraction'] > .01:
        raise ValueError('preflight contention gate failed')
    rows = []
    for index, mode in enumerate(contract.MODES):
        name = f'ci-bench-001-{index}'
        if index == 0:
            context = cpu.context(identity, assigned, index)
            journal = cpu.Journal(base / f'public/cpu-{index}.jsonl', context)
            journal.event('coverage_start')
            previous = journal.snapshot(0, state='not_started')
        else:
            # Reuse the previous terminal host endpoint exactly: no gap during
            # docker rm, next create, or inter-iteration controller work.
            context = cpu.context(identity, assigned, index, bridge['host']['begin_ns'])
            journal = cpu.Journal(base / f'public/cpu-{index}.jsonl', context)
            journal.append(dict(type='event', name='coverage_start', at_ns=bridge['host']['begin_ns']))
            previous = dict(bridge, sequence=0, state='not_started')
            journal.append(previous)
        header = dict(journal.header, coverage_start_ns=previous['host']['begin_ns'])
        journal.event('create_requested')
        container_id = run(docker_command(image_id, base / 'source', name, cpus, compiler, mode))
        journal.event('created', container_id=container_id)
        samples = []
        started = previous['host']['begin_ns'] / 1e9
        log = base / f'worker-{index}.log'
        ready = threading.Event()
        journal.event('start_requested')
        process = subprocess.Popen(['docker', 'start', '--attach', '--interactive', name], stdout=subprocess.PIPE,
                                   stdin=subprocess.PIPE, stderr=subprocess.STDOUT)
        reader = threading.Thread(target=drain, args=(process.stdout, log, ready))
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
            limits = validate_cgroup_limits(cgroup, cpus)
            group_identity = cgroup.stat()
            journal.event('cgroup_discovered', path=str(cgroup), inode=group_identity.st_ino,
                          device=group_identity.st_dev, pid=pid)
            # Account from container start, not the first observed nonzero counter.
            usage = 0
            last = time.monotonic()
            if last - started > 3:
                raise ValueError('cgroup startup monitoring gap')
            last = started
            peak = 0
            final_stats = None
            while process.poll() is None:
                # Target the next one-second boundary, rather than adding a
                # second after JSON persistence/observer work every iteration.
                time.sleep(max(0, 1 - (time.monotonic_ns()-previous['host']['begin_ns']) / 1e9))
                current = journal.snapshot(len(samples) + 1, cgroup)
                now = current['host']['begin_ns'] / 1e9
                stats = cgroup_stats(cgroup)
                peak = max(peak, stats['memory_peak'])
                stats.update(usage_usec=current['cgroup']['counters']['usage_usec'],
                             throttled_usec=current['cgroup']['counters']['throttled_usec'])
                final_stats = stats
                item = cpu.calculate(previous, current, header)
                samples.append(item)
                write(base / 'public/current-contention.json', dict(iteration=index, samples=samples))
                previous, usage, last = current, stats['usage_usec'], now
                if now - started > contract.ITERATION_SECONDS or log.stat().st_size >= LIMIT:
                    raise ValueError('per-iteration time/output limit reached')
                if ready.is_set() and current['cgroup']['procs'] == [pid]:
                    # Snapshot host-owned counters while the worker is alive.
                    # Release only after sampling; final evidence cannot disappear
                    # with cgroup removal between polls.
                    journal.event('worker_exited', snapshot=current['sequence'])
                    journal.event('accounted')
                    process.stdin.write(b'accounted\n')
                    process.stdin.flush()
                    break
            process.wait(timeout=10)
            journal.event('container_exited')
            # Keep one-second observation cadence through shutdown; a sub-tick
            # tail cannot distinguish an actual CPU stall from quantization.
            time.sleep(max(0, 1 - (time.monotonic_ns()-previous['host']['begin_ns']) / 1e9))
            current = journal.snapshot(len(samples) + 1, state='shutdown')
            bridge = current
            samples.append(cpu.calculate(previous, current, header, host_only=True))
            write(base / 'public/current-contention.json', dict(iteration=index, samples=samples))
            journal.event('coverage_end')
            cpu.replay(journal.records, identity, assigned, index, samples)
            reader.join(timeout=10)
            text = log.read_text(errors='replace')
            emitted = [line.removeprefix('CI_BENCH_RESULT ') for line in text.splitlines()
                       if line.startswith('CI_BENCH_RESULT ')]
            if process.returncode or len(emitted) != 1 or not samples:
                raise ValueError('worker failed or measurement missing')
            if time.monotonic() - last > 3 or samples[0]['at_seconds'] > 3:
                raise ValueError('CPU monitoring start/end gap')
            report = json.loads(emitted[0])
            state = json.loads(run(['docker', 'inspect', '--format', '{{json .State}}', name]))
            if state['OOMKilled'] or state['ExitCode'] != 0 or not final_stats:
                raise ValueError('OOM/incomplete worker')
            if final_stats['oom'] or final_stats['oom_kill']:
                raise ValueError('cgroup OOM event')
            row = dict(compiler=compiler, mode=mode, iteration=index, wall_seconds=time.monotonic() - started,
                       cpu_seconds=usage / 1e6, cgroup_peak_memory_bytes=peak,
                       final_cgroup=final_stats,
                       cgroup_limits=limits,
                       allocation=assigned, oom=False, exit_code=state['ExitCode'],
                       samples=samples, cpu_raw=journal.reference(),
                       candidate_report=report, worker_log=str(log.name))
            rows.append(row)
            write(base / 'public/progress.json', dict(identity=identity, runs=rows))
        finally:
            journal.close()
            subprocess.run(['docker', 'rm', '--force', name], capture_output=True, timeout=30, check=False)
            process.wait(timeout=30)
            reader.join(timeout=30)
    validate_rows(rows, compiler)
    write(base / 'public/result.json', dict(completed=True, valid=True, identity=identity,
          image_id=image_id, allocation=assigned, budget=contract.budget(),
          host_resources=host_resources,
          preflight=preflight, preflight_raw=preflight_journal.reference(), runs=rows, performance_success=False,
          isolation_scope='guest-visible CPUs/SMT and steal; hypervisor co-tenancy is not observable',
          evidence_trust='host resource counters; candidate-reported workload counts are not attestations'))


def validate_cells(report):
    names = ('ArraySample', 'Array_Encode_Hex_Binary', 'Encode_Hex_Binary',
             'Array_encode_hex_binary', 'Array_Decode_Hex_Binary')
    cells = {(profile, name, policy) for profile in ('oam', 'qualified')
             for name in names for policy in ('normal', 'Check', 'Ignore')}
    records = report.get('records', [])
    actual = [(r['profile'], r['name'], r['policy']) for r in records]
    if (len(actual) != 30 or set(actual) != cells
            or any(r['compiler'] != ('14.2.0' if report['compiler'] == '14' else '16.1.0')
                   or r['family'] != 'portable' or r['intentional_red'] is not False
                   or r['compiler_exit'] != 0 for r in records)):
        raise ValueError('incomplete/duplicate/wrong compiler consumer cells')
    builds = report.get('builds', [])
    expected = {f'{p}-{n}-{a}-build' for p, n, a in cells}
    consumers = [b for b in builds if not b['cell'].startswith('provider-')]
    providers = [b for b in builds if b['cell'].startswith('provider-')]
    if len(consumers) != 30 or {b['cell'] for b in consumers} != expected:
        raise ValueError('missing/duplicate consumer compilation/binding/linking')
    for build in consumers:
        namespace = 'task093' if build['cell'].startswith('qualified-') else 'oam'
        owned = {'helper_demand_consumer.adb', 'service_codec.adb', 'programs.ads',
                 f'programs-{namespace}.ads', 'ams_gra_oms_json_guard.adb'}
        units = build['units']
        if (len(units) != (111 if report['mode'] == 'original' else 5)
                or len(units) != len(set(units)) or not owned <= set(units)
                or build.get('bind') is not True or build.get('link') is not True
                or build.get('execute_marker') != 'TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n'
                or not re.fullmatch('[0-9a-f]{64}', build.get('source_sha256', ''))):
            raise ValueError('source/compiler/binder/linker/execution evidence differs')
    if report['mode'] == 'reuse':
        if len(providers) != 3 or {b['cell'] for b in providers} != {
                f'provider-{p}-build' for p in ('normal', 'Check', 'Ignore')}:
            raise ValueError('provider policy preparation incomplete')
        if any(len(b['units']) != 106 or len(set(b['units'])) != 106 for b in providers):
            raise ValueError('provider compilation incomplete')
    elif providers:
        raise ValueError('unexpected provider preparation')


def validate_rows(rows, compiler=None):
    if compiler is not None and compiler not in contract.COMPILERS:
        raise ValueError('unknown compiler')
    order = ORDER if compiler is None else [(compiler, m) for m in contract.MODES]
    if [(r['compiler'], r['mode']) for r in rows] != order:
        raise ValueError('complete compiler ABBA required')
    generators = set()
    seeds = {c: set() for c, _ in order}
    dependencies = {c: set() for c, _ in order}
    source_identities = {}
    for index, row in enumerate(rows):
        if row['iteration'] != index % 4:
            raise ValueError('missing/reordered iteration identity')
        report = row['candidate_report']
        expected = dict(provider=3180 if row['mode'] == 'original' else 318,
                        generated_support=150, bind=30, link=30, execute=30)
        if (report.get('completed') is not True or report.get('counts') != expected
                or report.get('instrumented') is not False or report.get('jobs') != contract.WORKERS
                or report.get('measurement_scope') != contract.SCOPE
                or report.get('source_commit') != contract.SOURCE_COMMIT
                or report.get('source_tree') != contract.SOURCE_TREE
                or report.get('compiler') != row['compiler'] or report.get('mode') != row['mode']):
            raise ValueError('candidate counts/identity/workload gate failed')
        for key in ('generator_sha256', 'seed_sha256', 'dependency_identity_sha256'):
            if not re.fullmatch('[0-9a-f]{64}', report.get(key, '')):
                raise ValueError('missing generator/seed identity')
        generators.add(report['generator_sha256'])
        seeds[row['compiler']].add(report['seed_sha256'])
        dependencies[row['compiler']].add(report['dependency_identity_sha256'])
        validate_samples(row['samples'], row['wall_seconds'])
        assigned = row['allocation']
        validate_limit_values(row['cgroup_limits'], assigned['cpus'])
        final = row['final_cgroup']
        if (final['oom'] != 0 or final['oom_kill'] != 0
                or final['usage_usec'] != row['samples'][-1]['usage_usec']
                or final['throttled_usec'] != row['samples'][-1]['throttled_usec']
                or final['memory_peak'] != row['cgroup_peak_memory_bytes']):
            raise ValueError('final live-cgroup accounting mismatch')
        topology = {int(c): v for c, v in assigned['topology'].items()}
        if assigned != validate_topology(topology, assigned['cpus'], assigned['monitored_smt_siblings']):
            raise ValueError('allocation evidence differs')
        first = next(r for r in rows if r['compiler'] == row['compiler'])
        if row['oom'] is not False or row['exit_code'] != 0 or assigned != first['allocation']:
            raise ValueError('OOM/exit/allocation mismatch')
        validate_cells(report)
        for build in report['builds']:
            if build['cell'].startswith('provider-'):
                continue
            key = (row['compiler'], build['cell'])
            if key in source_identities and source_identities[key] != build['source_sha256']:
                raise ValueError('A/B generated source identity changed')
            source_identities[key] = build['source_sha256']
        if (not math.isfinite(row['cpu_seconds']) or row['cpu_seconds'] <= 0
                or row['cgroup_peak_memory_bytes'] <= 0
                or row['cgroup_peak_memory_bytes'] > contract.MEMORY_BYTES
                or abs(row['cpu_seconds'] - row['samples'][-1]['usage_usec'] / 1e6) > .000001):
            raise ValueError('resource accounting missing')
        for key in ('helper_wall_seconds', 'helper_child_cpu_seconds', 'generator_build_seconds', 'setup_seconds'):
            if not math.isfinite(report.get(key, -1)) or report.get(key, -1) <= 0:
                raise ValueError('missing helper/generator/setup measurements')
        if (report['helper_wall_seconds'] > row['wall_seconds']
                or report['generator_build_seconds'] > report['setup_seconds']
                or report['helper_wall_seconds'] + report['setup_seconds'] > row['wall_seconds']):
            raise ValueError('inconsistent timing scopes')
    if len(generators) != 1 or any(len(s) != 1 for s in [*seeds.values(), *dependencies.values()]):
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
          run_id=os.environ.get('GITHUB_RUN_ID'), run_attempt=os.environ.get('GITHUB_RUN_ATTEMPT'),
          job=os.environ.get('GITHUB_JOB'), compiler=os.environ.get('BENCH_COMPILER'),
          job_id=json.loads((base / 'identity.json').read_text())['job_id'] if (base / 'identity.json').exists() else None,
          artifact_id=os.environ.get('MEASUREMENTS_ID'), artifact_digest=os.environ.get('MEASUREMENTS_DIGEST'),
          upload_step_envelope_seconds=time.monotonic() - start,
          uploaded_input_bytes=sum(p.stat().st_size for p in (base / 'public').iterdir() if p.is_file()),
          method='monotonic envelope includes inter-step scheduling; exact step/job duration from Jobs API',
          receipt_upload='measure after completion from Jobs API; cannot self-measure final upload'))


def validate_result(result, transport, identity, raw_loader=None):
    if (result.get('valid') is not True or result.get('completed') is not True
            or transport.get('outcome') != 'success'
            or not re.fullmatch('[1-9][0-9]*', transport.get('artifact_id') or '')
            or not re.fullmatch('[0-9a-f]{64}', transport.get('artifact_digest') or '')
            or any(transport.get(k) != identity.get(k) or not transport.get(k)
                   for k in ('run_id', 'run_attempt', 'job', 'job_id', 'compiler'))):
        raise ValueError('benchmark invalid/incomplete or mandatory upload failed')
    if result['identity'] != identity:
        raise ValueError('stale/cross-source result identity')
    validate_source_scope(identity['source_commit'], identity['source_tree'])
    if (result.get('budget') != contract.budget() or result.get('performance_success') is not False
            or result.get('allocation') != result['runs'][0]['allocation']):
        raise ValueError('budget/allocation/performance scope mismatch')
    validate_host_resources(result['host_resources']['available_memory_bytes'],
                            result['host_resources']['free_disk_bytes'])
    if any(identity[k] != transport[k] for k in ('run_id', 'run_attempt')):
        raise ValueError('stale result producer attempt')
    preflight = result['preflight']
    if (not all(math.isfinite(preflight[k]) and preflight[k] >= 0
                for k in ('external_busy_fraction', 'steal_fraction', 'elapsed_seconds'))
            or preflight['external_busy_fraction'] > .05 or preflight['steal_fraction'] > .01
            or not 10 <= preflight['elapsed_seconds'] <= 11):
        raise ValueError('invalid preflight evidence')
    validate_rows(result['runs'], identity['compiler'])
    if raw_loader is None or 'preflight_raw' not in result or any('cpu_raw' not in row for row in result['runs']):
        raise ValueError('complete CPU-v2 raw evidence required; legacy is diagnostic only')
    cpu.replay(cpu.resolve(result['preflight_raw'], raw_loader, 'cpu-preflight.jsonl'),
               identity, result['allocation'], -1, [preflight])
    for row in result['runs']:
        cpu.replay(cpu.resolve(row['cpu_raw'], raw_loader, f"cpu-{row['iteration']}.jsonl"),
                   identity, row['allocation'], row['iteration'], row['samples'])


def gate():
    base = work()
    compiler = compiler_job()
    identity = json.loads((base / 'identity.json').read_text())
    if (os.environ.get('RECEIPT_OUTCOME') != 'success' or identity['compiler'] != compiler
            or any(identity[k] != os.environ.get(e) for k, e in (
                ('run_id', 'GITHUB_RUN_ID'), ('run_attempt', 'GITHUB_RUN_ATTEMPT'), ('job', 'GITHUB_JOB')))):
        raise ValueError('mandatory receipt upload or producer identity failed')
    validate_result(json.loads((base / 'public/result.json').read_text()),
                    json.loads((base / 'upload-receipt.json').read_text()), identity,
                    lambda name: (base / 'public' / name).read_bytes())


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