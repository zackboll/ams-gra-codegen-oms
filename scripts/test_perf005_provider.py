#!/usr/bin/env python3
"""Fail-closed artifact identity, publication and immutable-consumption controls."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from perf005_provider import manifest, verify_variant
from perf005_sandbox import restrict_writes


class Provider(unittest.TestCase):
    def fixture(self, root):
        artifacts = root / "artifacts"
        artifacts.mkdir()
        for library in ("libgnatcoll_core.a", "libgnatcoll_minimal.a"):
            (artifacts / library).write_bytes(b"controlled test artifact")
        identity = dict(compiler="14.2.0", policy="Check", sources={"sha256": "frozen"})
        (root / "complete.json").write_text(json.dumps(dict(
            completed=True, identity=identity, artifacts=manifest(artifacts))))
        return identity

    def test_missing_corrupt_and_incomplete_artifacts(self):
        for mutation in ("missing", "corrupt", "incomplete", "unpublished", "extra", "symlink"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                identity = self.fixture(root)
                verify_variant(root, identity)
                path = root / "artifacts/libgnatcoll_core.a"
                if mutation == "missing":
                    path.unlink()
                elif mutation == "corrupt":
                    path.write_bytes(b"corrupt")
                elif mutation == "extra":
                    (root / "artifacts/extra.o").write_bytes(b"unexpected")
                elif mutation == "symlink":
                    (root / "artifacts/alias.o").symlink_to(path)
                elif mutation == "unpublished":
                    (root / "complete.json").unlink()
                else:
                    record = json.loads((root / "complete.json").read_text())
                    record["completed"] = False
                    (root / "complete.json").write_text(json.dumps(record))
                with self.assertRaises((RuntimeError, FileNotFoundError)):
                    verify_variant(root, identity)

    def test_wrong_source_compiler_policy_variant(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            identity = self.fixture(root)
            for field, wrong in (("compiler", "16.1.0"), ("policy", "Ignore"), ("sources", {"sha256": "wrong"})):
                with self.subTest(field=field), self.assertRaises(RuntimeError):
                    verify_variant(root, dict(identity, **{field: wrong}))

    def test_sandbox_denies_provider_mutation_even_by_owner(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            private = root / "private"
            private.mkdir()
            provider = root / "provider"
            provider.mkdir()
            artifact = provider / "lib.a"
            artifact.write_bytes(b"immutable")
            script = """import os,sys
from pathlib import Path
target=Path(sys.argv[1]); private=Path(sys.argv[2])
os.chmod(target,0o666)
try:
    target.write_bytes(b'corrupt')
except PermissionError:
    (private/'success').write_text('write denied')
else:
    raise SystemExit('shared provider was writable')
"""
            result = subprocess.run([sys.executable, "-c", script, str(artifact), str(private)],
                                    preexec_fn=lambda: restrict_writes([private]), capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertEqual(artifact.read_bytes(), b"immutable")
            self.assertTrue((private / "success").is_file())


if __name__ == "__main__":
    unittest.main()