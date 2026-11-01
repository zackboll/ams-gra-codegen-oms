#!/usr/bin/env python3
"""Focused production-generated heterogeneous dispatch consumer; no broker."""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/ada-lacal-dispatch"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True, type=Path)
    parser.add_argument("--setup", required=True, type=Path)
    parser.add_argument("--compiler", required=True, choices=["14", "16"])
    parser.add_argument("--generator", required=True, type=Path)
    args = parser.parse_args()
    scratch, setup = args.scratch.resolve(), args.setup.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, HOME=str(setup / "home"),
               XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"),
               XDG_DATA_HOME=str(setup / "home/.local/share"))

    def run(label, command, cwd=ROOT, negative=None):
        argv = list(map(str, command))
        (scratch / f"{label}.command.json").write_text(json.dumps(dict(argv=argv, cwd=str(cwd))) + "\n")
        result = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True)
        (scratch / f"{label}.stdout").write_text(result.stdout)
        (scratch / f"{label}.stderr").write_text(result.stderr)
        (scratch / f"{label}.exit").write_text(f"{result.returncode}\n")
        if negative:
            if result.returncode == 0 or f'"{negative}" not declared' not in result.stderr:
                raise RuntimeError(f"{label}: missing expected compile-time rejection")
        elif result.returncode:
            raise RuntimeError(f"{label} failed; see {scratch / (label + '.stderr')}")
        return result.stdout

    printed = run("printenv", ["alr", "-s", setup / "settings", "-n", "printenv"], setup / "consumer")
    provider = setup / "consumer/alire/cache/dependencies"
    ignore = shutil.ignore_patterns("obj", "obj-*", "lib", "bin", "*.o", "*.ali")
    shutil.copytree(provider, scratch / "provider", ignore=ignore)
    environment = scratch / "environment.sh"
    environment.write_text(printed.replace(str(provider), str(scratch / "provider")))

    def shell(command):
        return ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(map(str, command))]

    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    if not run("compiler", shell(["gnat", "--version"])).startswith(f"GNAT {version}\n"):
        raise RuntimeError("wrong compiler")
    shutil.copytree(ROOT / "ada", scratch / "ada", ignore=ignore)
    generated = scratch / "generated"
    run("generate", [args.generator.resolve(), "service-generate", "--schema", FIXTURE / "messages.xsd",
                     "--contract", FIXTURE / "service.yaml", "--language", "ada", "--world",
                     "closed-schema", "--with-codec", "--output", generated])
    source = (FIXTURE / "dispatch_consumer.adb").read_text()
    project = (f'with "{scratch}/ada/oms-lacal-runtime/oms_lacal_runtime";\n'
               'project Consumer is\nfor Source_Dirs use ("."); for Object_Dir use "obj";\n'
               'for Exec_Dir use "bin"; for Main use ("dispatch_consumer.adb");\n'
               'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato");\n'
               'end Compiler; end Consumer;\n')
    cells = []
    for optimization in ["0", "2"]:
        for policy in ["Normal", "Check", "Ignore"]:
            label = f"O{optimization}-{policy}"
            build = scratch / label
            shutil.copytree(generated, build)
            (build / "dispatch_consumer.adb").write_text(source)
            (build / "consumer.gpr").write_text(project)
            adc = build / "policy.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n" if policy != "Normal" else "")
            run(f"{label}-build", shell(["gprbuild", "-P", "consumer.gpr", "-p", "-j4", "-cargs:Ada",
                                        "-gnato", f"-O{optimization}", f"-gnatec={adc}"]), build)
            output = run(f"{label}-execute", shell([build / "bin/dispatch_consumer"]), build)
            checks = output.count("PASS ")
            if checks != 64 or not output.endswith("TASK107_DISPATCH_COMPLETE checks= 64\n"):
                raise RuntimeError("incomplete focused consumer")
            cells.append(dict(optimization=optimization, policy=policy, checks=checks))
            print("TASK107_CELL_COMPLETE", version, label, checks, flush=True)
    for label, old, new, missing in [
        ("input-publish", "Service_API.Function_Io.Exchange_Publish.Publisher", "IL.Publisher", "Publisher"),
        ("output-subscribe", "IL.Subscriber", "Service_API.Function_Io.Exchange_Publish.Subscriber", "Subscriber"),
    ]:
        build = scratch / label
        shutil.copytree(generated, build)
        (build / "dispatch_consumer.adb").write_text(source.replace(old, new))
        (build / "consumer.gpr").write_text(project)
        run(label, shell(["gprbuild", "-P", "consumer.gpr", "-p", "-j4"]), build, negative=missing)
    (scratch / "result.json").write_text(json.dumps(dict(completed=True, compiler=version, cells=cells,
        directionality_rejections=2), indent=2) + "\n")
    print("TASK107_QUALIFICATION_COMPLETE", version, "cells=6 directionality_rejections=2", flush=True)


if __name__ == "__main__":
    main()