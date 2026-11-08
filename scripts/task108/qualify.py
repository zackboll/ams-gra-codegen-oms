#!/usr/bin/env python3
"""Frozen-input WSS qualification, isolated compiler/backend build directories.

Requires an existing Alire GNATCOLL setup as used by TASK100, exact clean AHT
and websocket-ada checkouts, and (for mbedTLS) AHT's provisioned third_party tree.
Does not edit or build in those source checkouts. No insecure TLS fixture.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
AHT = "827b0c17dd964e935f8e88912ce0611ee92b6f16"
CODEC = "765d9cd612838eb00645037977c8cc9af7171c3e"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["scratch", "setup", "aht", "codec", "generator", "sleet"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--backend", choices=["openssl", "mbedtls"], default="openssl")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
    steps = []
    inputs = {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
              for folder in ["ada", "scripts/task108", "scripts/task100", "tests/ada-wss", "tests/ada-websocket"]
              for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts}
    inputs[str(args.generator.resolve())] = hashlib.sha256(args.generator.read_bytes()).hexdigest()

    def run(label, command, cwd=ROOT):
        command = list(map(str, command))
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as stdout, (scratch / f"{label}.stderr").open("wb") as stderr:
            result = subprocess.run(command, cwd=cwd, env=env, stdout=stdout, stderr=stderr, timeout=300)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        steps.append({"label": label, "exit": result.returncode})
        assert result.returncode == 0, f"{label}: {scratch / (label + '.stderr')}"
        return (scratch / f"{label}.stdout").read_text()

    run("source", ["git", "-C", ROOT, "rev-parse", "HEAD", "HEAD^{tree}"])
    run("source-status", ["git", "-C", ROOT, "status", "--porcelain"])
    for name, source, revision in [("aht", args.aht, AHT), ("codec", args.codec, CODEC)]:
        assert run(name + "-identity", ["git", "-C", source, "rev-parse", "HEAD"]).strip() == revision
        assert not run(name + "-clean", ["git", "-C", source, "status", "--porcelain"]).strip()
        dest = scratch / name
        dest.mkdir()
        # Archive only committed immutable sources, excluding existing objects.
        with subprocess.Popen(["git", "-C", str(source), "archive", revision], stdout=subprocess.PIPE) as archive:
            subprocess.run(["tar", "-x", "-C", str(dest)], stdin=archive.stdout, check=True)
            assert archive.wait() == 0
    if args.backend == "mbedtls":
        shutil.copytree(args.aht / "third_party/mbedtls-3.6.7", scratch / "aht/third_party/mbedtls-3.6.7")
    setup = args.setup.resolve()
    setup_env = dict(env, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                     XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
    printed = subprocess.check_output(["alr", "-s", str(setup / "settings"), "-n", "printenv"], cwd=setup / "consumer", env=setup_env, text=True)
    provider = setup / "consumer/alire/cache/dependencies"
    ignore = shutil.ignore_patterns("obj", "obj-*", "bin", "lib", ".git", "__pycache__", "*.o", "*.ali")
    shutil.copytree(provider, scratch / "provider", ignore=ignore)
    environment = scratch / "environment.sh"
    environment.write_text(printed.replace(str(provider), str(scratch / "provider")) +
        f'\nexport GPR_PROJECT_PATH={shlex.quote(str(scratch / "aht"))}:"$GPR_PROJECT_PATH"\n')
    env["OMS_WS_CODEC_SOURCE"] = str(scratch / "codec")

    def shell(command):
        return ["sh", "-ec", ". " + shlex.quote(str(environment)) + "; exec " + shlex.join(list(map(str, command)))]

    assert run("compiler", shell(["gnat", "--version"])).startswith("GNAT " + ("14.2.0" if args.compiler == "14" else "16.1.0") + "\n")
    run("platform", shell(["sh", "-ec", "uname -a; gprbuild --version; openssl version; sha256sum $(command -v gcc) $(command -v gprbuild)"]))
    build = scratch / "build"
    shutil.copytree(ROOT / "ada", build / "ada", ignore=ignore)
    shutil.copytree(ROOT / "tests/ada-websocket", build / "test")
    for path in (ROOT / "tests/ada-wss").glob("*.adb"):
        shutil.copy(path, build / "test")
    common = [args.generator.resolve(), "service-generate", "--schema", ROOT / "tests/fixtures/service-generate/lacal-websocket.xsd",
              "--contract", ROOT / "tests/fixtures/service-generate/lacal-runtime.yaml", "--language", "ada", "--world", "closed-schema"]
    run("generate-model", [*common, "--output", scratch / "model"])
    run("generate-codec", [*common, "--with-codec", "--output", build / "generated"])
    assert all(p.read_bytes() == (build / "generated" / p.name).read_bytes() for p in (scratch / "model").iterdir())
    consumer = build / "test/generated_consumer.adb"
    consumer.write_text(consumer.read_text().replace("TASK100_PACKAGE", "Programs.Oam"))
    (build / "consumer.gpr").write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\nproject Consumer is\n'
        'for Source_Dirs use ("generated", "test"); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("aht_probe.adb", "wss_probe.adb", "transport_probe.adb", "generated_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnata", "-gnato", "-O0"); end Compiler; end Consumer;\n')
    run("build", shell(["gprbuild", "-p", "-j4", "-P", build / "consumer.gpr"]))
    shim = scratch / "control_backpressure.so"
    run("shim", ["cc", "-shared", "-fPIC", "-Wall", "-Wextra", "-Werror", "-O2", ROOT / "tests/ada-wss/control_backpressure.c", "-o", shim])
    control_peer = scratch / "tls_control_peer"
    run("control-peer-build", ["cc", "-Wall", "-Wextra", "-Werror", "-O2", ROOT / "tests/ada-wss/tls_control_peer.c", "-lssl", "-lcrypto", "-o", control_peer])
    run("api", ["python3", ROOT / "scripts/task108/aht_consumer.py", "--probe", build / "bin/aht_probe", "--evidence", scratch / "api"])
    tls = ["--cert", scratch / "api/cert.pem", "--key", scratch / "api/key.pem"]
    run("control", ["python3", ROOT / "scripts/task108/tls_control.py", "--probe", build / "bin/aht_probe", *tls, "--evidence", scratch / "control"])
    run("poll", ["python3", ROOT / "scripts/task108/wss_peer.py", "--probe", build / "bin/wss_probe", *tls, "--shim", shim, "--evidence", scratch / "poll"])
    run("poll-control", ["python3", ROOT / "scripts/task108/poll_control.py", "--probe", build / "bin/wss_probe", "--peer", control_peer, *tls, "--evidence", scratch / "poll-control"])
    run("wss-regression", ["python3", ROOT / "scripts/task108/wss_regression.py", "--probe", build / "bin/transport_probe", "--consumer", build / "bin/generated_consumer", *tls, "--evidence", scratch / "wss-regression"])
    run("sleet", ["python3", ROOT / "scripts/task100/real_sleet.py", "--checkout", args.sleet.resolve(), "--consumer", build / "bin/generated_consumer",
                  "--tls-cert", scratch / "api/cert.pem", "--tls-key", scratch / "api/key.pem", "--evidence", scratch / "sleet"])
    assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest() == digest for p, digest in inputs.items()), "inputs changed during campaign"
    (scratch / "input-sources.json").write_text(json.dumps(inputs, indent=2) + "\n")
    sources = {str(p.relative_to(scratch)): hashlib.sha256(p.read_bytes()).hexdigest()
               for p in scratch.rglob("*") if p.is_file() and p.suffix in {".ads", ".adb", ".gpr", ".c", ".h", ".toml", ".xsd"}}
    (scratch / "sources.json").write_text(json.dumps(sources, indent=2) + "\n")
    (scratch / "result.json").write_text(json.dumps({"completed": True, "compiler": args.compiler, "backend": args.backend, "aht": AHT, "codec": CODEC, "steps": steps,
        "needs_read_runtime_exercised": False}, indent=2) + "\n")
    print("TASK108R1_QUALIFICATION_COMPLETE", args.compiler, args.backend)


if __name__ == "__main__":
    main()