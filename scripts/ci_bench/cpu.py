"""CI-BENCH-004 trusted raw CPU acquisition and producer-side calculation.

No candidate imports. USER_HZ quantization is a consistency bound, not permission
to discount an excursion. Unknown kernel accounting lag is never corrected.
"""
import json
import hashlib
import math
import os
from pathlib import Path
import re
import subprocess
import time
from fractions import Fraction

SCHEMA = 'ci-bench-cpu-v2'
FIELDS = ('user', 'nice', 'system', 'idle', 'iowait', 'irq', 'softirq', 'steal', 'guest', 'guest_nice')
REQUIRED = {'usage_usec', 'user_usec', 'system_usec', 'nr_periods', 'nr_throttled', 'throttled_usec'}
EVENTS = ('coverage_start', 'create_requested', 'created', 'start_requested',
          'cgroup_discovered', 'worker_exited', 'accounted', 'container_exited', 'coverage_end')


def integer(value, minimum=0):
    if type(value) is not int or value < minimum:
        raise ValueError('invalid integer counter/timestamp')
    return value


def parse_host(text, monitored):
    cpus = {}
    for line in text.splitlines():
        parts = line.split()
        if parts and re.fullmatch(r'cpu[0-9]+', parts[0]):
            cpu = int(parts[0][3:])
            if cpu in cpus or len(parts) != 11:
                raise ValueError('duplicate or truncated CPU line')
            cpus[cpu] = [integer(int(v)) for v in parts[1:]]
    if not set(monitored) <= set(cpus):
        raise ValueError('missing monitored/SMT CPU')
    return {str(c): cpus[c] for c in monitored}


def parse_cgroup(text):
    counters = {}
    for line in text.splitlines():
        parts = line.split()
        # Kernel extensions such as core_sched.force_idle_usec use dotted
        # names. Require nonempty lowercase/underscore components, not text.
        if (len(parts) != 2 or not re.fullmatch(r'[a-z_]+(?:\.[a-z_]+)*', parts[0])
                or not re.fullmatch(r'[0-9]+', parts[1]) or parts[0] in counters):
            raise ValueError('duplicate/truncated cgroup counter')
        counters[parts[0]] = integer(int(parts[1]))
    if not REQUIRED <= set(counters):
        raise ValueError('incomplete cgroup CPU counters')
    return counters


def context(identity, allocation, iteration, origin_ns=None):
    hz = os.sysconf('SC_CLK_TCK')
    return dict(type='context', schema=SCHEMA, identity=identity, allocation=allocation,
        iteration=iteration, monitored_cpus=allocation['monitored_smt_siblings'],
        clock=dict(id='CLOCK_MONOTONIC', origin_ns=time.monotonic_ns() if origin_ns is None else origin_ns,
                   resolution_ns=max(1, math.ceil(time.get_clock_info('monotonic').resolution * 1e9))),
        clk_tck=hz, getconf_clk_tck=int(subprocess.check_output(['getconf', 'CLK_TCK'])),
        boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
        kernel_release=os.uname().release, host_source='/proc/stat',
        cgroup_mount='/sys/fs/cgroup', cgroup_namespace_inode=Path('/proc/self/ns/cgroup').stat().st_ino)


class Journal:
    """Append-only, flushed, bounded evidence, retained even on failure."""
    def __init__(self, path, header):
        self.path, self.header = path, header
        self.stream = path.open('x')
        self.records = []
        self.append(header)

    def append(self, record):
        data = json.dumps(record, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n'
        if self.stream.tell() + len(data.encode()) > 16 * 1024**2:
            raise ValueError('raw CPU evidence limit exceeded')
        self.stream.write(data)
        self.stream.flush()
        self.records.append(record)
        return record

    def event(self, name, **values):
        return self.append(dict(type='event', name=name, at_ns=time.monotonic_ns(), **values))

    def snapshot(self, sequence, cgroup=None, state='active'):
        begin = time.monotonic_ns()
        text = Path('/proc/stat').read_text()
        end = time.monotonic_ns()
        values = parse_host(text, self.header['monitored_cpus'])
        # Retain exact monitored lines, not unrelated /proc/stat sections or
        # unmonitored CPU lines. Validate duplicates before selecting lines.
        selected = {f'cpu{c}' for c in self.header['monitored_cpus']}
        raw = ''.join(line+'\n' for line in text.splitlines() if line.split()[0] in selected)
        host = dict(begin_ns=begin, end_ns=end, raw=raw, cpus=values)
        cg = None
        if cgroup is not None:
            begin = time.monotonic_ns()
            stat_before = cgroup.stat()
            raw = (cgroup / 'cpu.stat').read_text()
            procs = sorted(map(int, (cgroup / 'cgroup.procs').read_text().split()))
            stat_after = cgroup.stat()
            end = time.monotonic_ns()
            if stat_before.st_ino != stat_after.st_ino:
                raise ValueError('cgroup replaced during read')
            cg = dict(begin_ns=begin, end_ns=end, path=str(cgroup),
                inode=stat_after.st_ino, device=stat_after.st_dev, raw=raw,
                counters=parse_cgroup(raw), procs=procs)
            cg['limits'] = {name: (cgroup / name).read_text().strip() for name in (
                'cpu.max', 'cpuset.cpus.effective', 'memory.max', 'memory.swap.max', 'pids.max')}
            cg['end_ns'] = time.monotonic_ns()
        return self.append(dict(type='snapshot', sequence=sequence, state=state, host=host, cgroup=cg))

    def close(self):
        self.stream.close()

    def reference(self):
        return reference(self.path.name, self.path.read_bytes())


def reference(name, data):
    return dict(schema=SCHEMA, file=name, sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))


def resolve(reference_record, loader, expected_name):
    data = loader(expected_name)
    if reference_record != reference(expected_name, data):
        raise ValueError('missing/corrupt raw CPU journal reference')
    return strict_json(data)


def strict_json(data):
    def nonfinite(value):
        raise ValueError('nonfinite JSON number: ' + value)

    def finite_float(value):
        parsed = float(value)
        if not math.isfinite(parsed):
            nonfinite(value)
        return parsed

    def pairs(items):
        result = {}
        for k, v in items:
            if k in result:
                raise ValueError('duplicate JSON key')
            result[k] = v
        return result
    if not data.endswith(b'\n'):
        raise ValueError('truncated raw CPU journal')
    return [json.loads(line, object_pairs_hook=pairs,
            parse_constant=nonfinite, parse_float=finite_float)
            for line in data.splitlines()]


def endpoint(snapshot, header):
    host, cg = snapshot['host'], snapshot['cgroup']
    if snapshot['type'] != 'snapshot' or snapshot['state'] not in ('not_started', 'active', 'shutdown', 'preflight'):
        raise ValueError('invalid snapshot state')
    integer(snapshot['sequence'])
    begin, end = integer(host['begin_ns']), integer(host['end_ns'])
    if not header['clock']['origin_ns'] <= begin <= end:
        raise ValueError('invalid host read bracket')
    if set(host['cpus']) != {str(c) for c in header['monitored_cpus']} or host['cpus'] != parse_host(host['raw'], header['monitored_cpus']):
        raise ValueError('unexpected/contradictory monitored CPUs')
    if {int(line.split()[0][3:]) for line in host['raw'].splitlines()} != set(header['monitored_cpus']):
        raise ValueError('unexpected raw monitored CPU')
    if cg is not None:
        a, b = integer(cg['begin_ns']), integer(cg['end_ns'])
        if not end <= a <= b or not re.fullmatch(r'/sys/fs/cgroup/(?:[A-Za-z0-9_.:-]+/)*docker-[0-9a-f]{64}\.scope|/sys/fs/cgroup/(?:[A-Za-z0-9_.:-]+/)*[0-9a-f]{64}', cg['path']):
            raise ValueError('invalid cgroup source/bracket')
        integer(cg['inode'], 1)
        integer(cg['device'])
        if cg['counters'] != parse_cgroup(cg['raw']) or cg['procs'] != sorted(set(cg['procs'])):
            raise ValueError('contradictory cgroup CPU evidence')
        for pid in cg['procs']:
            integer(pid, 1)
        limits(cg['limits'], header['allocation']['cpus'])
        end = b
    # A whole acquisition must fit within one USER_HZ unit. This is a read-skew
    # integrity requirement, not an allowance added to the 5% acceptance limit.
    if (end - begin) * header['clk_tck'] > 1_000_000_000:
        raise ValueError('excessive host/cgroup timestamp skew')
    return begin, end


def limits(values, cpus):
    quota, period = map(int, values['cpu.max'].split())
    selected = set()
    for part in values['cpuset.cpus.effective'].split(','):
        bounds = list(map(int, part.split('-')))
        selected.update(range(bounds[0], bounds[-1]+1))
    if (period <= 0 or quota != 2*period or selected != set(cpus)
            or int(values['memory.max']) != 6*1024**3 or values['memory.swap.max'] != '0'
            or values['pids.max'] != '256'):
        raise ValueError('raw snapshot CPU/resource limits mismatch')


def calculate(before, after, header, host_only=False):
    hz, cpus = header['clk_tck'], header['monitored_cpus']
    a, b = endpoint(before, header), endpoint(after, header)
    if after['sequence'] != before['sequence'] + 1 or a[1] >= b[0]:
        raise ValueError('missing/reordered sample or overlapping boundaries')
    low, high = (b[0] - a[1]) / 1e9, (b[1] - a[0]) / 1e9
    elapsed = (after['host']['begin_ns'] - before['host']['begin_ns']) / 1e9
    if not 0 < elapsed <= (11 if header['iteration'] == -1 else 3):
        raise ValueError('missing or gapped CPU samples')
    deltas = {}
    for c in cpus:
        values = [y - x for x, y in zip(before['host']['cpus'][str(c)], after['host']['cpus'][str(c)])]
        if any(v < 0 for v in values):
            # Includes iowait: Linux permits anomalies, not reliable repair.
            raise ValueError('per-CPU counter reversal; accounting unqualified')
        total = sum(values[:8])  # guest fields already included in user/nice.
        if total <= 0 or not low * hz - 8 <= total <= high * hz + 8:
            raise ValueError('per-CPU stall or impossible counter/wall accumulation')
        idle, steal = values[3] + values[4], values[7]
        deltas[str(c)] = dict(fields=values, total_ticks=total, idle_ticks=idle,
                             busy_ticks=total-idle, steal_ticks=steal)
    start, finish = before['cgroup'], after['cgroup']
    usage = throttle = 0
    if not host_only:
        if finish is None or (start is None and before['state'] != 'not_started'):
            raise ValueError('missing cgroup boundary')
        if start and (start['path'], start['inode'], start['device']) != (finish['path'], finish['inode'], finish['device']):
            raise ValueError('cgroup identity changed')
        initial = start['counters'] if start else {k: 0 for k in finish['counters']}
        if set(initial) != set(finish['counters']):
            raise ValueError('cgroup counter fields changed')
        change = {k: finish['counters'][k] - initial[k] for k in initial}
        if any(v < 0 for v in change.values()):
            raise ValueError('regressing cgroup counter')
        usage, throttle = change['usage_usec'], change['throttled_usec']
        # Three separately truncated microsecond fields: usage ~= user+system.
        if abs(change['user_usec'] + change['system_usec'] - usage) > 3:
            raise ValueError('inconsistent cgroup user/system accounting')
        if change['nr_throttled'] > change['nr_periods']:
            raise ValueError('inconsistent cgroup throttling accounting')
        if usage > header['allocation']['workers'] * high * 1e6 + 2:
            raise ValueError('impossible cgroup CPU accumulation')
    total = sum(d['total_ticks'] for d in deltas.values())
    busy = sum(d['busy_ticks'] for d in deltas.values())
    steal = sum(d['steal_ticks'] for d in deltas.values())
    residual = float(Fraction(busy, hz) - Fraction(usage, 1_000_000))
    # Six non-idle fields per CPU, each delta of two truncated counters has
    # absolute error <1 tick. Acquisition-window uncertainty adds capacity*time.
    skew = ((a[1]-a[0]) + (b[1]-b[0])) / 1e9
    negative_bound = 6 * len(cpus) / hz + len(cpus) * skew + 2e-6
    if residual < -negative_bound:
        raise ValueError('physically inconsistent negative CPU residual')
    fraction = float(max(0, Fraction(busy, hz)-Fraction(usage, 1_000_000)) * hz * len(cpus) / total)
    return dict(schema=SCHEMA, iteration=header['iteration'], sequence=before['sequence'], raw_before=before['sequence'],
        raw_after=after['sequence'], elapsed_seconds=elapsed,
        at_seconds=(after['host']['begin_ns']-header['coverage_start_ns'])/1e9,
        per_cpu=deltas, total_ticks=total, busy_ticks=busy, steal_ticks=steal,
        workload_delta_usec=usage, throttled_delta_usec=throttle,
        usage_usec=finish['counters']['usage_usec'] if finish else (start['counters']['usage_usec'] if start else 0),
        throttled_usec=finish['counters']['throttled_usec'] if finish else (start['counters']['throttled_usec'] if start else 0),
        signed_residual_seconds=residual, negative_resolution_bound_seconds=negative_bound,
        boundary_elapsed_min_seconds=low, boundary_elapsed_max_seconds=high,
        host_only=host_only, external_busy_fraction=fraction,
        steal_fraction=steal / total)


def replay(records, identity, allocation, iteration, published):
    """Producer-side raw replay, also used by the local final acceptance gate."""
    header = records[0]
    if (header['schema'] != SCHEMA or header['type'] != 'context' or header['identity'] != identity
            or header['allocation'] != allocation or header['iteration'] != iteration
            or header['monitored_cpus'] != allocation['monitored_smt_siblings']
            or header['clk_tck'] != header['getconf_clk_tck']
            or not 1 <= integer(header['clk_tck'], 1) <= 1_000_000
            or header['clock']['id'] != 'CLOCK_MONOTONIC'
            or not 1 <= integer(header['clock']['resolution_ns'], 1) <= 1_000_000_000 / header['clk_tck']
            or not re.fullmatch(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}', header['boot_id'])
            or not header['kernel_release'] or header['host_source'] != '/proc/stat'
            or header['cgroup_mount'] != '/sys/fs/cgroup'):
        raise ValueError('invalid raw CPU source/clock/schema identity')
    integer(header['cgroup_namespace_inode'], 1)
    # Producer topology validation is separate from the consumer implementation.
    selected, monitored = allocation['cpus'], allocation['monitored_smt_siblings']
    if (len(selected) != len(set(selected)) or len(selected) != 2
            or monitored != sorted(set(monitored)) or allocation['workers'] != 2
            or set(allocation['topology']) != {str(c) for c in monitored}):
        raise ValueError('invalid CPU topology identity')
    union, cores = set(), set()
    for c in selected:
        item = allocation['topology'][str(c)]
        if c not in item['siblings'] or item['core'] < 0 or item['package'] < 0:
            raise ValueError('invalid selected CPU topology')
        for sibling in item['siblings']:
            if allocation['topology'].get(str(sibling)) != item:
                raise ValueError('nonreciprocal SMT topology')
        union.update(item['siblings'])
        cores.add((item['package'], item['core']))
    if union != set(monitored) or allocation['guest_core_count'] != len(cores) or allocation['physical_isolation_proven'] is not False:
        raise ValueError('incorrect SMT coverage')
    events = [r for r in records[1:] if r['type'] == 'event']
    snapshots = [r for r in records[1:] if r['type'] == 'snapshot']
    if len(events) + len(snapshots) != len(records)-1 or len(snapshots) != len(published)+1:
        raise ValueError('missing/unexpected raw CPU records')
    if [s['sequence'] for s in snapshots] != list(range(len(snapshots))):
        raise ValueError('missing/repeated raw CPU sequence')
    names = ('coverage_start', 'coverage_end') if iteration == -1 else EVENTS
    if tuple(e['name'] for e in events) != names:
        raise ValueError('missing/reordered lifecycle coverage')
    times = [integer(e['at_ns']) for e in events]
    if times != sorted(set(times)):
        raise ValueError('regressing/repeated lifecycle timestamp')
    last_record = header['clock']['origin_ns']
    for record in records[1:]:
        begin = record['at_ns'] if record['type'] == 'event' else record['host']['begin_ns']
        end = begin if record['type'] == 'event' else (
            record['cgroup']['end_ns'] if record['cgroup'] else record['host']['end_ns'])
        if not last_record <= begin <= end:
            raise ValueError('raw CPU record chronology')
        last_record = end
    header = dict(header, coverage_start_ns=snapshots[0]['host']['begin_ns'])
    if not times[0] <= header['coverage_start_ns'] < snapshots[-1]['host']['end_ns'] <= times[-1]:
        raise ValueError('invalid start/end boundary coverage')
    if iteration != -1:
        ev = {r['name']: r for r in events}
        final = ev['worker_exited']['snapshot']
        if (snapshots[0]['cgroup'] is not None or snapshots[0]['state'] != 'not_started'
                or not snapshots[0]['host']['end_ns'] <= ev['create_requested']['at_ns']
                or not re.fullmatch('[0-9a-f]{64}', ev['created']['container_id'])
                or not snapshots[final]['host']['end_ns'] <= ev['worker_exited']['at_ns']
                or not ev['accounted']['at_ns'] <= ev['container_exited']['at_ns'] <= snapshots[-1]['host']['begin_ns']
                or not 0 < final < len(snapshots)-1):
            raise ValueError('missing startup/final worker interval')
        discovered = ev['cgroup_discovered']
        expected = (discovered['path'], discovered['inode'], discovered['device'])
        if ev['created']['container_id'] not in discovered['path']:
            raise ValueError('container/cgroup source mismatch')
        for s in snapshots[1:final+1]:
            cg = s['cgroup']
            if s['state'] != 'active' or not cg or (cg['path'], cg['inode'], cg['device']) != expected:
                raise ValueError('missing/replaced cgroup snapshot')
        if snapshots[final]['cgroup']['procs'] != [discovered['pid']]:
            raise ValueError('worker has not quiesced before acknowledgement')
        if not snapshots[0]['host']['begin_ns'] < discovered['at_ns'] <= snapshots[1]['host']['begin_ns']:
            raise ValueError('unobserved startup/cgroup discovery')
        if any(s['state'] != 'shutdown' or s['cgroup'] is not None for s in snapshots[final+1:]):
            raise ValueError('invalid shutdown snapshots')
    else:
        final = 0
        if len(snapshots) != 2 or any(s['cgroup'] is not None or s['state'] != 'preflight' for s in snapshots):
            raise ValueError('invalid preflight raw coverage')
    for i, expected in enumerate(published):
        actual = calculate(snapshots[i], snapshots[i+1], header, host_only=iteration == -1 or i >= final)
        if actual != expected:
            raise ValueError('published CPU measurement contradicts raw records')
    return True