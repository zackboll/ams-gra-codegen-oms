"""Focused production selection, fresh compilation, identity and layout gates."""
import importlib.util
import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from perf005_ci import available_jobs, validate_forced_build, validate_toolchain
from perf005_sandbox import consumer_layout

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), ROOT / "scripts" / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CI(unittest.TestCase):
    def test_container_git_ownership_trust_is_exact_and_nonpersistent(self):
        module = load("perf005-ci")
        root = Path("/exact/checkout")
        with patch.object(module.subprocess, "check_output", side_effect=["commit\n", "tree\n"]) as run:
            self.assertEqual(module.source_identity(root), ("commit", "tree"))
            self.assertEqual(run.call_args_list[0].args[0],
                             ["git", "-c", "safe.directory=/exact/checkout", "rev-parse", "HEAD"])
            self.assertEqual(run.call_args_list[1].args[0][-1], "HEAD^{tree}")
        source = Path(module.__file__).read_text()
        self.assertLess(source.index("commit, tree = source_identity(ROOT)"), source.index("helper.create_state("))

    def test_workflow_selects_reuse_and_cannot_revert(self):
        placement = load("test-task093-compiler-placement")
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertEqual(len(placement.placement(text)), 4)
        for old in ("scripts/perf005-ci.py", "--compilers 14 16", "--runtime-root /var/tmp/perf005-float32",
                    "--alr /opt/alire/bin/alr", "test -s /tmp/task095-evidence/perf005-ci/summary.json"):
            with self.subTest(old=old), self.assertRaises(ValueError):
                placement.placement(text.replace(old, "removed"))
        # All steps before/after the selected helper step remain byte-for-byte.
        # Pin R2 content, without requiring git history in shallow CI checkouts.
        start, end = "      - name: Float32-only helper-demand naming regression", "      - name: Portable GNAT 14 helper-demand naming regression"
        self.assertEqual(hashlib.sha256(text.split(start)[0].encode()).hexdigest(), "16e82776626a21435e9af02fb95b38402893b8516359bdefbbe9a31cd7806f24")
        self.assertEqual(hashlib.sha256(text.split(end)[1].replace("            /tmp/task095-evidence/perf005-ci/**\n", "").encode()).hexdigest(), "b6e8e88c53b6a61073e3b3625472bcba657da16a25f6ae370381f5eb45d497fd")

    def test_ci_rejects_partial_or_traced_selection_before_running(self):
        script = ROOT / "scripts/perf005-float32.py"
        for options in (["--ci"], ["--ci", "--reuse-provider", "--trace"],
                        ["--ci", "--reuse-provider", "--representative"]):
            command = [sys.executable, str(script), *options, "--", "--family", "float32", "--compiler", "14",
                       "--scratch", "/var/tmp/unused-perf005", "--generator", "/unused"]
            result = subprocess.run(command, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"CI requires complete", result.stderr)

    def test_available_allocation_not_four_cpu_precondition(self):
        with patch("perf005_ci.os.sched_getaffinity", return_value={0}), patch("perf005_ci.Path.is_file", return_value=False):
            self.assertEqual(available_jobs(), 1)
        with patch("perf005_ci.os.sched_getaffinity", return_value=set(range(8))), patch("perf005_ci.Path.is_file", return_value=True), patch("perf005_ci.Path.read_text", return_value="200000 100000"):
            self.assertEqual(available_jobs(), 2)

    def test_safe_layout_and_tmp_rejection(self):
        command = ["-XGNATCOLL_OBJECT_ROOT=/var/tmp/perf005/campaigns/c14/perf005-providers/Check/artifacts/"]
        writable, protected = consumer_layout(command, Path("/var/tmp/perf005/campaigns/c14/consumer"),
                                               Path("/var/tmp/perf005/state"), Path("/var/tmp/perf005/state/home"))
        self.assertNotIn(protected[0], writable)
        with self.assertRaises(RuntimeError):
            consumer_layout([command[0].replace("/var/tmp/", "/tmp/")], Path("/tmp/c"), Path("/tmp/state"), Path("/tmp/home"))

    def test_forced_build_requires_every_fresh_object_and_compiler_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            consumer = root / "consumer"
            support = root / "consumer-support"
            paths = [consumer / p for p in ("helper_demand_consumer.adb", "service_codec.adb", "programs.ads", "programs-oam.ads")]
            paths += [support / "src/guard.adb", support / "float32/float32.adb", support / "float32/occurrences.adb", support / "float32/bridge.c"]
            for p in paths:
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text("source")
                objdir = consumer / "obj" if p.parent == consumer else support / ("obj" if p.parent.name == "src" else "obj-float32")
                objdir.mkdir(exist_ok=True)
                obj = objdir / (p.stem + ".o")
                obj.write_bytes(b"object")
                if p.suffix != ".c":
                    obj.with_suffix(".ali").write_text("ALI")
            (consumer / "bin").mkdir()
            (consumer / "bin/helper_demand_consumer").write_bytes(b"executable")
            text = ("\n".join("[Ada] " + p.name for p in paths) + "\n[gprbind] helper\n[link] helper").encode()
            command = ["-f", "-XEXTERNALLY_BUILT=true"]
            self.assertEqual(len(validate_forced_build(command, consumer, text, 0)["compiled_sources"]), 8)
            for changed, cmd, stamp in ((text.replace(b"[Ada] programs.ads", b"skipped"), command, 0),
                                        (text, [], 0), (text, command, 2**63),
                                        (text.replace(b"[link]", b"no-link"), command, 0)):
                with self.assertRaises(RuntimeError):
                    validate_forced_build(cmd, consumer, changed, stamp)
            (consumer / "obj/programs.o").unlink()
            with self.assertRaises(RuntimeError):
                validate_forced_build(command, consumer, text, 0)

    def test_toolchain_hashes_fail_closed(self):
        from perf005_ci import COMPILERS, GPRBUILD_SHA256
        for compiler, (version, gcc, gnat, system, runtime) in COMPILERS.items():
            text = f"GNAT {version}\n" + "".join(f"{h}  /qualified/{name}\n" for name, h in
                    (("gcc", gcc), ("gnat", gnat), ("system.ads", system), ("libgnat.a", runtime), ("gprbuild", GPRBUILD_SHA256)))
            validate_toolchain(text.encode(), compiler)
            with self.assertRaises(RuntimeError):
                validate_toolchain(text.replace(gcc, "0" * 64).encode(), compiler)

    def test_state_uses_selected_verified_executable(self):
        module = load("perf005-state")
        source = Path(module.__file__).read_text()
        self.assertNotIn("exec /usr/local/bin/alr", source)
        self.assertIn("shlex.quote(str(alr))", source)
        self.assertIn("symlinks=True", source)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "alr"
            fake.write_text("#!/bin/sh\necho 'alr 2.1.1'\n")
            fake.chmod(0o755)
            with self.assertRaisesRegex(RuntimeError, "verified release"):
                module.create_state(root / "index", root / "toolchains", root / "state", fake)
            self.assertFalse((root / "state").exists())


if __name__ == "__main__":
    unittest.main()