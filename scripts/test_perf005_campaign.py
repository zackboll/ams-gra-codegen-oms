#!/usr/bin/env python3
"""Complete passing synthetic campaigns plus compiler-evidence mutations."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from test_perf005_equivalence import compare
from perf005_provider import digest, manifest


def fixture(root):
    root.mkdir()
    records, provenance, hashes = [], [], []
    compiler = "/usr/bin/gcc"
    compiler_hash = "a23ecab8ff08f09ad8c80602c2c5df7f49e09c25905cb8975902e101bf72635f"
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
            (root / f"{label}-provider-identity.log").write_text(f"{compiler}\nGNAT 14.2.0\n{compiler_hash}  {compiler}\n")
            for suffix in ("portable-reject", "portable-generate-reject"):
                (root / f"{label}-{suffix}.log").write_text("expected rejection")
                (root / f"{label}-{suffix}.profile.json").write_text('{"exit":1}')
            for policy in compare.POLICIES:
                cell = f"{label}-{policy}"
                records.append(dict(profile=profile, name=name, policy=policy, compiler="14.2.0"))
                (root / f"{cell}-alire.lock").write_text("frozen lock")
                (root / f"{cell}-gnat.log").write_text("GNAT 14.2.0")
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
                    backend = "/usr/libexec/gcc/x86_64-linux-gnu/14/" + ("cc1" if p.suffix == ".c" else "gnat1")
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
            for mutation in ("namespace", "policy", "source", "failed", "incomplete", "backend"):
                data = copy.deepcopy(original)
                executions = data["executions"]
                if mutation == "namespace":
                    data["executions"] = [r for r in executions if not any(a.endswith("programs-oam.ads") for a in r["argv"])]
                elif mutation == "policy":
                    executions[0]["argv"][-1] = f"-gnatec={traced}/oam-ArraySample-Ignore.adc"
                elif mutation == "source":
                    executions[0]["argv"][5] = "/unrelated/helper_demand_consumer.adb"
                else:
                    executions[8 if mutation == "backend" else 0]["exit"] = 1 if mutation in ("failed", "backend") else None
                path.write_text(json.dumps(data))
                with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                    compare.compare(baseline, pilot, traced)
            path.write_text(json.dumps(original))
            self.assertTrue(compare.compare(baseline, pilot, traced)["actual_compiler_coverage_verified"])


if __name__ == "__main__":
    unittest.main()