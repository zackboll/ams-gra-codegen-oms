"""Performance arithmetic controls; synthetic durations are not hosted results."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('performance', Path(__file__).with_name('real-uci-performance.py'))
performance = importlib.util.module_from_spec(spec)
spec.loader.exec_module(performance)


class Performance(unittest.TestCase):
    def test_execution_queue_and_runner_minutes_are_distinct(self):
        run = dict(id=1, head_sha='a'*40, status='completed', conclusion='success',
                   run_started_at='2026-10-27T00:00:00Z', updated_at='2026-10-27T00:20:00Z')
        job = dict(id=2, name='real-uci', status='completed', conclusion='success',
                   started_at='2026-10-27T00:00:00Z', completed_at='2026-10-27T00:20:00Z',
                   steps=[dict(started_at='2026-10-27T00:10:00Z', conclusion='success')])
        report = performance.workflow(run, [job], {'real-uci'})
        self.assertEqual(report['primary_execution_critical_path_seconds'], 600)
        self.assertEqual(report['primary_runner_minutes'], 10)
        self.assertEqual(report['workflow_elapsed_seconds'], 1200)
        self.assertEqual(report['jobs'][0]['eligibility_to_setup_seconds'], 600)
        faster = copy.deepcopy(report)
        faster['primary_execution_critical_path_seconds'] = 300
        faster['workflow_elapsed_seconds'] = 1500
        compared = performance.compare(report, faster)
        self.assertEqual(compared['primary_execution_reduction_percent'], 50)
        self.assertEqual(compared['workflow_elapsed_improvement_percent'], -25)
        for conclusion in ('failure', 'cancelled', 'skipped'):
            job['conclusion'] = conclusion
            with self.assertRaises(ValueError):
                performance.workflow(run, [job], {'real-uci'})


if __name__ == '__main__':
    unittest.main()