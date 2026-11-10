"""Real shell execution: captured stdout stays assertable and failures propagate."""
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]


class Timing(unittest.TestCase):
    def test_cargo_wrapper_retains_toolchain_flags_and_test_separator(self):
        result = subprocess.run(['bash', '-c', '''
set -euo pipefail
source "$1/scripts/qualification-timing.sh"
qualification_stage() { shift 2; printf '<%s>\\n' "$@"; }
cargo +1.95.0 test --locked --test witness -- --exact example
cargo fmt --all -- --check
''', 'bash', str(ROOT)], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [
            '<cargo>', '<+1.95.0>', '<test>', '<-vv>', '<--locked>',
            '<--test>', '<witness>', '<-->', '<--exact>', '<example>',
            '<cargo>', '<fmt>', '<--all>', '<-->', '<--check>'])

    def test_capture_live_diagnostics_and_nonzero_status(self):
        result = subprocess.run(['bash', '-c', '''
set -euo pipefail
source "$1/scripts/qualification-timing.sh"
status=0
output="$(qualification_stage negative bash -c 'echo witness; echo diagnostic >&2; exit 7')" || status=$?
test "$status" = 7
grep -Fxq witness <<<"$output"
grep -Fxq diagnostic <<<"$output"
printf 'CAPTURE_VERIFIED\n'
''', 'bash', str(ROOT)], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('CAPTURE_VERIFIED', result.stdout)
        self.assertIn('witness', result.stderr)
        self.assertIn('diagnostic', result.stderr)
        self.assertRegex(result.stderr, r'QUALIFICATION_STAGE end label=negative elapsed_seconds=\d+ exit=7')


if __name__ == '__main__':
    unittest.main()