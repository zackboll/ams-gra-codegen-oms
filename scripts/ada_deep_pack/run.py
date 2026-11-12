#!/usr/bin/env python3
"""Prepare once, then run every original Deep campaign in private workers."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ada_ci_pack.run import execute, monitor, sha, STOP, write

ROOT = Path(__file__).resolve().parents[2]
INVENTORY = json.loads(Path(__file__).with_name("inventory.json").read_text())
REV = "94796daf55ebd1134dce0925e6a204a05ad3423b"
TREE = "47da595b9078cac1a9351e8290d1ae7f3976367f"


def git(*args, cwd=ROOT):
    return subprocess.check_output(["git", "-c", f"safe.directory={cwd}", *args], cwd=cwd,
                                   env=dict(os.environ, GIT_OPTIONAL_LOCKS="0"), text=True).strip()


def identity(expected):
    if git("rev-parse", "HEAD") != expected or git("status", "--porcelain"):
        raise RuntimeError("wrong/dirty workflow source")
    for path, digest in (INVENTORY["qualification_sha256"] | INVENTORY["infrastructure_sha256"]).items():
        if sha(ROOT / path) != digest:
            raise RuntimeError("changed qualification input: " + path)
    return dict(sha=expected, tree=git("rev-parse", "HEAD^{tree}"),
                run_id=os.environ["GITHUB_RUN_ID"], attempt=os.environ["GITHUB_RUN_ATTEMPT"])


def suites(pack, main):
    return [name for name in INVENTORY["packs"][pack]
            if main or not INVENTORY["suites"][name]["main_only"]]


def freeze(root):
    for path in sorted(root.rglob("*"), reverse=True):
        if not path.is_symlink():
            path.chmod(path.stat().st_mode & ~0o222)
    root.chmod(root.stat().st_mode & ~0o222)


def manifest(root):
    result = {}
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            result[str(path.relative_to(root))] = dict(link=os.readlink(path))
        else:
            if path.stat().st_mode & 0o222:
                raise RuntimeError("writable shared input: " + str(path))
            if path.is_file():
                result[str(path.relative_to(root))] = dict(sha256=sha(path))
    if root.stat().st_mode & 0o222 or not result:
        raise RuntimeError("writable/empty shared input")
    return result


def relocate_setup(seed, destination):
    """Private Alire state; Alire 2.1.1 cannot traverse a symlinked toolchain root.

    Copy the already provisioned, immutable distribution (never download/build
    again). Keep executable/runtime bytes read-only in every private copy.
    """
    shutil.copytree(seed, destination, symlinks=True)
    old, new = str(seed).encode(), str(destination).encode()
    for path in destination.rglob("*"):
        if not path.is_file() or path.is_symlink():
            continue
        if "toolchains" in path.relative_to(destination).parts:
            continue
        data = path.read_bytes()
        if b"\0" not in data and old in data:
            path.write_bytes(data.replace(old, new))
    # Setup probe outputs are not consumer outputs; campaigns copy source only.
    for folder in (destination / "consumer/obj", destination / "consumer/bin"):
        shutil.rmtree(folder)


def render(text, name, directory, shared, compiler):
    task = INVENTORY["suites"][name]["task"]
    return (text.replace("${{ matrix.compiler }}", compiler)
            .replace(f"/tmp/task{task}-evidence", str(directory / "evidence"))
            .replace(f"/tmp/task{task}-sleet", str(shared / "sleet"))
            .replace(f"/tmp/task{task}-venv", str(directory / "venv"))
            .replace("/tmp/ci004-ada-pinned", str(directory / "owned")))


def bundle(directory):
    # Preserve empty stderr/negative exits and hidden source inputs too, without
    # reading multi-gigabyte tool distributions or compiler objects into memory.
    extensions = {".json", ".jsonl", ".txt", ".log", ".stdout", ".stderr", ".exit",
                  ".sha256", ".ads", ".adb", ".gpr", ".adc", ".xsd", ".yaml",
                  ".toml", ".lock", ".c", ".h"}
    return {str(p.relative_to(directory)): p.read_bytes()
            for p in directory.rglob("*") if p.is_file() and
            (p.suffix in extensions or p.name == "LICENSE") and "setup" not in p.relative_to(directory).parts}


def verify_suite(name, directory, compiler, source):
    task = INVENTORY["suites"][name]["task"]
    module = importlib.import_module(f"task{task}_evidence")
    args = [bundle(directory / "evidence"), compiler]
    if task == 100:
        args.append(True)
    args.extend([source["sha"], source["tree"], source["run_id"], source["attempt"]])
    return module.verify_bundle(*args)


def worker_git_env(env, source):
    """Trust only the verified checkout in this worker's protected Git scope.

    HOME belongs to the private Alire setup, not the job's Git configuration.
    Append command-scope entries without replacing any existing worker settings
    or writing global/system policy. The empty entry resets inherited trust.
    Plain Git in the frozen shell commands and their descendants inherits this.
    """
    repository = ROOT.resolve(strict=True)
    if "*" in str(repository):
        raise RuntimeError("wildcard worker source path")
    trusted = dict(env)
    count = int(trusted.get("GIT_CONFIG_COUNT", "0"))
    if count < 0 or trusted.get("GIT_CONFIG_PARAMETERS"):
        raise RuntimeError("unexpected worker Git command configuration")
    for value in ("", str(repository)):
        trusted[f"GIT_CONFIG_KEY_{count}"] = "safe.directory"
        trusted[f"GIT_CONFIG_VALUE_{count}"] = value
        count += 1
    trusted["GIT_CONFIG_COUNT"] = str(count)
    actual = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel", "HEAD", "HEAD^{tree}"],
        cwd=repository, env=trusted, text=True).splitlines()
    if actual != [str(repository), source["sha"], source["tree"]]:
        raise RuntimeError("worker source path/commit/tree mismatch")
    return trusted


def worker(name, directory, shared, compiler, source, cpu):
    started = time.monotonic()
    result = dict(suite=name, source=source, compiler=compiler, exit=1, steps=[])
    deadline = started + INVENTORY["suites"][name]["timeout_seconds"]
    env = dict(os.environ, HOME=str(directory / "evidence/setup/home"),
               PATH=str(directory / "wrappers") + os.pathsep + str(directory / "venv/bin") + os.pathsep + os.environ["PATH"],
               ADA_DEEP_ALR_TRACE=str(directory / "alr.jsonl"),
               TMPDIR=str(directory / "tmp"), PYTHONDONTWRITEBYTECODE="1",
               GIT_OPTIONAL_LOCKS="0",
               AMS_GRA_ADA_BUILD_JOBS="1", CARGO_BUILD_JOBS="1", RAYON_NUM_THREADS="1",
               OMP_NUM_THREADS="1", GITHUB_WORKSPACE=str(ROOT))
    try:
        # Git-only regressions run without Alire. Never pass which()'s None to
        # subprocess, or inherit a stale wrapper target. Production main still
        # requires the exact Alire installation before preparing any workers.
        env.pop("ADA_DEEP_REAL_ALR", None)
        alr = shutil.which("alr")
        if alr is not None:
            env["ADA_DEEP_REAL_ALR"] = alr
        env = worker_git_env(env, source)
        for index, step in enumerate(INVENTORY["suites"][name]["steps"]):
            command = render(step["run"], name, directory, shared, compiler)
            step_env = dict(env, **{k: render(v, name, directory, shared, compiler)
                                   for k, v in step.get("env", {}).items()})
            record = execute(["bash", "-euo", "pipefail", "-c", command], step_env, ROOT,
                             directory / f"step-{index}", max(0, deadline - time.monotonic()), cpu)
            result["steps"].append(dict(name=step["name"], **record))
        result["evidence"] = verify_suite(name, directory, compiler, source)
        result["exit"] = 0
    except Exception as exc:
        result["error"] = str(exc)
    result["elapsed_seconds"] = time.monotonic() - started
    write(directory / "completion.json", result)
    print(json.dumps(result), flush=True)
    return result


def prepare(root, names, compiler, source):
    shared = root / "shared"
    shared.mkdir()
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", CARGO_BUILD_JOBS="4")

    def run(label, command, cwd=ROOT, timeout=3600):
        return execute(list(map(str, command)), env, cwd, root / label, timeout)

    profiles = ["debug", "release"] if "ada-websocket-real-sleet" in names else ["release"]
    generators = {}
    for profile in profiles:
        run("generator-" + profile, ["cargo", "+1.95.0", "build", "--locked", "-p", "ams-gra-codegen-oms",
                                    *(["--release"] if profile == "release" else [])])
        generators[profile] = sha(ROOT / f"target/{profile}/ams-gra-codegen-oms")
    setup = root / "setup"
    run("setup", [sys.executable, ROOT / "scripts/task096/setup.py", "--scratch", setup, "--compiler", compiler])
    # Compiler distributions are immutable; mutable Alire configuration is copied.
    freeze(setup / "cache/toolchains")
    sleet = shared / "sleet"
    run("sleet-clone", ["git", "clone", "--quiet", "https://gitlab.com/zackboll/sleet.git", sleet])
    run("sleet-checkout", ["git", "checkout", "--quiet", "--detach", REV], sleet)
    if git("rev-parse", "HEAD", "HEAD^{tree}", cwd=sleet).splitlines() != [REV, TREE] or git("status", "--porcelain", cwd=sleet):
        raise RuntimeError("wrong/dirty pinned Sleet source")
    run("sleet-build", ["cargo", "+1.95.0", "build", "--release", "--locked", "-p", "sleet"], sleet)
    # Only the executable is a worker input. Do not retain Rust incremental or
    # intermediate state in the immutable shared distribution.
    shutil.move(sleet / "target/release/sleet", shared / "sleet-binary")
    shutil.rmtree(sleet / "target")
    (sleet / "target/release").mkdir(parents=True)
    shutil.move(shared / "sleet-binary", sleet / "target/release/sleet")
    schemas = {}
    pins = importlib.import_module("task101_evidence")
    for release in ("25", "26"):
        run("uci" + release, ["bash", ROOT / f"scripts/fetch-pinned-uci-2.{release[-1]}.sh", shared / ("uci" + release)])
        path = Path((root / ("uci" + release + ".log")).read_text().strip())
        include = path.with_name(f"UCI_SecurityMarkings_v2_{release[-1]}_0.xsd")
        if sha(path) != pins.PINS[release] or sha(include) != pins.INCLUDES[release]:
            raise RuntimeError("wrong UCI root/include")
        schemas[release] = dict(path=str(path), root=sha(path), include=sha(include))
    # Private venvs even within the same protocol-version pack.
    for name in names:
        directory = root / name
        evidence = directory / "evidence"
        evidence.mkdir(parents=True)
        (directory / "tmp").mkdir()
        (directory / "wrappers").mkdir()
        wrapper = directory / "wrappers/alr"
        wrapper.write_text("#!/bin/sh\nexec " + str(Path(sys.executable).resolve()) + " " +
                           str(Path(__file__).with_name("alr.py")) + ' "$@"\n')
        wrapper.chmod(0o755)
        relocate_setup(setup, evidence / "setup")
        (evidence / "source.txt").write_text(source["sha"] + "\n" + source["tree"] + "\n")
        shutil.copyfile(root / "platform.txt", evidence / "platform.txt")
        (evidence / "sleet-source.txt").write_text(REV + "\n" + TREE + "\n")
        shutil.copyfile(root / "sleet-build.log", evidence / "sleet-build.log")
        for release, schema in schemas.items():
            (evidence / f"uci{release}-root.txt").write_text(schema["path"] + "\n")
        run(name + "-venv", [sys.executable, "-m", "venv", directory / "venv"])
        version = INVENTORY["suites"][name]["websockets"]
        run(name + "-protocol", [directory / "venv/bin/pip", "install", "websockets==" + version])
    freeze(shared)
    return dict(generators=generators, schemas=schemas, sleet_commit=REV, sleet_tree=TREE,
                sleet_binary_sha256=sha(sleet / "target/release/sleet"),
                shared_manifest=manifest(shared), toolchain_manifest=manifest(setup / "cache/toolchains"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", choices=["a", "b"], required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--workers", type=int, choices=[2, 3, 4], required=True)
    parser.add_argument("--expected-sha", required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    args = parser.parse_args()
    source = identity(args.expected_sha)
    root = args.scratch.resolve()
    # Workflow writes platform diagnostics before invoking orchestration.
    if set(p.name for p in root.iterdir()) != {"platform.txt"}:
        raise RuntimeError("scratch must contain only fresh platform evidence")
    if subprocess.check_output(["alr", "--version"], text=True).strip() != "alr 2.1.1":
        raise RuntimeError("wrong Alire pin")
    names = suites(args.pack, os.environ["GITHUB_REF"] == "refs/heads/main")
    if not names:
        raise RuntimeError("pack has no applicable suites")
    cpus = sorted(os.sched_getaffinity(0))
    summary = dict(source=source, pack=args.pack, compiler=args.compiler, concurrency=args.workers,
                   runner_slots=1, applicable_suites=names, workers=[], exit=1)
    done, measurements = threading.Event(), []
    sampler = threading.Thread(target=monitor, args=(root, done, measurements))
    sampler.start()
    started = time.monotonic()
    for sig in (signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, lambda *_: STOP.set())
    try:
        summary["preparation"] = prepare(root, names, args.compiler, source)
        summary["preparation_seconds"] = time.monotonic() - started
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            futures = [pool.submit(worker, name, root / name, root / "shared", args.compiler,
                                   source, cpus[index % min(args.workers, len(cpus))])
                       for index, name in enumerate(names)]
            summary["workers"] = [future.result() for future in futures]
        identity(args.expected_sha)
        if STOP.is_set() or any(record["exit"] for record in summary["workers"]):
            raise RuntimeError("mandatory worker failed/incomplete")
        if manifest(root / "shared") != summary["preparation"]["shared_manifest"]:
            raise RuntimeError("shared dependency inputs changed")
        summary["exit"] = 0
    except Exception as exc:
        summary["error"] = str(exc)
    finally:
        done.wait(2)
        done.set()
        sampler.join()
        summary["elapsed_seconds"] = time.monotonic() - started
        summary["peak_host_memory_used_bytes"] = max((s["host_memory_used_bytes"] for s in measurements), default=0)
        summary["minimum_disk_free_bytes"] = min((s["disk_free_bytes"] for s in measurements), default=0)
        write(root / "summary.json", summary)
    raise SystemExit(summary["exit"])


if __name__ == "__main__":
    main()