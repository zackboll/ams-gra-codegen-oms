"""Dedicated hosted/local production Float64 campaign, fresh compiler provisioning.

Execute under job.py. Every child has separate command, streams and exit.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--generator", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    steps = []

    def run(label, command):
        record = {"label": label, "argv": command, "cwd": str(ROOT)}
        (scratch / f"{label}.command.json").write_text(json.dumps(record, indent=2) + "\n")
        print("START", label, flush=True)
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            process = subprocess.run(command, cwd=ROOT, stdout=out, stderr=err)
        temporary = scratch / f"{label}.exit.tmp"
        temporary.write_text(str(process.returncode) + "\n")
        temporary.replace(scratch / f"{label}.exit")
        steps.append(record | {"exit": process.returncode})
        (scratch / "checkpoint.json").write_text(json.dumps(steps, indent=2) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed {label}; production qualification incomplete")
        print("COMPLETE", label, flush=True)

    py = sys.executable
    probes = ROOT / "tests/ada-float64-probe"
    scripts = ROOT / "scripts/task097"
    run("runtime", ["sh", "-ec", "uname -a; getconf GNU_LIBC_VERSION; dpkg-query -W libc6 libc6-dev gnat-14 gcc-14; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6 /usr/bin/gnat /usr/bin/gcc $(command -v alr); gcc -v; rustc -Vv; test \"$(getconf GNU_LIBC_VERSION)\" = 'glibc 2.41'; test \"$(uname -m)\" = x86_64; gnat --version | grep '^GNAT 14.2.0'"])
    run("oracle-tests", [py, "-m", "unittest", "discover", "-s", str(probes), "-p", "test_oracle.py", "-v"])
    corpus = scratch / "corpus"
    run("oracle", [py, str(probes / "oracle.py"), str(corpus)])
    pinned = json.loads((probes / "corpus-manifest.json").read_text())
    actual = json.loads((corpus / "manifest.json").read_text())
    if actual != pinned:
        raise RuntimeError("frozen corpus recipe/hash mismatch")
    run("oracle-independent-review", [py, "-c",
        "import sys,pathlib; sys.path.insert(0,sys.argv[1]); import oracle; from test_oracle import reference; "
        "rows=pathlib.Path(sys.argv[2]).read_text().splitlines(); count=0\n"
        "for row in rows:\n"
        " _,expected,token=row.split('\\t',2); negative,value=oracle.rational_token(token)\n"
        " if isinstance(value,str): continue\n"
        " result=reference(value); actual=result if result=='OVERFLOW' else f'{result | (oracle.SIGN if negative else 0):016X}'\n"
        " if actual!=expected: raise RuntimeError((token,actual,expected))\n"
        " count+=1\n"
        "print(f'TASK097_INDEPENDENT_SEARCH_COMPLETE rows={count} disagreements=0')",
        str(probes), str(corpus / "conversions.tsv")])
    # The compiler=14 job owns the identical C O0/O2 corpus/lifecycle campaign.
    if args.compiler == "14":
        run("production-c-boundary", [py, str(scripts / "boundary.py"), "--scratch", str(scratch / "c-boundary"), "--corpus", str(corpus)])
    setup = scratch / "setup"
    run("setup", [py, str(ROOT / "scripts/task096/setup.py"), "--scratch", str(setup), "--compiler", args.compiler])
    common = ["--setup", str(setup), "--generator", str(args.generator.resolve())]
    run("checkpoint", [py, str(scripts / "checkpoint.py"), "--scratch", str(scratch / "small-checkpoint"), *common])
    for namespace in ["oam", "qualified"]:
        generated = scratch / f"generated-{namespace}"
        run(f"generated-{namespace}", [py, str(scripts / "generated.py"), "--scratch", str(generated), *common,
            "--corpus", str(corpus), "--namespace", namespace])
        run(f"environment-{namespace}", [py, str(scripts / "environment.py"), "--scratch", str(scratch / f"environment-{namespace}"), "--generated", str(generated)])
        run(f"conformance-{namespace}", [py, str(scripts / "conformance.py"), "--scratch", str(scratch / f"conformance-{namespace}"), "--generated", str(generated), "--generator", str(args.generator.resolve())])
    optimized = scratch / "generated-oam-O2"
    run("generated-oam-O2", [py, str(scripts / "generated.py"), "--scratch", str(optimized), *common,
        "--corpus", str(corpus), "--namespace", "oam", "--optimization", "2"])
    paths = [*scripts.glob("*.py"), *probes.glob("*.py"), ROOT / "scripts/task096/setup.py", args.generator.resolve()]
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "compiler": args.compiler, "production_generated": True, "steps": steps,
        "sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_PRODUCTION_QUALIFICATION_COMPLETE compiler=" + args.compiler, flush=True)


if __name__ == "__main__":
    main()