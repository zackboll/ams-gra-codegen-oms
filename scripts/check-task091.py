#!/usr/bin/env python3
"""Compile production-generated Duration consumers; missing gates are failures."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("guard", ROOT / "scripts/check-task087a.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def checked(command, cwd, log):
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_bytes(result.stdout)
    log.with_suffix(".command.json").write_text(json.dumps(
        {"command": command, "cwd": str(cwd), "exit": result.returncode}, indent=2) + "\n")
    print(f"exit={result.returncode} log={log}", flush=True)
    if result.returncode:
        raise RuntimeError(f"command failed: {command}; see {log}")
    return result.stdout


def unescape(text):
    return re.sub(r"\\u\{([0-9A-Fa-f]+)\}|\\([stnr\\])",
                  lambda m: chr(int(m[1], 16)) if m[1] else
                  {"s": " ", "t": "\t", "n": "\n", "r": "\r", "\\": "\\"}[m[2]], text)


def literal(text):
    data = text.encode("utf-8")
    return "String'(" + ", ".join(f"{i} => Character'Val ({b})" for i, b in enumerate(data, 1)) + ")" if data else '""'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    options = ["--force"] if args.compiler == "14" else []
    if checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire version differs")
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    corpus = ROOT / "tests/fixtures/temporal/duration.txt"
    oracle_hash = hashlib.sha256(corpus.read_bytes()).hexdigest()
    if oracle_hash != "1f723a55b4c79bc9e879c6d8ee1d7fa1b21d1e1d9b2dd725f763a8b9ab4222d7":
        raise RuntimeError("frozen Duration oracle bytes changed")
    calls = []
    for line in corpus.read_text().splitlines():
        if line.startswith("VALID "):
            source, expected = line[6:].split(" => ")
            calls.append(f"   Corpus ({literal(unescape(source))}, {literal(unescape(expected))}, True);")
        elif line == "INVALID" or line.startswith("INVALID "):
            calls.append(f'   Corpus ({literal(unescape(line[8:]))}, "", False);')
    if len(calls) != 100:
        raise RuntimeError(f"frozen corpus count changed: {len(calls)}")
    (scratch / "oracle.sha256").write_text(oracle_hash + "\n")
    qualified_namespace = "https://www.vdl.afrl.af.mil/programs/task091"
    qualified_schema = scratch / "qualified.xsd"
    original_schema = (fixtures / "codec-ada-duration.xsd").read_text()
    qualified_schema.write_text(original_schema.replace(
        'targetNamespace="https://www.vdl.afrl.af.mil/programs/oam"',
        f'targetNamespace="{qualified_namespace}"').replace(
        'xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam"',
        f'xmlns:uci="{qualified_namespace}"').replace(
        'elementFormDefault="qualified"',
        'xmlns:oam="https://www.vdl.afrl.af.mil/programs/oam" elementFormDefault="qualified"').replace(
        'uci:version=', 'oam:version='))
    def qualify(value):
        if isinstance(value, dict):
            return {f"{{{qualified_namespace}}}{key}": qualify(item) for key, item in value.items()}
        return value
    for filename in ["task091-duration.json", "task091-duration-absent.json"]:
        (scratch / filename).write_text(json.dumps(qualify(json.loads((fixtures / filename).read_text()))))
    named_schema = scratch / "named-only.xsd"
    named_schema.write_text(original_schema.replace('type="xs:duration"', 'type="uci:SpanType"'))
    for name in ["model", "codec", "qualified", "named"]:
        codec = name != "model"
        directory = scratch / name
        command = [str(args.generator.resolve()), "service-generate", "--schema",
                   str(qualified_schema if name == "qualified" else named_schema if name == "named"
                       else fixtures / "codec-ada-duration.xsd"), "--contract",
                   str(fixtures / "codec-ada-duration.yaml"), "--language", "ada",
                   "--world", "closed-schema", "--output", str(directory)]
        if codec:
            command.append("--with-codec")
        checked(command, ROOT, scratch / f"{name}-generate.log")
        if codec:
            golden_path = (scratch if name == "qualified" else fixtures) / "task091-duration.json"
            golden = json.loads(golden_path.read_text())
            reordered = json.dumps(dict(reversed(list(golden.items()))))
            (directory / "duration_codec_consumer-reordered_golden.adb").write_text(
                "separate (Duration_Codec_Consumer)\nprocedure Reordered_Golden is\n"
                f"   Item : constant DurationPayload := Decode_DurationPayload ({literal(reordered)});\n"
                "begin\n   Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_DurationPayload (Item)) =\n"
                "     AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden)));\nend Reordered_Golden;\n")
        consumer = "duration_codec_consumer" if codec else "duration_model_consumer"
        shutil.copy2(ROOT / f"tests/ada-generated-codec/{consumer}.adb", directory)
        if name == "qualified":
            path = directory / f"{consumer}.adb"
            text = path.read_text().replace("Programs.Oam", "Programs.Task091")
            for key in ["Named", "OtherNamed", "DirectDuration", "OptionalNamed", "OptionalDirect",
                        "InheritedDuration", "Details", "NestedDuration", "NestedDirect", "Unknown"]:
                text = text.replace(f'"{key}"', f'"{{{qualified_namespace}}}{key}"')
            path.write_text(text)
        if codec:
            (directory / "duration_codec_consumer-frozen_corpus.adb").write_text(
                "separate (Duration_Codec_Consumer)\nprocedure Frozen_Corpus is\nbegin\n" +
                "\n".join(calls) + "\nend Frozen_Corpus;\n")
            for path in (scratch / "model").glob("*.ad?") if name == "codec" else []:
                if path.name.startswith("duration_"):
                    continue
                if path.read_bytes() != (directory / path.name).read_bytes():
                    raise RuntimeError(f"model/API bytes changed: {path.name}")
            body = (directory / "service_codec.adb").read_text()
            if body.count("function Scalar_Duration") != (0 if name == "named" else 1):
                raise RuntimeError("direct helper missing or duplicated")
            if (directory / "service_codec.ads").read_text().count("function Encode_DurationPayload") != 1:
                raise RuntimeError("payload operations not deduplicated")
        (directory / "alire.toml").write_text(
            f'name = "task091_{name}"\nversion = "0.1.0-dev"\n'
            'description = "Duration qualification"\nlicenses = "MIT"\n'
            'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
            'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
            f'{compiler} = "={version}"\n[[pins]]\n'
            'ams_gra_oms_json = { path = "../support" }\n')
        (directory / "consumer.gpr").write_text(
            'with "ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            f'for Main use ("{consumer}.adb");\n'
            'package Compiler is for Default_Switches ("Ada") use '
            '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
        checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], directory,
                scratch / f"{name}-isolation.log")
        checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                       f"{compiler}={version}", "gprbuild=26.0.1"], directory,
                      scratch / f"{name}-toolchain.log")
        for policy in ["normal", "Check", "Ignore"]:
            command = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = scratch / f"{policy}.adc"
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                command += ["-cargs:Ada", f"-gnatec={adc}"]
            checked(command, directory, scratch / f"{name}-{policy}-build.log")
            lock = directory / "alire/alire.lock"
            guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
            shutil.copy2(lock, scratch / f"{name}-{policy}-alire.lock")
            identity = checked(["alr", "-n", "exec", "--", "gnat", "--version"], directory,
                                     scratch / f"{name}-{policy}-gnat.log")
            if f"GNAT {version}".encode() not in identity:
                raise RuntimeError("compiler identity differs")
            identity = checked(["alr", "-n", "exec", "--", "gprbuild", "--version"], directory,
                                     scratch / f"{name}-{policy}-gprbuild.log")
            if b"26.0.0" not in identity:
                raise RuntimeError("GPRBUILD executable identity differs")
            command = [str(directory / "bin" / consumer)]
            if codec:
                goldens = scratch if name == "qualified" else fixtures
                command += [str(goldens / "task091-duration.json"), str(goldens / "task091-duration-absent.json")]
            output = checked(command, directory, scratch / f"{name}-{policy}-run.log")
            expected = b"TASK091_CODEC_COMPLETE cases= 650\n" if codec else b"TASK091_MODEL_COMPLETE cases= 10\n"
            if output != expected:
                raise RuntimeError(f"incorrect completion marker: {output!r}")
    hashes = []
    for directory in [scratch / name for name in ["model", "codec", "qualified", "named"]]:
        for path in sorted(directory.glob("*.ad?")):
            hashes.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(scratch)}")
    (scratch / "sources.sha256").write_text("\n".join(hashes) + "\n")
    print(f"TASK091 QUALIFICATION PASSED compiler={version} policies=3 corpus=100")


if __name__ == "__main__":
    main()