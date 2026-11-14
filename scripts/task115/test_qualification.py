"""Fast stdlib tests plus optional execution of the actual Ada CLI/policy tests."""
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task115 import build
import pr_smoke


class SourceTests(unittest.TestCase):
    def test_contracts_receive_only_and_release_bound(self):
        for release in ("25", "26"):
            text = (build.ROOT / f"examples/ada-uci-client/contracts/uci{release}-service.yaml").read_text()
            self.assertEqual(text.count("direction: input"), 3)
            self.assertNotIn("direction: output", text)
            self.assertIn(f'uci_schema_version: "2.{release[-1]}"', text)
            self.assertIn("name: ada-uci-client", text)

    def test_pins_match_qualified_authorities(self):
        from task112 import qualify
        self.assertEqual(build.PINS, qualify.PINS)
        self.assertEqual(build.SCHEMAS, qualify.SCHEMAS)

    def test_source_paths_owned(self):
        self.assertEqual(pr_smoke.classify(["scripts/task115/build.py"])[0], {"ada"})
        self.assertEqual(pr_smoke.classify(["examples/ada-uci-client/src/ada_uci_client.adb"])[0], {"ada"})

    def test_no_runtime_harness_dependency(self):
        for path in (build.ROOT / "examples/ada-uci-client/src").iterdir():
            text = path.read_text().lower()
            self.assertNotIn("task112", text)
            self.assertNotIn("unchecked_access", text)
            self.assertNotIn("unchecked_deallocation", text)

    def test_one_global_deadline_configured_before_each_open(self):
        source = (build.ROOT / "examples/ada-uci-client/src/ada_uci_client.adb").read_text()
        self.assertEqual(source.count("Deadline := Ada.Real_Time.Clock"), 1)
        self.assertEqual(source.count("WS.Set_Absolute_Deadline (Wire, Deadline);"), 1)
        self.assertLess(source.index("WS.Set_Absolute_Deadline"), source.index("WS.Open"))
        self.assertIn("Result := P.Duration_Result (Established);", source)
        self.assertIn('Say ("FAILED deadline_during_delay");', source)
        self.assertNotIn('Say ("COMPLETE deadline_during_delay");', source)

    def test_no_exception_message_recovery_classifier(self):
        source = (build.ROOT / "examples/ada-uci-client/src/ada_uci_client.adb").read_text()
        epoch = source[source.index("procedure Epoch"):source.index("end Epoch;")]
        self.assertNotIn("Exception_Message", epoch)
        self.assertEqual(epoch.count("Result := P.Peer_Loss;"), 1)
        self.assertIn("when Session_Lost =>", epoch)


def live_tests(scratch):
    """Run, don't model, the argument parser and supervisor decision functions."""
    out = Path(scratch).resolve()
    for source in ("policy_tests.adb",):
        (out / "build/src" / source).write_text((Path(__file__).parent / source).read_text())
    command = ['sh', '-ec', f'. "{out}/environment.sh"; exec gprbuild -p -P "{out}/build/client.gpr" policy_tests.adb']
    env = dict(os.environ, OMS_WS_CODEC_SOURCE=str(out / "codec"))
    subprocess.run(command, env=env, check=True, timeout=120)
    result = subprocess.run([out / "build/bin/policy_tests"], capture_output=True, text=True, timeout=10, check=True)
    if "POLICY_TESTS_PASS" not in result.stdout:
        raise ValueError("Ada policy tests did not execute")
    executable = out / "build/bin/ada-uci-client"
    rows = []
    for args in (["--help"], [], ["--unknown"],
                 ["--url", "ws://localhost:1/", "--ca-path", "missing"],
                 ["--url", "wss://localhost:1/", "--ca-path", "missing", "--reconnect-attempts", "9"]):
        run = subprocess.run([executable, *args], capture_output=True, text=True, timeout=3)
        expected = 0 if args == ["--help"] else 2
        if run.returncode != expected or "CONNECTING" in run.stdout:
            raise ValueError("CLI preconnection validation failure: " + repr(args))
        rows.append(dict(args=args, exit=run.returncode, stdout=run.stdout))
    (out / "unit.json").write_text(json.dumps(dict(policy=result.stdout, cli=rows), indent=2) + "\n")
    print(result.stdout, end="")


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--build":
        live_tests(sys.argv[2])
    else:
        unittest.main()