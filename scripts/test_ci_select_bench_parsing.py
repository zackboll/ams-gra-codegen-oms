"""CI-BENCH-004R1 parser regressions; synthetic evidence, never a benchmark."""
import json
import unittest
from unittest.mock import patch

from ci_bench import cpu, cpu_audit
import test_ci_select_bench_cpu as cpu_tests
import test_ci_select_bench_reconcile as bundle_tests


def journal(records):
    return b''.join(json.dumps(r, sort_keys=True, allow_nan=False).encode() + b'\n'
                    for r in records)


def dotted_counters(records):
    for record in records:
        if record['type'] == 'snapshot' and record['cgroup']:
            record['cgroup']['counters']['core_sched.force_idle_usec'] = 1234567890123456789
    cpu_tests.synchronize(records)


class CounterParsing(unittest.TestCase):
    def raw(self):
        records, _, _ = cpu_tests.IndependentCPU().fixture()
        return next(r['cgroup']['raw'] for r in records
                    if r['type'] == 'snapshot' and r['cgroup'])

    def test_legitimate_optional_dotted_fields_preserve_exact_values(self):
        raw = self.raw() + 'core_sched.force_idle_usec 1234567890123456789\n'
        for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
            with self.subTest(parser=parse.__module__):
                result = parse(raw)
                self.assertEqual(result['core_sched.force_idle_usec'], 1234567890123456789)
                self.assertEqual(type(result['core_sched.force_idle_usec']), int)
                self.assertEqual(set(result), cpu.REQUIRED | {'core_sched.force_idle_usec'})
        self.assertEqual(cpu.parse_cgroup(raw), cpu_audit.cgparse(raw))

    def test_optional_counters_are_not_required(self):
        for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
            self.assertEqual(set(parse(self.raw())), cpu.REQUIRED)

    def test_complete_dotted_raw_evidence_replays_in_both_implementations(self):
        records, samples, allocation = cpu_tests.IndependentCPU().fixture()
        dotted_counters(records)
        data = journal(records)
        self.assertTrue(cpu.replay(cpu.strict_json(data), {}, allocation, 0, samples))
        with patch.object(cpu, 'parse_cgroup', side_effect=AssertionError('producer parser called')):
            self.assertTrue(cpu_audit.verify(cpu_audit.decode(data), {}, allocation, 0, samples))

    def test_duplicate_dotted_keys_rejected(self):
        raw = self.raw() + 'core_sched.force_idle_usec 1\ncore_sched.force_idle_usec 2\n'
        for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
            with self.subTest(parser=parse.__module__), self.assertRaises(ValueError):
                parse(raw)

    def test_malformed_counter_names_rejected(self):
        for name in ('.force_idle_usec', 'core_sched.', 'core_sched..force_idle_usec',
                     'core_sched/force_idle_usec', 'core-sched.force_idle_usec',
                     'core_sched.force_idle_usec!', 'Core_sched.force_idle_usec',
                     'core_sched.force idle_usec', 'core_sched.\u00e9'):
            for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
                with self.subTest(name=name, parser=parse.__module__), self.assertRaises(ValueError):
                    parse(self.raw() + name + ' 1\n')

    def test_counter_values_must_be_unsigned_ascii_integers(self):
        for value in ('-1', '+1', '1.5', 'NaN', 'Infinity', '1e3', '1_000', '\u0661'):
            for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
                with self.subTest(value=value, parser=parse.__module__), self.assertRaises(ValueError):
                    parse(self.raw() + 'optional_usec ' + value + '\n')

    def test_missing_required_fields_rejected_even_with_optional_fields(self):
        for field in cpu.REQUIRED:
            raw = ''.join(line + '\n' for line in self.raw().splitlines()
                          if not line.startswith(field + ' '))
            raw += 'core_sched.force_idle_usec 0\n'
            for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
                with self.subTest(field=field, parser=parse.__module__), self.assertRaises(ValueError):
                    parse(raw)

    def test_incomplete_counter_records_rejected(self):
        for line in ('optional_usec', 'optional_usec 1 extra', 'core_sched.force_idle_usec'):
            for parse in (cpu.parse_cgroup, cpu_audit.cgparse):
                with self.subTest(line=line, parser=parse.__module__), self.assertRaises(ValueError):
                    parse(self.raw() + line + '\n')


class JSONParsing(unittest.TestCase):
    def test_named_nonfinite_constants_rejected_directly(self):
        for token in (b'NaN', b'Infinity', b'-Infinity'):
            for decode in (cpu.strict_json, cpu_audit.decode):
                with self.subTest(token=token, decoder=decode.__module__), self.assertRaisesRegex(ValueError, 'nonfinite'):
                    decode(token + b'\n')

    def test_overflowing_json_numbers_rejected(self):
        for token in (b'1e9999', b'-1e9999', b'1.7976931348623159e308'):
            for decode in (cpu.strict_json, cpu_audit.decode):
                with self.subTest(token=token, decoder=decode.__module__), self.assertRaisesRegex(ValueError, 'nonfinite'):
                    decode(token + b'\n')

    def test_nonfinite_hidden_in_unused_nested_metadata_rejected(self):
        records, _, _ = cpu_tests.IndependentCPU().fixture()
        records[0]['review_metadata'] = {'unused': [None, {'number': 'TOKEN'}]}
        for token in (b'NaN', b'Infinity', b'-Infinity', b'1e9999', b'-1e9999'):
            data = journal(records).replace(b'"TOKEN"', token)
            for decode in (cpu.strict_json, cpu_audit.decode):
                with self.subTest(token=token, decoder=decode.__module__), self.assertRaisesRegex(ValueError, 'nonfinite'):
                    decode(data)

    def test_finite_numeric_metadata_and_literal_strings_remain_compatible(self):
        records, samples, allocation = cpu_tests.IndependentCPU().fixture()
        records[0]['review_metadata'] = {'unused': [0, -3, 1.25, -0.0, 1e308, 10**100,
                                                  'NaN', 'Infinity', None]}
        data = journal(records)
        self.assertEqual(cpu.strict_json(data), records)
        self.assertEqual(cpu_audit.decode(data), records)
        self.assertTrue(cpu.replay(cpu.strict_json(data), {}, allocation, 0, samples))
        self.assertTrue(cpu_audit.verify(cpu_audit.decode(data), {}, allocation, 0, samples))


class CompleteArtifactParsing(unittest.TestCase):
    def replace_journal(self, helper, bundles, data):
        bundle = bundles['ci-bench-001-gnat-14-measurements']
        bundle['cpu-0.jsonl'] = data
        reference = cpu.reference('cpu-0.jsonl', data)
        for name in ('result.json', 'progress.json'):
            helper.edit(bundles, '14', name, lambda value: value['runs'][0].update(cpu_raw=reference))

    def test_full_reconciliation_accepts_legitimate_optional_counter(self):
        helper = bundle_tests.Reconciliation()
        fixture, bundles = helper.fixture()
        data = bundles['ci-bench-001-gnat-14-measurements']['cpu-0.jsonl']
        records = [json.loads(line) for line in data.splitlines()]
        dotted_counters(records)
        self.replace_journal(helper, bundles, journal(records))
        self.assertEqual(helper.verify((fixture, bundles))['iterations'], 8)

    def test_independent_bundle_rejects_nonfinite_metadata_with_producer_disabled(self):
        for token in (b'NaN', b'Infinity', b'-Infinity', b'1e9999', b'-1e9999'):
            helper = bundle_tests.Reconciliation()
            fixture, bundles = helper.fixture()
            data = bundles['ci-bench-001-gnat-14-measurements']['cpu-0.jsonl']
            records = [json.loads(line) for line in data.splitlines()]
            records[0]['review_metadata'] = {'unused': ['TOKEN']}
            data = journal(records).replace(b'"TOKEN"', token)
            self.replace_journal(helper, bundles, data)
            with self.subTest(token=token), patch('ci_bench.controller.validate_result', return_value=None), \
                    self.assertRaisesRegex(ValueError, 'nonfinite'):
                helper.verify((fixture, bundles))

    def test_independent_bundle_rejects_duplicate_dotted_key_with_producer_disabled(self):
        helper = bundle_tests.Reconciliation()
        fixture, bundles = helper.fixture()
        data = bundles['ci-bench-001-gnat-14-measurements']['cpu-0.jsonl']
        records = [json.loads(line) for line in data.splitlines()]
        dotted_counters(records)
        group = next(r['cgroup'] for r in records if r['type'] == 'snapshot' and r['cgroup'])
        group['raw'] += 'core_sched.force_idle_usec 0\n'
        self.replace_journal(helper, bundles, journal(records))
        with patch('ci_bench.controller.validate_result', return_value=None), \
                self.assertRaisesRegex(ValueError, 'duplicate/corrupt cgroup counter'):
            helper.verify((fixture, bundles))


if __name__ == '__main__':
    unittest.main()