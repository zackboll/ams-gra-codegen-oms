"""Focused verifier controls; fabricated unit records are NOT TLS evidence."""
import copy
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task116.peer import validate, parse_prefix
import pr_smoke


def fixture(mode="handshake", millis=1000, elapsed=0.34):
    return dict(mode=mode, deadline_ms=millis, elapsed=elapsed, exit=0,
                stdout=f"RESOURCES fd= 5 tasks= 1\nDEADLINE_PROBE_PASS {mode}\n",
                cleanup=True, peer_errors=[], observations=dict(connections=1, sessions=[{}]))


class DeadlineTests(unittest.TestCase):
    def test_short_conservative_handshake(self):
        validate(fixture())

    def test_original_two_second_overrun_rejected(self):
        with self.assertRaises(ValueError):
            validate(fixture(elapsed=2.02))

    def test_missing_actual_operation_rejected(self):
        with self.assertRaises(ValueError):
            validate(fixture("write", elapsed=0.1))

    def test_harness_or_probe_failure_rejected(self):
        for update in (dict(exit=124), dict(cleanup=False), dict(peer_errors=["timeout"]), dict(stdout="")):
            case = fixture(); case.update(update)
            with self.assertRaises(ValueError):
                validate(case)

    def test_expired_no_connection(self):
        case = fixture("expired-open", -1, 0.01)
        case["observations"] = dict(connections=0, sessions=[])
        validate(case)
        case["observations"]["connections"] = 1
        with self.assertRaises(ValueError):
            validate(case)

    def test_legacy_budget(self):
        validate(fixture("legacy-handshake", 0, 2.02))
        with self.assertRaises(ValueError):
            validate(fixture("legacy-handshake", 0, 0.34))

    def test_partial_capture_is_not_complete_publication(self):
        mask = b"abcd"
        payload = b"Hello"
        wire = b"\x81\x85" + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
        self.assertEqual(parse_prefix(wire), ([(1, payload)], 0))
        self.assertEqual(parse_prefix(wire[:-1]), ([], len(wire) - 1))

    def test_unmasked_capture_rejected(self):
        with self.assertRaises(ValueError):
            parse_prefix(b"\x81\x00")

    def test_selector_ownership(self):
        self.assertEqual(pr_smoke.classify(["scripts/task116/peer.py"])[0], {"ada"})
        self.assertEqual(pr_smoke.classify(["tests/ada-wss-deadline/deadline_probe.adb"])[0], {"ada"})


if __name__ == "__main__":
    unittest.main()