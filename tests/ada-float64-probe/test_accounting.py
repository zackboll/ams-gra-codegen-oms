import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("runner", ROOT / "scripts/check-task096.py")
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
ACCOUNT_SPEC = importlib.util.spec_from_file_location("account", ROOT / "scripts/task096/account.py")
account = importlib.util.module_from_spec(ACCOUNT_SPEC)
ACCOUNT_SPEC.loader.exec_module(account)


class AccountingTests(unittest.TestCase):
    def test_strict_conversion_attribution(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rows"
            row = "DOM\t3FF0000000000000\t3FF0000000000000\tINTEGER_VALUE\tcomplete\t1\n"
            path.write_text(row + row + "marker\n")
            with self.assertRaises(RuntimeError):
                account.conversion_rows(path, {"1": "3FF0000000000000"})
            path.write_text(row + "marker\n")
            with self.assertRaises(RuntimeError):
                account.conversion_rows(path, {"1": "3FF0000000000000"})
            with self.assertRaises(RuntimeError):
                account.require(False, "missing final result")

    def test_complete_conversion(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rows"
            rows = [
                "strtod_l\t3FF0000000000000\t3FF0000000000000\tINTEGER_VALUE\t0:1:0:3FF0000000000000\t1",
                "IEEE_Value\t3FF0000000000000\t3FF0000000000000\tINTEGER_VALUE\tcomplete\t1",
                "DOM\t3FF0000000000000\t3FF0000000000000\tINTEGER_VALUE\tJSON_Int_Type\t1",
                "TASK096_NUMERIC_COMPLETE mode=convert count= 1",
            ]
            path.write_text("\n".join(rows) + "\n")
            self.assertEqual(runner.measure_conversion(path, 1)["failures"], {})
            path.write_text("\n".join(rows[:-1]) + "\n")
            with self.assertRaises(RuntimeError):
                runner.measure_conversion(path, 1)

    def test_encoding_not_self_roundtrip(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rows"
            path.write_text("writer\t3FF0000000000001\t1.00000000000000E+00\n"
                            "format17\t3FF0000000000001\t1.0000000000000002\n"
                            "TASK096_NUMERIC_COMPLETE mode=encode count= 1\n")
            result = runner.measure_encoding(path, {"3FF0000000000001"}, runner.load_oracle())
            self.assertEqual(result["failures"], {"writer": 1})

    def test_duplicate_encoding_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rows"
            path.write_text("writer\t3FF0000000000000\t1\n" * 2 + "marker\n")
            with self.assertRaises(RuntimeError):
                runner.measure_encoding(path, {"3FF0000000000000"}, runner.load_oracle())


if __name__ == "__main__":
    unittest.main()