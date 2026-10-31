"""R1 independent evidence gates; fake fixtures are verifier-only controls."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from perf009_evidence import read_inputs, seal, verify_artifacts, verify_seal, verify_trace
from perf005_provider import digest
from perf005_provider import manifest
from perf009_provider import ORIGIN, validate_lock, verify_provider

ROOT = Path(__file__).resolve().parents[1]


class Evidence(unittest.TestCase):
    def test_provider_publication_hash_and_policy_cannot_be_rewritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            artifacts = root / "artifacts"
            artifacts.mkdir()
            for name in ("unit.o", "libgnatcoll_core.a", "libgnatcoll_minimal.a"):
                (artifacts / name).write_bytes(b"prepared")
                (artifacts / name).chmod(0o444)
            identity = dict(compiler="14", policy="Check")
            record = dict(completed=True, identity=identity, artifacts=manifest(artifacts))
            (root / "complete.json").write_text(json.dumps(record))
            expected = digest(root / "complete.json")
            verify_provider(root, identity, expected)
            with self.assertRaises(RuntimeError):
                verify_provider(root, dict(identity, policy="Ignore"), expected)
            path = artifacts / "unit.o"
            path.chmod(0o644)
            path.write_bytes(b"mutated")
            path.chmod(0o444)
            record["artifacts"] = manifest(artifacts)
            (root / "complete.json").write_text(json.dumps(record))
            with self.assertRaisesRegex(RuntimeError, "manifest changed"):
                verify_provider(root, identity, expected)

    def test_lock_exact_versions_unique_releases_and_origin(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "lock"
            releases = [("gnatcoll", "26.0.0"), ("gnatcoll_minimal", "26.0.0"),
                        ("gprbuild", "26.0.1"), ("gnat_external", "14.2.0")]
            def text(items):
                return "".join(f'[[solution.state]]\ncrate="{name}"\n[solution.state.release]\nname="{name}"\nversion="{version}"\n'
                               f'[solution.state.release.origin]\nhashes=["{ORIGIN}"]\n' for name, version in items)
            alias = text([releases[-1]]).replace('crate="gnat_external"', 'crate="gnat"')
            original = text(releases) + alias
            path.write_text(original)
            validate_lock(path, "14")
            for value in (original + text([releases[0]]), original.replace("26.0.0", "25.0.0"),
                          original.replace(ORIGIN, "wrong"), text(releases), original + alias,
                          text(releases) + alias.replace('version="14.2.0"', 'version="16.1.0"'),
                          original + text([releases[0]]).replace('crate="gnatcoll"', 'crate="unexpected"')):
                path.write_text(value)
                with self.assertRaises(RuntimeError):
                    validate_lock(path, "14")

    def test_native_compiler_virtual_alias_is_exact(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "lock"
            releases = [("gnatcoll", "26.0.0"), ("gnatcoll_minimal", "26.0.0"),
                        ("gprbuild", "26.0.1"), ("gnat_native", "16.1.0")]
            original = "".join(f'[[solution.state]]\ncrate="{name}"\n[solution.state.release]\n'
                               f'name="{name}"\nversion="{version}"\n[solution.state.release.origin]\n'
                               f'hashes=["{ORIGIN}"]\n' for name, version in releases)
            alias = original[original.rfind('[[solution.state]]'):].replace('crate="gnat_native"', 'crate="gnat"')
            path.write_text(original + alias)
            validate_lock(path, "16")
            for changed in (original + alias.replace(ORIGIN, "changed"),
                            original + alias + alias,
                            original + alias + '[[solution.state]]\ncrate="other"\n'
                            '[solution.state.release]\nname="gnat_external"\nversion="14.2.0"\n'):
                path.write_text(changed)
                with self.assertRaises(RuntimeError):
                    validate_lock(path, "16")

    def test_seal_rejects_truncation_extra_altered_and_symlink(self):
        for mutation in ("truncate", "alter", "extra", "symlink", "missing"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                (root / "one").write_text("one")
                (root / "two").write_text("two")
                expected = seal(root)
                verify_seal(root, expected)
                if mutation == "truncate":
                    record = json.loads((root / "evidence-seal.json").read_text())
                    record["files"].pop("two")
                    (root / "evidence-seal.json").write_text(json.dumps(record))
                elif mutation == "alter":
                    (root / "one").write_text("changed")
                elif mutation == "extra":
                    (root / "three").write_text("extra")
                elif mutation == "symlink":
                    (root / "three").symlink_to(root / "one")
                else:
                    (root / "one").unlink()
                with self.assertRaises(RuntimeError):
                    verify_seal(root, expected)

    def test_read_time_configuration_not_postbuild_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            trace = root / "build.strace"
            trace.write_text('12 100.0 execve("/tool/gnat1", ["/tool/gnat1", "-gnatec=/tmp/GPR.1/config"], 0x1) = 0\n'
                             '12 100.1 read(3</tmp/GPR.1/config>, "actual\\n", 4096) = 7\n'
                             '12 101.0 +++ exited with 0 +++\n')
            from perf005_profile import executions
            data = dict(executions=executions(trace), pre_execution_mapping_inputs={},
                        gpr_temporary_inputs={"/tmp/GPR.1/config": "actual\n"})
            profile = root / "build.profile.json"
            profile.write_text(json.dumps(data))
            verify_trace(profile)
            data["gpr_temporary_inputs"]["/tmp/GPR.1/config"] = "postbuild changed\n"
            profile.write_text(json.dumps(data))
            with self.assertRaisesRegex(RuntimeError, "read-time"):
                verify_trace(profile)

    def test_response_read_witness_and_truncated_trace(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            trace = root / "build.strace"
            trace.write_text('12 100.0 execve("/tool/gcc", ["/tool/gcc", "-c", "@/tmp/GPR.1/response"], 0x1) = 0\n'
                             '12 100.1 read(3</tmp/GPR.1/response>, "-O0\\n", 4096) = 4\n'
                             '12 101.0 +++ exited with 0 +++\n')
            from perf005_profile import executions
            data = dict(executions=executions(trace), pre_execution_mapping_inputs={},
                        gpr_temporary_inputs={"/tmp/GPR.1/response": "-O0\n"})
            profile = root / "build.profile.json"
            profile.write_text(json.dumps(data))
            verify_trace(profile)
            data["gpr_temporary_inputs"]["/tmp/GPR.1/response"] = "-O2\n"
            profile.write_text(json.dumps(data))
            with self.assertRaisesRegex(RuntimeError, "read-time"):
                verify_trace(profile)
            trace.write_text(trace.read_text().replace(' = 4', ' = 99'))
            with self.assertRaisesRegex(RuntimeError, "truncated"):
                read_inputs(trace, digest(trace))

    def test_artifact_inventory_rejects_alias_and_stale(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            artifacts = {}
            for i in range(11):
                p = root / f"{i}.o"
                p.write_bytes(b"artifact")
                artifacts[str(p)] = digest(p)
            verify_artifacts(dict(artifacts=artifacts), 0)
            with self.assertRaises(RuntimeError):
                verify_artifacts(dict(artifacts=artifacts), 2**63)
            duplicate = dict(artifacts)
            duplicate[str(root / ".." / root.name / "0.o")] = digest(root / "0.o")
            with self.assertRaises(RuntimeError):
                verify_artifacts(dict(artifacts=duplicate), 0)

    def test_contention_includes_smt_and_fails_closed(self):
        spec = importlib.util.spec_from_file_location("benchmark", ROOT / "scripts/perf009-benchmark.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        # Use the actual topology with a controlled all-busy counter delta.
        cpus = set(module.os.sched_getaffinity(0))
        before = {cpu: [0] * 10 for cpu in range(max(cpus) + 1)}
        after = {cpu: [100] + [0] * 9 for cpu in range(max(cpus) + 1)}
        with patch.object(module, "cpu_counters", side_effect=[before, after]), patch.object(module.time, "sleep"):
            result = module.contention()
        self.assertFalse(result["uncontended"])
        self.assertTrue(set(result["physical_siblings"]) >= set(result["affinity"]))


if __name__ == "__main__":
    unittest.main()