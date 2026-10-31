"""Deterministic synthetic CPU-v2 fixtures. Never hosted qualification evidence."""
import copy
from . import cpu


def evidence(identity, allocation, iteration):
    cpus = allocation['monitored_smt_siblings']
    origin = 1_000_000_000 if iteration == -1 else (20+iteration*10)*1_000_000_000
    h = dict(type='context', schema=cpu.SCHEMA, identity=identity, allocation=allocation,
        iteration=iteration, monitored_cpus=cpus, clk_tck=100, getconf_clk_tck=100,
        clock=dict(id='CLOCK_MONOTONIC', origin_ns=origin, resolution_ns=1),
        boot_id='12345678-1234-1234-1234-123456789abc', kernel_release='fixture',
        host_source='/proc/stat', cgroup_mount='/sys/fs/cgroup', cgroup_namespace_inode=1)
    group_path = '/sys/fs/cgroup/system.slice/docker-' + 'd'*64 + '.scope'
    def snapshot(seq, at, active=False, state='active'):
        ticks = int(at*100)
        values = {}
        for c in cpus:
            busy = ticks if c in allocation['cpus'] and active else 0
            values[str(c)] = [busy, 0, 0, ticks-busy, 0, 0, 0, 0, 0, 0]
        raw = '\n'.join('cpu'+str(c)+' '+' '.join(map(str, values[str(c)])) for c in cpus)+'\n'
        begin = origin+int(at*1e9)
        group = None
        if active:
            counters = dict(usage_usec=int(at*2e6), user_usec=int(at*2e6), system_usec=0,
                            nr_periods=int(at*10), nr_throttled=0, throttled_usec=0)
            group = dict(begin_ns=begin+2000, end_ns=begin+3000, path=group_path,
                         inode=42, device=1, raw=''.join(f'{k} {v}\n' for k,v in counters.items()),
                         counters=counters, procs=[123])
            group['limits'] = {'cpu.max': '200000 100000',
                'cpuset.cpus.effective': ','.join(map(str,allocation['cpus'])),
                'memory.max': str(6*1024**3), 'memory.swap.max': '0', 'pids.max': '256'}
        return dict(type='snapshot', sequence=seq, state=state,
                    host=dict(begin_ns=begin, end_ns=begin+1000, raw=raw, cpus=values), cgroup=group)
    def event(name, at, **kwargs):
        return dict(type='event', name=name, at_ns=origin+int(at*1e9), **kwargs)
    if iteration == -1:
        start, end = snapshot(0, .001, state='preflight'), snapshot(1, 10.001, state='preflight')
        records = [h, event('coverage_start', 0), start, end, event('coverage_end', 10.002)]
        header = dict(h, coverage_start_ns=start['host']['begin_ns'])
        return records, [cpu.calculate(start, end, header, host_only=True)]
    # Baseline before launch is exactly zero. Shutdown retains host busy totals
    # from worker exit, but gives all additional ticks to idle.
    start = snapshot(0, 0, state='not_started')
    active = snapshot(1, 1, active=True)
    end = copy.deepcopy(active)
    end.update(sequence=2, state='shutdown', cgroup=None)
    end['host']['begin_ns'] += 100_000_000
    end['host']['end_ns'] += 100_000_000
    for values in end['host']['cpus'].values():
        values[3] += 10
    end['host']['raw'] = '\n'.join('cpu'+str(c)+' '+' '.join(map(str,end['host']['cpus'][str(c)])) for c in cpus)+'\n'
    records = [h, event('coverage_start', 0), start, event('create_requested', .001),
        event('created', .002, container_id='d'*64), event('start_requested', .003),
        event('cgroup_discovered', .004, path=group_path, inode=42, device=1, pid=123),
        active, event('worker_exited', 1.001, snapshot=1), event('accounted', 1.002),
        event('container_exited', 1.05), end, event('coverage_end', 1.101)]
    header = dict(h, coverage_start_ns=start['host']['begin_ns'])
    return records, [cpu.calculate(start, active, header), cpu.calculate(active, end, header, host_only=True)]


def bridge(records, samples, previous):
    """Join synthetic iterations at an identical absolute host endpoint."""
    first = next(r for r in records if r['type'] == 'snapshot')
    shift = previous['host']['begin_ns'] - first['host']['begin_ns']
    old = copy.deepcopy(first['host']['cpus'])
    for record in records:
        if record['type'] == 'context':
            record['clock']['origin_ns'] += shift
        elif record['type'] == 'event':
            record['at_ns'] += shift
        else:
            for source in (record['host'], record['cgroup']):
                if source:
                    source['begin_ns'] += shift
                    source['end_ns'] += shift
            for key, values in record['host']['cpus'].items():
                record['host']['cpus'][key] = [v+base-old[key][i] for i,(v,base) in
                    enumerate(zip(values,previous['host']['cpus'][key]))]
            record['host']['raw'] = ''.join('cpu'+key+' '+' '.join(map(str,value))+'\n'
                                            for key,value in record['host']['cpus'].items())
    header = dict(records[0], coverage_start_ns=first['host']['begin_ns'])
    points = [r for r in records if r['type'] == 'snapshot']
    samples[:] = [cpu.calculate(a,b,header, host_only=i>0) for i,(a,b) in enumerate(zip(points,points[1:]))]