#!/usr/bin/env python3
"""Isolated GNAT/AHT build and real TLS caller-deadline qualification."""
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
from task112.qualify import PINS, IGNORE


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs():
    return {str(p.relative_to(ROOT)): digest(p) for folder in
            ("ada", "scripts/task116", "tests/ada-wss-deadline")
            for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts}


def execute(args):
    out = args.scratch.resolve()
    if out.is_relative_to(ROOT):
        raise ValueError("use external scratch")
    out.mkdir(parents=True, exist_ok=False)
    before = inputs()
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
    steps = []

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
                    os.killpg(process.pid, signal.SIGKILL); process.wait(timeout=5)
                    code = 124
        steps.append(dict(label=label, exit=code, timed_out=timed_out))
        (out / "steps.json").write_text(json.dumps(steps, indent=2) + "\n")
        if code:
            raise RuntimeError(f"{label} failed: {out / (label + '.stderr')}")
        return (out / (label + ".stdout")).read_text()

    source = run("source", ["git", "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()
    dirty = bool(run("source-status", ["git", "status", "--porcelain"]).strip())
    if dirty and not args.development:
        raise ValueError("acceptance requires clean committed source")
    identities = {}
    for name, pin in PINS.items():
        dep = args.deps.resolve() / name
        identity = run(name + "-identity", ["git", "-C", dep, "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()
        if identity[0] != pin or run(name + "-clean", ["git", "-C", dep, "status", "--porcelain"]).strip():
            raise ValueError("wrong or dirty dependency " + name)
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
    run("platform", shell(["sh", "-ec", "uname -a; gprbuild --version; openssl version; "
                          "sha256sum $(command -v gnat) $(command -v gcc) $(command -v gprbuild)"]))
    build = out / "build"
    shutil.copytree(ROOT / "ada", build / "ada", ignore=IGNORE)
    shutil.copytree(ROOT / "tests/ada-wss-deadline", build / "test")
    (build / "probe.gpr").write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\nproject Probe is\n'
        'for Source_Dirs use ("test"); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("deadline_probe.adb"); package Compiler is\n'
        'for Default_Switches ("Ada") use ("-gnat2022", "-gnata", "-gnato", "-O0"); end Compiler; end Probe;\n')
    run("build", shell(["gprbuild", "-p", "-j4", "-P", build / "probe.gpr"]))
    run("certificate", ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
                        "-keyout", out / "key.pem", "-out", out / "cert.pem", "-subj", "/CN=localhost",
                        "-addext", "subjectAltName=DNS:localhost"])
    (out / "key.pem").chmod(0o600)
    run("cases", [sys.executable, ROOT / "scripts/task116/peer.py", "--probe", build / "bin/deadline_probe",
                  "--cert", out / "cert.pem", "--key", out / "key.pem", "--evidence", out / "cases"])
    if before != inputs():
        raise ValueError("qualification sources changed")
    for name in PINS:
        if run(name + "-final-clean", ["git", "-C", args.deps / name, "status", "--porcelain"]).strip():
            raise ValueError("dependency dirtied " + name)
    result = dict(completed=not args.development, development=args.development, source=source,
                  compiler=compiler, backend=args.backend, dependencies=identities, inputs=before,
                  executable_sha256=digest(build / "bin/deadline_probe"), steps=steps)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    (out / "artifacts.json").write_text(json.dumps({str(p.relative_to(out)): digest(p)
        for p in out.rglob("*") if p.is_file() and p.name != "artifacts.json" and p.name != "key.pem"}, indent=2) + "\n")
    print("TASK116_COMPLETE", compiler, args.backend, flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "deps"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    parser.add_argument("--development", action="store_true")
    execute(parser.parse_args())