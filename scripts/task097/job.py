"""Task-owned durable job records around the existing Linux session supervisor.

Launch with ``nohup python3 .../job.py JOB -- COMMAND ... > launch.log 2>&1 &``.
JOB must not exist. A missing result.json means incomplete, never successful.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]


def atomic(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("job", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        parser.error("missing command")
    job = args.job.resolve()
    job.mkdir(parents=True, exist_ok=False)
    atomic(job / "command.json", {"argv": command, "cwd": os.getcwd()})
    paths = [Path(__file__), ROOT / "scripts/task076/supervisor.py"]
    paths += [Path(item) for item in command if Path(item).is_file()]
    atomic(job / "inputs.json", {
        str(p.resolve()): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in paths
    })
    spec = importlib.util.spec_from_file_location(
        "established_supervisor", ROOT / "scripts/task076/supervisor.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    # The child wrapper retains separate streams and an atomic exit record.
    wrapper = (
        "import json,pathlib,subprocess,sys; p=pathlib.Path(sys.argv[1]); "
        "o=(p/'stdout.log').open('wb'); e=(p/'stderr.log').open('wb'); "
        "r=subprocess.run(sys.argv[2:],stdout=o,stderr=e); o.close(); e.close(); "
        "t=p/'exit.json.tmp'; t.write_text(json.dumps({'exit':r.returncode})+'\\n'); "
        "t.replace(p/'exit.json'); print('TASK097_JOB_FINISHED',flush=True); "
        "sys.exit(r.returncode)"
    )
    try:
        module.run_checked(
            [sys.executable, "-c", wrapper, str(job), *command], os.environ.copy(),
            phase="task097", timeout=14400, marker="TASK097_JOB_FINISHED",
            log=job / "supervisor.log")
    except BaseException as exc:
        atomic(job / "result.json", {"completed": False, "error": repr(exc)})
        raise
    atomic(job / "result.json", {"completed": True, "exit": 0})


if __name__ == "__main__":
    main()