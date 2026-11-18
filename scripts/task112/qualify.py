#!/usr/bin/env python3
"""Isolated immutable-dependency build and established-session recovery cell."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task106 import qualify as task106
from task110 import qualify as task110

PINS = dict(aht="827b0c17dd964e935f8e88912ce0611ee92b6f16",
            codec="765d9cd612838eb00645037977c8cc9af7171c3e",
            sleet="94796daf55ebd1134dce0925e6a204a05ad3423b",
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


def inputs(test_directory="tests/ada-wss-session-recovery"):
    return {str(p.relative_to(ROOT)): digest(p) for folder in
            ("ada", "scripts", test_directory, "tests/fixtures")
            for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts}


def execute(args, profile=None):
    test_directory = getattr(profile, "TEST_DIRECTORY", "tests/ada-wss-session-recovery")
    recovery_script = getattr(profile, "RECOVERY_SCRIPT", "scripts/task112/recovery.py")
    task_name = getattr(profile, "TASK_NAME", "TASK112")
    out = args.scratch.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
    steps = []
    before = inputs(test_directory)
    (out / "input-sources.json").write_text(json.dumps(before, indent=2) + "\n")

    def run(label, command, cwd=ROOT, extra_env=None):
        argv = list(map(str, command))
        (out / (label + ".command.json")).write_text(json.dumps(dict(argv=argv, cwd=str(cwd))) + "\n")
        with (out / (label + ".stdout")).open("wb") as stdout, (out / (label + ".stderr")).open("wb") as stderr:
            with subprocess.Popen(argv, cwd=cwd, env=extra_env or env, stdout=stdout, stderr=stderr,
                                  start_new_session=True) as process:
                timed_out = False
                try:
                    code = process.wait(timeout=300)
                except subprocess.TimeoutExpired:
                    timed_out = True
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
                    code = 124
        steps.append(dict(label=label, exit=code, timed_out=timed_out))
        (out / (label + ".exit")).write_text(str(code) + "\n")
        (out / "steps.json").write_text(json.dumps(steps, indent=2) + "\n")
        if code:
            raise RuntimeError(f"{label} failed: {out / (label + '.stderr')}")
        return (out / (label + ".stdout")).read_text()

    source = run("source", ["git", "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()
    if run("source-status", ["git", "status", "--porcelain"]).strip() and not args.development:
        raise ValueError("acceptance requires clean source")
    for name, pin in PINS.items():
        dependency = args.deps.resolve() / name
        if run(name + "-identity", ["git", "-C", dependency, "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()[0] != pin:
            raise ValueError("wrong dependency " + name)
        if run(name + "-clean", ["git", "-C", dependency, "status", "--porcelain"]).strip():
            raise ValueError("dirty dependency " + name)
        if name in ("aht", "codec"):
            run(name + "-archive", ["git", "-C", dependency, "archive", "--output", out / (name + ".tar"), pin])
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

    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    if not run("compiler", shell(["gnat", "--version"])).startswith("GNAT " + version + "\n"):
        raise ValueError("wrong compiler")
    run("platform", shell(["sh", "-ec", "uname -a; getconf GNU_LIBC_VERSION; gprbuild --version; "
        "openssl version; sha256sum $(command -v gnat) $(command -v gcc) $(command -v gprbuild)"]))
    (out / "generator.sha256").write_text(digest(args.generator) + "\n")
    (out / "sleet-binary.sha256").write_text(digest(args.deps / "sleet/target/release/sleet") + "\n")
    run("certificate", ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
        "-keyout", out / "key.pem", "-out", out / "cert.pem", "-subj", "/CN=localhost",
        "-addext", "subjectAltName=DNS:localhost"])
    (out / "key.pem").chmod(0o600)
    for release in args.release:
        schema_dir = out / ("uci" + release)
        schema_dir.mkdir()
        for name, pin in SCHEMAS[release].items():
            original = getattr(args, "schema" + release).resolve() / name
            if digest(original) != pin:
                raise ValueError("changed authentic schema")
            shutil.copyfile(original, schema_dir / name)
        schema = schema_dir / next(iter(SCHEMAS[release]))
        task = "110" if release == "26" else "106"
        generated = out / ("generated" + release)
        run("generate" + release, [args.generator.resolve(), "service-generate", "--schema", schema,
            "--contract", ROOT / f"tests/fixtures/task{task}/uci{release}-service.yaml", "--language", "ada",
            "--world", "closed-schema", "--with-codec", "--output", generated])
        pins = task110.GENERATED if release == "26" else task106.GENERATED
        if {p.name: digest(p) for p in generated.iterdir()} != pins:
            raise ValueError("generated model/codec changed")
        (out / ("generated" + release + "-sources.json")).write_text(json.dumps(pins, indent=2) + "\n")
        build = out / ("build" + release)
        shutil.copytree(ROOT / "ada", build / "ada", ignore=IGNORE)
        shutil.copytree(ROOT / test_directory, build / "test")
        shutil.copytree(generated, build / "generated")
        (build / "consumer.gpr").write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\n'
            'with "ada/oms-json-support/ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("test", "generated"); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            'for Main use ("recovery_consumer.adb"); package Compiler is\n'
            'for Default_Switches ("Ada") use ("-gnat2022", "-gnata", "-gnato", "-O0"); end Compiler; end Consumer;\n')
        run("build" + release, shell(["gprbuild", "-p", "-j4", "-P", build / "consumer.gpr"]))
        run("recovery" + release, [sys.executable, ROOT / recovery_script, "--release", release,
            "--evidence", out / ("recovery" + release), "--consumer", build / "bin/recovery_consumer",
            "--sleet", args.deps / "sleet", "--schema", schema, "--cert", out / "cert.pem", "--key", out / "key.pem"])
    for name in PINS:
        if run(name + "-final-clean", ["git", "-C", args.deps / name, "status", "--porcelain"]).strip():
            raise ValueError("dependency dirtied " + name)
    if inputs(test_directory) != before:
        raise ValueError("sources changed during qualification")
    result = dict(completed=not args.development, development=args.development, compiler=version,
                  backend=args.backend, releases=args.release, source=source, steps=steps)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    (out / "artifacts.json").write_text(json.dumps({str(p.relative_to(out)): digest(p) for p in out.rglob("*")
        if p.is_file() and p.name != "artifacts.json"}, indent=2) + "\n")
    print(task_name + "_QUALIFICATION_COMPLETE", version, args.backend, flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "deps", "schema25", "schema26", "generator"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    parser.add_argument("--release", choices=("25", "26"), nargs="+", default=["26"])
    parser.add_argument("--development", action="store_true", help="diagnostic only, not acceptance")
    execute(parser.parse_args())