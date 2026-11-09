"""Parser and evidence mutations. Synthetic fixtures are NOT runtime evidence."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from task114 import recovery as r
from task114.framing import Gate, Parser, MAX_FRAME
from task113.test_recovery import fixture as old_fixture, output as old_output

UPGRADE = b"GET / HTTP/1.1\r\nSec-WebSocket-Protocol: owp\r\n\r\n"


def frame(text, opcode=1):
    payload = text.encode() if isinstance(text, str) else text
    size = len(payload)
    header = bytes((128 | opcode, 128 | (size if size < 126 else 126 if size < 65536 else 127)))
    if size >= 126:
        header += struct.pack("!H" if size < 65536 else "!Q", size)
    mask = b"a\x00\xffz"
    return header + mask + bytes(v ^ mask[i % 4] for i, v in enumerate(payload))


def fixture(release="26"):
    data = old_fixture(release)
    for row in data:
        wire, mode = row["wire"], row["mode"]
        if mode == "normal":
            wire["ada"]["frames"].insert(-1, dict(opcode=1, text="UNSUB 3"))
            wire["sleet"]["frames"].append(dict(opcode=1, text="+OK"))
        else:
            wire["sleet"]["frames"].pop()
            row["shutdown"] = "reset_after_complete_interception_and_pending_barrier"
            if mode == "unsub":
                wire["sleet"]["frames"].append(dict(opcode=1, text="MSG 3 " + json.dumps(
                    {r.base.NAMES[2]: r.plan(release)[3]["external"][2]})))
        wire["delivered"] = copy.deepcopy(wire["sleet"])
        wire["forwarded"] = copy.deepcopy(wire["ada"])
        if mode != "normal":
            wire["forwarded"]["frames"].pop()
    return data


def output():
    text = old_output().replace("TASK113", "TASK114")
    for epoch in (1, 3, 5):
        text = text.replace(f"RETIRED {epoch} callbacks= 2 2 2 acks= 7",
                            f"EVENT {epoch} COMMAND_ACCEPTED 3 +OK\nCONTROL_UNSUB {epoch} sid=3 acks=8 active=false\n"
                            f"RETIRED {epoch} callbacks= 2 2 2 acks= 8")
        text = text.replace(f"EVENT {epoch} INITIALIZED 0 info", f"EVENT {epoch} INITIALIZED 0 info\nEVENT {epoch} COMMAND_ACCEPTED 3 +OK", 1)
        # Original synthetic events all use SID0. Keep the total count at 8.
        text = text.replace(f"EVENT {epoch} COMMAND_ACCEPTED 0 +OK\n", "", 1)
    return text + "\nPROBE_REJECTED 4 sid=3 busy=true acks=7"


class FramingTests(unittest.TestCase):
    def test_all_single_split_boundaries_and_mask(self):
        raw = UPGRADE + frame("SUB 3 SurvivabilityRiskLevel task110-survivability")
        for split in range(len(raw) + 1):
            parser = Parser()
            rows = parser.feed(raw[:split]) + parser.feed(raw[split:])
            parser.finish()
            self.assertEqual(b"".join(row[1] for row in rows), raw)
            self.assertEqual(rows[-1][2], "SUB 3 SurvivabilityRiskLevel task110-survivability")

    def test_extended_lengths_bytewise_headers(self):
        for size in (0, 125, 126, 65535, 65536, MAX_FRAME):
            parser = Parser()
            raw = UPGRADE + frame(b"x" * size)
            rows = []
            for i in range(min(len(raw), len(UPGRADE) + 14)):
                rows += parser.feed(raw[i:i + 1])
            for i in range(min(len(raw), len(UPGRADE) + 14), len(raw), 65536):
                rows += parser.feed(raw[i:i + 65536])
            parser.finish()
            self.assertEqual(len(rows[-1][2]), size)

    def test_coalesced_frames_stop_at_exact_target(self):
        preceding = UPGRADE + frame("INIT {}") + frame("SUB 1 L t") + frame("SUB 2 R t") + frame(b"p", 9)
        target = frame("SUB 3 S t")
        for chunk in (1, 2, 7, 65536):
            gate = Gate("sub")
            raw = preceding + target
            forwarded = b"".join(gate.feed(raw[i:i + chunk]) for i in range(0, len(raw), chunk))
            self.assertEqual(forwarded, preceding)
            self.assertEqual(gate.target, (len(preceding), target, "SUB 3 S t"))
            gate.parser.finish()

    def test_non_target_modes_preserve_all_bytes(self):
        raw = UPGRADE + frame("SUB 3 S t") + frame("UNSUB 3") + frame(b"\x03\xe8", 8)
        self.assertEqual(Gate("normal").feed(raw), raw)
        gate = Gate("unsub")
        with self.assertRaises(ValueError):
            gate.feed(raw)  # A frame following a selected command fails closed.

    def test_invalid_boundaries_and_limits(self):
        invalid = [b"\x01\x80" + b"1234", b"\xc1\x80" + b"1234", b"\x82\x80" + b"1234",
                   b"\x81\x00", b"\x81\xfe\x00\x01", b"\x81\xff" + struct.pack("!Q", MAX_FRAME + 1),
                   b"\x89\xfe\x00\x7e", frame(b"\xff"), frame(b"x", 8)]
        for raw in invalid:
            with self.subTest(raw=raw[:12]), self.assertRaises(ValueError):
                Parser().feed(UPGRADE + raw)
        with self.assertRaises(ValueError):
            Parser().feed(b"x" * 16385)

    def test_truncated_frames_fail_finish(self):
        raw = frame("SUB 3 S t")
        for end in range(1, len(raw)):
            parser = Parser()
            parser.feed(UPGRADE + raw[:end])
            with self.assertRaises(ValueError):
                parser.finish()


class EvidenceTests(unittest.TestCase):
    def test_valid_both_releases(self):
        for release in ("25", "26"):
            r.validate_wire(fixture(release), release)
        self.assertEqual(len(r.validate_output(output())), 5)

    def test_raw_boundary_mutations(self):
        prefix = UPGRADE + frame("SUB 1 L t") + frame("SUB 2 R t")
        target = frame("SUB 3 S t")
        row = dict(mode="sub", target_offset=len(prefix), target_bytes=len(target), target_text="SUB 3 S t",
                   target_sha256=hashlib.sha256(target).hexdigest())
        r.validate_boundary(prefix + target, prefix, row)
        for ada, forwarded in ((prefix, prefix), (prefix + target[:-1], prefix),
                               (prefix + target, prefix + target[:1]), (prefix + target, prefix[:-1]),
                               (prefix + target + target, prefix)):
            with self.subTest(lengths=(len(ada), len(forwarded))), self.assertRaises(ValueError):
                r.validate_boundary(ada, forwarded, row)

    def test_wire_mutations(self):
        for epoch, side, action, value in (
                (1, "ada", "pop", None), (1, "forwarded", "append", dict(opcode=1, text="SUB 3 S t")),
                (1, "sleet", "append", dict(opcode=1, text="+OK")),
                (3, "delivered", "append", dict(opcode=1, text="+OK")),
                (0, "sleet", "pop", None), (3, "sleet", "pop", None),
                (4, "ada", "append", dict(opcode=1, text="PUB stale {}")),
                (4, "ada", "append", dict(opcode=1, text="SUB 3 S t"))):
            data = fixture()
            frames = data[epoch]["wire"][side]["frames"]
            frames.pop() if action == "pop" else frames.append(value)
            with self.subTest(epoch=epoch, side=side), self.assertRaises(ValueError):
                r.validate_wire(data, "26")

    def test_output_mutations(self):
        for old, new in (("PROBE_REJECTED 4", "PROBE_REJECTED 3"),
                         ("CONTROL_UNSUB 3", "CONTROL_UNSUB 1"),
                         ("CLEANUP 5 fd= 8", "CLEANUP 5 fd= 9"),
                         ("busy=true active=true,true,false acks=3", "busy=false active=true,true,true acks=4"),
                         ("close=1 active=0", "close=2 active=1"),
                         ("TASK114_COMPLETE", "TIMEOUT"), ("lost=0 busy=false", "lost=1 busy=true")):
            with self.subTest(old=old), self.assertRaises(ValueError):
                r.validate_output(output().replace(old, new, 1))

    def test_broker_mutations(self):
        owners, lines = {}, []
        for epoch, mode in enumerate(r.MODES, 1):
            connection = str(epoch + 1)
            owners[connection] = [(str(sid), "task110-" + family, name)
                                  for sid, (family, name) in enumerate(r.base.cases.MESSAGES.items(), 1)
                                  if sid < 3 or mode != "sub"]
            for sid, topic, name in owners[connection]:
                lines.append(f"connection{{id={connection}}}: subscription registered sid={sid} topic={topic} message_name={name}")
            if mode == "normal":
                lines.append(f"connection{{id={connection}}}: subscription removed sid=3")
            ops = ["INIT"] + ["SUB"] * (2 if mode == "sub" else 3)
            if mode != "sub":
                ops += ["PUB"] * 3
            if mode == "normal":
                ops += ["UNSUB"]
            for op in ops:
                lines += [f'connection{{id={connection}}}: recv operation="{op}"',
                          f'connection{{id={connection}}}: send operation="+OK"']
            lines.append(f"connection{{id={connection}}}: disconnected")
        log = "\n".join(lines)
        r.validate_broker(log, owners, fixture())
        for changed in (log + "\nconnection{id=3}: subscription registered sid=3 topic=t",
                        log + "\nconnection{id=5}: subscription removed sid=3",
                        log.replace("connection{id=4}: subscription removed sid=3", ""),
                        log + '\nconnection{id=5}: recv operation="UNSUB"',
                        log.replace("disconnected", "")):
            with self.assertRaises(ValueError):
                r.validate_broker(changed, owners, fixture())

    def test_historical_defaults_remain_optional(self):
        import inspect
        from task112 import qualify, verify
        for function in (r.base.execute, qualify.execute, verify.verify):
            self.assertIsNone(list(inspect.signature(function).parameters.values())[-1].default)


if __name__ == "__main__":
    unittest.main()