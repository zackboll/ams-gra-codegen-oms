"""Focused production selection, fresh compilation, identity and layout gates."""
import importlib.util
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

    def test_unrelated_workflow_maintenance_is_accepted(self):
        placement = load("test-task093-compiler-placement")
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        optimized = placement.job(text, "ada-float32-codec")
        mutations = {
            "comment": text.replace("# Task 055: Fast CI.", "# Harmless maintenance: Fast CI."),
            "description": text.replace("Qualify Python model generation (runtime probes must execute)",
                                        "Qualify Python model generation and runtime probes"),
            "step": text.replace(
                "      - name: Qualify Python model generation", "      - name: Describe Python runner\n"
                "        run: echo 'Python qualification runner'\n      - name: Qualify Python model generation", 1),
        }
        for label, changed in mutations.items():
            with self.subTest(mutation=label):
                self.assertNotEqual(changed, text)
                self.assertEqual(placement.job(changed, "ada-float32-codec"), optimized)
                self.assertEqual(placement.placement(changed), placement.placement(text))

    def test_float32_qualification_mutations_are_rejected(self):
        placement = load("test-task093-compiler-placement")
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        body = placement.job(text, "ada-float32-codec")
        replacements = [
            ("wrapper", "python3 scripts/perf005-ci.py", "echo removed"),
            ("GNAT14", "--compilers 14 16", "--compilers 16"),
            ("GNAT16", "--compilers 14 16", "--compilers 14"),
            ("compiler order", "--compilers 14 16", "--compilers 16 14"),
            ("partial profiles", "--compilers 14 16", "--compilers 14 16 --profiles oam"),
            ("partial policies", "--compilers 14 16", "--compilers 14 16 --policies normal"),
            ("Alire selection", "--alr /opt/alire/bin/alr", ""),
            ("Alire verification", "sha256sum --check --strict", "cat"),
            ("unsafe layout", "--runtime-root /var/tmp/perf005-float32", "--runtime-root /tmp/perf005-float32"),
            ("overlapping evidence", "--evidence /tmp/task095-evidence/perf005-ci", "--evidence /var/tmp/perf005-float32"),
            ("summary", "test -s /tmp/task095-evidence/perf005-ci/summary.json", "echo removed"),
            ("manifest", "sha256sum --check --quiet /tmp/task095-evidence/perf005-ci/evidence.sha256", "echo removed"),
            ("case-preserving upload", "            /tmp/task095-evidence/perf005-ci/**\n", ""),
            ("upload failure", "if-no-files-found: error", "if-no-files-found: warn"),
            ("portable GNAT14", "python3 scripts/check-task093-helper-demand.py --family portable --compiler 14", "echo removed"),
            ("numeric corpus", "--numeric-corpus", ""),
            ("environment", "--environment", ""),
            ("mixed", "python3 scripts/check-task093-mixed.py", "echo removed"),
            ("job ignored", "    runs-on:", "    continue-on-error: true\n    runs-on:"),
            ("job skipped", "    runs-on:", "    if: false\n    runs-on:"),
            ("step ignored", "        shell: bash", "        continue-on-error: true\n        shell: bash"),
            ("step skipped", "        shell: bash", "        if: false\n        shell: bash"),
            ("shell failure swallowed", "--numeric-corpus", "--numeric-corpus || true"),
            ("errexit disabled", "          set -euo pipefail", "          set -euo pipefail\n          set +e"),
            ("early success", "          set -euo pipefail", "          set -euo pipefail\n          exit 0"),
        ]
        for label, old, new in replacements:
            with self.subTest(mutation=label):
                self.assertIn(old, body)
                changed = text.replace(body, body.replace(old, new))
                self.assertNotEqual(changed, text)
                with self.assertRaises(ValueError):
                    placement.placement(changed)
        for name in ("Float32-only helper-demand naming regression",
                     "Portable GNAT 14 helper-demand naming regression",
                     "Execute both compilers profiles and assertion policies",
                     "Binary Float32 composition in the qualified runtime"):
            start = body.index("      - name: " + name + "\n")
            end = body.index("      - ", start + 1)
            with self.subTest(removed_step=name), self.assertRaises(ValueError):
                placement.placement(text.replace(body, body[:start] + body[end:]))

    def test_wrapper_retains_provider_selection_and_complete_evidence(self):
        source = (ROOT / "scripts/perf005-ci.py").read_text()
        for required in ('"--ci", "--reuse-provider", "--"',
                         'check=True', 'archive_campaign(campaign, evidence / f"{campaign.name}.tar.gz")',
                         '(6, 60, 40, 480, 60)', '"evidence.sha256"', 'validate_layout('):
            with self.subTest(required=required):
                self.assertIn(required, source)

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