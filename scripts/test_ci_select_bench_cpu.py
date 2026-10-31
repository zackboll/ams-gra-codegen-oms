"""CPU-v2 independent consumer mutations, including authenticated bundle replay."""
import copy
import json
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch

from ci_bench import cpu, cpu_audit, fixtures
from test_ci_select_bench import rows
import test_ci_select_bench_reconcile as bundle_tests


def synchronize(records):
    for record in records:
        if record['type'] != 'snapshot':
            continue
        host = record['host']
        host['raw'] = ''.join('cpu'+key+' '+' '.join(map(str, value))+'\n' for key,value in host['cpus'].items())
        if record['cgroup']:
            record['cgroup']['raw'] = ''.join(f'{k} {v}\n' for k,v in record['cgroup']['counters'].items())


class IndependentCPU(unittest.TestCase):
    def test_trusted_acquisition_journal_round_trip(self):
        r,s,a = self.fixture()
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'cpu.jsonl'
            journal=cpu.Journal(path,r[0])
            for record in r[1:]:
                journal.append(record)
            journal.close()
            self.assertEqual(cpu.strict_json(path.read_bytes()),r)
            self.assertEqual(cpu_audit.decode(path.read_bytes()),r)
            self.assertEqual(cpu.resolve(journal.reference(),lambda _:path.read_bytes(),path.name),r)

    def fixture(self):
        allocation = rows()[0]['allocation']
        records, samples = fixtures.evidence({}, allocation, 0)
        return records, samples, allocation

    def reject(self, change):
        records, samples, allocation = self.fixture()
        change(records, samples)
        synchronize(records)
        with self.assertRaises((ValueError, KeyError, IndexError, TypeError)):
            cpu_audit.verify(records, {}, allocation, 0, samples)
        with self.assertRaises((ValueError, KeyError, IndexError, TypeError)):
            cpu.replay(records, {}, allocation, 0, samples)

    def test_clean_positive_and_exact_independent_equations(self):
        r, s, a = self.fixture()
        self.assertTrue(cpu_audit.verify(r, {}, a, 0, s))
        self.assertTrue(cpu.replay(r, {}, a, 0, s))

    def test_single_cpu_regression_masked_by_other_cpu(self):
        def change(r, s):
            r[2]['host']['cpus']['0'][3] = 10
            r[7]['host']['cpus']['1'][3] += 10
        self.reject(change)

    def test_stall_compensated_by_another_cpu(self):
        def change(r, s):
            r[7]['host']['cpus']['0'] = [0]*10
            r[7]['host']['cpus']['1'][3] += 100
        self.reject(change)

    def test_impossible_excess_ticks(self):
        self.reject(lambda r,s: r[7]['host']['cpus']['0'].__setitem__(0, 1000))

    def test_negative_residual_outside_resolution(self):
        def change(r,s):
            for values in r[7]['host']['cpus'].values():
                values[0], values[3] = 0, 100
        self.reject(change)

    def test_host_cgroup_endpoint_skew(self):
        self.reject(lambda r,s: r[7]['cgroup'].update(end_ns=r[7]['host']['begin_ns']+20_000_000))

    def test_missing_startup_interval(self):
        self.reject(lambda r,s: r.pop(2))

    def test_missing_final_interval(self):
        self.reject(lambda r,s: r.pop(-2))

    def test_duplicate_sample_sequence(self):
        self.reject(lambda r,s: r[-2].update(sequence=1))

    def test_reordered_samples(self):
        self.reject(lambda r,s: s.reverse())

    def test_incorrect_clk_tck(self):
        self.reject(lambda r,s: r[0].update(clk_tck=1000))

    def test_forged_matching_clk_tck_fails_wall_accumulation(self):
        self.reject(lambda r,s: r[0].update(clk_tck=1000, getconf_clk_tck=1000))

    def test_missing_smt_counter(self):
        self.reject(lambda r,s: r[7]['host']['cpus'].pop('1'))

    def test_unexpected_monitored_cpu(self):
        self.reject(lambda r,s: r[7]['host']['cpus'].update({'99':[0]*10}))

    def test_nonfinite_published_delta(self):
        self.reject(lambda r,s: s[0].update(workload_delta_usec=float('nan')))

    def test_duplicate_cpu_raw_line(self):
        r,s,a = self.fixture()
        r[7]['host']['raw'] += r[7]['host']['raw'].splitlines()[0]+'\n'
        with self.assertRaises(ValueError):
            cpu_audit.verify(r, {}, a, 0, s)

    def test_truncated_cpu_counter(self):
        self.reject(lambda r,s: r[7]['host']['cpus']['0'].pop())

    def test_invalid_cgroup_identity(self):
        self.reject(lambda r,s: r[7]['cgroup'].update(inode=0))

    def test_changed_cpuset(self):
        self.reject(lambda r,s: r[7]['cgroup']['limits'].update({'cpuset.cpus.effective':'0'}))

    def test_missing_resource_evidence(self):
        self.reject(lambda r,s: r[7]['cgroup'].pop('limits'))

    def test_raw_identity_cannot_substitute_for_provenance(self):
        self.reject(lambda r,s: r[0]['identity'].update(run_attempt='99'))

    def test_missing_cgroup_counter(self):
        self.reject(lambda r,s: r[7]['cgroup']['counters'].pop('nr_periods'))

    def test_regressing_cgroup_counter(self):
        self.reject(lambda r,s: r[7]['cgroup']['counters'].update(user_usec=-1))

    def test_inconsistent_cgroup_split(self):
        self.reject(lambda r,s: r[7]['cgroup']['counters'].update(system_usec=100))

    def test_external_busy_is_rejected_independently(self):
        def change(r,s):
            r[7]['cgroup']['counters'].update(usage_usec=1900000, user_usec=1900000)
        self.reject(change)

    def test_steal_above_existing_threshold(self):
        def change(r,s):
            r[7]['host']['cpus']['0'][0] -= 3
            r[7]['host']['cpus']['0'][7] += 3
        self.reject(change)

    def test_throttling_above_existing_threshold(self):
        def change(r,s):
            r[7]['cgroup']['counters'].update(throttled_usec=10001, nr_throttled=1)
        self.reject(change)

    def test_clock_source_mismatch(self):
        self.reject(lambda r,s: r[0]['clock'].update(id='CLOCK_REALTIME'))

    def test_worker_not_quiescent(self):
        self.reject(lambda r,s: r[7]['cgroup'].update(procs=[123,124]))

    def test_duplicate_json_keys_and_truncation(self):
        for data in (b'{"type":1,"type":2}\n', b'{}'):
            with self.subTest(data=data), self.assertRaises(ValueError):
                cpu_audit.decode(data)

    def test_independent_consumer_does_not_use_producer_calculation(self):
        r,s,a = self.fixture()
        with patch.object(cpu, 'calculate', side_effect=AssertionError('producer called')):
            self.assertTrue(cpu_audit.verify(r, {}, a, 0, s))

    def test_producer_also_rejects_adversarial_counter_pair(self):
        r,s,a = self.fixture()
        r[7]['host']['cpus']['0'] = [0]*10
        r[7]['host']['cpus']['1'][3] += 100
        synchronize(r)
        with self.assertRaises(ValueError):
            cpu.replay(r, {}, a, 0, s)


class FullArtifactCPU(unittest.TestCase):
    def test_rehashed_counter_corruption_reaches_independent_arithmetic(self):
        helper = bundle_tests.Reconciliation()
        fixture, bundles = helper.fixture()
        bundle = bundles['ci-bench-001-gnat-14-measurements']
        raw = [json.loads(line) for line in bundle['cpu-0.jsonl'].splitlines()]
        raw[7]['host']['cpus']['0'][0] += 1000
        synchronize(raw)
        data = b''.join(json.dumps(r,sort_keys=True).encode()+b'\n' for r in raw)
        bundle['cpu-0.jsonl'] = data
        ref = cpu.reference('cpu-0.jsonl', data)
        helper.edit(bundles,'14','result.json',lambda result: result['runs'][0].update(cpu_raw=ref))
        with patch('ci_bench.controller.validate_result', return_value=None), self.assertRaisesRegex(ValueError, 'per-CPU'):
            helper.verify((fixture,bundles))

    def test_missing_inter_iteration_boundary_rejected(self):
        helper = bundle_tests.Reconciliation()
        fixture,bundles = helper.fixture()
        bundle=bundles['ci-bench-001-gnat-14-measurements']
        records,samples,allocation=IndependentCPU().fixture()
        # Raw fixture from an unrelated timeline but otherwise individually valid.
        result=json.loads(bundle['result.json'])
        identity=result['identity']
        records,samples=fixtures.evidence(identity,allocation,1)
        data=b''.join(json.dumps(r,sort_keys=True).encode()+b'\n' for r in records)
        bundle['cpu-1.jsonl']=data
        helper.edit(bundles,'14','result.json',lambda r:r['runs'][1].update(
            cpu_raw=cpu.reference('cpu-1.jsonl',data),samples=samples))
        with self.assertRaisesRegex(ValueError,'inter-iteration'):
            helper.verify((fixture,bundles))

    def test_missing_raw_artifact_rejected(self):
        helper = bundle_tests.Reconciliation()
        fixture, bundles = helper.fixture()
        bundles['ci-bench-001-gnat-14-measurements'].pop('cpu-0.jsonl')
        with self.assertRaises((ValueError, KeyError)):
            helper.verify((fixture, bundles))

    def test_altered_raw_with_recomputed_packaging_still_rejected(self):
        helper = bundle_tests.Reconciliation()
        fixture, bundles = helper.fixture()
        def change(result):
            result['runs'][0]['cpu_raw']['sha256'] = '0'*64
        helper.edit(bundles, '14', 'result.json', change)
        with self.assertRaises(ValueError):
            helper.verify((fixture,bundles))

    def test_old_derived_only_schema_never_newly_qualified(self):
        helper = bundle_tests.Reconciliation()
        fixture,bundles=helper.fixture()
        helper.edit(bundles, '14', 'result.json', lambda result: result.pop('preflight_raw'))
        with self.assertRaises(ValueError):
            helper.verify((fixture,bundles))

    def test_independent_full_path_rejects_even_with_producer_gate_disabled(self):
        helper=bundle_tests.Reconciliation()
        fixture,bundles=helper.fixture()
        helper.edit(bundles,'14','result.json', lambda result: result['runs'][0]['samples'][0].update(external_busy_fraction=.04))
        with patch('ci_bench.controller.validate_result', return_value=None), self.assertRaises(ValueError):
            helper.verify((fixture,bundles))


if __name__ == '__main__':
    unittest.main()