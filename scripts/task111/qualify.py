#!/usr/bin/env python3
"""Isolated, pinned builds and opt-in CA_PEM qualification (Linux)."""
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


def consumer_source(text, release):
    """Change only Open in a scratch copy; retain all independent assertions."""
    old = ('   WS.Open (Wire, Ada.Command_Line.Argument (1));' if release == "25" else
           '   WS.Open (Wire, Ada.Command_Line.Argument (1),\n'
           '     CA_Path => Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA", ""));')
    if text.count(old) != 1:
        raise ValueError("historical consumer Open shape changed")
    if release == "25":
        text = "with Ada.Environment_Variables;\n" + text
    return "with Test_PEM;\n" + text.replace(old,
        '   WS.Open (Wire, Ada.Command_Line.Argument (1), CA_Path => "",\n'
        '     CA_PEM => Test_PEM.Read (Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA")));')


class Campaign:
    def __init__(self, args):
        self.args = args
        self.out = args.scratch.resolve()
        self.out.mkdir(parents=True, exist_ok=False)
        self.env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", AHT_TLS_BACKEND=args.backend)
        self.steps = []

    def run(self, label, command, cwd=ROOT, env=None, timeout=300):
        argv = list(map(str, command))
        (self.out / (label + ".command.json")).write_text(json.dumps(dict(argv=argv, cwd=str(cwd))) + "\n")
        with (self.out / (label + ".stdout")).open("wb") as out, (self.out / (label + ".stderr")).open("wb") as err:
            with subprocess.Popen(argv, cwd=cwd, env=env or self.env, stdout=out, stderr=err,
                                  start_new_session=True) as process:
                timed_out = False
                try:
                    code = process.wait(timeout=timeout)
                except subprocess.TimeoutExpired:
                    timed_out = True
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
                    code = 124
        self.steps.append(dict(label=label, exit=code, timed_out=timed_out))
        (self.out / (label + ".exit")).write_text(str(code) + "\n")
        if code:
            raise RuntimeError(f"{label} failed: {self.out / (label + '.stderr')}")
        return (self.out / (label + ".stdout")).read_text()

    def shell(self, command):
        return ["sh", "-ec", ". " + shlex.quote(str(self.out / "environment.sh")) +
                "; exec " + shlex.join(list(map(str, command)))]

    def build(self, name, source, mains):
        build = self.out / name
        shutil.copytree(ROOT / "ada", build / "ada", ignore=IGNORE)
        shutil.copytree(ROOT / "tests/ada-wss-trust", build / "test")
        if source:
            shutil.copytree(source, build / "generated")
        (build / "consumer.gpr").write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\n'
            'with "ada/oms-json-support/ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("test"' + (', "generated"' if source else '') + ');\n'
            'for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            'for Main use (' + ', '.join('"' + m + '.adb"' for m in mains) + ');\n'
            'package Compiler is for Default_Switches ("Ada") use '
            '("-gnat2022", "-gnata", "-gnato", "-O0"); end Compiler; end Consumer;\n')
        return build

    def execute(self):
        args, out = self.args, self.out
        self.run("source", ["git", "rev-parse", "HEAD", "HEAD^{tree}"])
        if self.run("source-status", ["git", "status", "--porcelain"]).strip() and not args.development:
            raise ValueError("acceptance requires clean source")
        before = {str(p.relative_to(ROOT)): digest(p) for folder in
                  ("ada", "scripts", "tests/ada-wss-trust", "tests/ada-real-uci-multimessage",
                   "tests/ada-real-uci-multimessage26", "tests/fixtures")
                  for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts}
        (out / "input-sources.json").write_text(json.dumps(before, indent=2) + "\n")
        for name, pin in PINS.items():
            source = args.deps.resolve() / name
            if self.run(name + "-identity", ["git", "-C", source, "rev-parse", "HEAD", "HEAD^{tree}"]).splitlines()[0] != pin:
                raise ValueError("wrong dependency " + name)
            if self.run(name + "-clean", ["git", "-C", source, "status", "--porcelain"]).strip():
                raise ValueError("dirty dependency " + name)
            if name in ("aht", "codec"):
                self.run(name + "-archive", ["git", "-C", source, "archive", "--output", out / (name + ".tar"), pin])
                (out / name).mkdir()
                self.run(name + "-extract", ["tar", "-xf", out / (name + ".tar"), "-C", out / name])
        if args.backend == "mbedtls":
            shutil.copytree(args.deps / "aht/third_party/mbedtls-3.6.7", out / "aht/third_party/mbedtls-3.6.7")
        # Verify both authentic closures even in focused-only cells.
        for release in ("25", "26"):
            for name, pin in SCHEMAS[release].items():
                if digest(getattr(args, "schema" + release).resolve() / name) != pin:
                    raise ValueError("schema changed " + name)
        (out / "schema-sources.json").write_text(json.dumps(SCHEMAS, indent=2) + "\n")
        setup = args.setup.resolve()
        setup_env = dict(self.env, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                         XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
        printed = self.run("printenv", ["alr", "-s", setup / "settings", "-n", "printenv"], setup / "consumer", setup_env)
        provider = setup / "consumer/alire/cache/dependencies"
        shutil.copytree(provider, out / "provider", ignore=IGNORE)
        (out / "environment.sh").write_text(printed.replace(str(provider), str(out / "provider")) +
            '\nexport GPR_PROJECT_PATH=' + shlex.quote(str(out / "aht")) + ':"$GPR_PROJECT_PATH"\n')
        self.env["OMS_WS_CODEC_SOURCE"] = str(out / "codec")
        shutil.copyfile(setup / "consumer/alire/alire.lock", out / "alire.lock")
        version = "14.2.0" if args.compiler == "14" else "16.1.0"
        if not self.run("compiler", self.shell(["gnat", "--version"])).startswith("GNAT " + version + "\n"):
            raise ValueError("wrong compiler")
        self.run("platform", self.shell(["sh", "-ec", "uname -a; getconf GNU_LIBC_VERSION; gprbuild --version; "
                    "openssl version; sha256sum $(command -v gnat) $(command -v gcc) $(command -v gprbuild)"]))
        for cert, host in (("cert", "localhost"), ("unrelated", "unrelated.invalid")):
            self.run(cert, ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
                     "-keyout", out / (cert + "-key.pem"), "-out", out / (cert + ".pem"),
                     "-subj", "/CN=" + host, "-addext", "subjectAltName=DNS:" + host])
            (out / (cert + "-key.pem")).chmod(0o600)
            self.run(cert + "-identity", ["openssl", "x509", "-in", out / (cert + ".pem"), "-noout",
                                         "-subject", "-ext", "subjectAltName", "-fingerprint", "-sha256"])
        build = self.build("focused-build", None, ["trust_probe"])
        self.run("focused-build", self.shell(["gprbuild", "-p", "-j4", "-P", build / "consumer.gpr"]))
        self.run("focused", [sys.executable, ROOT / "scripts/task111/trust.py", "--probe", build / "bin/trust_probe",
                 "--cert", out / "cert.pem", "--key", out / "cert-key.pem", "--unrelated", out / "unrelated.pem",
                 "--evidence", out / "focused"], timeout=300)
        if args.memcheck:
            self.run("memcheck", [sys.executable, ROOT / "scripts/task111/trust.py", "--probe", build / "bin/trust_probe",
                     "--cert", out / "cert.pem", "--key", out / "cert-key.pem", "--unrelated", out / "unrelated.pem",
                     "--evidence", out / "memcheck", "--memcheck", args.memcheck,
                     "--memcheck-lib", args.memcheck_lib], timeout=300)
        for release in args.integration:
            self.integration(release)
        for name in PINS:
            if self.run(name + "-final-clean", ["git", "-C", args.deps / name, "status", "--porcelain"]).strip():
                raise ValueError("dependency dirtied " + name)
        if any(digest(ROOT / path) != pin for path, pin in before.items()):
            raise ValueError("inputs changed")
        (out / "result.json").write_text(json.dumps(dict(completed=not args.development, development=args.development,
            compiler=version, backend=args.backend, steps=self.steps, integration=args.integration,
            needs_read_runtime_exercised=False), indent=2) + "\n")
        print("TASK111_COMPLETE", version, args.backend, flush=True)

    def integration(self, release):
        out, args = self.out, self.args
        cell = out / ("uci" + release)
        cell.mkdir()
        task = "106" if release == "25" else "110"
        schema = getattr(args, "schema" + release).resolve() / next(iter(SCHEMAS[release]))
        generated = cell / "generated"
        command = [args.generator.resolve(), "service-generate", "--schema", schema,
                   "--contract", ROOT / f"tests/fixtures/task{task}/uci{release}-service.yaml",
                   "--language", "ada", "--world", "closed-schema", "--with-codec", "--output", generated]
        self.run("generate" + release, command)
        pins = task106.GENERATED if release == "25" else task110.GENERATED
        if {p.name: digest(p) for p in generated.iterdir()} != pins:
            raise ValueError("generated codec/model identity changed")
        (cell / "generated-sources.json").write_text(json.dumps(pins, indent=2) + "\n")
        build = self.build("uci" + release + "-build", generated, ["multimessage_consumer"])
        original = ROOT / ("tests/ada-real-uci-multimessage" + ("26" if release == "26" else "")) / "multimessage_consumer.adb"
        (build / "test/multimessage_consumer.adb").write_text(consumer_source(original.read_text(), release))
        self.run("build" + release, self.shell(["gprbuild", "-p", "-j4", "-P", build / "consumer.gpr"]))
        adapter = cell / "wss-consumer"
        relay = [sys.executable, ROOT / "scripts/task109/relay.py", "--consumer", build / "bin/multimessage_consumer",
                 "--cert", out / "cert.pem", "--key", out / "cert-key.pem", "--evidence", cell / "tls"]
        if release == "26":
            relay += ["--schema-version", "002.6.0", "--service-id", "task110-multimessage"]
        adapter.write_text("#!/bin/sh\nexec " + shlex.join(list(map(str, relay))) + ' "$@"\n')
        adapter.chmod(0o755)
        self.run("wss" + release, [sys.executable, ROOT / f"scripts/task{task}/sleet.py", "--checkout", args.deps / "sleet",
                 "--schema", schema, "--consumer", adapter, "--evidence", cell / "wss"])
        verifier = "109" if release == "25" else "110"
        self.run("verify" + release, [sys.executable, ROOT / f"scripts/task{verifier}/verify.py", cell])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "setup", "deps", "schema25", "schema26", "generator"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--compiler", choices=("14", "16"), required=True)
    parser.add_argument("--backend", choices=("openssl", "mbedtls"), required=True)
    parser.add_argument("--integration", choices=("25", "26"), nargs="*", default=[])
    parser.add_argument("--development", action="store_true", help="diagnostics only, not acceptance")
    parser.add_argument("--memcheck", type=Path)
    parser.add_argument("--memcheck-lib", type=Path)
    Campaign(parser.parse_args()).execute()


if __name__ == "__main__":
    main()