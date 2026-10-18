"""Verify failure ownership/categories, not a passing resource qualification.

Independent candidate-provider builds under both compilers/policies; NULL
allocation interface, exact reached counts, internal-error controls, recovery.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--policies", nargs="+", choices=["Normal", "Check", "Ignore"], default=["Normal", "Check", "Ignore"])
    parser.add_argument("--expect-pristine-unescape-error", action="store_true")
    parser.add_argument("--provider-only", action="store_true")
    args = parser.parse_args()
    scratch, candidate = args.scratch.resolve(), args.candidate.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    ignore = shutil.ignore_patterns("obj", "lib", "bin", "*.o", "*.ali", "*.a", "*.so", "*.lexch", "*.bexch")
    for name in ["consumer", "provider", "support"]:
        shutil.copytree(candidate / name, scratch / name, ignore=ignore)
    for source in consumer.glob("b__*"):
        source.unlink()
    shutil.copy2(ROOT / "tests/ada-float64-codec/c1_malloc_fault.c", consumer)
    for extension in ["ads", "adb"]:
        shutil.copy2(ROOT / f"tests/ada-float64-codec/c1_null_allocation_faults.{extension}", consumer / f"allocation_faults.{extension}")
    mains = ["c1_provider_probe", "c1_generated_probe", "c1_model_probe", "c1_exception_controls"]
    for main in mains:
        source = (ROOT / "tests/ada-float64-codec" / (main + ".adb")).read_text()
        if main != "c1_exception_controls":
            # Reporting runs strictly AFTER Disarm and successful recovery.
            unit = "Ada.Text_IO.Put_Line" if main != "c1_generated_probe" else "Put_Line"
            marker = {"c1_provider_probe": "TASK097_C1_PROVIDER_RECOVERY_COMPLETE",
                      "c1_generated_probe": "TASK097_C1_GENERATED_RECOVERY_COMPLETE",
                      "c1_model_probe": "TASK097_C1_MODEL_RECOVERY_COMPLETE"}[main]
            source = source.replace(f'{unit} ("{marker}");',
                f'{unit} ("{marker}");\n   {unit} ("TASK097_C1_ALLOCATOR_OBSERVATION attempts=" & Allocation_Faults.Attempts\'Image &\n     " failures=" & Allocation_Faults.Failures\'Image & " size=" & Allocation_Faults.Failed_Size\'Image);')
        (consumer / (main + ".adb")).write_text(source)
    shutil.copy2(candidate / "valid.json", scratch)
    environment = scratch / "environment.sh"
    environment.write_text((candidate / "environment.sh").read_text().replace(str(candidate / "provider"), str(scratch / "provider")))
    (consumer / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
        'for Languages use ("Ada", "C"); for Source_Dirs use (".");\n'
        'for Object_Dir use "obj"; for Exec_Dir use "bin";\nfor Main use (' +
        ', '.join('"' + n + '.adb"' for n in mains) + ');\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0", "-g");\n'
        'for Default_Switches ("C") use ("-O0", "-g", "-Wall", "-Wextra", "-Werror"); end Compiler;\n'
        'package Linker is for Default_Switches ("Ada") use ("-Wl,--wrap=__gnat_malloc"); end Linker;\nend Consumer;\n')
    rows = []

    def run(label, command):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(consumer)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=consumer, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        if result.returncode:
            raise RuntimeError(f"failed {label}")
        output = (scratch / f"{label}.stdout").read_text()
        rows.append({"case": label, "exit": result.returncode, "output": output})
        return output

    for policy in args.policies:
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        run(policy + "-build", ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)])
        for main, operations in [("c1_provider_probe", [("provider", ["0"]), ("success", ["1000000"])]),
            ("c1_model_probe", [("model", ["0"]), ("success", ["1000000"])]),
            ("c1_generated_probe", [("decode33", ["decode", "33", str(scratch / "valid.json")]),
                                    ("encode4", ["encode", "4", str(scratch / "valid.json")]),
                                    ("decode-success", ["decode", "1000000", str(scratch / "valid.json")]),
                                    ("encode-success", ["encode", "1000000", str(scratch / "valid.json")])])]:
            if args.provider_only and main != "c1_provider_probe":
                continue
            for name, arguments in operations:
                output = run(f"{policy}-{main}-{name}", [str(consumer / "bin" / main), *arguments])
                if "RECOVERY_COMPLETE" not in output or "TASK097_C1_ALLOCATOR_OBSERVATION" not in output:
                    raise RuntimeError("missing recovery/count")
                unescape_category = "PROGRAM_ERROR" if args.expect_pristine_unescape_error else "STORAGE_ERROR"
                if name == "provider" and (f"EXCEPTION {unescape_category}" not in output or "failures= 1 size= 28" not in output):
                    raise RuntimeError("localized unescape category/size mismatch")
                if name == "decode33" and ("EXCEPTION STORAGE_ERROR" not in output or "failures= 1" not in output):
                    raise RuntimeError("generated ordinal did not reach a recoverable allocation")
                if "success" in name and ("_SUCCESS" not in output or "failures= 0" not in output):
                    raise RuntimeError("bad no-fault control")
        for mode in ["program", "lookalike", "controlled", "unrelated-resource", "validation"]:
            output = run(f"{policy}-control-{mode}", [str(consumer / "bin/c1_exception_controls"), mode])
            expected = {"program": "PROGRAM_ERROR", "lookalike": "PROGRAM_ERROR", "controlled": "PROGRAM_ERROR",
                        "unrelated-resource": "STORAGE_ERROR", "validation": "CONSTRAINT_ERROR"}[mode]
            if not output.startswith(f"TASK097_C1_CONTROL {mode} {expected}\n"):
                raise RuntimeError("internal/resource/validation category hidden")
        print("COMPLETE DIAGNOSTIC", policy, flush=True)
    files = [p for parent in [consumer, scratch / "provider", scratch / "support"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".adb", ".ads", ".c", ".gpr", ".h"]]
    files += [Path(__file__), scratch / "valid.json"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed_diagnostic": True, "production_qualified": False, "rows": rows}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_C1_SCOPE_DIAGNOSTICS_COMPLETE", flush=True)


if __name__ == "__main__":
    main()