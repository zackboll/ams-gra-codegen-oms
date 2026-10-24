#!/usr/bin/env python3
"""Complete passing synthetic campaigns plus compiler-evidence mutations."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import perf005_evidence as evidence
from perf005_provider import digest, manifest
from test_perf005_equivalence import compare


COMPILER_VERSION = "16.1.0"
COMPILER_HASH = "238f4adafcb1425c84da6519068fdfcf33e57e406fc793e2958a9238789900de"


def fixture(root):
    root.mkdir()
    records, provenance, hashes = [], [], []
    # Retained evidence may refer to a qualified container-only compiler. Keep
    # this synthetic path private and absent, never bound to the host's GCC.
    compiler = str(root / "synthetic-toolchain/bin/gcc")
    for policy in compare.POLICIES:
        variant = root / "perf005-providers" / policy
        artifacts = variant / "artifacts"
        artifacts.mkdir(parents=True)
        for name in ("libgnatcoll_core.a", "libgnatcoll_minimal.a"):
            (artifacts / name).write_bytes(b"test artifact")
        (variant / "complete.json").write_text(json.dumps(dict(completed=True, identity=dict(policy=policy), artifacts=manifest(artifacts))))
        (root / f"provider-{policy}-build.log").write_text("libgnatcoll_core.a libgnatcoll_minimal.a")
    for profile in ("oam", "qualified"):
        for name in compare.NAMES:
            label = f"{profile}-{name}"
            consumer = root / label
            support = root / (label + "-support")
            consumer.mkdir()
            (root / (label + "-plain")).mkdir()
            namespace = "oam" if profile == "oam" else "task093"
            paths = [consumer / n for n in ("helper_demand_consumer.adb", "service_codec.adb", "programs.ads", f"programs-{namespace}.ads")]
            paths += [support / "src/ams_gra_oms_json_guard.adb", support / "float32/ams_gra_oms_json_float32.adb", support / "float32/ams_gra_oms_json_occurrences.adb", support / "float32/ams_gra_float32.c"]
            for p in paths:
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text("frozen fixture source\n")
                hashes.append(f"{digest(p)}  {p}\n")
            for dep in ("gnatcoll", "gnatcoll_minimal"):
                directory = consumer / "alire/cache/dependencies" / (dep + "_26.0.0_test")
                directory.mkdir(parents=True)
                (directory / "source.ads").write_text("same provider")
            (root / f"{label}-provider-identity.log").write_text(f"{compiler}\nGNAT {COMPILER_VERSION}\n{COMPILER_HASH}  {compiler}\n")
            for suffix in ("portable-reject", "portable-generate-reject"):
                (root / f"{label}-{suffix}.log").write_text("expected rejection")
                (root / f"{label}-{suffix}.profile.json").write_text('{"exit":1}')
            for policy in compare.POLICIES:
                cell = f"{label}-{policy}"
                records.append(dict(profile=profile, name=name, policy=policy, compiler=COMPILER_VERSION))
                (root / f"{cell}-alire.lock").write_text("frozen lock")
                (root / f"{cell}-gnat.log").write_text(f"GNAT {COMPILER_VERSION}")
                (root / f"{cell}-execute.log").write_bytes(b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n")
                adc = root / f"{cell}.adc"
                if policy != "normal":
                    adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                invocations = []
                for p in paths:
                    args = [compiler, "-c", "-gnat2022", "-gnato", "-O0", str(p)]
                    if p.suffix != ".c" and policy != "normal":
                        args.append(f"-gnatec={adc}")
                    invocations.append(dict(executable=compiler, argv=args, exit=0, inclusive_elapsed_seconds=0.1))
                for p in paths:
                    backend_name = "cc1" if p.suffix == ".c" else "gnat1"
                    backend = str(Path(compiler).parent.parent / f"libexec/gcc/x86_64-pc-linux-gnu/{COMPILER_VERSION}" / backend_name)
                    args = [backend, str(p)]
                    if p.suffix != ".c" and policy != "normal":
                        args.append(f"-gnatec={adc}")
                    invocations.append(dict(executable=backend, argv=args, exit=0, inclusive_elapsed_seconds=0.05))
                (root / f"{cell}-build.profile.json").write_text(json.dumps(dict(
                    exit=0, command=["-f", "-XEXTERNALLY_BUILT=true"], executions=invocations)))
                (root / f"{cell}-build.log").write_text("\n".join(("[C] " if p.suffix == ".c" else "[Ada] ") + p.name for p in paths))
                variant = root / "perf005-providers" / policy
                provenance.append(dict(cell=cell + "-build", provider=str(variant), policy=policy, manifest_sha256=digest(variant / "complete.json")))
    (root / "runtime.log").write_text("qualified runtime")
    (root / "sources.sha256").write_text("".join(hashes))
    (root / "result.json").write_text(json.dumps(dict(completed=True, red=False, records=records)))
    (root / "perf005-provider-consumers.json").write_text(json.dumps(provenance))


class Campaign(unittest.TestCase):
    def test_complete_campaign_and_compiler_mutations(self):
        with tempfile.TemporaryDirectory(dir=Path.home()) as directory:
            baseline, pilot, traced = [Path(directory) / n for n in ("baseline", "pilot", "traced")]
            for root in (baseline, pilot, traced):
                fixture(root)
            self.assertEqual(compare.compare(baseline, pilot, traced)["positive_cells"], 30)
            path = traced / "oam-ArraySample-Check-build.profile.json"
            original = json.loads(path.read_text())
            for mutation, diagnostic in (
                    ("namespace", "incomplete project-owned compilation inventory"),
                    ("policy", "effective assertion policy differs"),
                    ("source", "unexpected/provider/unowned compilation source"),
                    ("failed", "failed/incomplete compiler execution"),
                    ("incomplete", "failed/incomplete compiler execution"),
                    ("backend", "failed/incomplete compiler execution"),
                    ("missing-backend", "missing/ambiguous compiler backend execution"),
                    ("backend-path", "compiler backend differs from qualified toolchain"),
                    ("backend-policy", "backend assertion policy differs"),
                    ("compiler-path", "compiler path differs from qualified identity"),
                    ("argv-path", "compiler path differs from qualified identity"),
                    ("option", "project-owned Ada options differ")):
                data = copy.deepcopy(original)
                executions = data["executions"]
                if mutation == "namespace":
                    data["executions"] = [r for r in executions if not any(a.endswith("programs-oam.ads") for a in r["argv"])]
                elif mutation == "policy":
                    executions[0]["argv"][-1] = f"-gnatec={traced}/oam-ArraySample-Ignore.adc"
                elif mutation == "source":
                    executions[0]["argv"][5] = "/unrelated/helper_demand_consumer.adb"
                elif mutation == "missing-backend":
                    del executions[8]
                elif mutation == "backend-path":
                    executions[8]["executable"] = "/unrelated/gnat1"
                elif mutation == "backend-policy":
                    executions[8]["argv"][-1] = f"-gnatec={traced}/oam-ArraySample-Ignore.adc"
                elif mutation == "compiler-path":
                    executions[0]["executable"] = "/unrelated/gcc"
                elif mutation == "argv-path":
                    executions[0]["argv"][0] = "/unrelated/gcc"
                elif mutation == "option":
                    executions[0]["argv"].remove("-gnato")
                else:
                    executions[8 if mutation == "backend" else 0]["exit"] = 1 if mutation in ("failed", "backend") else None
                path.write_text(json.dumps(data))
                with self.subTest(mutation=mutation), self.assertRaisesRegex(RuntimeError, diagnostic):
                    compare.compare(baseline, pilot, traced)
            path.write_text(json.dumps(original))
            self.assertTrue(compare.compare(baseline, pilot, traced)["actual_compiler_coverage_verified"])

    def test_complete_campaign_is_independent_of_host_gcc(self):
        with tempfile.TemporaryDirectory(dir=Path.home()) as directory:
            roots = [Path(directory) / n for n in ("baseline", "pilot", "traced")]
            for root in roots:
                fixture(root)
            is_file, read_bytes = Path.is_file, Path.read_bytes
            for host_gcc in (None, b"unrelated GCC 12", b"unrelated GCC 15"):
                def host_is_file(path):
                    return host_gcc is not None if path == Path("/usr/bin/gcc") else is_file(path)
                def host_read_bytes(path):
                    return host_gcc if path == Path("/usr/bin/gcc") else read_bytes(path)
                with self.subTest(host_gcc=host_gcc), \
                        patch.object(Path, "is_file", host_is_file), \
                        patch.object(Path, "read_bytes", host_read_bytes):
                    result = compare.compare(*roots)
                    self.assertEqual(result["positive_cells"], 30)
                    self.assertTrue(result["actual_compiler_coverage_verified"])

    def test_qualified_compiler_identity_mutations(self):
        with tempfile.TemporaryDirectory(dir=Path.home()) as directory:
            root = Path(directory) / "traced"
            fixture(root)
            label = "oam-ArraySample"
            path = root / f"{label}-provider-identity.log"
            original = path.read_text()
            for mutation in (original.replace(COMPILER_HASH, "0" * 64),
                             original.replace(f"GNAT {COMPILER_VERSION}", "GNAT 14.2.0"),
                             original.replace(COMPILER_HASH, "a23ecab8ff08f09ad8c80602c2c5df7f49e09c25905cb8975902e101bf72635f")):
                path.write_text(mutation)
                with self.subTest(identity=mutation), self.assertRaisesRegex(RuntimeError, "compiler hash/version differs from qualified seed"):
                    evidence.validate_cell(root, label, "Check")
            path.write_text(original)
            result_path = root / "result.json"
            result = json.loads(result_path.read_text())
            for version in ("14.2.0", "99.0.0"):
                result["records"][0]["compiler"] = version
                result_path.write_text(json.dumps(result))
                with self.subTest(version=version), self.assertRaisesRegex(RuntimeError, "compiler hash/version differs from qualified seed"):
                    evidence.validate_cell(root, label, "Check")

    def test_changed_actual_compiler_binary_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=Path.home()) as directory:
            root = Path(directory) / "traced"
            fixture(root)
            label = "oam-ArraySample"
            compiler, identity = evidence.compiler_identity(root, label)
            self.assertEqual(identity, COMPILER_HASH)
            executable = Path(compiler)
            self.assertFalse(executable.exists())
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"#!/bin/sh\nexit 0\n")
            executable.chmod(0o755)
            self.assertNotEqual(digest(executable), identity)
            with self.assertRaisesRegex(RuntimeError, "compiler binary identity changed"):
                evidence.validate_cell(root, label, "Check")

    def test_source_provenance_is_required(self):
        with tempfile.TemporaryDirectory(dir=Path.home()) as directory:
            root = Path(directory) / "traced"
            fixture(root)
            path = root / "sources.sha256"
            original = path.read_text()
            for mutation in ("", original.replace(original.splitlines()[0][:64], "0" * 64, 1)):
                path.write_text(mutation)
                with self.subTest(manifest=mutation), self.assertRaisesRegex(RuntimeError, "source identity mismatch"):
                    evidence.validate_cell(root, "oam-ArraySample", "Check")


if __name__ == "__main__":
    unittest.main()