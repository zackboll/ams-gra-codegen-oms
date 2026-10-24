"""Fail-closed map semantics, read-byte integrity, and identity controls."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from perf005_mapping import parse_mapping, provider_equivalence, reconcile_maps, source_reads


class Mapping(unittest.TestCase):
    def test_reconciled_maps_do_not_relax_other_effective_inputs(self):
        source = "<CAMPAIGN>/provider.adb"
        left = [["-O2", ["-gnatem=", [["unit%b", "provider.adb", "/left/provider.adb"]]], source]]
        left += [["-O2", f"source-{i}.c"] for i in range(105)]
        right = [["-O2", ["-gnatem=", [["unit%b", "provider.adb", "/right/provider.adb"]]], source]]
        right += [["-O2", f"source-{i}.c"] for i in range(105)]
        reports = [dict(source=source)]
        with patch("perf005_mapping.reconcile_policy", return_value=reports):
            with patch("perf005_mapping.provider_inputs", side_effect=[left, right]):
                self.assertEqual(provider_equivalence(Path("/left"), Path("/right"), "normal"), reports)
            for mutation in ("option", "inventory", "unreconciled"):
                import copy
                changed = copy.deepcopy(right)
                if mutation == "option":
                    changed[1][0] = "-O0"
                elif mutation == "inventory":
                    changed.pop()
                else:
                    changed[0][-1] = "<CAMPAIGN>/unreconciled.adb"
                with self.subTest(mutation=mutation), patch("perf005_mapping.provider_inputs", side_effect=[left, changed]):
                    with self.assertRaises(RuntimeError):
                        provider_equivalence(Path("/left"), Path("/right"), "normal")

    def test_parser_rejects_both_replaceable_keys_and_incomplete_evidence(self):
        valid = "unit%s\nunit.ads\n/src/unit.ads\n"
        self.assertEqual(parse_mapping(valid), [("unit%s", "unit.ads", "/src/unit.ads")])
        for text in ("", "unit%s\nunit.ads\n", "unit\nunit.ads\n/src/unit.ads\n",
                     valid + valid,
                     valid + "other%s\nunit.ads\n/other/unit.ads\n",
                     valid + "unit%s\nalternate.ads\n/src/alternate.ads\n",
                     "unit%s\nunit.ads\n/\n"):
            with self.subTest(text=text), self.assertRaises(RuntimeError):
                parse_mapping(text)

    def test_runtime_only_equivalence_and_mutations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "original", root / "pilot"
            for side in (left, right):
                (side / "adainclude").mkdir(parents=True)
                (side / "unit.ads").write_text("package Unit is end Unit;\n")
                (side / "adainclude/ada.ads").write_text("package Ada is end Ada;\n")
                (side / "oam-ArraySample-provider-identity.log").write_text(str(side / "adainclude") + "\n")
            def mapping(side):
                return f"unit%s\nunit.ads\n{side}/unit.ads\n"
            extra = f"ada%s\nada.ads\n{right}/adainclude/ada.ads\n"
            reads = {"<CAMPAIGN>/unit.ads": hashlib.sha256((left / "unit.ads").read_bytes()).hexdigest()}
            def check(text=None, other_reads=None):
                return reconcile_maps(mapping(left), mapping(right) + (extra if text is None else text),
                                      left, right, reads, reads if other_reads is None else other_reads,
                                      ["/usr/bin/gcc", "/usr/bin/gcc"])
            with patch("perf005_mapping.runtime_filename", return_value="ada.ads"):
                self.assertEqual(check()["resolved"][-1]["classification"], "Proven irrelevant mapping difference")
                for text in (extra.replace("ada%s", "ada%b"), extra.replace(str(right / "adainclude"), str(left)),
                             extra + f"ada%s\nalternate.ads\n{right}/alternate.ads\n"):
                    with self.subTest(text=text), self.assertRaises(RuntimeError):
                        check(text)
                with self.assertRaisesRegex(RuntimeError, "resolution differs"):
                    check(other_reads={"different": "hash"})
                (right / "adainclude/ada.ads").write_text("package Ada is X : Integer; end Ada;\n")
                with self.assertRaisesRegex(RuntimeError, "hash differs"):
                    check()
                (right / "unit.ads").write_text("package Unit is X : Integer; end Unit;\n")
                with self.assertRaisesRegex(RuntimeError, "hash differs"):
                    check("")

    def test_source_reads_require_complete_exact_bytes_and_handle_resumption(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "unit.ads"
            source.write_bytes(b"--\xff\n")
            trace = root / "reads.strace"
            trace.write_text(f'12 1.0 read(6<{source}>, <unfinished ...>\n'
                             '12 1.1 <... read resumed>"--\\377\\n", 4) = 4\n')
            self.assertEqual(source_reads(trace, frozenset([12]))[12][str(source)],
                             hashlib.sha256(source.read_bytes()).hexdigest())
            source_reads.cache_clear()
            source.write_bytes(b"changed")
            with self.assertRaisesRegex(RuntimeError, "incomplete/changed"):
                source_reads(trace, frozenset([12]))
            source_reads.cache_clear()
            trace.write_text(f'12 1.0 read(6<{source}>, "x", 4) = 4\n')
            with self.assertRaisesRegex(RuntimeError, "truncated"):
                source_reads(trace, frozenset([12]))


if __name__ == "__main__":
    unittest.main()