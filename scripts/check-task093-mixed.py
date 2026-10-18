#!/usr/bin/env python3
"""Task 093-C3 production-generated Binary/Float32 composition qualification."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("temporal", ROOT / "scripts/check-task094.py")
temporal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(temporal)
checked = temporal.checked
RUNTIME = "linux-x86_64-glibc-2.41"
NAMESPACE = "https://www.vdl.afrl.af.mil/programs/task093"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    parser.add_argument("--profiles", nargs="+", choices=["oam", "qualified"], default=["oam", "qualified"])
    parser.add_argument("--policies", nargs="+", choices=["normal", "Check", "Ignore"], default=["normal", "Check", "Ignore"])
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    if subprocess.check_output(["getconf", "GNU_LIBC_VERSION"]).strip() != b"glibc 2.41" or subprocess.check_output(["uname", "-m"]).strip() != b"x86_64":
        raise RuntimeError("unqualified runtime")
    checked(["sh", "-ec", "uname -m; getconf GNU_LIBC_VERSION; dpkg-query -W libc6; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6; alr --version"], ROOT, scratch / "runtime")
    fixtures = ROOT / "tests/fixtures/service-generate"
    original = (fixtures / "codec-ada-hexbinary-float32.xsd").read_text()
    golden = (fixtures / "task093-mixed.json").read_text().strip()
    canonical = (fixtures / "task093-mixed-canonical.json").read_text().strip()
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    records = []
    for profile in args.profiles:
        schema = scratch / f"{profile}.xsd"
        source = original
        text = golden
        canonical_text = canonical
        consumers = {name: (ROOT / f"tests/ada-generated-codec/{name}.adb").read_text()
                     for name in ["hexbinary_float32_consumer", "hexbinary_float32_environment"]}
        if profile == "qualified":
            temporal.QUALIFIED = NAMESPACE
            source = temporal.qualified_schema(source, scratch / "namespace-conversion.json")
            # Change only JSON key spellings: never parse numeric golden lexemes.
            keys = ["Picks", "Nested", "Enabled", "Dates", "Spans", "Data", "Optional", "Hash", "Blob", "Required", "Checked", "Samples", "Unknown", "Da\\u0074a", "Nes\\u0074ed"]
            for key in keys:
                text = text.replace(f'"{key}":', f'"{{{NAMESPACE}}}{key}":')
                canonical_text = canonical_text.replace(f'"{key}":', f'"{{{NAMESPACE}}}{key}":')
                for name in consumers:
                    consumers[name] = consumers[name].replace(f'""{key}""', f'""{{{NAMESPACE}}}{key}""')
                    consumers[name] = consumers[name].replace(f'"{key}"', f'"{{{NAMESPACE}}}{key}"')
            consumers = {name: value.replace("Programs.Oam", "Programs.Task093") for name, value in consumers.items()}
            assert "1.000000059604644775390626" in text and '"version"' not in text
        schema.write_text(source)
        golden_path = scratch / f"{profile}.json"
        golden_path.write_text(text + "\n")
        canonical_path = scratch / f"{profile}-canonical.json"
        canonical_path.write_text(canonical_text + "\n")
        directory = scratch / profile
        common = [str(args.generator.resolve()), "service-generate", "--schema", str(schema), "--contract", str(fixtures / "codec-ada-hexbinary-float32.yaml"), "--language", "ada", "--world", "closed-schema"]
        plain = scratch / f"{profile}-plain"
        checked([*common, "--output", str(plain)], ROOT, scratch / f"{profile}-plain-generate")
        checked([*common, "--with-codec", "--ada-codec-runtime", RUNTIME, "--output", str(directory)], ROOT, scratch / f"{profile}-generate")
        for path in plain.iterdir():
            if path.read_bytes() != (directory / path.name).read_bytes():
                raise RuntimeError(f"model/API changed: {path}")
        for label, old, new in [
            ("repeated-binary", 'name="Data" type="xs:hexBinary"', 'name="Data" type="xs:hexBinary" maxOccurs="2"'),
            ("binary-choice", 'name="Enabled" type="xs:boolean"', 'name="Enabled" type="xs:hexBinary"'),
            ("named-binary-choice", 'name="Enabled" type="xs:boolean"', 'name="Enabled" type="uci:Blob"'),
            ("float-choice", 'name="Enabled" type="xs:boolean"', 'name="Enabled" type="xs:float"'),
        ]:
            negative = scratch / f"{profile}-{label}.xsd"
            assert source.count(old) == 1
            negative.write_text(source.replace(old, new))
            for qualified in [False, True]:
                for action in ["service-check", "service-generate"]:
                    destination = scratch / f"{profile}-{label}-{qualified}-{action}-output"
                    command = [*common]
                    command[1] = action
                    command[3] = str(negative)
                    command += ["--with-codec"]
                    if qualified:
                        command += ["--ada-codec-runtime", RUNTIME]
                    if action == "service-generate":
                        command += ["--output", str(destination)]
                    result = subprocess.run(command, cwd=ROOT, capture_output=True)
                    stem = scratch / f"{profile}-{label}-{qualified}-{action}"
                    stem.with_suffix(".log").write_bytes(result.stdout + result.stderr)
                    stem.with_suffix(".command.json").write_text(json.dumps(dict(command=command, exit=result.returncode), indent=2) + "\n")
                    if result.returncode != 1 or destination.exists():
                        raise RuntimeError(f"non-atomic unsupported shape: {stem}")
        for name, value in consumers.items():
            (directory / f"{name}.adb").write_text(value)
        for name in ["environment_harness.c", "rounding_harness.c"]:
            shutil.copy2(ROOT / "tests/ada-float32-probe" / name, directory)
        (directory / "alire.toml").write_text(
            'name = "task093_mixed"\nversion = "0.1.0-dev"\ndescription = "Binary Float32 integration"\nlicenses = "MIT"\nproject-files = ["consumer.gpr"]\n[[depends-on]]\n'
            'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
            f'{compiler} = "={version}"\n[[pins]]\nams_gra_oms_json = {{ path = "../support" }}\n')
        (directory / "consumer.gpr").write_text(
            'with "../support/ams_gra_oms_json_float32";\nproject Consumer is\nfor Languages use ("Ada", "C");\nfor Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            'for Main use ("hexbinary_float32_consumer.adb", "hexbinary_float32_environment.adb");\npackage Compiler is\nfor Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\nfor Default_Switches ("C") use ("-O2", "-Wall", "-Wextra", "-Werror");\nend Compiler;\npackage Linker is for Default_Switches ("Ada") use ("-lm"); end Linker;\nend Consumer;\n')
        checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], directory, scratch / f"{profile}-isolation")
        checked(["alr", "-n", "--force", "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"], directory, scratch / f"{profile}-toolchain")
        for policy in args.policies:
            label = f"{profile}-{policy}"
            command = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = scratch / f"{label}.adc"
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                command += ["-cargs:Ada", f"-gnatec={adc}"]
            checked(command, directory, scratch / f"{label}-build")
            lock = directory / "alire/alire.lock"
            temporal.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
            shutil.copy2(lock, scratch / f"{label}-alire.lock")
            identity = checked(["alr", "-n", "exec", "--", "gnat", "--version"], directory, scratch / f"{label}-gnat")
            if f"GNAT {version}".encode() not in identity:
                raise RuntimeError("compiler mismatch")
            for name, marker in [("consumer", b"TASK093_C3_MIXED_COMPLETE checks= 95 rejections= 26\n"), ("environment", b"TASK093_C3_ENVIRONMENT_COMPLETE checks= 770 rejections= 126\n")]:
                output = checked([str(directory / f"bin/hexbinary_float32_{name}"), str(golden_path), str(canonical_path)], directory, scratch / f"{label}-{name}")
                if output != marker:
                    raise RuntimeError(f"incomplete execution: {output!r}")
            records.append(dict(compiler=version, profile=profile, policy=policy))
    paths = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".c", ".gpr", ".lock"]]
    paths += [args.generator.resolve(), pathlib.Path(__file__).resolve(), *fixtures.glob("*hexbinary-float32.*"), fixtures / "task093-mixed.json", fixtures / "task093-mixed-canonical.json"]
    (scratch / "sources.sha256").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps(dict(completed=True, cells=records), indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print(f"TASK093_C3_MIXED_QUALIFICATION_COMPLETE compiler={version} cells={len(records)}")


if __name__ == "__main__":
    main()