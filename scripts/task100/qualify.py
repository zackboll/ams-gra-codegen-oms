#!/usr/bin/env python3
"""Pinned isolated WebSocket compiler/policy/boundary execution campaign."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
PINS = {
    "aht": ("https://github.com/ovenpasta/aht.git", "664d5ddeb568626b2f9ff66054c7091894b3b8d4"),
    "websocket-ada": ("https://github.com/kokhlo/websocket-ada.git", "765d9cd612838eb00645037977c8cc9af7171c3e"),
}
OAM = "https://www.vdl.afrl.af.mil/programs/oam"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--sleet", type=Path)
    args = parser.parse_args()
    scratch, setup = args.scratch.resolve(), args.setup.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, HOME=str(setup / "home"), PYTHONDONTWRITEBYTECODE="1",
               XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"),
               XDG_DATA_HOME=str(setup / "home/.local/share"))
    steps = []
    inputs = [p for directory in [ROOT / "ada", ROOT / "scripts/task100", ROOT / "tests/ada-websocket"]
              for p in directory.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".gpr", ".py", ".c", ".h"]]
    inputs += [ROOT / "tests/fixtures/service-generate/lacal-websocket.xsd",
               ROOT / "tests/fixtures/service-generate/lacal-runtime.yaml", args.generator.resolve()]
    hashes = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}

    def run(label, command, cwd=ROOT):
        argv = list(map(str, command))
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": argv, "cwd": str(cwd)}) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(argv, cwd=cwd, env=env, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        steps.append({"label": label, "exit": result.returncode})
        if result.returncode:
            raise RuntimeError(f"{label} failed: {scratch / (label + '.stdout')}, .stderr")
        return (scratch / f"{label}.stdout").read_text()

    run("source", ["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD", "HEAD^{tree}"])
    run("source-status", ["git", "-c", f"safe.directory={ROOT}", "status", "--porcelain"])
    printed = run("printenv", ["alr", "-s", setup / "settings", "-n", "printenv"], setup / "consumer")
    provider = setup / "consumer/alire/cache/dependencies"
    ignore = shutil.ignore_patterns("obj", "obj-*", "lib", "bin", ".git", "*.o", "*.ali", "__pycache__")
    shutil.copytree(provider, scratch / "provider", ignore=ignore)
    environment = scratch / "environment.sh"
    environment.write_text(printed.replace(str(provider), str(scratch / "provider")))

    def shell(command):
        return ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(map(str, command))]

    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    assert run("compiler", shell(["gnat", "--version"])).startswith(f"GNAT {version}\n")
    run("platform", shell(["sh", "-ec", "test $(uname -m) = x86_64; test \"$(getconf GNU_LIBC_VERSION)\" = 'glibc 2.41'; uname -m; getconf GNU_LIBC_VERSION; gprbuild --version; "
                          "sha256sum $(command -v gnat) $(command -v gcc) $(command -v gprbuild)"]))
    for name, (url, revision) in PINS.items():
        dest = scratch / "deps" / name
        run(f"{name}-clone", ["git", "clone", "--quiet", url, dest])
        run(f"{name}-pin", ["git", "-C", dest, "checkout", "--quiet", "--detach", revision])
        assert run(f"{name}-identity", ["git", "-C", dest, "rev-parse", "HEAD"]).strip() == revision
        assert not run(f"{name}-clean", ["git", "-C", dest, "status", "--porcelain"])
    env["OMS_WS_AHT_SOURCE"] = str(scratch / "deps/aht")
    env["OMS_WS_CODEC_SOURCE"] = str(scratch / "deps/websocket-ada")
    shim = scratch / "partial_send.so"
    run("partial-send-shim", ["cc", "-shared", "-fPIC", "-Wall", "-Wextra", "-Werror", "-O2",
        ROOT / "tests/ada-websocket/partial_send.c", "-o", shim])
    cells = []
    for namespace in ["oam", "qualified"]:
        for profile in ["portable", "lossless"]:
            name = f"{namespace}-{profile}"
            schema = (ROOT / "tests/fixtures/service-generate/lacal-websocket.xsd").read_text()
            if namespace == "qualified":
                schema = schema.replace(f'xmlns:uci="{OAM}"', f'xmlns:oam="{OAM}" xmlns:uci="urn:task100"')
                schema = schema.replace(f'targetNamespace="{OAM}"', 'targetNamespace="urn:task100"').replace("uci:version=", "oam:version=")
            if profile == "lossless":
                schema = schema.replace('<xs:element name="Count"', '<xs:element name="Sample" type="xs:double"/>\n  <xs:element name="Count"')
            path = scratch / f"{name}.xsd"
            path.write_text(schema)
            generated = scratch / f"{name}-generated"
            common = [args.generator.resolve(), "service-generate", "--schema", path,
                      "--contract", ROOT / "tests/fixtures/service-generate/lacal-runtime.yaml",
                      "--language", "ada", "--world", "closed-schema"]
            run(f"{name}-model", [*common, "--output", scratch / f"{name}-model"])
            runtime = ["--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1"] if profile == "lossless" else []
            run(f"{name}-generate", [*common, "--with-codec", *runtime, "--output", generated])
            for model in (scratch / f"{name}-model").iterdir():
                assert model.read_bytes() == (generated / model.name).read_bytes(), "generation changed model/API"
            source = (ROOT / "tests/ada-websocket/generated_consumer.adb").read_text().replace(
                "TASK100_PACKAGE", "Programs.Oam" if namespace == "oam" else "Urn.Task100")
            if profile == "lossless":
                source = source.replace("with Ada.Command_Line;", "with Ada.Command_Line;\nwith Interfaces;")
                source = source.replace("   procedure Validate (Message", "   use type Interfaces.IEEE_Float_64;\n   procedure Validate (Message")
                source = source.replace('      if Message.Count /= 42', '      if Message.Sample /= 1.0 + 2.0 ** (-52) then raise Program_Error with "Float64 changed"; end if;\n      if Message.Count /= 42')
                source = source.replace('("{""" & Key ("Count")', '("{""" & Key ("Sample") & """:1.0000000000000002,""" & Key ("Count")')
            for policy in ["Normal", "Check", "Ignore"]:
                for optimization in ["0", "2"]:
                    cell = f"{name}-{policy}-O{optimization}"
                    build = scratch / cell
                    shutil.copytree(ROOT / "ada", build / "ada", ignore=ignore)
                    shutil.copytree(generated, build / "generated")
                    shutil.copytree(ROOT / "tests/ada-websocket", build / "test")
                    (build / "test/generated_consumer.adb").write_text(source)
                    mains = '"transport_probe.adb", "generated_consumer.adb"'
                    if profile == "lossless":
                        float_source = (build / "test/float_probe.adb").read_text().replace(
                            "TASK100_PACKAGE", "Programs.Oam" if namespace == "oam" else "Urn.Task100")
                        (build / "test/float_probe.adb").write_text(float_source)
                        mains += ', "float_probe.adb"'
                    else:
                        (build / "test/float_probe.adb").unlink()
                    extra = 'with "ada/oms-json-support/ams_gra_oms_json_lossless";\n' if profile == "lossless" else ""
                    (build / "consumer.gpr").write_text(
                        'with "ada/oms-lacal-runtime/oms_lacal_websocket";\n' + extra +
                        'project Consumer is\nfor Source_Dirs use ("generated", "test");\n'
                        'for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
                        f'for Main use ({mains});\n'
                        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato");\n'
                        'end Compiler; end Consumer;\n')
                    adc = build / "policy.adc"
                    adc.write_text("" if policy == "Normal" else f"pragma Assertion_Policy ({policy});\n")
                    run(f"{cell}-build", shell(["gprbuild", "-p", "-j4", "-P", "consumer.gpr",
                        "-cargs:Ada", "-gnato", f"-O{optimization}", f"-gnatec={adc}"]), build)
                    float_args = ["--float-probe", build / "bin/float_probe", "--namespace", namespace] if profile == "lossless" else []
                    output = run(f"{cell}-local", ["python3", ROOT / "scripts/task100/local_peer.py",
                        "--probe", build / "bin/transport_probe", "--consumer", build / "bin/generated_consumer",
                        "--evidence", build / "local", "--partial-send-shim", shim, *float_args])
                    expected_cases = 44 if profile == "lossless" else 43
                    assert f"TASK100_LOCAL_SUITE_COMPLETE {expected_cases}\n" in output
                    local_result = json.loads((build / "local/result.json").read_text())
                    assert local_result["completed"] and len(local_result["cases"]) == expected_cases
                    if args.sleet and namespace == "oam":
                        if profile == "portable":
                            output = run(f"{cell}-sleet", ["python3", ROOT / "scripts/task100/real_sleet.py",
                                "--checkout", args.sleet.resolve(), "--consumer", build / "bin/generated_consumer",
                                "--evidence", build / "sleet"])
                            assert "TASK100_REAL_PINNED_SLEET_COMPLETE" in output
                    cells.append({"namespace": namespace, "profile": profile, "policy": policy, "optimization": optimization})
                    print("TASK100_CELL_COMPLETE", version, cell, flush=True)
    assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest() == digest for p, digest in hashes.items()), "inputs changed during campaign"
    (scratch / "input-sources.json").write_text(json.dumps(hashes, indent=2) + "\n")
    sources = {str(p.relative_to(scratch)): hashlib.sha256(p.read_bytes()).hexdigest()
               for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".gpr", ".lock", ".toml", ".xsd", ".adc"]}
    (scratch / "sources.json").write_text(json.dumps(sources, indent=2) + "\n")
    (scratch / "result.json").write_text(json.dumps({"completed": True, "compiler": version,
        "cells": cells, "steps": steps, "real_sleet": bool(args.sleet)}, indent=2) + "\n")
    print("TASK100_QUALIFICATION_COMPLETE", version, len(cells))


if __name__ == "__main__":
    main()