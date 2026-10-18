"""Provision a fresh task-owned Alire home; never use the developer cache."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    args = parser.parse_args()
    root = args.scratch.resolve()
    root.mkdir(parents=True, exist_ok=False)
    for name in ["home", "settings", "cache", "consumer"]:
        (root / name).mkdir()
    env = os.environ.copy()
    env.update(HOME=str(root / "home"), XDG_CONFIG_HOME=str(root / "home/.config"),
               XDG_CACHE_HOME=str(root / "home/.cache"),
               XDG_DATA_HOME=str(root / "home/.local/share"))
    consumer = root / "consumer"
    sequence = 0

    def run(command):
        nonlocal sequence
        sequence += 1
        prefix = root / f"step-{sequence:02d}"
        prefix.with_suffix(".command.json").write_text(json.dumps(command) + "\n")
        print("START", sequence, command, flush=True)
        with prefix.with_suffix(".stdout").open("wb") as out, prefix.with_suffix(".stderr").open("wb") as err:
            result = subprocess.run(command, cwd=consumer, env=env, stdout=out, stderr=err)
        temporary = prefix.with_suffix(".exit.tmp")
        temporary.write_text(str(result.returncode) + "\n")
        temporary.replace(prefix.with_suffix(".exit"))
        if result.returncode:
            raise RuntimeError(f"setup step {sequence} failed")
        return prefix.with_suffix(".stdout").read_text()

    if args.compiler == "14":
        # Do not request the external version until the installed binary proves it.
        if not run(["gnat", "--version"]).startswith("GNAT 14.2.0\n"):
            raise RuntimeError("external GNAT is not 14.2.0")
    alr = ["alr", "-s", str(root / "settings"), "-n"]
    if run(["alr", "--version"]).strip() != "alr 2.1.1":
        raise RuntimeError("wrong Alire")
    run(alr + ["settings", "--global", "--set", "cache.dir", str(root / "cache")])
    run(alr + ["settings", "--global", "--set", "dependencies.shared", "false"])
    compiler, version = ("gnat_external", "14.2.0") if args.compiler == "14" else ("gnat_native", "16.1.0")
    (consumer / "alire.toml").write_text(
        'name = "task096_probe"\nversion = "0.1.0-dev"\n'
        'description = "Task096 independent research"\nlicenses = "Apache-2.0"\n'
        'project-files = ["probe.gpr"]\n[[depends-on]]\n'
        f'{compiler} = "={version}"\ngprbuild = "=26.0.1"\n'
        'gnatcoll = "=26.0.0"\ngnatcoll_minimal = "=26.0.0"\n')
    (consumer / "probe.gpr").write_text(
        'with "gnatcoll_core";\nproject Probe is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("setup_probe.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
        'end Compiler; end Probe;\n')
    (consumer / "setup_probe.adb").write_text(
        'with Ada.Text_IO; with GNATCOLL.JSON;\nprocedure Setup_Probe is\n'
        'begin Ada.Text_IO.Put_Line (GNATCOLL.JSON.Write (GNATCOLL.JSON.Create (True)));\n'
        'end Setup_Probe;\n')
    run(alr + ["--force", "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"])
    run(alr + ["build"])
    if run([str(consumer / "bin/setup_probe")]).strip() != "true":
        raise RuntimeError("provider smoke test failed")
    run(alr + ["exec", "--", "sh", "-ec", "gnat --version; gprbuild --version; gcc -v; command -v gnat gprbuild gcc; sha256sum $(command -v gnat) $(command -v gprbuild) $(command -v gcc)"])
    run(alr + ["version"])
    files = [p for p in root.rglob("*") if p.is_file() and
             (p.name in ["alire.lock", "alire.toml", "gnatcoll-json.adb", "gnatcoll-json.ads"])]
    identities = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    (root / "identities.json").write_text(json.dumps(identities, indent=2, sort_keys=True) + "\n")
    print("TASK096_SETUP_COMPLETE", compiler, version, flush=True)


if __name__ == "__main__":
    main()