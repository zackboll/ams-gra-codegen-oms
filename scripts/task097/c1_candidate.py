"""Diagnostic provider candidate, NOT a production dependency qualification.

Explicit pristine hash, source patch, no cache modifications, clean rebuilds.
Each compiler owns an independent provider and consumer tree.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PRISTINE = "e9c95ad6b0899821852a787c586bb772017fabee1a2a852d5edf21f026687e3b"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--pristine", type=Path, required=True)
    args = parser.parse_args()
    scratch, pristine = args.scratch.resolve(), args.pristine.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    records = []

    def run(label, command, cwd, expected=0):
        row = {"label": label, "argv": command, "cwd": str(cwd), "expected_exit": expected}
        (scratch / f"{label}.command.json").write_text(json.dumps(row, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=cwd, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        records.append(row | {"exit": result.returncode})
        if expected is not None and result.returncode != expected:
            raise RuntimeError(f"{label}: exit {result.returncode}")
        print("COMPLETE", label, result.returncode, flush=True)
        return (scratch / f"{label}.stdout").read_text()

    ignore = shutil.ignore_patterns("obj", "lib", "bin", "*.o", "*.ali", "*.a", "*.so", "*.lexch", "*.bexch")
    for name in ["consumer", "support", "provider"]:
        shutil.copytree(pristine / name, scratch / name, ignore=ignore)
    consumer = scratch / "consumer"
    for source in consumer.glob("b__*"):
        source.unlink()
    for name in ["c1_generated_probe.adb", "c1_provider_probe.adb"]:
        shutil.copy2(ROOT / "tests/ada-float64-codec" / name, consumer)
    shutil.copy2(pristine / "valid.json", scratch)
    environment = scratch / "environment.sh"
    environment.write_text((pristine / "environment.sh").read_text().replace(str(pristine / "provider"), str(scratch / "provider")))
    project = (consumer / "consumer.gpr").read_text().replace('"c1_provider_probe.adb")', '"c1_provider_probe.adb", "c1_generated_probe.adb")')
    project = project.replace('for Source_Dirs', 'for Languages use ("Ada", "C"); for Source_Dirs')
    (consumer / "consumer.gpr").write_text(project)
    patches = []
    for provider in sorted((scratch / "provider").glob("gnatcoll*")):
        source = provider / "core/src/gnatcoll-strings_impl.adb"
        if not source.exists():
            continue
        before = hashlib.sha256(source.read_bytes()).hexdigest()
        if before != PRISTINE:
            raise RuntimeError("not pristine pinned provider")
        run("patch-" + provider.name, ["patch", "--batch", "--fuzz=0", "-p1", "-i",
            str(ROOT / "tests/ada-float64-codec/c1_provider-transition.patch")], provider)
        patches.append({"source": str(source), "pristine_sha256": before, "candidate_sha256": hashlib.sha256(source.read_bytes()).hexdigest()})
    shell = lambda command: ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)]
    for phase in ["pristine-allocator", "null-allocator"]:
        if phase == "null-allocator":
            shutil.copy2(ROOT / "tests/ada-float64-codec/c1_malloc_fault.c", consumer)
            allocation = (consumer / "allocation_faults.adb").read_text()
            allocation = allocation.replace('   function Allocate (Size', '''   procedure Fail_Next with Import, Convention => C,
     External_Name => "task097_c1_fail_next_malloc";
   function Allocate (Size''')
            allocation = allocation.replace('            raise Storage_Error with "Task097 injected output/input allocation failure";',
                '            Fail_Next;\n            return Real_Allocate (Size);')
            (consumer / "allocation_faults.adb").write_text(allocation)
        run(phase + "-build", shell(["gprbuild", "-P", "consumer.gpr", "-p", "-f", "-cargs:Ada", "-g"]), consumer)
        for budget in range(3):
            run(f"{phase}-provider-{budget}", [str(consumer / "bin/c1_provider_probe"), str(budget)], consumer)
        for mode, budget in [("decode", "33"), ("encode", "4"), ("decode", "0"), ("encode", "0")]:
            run(f"{phase}-{mode}-{budget}", [str(consumer / "bin/c1_generated_probe"), mode, budget, str(scratch / "valid.json")], consumer)
    (scratch / "provider-candidate.json").write_text(json.dumps(patches, indent=2) + "\n")
    paths = [p for parent in [consumer, scratch / "support", scratch / "provider"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".adb", ".ads", ".c", ".h", ".gpr"]]
    paths += [Path(__file__), ROOT / "tests/ada-float64-codec/c1_provider-transition.patch", scratch / "valid.json"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed_diagnostic_candidate": True, "production_qualified": False, "patches": patches, "commands": records}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_C1_CANDIDATE_DIAGNOSTIC_COMPLETE", flush=True)


if __name__ == "__main__":
    main()