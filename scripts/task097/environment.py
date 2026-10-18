"""All independent x87/MXCSR pairs and DAZ/FTZ states, generated codecs."""
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
    parser.add_argument("--generated", type=Path, required=True)
    args = parser.parse_args()
    scratch, generated = args.scratch.resolve(), args.generated.resolve()
    result = json.loads((generated / "result.json").read_text())
    if not result.get("completed") or not result.get("production_generated"):
        raise RuntimeError("generated matrix not completed")
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    consumer.mkdir()
    for path in (generated / "consumer").glob("*.ad?"):
        if path.name != "generated_corpus.adb":
            shutil.copy2(path, consumer)
    shutil.copytree(generated / "support", scratch / "support")
    shutil.copytree(generated / "provider", scratch / "provider")
    environment = scratch / "environment.sh"
    environment.write_text((generated / "environment.sh").read_text().replace(str(generated / "provider"), str(scratch / "provider")))
    source = (ROOT / "tests/ada-float64-codec/environment.adb").read_text()
    if result["namespace"] == "qualified":
        source = source.replace("Programs.Oam", "Urn.Task097").replace(
            'function Key (Local : String) return String is (Local);',
            'function Key (Local : String) return String is ("{urn:task097}" & Local);')
    (consumer / "environment.adb").write_text(source)
    for name in ["rounding_harness.c", "environment_harness.c"]:
        shutil.copy2(ROOT / "tests/ada-float32-probe" / name, consumer)
    project = (generated / "consumer/consumer.gpr").read_text().replace('"generated_corpus.adb"', '"environment.adb"')
    project = project.replace('for Source_Dirs', 'for Languages use ("Ada", "C");\nfor Source_Dirs')
    (consumer / "consumer.gpr").write_text(project)

    def run(label, command):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(consumer)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            process = subprocess.run(command, cwd=consumer, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed {label}")
        return (scratch / f"{label}.stdout").read_text()

    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        run(f"{policy}-build", ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)])
        for x87 in range(4):
            for sse in range(4):
                for flushing in range(4):
                    label = f"{policy}-{x87}-{sse}-{flushing}"
                    output = run(label, [str(consumer / "bin/environment"), str(x87), str(sse), str(flushing)])
                    rejected = 0 if (x87, sse, flushing) == (0, 0, 0) else 2
                    expected = f"TASK097_GENERATED_ENVIRONMENT_COMPLETE x87= {x87} mxcsr= {sse} flushing= {flushing} rejections= {rejected}\n"
                    if output != expected:
                        raise RuntimeError("incomplete environment cell")
                    cells.append({"policy": policy, "x87": x87, "mxcsr": sse, "flushing": flushing})
        print("COMPLETE", policy, "pairs=64", flush=True)
    paths = [p for parent in [consumer, scratch / "support"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".adb", ".ads", ".c", ".h", ".gpr"]]
    paths += [Path(__file__)]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True, "namespace": result["namespace"], "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_GENERATED_ENVIRONMENT_MATRIX_COMPLETE executions=192", flush=True)


if __name__ == "__main__":
    main()