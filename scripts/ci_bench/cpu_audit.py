"""Independent CPU-v2 evidence consumer; does not import the producer sampler.

Uses integer nanoseconds and Fraction arithmetic for integrity comparisons.
Published floats are verified only after independently reconstructing inputs.
"""
from fractions import Fraction
import json
import math
import re


def require(condition, reason):
    if not condition:
        raise ValueError('CPU audit: ' + reason)


def number(value, minimum=0):
    require(type(value) is int and value >= minimum, 'non-integer/negative counter')
    return value


def decode(data):
    def pairs(values):
        result = {}
        for name, value in values:
            require(name not in result, 'duplicate raw JSON key')
            result[name] = value
        return result
    require(len(data) <= 16 * 1024**2 and data.endswith(b'\n'), 'oversized/truncated raw journal')
    try:
        return [json.loads(line, object_pairs_hook=pairs, parse_constant=lambda _: None)
                for line in data.splitlines()]
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError('CPU audit: malformed raw journal') from error


def proc(text, selected):
    found = {}
    for line in text.splitlines():
        parts = line.split()
        if not parts or not re.fullmatch(r'cpu[0-9]+', parts[0]):
            continue
        cpu_id = int(parts[0][3:])
        require(cpu_id not in found and len(parts) == 11, 'duplicate/truncated CPU counters')
        require(all(re.fullmatch(r'[0-9]+', v) for v in parts[1:]), 'invalid CPU counter')
        found[cpu_id] = list(map(int, parts[1:]))
    require(set(selected) == set(found), 'missing/unexpected SMT/selected CPU counters')
    return {str(k): found[k] for k in selected}


def cgparse(text):
    found = {}
    for line in text.splitlines():
        parts = line.split()
        require(len(parts) == 2 and re.fullmatch(r'[a-z_]+', parts[0]) and
                re.fullmatch(r'[0-9]+', parts[1]) and parts[0] not in found,
                'duplicate/corrupt cgroup counter')
        found[parts[0]] = int(parts[1])
    require({'usage_usec', 'user_usec', 'system_usec', 'nr_periods', 'nr_throttled',
             'throttled_usec'} <= set(found), 'incomplete cgroup cpu.stat')
    return found


def topology(allocation):
    cpus, monitored = allocation['cpus'], allocation['monitored_smt_siblings']
    require(all(type(c) is int and c >= 0 for c in cpus + monitored), 'invalid logical CPU ID')
    require(len(cpus) == len(set(cpus)) == allocation['workers'] == 2 and
            monitored == sorted(set(monitored)) and set(cpus) <= set(monitored), 'CPU budget/set')
    require(set(allocation['topology']) == {str(c) for c in monitored}, 'topology inventory')
    siblings, cores = set(), set()
    for c in monitored:
        item = allocation['topology'][str(c)]
        package, core, group = item['package'], item['core'], item['siblings']
        number(package)
        number(core)
        require(group == sorted(set(group)) and c in group and set(group) <= set(monitored), 'SMT topology')
        for sibling in group:
            require(allocation['topology'][str(sibling)] == item, 'nonreciprocal SMT topology')
        if c in cpus:
            siblings.update(group)
            cores.add((package, core))
    require(siblings == set(monitored) and allocation['guest_core_count'] == len(cores) and
            allocation['physical_isolation_proven'] is False, 'SMT coverage/physical isolation claim')


def bracket(snapshot, header):
    require(snapshot['type'] == 'snapshot' and snapshot['state'] in
            ('not_started', 'active', 'shutdown', 'preflight'), 'snapshot identity')
    number(snapshot['sequence'])
    host = snapshot['host']
    lo, hi = number(host['begin_ns']), number(host['end_ns'])
    require(header['clock']['origin_ns'] <= lo <= hi, 'host timestamp bracket')
    require(set(host['cpus']) == {str(c) for c in header['monitored_cpus']} and
            host['cpus'] == proc(host['raw'], header['monitored_cpus']), 'raw/parsed CPU disagreement')
    for values in host['cpus'].values():
        require(len(values) == 10, 'truncated parsed counters')
        for v in values:
            number(v)
    group = snapshot['cgroup']
    if group is not None:
        x, y = number(group['begin_ns']), number(group['end_ns'])
        require(hi <= x <= y, 'host/cgroup read order')
        require(re.fullmatch(r'/sys/fs/cgroup/(?:[A-Za-z0-9_.:-]+/)*(?:docker-[0-9a-f]{64}\.scope|[0-9a-f]{64})',
                            group['path']), 'untrusted cgroup source')
        number(group['inode'], 1)
        number(group['device'])
        require(group['counters'] == cgparse(group['raw']), 'raw/parsed cgroup disagreement')
        require(group['procs'] == sorted(set(group['procs'])), 'cgroup process inventory')
        for pid in group['procs']:
            number(pid, 1)
        limits = group['limits']
        quota, period = map(int, limits['cpu.max'].split())
        selected = set()
        for interval in limits['cpuset.cpus.effective'].split(','):
            edges = list(map(int, interval.split('-')))
            require(1 <= len(edges) <= 2 and edges[-1] >= edges[0], 'invalid cpuset range')
            selected.update(range(edges[0], edges[-1]+1))
        require(quota == 2*period and period > 0 and selected == set(header['allocation']['cpus']) and
                limits['memory.max'] == str(6*1024**3) and limits['memory.swap.max'] == '0' and
                limits['pids.max'] == '256', 'CPU/resource limits changed')
        hi = y
    require((hi-lo) * header['clk_tck'] <= 10**9, 'endpoint skew exceeds one counter unit')
    return lo, hi


def recompute(left, right, header, host_only):
    hz, ids = header['clk_tck'], header['monitored_cpus']
    l, r = bracket(left, header), bracket(right, header)
    require(right['sequence'] == left['sequence'] + 1 and l[1] < r[0], 'interval sequence/overlap')
    dt_ns = right['host']['begin_ns'] - left['host']['begin_ns']
    require(0 < dt_ns <= (11 if header['iteration'] == -1 else 3) * 10**9, 'sampling gap')
    low, high = Fraction(r[0]-l[1], 10**9), Fraction(r[1]-l[0], 10**9)
    per_cpu = {}
    for identifier in ids:
        key = str(identifier)
        original, updated = left['host']['cpus'][key], right['host']['cpus'][key]
        increments = [updated[i]-original[i] for i in range(10)]
        require(min(increments) >= 0, 'per-CPU field reversal')
        total = sum(increments[0:8])
        require(total > 0 and low*hz-8 <= total <= high*hz+8, 'stalled/impossible per-CPU total')
        idle = increments[3]+increments[4]
        per_cpu[key] = dict(fields=increments, total_ticks=total, idle_ticks=idle,
                            busy_ticks=total-idle, steal_ticks=increments[7])
    initial, final = left['cgroup'], right['cgroup']
    usage = throttle = 0
    if not host_only:
        require(final is not None and (initial is not None or left['state'] == 'not_started'), 'missing cgroup endpoint')
        if initial:
            require(all(initial[k] == final[k] for k in ('path', 'inode', 'device')), 'cgroup identity changed')
        a = initial['counters'] if initial else dict.fromkeys(final['counters'], 0)
        b = final['counters']
        require(set(a) == set(b), 'changed cgroup counter set')
        delta = {k: b[k]-a[k] for k in a}
        require(min(delta.values()) >= 0, 'regressing cgroup accounting')
        usage, throttle = delta['usage_usec'], delta['throttled_usec']
        require(abs(usage-delta['user_usec']-delta['system_usec']) <= 3, 'inconsistent cgroup CPU split')
        require(delta['nr_throttled'] <= delta['nr_periods'], 'inconsistent cgroup periods')
        require(usage <= header['allocation']['workers']*high*10**6+2, 'excess workload CPU')
    total = sum(c['total_ticks'] for c in per_cpu.values())
    busy = sum(c['busy_ticks'] for c in per_cpu.values())
    steal = sum(c['steal_ticks'] for c in per_cpu.values())
    residual = Fraction(busy, hz)-Fraction(usage, 10**6)
    skew = Fraction(l[1]-l[0]+r[1]-r[0], 10**9)
    bound = Fraction(6*len(ids), hz)+len(ids)*skew+Fraction(2, 10**6)
    require(residual >= -bound, 'negative residual violates conservation')
    result = dict(schema='ci-bench-cpu-v2', iteration=header['iteration'], sequence=left['sequence'], raw_before=left['sequence'],
        raw_after=right['sequence'], elapsed_seconds=float(Fraction(dt_ns, 10**9)),
        at_seconds=float(Fraction(right['host']['begin_ns']-header['coverage_start_ns'], 10**9)),
        per_cpu=per_cpu, total_ticks=total, busy_ticks=busy, steal_ticks=steal,
        workload_delta_usec=usage, throttled_delta_usec=throttle,
        usage_usec=final['counters']['usage_usec'] if final else (initial['counters']['usage_usec'] if initial else 0),
        throttled_usec=final['counters']['throttled_usec'] if final else (initial['counters']['throttled_usec'] if initial else 0),
        signed_residual_seconds=float(residual), negative_resolution_bound_seconds=float(bound),
        boundary_elapsed_min_seconds=float(low), boundary_elapsed_max_seconds=float(high),
        host_only=host_only, external_busy_fraction=float(max(0, residual)*hz*len(ids)/total),
        steal_fraction=float(Fraction(steal, total)))
    # Acceptance constants are intentionally not imported from the producer.
    require(result['external_busy_fraction'] <= .05, 'external busy exceeds 5%')
    require(result['steal_fraction'] <= .01, 'steal exceeds 1%')
    require(Fraction(throttle, 10**6) <= Fraction(dt_ns, 10**9)*Fraction(1, 100), 'throttling exceeds 1%')
    return result


def equal(actual, published):
    if isinstance(actual, dict):
        require(isinstance(published, dict) and set(actual) == set(published), 'derived fields missing/extra')
        for key in actual:
            equal(actual[key], published[key])
    elif isinstance(actual, float):
        require(type(published) in (int, float) and math.isfinite(published) and
                abs(actual-published) <= 8*math.ulp(actual), 'derived measurement altered')
    else:
        require(type(actual) is type(published) and actual == published, 'derived counters/identity altered')


def verify(records, identity, allocation, iteration, samples):
    require(records and samples, 'missing raw records')
    h = records[0]
    topology(allocation)
    require(h['type'] == 'context' and h['schema'] == 'ci-bench-cpu-v2' and h['identity'] == identity and
            h['allocation'] == allocation and h['iteration'] == iteration and
            h['monitored_cpus'] == allocation['monitored_smt_siblings'], 'schema/producer/iteration identity')
    hz = number(h['clk_tck'], 1)
    require(hz == number(h['getconf_clk_tck'], 1) and hz <= 10**6, 'CLK_TCK identities differ')
    require(h['clock']['id'] == 'CLOCK_MONOTONIC' and 0 < number(h['clock']['resolution_ns'], 1)*hz <= 10**9,
            'clock identity/resolution')
    number(h['clock']['origin_ns'])
    require(re.fullmatch(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}', h['boot_id']) and
            h['kernel_release'] and h['host_source'] == '/proc/stat' and h['cgroup_mount'] == '/sys/fs/cgroup',
            'counter source/kernel identity')
    number(h['cgroup_namespace_inode'], 1)
    events, endpoints = [], []
    for rec in records[1:]:
        require(rec['type'] in ('event', 'snapshot'), 'unexpected raw record')
        (events if rec['type'] == 'event' else endpoints).append(rec)
    previous_record = number(h['clock']['origin_ns'])
    for rec in records[1:]:
        begin = rec['at_ns'] if rec['type'] == 'event' else rec['host']['begin_ns']
        end = begin if rec['type'] == 'event' else (rec['cgroup']['end_ns'] if rec['cgroup'] else rec['host']['end_ns'])
        require(previous_record <= number(begin) <= number(end), 'raw record chronology')
        previous_record = end
    require(len(endpoints) == len(samples)+1 and [s['sequence'] for s in endpoints] == list(range(len(endpoints))),
            'missing/duplicated/reordered samples')
    names = ('coverage_start', 'coverage_end') if iteration == -1 else (
        'coverage_start', 'create_requested', 'created', 'start_requested', 'cgroup_discovered',
        'worker_exited', 'accounted', 'container_exited', 'coverage_end')
    require(tuple(e['name'] for e in events) == names, 'missing lifecycle events')
    ts = [number(e['at_ns']) for e in events]
    require(ts == sorted(set(ts)), 'lifecycle timestamp order')
    h = dict(h, coverage_start_ns=endpoints[0]['host']['begin_ns'])
    require(ts[0] <= h['coverage_start_ns'] < endpoints[-1]['host']['end_ns'] <= ts[-1], 'boundary coverage')
    if iteration == -1:
        require(len(endpoints) == 2 and all(s['state'] == 'preflight' and s['cgroup'] is None for s in endpoints),
                'preflight raw coverage')
        final = 0
    else:
        ev = {e['name']: e for e in events}
        final = number(ev['worker_exited']['snapshot'], 1)
        require(final < len(endpoints)-1, 'missing final accounting/shutdown interval')
        require(endpoints[0]['state'] == 'not_started' and endpoints[0]['cgroup'] is None and
                endpoints[0]['host']['end_ns'] <= ev['create_requested']['at_ns'], 'missing startup coverage')
        require(re.fullmatch('[0-9a-f]{64}', ev['created']['container_id']) and
                ev['created']['container_id'] in ev['cgroup_discovered']['path'], 'container identity')
        require(endpoints[final]['host']['end_ns'] <= ev['worker_exited']['at_ns'] and
                ev['accounted']['at_ns'] <= ev['container_exited']['at_ns'] <= endpoints[-1]['host']['begin_ns'],
                'missing exit boundary coverage')
        discovered = ev['cgroup_discovered']
        require(endpoints[0]['host']['begin_ns'] < discovered['at_ns'] <= endpoints[1]['host']['begin_ns'],
                'cgroup startup discovery gap')
        for s in endpoints[1:final+1]:
            group = s['cgroup']
            require(s['state'] == 'active' and group and
                    all(group[k] == discovered[k] for k in ('path', 'inode', 'device')), 'untrusted cgroup identity')
        require(endpoints[final]['cgroup']['procs'] == [number(discovered['pid'], 1)], 'worker not quiescent')
        require(all(s['state'] == 'shutdown' and s['cgroup'] is None for s in endpoints[final+1:]),
                'contradictory shutdown state')
    for index, measurement in enumerate(samples):
        equal(recompute(endpoints[index], endpoints[index+1], h, iteration == -1 or index >= final), measurement)
    return True