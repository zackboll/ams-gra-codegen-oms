#!/usr/bin/env python3
"""Build the standalone Ada client; no qualification runner is required."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
PINS = dict(aht="827b0c17dd964e935f8e88912ce0611ee92b6f16",
            codec="765d9cd612838eb00645037977c8cc9af7171c3e",
            sleet="e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b",
            uci25="093610b7753944059360d3236770ab446d039556",
            uci26="78eb61b6112c8bffa40820c33124b57787fc5bd9")
SCHEMAS = {
    "25": {"UCI_MessageDefinitions_v2_5_0.xsd": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
           "UCI_SecurityMarkings_v2_5_0.xsd": "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b"},
    "26": {"UCI_MessageDefinitions_v2_6_0.xsd": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
           "UCI_SecurityMarkings_v2_6_0.xsd": "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88"}}
IGNORE = shutil.ignore_patterns("obj", "obj-*", "bin", "lib", ".git", "__pycache__", "*.o", "*.ali")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs():
    return {str(p.relative_to(ROOT)): digest(p) for folder in
            ("examples/ada-uci-client", "scripts/task115", "ada")
            for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts}


def build(args):
    out = args.scratch.resolve()
    if out.is_relative_to(ROOT):
        raise ValueError("scratch must be outside the repository")
    out.mkdir(parents=True, exist_ok=False)
    before = inputs()
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
    steps = []

    def run(label, command, cwd=ROOT, extra_env=None):
        argv = list(map(str, command))
        (out / (label + ".command.json")).write_text(json.dumps(dict(argv=argv, cwd=str(cwd))) + "\n")
        with (out / (label + ".stdout")).open("w") as stdout, (out / (label + ".stderr")).open("w") as stderr:
            result = subprocess.run(argv, cwd=cwd, env=extra_env or env,
                                    stdout=stdout, stderr=stderr, timeout=300, check=False)
        steps.append(dict(label=label, exit=result.returncode))
        (out / "steps.json").write_text(json.dumps(steps, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(f"{label} failed; see {out / (label + '.stderr')}")
        return (out / (label + ".stdout")).read_text()

    source = run("source", ["git", "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()
    status = run("source-status", ["git", "status", "--porcelain"])
    identities = {}
    for name, pin in PINS.items():
        dep = args.deps.resolve() / name
        identity = run(name + "-identity", ["git", "-C", dep, "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()
        if identity[0] != pin or run(name + "-clean", ["git", "-C", dep, "status", "--porcelain"]).strip():
            raise ValueError("wrong or dirty dependency: " + name)
        identities[name] = identity
        if name in ("aht", "codec"):
            run(name + "-archive", ["git", "-C", dep, "archive", "--output", out / (name + ".tar"), pin])
            (out / name).mkdir()
            run(name + "-extract", ["tar", "-xf", out / (name + ".tar"), "-C", out / name])
    if args.backend == "mbedtls":
        shutil.copytree(args.deps / "aht/third_party/mbedtls-3.6.7", out / "aht/third_party/mbedtls-3.6.7")
    setup = args.setup.resolve()
    setup_env = dict(env, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                     XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
    printed = run("printenv", ["alr", "-s", setup / "settings", "-n", "printenv"], setup / "consumer", setup_env)
    provider = setup / "consumer/alire/cache/dependencies"
    shutil.copytree(provider, out / "provider", ignore=IGNORE)
    environment = out / "environment.sh"
    environment.write_text(printed.replace(str(provider), str(out / "provider")) +
                          '\nexport GPR_PROJECT_PATH=' + shlex.quote(str(out / "aht")) + ':"$GPR_PROJECT_PATH"\n')
    env["OMS_WS_CODEC_SOURCE"] = str(out / "codec")
    shutil.copyfile(setup / "consumer/alire/alire.lock", out / "alire.lock")

    def shell(command):
        return ["sh", "-ec", ". " + shlex.quote(str(environment)) + "; exec " + shlex.join(list(map(str, command)))]

    compiler = run("compiler", shell(["gnat", "--version"])).splitlines()[0]
    if compiler != "GNAT " + {"14": "14.2.0", "16": "16.1.0"}[args.compiler]:
        raise ValueError("wrong compiler")
    run("platform", shell(["sh", "-ec", "uname -a; gprbuild --version; openssl version"]))
    schema_dir = out / "schema"
    schema_dir.mkdir()
    for name, pin in SCHEMAS[args.release].items():
        original = args.schema.resolve() / name
        if digest(original) != pin:
            raise ValueError("authentic schema hash mismatch: " + name)
        shutil.copyfile(original, schema_dir / name)
    schema = schema_dir / next(iter(SCHEMAS[args.release]))
    contract = ROOT / f"examples/ada-uci-client/contracts/uci{args.release}-service.yaml"
    common = ["--schema", schema, "--contract", contract, "--language", "ada", "--world", "closed-schema", "--with-codec"]
    run("service-check", [args.generator.resolve(), "service-check", *common])
    run("generate", [args.generator.resolve(), "service-generate", *common, "--output", out / "generated"])
    build_dir = out / "build"
    shutil.copytree(ROOT / "ada", build_dir / "ada", ignore=IGNORE)
    shutil.copytree(ROOT / "examples/ada-uci-client/src", build_dir / "src")
    shutil.copytree(out / "generated", build_dir / "generated")
    (build_dir / "src/uci_client-build_config.ads").write_text(
        'package UCI_Client.Build_Config is\n   Schema : constant String := "002.' + args.release[-1] + '.0";\nend UCI_Client.Build_Config;\n')
    (build_dir / "client.gpr").write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\n'
        'with "ada/oms-json-support/ams_gra_oms_json";\nproject Client is\n'
        'for Source_Dirs use ("src", "generated"); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("ada_uci_client.adb"); package Builder is\n'
        'for Executable ("ada_uci_client.adb") use "ada-uci-client"; end Builder;\npackage Compiler is\n'
        'for Default_Switches ("Ada") use ("-gnat2022", "-gnata", "-gnato", "-O0"); end Compiler; end Client;\n')
    run("build", shell(["gprbuild", "-p", "-j4", "-P", build_dir / "client.gpr"]))
    if inputs() != before:
        raise ValueError("application/build inputs changed during build")
    result = dict(source=source, dirty=bool(status.strip()), inputs=before, dependencies=identities,
                  compiler=compiler, backend=args.backend, release=args.release,
                  schemas=SCHEMAS[args.release], generator_sha256=digest(args.generator),
                  generated={p.name: digest(p) for p in (out / "generated").iterdir()},
                  executable_sha256=digest(build_dir / "bin/ada-uci-client"))
    (out / "build.json").write_text(json.dumps(result, indent=2) + "\n")
    print(build_dir / "bin/ada-uci-client", flush=True)


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "deps", "schema", "generator"):
        result.add_argument("--" + name, type=Path, required=True)
    result.add_argument("--release", choices=("25", "26"), required=True)
    result.add_argument("--compiler", choices=("14", "16"), required=True)
    result.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    return result


if __name__ == "__main__":
    build(parser().parse_args())