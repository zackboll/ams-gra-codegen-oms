"""Reproduce Task096 locally from fresh task-owned homes; never dispatch CI.

Requires installed, verified GNAT 14.2.0, Alire 2.1.1, Rust, gcc, glibc 2.41,
and de_DE.UTF-8. Invoke under job.py for durable detached supervision.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    steps = []

    def run(label, argv, stdin=None, env=None):
        record = {"label": label, "argv": argv, "cwd": str(ROOT)}
        (scratch / f"{label}.command.json").write_text(json.dumps(record, indent=2) + "\n")
        print("START", label, flush=True)
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(argv, cwd=ROOT, stdin=stdin, stdout=out, stderr=err, env=env)
        temporary = scratch / f"{label}.exit.tmp"
        temporary.write_text(str(result.returncode) + "\n")
        temporary.replace(scratch / f"{label}.exit")
        steps.append(record | {"exit": result.returncode})
        (scratch / "checkpoint.json").write_text(json.dumps(steps, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(f"failed {label}; research incomplete")
        print("COMPLETE", label, flush=True)

    py = sys.executable
    probes = ROOT / "tests/ada-float64-probe"
    scripts = ROOT / "scripts/task096"
    run("runtime", ["sh", "-ec", "uname -a; getconf GNU_LIBC_VERSION; dpkg-query -W libc6 libc6-dev gnat-14 gcc-14; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6 /usr/bin/gnat /usr/bin/gcc /usr/local/bin/alr; gcc -v; rustc -Vv; test \"$(getconf GNU_LIBC_VERSION)\" = 'glibc 2.41'; test \"$(uname -m)\" = x86_64"])
    run("unit-tests", [py, "-m", "unittest", "discover", "-s", str(probes), "-p", "test_*.py", "-v"])
    corpus = scratch / "corpus"
    run("oracle", [py, str(probes / "oracle.py"), str(corpus)])
    run("oracle-independent-review", [py, "-c",
        "import sys,pathlib; sys.path.insert(0,sys.argv[1]); import oracle; from test_oracle import reference; "
        "rows=pathlib.Path(sys.argv[2]).read_text().splitlines(); count=0\n"
        "for row in rows:\n"
        " _,expected,token=row.split('\\t',2); negative,value=oracle.rational_token(token)\n"
        " if isinstance(value,str): continue\n"
        " result=reference(value); actual=result if result=='OVERFLOW' else f'{result | (oracle.SIGN if negative else 0):016X}'\n"
        " if actual!=expected: raise RuntimeError((token,actual,expected))\n"
        " count+=1\n"
        "print(f'TASK096_INDEPENDENT_SEARCH_COMPLETE rows={count} disagreements=0')",
        str(probes), str(corpus / "conversions.tsv")])
    crosscheck = scratch / "crosscheck"
    run("crosscheck-build", ["rustc", "--edition=2024", "-O", str(probes / "crosscheck.rs"), "-o", str(crosscheck)])
    with (corpus / "conversions.tsv").open("rb") as input_file:
        run("crosscheck", [str(crosscheck)], stdin=input_file)
    target = scratch / "cargo-target"
    run("generator-build", ["cargo", "build", "--locked", "-p", "ams-gra-codegen-oms"], env=os.environ | {"CARGO_TARGET_DIR": str(target)})
    generator = target / "debug/ams-gra-codegen-oms"
    run("c-corpus", [py, str(scripts / "corpus_c.py"), "--scratch", str(scratch / "c-corpus"), "--corpus", str(corpus)])
    run("c-environment", [py, str(scripts / "environment.py"), "--scratch", str(scratch / "c-environment")])
    for compiler in ["14", "16"]:
        setup = scratch / f"setup{compiler}"
        run(f"setup{compiler}", [py, str(scripts / "setup.py"), "--scratch", str(setup), "--compiler", compiler])
        run(f"numeric{compiler}", [py, str(ROOT / "scripts/check-task096.py"), "--scratch", str(scratch / f"numeric{compiler}"), "--setup", str(setup), "--corpus", str(corpus)])
        for profile in ["oam", "qualified"]:
            model = scratch / f"model{compiler}-{profile}"
            document = scratch / f"document{compiler}-{profile}"
            run(f"model{compiler}-{profile}", [py, str(scripts / "model.py"), "--scratch", str(model), "--setup", str(setup), "--generator", str(generator), "--profile", profile])
            run(f"document{compiler}-{profile}", [py, str(scripts / "document.py"), "--scratch", str(document), "--setup", str(setup), "--model", str(model), "--profile", profile, "--corpus", str(corpus)])
            run(f"environment{compiler}-{profile}", [py, str(scripts / "environment_ada.py"), "--scratch", str(scratch / f"environment{compiler}-{profile}"), "--document", str(document), "--profile", profile])
    for optimization in ["0", "2"]:
        binary = scratch / f"bridge-O{optimization}"
        run(f"bridge-build-O{optimization}", ["gcc", "-std=c11", f"-O{optimization}", "-Wall", "-Wextra", "-Werror", str(probes / "boundary.c"), str(probes / "bridge_controls.c"), "-Wl,--wrap=newlocale", "-Wl,--wrap=uselocale", "-lm", "-o", str(binary)])
        run(f"bridge-O{optimization}", [str(binary)])
    files = [*probes.glob("*"), *scripts.glob("*.py"), ROOT / "scripts/check-task096.py"]
    identities = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files if p.is_file()}
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed_local_measurements": True, "steps": steps, "sources": identities}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK096_REPRODUCTION_COMPLETE", flush=True)


if __name__ == "__main__":
    main()