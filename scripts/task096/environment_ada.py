"""Independent Ada controls over a frozen copy of the document consumer."""
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
    parser.add_argument("--document", type=Path, required=True)
    parser.add_argument("--profile", choices=["oam", "qualified"], required=True)
    args = parser.parse_args()
    scratch, document = args.scratch.resolve(), args.document.resolve()
    if not (document / "result.json").exists():
        raise RuntimeError("document worker must have completed before copying sources")
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    shutil.copytree(document / "consumer", consumer)
    shutil.copytree(document / "support", scratch / "support")
    source = (ROOT / "tests/ada-float64-probe/environment_probe.adb").read_text()
    if args.profile == "qualified":
        source = source.replace("Programs.Oam", "Urn.Task096")
    (consumer / "environment_probe.adb").write_text(source)
    for name in ["rounding_harness.c", "environment_harness.c"]:
        shutil.copyfile(ROOT / "tests/ada-float32-probe" / name, consumer / name)
    project = (consumer / "consumer.gpr").read_text().replace('"document_probe.adb"', '"environment_probe.adb"')
    (consumer / "consumer.gpr").write_text(project)
    environment = scratch / "environment.sh"
    if (document / "provider").exists():
        shutil.copytree(document / "provider", scratch / "provider")
        environment.write_text((document / "environment.sh").read_text().replace(
            str(document / "provider"), str(scratch / "provider")))
    else:
        raise RuntimeError("document consumer must have isolated provider builds")
    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        shell = ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)]
        (scratch / f"{policy}-build.command.json").write_text(json.dumps(shell) + "\n")
        with (scratch / f"{policy}-build.stdout").open("wb") as out, (scratch / f"{policy}-build.stderr").open("wb") as err:
            process = subprocess.run(shell, cwd=consumer, stdout=out, stderr=err)
        (scratch / f"{policy}-build.exit").write_text(str(process.returncode) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed {policy} build")
        for x87 in range(4):
            for sse in range(4):
                for flushing in range(4):
                    label = f"{policy}-{x87}-{sse}-{flushing}"
                    command = [str(consumer / "bin/environment_probe"), str(x87), str(sse), str(flushing)]
                    (scratch / f"{label}.command.json").write_text(json.dumps(command) + "\n")
                    with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
                        process = subprocess.run(command, cwd=consumer, stdout=out, stderr=err)
                    (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
                    if process.returncode:
                        raise RuntimeError(f"failed {label}")
                    supported = (x87, sse, flushing) == (0, 0, 0)
                    expected = (f"TASK096_ADA_ENVIRONMENT_COMPLETE x87= {x87} mxcsr= {sse} flushing= {flushing} "
                                f"checks= {17 if supported else 19} rejections= {0 if supported else 5}\n")
                    if (scratch / f"{label}.stdout").read_text() != expected:
                        raise RuntimeError(f"incomplete {label}")
                    cells.append({"policy": policy, "x87": x87, "mxcsr": sse, "flushing": flushing})
        print("COMPLETE", policy, "pairs=64", flush=True)
    paths = [p for p in consumer.iterdir() if p.is_file()]
    paths += [Path(__file__), ROOT / "tests/ada-float64-probe/environment_probe.adb"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "profile": args.profile, "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK096_ADA_ENVIRONMENT_MATRIX_COMPLETE executions=192", flush=True)


if __name__ == "__main__":
    main()