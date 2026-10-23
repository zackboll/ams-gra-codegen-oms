#!/usr/bin/env python3
"""Task 095 generated-code execution on the explicitly qualified runtime."""
import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import tomllib
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[1]
PROFILE = "linux-x86_64-glibc-2.41"


def checked(command, cwd, log):
    log.with_suffix(log.suffix + ".command.json").write_text(json.dumps({"command": command, "cwd": str(cwd)}, indent=2) + "\n")
    with log.open("wb") as output:
        result = subprocess.run(command, cwd=cwd, stdout=output, stderr=subprocess.STDOUT)
    log.with_suffix(log.suffix + ".exit").write_text(f"{result.returncode}\n")
    if result.returncode:
        raise RuntimeError(f"exit {result.returncode}: {command}; {log}")
    return log.read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    parser.add_argument("--profile", choices=["oam", "qualified"], required=True)
    parser.add_argument("--numeric-corpus", action="store_true")
    parser.add_argument("--environment", action="store_true", help="generated C2 floating-environment controls")
    parser.add_argument("--rounding", action="store_true", help="generated C3 independent x87/MXCSR pairs")
    parser.add_argument("--identifiers", action="store_true", help="generated C2 adversarial type names")
    parser.add_argument("--optimized", action="store_true", help="additional -O2 environment execution")
    args = parser.parse_args()
    build_jobs = int(os.environ.get("OMS_CI_ADA_BUILD_JOBS", "1"))
    if not 1 <= build_jobs <= 4:
        parser.error("OMS_CI_ADA_BUILD_JOBS must be between 1 and 4")
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    oracle = None
    if args.numeric_corpus:
        specification = importlib.util.spec_from_file_location("task095_oracle", ROOT / "tests/ada-float32-probe/oracle.py")
        oracle = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(oracle)
        rows = (ROOT / "tests/ada-float32-probe/conversions.tsv").read_text().splitlines()
        if len(rows) != 15710:
            raise RuntimeError("incomplete independent oracle corpus")
        for row in rows:
            _, expected, token = row.split("\t")
            if oracle.oracle(token) != expected:
                raise RuntimeError(f"independent oracle mismatch: {token}")
        (scratch / "oracle.log").write_text("TASK095_EXACT_ORACLE_COMPLETE rows=15710 failures=0\n")
    if subprocess.check_output(["uname", "-m"]).strip() != b"x86_64":
        raise RuntimeError("unqualified architecture")
    if subprocess.check_output(["getconf", "GNU_LIBC_VERSION"]).strip() != b"glibc 2.41":
        raise RuntimeError("unqualified libc")
    checked(["sh", "-ec", "uname -m; getconf GNU_LIBC_VERSION; dpkg-query -W libc6; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6"], ROOT, scratch / "runtime.log")
    fixtures = ROOT / "tests/fixtures/service-generate"
    schema = fixtures / "codec-ada-float32.xsd"
    golden = fixtures / "task095-populated.json"
    contract = fixtures / "codec-ada-float32.yaml"
    source = (ROOT / "tests/ada-generated-codec/float32_consumer.adb").read_text()
    main_name = "float32_consumer"
    marker = b"TASK095_GENERATED_COMPLETE checks= 96 rejections= 14\n"
    if args.identifiers:
        schema = fixtures / "codec-ada-float32-identifiers.xsd"
        contract = fixtures / "codec-ada-float32-identifiers.yaml"
        golden = fixtures / "task095-identifiers.json"
        main_name = "float32_identifiers"
        source = (ROOT / f"tests/ada-generated-codec/{main_name}.adb").read_text()
        marker = b"TASK095_C2_IDENTIFIERS_COMPLETE checks= 12\n"
    if args.profile == "qualified":
        oam = "https://www.vdl.afrl.af.mil/programs/oam"
        qualified = "urn:task095"
        schema_text = schema.read_text().replace(f'xmlns:uci="{oam}"', f'xmlns:uci="{qualified}"')
        schema_text = schema_text.replace(f'targetNamespace="{oam}"', f'targetNamespace="{qualified}"')
        schema_text = schema_text.replace('elementFormDefault="qualified"', f'xmlns:oam="{oam}" elementFormDefault="qualified"')
        schema_text = schema_text.replace("uci:version=", "oam:version=")
        original_tree = ET.fromstring(schema.read_text())
        qualified_tree = ET.fromstring(schema_text)
        elements = "{http://www.w3.org/2001/XMLSchema}element"
        version_key = f"{{{oam}}}version"
        old_messages = original_tree.findall(elements)
        new_messages = qualified_tree.findall(elements)
        if not old_messages or len(old_messages) != len(new_messages):
            raise RuntimeError("qualified message annotation mismatch")
        for old, new in zip(old_messages, new_messages, strict=True):
            if old.attrib[version_key] != new.attrib[version_key] or [key for key in new.attrib if key.endswith("}version")] != [version_key]:
                raise RuntimeError("non-OAM version annotation changed")
        schema = scratch / "qualified.xsd"
        schema.write_text(schema_text)
        text = golden.read_text()
        keys = ["Required", "Optional", "Named", "Closed", "Open", "Lower", "Bounded",
                "Unbounded", "NamedRepeated", "Nested", "Value", "Choice", "Record", "Count",
                "Enabled", "Observed", "Elapsed", "Unknown", "Requ\\u0069red"]
        keys += ["Reading", "Detail", "Measurement", "Content", "More"]
        for key in keys:
            text = text.replace(f'"{key}"', f'"{{{qualified}}}{key}"')
            source = source.replace(f'""{key}""', f'""{{{qualified}}}{key}""')
        golden = scratch / "qualified.json"
        golden.write_text(text)
        source = source.replace("Programs.Oam", "Urn.Task095")
    consumer = scratch / "consumer"
    command = [str(args.generator.resolve()), "service-generate", "--schema", str(schema),
               "--contract", str(contract), "--language", "ada", "--world", "closed-schema"]
    checked([*command, "--with-codec", "--ada-codec-runtime", PROFILE, "--output", str(consumer)],
            ROOT, scratch / "generate.log")
    model = scratch / "model"
    checked([*command, "--output", str(model)], ROOT, scratch / "model.log")
    readiness = command.copy()
    readiness[1] = "service-check"
    checked([*readiness, "--with-codec", "--ada-codec-runtime", PROFILE], ROOT, scratch / "readiness.log")
    portable = subprocess.run([*readiness, "--with-codec"], cwd=ROOT, capture_output=True)
    (scratch / "portable.log").write_bytes(portable.stdout + portable.stderr)
    (scratch / "portable.exit").write_text(f"{portable.returncode}\n")
    if portable.returncode == 0 or b"Float32" not in portable.stdout + portable.stderr:
        raise RuntimeError("portable Float32 boundary changed")
    for path in model.iterdir():
        if path.read_bytes() != (consumer / path.name).read_bytes():
            raise RuntimeError(f"model/API differs: {path.name}")
    (consumer / f"{main_name}.adb").write_text(source)
    mains = f'"{main_name}.adb"'
    if args.environment:
        environment = (ROOT / "tests/ada-generated-codec/float32_environment.adb").read_text()
        if args.profile == "qualified":
            environment = environment.replace("Programs.Oam", "Urn.Task095")
        (consumer / "float32_environment.adb").write_text(environment)
        shutil.copy2(ROOT / "tests/ada-float32-probe/environment_harness.c", consumer)
        mains += ', "float32_environment.adb"'
    if args.rounding:
        rounding = (ROOT / "tests/ada-generated-codec/float32_rounding.adb").read_text()
        if args.profile == "qualified":
            rounding = rounding.replace("Programs.Oam", "Urn.Task095")
        (consumer / "float32_rounding.adb").write_text(rounding)
        shutil.copy2(ROOT / "tests/ada-float32-probe/rounding_harness.c", consumer)
        shutil.copy2(ROOT / "tests/ada-float32-probe/environment_harness.c", consumer)
        mains += ', "float32_rounding.adb"'
    if args.numeric_corpus:
        shutil.copy2(ROOT / "tests/ada-float32-probe/numeric_probe.adb", consumer)
        shutil.copy2(ROOT / "tests/ada-float32-probe/occurrence_controls.adb", consumer)
        mains += ', "numeric_probe.adb", "occurrence_controls.adb"'
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task095_consumer"\nversion = "0.1.0-dev"\n'
        'description = "Generated Float32 qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        f'{compiler} = "={version}"\ngprbuild = "=26.0.1"\n'
        'ams_gra_oms_json = "=0.1.0-dev"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_float32";\nproject Consumer is\n'
        'for Languages use ("Ada", "C");\nfor Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        f'for Main use ({mains});\npackage Compiler is\n'
        f'for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "{"-O2" if args.optimized else "-O0"}");\n'
        'for Default_Switches ("C") use ("-O2", "-Wall", "-Wextra", "-Werror");\n'
        'end Compiler; end Consumer;\n')
    checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], consumer, scratch / "isolation.log")
    checked(["alr", "-n", "--force", "toolchain", "--select", "--local",
             f"{compiler}={version}", "gprbuild=26.0.1"], consumer, scratch / "toolchain.log")
    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        build = ["alr", "-n", "build", "--", "-f", f"-j{build_jobs}"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            build += ["-cargs:Ada", f"-gnatec={adc}"]
        checked(build, consumer, scratch / f"{policy}-build.log")
        identity = checked(["alr", "-n", "exec", "--", "gnat", "--version"], consumer, scratch / f"{policy}-gnat.log")
        if f"GNAT {version}".encode() not in identity:
            raise RuntimeError("compiler identity mismatch")
        lock = consumer / "alire/alire.lock"
        releases = [s["release"] for s in tomllib.loads(lock.read_text())["solution"]["state"] if "release" in s]
        for name in ["gnatcoll", "gnatcoll_minimal"]:
            release = next(r for r in releases if r["name"] == name)
            archive = "sha512:cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b"
            if release["version"] != "26.0.0" or archive not in release["origin"]["hashes"]:
                raise RuntimeError("wrong provider")
        provider_sources = list((consumer / "alire").rglob("gnatcoll-json.adb"))
        if not provider_sources or any(hashlib.sha256(p.read_bytes()).hexdigest() != "7e07277731a44ea08716fd61a6462ebe7a7ce0722adc6dc787cbbaafd3099063" for p in provider_sources):
            raise RuntimeError("provider source hash mismatch")
        shutil.copy2(lock, scratch / f"{policy}-alire.lock")
        output = checked([str(consumer / f"bin/{main_name}"), str(golden)], consumer, scratch / f"{policy}-run.log")
        if output != marker:
            raise RuntimeError(f"incomplete generated execution: {output!r}")
        if args.environment:
            for mode in range(4):
                environment_output = checked([str(consumer / "bin/float32_environment"), str(golden), str(mode)], consumer, scratch / f"{policy}-environment-{mode}.log")
                expected = f"TASK095_C2_ENVIRONMENT_COMPLETE mode= {mode} checks= {49 if mode == 0 else 43} rejections= {0 if mode == 0 else 16}\n".encode()
                if environment_output != expected:
                    raise RuntimeError("incomplete generated environment controls")
        if args.rounding:
            pair_results = []
            for x87 in range(4):
                for mxcsr in range(4):
                    rounding_output = checked([str(consumer / "bin/float32_rounding"), str(golden), str(x87), str(mxcsr)], consumer, scratch / f"{policy}-rounding-{x87}-{mxcsr}.log")
                    supported = x87 == 0 and mxcsr == 0
                    expected = f"TASK095_C3_ADA_ROUNDING_COMPLETE x87= {x87} mxcsr= {mxcsr} checks= {218 if supported else 290} rejections= {0 if supported else 45}\n".encode()
                    if rounding_output != expected:
                        raise RuntimeError(f"incomplete rounding pair {x87}/{mxcsr}: {rounding_output!r}")
                    pair_results.append({"x87": x87, "mxcsr": mxcsr, "supported": supported})
            (scratch / f"{policy}-rounding-pairs.json").write_text(json.dumps(pair_results, indent=2) + "\n")
        if args.numeric_corpus:
            controls = checked([str(consumer / "bin/occurrence_controls")], consumer, scratch / f"{policy}-occurrences.log")
            if controls != b"TASK095_OCCURRENCE_CONTROLS_COMPLETE checks= 27\n":
                raise RuntimeError("occurrence controls incomplete")
            expected_hashes = {
                "conversions.tsv": "a1b0a2a6f0a9c8f11371391930b716e04ca64c5072ef8b0fb12ce273f4d6c140",
                "encoding.tsv": "a2b9a9c043e218c2e1cadf5118bb698c7e413ea66ef14fc0036ade8f3f682ccc",
            }
            for mode, filename, count in [("convert", "conversions.tsv", 15710), ("encode", "encoding.tsv", 5552)]:
                corpus = ROOT / "tests/ada-float32-probe" / filename
                if hashlib.sha256(corpus.read_bytes()).hexdigest() != expected_hashes[filename]:
                    raise RuntimeError("changed R1 corpus")
                numeric = checked([str(consumer / "bin/numeric_probe"), mode, str(corpus)], consumer, scratch / f"{policy}-{mode}.log")
                ada_fail = 1972 if mode == "convert" else 0
                numeric_marker = f"R1_NUMERIC_COMPLETE mode={mode} count= {count} ieee_fail= {ada_fail} float_fail= {ada_fail} c_fail= 0 write_fail= 0"
                if numeric.splitlines()[-1] != numeric_marker.encode():
                    raise RuntimeError("numeric corpus incomplete or failed")
                if b"MISMATCH\tstrtof_l" in numeric:
                    raise RuntimeError("production numeric mismatch")
                if mode == "encode" and len(numeric.splitlines()) != count + 4:
                    raise RuntimeError("encoding row count mismatch")
                if mode == "encode":
                    for line in numeric.decode().splitlines()[:count]:
                        expected, actual, spelling = line.split("\t")
                        if expected != actual or oracle.oracle(spelling) != expected:
                            raise RuntimeError("writer changed Float32 value")
        cells.append({"compiler": version, "profile": args.profile, "policy": policy, "marker": output.decode().strip()})
    paths = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".c", ".lock", ".gpr"]]
    paths += [args.generator.resolve(), schema, contract, golden, pathlib.Path(__file__).resolve(),
              ROOT / f"tests/ada-generated-codec/{main_name}.adb"]
    if args.environment:
        paths += [ROOT / "tests/ada-generated-codec/float32_environment.adb", ROOT / "tests/ada-float32-probe/environment_harness.c"]
    if args.rounding:
        paths += [ROOT / "tests/ada-generated-codec/float32_rounding.adb", ROOT / "tests/ada-float32-probe/rounding_harness.c"]
    (scratch / "sources.sha256").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print(f"TASK095_GENERATED_QUALIFICATION_COMPLETE compiler={version} profile={args.profile} policies=3")


if __name__ == "__main__":
    main()