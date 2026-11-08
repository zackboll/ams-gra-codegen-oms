"""Offline mutation tests; synthetic observations are not live TLS evidence."""
import copy
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task111.qualify import consumer_source
from task111.trust import NEGATIVE, validate
from task111.receipt import check_resources
import pr_smoke


class QualificationTests(unittest.TestCase):
    def test_adaptation_is_only_trust_input(self):
        for release in ("25", "26"):
            path = ROOT / ("tests/ada-real-uci-multimessage" + ("26" if release == "26" else "")) / "multimessage_consumer.adb"
            old = path.read_text()
            new = consumer_source(old, release)
            self.assertIn('CA_Path => ""', new)
            self.assertIn('CA_PEM => Test_PEM.Read', new)
            restored = new.replace("with Test_PEM;\n", "")
            if release == "25":
                restored = restored.replace("with Ada.Environment_Variables;\n", "")
            restored = restored.replace(
                '   WS.Open (Wire, Ada.Command_Line.Argument (1), CA_Path => "",\n'
                '     CA_PEM => Test_PEM.Read (Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA")));',
                '   WS.Open (Wire, Ada.Command_Line.Argument (1));' if release == "25" else
                '   WS.Open (Wire, Ada.Command_Line.Argument (1),\n'
                '     CA_Path => Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA", ""));')
            self.assertEqual(restored, old)

    def test_adaptation_fails_on_drift(self):
        for release in ("25", "26"):
            with self.assertRaises(ValueError):
                consumer_source("changed shape", release)

    def test_all_required_cases(self):
        self.assertEqual(set(NEGATIVE), {"untrusted", "malformed", "hostname", "both", "plaintext", "handshake-close", "upgrade"})

    def test_positive_control_and_mutations(self):
        peer = dict(accepted=True, clienthello=True, tls="TLSv1.3", application=["INIT {}"], normal_close=True)
        validate("pem", "DONE pem", peer)
        for key, value in (("clienthello", False), ("normal_close", False), ("application", []),
                           ("tls", None), ("error", "fixture crashed")):
            bad = dict(peer, **{key: value})
            with self.assertRaises(ValueError):
                validate("pem", "DONE pem", bad)

    def test_rejection_control_and_mutations(self):
        peer = dict(accepted=True, clienthello=True, application=[], handshake_failed="alert")
        output = "REJECTED category=Transport_Error\nDONE untrusted"
        validate("untrusted", output, peer)
        for key, value in (("application", ["INIT {}"]), ("clienthello", False), ("handshake_failed", None),
                           ("error", "socket timed out"), ("accepted", False)):
            with self.assertRaises(ValueError):
                validate("untrusted", output, dict(peer, **{key: value}))
        for bad in ("DONE untrusted", output.replace("Transport_Error", "Program_Error"), "HARNESS_FAILURE\n" + output):
            with self.assertRaises(ValueError):
                validate("untrusted", bad, peer)

    def test_configuration_cannot_dial(self):
        for mode in ("both", "plaintext"):
            output = "REJECTED category=Transport_Error\nDONE " + mode
            validate(mode, output, dict(accepted=False, application=[]))
            with self.assertRaises(ValueError):
                validate(mode, output, dict(accepted=True, application=[]))

    def test_malformed_der_may_dial_but_never_handshake(self):
        output = "REJECTED category=Transport_Error\nDONE malformed"
        validate("malformed", output, dict(accepted=True, pre_tls_eof=True, application=[]))
        validate("malformed", output, dict(accepted=False, application=[]))
        with self.assertRaises(ValueError):
            validate("malformed", output, dict(accepted=True, clienthello=True, application=[]))

    def test_upgrade_requires_verified_tls_and_eof(self):
        peer = dict(accepted=True, clienthello=True, tls="TLSv1.2", application=[], invalid_upgrade="not-owp", eof=True)
        output = "REJECTED category=Transport_Error\nDONE upgrade"
        validate("upgrade", output, peer)
        for key in ("tls", "invalid_upgrade", "eof"):
            bad = copy.deepcopy(peer)
            del bad[key]
            with self.assertRaises(ValueError):
                validate("upgrade", output, bad)

    def test_selector_changed_paths(self):
        for path in ("scripts/task111/trust.py", "tests/ada-wss-trust/trust_probe.adb"):
            self.assertEqual(pr_smoke.classify([path])[0], {"ada"})
        self.assertEqual(pr_smoke.classify(["scripts/task1110/trust.py"])[0], set(pr_smoke.BACKENDS))

    def test_resource_receipt_distinguishes_warmup_and_leaks(self):
        case = dict(mode="pem", output="RESOURCES fd= 5 tasks= 1\nDONE pem", seconds=0.05,
                    peer=dict(accepted=True, clienthello=True, tls="TLSv1.3", application=["INIT {}"], normal_close=True))
        focused = dict(baseline="RESOURCES fd= 5 tasks= 1\nDONE baseline",
                       final="RESOURCES fd= 5 tasks= 1\nCOMPLETE", cases=[copy.deepcopy(case) for _ in range(15)])
        focused["cases"][0]["output"] = focused["cases"][0]["output"].replace("fd= 5", "fd= 4")
        check_resources(focused)
        for token in ("fd= 6 tasks= 1", "fd= 5 tasks= 2"):
            bad = copy.deepcopy(focused)
            bad["cases"][12]["output"] = "RESOURCES " + token + "\nDONE pem"
            with self.assertRaises(ValueError):
                check_resources(bad)
        bad = copy.deepcopy(focused)
        bad["cases"][12]["seconds"] = 10
        with self.assertRaises(ValueError):
            check_resources(bad)
        bad = copy.deepcopy(focused)
        bad["final"] = "RESOURCES fd= 5 tasks= 2\nCOMPLETE"
        with self.assertRaises(ValueError):
            check_resources(bad)


if __name__ == "__main__":
    unittest.main()