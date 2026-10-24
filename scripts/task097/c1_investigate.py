"""Isolated pristine-checkpoint allocation reproductions; no cache edits.

Run under job.py. Expected red child exits are recorded as red, not successes.
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
OLD = Path("/home/zboll/task097-scratch")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    consumer.mkdir()
    original = OLD / "conformance16-oam-03"
    records = []

    def run(label, command, cwd, expected=0):
        record = {"argv": command, "cwd": str(cwd), "expected_exit": expected}
        (scratch / f"{label}.command.json").write_text(json.dumps(record, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            process = subprocess.run(command, cwd=cwd, stdout=out, stderr=err,
                                     env=os.environ | {"PYTHONDONTWRITEBYTECODE": "1"})
        (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
        records.append(record | {"label": label, "actual_exit": process.returncode})
        if expected is not None and process.returncode != expected:
            raise RuntimeError(f"{label}: exit {process.returncode}, expected {expected}")
        print("COMPLETE", label, process.returncode, flush=True)
        return (scratch / f"{label}.stdout").read_text()

    for source in (original / "consumer").glob("*.ad?"):
        if source.name not in ["allocation_faults.ads", "allocation_faults.adb", "conformance.adb"]:
            shutil.copy2(source, consumer)
    for name in ["allocation_faults.ads", "allocation_faults.adb", "conformance.adb", "c1_provider_probe.adb"]:
        shutil.copy2(ROOT / "tests/ada-float64-codec" / name, consumer)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    shutil.copy2(original / "valid.json", scratch)
    shutil.copy2(original / "conformance.xsd", scratch)
    setup = OLD / f"setup{args.compiler}"
    provider = setup / "consumer/alire/cache/dependencies"
    shutil.copytree(provider, scratch / "provider",
        ignore=shutil.ignore_patterns("obj", "lib", "*.o", "*.ali", "*.lexch", "*.a", "*.so"))
    output = run("environment", ["alr", "-s", str(setup / "settings"), "-n", "printenv"], setup / "consumer")
    environment = scratch / "environment.sh"
    environment.write_text(output.replace(str(provider), str(scratch / "provider")))
    project = (original / "consumer/consumer.gpr").read_text().replace(
        'for Main use ("conformance.adb")', 'for Main use ("conformance.adb", "c1_provider_probe.adb")')
    (consumer / "consumer.gpr").write_text(project)
    shell = lambda command: ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)]
    run("identity", shell(["sh", "-ec", "gnat --version; gprbuild --version; gcc --version; getconf GNU_LIBC_VERSION; dpkg-query -W libc6; sha256sum $(command -v gnat) $(command -v gcc) $(command -v gprbuild) /usr/lib/x86_64-linux-gnu/libc.so.6; locale"]), consumer)
    run("build-original", shell(["gprbuild", "-P", "consumer.gpr", "-p"]), consumer)
    run("original-allocation", [str(consumer / "bin/conformance"), "allocation", str(scratch / "valid.json")], consumer, 1)
    for budget in range(12):
        run(f"provider-{budget}", [str(consumer / "bin/c1_provider_probe"), str(budget)], consumer)
    # Symbols only: keep original O0 consumer / O2 provider optimization and
    # inlining defaults. Debugger observations allocate outside the inferior.
    run("build-debug", shell(["gprbuild", "-P", "consumer.gpr", "-p", "-f", "-cargs:Ada", "-g"]), consumer)
    trace = scratch / "trace.gdb"
    trace.write_text("""set pagination off
set confirm off
set language c
break allocation_faults.adb:20
ignore 1 67
commands
silent
printf "TASK097_C1_FIRST_INJECTED_FAILURE\\n"
bt 24
info args
up
info args
disable 1
break __gnat_raise_exception
commands
silent
printf "TASK097_C1_EXCEPTION_SEQUENCE\\n"
info args
bt 18
continue
end
continue
end
run
""")
    run("generated-first-failure-trace", ["gdb", "-q", "-batch", "-x", str(trace), "--args",
        str(consumer / "bin/conformance"), "allocation", str(scratch / "valid.json")], consumer)
    paths = [p for directory in [consumer, scratch / "support", scratch / "provider"] for p in directory.rglob("*")
             if p.is_file() and p.suffix in [".ads", ".adb", ".gpr", ".c", ".h"]]
    paths += [Path(__file__), scratch / "valid.json", scratch / "conformance.xsd", consumer / "bin/conformance", consumer / "bin/c1_provider_probe"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed_investigation": True, "expected_red": True, "compiler": args.compiler, "commands": records}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_C1_PRISTINE_REPRODUCTION_COMPLETE", args.compiler, flush=True)


if __name__ == "__main__":
    main()