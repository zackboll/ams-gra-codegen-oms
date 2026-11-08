"""Lightweight stdlib checks; no toolchain, broker or schema download required."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


qualify = load("task109_qualify", ROOT / "scripts/task109/qualify.py")
relay = load("task109_relay", ROOT / "scripts/task109/relay.py")


class QualificationTests(unittest.TestCase):
    def test_only_ca_input_changes(self):
        source = (ROOT / "tests/ada-real-uci-multimessage/multimessage_consumer.adb").read_text()
        adapted = qualify.consumer_source(source)
        restored = adapted.removeprefix("with Ada.Environment_Variables;\n").replace(
            '   WS.Open (Wire, Ada.Command_Line.Argument (1),\n'
            '     CA_Path => Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA", ""));',
            "   WS.Open (Wire, Ada.Command_Line.Argument (1));")
        self.assertEqual(restored, source)
        self.assertIn("C.Poll (Core, Delivery);", adapted)
        self.assertNotIn("Insecure", adapted)

    def test_fail_closed_consumer_shape(self):
        for source in ("begin null; end;", "   WS.Open (Wire, Ada.Command_Line.Argument (1));" * 2):
            with self.assertRaises(ValueError):
                qualify.consumer_source(source)

    def test_passive_masked_and_control_frames(self):
        payload, mask = b"INIT {}", b"abcd"
        masked = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
        wire = b"GET / HTTP/1.1\r\n\r\n" + bytes([0x81, 0x80 | len(payload)]) + mask + masked + b"\x89\x00\x88\x00"
        result = relay.frames(wire)
        self.assertEqual([f["opcode"] for f in result["frames"]], [1, 9, 8])
        self.assertEqual(result["frames"][0]["text"], "INIT {}")

    def test_extended_payloads(self):
        for size, marker, width in ((200, 126, 2), (66000, 127, 8)):
            data = b"HTTP/1.1 101 Switching Protocols\r\n\r\n\x81" + bytes([marker]) + size.to_bytes(width, "big") + b"x" * size
            self.assertEqual(relay.frames(data)["frames"][0]["bytes"], size)

    def test_partial_wire_is_not_success(self):
        for data in (b"", b"HTTP\r\n\r\n\x81", b"HTTP\r\n\r\n\x81\x05x"):
            with self.assertRaises(ValueError):
                relay.frames(data)


if __name__ == "__main__":
    unittest.main()