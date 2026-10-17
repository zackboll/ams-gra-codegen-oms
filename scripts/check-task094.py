#!/usr/bin/env python3
"""Execute production-generated repeated temporal codecs; fail on missing gates."""
import argparse
import hashlib
import io
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("guard", ROOT / "scripts/check-task087a.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

OAM = "https://www.vdl.afrl.af.mil/programs/oam"
QUALIFIED = "https://www.vdl.afrl.af.mil/programs/task094"
XS = "{http://www.w3.org/2001/XMLSchema}"


def qualified_schema(original, manifest):
    """Move model identity, not the independently namespaced version metadata."""
    replacements = [
        (f'targetNamespace="{OAM}"', f'targetNamespace="{QUALIFIED}"', 1),
        (f'xmlns:uci="{OAM}"', f'xmlns:uci="{QUALIFIED}"', 1),
        ('elementFormDefault="qualified"',
         f'xmlns:oam="{OAM}" elementFormDefault="qualified"', 1),
        ('uci:version=', 'oam:version=', 1),
    ]
    converted = original
    for old, new, count in replacements:
        if converted.count(old) != count:
            raise RuntimeError(f"ambiguous namespace replacement: {old}")
        converted = converted.replace(old, new)
    bindings = dict(value for _, value in ET.iterparse(io.StringIO(converted), events=["start-ns"]))
    before, after = ET.fromstring(original), ET.fromstring(converted)
    assert after.attrib["targetNamespace"] == QUALIFIED
    assert bindings["uci"] == QUALIFIED and bindings["oam"] == OAM
    version_key = f"{{{OAM}}}version"
    messages_before = before.findall(XS + "element")
    messages_after = after.findall(XS + "element")
    assert messages_before and len(messages_before) == len(messages_after)
    versions = []
    for old, new in zip(messages_before, messages_after, strict=True):
        assert version_key in old.attrib and version_key in new.attrib
        assert old.attrib[version_key] == new.attrib[version_key]
        assert [key for key in new.attrib if key.endswith("}version")] == [version_key]
        versions.append({"message": new.attrib["name"], "value": new.attrib[version_key]})
    declarations = {node.attrib["name"] for node in after
                    if node.tag in [XS + "simpleType", XS + "complexType"]}
    references = []
    for node in after.iter():
        reference = node.attrib.get("type", "")
        if reference.startswith("uci:"):
            assert reference[4:] in declarations
            references.append(reference)
    assert len(list(before.iter())) == len(list(after.iter()))
    after.attrib["targetNamespace"] = OAM
    # Expanded version names are identical despite the changed lexical prefix.
    assert ET.tostring(before) == ET.tostring(after), "unrelated schema modification"
    restored = converted
    for old, new, _ in reversed(replacements):
        restored = restored.replace(new, old)
    assert restored == original, "non-namespace bytes changed"
    manifest.write_text(json.dumps({
        "source_sha256": hashlib.sha256(original.encode()).hexdigest(),
        "qualified_sha256": hashlib.sha256(converted.encode()).hexdigest(),
        "target_namespace": QUALIFIED, "bindings": bindings,
        "version_qname": version_key, "versions": versions,
        "local_type_references": references, "node_count": len(list(before.iter())),
        "element_count": len(list(before.iter(XS + "element"))),
        "type_count": len(declarations),
        "replacement_counts": {old: count for old, _, count in replacements},
    }, indent=2) + "\n")
    return converted


def checked(command, cwd, stem):
    log = pathlib.Path(str(stem) + ".log")
    record = pathlib.Path(str(stem) + ".command.json")
    if log.exists() or record.exists():
        raise RuntimeError(f"evidence collision: {stem}")
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_bytes(result.stdout)
    record.write_text(json.dumps({"command": command, "cwd": str(cwd),
                                  "exit": result.returncode}, indent=2) + "\n")
    print(f"exit={result.returncode} log={log}", flush=True)
    if result.returncode:
        raise RuntimeError(f"command failed; see {log}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    campaigns = parser.add_mutually_exclusive_group()
    campaigns.add_argument("--composition", action="store_true")
    campaigns.add_argument("--combined", action="store_true")
    parser.add_argument("--profiles", nargs="+", choices=["oam", "qualified"], default=["oam", "qualified"])
    parser.add_argument("--policies", nargs="+", choices=["normal", "Check", "Ignore"], default=["normal", "Check", "Ignore"])
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    options = ["--force"] if args.compiler == "14" else []
    if args.combined:
        fixture = "codec-ada-choice-temporal.xsd"
        golden_name = "task094-choice-temporal.json"
        name = "repeated_choice_temporal_consumer"
        marker = b"TASK094_COMBINED_COMPLETE checks= 26 rejections= 2\n"
    elif args.composition:
        fixture = "codec-ada-repeated-temporal-composition.xsd"
        golden_name = "task094-composition.json"
        name = "repeated_temporal_composition_consumer"
        marker = b"TASK094_COMPOSITION_COMPLETE checks= 19 rejections= 3\n"
    else:
        fixture = "codec-ada-repeated-temporal.xsd"
        golden_name = "task094-temporal.json"
        name = "repeated_temporal_codec_consumer"
        marker = b"TASK094_CODEC_COMPLETE checks= 54 rejections= 17\n"
    if checked(["alr", "--version"], scratch, scratch / "alire").strip() != b"alr 2.1.1":
        raise RuntimeError("wrong Alire version")
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    for profile in args.profiles:
        prefix = f"{profile}-gnat{version}"
        schema = scratch / f"{profile}.xsd"
        namespace = QUALIFIED
        text = (fixtures / fixture).read_text()
        golden = (fixtures / golden_name).read_text()
        minimum = (fixtures / "task094-temporal-minimum.json").read_text()
        consumer_text = (ROOT / "tests/ada-generated-codec" / (name + ".adb")).read_text()
        if profile == "qualified":
            text = qualified_schema(text, scratch / (prefix + "-namespace-conversion.json"))
            def qualify(value):
                if isinstance(value, dict):
                    return {f"{{{namespace}}}{key}": qualify(child) for key, child in value.items()}
                if isinstance(value, list):
                    return [qualify(child) for child in value]
                return value
            golden = json.dumps(qualify(json.loads(golden)))
            minimum = json.dumps({f"{{{namespace}}}{k}": v for k, v in json.loads(minimum).items()})
            consumer_text = consumer_text.replace("Programs.Oam", "Programs.Task094")
            keys = [*json.loads((fixtures / golden_name).read_text()), "Unknown", "Dates", "Spans", "Nested", "Enabled"]
            for key in keys:
                consumer_text = consumer_text.replace(f'"{key}"', f'"{{{namespace}}}{key}"')
        schema.write_text(text)
        golden_path = scratch / f"{profile}-golden.json"
        golden_path.write_text(golden)
        minimum_path = scratch / f"{profile}-minimum.json"
        minimum_path.write_text(minimum)
        directory = scratch / profile
        command = [str(args.generator.resolve()), "service-generate", "--schema", str(schema),
                   "--contract", str(fixtures / "codec-ada-repeated-temporal.yaml"),
                   "--language", "ada", "--world", "closed-schema"]
        plain = scratch / (profile + "-plain")
        checked(command + ["--output", str(plain)], ROOT, scratch / (prefix + "-plain-generate"))
        checked(command + ["--with-codec", "--output", str(directory)], ROOT,
                scratch / (prefix + "-codec-generate"))
        parity = {}
        for path in plain.iterdir():
            if path.read_bytes() != (directory / path.name).read_bytes():
                raise RuntimeError(f"model/API changed: {path.name}")
            parity[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
        (scratch / (prefix + "-model-parity.json")).write_text(json.dumps(parity, indent=2))
        body = (directory / "service_codec.adb").read_text()
        for helper in ["Scalar_Date_Time", "Scalar_Duration"]:
            # The combined fixture uses named Duration only: no direct scalar
            # helper should be emitted for that carrier.
            expected = 0 if args.combined and helper == "Scalar_Duration" else 1
            if body.count("function " + helper + " (") != expected:
                raise RuntimeError(f"missing/duplicated repeated-only helper: {helper}")
        (directory / (name + ".adb")).write_text(consumer_text)
        (directory / "alire.toml").write_text(
            'name = "task094_consumer"\nversion = "0.1.0-dev"\n'
            'description = "Repeated temporal qualification"\nlicenses = "MIT"\n'
            'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
            'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
            f'{compiler} = "={version}"\n[[pins]]\n'
            'ams_gra_oms_json = { path = "../support" }\n')
        (directory / "consumer.gpr").write_text(
            'with "ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            f'for Main use ("{name}.adb");\n'
            'package Compiler is for Default_Switches ("Ada") use '
            '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
        checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], directory,
                scratch / (prefix + "-isolation"))
        checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                 f"{compiler}={version}", "gprbuild=26.0.1"], directory,
                scratch / (prefix + "-toolchain"))
        for policy in args.policies:
            identity = f"{prefix}-{policy}-{name}"
            build = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = scratch / (identity + ".adc")
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                build += ["-cargs:Ada", f"-gnatec={adc}"]
            checked(build, directory, scratch / (identity + "-build"))
            lock = directory / "alire/alire.lock"
            guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
            shutil.copy2(lock, scratch / (identity + "-alire.lock"))
            for tool, banner in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
                if banner.encode() not in checked(["alr", "-n", "exec", "--", tool, "--version"],
                                                  directory, scratch / (identity + "-" + tool)):
                    raise RuntimeError(f"wrong {tool} identity")
            output = checked([str(directory / "bin" / name), str(golden_path), str(minimum_path)], directory,
                             scratch / (identity + "-execute"))
            if output != marker:
                raise RuntimeError(f"wrong marker/counts: {output!r}")
    paths = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".lock"]]
    paths.append(args.generator.resolve())
    (scratch / "sources.sha256").write_text("".join(
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    print(f"TASK094 QUALIFICATION PASSED compiler={version} profiles={len(args.profiles)} policies={len(args.policies)} {marker.decode().strip()}")


if __name__ == "__main__":
    main()