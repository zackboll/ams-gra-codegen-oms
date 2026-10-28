#!/usr/bin/env python3
"""Compile real generated Ada facade/codec consumers; no network or UCI fetch."""
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
OAM = "https://www.vdl.afrl.af.mil/programs/oam"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True, type=Path)
    parser.add_argument("--setup", required=True, type=Path)
    parser.add_argument("--compiler", required=True, choices=["14", "16"])
    parser.add_argument("--generator", required=True, type=Path)
    args = parser.parse_args()
    inputs = [Path(__file__), ROOT / "tests/ada-lacal-runtime/runtime_consumer.adb",
              ROOT / "tests/fixtures/service-generate/lacal-runtime.xsd",
              ROOT / "tests/fixtures/service-generate/lacal-runtime.yaml"]
    inputs += [p for p in (ROOT / "ada").rglob("*") if p.is_file() and
               p.suffix in [".ads", ".adb", ".gpr", ".c", ".h"]]
    input_hashes = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
    scratch, setup = args.scratch.resolve(), args.setup.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, HOME=str(setup / "home"),
               XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"),
               XDG_DATA_HOME=str(setup / "home/.local/share"))
    steps = []

    def run(label, command, cwd=ROOT):
        argv = list(map(str, command))
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": argv, "cwd": str(cwd)}) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(argv, cwd=cwd, env=env, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(f"{result.returncode}\n")
        steps.append({"label": label, "exit": result.returncode})
        if result.returncode:
            raise RuntimeError(f"{label} failed: {scratch / (label + '.stderr')}")
        return (scratch / f"{label}.stdout").read_text()

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
    run("platform", shell(["sh", "-ec", "getconf GNU_LIBC_VERSION; uname -m; gprbuild --version; "
                              "sha256sum $(command -v gnat) $(command -v gcc) $(command -v gprbuild)"]))
    # Provisioning uses a private HOME. Container checkout ownership therefore
    # needs its own exact-directory trust, not checkout's temporary HOME config.
    git = ["git", "-c", f"safe.directory={ROOT}"]
    run("source", [*git, "rev-parse", "HEAD", "HEAD^{tree}"])
    run("source-status", [*git, "status", "--porcelain"])
    run("dependency-lock", ["cat", setup / "consumer/alire/alire.lock"])
    shutil.copytree(ROOT / "ada", scratch / "ada", ignore=ignore)
    # Freeze inputs once, not midway through namespace/profile cells. An editor
    # changing the consumer during a campaign must not create mixed evidence.
    template = (ROOT / "tests/ada-lacal-runtime/runtime_consumer.adb").read_text()
    schema_template = (ROOT / "tests/fixtures/service-generate/lacal-runtime.xsd").read_text()
    cells = []
    for namespace in ["oam", "qualified"]:
        uri = OAM if namespace == "oam" else "urn:task099"
        package = "Programs.Oam" if namespace == "oam" else "Urn.Task099"
        for profile in ["portable", "lossless"]:
            label = f"{namespace}-{profile}"
            schema = schema_template
            if namespace == "qualified":
                schema = schema.replace(f'xmlns:uci="{OAM}"', f'xmlns:oam="{OAM}" xmlns:uci="{uri}"')
                schema = schema.replace(f'targetNamespace="{OAM}"', f'targetNamespace="{uri}"')
                schema = schema.replace("uci:version=", "oam:version=")
            if profile == "lossless":
                schema = schema.replace('<xs:element name="Count"', '<xs:element name="Sample" type="xs:double"/>\n  <xs:element name="Count"')
            schema_path = scratch / f"{label}.xsd"
            schema_path.write_text(schema)
            common = ["--schema", schema_path, "--contract", ROOT / "tests/fixtures/service-generate/lacal-runtime.yaml",
                      "--language", "ada", "--world", "closed-schema"]
            generated, model = scratch / f"{label}-generated", scratch / f"{label}-model"
            codec = ["--with-codec"]
            if profile == "lossless":
                codec += ["--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1"]
            run(f"{label}-model", [args.generator.resolve(), "service-generate", *common, "--output", model])
            run(f"{label}-generate", [args.generator.resolve(), "service-generate", *common, *codec, "--output", generated])
            for path in model.iterdir():
                if path.read_bytes() != (generated / path.name).read_bytes():
                    raise RuntimeError("codec generation changed model/API")
            source = template.replace("TASK099_PACKAGE", package)
            if profile == "lossless":
                source = source.replace("with Ada.Exceptions;", "with Ada.Exceptions;\nwith Interfaces;")
                source = source.replace("   Count : Natural := 0;", "   Count : Natural := 0;\n   Token : Text := To_Unbounded_String (\"0\");\n   Expected : Interfaces.IEEE_Float_64 := 0.0;\n   use type Interfaces.IEEE_Float_64;")
                source = source.replace('Key ("At") & """:""" & Clock', 'Key ("Sample") & """:" & To_String (Token) & ",""" & Key ("At") & """:""" & Clock')
                source = source.replace("      Calls_A := Calls_A + 1;", "      if Message.Sample /= Expected then raise Program_Error with \"Float64 value changed\"; end if;\n      Calls_A := Calls_A + 1;")
                extra = '''
   declare
      type Tokens is array (Positive range <>) of Text;
      Inputs : constant Tokens :=
        (To_Unbounded_String ("-0"), To_Unbounded_String ("4.9406564584124654e-324"),
         To_Unbounded_String ("1.7976931348623157e308"),
         To_Unbounded_String ("1.00000000000000011102230246251565404236316680908203126"));
      type Values is array (Positive range <>) of Interfaces.IEEE_Float_64;
      Wants : constant Values := (-0.0, 2.0 ** (-1074),
         (2.0 - 2.0 ** (-52)) * 2.0 ** 1023, 1.0 + 2.0 ** (-52));
   begin
      for I in Inputs'Range loop
         Token := Inputs (I);
         Expected := Wants (I);
         N := Calls_A;
         Receive ("MSG 1 " & Env ("MessageA", Body_Text));
         Check (Calls_A = N + 1, "lossless typed Float64" & I'Image);
         declare Roundtrip : constant Model.SharedPayload :=
           Service_Codec.Decode_SharedPayload (To_String (Last));
         begin
            Check (Roundtrip.Sample = Expected, "binary64 codec round trip" & I'Image);
            if I = 1 then
               Check (Ada.Strings.Fixed.Index (To_String (Last), "-0") > 0, "signed zero preserved");
            end if;
         end;
      end loop;
      Token := To_Unbounded_String ("0");
      Expected := 0.0;
      Calls_A := 3;
   end;
'''
                unsubscribe = "   N := M.Sent_Count (Wire);\n   A.Unsubscribe (BA, SID_A);"
                source = source.replace(unsubscribe, extra + unsubscribe)
            for optimization in ["0", "2"]:
                for policy in ["Normal", "Check", "Ignore"]:
                    cell = f"{label}-O{optimization}-{policy}"
                    build = scratch / cell
                    shutil.copytree(generated, build)
                    (build / "runtime_consumer.adb").write_text(source)
                    extra_project = (f'with "{scratch}/ada/oms-json-support/ams_gra_oms_json_lossless";\n'
                                     if profile == "lossless" else "")
                    (build / "consumer.gpr").write_text(
                        f'with "{scratch}/ada/oms-lacal-runtime/oms_lacal_runtime";\n' + extra_project +
                        'project Consumer is\nfor Source_Dirs use ("."); for Object_Dir use "obj";\n'
                        'for Exec_Dir use "bin"; for Main use ("runtime_consumer.adb");\n'
                        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato");\n'
                        'end Compiler; end Consumer;\n')
                    adc = build / "policy.adc"
                    adc.write_text(f"pragma Assertion_Policy ({policy});\n" if policy != "Normal" else "")
                    # -cargs applies to core, support and consumer, not just main.
                    run(f"{cell}-build", shell(["gprbuild", "-P", "consumer.gpr", "-p", "-j4",
                        "-cargs:Ada", "-gnato", f"-O{optimization}", f"-gnatec={adc}"]), build)
                    output = run(f"{cell}-execute", shell([build / "bin/runtime_consumer"]), build)
                    checks = 62 if profile == "lossless" else 53
                    if not output.endswith(f"TASK099_CONSUMER_COMPLETE checks= {checks}\n") or output.count("PASS ") != checks:
                        raise RuntimeError(f"incomplete execution {cell}")
                    cells.append({"namespace": namespace, "profile": profile, "policy": policy,
                                  "optimization": optimization, "checks": checks})
                    print("TASK099_CELL_COMPLETE", version, cell, checks, flush=True)
    paths = [Path(__file__), args.generator.resolve(), ROOT / "tests/ada-lacal-runtime/runtime_consumer.adb",
             ROOT / "tests/fixtures/service-generate/lacal-runtime.xsd",
             ROOT / "tests/fixtures/service-generate/lacal-runtime.yaml"]
    paths += [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".gpr", ".xsd", ".lock"]
              and "provider" not in p.parts]
    paths += [p for p in (scratch / "provider").rglob("*") if p.is_file() and
              p.name in ["alire.toml", "gnatcoll-json.ads", "gnatcoll-json.adb"]]
    paths += [setup / "consumer/alire/alire.lock", setup / "identities.json"]
    if any(hashlib.sha256(Path(p).read_bytes()).hexdigest() != digest for p, digest in input_hashes.items()):
        raise RuntimeError("source changed during qualification; discard mixed campaign")
    (scratch / "input-sources.json").write_text(json.dumps(input_hashes, indent=2) + "\n")
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    (scratch / "result.json").write_text(json.dumps({"completed": True, "production_generated": True,
        "compiler": version, "cells": cells, "steps": steps}, indent=2) + "\n")
    print("TASK099_QUALIFICATION_COMPLETE", version, "cells=24", flush=True)


if __name__ == "__main__":
    main()