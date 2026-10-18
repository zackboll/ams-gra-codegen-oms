"""Generate unchanged production models and execute a model-only consumer."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PROBES = ROOT / "tests/ada-float64-probe"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--profile", choices=["oam", "qualified"], required=True)
    args = parser.parse_args()
    scratch, setup = args.scratch.resolve(), args.setup.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    schema = PROBES / "model.xsd"
    source = (PROBES / "model_consumer.adb").read_text()
    if args.profile == "qualified":
        # Change only declaration namespace, retaining the OAM version annotation.
        text = schema.read_text().replace('xmlns:o=', 'xmlns:version="https://www.vdl.afrl.af.mil/programs/oam" xmlns:o=')
        text = text.replace('xmlns:o="https://www.vdl.afrl.af.mil/programs/oam"', 'xmlns:o="urn:task096"')
        text = text.replace('targetNamespace="https://www.vdl.afrl.af.mil/programs/oam"', 'targetNamespace="urn:task096"')
        text = text.replace('o:version=', 'version:version=')
        schema = scratch / "qualified.xsd"
        schema.write_text(text)
        source = source.replace("Programs.Oam", "Urn.Task096")
    env = os.environ.copy()
    env.update(HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
    alr = ["alr", "-s", str(setup / "settings"), "-n"]
    consumer = scratch / "consumer"
    sequence = 0

    def run(command, cwd, label):
        nonlocal sequence
        sequence += 1
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=cwd, env=env, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        if result.returncode:
            raise RuntimeError(f"failed {label}")
        return (scratch / f"{label}.stdout").read_text()

    run([str(args.generator.resolve()), "service-generate", "--schema", str(schema),
         "--contract", str(PROBES / "model.yaml"), "--language", "ada", "--world", "closed-schema",
         "--output", str(consumer)], ROOT, "generation")
    if any("codec" in p.name.lower() for p in consumer.iterdir()):
        raise RuntimeError("unexpected codec artifact")
    (consumer / "model_consumer.adb").write_text(source)
    (consumer / "consumer.gpr").write_text(
        'project Consumer is\nfor Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("model_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
        'end Compiler; end Consumer;\n')
    # Obtain the exact provisioned tool environment, not a developer toolchain.
    environment = run(alr + ["printenv"], setup / "consumer", "printenv")
    environment_file = scratch / "environment.sh"
    environment_file.write_text(environment)
    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-f", "-p"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        import shlex
        run(["sh", "-ec", f". {shlex.quote(str(environment_file))}; exec " + shlex.join(command)], consumer, f"{policy}-build")
        output = run([str(consumer / "bin/model_consumer")], consumer, f"{policy}-run")
        if output != "TASK096_MODEL_ONLY_COMPLETE checks= 14 rejections= 3\n":
            raise RuntimeError("incomplete model checkpoint")
        cells.append({"policy": policy, "output": output.strip()})
    files = [p for p in consumer.iterdir() if p.is_file()]
    files += [args.generator.resolve(), schema, PROBES / "model.yaml", Path(__file__)]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}, indent=2) + "\n")
    (scratch / "result.json").write_text(json.dumps({"model_only": True, "profile": args.profile, "cells": cells}, indent=2) + "\n")
    print("TASK096_MODEL_CHECKPOINT_COMPLETE", args.profile, flush=True)


if __name__ == "__main__":
    main()