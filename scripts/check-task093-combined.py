#!/usr/bin/env python3
"""Execute Task 093's combined Binary/Choice/temporal generated Ada corpus."""
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
NAMESPACE = "https://www.vdl.afrl.af.mil/programs/task093"


def qualify(value):
    if isinstance(value, dict):
        return {f"{{{NAMESPACE}}}{key}": qualify(child) for key, child in value.items()}
    if isinstance(value, list):
        return [qualify(child) for child in value]
    return value


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
    fixtures = ROOT / "tests/fixtures/service-generate"
    original = (fixtures / "codec-ada-hexbinary-combined.xsd").read_text()
    consumer_source = ROOT / "tests/ada-generated-codec/hexbinary_combined_consumer.adb"
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    if checked(["alr", "--version"], scratch, scratch / "alire").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire identity differs")
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    records = []
    for profile in args.profiles:
        schema = scratch / f"{profile}.xsd"
        source = original
        consumer_text = consumer_source.read_text()
        golden = json.loads((fixtures / "task093-combined.json").read_text())
        if profile == "qualified":
            # Reuse Task 094's structural/reversible metadata-preserving checks,
            # with the Task 093 model namespace; no frozen fixture is changed.
            temporal.QUALIFIED = NAMESPACE
            source = temporal.qualified_schema(source, scratch / "namespace-conversion.json")
            golden = qualify(golden)
            consumer_text = consumer_text.replace("Programs.Oam", "Programs.Task093")
            for key in ["Picks", "Nested", "Enabled", "Dates", "Spans", "Data", "Hash"]:
                consumer_text = consumer_text.replace(f'"{key}"', f'"{{{NAMESPACE}}}{key}"')
        schema.write_text(source)
        golden_path = scratch / f"{profile}.json"
        golden_path.write_text(json.dumps(golden, indent=2) + "\n")
        directory = scratch / profile
        common = [str(args.generator.resolve()), "service-generate", "--schema", str(schema),
                  "--contract", str(fixtures / "codec-ada-hexbinary-combined.yaml"),
                  "--language", "ada", "--world", "closed-schema"]
        plain = scratch / (profile + "-plain")
        checked([*common, "--output", str(plain)], ROOT, scratch / (profile + "-plain-generate"))
        checked([*common, "--with-codec", "--output", str(directory)], ROOT,
                scratch / (profile + "-codec-generate"))
        parity = {}
        for path in plain.iterdir():
            if path.read_bytes() != (directory / path.name).read_bytes():
                raise RuntimeError(f"model/API changed: {path.name}")
            parity[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
        (scratch / (profile + "-model-parity.json")).write_text(json.dumps(parity, indent=2) + "\n")
        body = (directory / "service_codec.adb").read_text()
        for helper, count in [("Encode_Hex_Binary", 1), ("Decode_Hex_Binary", 1),
                              ("Scalar_Date_Time", 1), ("Scalar_Duration", 0)]:
            if body.count("function " + helper + " (") != count:
                raise RuntimeError(f"incorrect conditional helper count: {helper}")
        # A repeated Choice containing a Record with a Binary is admitted;
        # repetition of that Binary and Binary alternatives remain independent
        # negative controls, checked atomically through both CLI actions.
        controls = [
            ("direct-repeated", source.replace('name="Data" type="xs:hexBinary"',
             'name="Data" type="xs:hexBinary" maxOccurs="2"'), "direct Binary repeated element is not qualified"),
            ("named-repeated", source.replace('name="Hash" type="uci:Digest" minOccurs="0"',
             'name="Hash" type="uci:Digest" minOccurs="0" maxOccurs="2"'), "named Binary repeated element is not qualified"),
            ("direct-alternative", source.replace('name="Enabled" type="xs:boolean"',
             'name="Enabled" type="xs:hexBinary"'), "direct Binary Choice alternative is not qualified"),
            ("named-alternative", source.replace('name="Enabled" type="xs:boolean"',
             'name="Enabled" type="uci:Digest"'), "named Binary Choice alternative is not qualified"),
        ]
        for label, text, diagnostic in controls:
            negative = scratch / f"{profile}-{label}.xsd"
            negative.write_text(text)
            for action in ["service-check", "service-generate"]:
                output = scratch / f"{profile}-{label}-{action}-output"
                argv = [str(args.generator.resolve()), action, "--schema", str(negative),
                        "--contract", str(fixtures / "codec-ada-hexbinary-combined.yaml"),
                        "--language", "ada", "--world", "closed-schema", "--with-codec"]
                if action == "service-generate":
                    argv += ["--output", str(output)]
                result = subprocess.run(argv, cwd=ROOT, capture_output=True)
                log = scratch / f"{profile}-{label}-{action}.log"
                log.write_bytes(result.stdout + result.stderr)
                (log.with_suffix(".command.json")).write_text(json.dumps(
                    dict(argv=argv, exit_code=result.returncode, diagnostic=diagnostic), indent=2) + "\n")
                if result.returncode != 1 or diagnostic.encode() not in log.read_bytes() or output.exists():
                    raise RuntimeError(f"negative control failed: {log}")
        (directory / consumer_source.name).write_text(consumer_text)
        (directory / "alire.toml").write_text(
            'name = "task093_combined"\nversion = "0.1.0-dev"\n'
            'description = "Binary Choice temporal integration"\nlicenses = "MIT"\n'
            'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
            'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
            f'{compiler} = "={version}"\n[[pins]]\n'
            'ams_gra_oms_json = { path = "../support" }\n')
        (directory / "consumer.gpr").write_text(
            'with "ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            'for Main use ("hexbinary_combined_consumer.adb");\n'
            'package Compiler is for Default_Switches ("Ada") use '
            '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
        checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], directory,
                scratch / (profile + "-isolation"))
        checked(["alr", "-n", *(["--force"] if args.compiler == "14" else []),
                 "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"],
                directory, scratch / (profile + "-toolchain"))
        for policy in args.policies:
            label = f"{profile}-{policy}"
            command = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = scratch / (label + ".adc")
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                command += ["-cargs:Ada", f"-gnatec={adc}"]
            checked(command, directory, scratch / (label + "-build"))
            lock = directory / "alire/alire.lock"
            temporal.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
            shutil.copy2(lock, scratch / (label + "-alire.lock"))
            for tool, identity in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
                if identity.encode() not in checked(["alr", "-n", "exec", "--", tool, "--version"],
                                                    directory, scratch / (label + "-" + tool)):
                    raise RuntimeError(f"{tool} identity differs")
            output = checked([str(directory / "bin/hexbinary_combined_consumer"), str(golden_path)],
                             directory, scratch / (label + "-run"))
            if output != b"TASK093_COMBINED_COMPLETE checks= 49 rejections= 9\n":
                raise RuntimeError(f"incorrect completion marker: {output!r}")
            records.append(dict(profile=profile, policy=policy, checks=49, rejections=9))
    paths = list(scratch.rglob("*.ad?")) + [args.generator.resolve(), consumer_source,
            pathlib.Path(__file__).resolve(), fixtures / "codec-ada-hexbinary-combined.xsd",
            fixtures / "codec-ada-hexbinary-combined.yaml", fixtures / "task093-combined.json"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(paths)}, indent=2) + "\n")
    (scratch / "qualification.json").write_text(json.dumps(dict(
        compiler=version, cells=records, status="PASS"), indent=2) + "\n")
    print(f"TASK093_COMBINED_QUALIFICATION_PASS compiler={version} cells={len(records)}")


if __name__ == "__main__":
    main()