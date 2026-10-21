#!/usr/bin/env python3
"""Task 098: compiler-backed named TimeZulu Ada OMS JSON codec qualification.

Each cell (namespace x runtime profile x assertion policy) checks readiness,
generates the actual production codec atomically, proves model/API bytes are
unchanged by --with-codec, compiles the generated model, service API and
codec with a fixed consumer, and compares every result line with
expectations from scripts/task098/cases.py (frozen Task 061 corpus and
independently authored documents) -- never from the codec under test.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("task098_cases", ROOT / "scripts/task098/cases.py")
cases_module = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(cases_module)

OAM = "https://www.vdl.afrl.af.mil/programs/oam"
QUALIFIED = "urn:task098"
FIXTURES = ROOT / "tests/fixtures/service-generate"
CONSUMER = ROOT / "tests/ada-time-zulu-codec/time_consumer.adb"
# label: (fixture stem, payload type, --ada-codec-runtime, support project file)
PROFILES = {
    "portable": ("codec-ada-time-zulu", "ClockPayload", None, "ams_gra_oms_json"),
    "float32": ("codec-ada-time-zulu-float32", "Float32ClockPayload",
                "linux-x86_64-glibc-2.41", "ams_gra_oms_json_float32"),
    "lossless": ("codec-ada-time-zulu-mixed", "MixedPayload",
                 "linux-x86_64-glibc-2.41-lossless-v1", "ams_gra_oms_json_lossless"),
}


def qualified_schema(text):
    for old, new in [(f'xmlns:uci="{OAM}"', f'xmlns:oam="{OAM}" xmlns:uci="{QUALIFIED}"'),
                     (f'targetNamespace="{OAM}"', f'targetNamespace="{QUALIFIED}"'),
                     ("uci:version=", "oam:version=")]:
        if text.count(old) != 1:
            raise RuntimeError(f"ambiguous namespace rewrite: {old}")
        text = text.replace(old, new)
    return text


class Campaign:
    """Durable per-step command/stream/exit records in one task-owned tree."""

    def __init__(self, scratch, setup, compiler):
        self.scratch, self.setup = scratch, setup
        self.version = "14.2.0" if compiler == "14" else "16.1.0"
        self.env = dict(os.environ, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                        XDG_CACHE_HOME=str(setup / "home/.cache"),
                        XDG_DATA_HOME=str(setup / "home/.local/share"))
        self.steps = []
        self.environment = scratch / "environment.sh"

    def run_any(self, label, command, cwd=ROOT):
        """Record one step and return its exit status without judging it."""
        prefix = self.scratch / label
        if prefix.with_suffix(".exit").exists():
            raise RuntimeError(f"evidence collision: {label}")
        argv = list(map(str, command))
        prefix.with_suffix(".command.json").write_text(json.dumps({"argv": argv, "cwd": str(cwd)}, indent=2) + "\n")
        with prefix.with_suffix(".stdout").open("wb") as out, prefix.with_suffix(".stderr").open("wb") as err:
            result = subprocess.run(argv, cwd=cwd, env=self.env, stdout=out, stderr=err)
        prefix.with_suffix(".exit").write_text(f"{result.returncode}\n")
        self.steps.append({"label": label, "exit": result.returncode})
        return result.returncode

    def run(self, label, command, cwd=ROOT):
        if self.run_any(label, command, cwd):
            raise RuntimeError(f"{label}: nonzero exit; see {self.scratch / label}.stderr")
        return (self.scratch / f"{label}.stdout").read_text()

    def shell(self, command):
        return ["sh", "-ec", f". {shlex.quote(str(self.environment))}; exec " + shlex.join(map(str, command))]

    def provision(self):
        printed = self.run("printenv", ["alr", "-s", self.setup / "settings", "-n", "printenv"],
                           self.setup / "consumer")
        provider = self.setup / "consumer/alire/cache/dependencies"
        ignore = shutil.ignore_patterns("obj", "obj-*", "lib", "bin", "*.o", "*.ali")
        # Never share a writable provider build tree among workers.
        shutil.copytree(provider, self.scratch / "provider", ignore=ignore)
        self.environment.write_text(printed.replace(str(provider), str(self.scratch / "provider")))
        if not self.run("gnat-version", self.shell(["gnat", "--version"])).startswith(f"GNAT {self.version}\n"):
            raise RuntimeError("compiler identity mismatch")
        self.run("gprbuild-version", self.shell(["gprbuild", "--version"]))
        shutil.copytree(ROOT / "ada/oms-json-support", self.scratch / "support", ignore=ignore)

    def execute(self, label, generated, package, payload, project, policy, documents, expectations, namespace):
        """Build one policy from a private copy and compare every result line."""
        build = self.scratch / label
        shutil.copytree(generated, build)
        (build / "time_consumer.adb").write_text(
            CONSUMER.read_text().replace("TASK098_PACKAGE", package).replace("TASK098_PAYLOAD", payload))
        (build / "consumer.gpr").write_text(
            f'with "{self.scratch / "support" / project}";\nproject Consumer is\n'
            'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            'for Main use ("time_consumer.adb");\npackage Compiler is\n'
            'for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\nend Compiler;\nend Consumer;\n')
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            (build / "policy.adc").write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={build / 'policy.adc'}"]
        self.run(f"{label}-build", self.shell(command), build)
        output = self.run(f"{label}-execute", [build / "bin/time_consumer", documents], build).split("\n")
        accepted = sum(expected is not None for expected in expectations)
        marker = (f"TASK098_CONSUMER_COMPLETE count= {len(expectations)} accepted= {accepted} "
                  f"rejected= {len(expectations) - accepted}")
        if output[-1] != "" or len(output) != len(expectations) + 2 or output[-2] != marker:
            raise RuntimeError(f"{label}: incomplete execution {output[-2:]!r}, expected {marker!r}")
        for index, (line, expected) in enumerate(zip(output[:-2], expectations, strict=True), 1):
            kind, _, detail = line.partition("\t")
            if expected is None:
                if kind != "REJECT":
                    raise RuntimeError(f"{label}: document {index} accepted invalid input")
            elif kind != "ACCEPT":
                raise RuntimeError(f"{label}: document {index} rejected valid input: {detail}")
            elif not cases_module.same(cases_module.decode(detail),
                                       cases_module.qualify_keys(expected, namespace, OAM)):
                raise RuntimeError(f"{label}: document {index} lexical/kind mismatch: {detail}")
        return accepted

    def generate(self, generator, cell, schema, stem, runtime):
        """Readiness, atomic generation and model/API byte preservation."""
        common = ["--schema", schema, "--contract", FIXTURES / f"{stem}.yaml",
                  "--language", "ada", "--world", "closed-schema"]
        codec = ["--with-codec"] + (["--ada-codec-runtime", runtime] if runtime else [])
        if "codec status: READY\n" not in self.run(f"{cell}-check", [generator, "service-check", *common, *codec]):
            raise RuntimeError(f"{cell}: codec not READY")
        model, generated = self.scratch / f"{cell}-model", self.scratch / f"{cell}-generated"
        self.run(f"{cell}-model", [generator, "service-generate", *common, "--output", model])
        self.run(f"{cell}-generate", [generator, "service-generate", *common, *codec, "--output", generated])
        for path in model.iterdir():
            if path.read_bytes() != (generated / path.name).read_bytes():
                raise RuntimeError(f"{cell}: --with-codec changed model/API {path.name}")
        body = (generated / "service_codec.adb").read_text()
        if body.count('raise Codec_Error with "invalid named Time"') != 1 or "Scalar_Time" in body:
            raise RuntimeError(f"{cell}: unexpected generated Time conversion shape")
        return generated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True, help="scripts/task096/setup.py output")
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--namespaces", nargs="+", choices=["oam", "qualified"], default=["oam", "qualified"])
    parser.add_argument("--profiles", nargs="+", choices=list(PROFILES), default=list(PROFILES))
    parser.add_argument("--policies", nargs="+", choices=["Normal", "Check", "Ignore"],
                        default=["Normal", "Check", "Ignore"])
    args = parser.parse_args()
    scratch, generator = args.scratch.resolve(), args.generator.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    campaign = Campaign(scratch, args.setup.resolve(), args.compiler)
    corpus = cases_module.load_corpus()
    campaign.provision()
    cells = []
    for namespace_label in args.namespaces:
        namespace = OAM if namespace_label == "oam" else QUALIFIED
        package = "Programs.Oam" if namespace_label == "oam" else "Urn.Task098"
        for profile in args.profiles:
            stem, payload, runtime, project = PROFILES[profile]
            cell = f"{namespace_label}-{profile}"
            schema = FIXTURES / f"{stem}.xsd"
            if namespace_label == "qualified":
                schema = scratch / f"{cell}.xsd"
                schema.write_text(qualified_schema((FIXTURES / f"{stem}.xsd").read_text()))
            generated = campaign.generate(generator, cell, schema, stem, runtime)
            cases = cases_module.build_cases(profile, corpus)
            lines = [json.dumps(cases_module.qualify_keys(document, namespace, OAM)) for document, _ in cases]
            lines += [lines[0][:-9], "[]"]  # malformed (truncated) JSON; non-object document
            documents = scratch / f"{cell}-documents.jsonl"
            documents.write_text("\n".join(lines) + "\n")
            expectations = [expected for _, expected in cases] + [None, None]
            for policy in args.policies:
                accepted = campaign.execute(f"{cell}-{policy}", generated, package, payload, project, policy,
                                            documents, expectations, namespace)
                cells.append({"compiler": campaign.version, "namespace": namespace_label, "profile": profile,
                              "runtime": runtime or "portable", "policy": policy,
                              "documents": len(expectations), "accepted": accepted,
                              "rejected": len(expectations) - accepted})
                print("TASK098_CELL_COMPLETE", campaign.version, cell, policy, len(expectations), flush=True)
    paths = [Path(__file__), ROOT / "scripts/task098/cases.py", cases_module.CORPUS, CONSUMER, generator]
    paths += sorted(p for p in scratch.rglob("*") if p.is_file() and (
        p.suffix == ".xsd" or (p.suffix in [".ads", ".adb"] and p.parent.name.endswith("-generated"))))
    (scratch / "sources.json").write_text(json.dumps(
        {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True,
                                     "compiler": campaign.version, "cells": cells,
                                     "steps": campaign.steps}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print(f"TASK098_QUALIFICATION_COMPLETE compiler={campaign.version} cells={len(cells)}", flush=True)


if __name__ == "__main__":
    main()
