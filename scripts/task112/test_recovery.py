"""Lightweight negative controls for recovery evidence; no compiler required."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import recovery


def fixture():
    """Unit-only fake captures, never used as integration evidence."""
    def text(value):
        return dict(opcode=1, text=value)
    result = []
    for epoch, mode in enumerate(recovery.MODES, 1):
        client = [text('INIT {"schema":"002.6.0","service_id":"task110-multimessage"}')]
        client += [text(f"SUB {i} name topic") for i in range(1, 4)]
        client += [text("PUB topic {}") for _ in range(4 if mode == "pending" else 3)]
        delivered = [text("INFO {}:unit-only")] + [text("+OK") for _ in range(7)]
        delivered += [text("MSG 1 {}") for _ in range(6)]
        broker = delivered + ([text("+OK")] if mode == "pending" else [])
        wire = {key: dict(upgrade="Sec-WebSocket-Protocol: owp", frames=value)
                for key, value in dict(ada=client, sleet=broker, delivered=delivered).items()}
        result.append(dict(epoch=epoch, mode=mode, completed=True, wire=wire))
    return result


class RecoveryTests(unittest.TestCase):
    def output(self):
        lines = []
        for epoch in range(1, 9):
            lines += [f"EVENT {epoch} CONNECTION_CLOSED 0 closed", f"EVENT {epoch} INITIALIZED 0 info"]
            lines += [f"EVENT {epoch} COMMAND_ACCEPTED 0 +OK"] * 7
            lines += [f"CALL {epoch} {family} {count}" for family in range(1, 4) for count in (1, 2)]
            lines += [f"RETIRED {epoch}", f"CLEANUP {epoch} fd= 8 tasks= 3"]
        return "\n".join(lines + ["TASK112_COMPLETE epochs=8"])

    def test_output_counts_and_resources(self):
        self.assertEqual(len(recovery.validate_output(self.output())), 8)

    def test_leaked_resource_rejected(self):
        with self.assertRaisesRegex(ValueError, "growth"):
            recovery.validate_output(self.output().replace("CLEANUP 8 fd= 8", "CLEANUP 8 fd= 9"))

    def test_callback_wrong_epoch_rejected(self):
        with self.assertRaisesRegex(ValueError, "isolation"):
            recovery.validate_output(self.output().replace("CALL 8 1 1", "CALL 7 1 1"))

    def test_unexpected_eof_exception_rejected(self):
        with self.assertRaisesRegex(ValueError, "unexpected"):
            recovery.validate_output(self.output() + "\nTRANSPORT_EXCEPTION 2 not allowed")

    def test_fixture_is_only_a_verifier_unit_control(self):
        recovery.validate_wire(fixture(), "26")

    def test_missing_epoch_rejected(self):
        with self.assertRaises(ValueError):
            recovery.validate_wire(fixture()[:-1], "26")

    def test_replayed_publication_rejected(self):
        data = fixture()
        data[6]["wire"]["ada"]["frames"].append(dict(opcode=1, text="PUB stale {}"))
        with self.assertRaisesRegex(ValueError, "replay"):
            recovery.validate_wire(data, "26")

    def test_phantom_ack_rejected(self):
        data = fixture()
        data[6]["wire"]["delivered"]["frames"].append(dict(opcode=1, text="+OK"))
        with self.assertRaisesRegex(ValueError, "ACK"):
            recovery.validate_wire(data, "26")

    def test_pending_ack_must_actually_exist(self):
        data = fixture()
        data[5]["wire"]["sleet"]["frames"].pop()
        with self.assertRaisesRegex(ValueError, "withheld"):
            recovery.validate_wire(data, "26")

    def test_ws_close_not_tls_eof(self):
        data = fixture()
        data[1]["wire"]["delivered"]["frames"].append(dict(opcode=8, text="03e8"))
        with self.assertRaisesRegex(ValueError, "WS Close"):
            recovery.validate_wire(data, "26")

    def test_stale_sid_rejected(self):
        data = fixture()
        data[2]["wire"]["ada"]["frames"][1]["text"] = "SUB 4 name topic"
        with self.assertRaisesRegex(ValueError, "subscriptions"):
            recovery.validate_wire(data, "26")

    def test_release_specific_independent_documents(self):
        for release in ("25", "26"):
            corpus = recovery.plan(release)
            self.assertEqual(tuple(item["mode"] for item in corpus), recovery.MODES)
            identities = set()
            for item in corpus:
                for phase in ("publish", "external"):
                    for document in item[phase]:
                        header = document["MessageHeader"]
                        self.assertEqual(header["SchemaVersion"], f"002.{release[-1]}.0")
                    identities.add(item[phase][0]["MessageHeader"]["Timestamp"])
            self.assertEqual(len(identities), 16)
            self.assertNotIn("epoch", json.dumps(corpus[0]["publish"]))
            original = copy.deepcopy(corpus[0]["publish"])
            corpus[1]["publish"][0]["MessageData"].clear()
            self.assertEqual(corpus[0]["publish"], original)


if __name__ == "__main__":
    unittest.main()