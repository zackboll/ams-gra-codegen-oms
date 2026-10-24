"""Task097-owned sequential inherited compiler campaigns with durable child logs."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--uci25", type=Path, required=True)
    parser.add_argument("--uci26", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    shared = ["--compiler", args.compiler, "--generator", str(args.generator.resolve())]
    campaigns = [(f"task{task}", [f"check-task{task}.py"]) for task in ["087b", "088", "089", "089r", "090", "090r", "091", "091r", "092", "093", "093-combined", "094"]]
    campaigns += [("092-qualified", ["check-task092.py", "--profile", "qualified"]),
                  ("093-mixed", ["check-task093-mixed.py"])]
    for profile in ["oam", "qualified"]:
        campaigns += [(f"095-{profile}", ["check-task095.py", "--profile", profile, "--numeric-corpus", "--environment", "--rounding"]),
                      (f"095-identifiers-{profile}", ["check-task095.py", "--profile", profile, "--identifiers"])]
    for family in ["portable", "float32"]:
        campaigns.append((f"093-helper-{family}", ["check-task093-helper-demand.py", "--family", family]))
    campaigns += [("092-real", ["check-task092-real.py", "--uci25", str(args.uci25.resolve()), "--uci26", str(args.uci26.resolve())])]
    for version, schema in [("25", args.uci25), ("26", args.uci26)]:
        campaigns.append((f"093-log-{version}", ["check-task093-log.py", "--version", version, "--schema", str(schema.resolve())]))
    rows = []
    for label, command in campaigns:
        command = [sys.executable, str(ROOT / "scripts" / command[0]), *command[1:], *shared, "--scratch", str(scratch / label)]
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(ROOT)}, indent=2) + "\n")
        print("START", label, flush=True)
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            process = subprocess.run(command, cwd=ROOT, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
        rows.append({"campaign": label, "exit": process.returncode})
        (scratch / "checkpoint.json").write_text(json.dumps(rows, indent=2) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed inherited campaign {label}")
        print("COMPLETE", label, flush=True)
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "compiler": args.compiler, "campaigns": rows}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_INHERITED_COMPLETE campaigns=" + str(len(rows)), flush=True)


if __name__ == "__main__":
    main()