"""Preserve real-Sleet inventory and stable mandatory-source build admission."""
from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]


class RustSleetLifecycle(unittest.TestCase):
    def test_executable_procfs_negative_controls(self):
        result = subprocess.run(['bash', str(ROOT / 'scripts/check-rust-task-workers.sh')],
                                capture_output=True, text=True, timeout=120)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('test result: ok. 8 passed; 0 failed; 0 ignored;', result.stdout)

    def test_all_original_completion_markers_and_tests_are_retained(self):
        path = 'scripts/run-real-sleet-test.sh'
        before = subprocess.check_output(['git', 'show',
            'ac333a98eab35c590ff61f7e45f248015499c9a4:' + path], cwd=ROOT, text=True)
        # The 2,000-join check is independent of FileMetadata's mixed-family
        # campaign, so upstream DLZ incompatibility cannot prevent its execution.
        count = "grep -qE '^test result: ok\\. 6 passed; 0 failed; 0 ignored;'"
        self.assertEqual(before.count(count), 1)
        before = before.replace(count, count.replace('6 passed', '7 passed'))
        after = (ROOT / path).read_text()
        # PR208's reviewed candidate runner preserves the original families and
        # adds source/binary identity checks and split standalone tests. Compare
        # reconciliation against that complete inventory, not the superseded pin.
        # Every original command remains in order; timing wrappers are transparent.
        after = re.sub(r'qualification_stage "[^"\n]+" ', '', after)
        after = after.replace('qualification_stage py005 ', '')
        index = 0
        for line in before.splitlines():
            if not line.strip():
                continue
            found = after.find(line, index)
            self.assertNotEqual(found, -1, 'removed/reordered original runner line: ' + line)
            index = found + len(line)
        self.assertEqual(re.findall(r'--test [a-z_]+', before),
                         re.findall(r'--test [a-z_]+', after))
        self.assertIn('bash "$ROOT/scripts/check-rust-task-workers.sh"', after)

    def test_real_join_regression_is_an_independent_required_test(self):
        source = (ROOT / 'examples/rust-uci-client/tests/real_sleet.rs').read_text()
        self.assertIn('fn standalone_runtime_joins() {\n    support::repeated_runtime_joins();\n}', source)
        self.assertEqual(source.count('support::repeated_runtime_joins();'), 1)
        support = (ROOT / 'examples/rust-uci-client/tests/support/reconnect.rs').read_text()
        self.assertIn('for _ in 0..2_000 {\n        connect(&url).close().unwrap();\n'
                      '        assert_eq!(application_workers(pid), before);\n'
                      '        assert_eq!(connections(&url), 0);', support)

    def test_stable_flags_preserve_same_complete_build_admission(self):
        runner = (ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        build = (ROOT / 'crates/runtime-rust-facade-tests/build.rs').read_text()
        admission = build.split('let required =', 1)[1].split(';', 1)[0]
        variables = set(re.findall(r'AMS_GRA_REQUIRE_REAL_UCI_[A-Z_]+', admission))
        self.assertEqual(len(variables), 7)
        setup = runner.split('# build.rs watches every', 1)[1].split('if [ $#', 1)[0]
        self.assertEqual(set(re.findall(r'(AMS_GRA_REQUIRE_REAL_UCI_[A-Z_]+)=1', setup)), variables)
        self.assertIn('export CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3', setup)
        self.assertIn('verify_hash(&root, root_hash)', build)
        for family in ('log', 'rdma', 'multimessage', 'survivability', 'dlz',
                       'file_metadata', 'orders_metadata'):
            self.assertIn('("' + family + '",', build)


if __name__ == '__main__':
    unittest.main()