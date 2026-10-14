#!/usr/bin/env python3
"""Execute production-generated Task 088 Choice codecs; no skip path."""
import argparse
import hashlib
import importlib.util
import pathlib
import shutil
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("guard", ROOT / "scripts/check-task087a.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    generator = str(args.generator.resolve())
    fixtures = ROOT / "tests/fixtures/service-generate"
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    options = ["--force"] if args.compiler == "14" else []
    if guard.checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("wrong Alire version")
    hashes = []
    for fixture, main_name, marker in [
        ("codec-ada-choice", "choice_consumer", b"TASK088_CODEC_COMPLETE cases= 172\n"),
        ("codec-choice", "qualified_choice_consumer", b"TASK088_QUALIFIED_COMPLETE cases= 2\n"),
    ]:
        consumer = scratch / fixture
        command = [generator, "service-generate", "--schema", str(fixtures / (fixture + ".xsd")),
                   "--contract", str(fixtures / (fixture + ".yaml")), "--language", "ada",
                   "--world", "closed-schema"]
        plain = scratch / (fixture + "-plain")
        guard.checked(command + ["--output", str(plain)], ROOT, scratch / (fixture + "-plain.log"))
        guard.checked(command + ["--with-codec", "--output", str(consumer)], ROOT,
                      scratch / (fixture + "-generate.log"))
        for source in plain.iterdir():
            if source.read_bytes() != (consumer / source.name).read_bytes():
                raise RuntimeError("model/API bytes changed")
        for source in sorted(consumer.glob("*.ad*")):
            hashes.append(f"{hashlib.sha256(source.read_bytes()).hexdigest()}  {fixture}/{source.name}")
        shutil.copy2(ROOT / "tests/ada-generated-codec" / (main_name + ".adb"), consumer)
        (consumer / "alire.toml").write_text(
            'name = "task088_consumer"\nversion = "0.1.0-dev"\n'
            'description = "Choice qualification"\nlicenses = "MIT"\n'
            'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
            'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
            f'{compiler} = "={version}"\n[[pins]]\n'
            'ams_gra_oms_json = { path = "../support" }\n')
        (consumer / "consumer.gpr").write_text(
            'with "ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
            f'for Exec_Dir use "bin"; for Main use ("{main_name}.adb");\n'
            'package Compiler is for Default_Switches ("Ada") use '
            '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
        guard.checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                       f"{compiler}={version}", "gprbuild=26.0.1"], consumer,
                      scratch / (fixture + "-toolchain.log"))
        for policy in ["normal", "Check", "Ignore"]:
            prefix = fixture + "-" + policy
            build = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = scratch / (policy + ".adc")
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                build += ["-cargs:Ada", f"-gnatec={adc}"]
            guard.checked(build, consumer, scratch / (prefix + "-build.log"))
            guard.validate_solution(tomllib.loads((consumer / "alire/alire.lock").read_text()), compiler)
            for tool, expected in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
                output = guard.checked(["alr", "-n", "exec", "--", tool, "--version"], consumer,
                                       scratch / (prefix + "-" + tool + ".log"))
                if expected.encode() not in output:
                    raise RuntimeError("wrong compiler/build tool")
            run = [str(consumer / "bin" / main_name)]
            if fixture == "codec-ada-choice":
                run += [str(fixtures / "codec-ada-choice.json")]
            if guard.checked(run, consumer, scratch / (prefix + "-run.log")) != marker:
                raise RuntimeError("absent/incorrect completion marker")
    # Readiness rejection must also leave no partial output.
    text = (fixtures / "codec-ada-choice.xsd").read_text()
    for label, replacement in [("optional", 'minOccurs="0"'),
                               ("repeated", 'maxOccurs="2"'),
                               ("nillable", 'nillable="true"')]:
        schema = scratch / (label + ".xsd")
        schema.write_text(text.replace('name="Enabled" type="xs:boolean"',
                                       f'name="Enabled" type="xs:boolean" {replacement}'))
        output = scratch / (label + "-output")
        result = subprocess.run([generator, "service-generate", "--schema", str(schema),
                                 "--contract", str(fixtures / "codec-ada-choice.yaml"),
                                 "--language", "ada", "--world", "closed-schema", "--with-codec",
                                 "--output", str(output)], capture_output=True, check=False)
        (scratch / (label + "-rejection.log")).write_bytes(result.stdout + result.stderr)
        if result.returncode != 1 or output.exists():
            raise RuntimeError("unsupported alternative was not rejected atomically")
    schema = scratch / "polymorphic.xsd"
    schema.write_text(text.replace('<xs:complexType name="Details">',
                                   '<xs:complexType name="Details" abstract="true">'))
    output = scratch / "polymorphic-output"
    result = subprocess.run([generator, "service-generate", "--schema", str(schema),
                             "--contract", str(fixtures / "codec-ada-choice.yaml"),
                             "--language", "ada", "--world", "closed-schema", "--with-codec",
                             "--output", str(output)], capture_output=True, check=False)
    (scratch / "polymorphic-rejection.log").write_bytes(result.stdout + result.stderr)
    if result.returncode != 1 or output.exists():
        raise RuntimeError("polymorphic alternative was not rejected atomically")
    (scratch / "source-hashes.txt").write_text("\n".join(hashes) + "\n")
    print(f"TASK088 QUALIFICATION PASSED compiler={version} policies=3 cases=174")


if __name__ == "__main__":
    main()