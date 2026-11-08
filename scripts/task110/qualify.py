#!/usr/bin/env python3
"""Authentic UCI 2.6 three-message verified WSS qualification.

Requires the existing Task096 Alire setup, pinned clean dependency checkouts,
and a freshly built production generator. Task108 safety is run separately.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task106 import qualify as task106


GENERATED = {
    'service_api.ads': 'abe3f9933fa2521b649b5bd3973eb09891bea757197933eae36a1594907700f6',
    'service_codec.adb': 'ca0ac6ebd5517df1075f023d9728ec29ed669ef4ef30486f1c82e1d8a4e9f632',
    'programs.ads': 'a5d92d26a18c03999588818e68c5a8b3426ff1c780a7582bf21121727b4a0d16',
    'service_codec.ads': '60bbec5768703bad5bdf276764b9db89837d0d7be918de2d23d793a8259731b1',
    'programs-oam.ads': 'd7fcf77ec9471b64b0ce18fc894a879c5dd44e21a79ddf94fc9b36119fcafb81',
    'programs-oam.adb': '7c7ff8289932666769da1f2d3c0f51c52c27c594a516c789f69a318963cd4ddf',
}


def inputs():
    paths = task106.inputs()
    for folder in ("scripts/task110", "tests/fixtures/task110", "tests/ada-real-uci-multimessage26",
                   "scripts/task109", "scripts/task108", "scripts/task105", "tests/fixtures/task105"):
        paths.update({str(p.relative_to(ROOT)): task106.digest(p) for p in (ROOT / folder).rglob("*")
                      if p.is_file() and "__pycache__" not in p.parts})
    return paths


def main():
    from task110 import admission

    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "aht", "codec", "sleet", "uci26", "uci-release", "generator"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    parser.add_argument("--development", action="store_true", help="dirty-source diagnostics, not acceptance")
    args = parser.parse_args()
    out, setup = args.scratch.resolve(), args.setup.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
    before = inputs()
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
    identity = run("uci-release-identity", ["git", "-C", args.uci_release, "rev-parse", "HEAD", "HEAD^{tree}"])
    if (identity.splitlines()[0] != "78eb61b6112c8bffa40820c33124b57787fc5bd9" or
            run("uci-release-clean", ["git", "-C", args.uci_release, "status", "--porcelain"]).strip()):
        raise ValueError("wrong/dirty UCI release")
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
    (out / "uci26").mkdir()
    for name, pin in admission.PINS.items():
        source = args.uci26.resolve().parent / name
        if task106.digest(source) != pin:
            raise ValueError("authentic UCI schema changed")
        shutil.copyfile(source, out / "uci26" / name)
    (out / "schema-sources.json").write_text(json.dumps(admission.PINS, indent=2) + "\n")
    schema = out / "uci26/UCI_MessageDefinitions_v2_6_0.xsd"
    common = ["--schema", schema, "--contract", ROOT / "tests/fixtures/task110/uci26-service.yaml",
              "--language", "ada", "--world", "closed-schema"]
    report = run("check", [generator, "service-check", *common, "--with-codec"])
    for text in ("selected oms messages: 3\n", "renderable selected oms messages: 3\n",
                 "selected type closure: 65\n", "renderable selected types: 65\n", "status: READY\n",
                 "codec renderable emitted declarations: 64/64\ncodec status: READY\n"):
        if text not in report:
            raise ValueError("combined service readiness changed")
    run("generate", [generator, "service-generate", *common, "--with-codec", "--output", out / "generated"])
    run("model", [generator, "service-generate", *common, "--output", out / "model"])
    hashes = {p.name: task106.digest(p) for p in (out / "generated").iterdir()}
    if hashes != GENERATED or any(p.read_bytes() != (out / "generated" / p.name).read_bytes()
                                         for p in (out / "model").iterdir()):
        raise ValueError("Task110 generated identity changed")
    (out / "generated-sources.json").write_text(json.dumps(hashes, indent=2) + "\n")
    parity = {}
    for number, key, payload in ((102, "log", "LogMT"), (103, "rdma", "RDMA_InitializeSetupMT"),
                                 (105, "survivability", "SurvivabilityRiskLevelMT")):
        baseline = out / ("baseline-" + key)
        run("baseline-" + key, [generator, "service-generate", "--schema", schema,
            "--contract", ROOT / f"tests/fixtures/task{number}/uci26-service.yaml", "--language", "ada",
            "--world", "closed-schema", "--with-codec", "--output", baseline])
        pins = {p.name: task106.digest(p) for p in baseline.iterdir()}
        if key == "survivability":
            from task105_evidence import BASELINE
            if any(pins.get(name) != pin for name, pin in BASELINE.items()):
                raise ValueError("independently qualified Task105 baseline changed")
        # Compare every record representation in the independently generated
        # family, including nested types, not whole-union source equality.
        pattern = r"^   type (\w+) is record\n.*?^   end record;"
        records = {}
        combined = (out / "generated/programs-oam.ads").read_text()
        for match in re.finditer(pattern, (baseline / "programs-oam.ads").read_text(), re.M | re.S):
            if match.group(0) not in combined:
                raise ValueError("combined representation differs: " + key + "/" + match.group(1))
            records[match.group(1)] = hashlib.sha256(match.group(0).encode()).hexdigest()
        if payload not in records:
            raise ValueError("missing family payload record")
        parity[key] = dict(payload=payload, records=records, baseline_hashes=pins)
    (out / "representation-parity.json").write_text(json.dumps(parity, indent=2) + "\n")
    run("admission", [sys.executable, ROOT / "scripts/task110/admission.py", "--evidence", out / "admission",
                      "--sleet", args.sleet.resolve(), "--uci26", schema])
    build = out / "build"
    shutil.copytree(ROOT / "ada", build / "ada", ignore=ignore)
    shutil.copytree(out / "generated", build / "generated")
    original = (ROOT / "tests/ada-real-uci-multimessage26/multimessage_consumer.adb").read_text()
    consumer = build / "multimessage_consumer.adb"
    consumer.write_text(original)
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
        "--evidence", out / "tls", "--schema-version", "002.6.0", "--service-id", "task110-multimessage"])) + ' "$@"\n')
    adapter.chmod(0o755)
    base = [sys.executable, ROOT / "scripts/task110/sleet.py", "--checkout", args.sleet.resolve(), "--schema", schema]
    run("wss", [*base, "--evidence", out / "wss", "--consumer", adapter])
    run("verify", [sys.executable, ROOT / "scripts/task110/verify.py", out])
    for name, source in (("aht", args.aht), ("codec", args.codec), ("sleet", args.sleet)):
        if run(name + "-final-clean", ["git", "-C", source, "status", "--porcelain"]).strip():
            raise ValueError("dependency dirtied during qualification: " + name)
    if any(task106.digest(ROOT / path) != pin for path, pin in before.items()):
        raise ValueError("qualification source changed during execution")
    (out / "sources.json").write_text(json.dumps({str(p.relative_to(out)): task106.digest(p)
        for p in out.rglob("*") if p.is_file() and p.suffix in (".ads", ".adb", ".gpr", ".xsd", ".toml")}, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(dict(completed=not args.development, development=args.development,
        compiler=version, backend=args.backend,
        authentic_uci="2.6", steps=steps, needs_read_runtime_exercised=False), indent=2) + "\n")
    print("TASK110_QUALIFICATION_COMPLETE", version, args.backend, flush=True)


if __name__ == "__main__":
    main()