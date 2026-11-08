#!/usr/bin/env python3
"""Focused authentic UCI WSS cell; Task106 remains the delivery authority.

Requires the existing Task096 Alire setup, pinned clean dependency checkouts,
and a freshly built production generator. Task108 safety is run separately.
"""
import argparse
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
sys.path.insert(0, str(ROOT / "scripts/task106"))
import qualify as task106


def consumer_source(text):
    """Only adapt the scratch copy's CA input; preserve every Task106 assertion."""
    old = "   WS.Open (Wire, Ada.Command_Line.Argument (1));"
    if text.count(old) != 1:
        raise ValueError("Task106 Open shape changed")
    return "with Ada.Environment_Variables;\n" + text.replace(old,
        '   WS.Open (Wire, Ada.Command_Line.Argument (1),\n'
        '     CA_Path => Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA", ""));')


def main():
    import admission

    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "aht", "codec", "sleet", "uci25", "generator"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    parser.add_argument("--plaintext", action="store_true", help="also run unchanged Task106 consumer/driver")
    parser.add_argument("--development", action="store_true", help="dirty-source diagnostics, not acceptance")
    args = parser.parse_args()
    out, setup = args.scratch.resolve(), args.setup.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
    before = task106.inputs()
    before.update({str(p.relative_to(ROOT)): task106.digest(p)
                   for p in (ROOT / "scripts/task109").glob("*.py")})
    (out / "input-sources.json").write_text(json.dumps(before, indent=2) + "\n")
    steps = []

    def run(label, command, cwd=ROOT, extra_env=None):
        argv = list(map(str, command))
        (out / (label + ".command.json")).write_text(json.dumps(dict(argv=argv, cwd=str(cwd))) + "\n")
        timed_out = False
        with (out / (label + ".stdout")).open("wb") as stdout, (out / (label + ".stderr")).open("wb") as stderr:
            with subprocess.Popen(argv, cwd=cwd, env=extra_env or env, stdout=stdout, stderr=stderr,
                                  start_new_session=True) as process:
                try:
                    code = process.wait(timeout=300)
                except subprocess.TimeoutExpired:
                    timed_out = True
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        pass
                    # Even if the direct child exited, kill any descendants
                    # retaining sockets/tasks after a campaign timeout.
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait(timeout=5)
                    code = 124
        (out / (label + ".exit")).write_text(str(code) + "\n")
        steps.append(dict(label=label, exit=code, timed_out=timed_out))
        if code:
            raise RuntimeError(f"{label} failed: {out / (label + '.stderr')}")
        return (out / (label + ".stdout")).read_text()

    run("source", ["git", "rev-parse", "HEAD", "HEAD^{tree}"])
    if run("source-status", ["git", "status", "--porcelain"]).strip() and not args.development:
        raise ValueError("acceptance requires clean OMS sources")
    for name, source, revision in (("aht", args.aht, task106.DEPS["aht"][1]),
                                   ("codec", args.codec, task106.DEPS["websocket-ada"][1]),
                                   ("sleet", args.sleet, admission.REV)):
        identity = run(name + "-identity", ["git", "-C", source, "rev-parse", "HEAD", "HEAD^{tree}"])
        if identity.splitlines()[0] != revision or run(name + "-clean", ["git", "-C", source, "status", "--porcelain"]).strip():
            raise ValueError("wrong/dirty dependency: " + name)
        if name != "sleet":
            archive = out / (name + ".tar")
            run(name + "-archive", ["git", "-C", source, "archive", "--output", archive, revision])
            (out / name).mkdir()
            run(name + "-extract", ["tar", "-xf", archive, "-C", out / name])
    if args.backend == "mbedtls":
        shutil.copytree(args.aht / "third_party/mbedtls-3.6.7", out / "aht/third_party/mbedtls-3.6.7")
    setup_env = dict(env, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                     XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
    printed = run("printenv", ["alr", "-s", setup / "settings", "-n", "printenv"], setup / "consumer", setup_env)
    ignore = shutil.ignore_patterns("obj", "obj-*", "bin", "lib", ".git", "__pycache__", "*.o", "*.ali")
    provider = setup / "consumer/alire/cache/dependencies"
    shutil.copytree(provider, out / "provider", ignore=ignore)
    environment = out / "environment.sh"
    environment.write_text(printed.replace(str(provider), str(out / "provider")) +
        f'\nexport GPR_PROJECT_PATH={shlex.quote(str(out / "aht"))}:"$GPR_PROJECT_PATH"\n')
    env["OMS_WS_CODEC_SOURCE"] = str(out / "codec")
    shutil.copyfile(setup / "consumer/alire/alire.lock", out / "alire.lock")

    def shell(command):
        return ["sh", "-ec", ". " + shlex.quote(str(environment)) + "; exec " + shlex.join(map(str, command))]

    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    if not run("compiler", shell(["gnat", "--version"])).startswith("GNAT " + version + "\n"):
        raise ValueError("wrong compiler")
    run("platform", shell(["sh", "-ec", "uname -a; getconf GNU_LIBC_VERSION; gprbuild --version; openssl version; "
                          "sha256sum $(command -v gcc) $(command -v gnat) $(command -v gprbuild)"]))
    generator = args.generator.resolve()
    (out / "generator.sha256").write_text(task106.digest(generator) + "\n")
    (out / "uci25").mkdir()
    for name, pin in admission.PINS.items():
        source = args.uci25.resolve().parent / name
        if task106.digest(source) != pin:
            raise ValueError("authentic UCI schema changed")
        shutil.copyfile(source, out / "uci25" / name)
    (out / "schema-sources.json").write_text(json.dumps(admission.PINS, indent=2) + "\n")
    schema = out / "uci25/UCI_MessageDefinitions_v2_5_0.xsd"
    common = ["--schema", schema, "--contract", ROOT / "tests/fixtures/task106/uci25-service.yaml",
              "--language", "ada", "--world", "closed-schema"]
    report = run("check", [generator, "service-check", *common, "--with-codec"])
    for text in ("selected oms messages: 3\n", "selected type closure: 64\n",
                 "codec renderable emitted declarations: 63/63\ncodec status: READY\n"):
        if text not in report:
            raise ValueError("combined service readiness changed")
    run("generate", [generator, "service-generate", *common, "--with-codec", "--output", out / "generated"])
    run("model", [generator, "service-generate", *common, "--output", out / "model"])
    hashes = {p.name: task106.digest(p) for p in (out / "generated").iterdir()}
    if hashes != task106.GENERATED or any(p.read_bytes() != (out / "generated" / p.name).read_bytes()
                                         for p in (out / "model").iterdir()):
        raise ValueError("Task106 generated identity changed")
    (out / "generated-sources.json").write_text(json.dumps(hashes, indent=2) + "\n")
    run("admission", [sys.executable, ROOT / "scripts/task106/admission.py", "--evidence", out / "admission",
                      "--sleet", args.sleet.resolve(), "--uci25", schema])
    build = out / "build"
    shutil.copytree(ROOT / "ada", build / "ada", ignore=ignore)
    shutil.copytree(out / "generated", build / "generated")
    original = (ROOT / "tests/ada-real-uci-multimessage/multimessage_consumer.adb").read_text()
    consumer = build / "multimessage_consumer.adb"
    consumer.write_text(consumer_source(original))
    (build / "consumer.gpr").write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\n'
        'with "ada/oms-json-support/ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("generated", "."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("multimessage_consumer.adb");\npackage Compiler is\n'
        'for Default_Switches ("Ada") use ("-gnat2022", "-gnata", "-gnato", "-O0"); end Compiler; end Consumer;\n')
    run("build", shell(["gprbuild", "-p", "-j4", "-P", build / "consumer.gpr"]))
    run("certificate", ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", out / "key.pem",
                        "-out", out / "cert.pem", "-days", "2", "-subj", "/CN=localhost",
                        "-addext", "subjectAltName=DNS:localhost"])
    adapter = out / "wss-consumer"
    adapter.write_text("#!/bin/sh\nexec " + shlex.join(map(str, [sys.executable, ROOT / "scripts/task109/relay.py",
        "--consumer", build / "bin/multimessage_consumer", "--cert", out / "cert.pem", "--key", out / "key.pem",
        "--evidence", out / "tls"])) + ' "$@"\n')
    adapter.chmod(0o755)
    base = [sys.executable, ROOT / "scripts/task106/sleet.py", "--checkout", args.sleet.resolve(), "--schema", schema]
    run("wss", [*base, "--evidence", out / "wss", "--consumer", adapter])
    if args.plaintext:
        # Rebuild the original source, not the CA-adapted version, for regression.
        consumer.write_text(original)
        run("plaintext-build", shell(["gprbuild", "-p", "-j4", "-P", build / "consumer.gpr"]))
        run("plaintext", [*base, "--evidence", out / "plaintext", "--consumer", build / "bin/multimessage_consumer"])
        consumer.write_text(consumer_source(original))
    run("verify", [sys.executable, ROOT / "scripts/task109/verify.py", out])
    for name, source in (("aht", args.aht), ("codec", args.codec), ("sleet", args.sleet)):
        if run(name + "-final-clean", ["git", "-C", source, "status", "--porcelain"]).strip():
            raise ValueError("dependency dirtied during qualification: " + name)
    if any(task106.digest(ROOT / path) != pin for path, pin in before.items()):
        raise ValueError("qualification source changed during execution")
    (out / "sources.json").write_text(json.dumps({str(p.relative_to(out)): task106.digest(p)
        for p in out.rglob("*") if p.is_file() and p.suffix in (".ads", ".adb", ".gpr", ".xsd", ".toml")}, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(dict(completed=not args.development, development=args.development,
        compiler=version, backend=args.backend,
        authentic_uci="2.5", plaintext=args.plaintext, steps=steps, needs_read_runtime_exercised=False), indent=2) + "\n")
    print("TASK109_QUALIFICATION_COMPLETE", version, args.backend, flush=True)


if __name__ == "__main__":
    main()