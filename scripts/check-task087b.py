#!/usr/bin/env python3
"""Execute the production-generated Ada codec; no skip path."""
import argparse
import importlib.util
import pathlib
import shutil
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("guard_qualification", ROOT / "scripts/check-task087a.py")
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
    consumer = scratch / "consumer"
    fixtures = ROOT / "tests/fixtures/service-generate"
    guard.checked([str(args.generator.resolve()), "service-generate", "--schema",
                   str(fixtures / "codec-ada-minimal.xsd"), "--contract",
                   str(fixtures / "codec-ada-minimal.yaml"), "--language", "ada",
                   "--world", "closed-schema", "--with-codec", "--output", str(consumer)],
                  ROOT, scratch / "generate.log")
    shutil.copy2(ROOT / "tests/ada-generated-codec/codec_consumer.adb", consumer)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task087b_consumer"\nversion = "0.1.0-dev"\n'
        'description = "Generated codec qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("codec_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use '
        '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
    if guard.checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire version differs")
    options = ["--force"] if args.compiler == "14" else []
    guard.checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                   f"{compiler}={version}", "gprbuild=26.0.1"], consumer, scratch / "toolchain.log")
    for policy in ["normal", "Check", "Ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        guard.checked(command, consumer, scratch / f"{policy}-build.log")
        guard.validate_solution(tomllib.loads((consumer / "alire/alire.lock").read_text()), compiler)
        for tool, expected in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            tool_output = guard.checked(["alr", "-n", "exec", "--", tool, "--version"],
                                        consumer, scratch / f"{policy}-{tool}.log")
            if expected.encode() not in tool_output:
                raise RuntimeError(f"wrong selected {tool} version")
        output = guard.checked([str(consumer / "bin/codec_consumer"),
                                str(fixtures / "codec-ada-minimal.json")], consumer,
                               scratch / f"{policy}-run.log")
        if output.count(b"TASK087B_CODEC_COMPLETE cases= 41\n") != 1 or output.count(b"PASS ") != 41:
            raise RuntimeError("missing or incorrect executed test count/completion marker")
    shutil.copy2(consumer / "alire/alire.lock", scratch / "qualified-alire.lock")
    # Additional constructs within the same policy; the independent golden and
    # original fixture are never rewritten. No new frontend interpretation.
    subset = scratch / "subset"
    text = (fixtures / "codec-ada-minimal.xsd").read_text()
    text = text.replace('  <xs:complexType name="AdaPayload">\n    <xs:sequence>', '''
  <xs:simpleType name="SmallUnsigned"><xs:restriction base="xs:unsignedByte"/></xs:simpleType>
  <xs:simpleType name="NamedBoolean"><xs:restriction base="xs:boolean"/></xs:simpleType>
  <xs:complexType name="Base"><xs:sequence><xs:element name="Inherited" type="xs:string"/></xs:sequence></xs:complexType>
  <xs:complexType name="AdaPayload"><xs:complexContent><xs:extension base="uci:Base">
    <xs:sequence>''')
    text = text.replace('      <xs:element name="OptionalLabel" type="uci:Label" minOccurs="0"/>', '''
      <xs:element name="OptionalLabel" type="uci:Label" minOccurs="0"/>
      <xs:element name="Signed" type="xs:byte"/>
      <xs:element name="Unsigned" type="xs:unsignedByte"/>
      <xs:element name="NamedUnsigned" type="uci:SmallUnsigned"/>
      <xs:element name="NamedBoolean" type="uci:NamedBoolean"/>
      <xs:element name="OptionalDetails" type="uci:Details" minOccurs="0"/>''')
    text = text.replace('    </xs:sequence>\n  </xs:complexType>\n  <xs:element name="AdaNotice"',
                        '    </xs:sequence></xs:extension></xs:complexContent>\n  </xs:complexType>\n  <xs:element name="AdaNotice"')
    schema = scratch / "subset.xsd"
    schema.write_text(text)
    guard.checked([str(args.generator.resolve()), "service-generate", "--schema", str(schema),
                   "--contract", str(fixtures / "codec-ada-minimal.yaml"), "--language", "ada",
                   "--world", "closed-schema", "--with-codec", "--output", str(subset)],
                  ROOT, scratch / "subset-generate.log")
    shutil.copy2(ROOT / "tests/ada-generated-codec/subset_consumer.adb", subset)
    shutil.copy2(consumer / "alire.toml", subset)
    (subset / "consumer.gpr").write_text((consumer / "consumer.gpr").read_text().replace("codec_consumer.adb", "subset_consumer.adb"))
    guard.checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                   f"{compiler}={version}", "gprbuild=26.0.1"], subset, scratch / "subset-toolchain.log")
    for policy in ["normal", "Check", "Ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            command += ["-cargs:Ada", f"-gnatec={scratch / (policy + '.adc')}"]
        guard.checked(command, subset, scratch / f"subset-{policy}-build.log")
        output = guard.checked([str(subset / "bin/subset_consumer")], subset, scratch / f"subset-{policy}-run.log")
        if output != b"TASK087B_SUBSET_COMPLETE cases= 7\n":
            raise RuntimeError("subset corpus did not execute")
    # The frontend rejects xs:unsignedLong even before model readiness. The
    # independent core IR test covers the signed-64 codec domain boundary.
    schema.write_text(text.replace('name="Unsigned" type="xs:unsignedByte"',
                                   'name="Unsigned" type="xs:unsignedLong"'))
    import subprocess
    rejected_dir = scratch / "unsupported-output"
    rejected = subprocess.run([str(args.generator.resolve()), "service-generate", "--schema", str(schema),
                               "--contract", str(fixtures / "codec-ada-minimal.yaml"), "--language", "ada",
                               "--world", "closed-schema", "--with-codec", "--output", str(rejected_dir)],
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    (scratch / "unsupported-unsigned.log").write_bytes(rejected.stdout)
    if rejected.returncode != 1 or rejected_dir.exists() or b"unsupported XSD construct: xs:unsignedLong" not in rejected.stdout:
        raise RuntimeError("unsupported unsigned domain preflight did not fail atomically")
    print(f"TASK087B QUALIFICATION PASSED compiler={version} policies=3")


if __name__ == "__main__":
    main()