"""Observe raw DOM/Ada candidate environment dependence, not certification."""
import argparse
import json
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--document", type=Path, required=True)
    args = parser.parse_args()
    scratch, document = args.scratch.resolve(), args.document.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    shutil.copytree(document / "consumer", consumer)
    shutil.copytree(document / "support", scratch / "support")
    shutil.copytree(document / "provider", scratch / "provider")
    environment = scratch / "environment.sh"
    environment.write_text((document / "environment.sh").read_text().replace(str(document / "provider"), str(scratch / "provider")))
    shutil.copyfile(ROOT / "tests/ada-float64-probe/candidate_environment.adb", consumer / "candidate_environment.adb")
    for name in ["rounding_harness.c", "environment_harness.c"]:
        shutil.copyfile(ROOT / "tests/ada-float32-probe" / name, consumer / name)
    (consumer / "consumer.gpr").write_text((consumer / "consumer.gpr").read_text().replace('"document_probe.adb"', '"candidate_environment.adb"'))
    tokens = ["-0", "-0.0", "1.0000000000000002", "1.2345678901234567",
              "262329577.35378633439540863037109375", "5e-324", "1e-324", "1e309", "9007199254740993"]
    token_file = scratch / "tokens.txt"
    token_file.write_text("\n".join(tokens) + "\n")
    results = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-f", "-p"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        with (scratch / f"{policy}-build.stdout").open("wb") as out, (scratch / f"{policy}-build.stderr").open("wb") as err:
            subprocess.run(["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)], cwd=consumer, stdout=out, stderr=err, check=True)
        observations = {}
        for x87 in range(4):
            for sse in range(4):
                for flushing in range(4):
                    label = f"{policy}-{x87}-{sse}-{flushing}"
                    command = [str(consumer / "bin/candidate_environment"), str(x87), str(sse), str(flushing), str(token_file)]
                    (scratch / f"{label}.command.json").write_text(json.dumps(command) + "\n")
                    with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
                        process = subprocess.run(command, cwd=consumer, stdout=out, stderr=err)
                    (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
                    if process.returncode:
                        raise RuntimeError(label)
                    lines = (scratch / f"{label}.stdout").read_text().splitlines()
                    if len(lines) != len(tokens) + 3 or lines[-1] != f"TASK096_RAW_CANDIDATE_ENVIRONMENT_COMPLETE count= {len(tokens)}":
                        raise RuntimeError("missing raw candidate observations")
                    observations[(x87, sse, flushing)] = lines[:-1]
        baseline = observations[(0, 0, 0)]
        for controls, lines in observations.items():
            changes = [row for row, before in zip(lines, baseline, strict=True) if row != before]
            results.append({"policy": policy, "x87": controls[0], "mxcsr": controls[1], "flushing": controls[2],
                            "changed_rows_from_nearest": len(changes), "examples": changes[:2]})
        print("COMPLETE", policy, flush=True)
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"observational_only": True, "executions": results}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK096_RAW_CANDIDATE_ENVIRONMENT_MATRIX_COMPLETE executions=192", flush=True)


if __name__ == "__main__":
    main()