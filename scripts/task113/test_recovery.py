"""Unit-only verifier mutation controls; never integration evidence."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task113 import recovery as r


def fixture(release="26"):
    result = []
    prefix = "task110" if release == "26" else "task106"
    topics = [prefix + "-" + key for key in r.base.cases.MESSAGES]
    for item in r.plan(release):
        mode = item["mode"]
        client = ["INIT " + json.dumps(dict(versions=["1.0"], schema=f"002.{release[-1]}.0",
                                           service_id=prefix + "-multimessage", verbose=True))]
        client += [f"SUB {sid} {name} {topic}" for sid, (name, topic) in enumerate(zip(r.base.NAMES, topics), 1)]
        delivered = ["INFO {}"] + ["+OK"] * (3 if mode == "sub" else 7)
        if mode != "sub":
            client += [f"PUB {topic} " + json.dumps({name: doc}) for name, topic, doc in
                       zip(r.base.NAMES, topics, item["publish"])]
            delivered += [f"MSG {sid} " + json.dumps({name: doc}) for phase in ("publish", "external")
                          for sid, (name, doc) in enumerate(zip(r.base.NAMES, item[phase]), 1)]
        if mode == "unsub":
            client += ["UNSUB 3"]
        server = delivered + (["+OK"] if mode != "normal" else [])
        wire = {side: dict(upgrade="Sec-WebSocket-Protocol: owp",
                           frames=[dict(opcode=1, text=t) for t in commands])
                for side, commands in dict(ada=client, sleet=server, delivered=delivered).items()}
        if mode == "normal":
            wire["ada"]["frames"].append(dict(opcode=8, text="03e8"))
        result.append(dict(epoch=item["epoch"], mode=mode, completed=True, wire=wire,
                           withheld_actual_ack=mode != "normal",
                           shutdown="socket_reset_after_actual_withheld_ack"))
    return result


def output():
    lines = []
    for epoch, mode in enumerate(r.MODES, 1):
        acks, calls = (3, 0) if mode == "sub" else (7, 2)
        lines += [f"EVENT {epoch} CONNECTION_CLOSED 0 closed", f"EVENT {epoch} INITIALIZED 0 info"]
        lines += [f"EVENT {epoch} COMMAND_ACCEPTED 0 +OK"] * acks
        if calls:
            lines += [f"CALL {epoch} {family} {count}" for family in range(1, 4) for count in (1, 2)]
        if mode != "normal":
            lines += [f"PENDING {epoch} {mode} sid=3 busy=true active=true,true,false acks={acks}",
                      f"ARMED {epoch} {mode}", f"TRANSPORT_EXCEPTION {epoch} Connection reset by peer"]
        lines += [f"RETIRED {epoch} callbacks= {calls} {calls} {calls} acks= {acks} close=1 active=0 lost=0 busy=false",
                  f"CLEANUP {epoch} fd= 8 tasks= 3"]
    return "\n".join(lines + ["TASK113_COMPLETE epochs=5"])


class RecoveryTests(unittest.TestCase):
    def test_valid_unit_fixture_both_releases(self):
        for release in ("25", "26"):
            r.validate_wire(fixture(release), release)
        self.assertEqual(len(r.validate_output(output())), 5)

    def test_plan_is_independent_and_release_specific(self):
        a, b = r.plan("25"), r.plan("26")
        self.assertEqual(tuple(i["mode"] for i in a), r.MODES)
        self.assertEqual(len({i["external"][0]["MessageHeader"]["Timestamp"] for i in a}), 5)
        self.assertEqual(a[0]["publish"][0]["MessageHeader"]["SchemaVersion"], "002.5.0")
        self.assertEqual(b[0]["publish"][0]["MessageHeader"]["SchemaVersion"], "002.6.0")
        original = copy.deepcopy(a[0])
        a[1]["publish"][0].clear()
        self.assertEqual(a[0], original)

    def test_boundaries_require_actual_command(self):
        captures = fixture()
        self.assertTrue(r.boundary("sub", captures[1]["wire"]["ada"]))
        self.assertTrue(r.boundary("unsub", captures[3]["wire"]["ada"]))
        self.assertFalse(r.boundary("unsub", captures[2]["wire"]["ada"]))
        captures[1]["wire"]["ada"]["frames"].pop()
        self.assertFalse(r.boundary("sub", captures[1]["wire"]["ada"]))

    def test_wire_mutations_rejected(self):
        mutations = [
            (1, "delivered", "append", dict(opcode=1, text="+OK")),
            (3, "delivered", "append", dict(opcode=1, text="+OK")),
            (1, "sleet", "pop", None),
            (3, "sleet", "pop", None),
            (2, "ada", "append", dict(opcode=1, text="SUB 3 stale topic")),
            (4, "ada", "append", dict(opcode=1, text="UNSUB 3")),
            (2, "ada", "append", dict(opcode=1, text="PUB stale {}")),
            (1, "delivered", "append", dict(opcode=8, text="03e8")),
            (4, "delivered", "pop", None),
        ]
        for epoch, side, operation, value in mutations:
            with self.subTest(epoch=epoch, side=side, operation=operation):
                data = fixture()
                frames = data[epoch]["wire"][side]["frames"]
                frames.append(value) if operation == "append" else frames.pop()
                with self.assertRaises(ValueError):
                    r.validate_wire(data, "26")

    def test_stale_payload_rejected(self):
        data = fixture()
        data[4]["wire"]["delivered"]["frames"][-1] = data[2]["wire"]["delivered"]["frames"][-1]
        with self.assertRaises(ValueError):
            r.validate_wire(data, "26")

    def test_missing_epoch_and_wrong_identity_rejected(self):
        with self.assertRaises(ValueError):
            r.validate_wire(fixture()[:-1], "26")
        data = fixture()
        data[2]["epoch"] = 1
        with self.assertRaises(ValueError):
            r.validate_wire(data, "26")

    def test_output_mutations_rejected(self):
        for old, new in [("CLEANUP 5 fd= 8", "CLEANUP 5 fd= 9"),
                         ("CALL 5 1 1", "CALL 3 1 1"),
                         ("busy=true active=true,true,false acks=3", "busy=true active=true,true,true acks=3"),
                         ("close=1 active=0", "close=2 active=0"),
                         ("EVENT 2 COMMAND_ACCEPTED", "EVENT 2 UNKNOWN"),
                         ("Connection reset by peer", "unexpected error"),
                         ("lost=0 busy=false", "lost=0 busy=true")]:
            with self.subTest(old=old), self.assertRaises(ValueError):
                r.validate_output(output().replace(old, new, 1))

    def test_unexpected_exception_rejected(self):
        with self.assertRaises(ValueError):
            r.validate_output(output() + "\nTRANSPORT_EXCEPTION 3 Connection reset by peer")

    def test_broker_processing_requires_independent_log(self):
        owners = {str(epoch + 1): [(str(sid), "task110-" + family, name)
                  for sid, (family, name) in enumerate(r.base.cases.MESSAGES.items(), 1)] for epoch in range(1, 6)}
        log = ("connection{id=3 service_id=x}: subscription registered sid=3 topic=topic3\n"
               "connection{id=5 service_id=x}: subscription removed sid=3\n")
        self.assertEqual(len(r.validate_broker(log, owners, fixture())["pending_operations"]), 2)
        for changed in ("", log.replace("id=3", "id=2"), log + log):
            with self.assertRaises(ValueError):
                r.validate_broker(changed, owners, fixture())

    def test_historical_profiles_are_optional(self):
        import inspect
        from task112 import qualify, verify
        for function in (r.base.execute, qualify.execute, verify.verify):
            self.assertIsNone(list(inspect.signature(function).parameters.values())[-1].default)


if __name__ == "__main__":
    unittest.main()