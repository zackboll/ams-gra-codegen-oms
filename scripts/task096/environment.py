"""Run all 16 independent rounding pairs x all 4 flushing modes at O0/O2."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    probes = ROOT / "tests/ada-float64-probe"
    inherited = ROOT / "tests/ada-float32-probe"
    results = []
    paths = [probes / "boundary.c", probes / "environment.c",
             inherited / "rounding_harness.c", inherited / "environment_harness.c"]
    for optimization in ["0", "2"]:
        binary = scratch / f"environment-O{optimization}"
        command = ["gcc", "-std=c11", f"-O{optimization}", "-Wall", "-Wextra", "-Werror",
                   *map(str, paths), "-lm", "-o", str(binary)]
        (scratch / f"build-O{optimization}.command.json").write_text(json.dumps(command) + "\n")
        subprocess.run(command, check=True)
        for x87 in range(4):
            for sse in range(4):
                for flushing in range(4):
                    label = f"O{optimization}-{x87}-{sse}-{flushing}"
                    command = [str(binary), str(x87), str(sse), str(flushing)]
                    (scratch / f"{label}.command.json").write_text(json.dumps(command) + "\n")
                    with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
                        process = subprocess.run(command, stdout=out, stderr=err)
                    (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
                    if process.returncode:
                        raise RuntimeError(f"failed {label}")
                    expected_checks = 37 if (x87, sse, flushing) == (0, 0, 0) else 68
                    expected = f"TASK096_C_ENVIRONMENT_COMPLETE x87={x87} mxcsr={sse} flushing={flushing} checks={expected_checks}\n"
                    text = (scratch / f"{label}.stdout").read_text()
                    if text != expected:
                        raise RuntimeError(f"incomplete {label}: {text!r}")
                    results.append({"optimization": optimization, "x87": x87, "mxcsr": sse, "flushing": flushing})
        print("COMPLETE O" + optimization, flush=True)
    paths += [Path(__file__), *scratch.glob("environment-O*")]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "executions": results}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK096_C_ENVIRONMENT_MATRIX_COMPLETE executions=128", flush=True)


if __name__ == "__main__":
    main()