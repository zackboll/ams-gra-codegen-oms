#!/usr/bin/env python3
"""Adversarial controls for the actual OrdersMetadata real-Sleet runner path."""
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/run-real-sleet-test.sh"
REV = "94796daf55ebd1134dce0925e6a204a05ad3423b"
TREE = "47da595b9078cac1a9351e8290d1ae7f3976367f"


def transcript():
    lines = []
    for version in ("002.5.0", "002.6.0"):
        for marker in ("MODEL CODEC", "TYPED PUBLISH AND INDEPENDENT RECEIVE",
                       "PINNED SLEET", "SUBSCRIPTION REMOVAL AND CLEANUP"):
            lines.append(f"RUST-OMS-012 {version} {marker}: PASSED")
        lines.append(f"EXTERNAL ORDERSMETADATA {version}: PASSED")
    lines.extend(["RUST-OMS-012 INDEPENDENT CARGO CONSUMER: PASSED",
                  "test result: ok. 5 passed; 0 failed; 0 ignored;",
                  "test result: ok. 1 passed; 0 failed; 0 ignored;"])
    return "\n".join(lines) + "\n"


class OrdersRunner(unittest.TestCase):
    def test_consumer_uses_repository_locked_dependencies(self):
        workspace = tomllib.loads((ROOT / "Cargo.lock").read_text())
        consumer = tomllib.loads((ROOT / "tests/fixtures/rust-oms-012/consumer.Cargo.lock").read_text())
        def identity(package):
            return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))
        allowed = {identity(package) for package in workspace["package"]}
        for package in consumer["package"]:
            if package["name"] != "orders-metadata-consumer":
                self.assertIn(identity(package), allowed)
        self.assertNotIn("ams-gra-oms-runtime-rust-facade-tests",
                         {package["name"] for package in consumer["package"]})

    def execute(self, output=None, missing=(), status=0):
        with tempfile.TemporaryDirectory(prefix="orders-runner-control-") as scratch:
            directory = Path(scratch)
            tools = directory / "bin"
            tools.mkdir()
            # Mock only external commands, not the actual runner's decisions.
            scripts = {
                "git": f'''#!/bin/bash
case "$*" in
  *"rev-parse HEAD") echo {REV};;
  *"rev-parse HEAD^{{tree}}") echo {TREE};;
esac
''',
                "rustfmt": "#!/bin/bash\nexit 0\n",
                "rustc": "#!/bin/bash\necho \"rustc $*\" >> \"$CONTROL_CALLS\"\necho 'rustc controlled version'\n",
                "cargo": '''#!/bin/bash
echo "$*" >> "$CONTROL_CALLS"
if [[ " $* " == *" test "* ]]; then
  cat "$CONTROL_OUTPUT"
  exit "$CONTROL_STATUS"
fi
''',
            }
            for name, script in scripts.items():
                path = tools / name
                path.write_text(script)
                path.chmod(0o755)
            (directory / "output").write_text(transcript() if output is None else output)
            # The real runner fingerprints the built broker; provide the
            # controlled build artifact instead of weakening that check.
            (directory / "target/release").mkdir(parents=True)
            (directory / "target/release/sleet").write_bytes(b"controlled broker")
            environment = {key: value for key, value in os.environ.items()
                           if not key.startswith("AMS_GRA_")}
            environment.update(PATH=f"{tools}:{os.environ['PATH']}",
                               AMS_GRA_RUST_ORDERS_METADATA_ONLY="1",
                               AMS_GRA_UCI_2_5_ROOT="/authentic/25.xsd",
                               AMS_GRA_UCI_2_6_ROOT="/authentic/26.xsd",
                               CONTROL_CALLS=str(directory / "calls"),
                               CONTROL_OUTPUT=str(directory / "output"),
                               CONTROL_STATUS=str(status))
            for version in missing:
                del environment[f"AMS_GRA_UCI_2_{version}_ROOT"]
            result = subprocess.run(["bash", str(RUNNER), str(directory)],
                                    env=environment, capture_output=True, text=True,
                                    check=False, timeout=10)
            calls = (directory / "calls").read_text() if (directory / "calls").exists() else ""
            return result, calls

    def test_complete_evidence(self):
        result, calls = self.execute()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(calls.count(" test "), 1)
        self.assertIn("--test real_orders_metadata --test real_orders_metadata_consumer", calls)
        self.assertNotIn("rust-uci-client", calls)
        self.assertIn("rustc --version", calls)
        self.assertIn("rustc controlled version", result.stdout)
        self.assertIn("clean=true binary=", result.stdout)
        self.assertIn(f"RUST-OMS-012 real pinned Sleet ({REV}): PASSED", result.stdout)

    def test_missing_roots_fail_before_build(self):
        for missing in [("5",), ("6",), ("5", "6")]:
            with self.subTest(missing=missing):
                result, calls = self.execute(missing=missing)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(calls, "")
                self.assertIn("root required", result.stderr)

    def test_missing_markers(self):
        for line in transcript().splitlines():
            with self.subTest(line=line):
                result, _ = self.execute(transcript().replace(line + "\n", ""))
                self.assertNotEqual(result.returncode, 0, line)

    def test_zero_skips_ignored_and_wrong_counts(self):
        for output in [transcript().replace("5 passed", "0 passed"),
                       transcript().replace("1 passed", "0 passed"),
                       transcript().replace("5 passed", "4 passed"),
                       transcript().replace("0 ignored", "1 ignored"),
                       transcript() + "SKIPPED: missing input\n",
                       transcript() + "test result: ok. 0 passed; 0 failed; 0 ignored;\n"]:
            with self.subTest(output=output):
                result, _ = self.execute(output)
                self.assertNotEqual(result.returncode, 0)

    def test_command_failure_preserved(self):
        result, _ = self.execute(transcript() + "EXACT_DIAGNOSTIC\n", status=23)
        self.assertEqual(result.returncode, 23)
        self.assertIn("EXACT_DIAGNOSTIC", result.stdout)


if __name__ == "__main__":
    unittest.main()