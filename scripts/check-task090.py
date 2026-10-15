#!/usr/bin/env python3
"""Mandatory production-generated DateTime codec qualification (no skips)."""
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
    (log.with_suffix(".command.json")).write_text(json.dumps(command) + "\n")
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_bytes(result.stdout)
    log.with_suffix(".exit").write_text(str(result.returncode) + "\n")
    print(f"exit={result.returncode} log={log}", flush=True)
    if result.returncode:
        raise RuntimeError(f"command failed: {command}; see {log}")
    return result.stdout


def unescape(text):
    return re.sub(r"\\([stnr\\])", lambda m: {
        "s": " ", "t": "\t", "n": "\n", "r": "\r", "\\": "\\"}[m[1]], text)


def corpus(profile):
    paths = [ROOT / "tests/fixtures/temporal/datetime-zulu.txt"]
    if profile == "direct":
        paths.append(ROOT / "tests/fixtures/temporal/datetime-direct.txt")
    for path in paths:
        for line in path.read_text().splitlines():
            if not line.startswith(("VALID ", "INVALID")):
                continue
            valid = line.startswith("VALID ")
            text = line.split(" ", 1)[1].strip() if " " in line else ""
            if valid:
                text, expected = text.split(" => ")
            else:
                expected = ""
            # The frozen named oracle rejects these six solely for lacking Z.
            # Their expected direct lexical forms are independently explicit.
            if profile == "direct" and path.name == "datetime-zulu.txt" and not valid:
                if text in ["2026-09-20T12:34:56", "2026-09-20T12:34:56+00:00",
                            "2026-09-20T12:34:56-00:00", "2026-09-20T12:34:56+05:00",
                            "2026-09-20T12:34:56-13:30", "2026-09-20T12:34:56+14:00"]:
                    valid, expected = True, text
            yield valid, unescape(text), unescape(expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    checked(["git", "rev-parse", "HEAD"], ROOT, scratch / "head.log")
    for name, expected in [
        ("datetime-zulu.txt", "3230a3270dc2239c53435235578c343248809c08d8d30af9b85a2392ae3c86c2"),
        ("datetime-direct.txt", "a0417fee4fbd1820618f60282200502fce543836e76dd1f952ac0612f33756a5"),
    ]:
        path = ROOT / "tests/fixtures/temporal" / name
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise RuntimeError("frozen DateTime oracle differs")
        shutil.copy2(path, scratch)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    options = ["--force"] if args.compiler == "14" else []
    if checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire version differs")
    for profile, fixture in [("named", "temporal"), ("direct", "direct-datetime")]:
        consumer = scratch / profile
        fixtures = ROOT / "tests/fixtures/service-generate"
        base = [str(args.generator.resolve()), "service-generate", "--schema",
                str(fixtures / f"{fixture}.xsd"), "--contract", str(fixtures / f"{fixture}.yaml"),
                "--language", "ada", "--world", "closed-schema"]
        model = scratch / f"{profile}-model"
        checked(base + ["--output", str(model)], ROOT, scratch / f"{profile}-model.log")
        checked(base + ["--with-codec", "--output", str(consumer)], ROOT,
                      scratch / f"{profile}-generate.log")
        for path in model.iterdir():
            if path.read_bytes() != (consumer / path.name).read_bytes():
                raise RuntimeError("model/API bytes changed")
        main_name = f"datetime_{profile}_consumer"
        shutil.copy2(ROOT / f"tests/ada-generated-codec/{main_name}.adb", consumer)
        golden = fixtures / f"task090-{profile}.json"
        shutil.copy2(golden, scratch)
        cases = list(corpus(profile))
        corpus_file = scratch / f"{profile}-corpus.txt"
        with corpus_file.open("w") as out:
            for valid, text, expected in cases:
                payload = {"{urn:test}Observed": text}
                if profile == "named":
                    payload["{urn:test}Label"] = "corpus"
                out.write(("VALID" if valid else "INVALID") + "\n" + json.dumps(payload) + "\n" + expected + "\n")
        (consumer / "alire.toml").write_text(
            'name = "task090_consumer"\nversion = "0.1.0-dev"\n'
            'description = "DateTime codec qualification"\nlicenses = "MIT"\n'
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
        checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"],
                      consumer, scratch / f"{profile}-sandbox.log")
        checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                       f"{compiler}={version}", "gprbuild=26.0.1"], consumer,
                      scratch / f"{profile}-toolchain.log")
        for policy in ["normal", "Check", "Ignore"]:
            command = ["alr", "-n", "build", "--", "-f"]
            if policy != "normal":
                adc = scratch / f"{policy}.adc"
                adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                command += ["-cargs:Ada", f"-gnatec={adc}"]
            checked(command, consumer, scratch / f"{profile}-{policy}-build.log")
            lock = consumer / "alire/alire.lock"
            guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
            shutil.copy2(lock, scratch / f"{profile}-qualified-alire.lock")
            for tool, identity in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
                output = checked(["alr", "-n", "exec", "--", tool, "--version"], consumer,
                                       scratch / f"{profile}-{policy}-{tool}.log")
                if identity.encode() not in output:
                    raise RuntimeError("compiler identity differs")
            output = checked([str(consumer / f"bin/{main_name}"), str(golden), str(corpus_file)],
                                   consumer, scratch / f"{profile}-{policy}-run.log")
            count = (22 if profile == "named" else 19) + sum(2 if valid else 1 for valid, _, _ in cases)
            marker = f"TASK090_{profile.upper()}_COMPLETE cases= {count}\n".encode()
            if output != marker or count <= 0:
                raise RuntimeError(f"completion/count differs: expected {marker!r}, got {output!r}")
    hashes = []
    for path in sorted(scratch.rglob("*")):
        if path.is_file() and path.suffix in [".ads", ".adb", ".json", ".lock", ".txt"]:
            hashes.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(scratch)}")
    hashes.append(f"{hashlib.sha256(args.generator.read_bytes()).hexdigest()}  generator")
    (scratch / "sources.sha256").write_text("\n".join(hashes) + "\n")
    print(f"TASK090 QUALIFICATION PASSED compiler={version} profiles=2 policies=3")


if __name__ == "__main__":
    main()