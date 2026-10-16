#!/usr/bin/env python3
"""Generate, compile and execute Task 092 against unchanged hash-pinned UCI roots."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import shutil
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("integration", ROOT / "scripts/check-task091r.py")
integration = importlib.util.module_from_spec(spec)
spec.loader.exec_module(integration)
checked = integration.checked


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--uci25", type=pathlib.Path, required=True)
    parser.add_argument("--uci26", type=pathlib.Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    if checked(["alr", "--version"], ROOT, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("wrong Alire")
    roots = {"25": args.uci25.resolve(), "26": args.uci26.resolve()}
    expected = {"25": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
                "26": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b"}
    security = {"25": "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b",
                "26": "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88"}
    for version, path in roots.items():
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected[version]:
            raise RuntimeError("pinned root drift")
        marking = path.with_name(f"UCI_SecurityMarkings_v2_{version[-1]}_0.xsd")
        if hashlib.sha256(marking.read_bytes()).hexdigest() != security[version]:
            raise RuntimeError("pinned security schema drift")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    banner = "14.2.0" if args.compiler == "14" else "16.1.0"
    for version, message, stem, count in [
        ("25", "RDMA_InitializeSetup", "rdma", 12),
        ("26", "RDMA_InitializeSetup", "rdma", 12),
        ("25", "SurvivabilityRiskLevel", "survivability", 8),
    ]:
        evidence = scratch / f"{version}-{stem}"
        evidence.mkdir()
        consumer = evidence / "consumer"
        fixtures = ROOT / "tests/fixtures/service-generate"
        contract = evidence / "contract.yaml"
        contract.write_text((fixtures / "codec-ada-minimal.yaml").read_text()
                            .replace("AdaNotice", message).replace("AdaEcho", message))
        checked([str(args.generator.resolve()), "service-generate", "--schema", str(roots[version]),
                 "--contract", str(contract), "--language", "ada", "--world", "closed-schema",
                 "--with-codec", "--output", str(consumer)], ROOT, evidence / "generate.log")
        shutil.copytree(ROOT / "ada/oms-json-support", evidence / "support")
        source = ROOT / f"tests/ada-generated-codec/task092_real_{stem}.adb"
        shutil.copy2(source, consumer)
        (consumer / "alire.toml").write_text(
            'name = "task092_real"\nversion = "0.1.0-dev"\n'
            'description = "Pinned UCI qualification"\nlicenses = "MIT"\n'
            'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
            'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
            f'{compiler} = "={banner}"\n[[pins]]\n'
            'ams_gra_oms_json = { path = "../support" }\n')
        (consumer / "consumer.gpr").write_text(
            'with "ams_gra_oms_json"; project Consumer is\n'
            'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            f'for Main use ("{source.name}");\n'
            'package Compiler is for Default_Switches ("Ada") use '
            '("-gnat2022", "-gnato", "-O0"); end Compiler; end Consumer;\n')
        checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"],
                consumer, evidence / "isolation.log")
        checked(["alr", "-n", *(["--force"] if args.compiler == "14" else []),
                 "toolchain", "--select", "--local", f"{compiler}={banner}", "gprbuild=26.0.1"],
                consumer, evidence / "toolchain.log")
        golden = fixtures / f"task092-real-{stem}{version}.json"
        for policy in ["normal", "Check", "Ignore"]:
            command = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = evidence / f"{policy}.adc"
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                command += ["-cargs:Ada", f"-gnatec={adc}"]
            checked(command, consumer, evidence / f"{policy}-build.log")
            lock = consumer / "alire/alire.lock"
            integration.datetime.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
            shutil.copy2(lock, evidence / f"{policy}-alire.lock")
            for tool, identity in [("gnat", f"GNAT {banner}"), ("gprbuild", "26.0.0")]:
                if identity.encode() not in checked(["alr", "-n", "exec", "--", tool, "--version"],
                                                    consumer, evidence / f"{policy}-{tool}.log"):
                    raise RuntimeError("wrong tool identity")
            output = checked([str(consumer / "bin" / source.stem), str(golden)],
                             consumer, evidence / f"{policy}-run.log")
            if output != f"TASK092_REAL_PASS checks= {count}\n".encode():
                raise RuntimeError("wrong completion")
        paths = list(consumer.glob("*.ad?")) + [golden, source, roots[version], args.generator.resolve()]
        (evidence / "sources.sha256").write_text("".join(
            f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    print(f"TASK092_REAL_QUALIFICATION_PASS compiler={banner} selections=3 policies=3")


if __name__ == "__main__":
    main()