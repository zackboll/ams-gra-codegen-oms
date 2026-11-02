#!/usr/bin/env python3
"""One source-bound setup, complete JSON guard, then isolated Ada workers."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import glob
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
INVENTORY = json.loads(Path(__file__).with_name("inventory.json").read_text())
STOP = threading.Event()


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n")


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(expected):
    git = lambda ref: subprocess.check_output(["git", "rev-parse", ref], cwd=ROOT, text=True).strip()
    if git("HEAD") != expected:
        raise RuntimeError("source identity mismatch")
    if subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=ROOT):
        raise RuntimeError("qualification source is dirty")
    for name, digest in INVENTORY["qualification_sha256"].items():
        if sha(ROOT / name) != digest:
            raise RuntimeError("qualification input mismatch: " + name)
    return dict(sha=expected, tree=git("HEAD^{tree}"),
                run_id=os.environ.get("GITHUB_RUN_ID"), attempt=os.environ.get("GITHUB_RUN_ATTEMPT"))


def execute(command, env, directory, stem, timeout, cpu=None):
    """Reap each process group on failure/timeout/cancellation; never lose its exit."""
    started = time.monotonic()
    if cpu is not None:
        command = ["taskset", "--cpu-list", str(cpu), *command]
    with stem.with_suffix(".log").open("wb") as log:
        process = subprocess.Popen(command, cwd=directory, env=env, stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        reason = None
        while process.poll() is None:
            if STOP.is_set() or time.monotonic() - started > timeout:
                reason = "terminated" if STOP.is_set() else "timeout"
                break
            time.sleep(0.1)
        # Also terminate background descendants after their parent exits.
        try:
            os.killpg(process.pid, signal.SIGTERM)
            time.sleep(0.2)
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        code = process.wait()
    record = dict(command=command, exit=code, reason=reason, elapsed_seconds=time.monotonic() - started)
    write(stem.with_suffix(".json"), record)
    stem.with_suffix(".exit").write_text(str(code) + "\n")
    if code or reason:
        raise RuntimeError(f"command failed: {record}; log={stem.with_suffix('.log')}")
    return record


def environment(directory, real_alr, seed=None):
    directory.mkdir(parents=True, exist_ok=False)
    settings, cache = directory / "settings", directory / "cache"
    if seed:
        shutil.copytree(seed / "settings", settings)
        shutil.copytree(seed / "cache", cache, ignore=shutil.ignore_patterns("builds", "tmp"))
    else:
        settings.mkdir()
        cache.mkdir()
    env = dict(os.environ, PACK_REAL_ALR=real_alr, PACK_SETTINGS=str(settings),
               PACK_ALR_TRACE=str(directory / "alr.jsonl"), PYTHONDONTWRITEBYTECODE="1",
               CARGO_BUILD_JOBS="1", CARGO_NET_OFFLINE="true", RAYON_NUM_THREADS="1",
               OMP_NUM_THREADS="1", MAKEFLAGS="-j1", TMPDIR=str(directory / "tmp"))
    Path(env["TMPDIR"]).mkdir()
    for key, value in (("cache.dir", str(cache)), ("toolchain.dir", str(cache / "toolchains")),
                       ("index.auto_update", "0"), ("dependencies.shared", "false")):
        subprocess.run([real_alr, "--settings", str(settings), "-n", "settings", "--global",
                        "--set", key, value], env=env, cwd=directory, check=True)
    bindir = directory / "bin"
    bindir.mkdir()
    wrapper = bindir / "alr"
    wrapper.write_text("#!/bin/sh\nexec python3 " + str(Path(__file__).with_name("alr.py")) + ' "$@"\n')
    wrapper.chmod(0o755)
    env["PATH"] = str(bindir) + os.pathsep + env["PATH"]
    env["RUNNER_TEMP"] = str(directory)
    env["GITHUB_WORKSPACE"] = str(ROOT)
    return env


def original_steps(name, directory, env, cpu):
    results = []
    deadline = time.monotonic() + INVENTORY["suites"][name]["timeout_seconds"]
    for i, step in enumerate(INVENTORY["suites"][name]["steps"]):
        result = execute(["bash", "-euo", "pipefail", "-c", step["run"]], env, ROOT,
                         directory / f"step-{i}", max(0, deadline - time.monotonic()), cpu)
        results.append(dict(name=step["name"], **result))
    return results


def verify_artifacts(name, directory):
    """Unlike upload-artifact, require every original artifact pattern to match."""
    artifact = INVENTORY["suites"][name]["artifact"]
    for pattern in artifact["path"].splitlines():
        pattern = pattern.replace("${{ runner.temp }}", str(directory))
        if not any(Path(p).is_file() and Path(p).stat().st_size for p in glob.glob(pattern, recursive=True)):
            raise RuntimeError("missing artifact: " + pattern)


def verify_markers(name, directory):
    """Independent completion inventory, additional to all original assertions."""
    requirements = {
        "ada-json-guard": [("task087a-qualification/*-run.log", "GUARDED CORPUS PASSED cases= 199", 3),
                           ("task087a-qualification/consumer-run.log", "TASK087A CLEAN CONSUMER: PASSED", 1)],
        "ada-generated-codec": [("task087b-codec/*-run.log", "TASK087B_CODEC_COMPLETE cases= 41", 3),
                                ("task087b-codec/subset-*-run.log", "TASK087B_SUBSET_COMPLETE cases= 7", 3),
                                ("task088-choice/*-run.log", "TASK088_CODEC_COMPLETE cases= 172", 3),
                                ("task088-choice/*-run.log", "TASK088_QUALIFIED_COMPLETE cases= 2", 3)],
        "ada-repeated-temporal-codec": [(f"{folder}/*-execute.log", marker, 6) for folder, marker in [
            ("task094", "TASK094_CODEC_COMPLETE checks= 54 rejections= 17"),
            ("task094-composition", "TASK094_COMPOSITION_COMPLETE checks= 19 rejections= 3"),
            ("task094-combined", "TASK094_COMBINED_COMPLETE checks= 26 rejections= 2")]],
        "ada-datetime-codec": [("task090/named-*-run.log", "TASK090_NAMED_COMPLETE cases= 161", 3),
                               ("task090/direct-*-run.log", "TASK090_DIRECT_COMPLETE cases= 205", 3),
                               ("task090r/*-run.log", "TASK090R_COMBINED_PASS", 3)],
        "ada-hexbinary-codec": [(f"{folder}/*-run.log", marker, count) for folder, marker, count in [
            ("task093-hexbinary", "TASK093_CODEC_COMPLETE cases= 33", 3),
            ("task093-hexbinary", "TASK093_MODEL_COMPLETE cases= 6", 3),
            ("task093-qualified", "TASK093_CODEC_COMPLETE cases= 33", 3),
            ("task093-qualified", "TASK093_MODEL_COMPLETE cases= 6", 3),
            ("task093-combined", "TASK093_COMBINED_COMPLETE checks= 49 rejections= 9", 6)]] + [
                ("task093-c4-16/*-execute.log", "TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7", 30)],
        "ada-repeated-codec": [("task089-codec/*-codec-run.log", "TASK089_CODEC_COMPLETE cases= 121", 3),
                               ("task089-codec/*-model-run.log", "TASK089_MODEL_BASELINE_PASS", 3),
                               ("task089r-codec/*-run.log", "TASK089R_COMBINED_PASS", 3)],
        "ada-duration-codec": [("task091-duration/*-run.log", "TASK091_CODEC_COMPLETE cases= 650", 9),
                               ("task091-duration/*-run.log", "TASK091_MODEL_COMPLETE cases= 10", 3),
                               ("task091r-integrated/*-run.log", "TASK091R_COMBINED_PASS checks= 52 rejections= 20", 3)],
        "ada-repeated-choice-codec": [(f"{folder}/*-run.log", "TASK092_CODEC_PASS checks= 63 rejections= 17", 3)
                                      for folder in ("task092-codec", "task092-qualified")],
    }
    evidence = []
    for pattern, marker, count in requirements[name]:
        matches = [p for p in directory.glob(pattern)
                   if sum(re.fullmatch(marker, line) is not None for line in p.read_text().splitlines()) == 1]
        if len(matches) != count:
            raise RuntimeError(f"missing/incorrect completion evidence: {pattern} {marker}: {len(matches)} != {count}")
        evidence.extend(dict(path=str(p.relative_to(directory)), sha256=sha(p), marker=marker) for p in matches)
    return evidence


def worker(name, directory, env, cpu, source):
    started = time.monotonic()
    record = dict(suite=name, cpu=cpu, source=source, exit=1)
    try:
        record["steps"] = original_steps(name, directory, env, cpu)
        verify_artifacts(name, directory)
        record["evidence"] = verify_markers(name, directory)
        record["exit"] = 0
    except Exception as exc:
        record["error"] = str(exc)
    record["elapsed_seconds"] = time.monotonic() - started
    write(directory / "completion.json", record)
    print(json.dumps(record), flush=True)
    return record


def monitor(directory, done, measurements):
    """Sample occupied host resources, not queue time; retain raw samples."""
    previous = None
    with (directory / "resources.jsonl").open("w") as log:
        while not done.is_set():
            cpu = list(map(int, Path("/proc/stat").read_text().splitlines()[0].split()[1:]))
            memory = {line.split(":")[0]: int(line.split()[1]) * 1024
                      for line in Path("/proc/meminfo").read_text().splitlines() if line.split()[1].isdigit()}
            disk = shutil.disk_usage(directory)
            busy = None if previous is None else 1 - ((cpu[3] + cpu[4] - previous[3] - previous[4]) / max(1, sum(cpu) - sum(previous)))
            sample = dict(time=time.time(), cpu_busy_fraction=busy,
                          host_memory_used_bytes=memory["MemTotal"] - memory["MemAvailable"],
                          disk_used_bytes=disk.used, disk_free_bytes=disk.free)
            measurements.append(sample)
            log.write(json.dumps(sample) + "\n")
            log.flush()
            previous = cpu
            done.wait(2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", choices=["a", "b"], required=True)
    parser.add_argument("--workers", type=int, choices=[2, 3, 4], required=True)
    parser.add_argument("--expected-sha", required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    args = parser.parse_args()
    source = identity(args.expected_sha)
    cpus = sorted(os.sched_getaffinity(0))
    if os.environ.get("GITHUB_ACTIONS") == "true" and len(cpus) != 4:
        raise RuntimeError(f"benchmark/pack requires hosted four-vCPU affinity, got {cpus}")
    if len(cpus) < args.workers:
        raise RuntimeError("insufficient CPU affinity")
    real_alr = shutil.which("alr")
    if not real_alr or subprocess.check_output([real_alr, "--version"]).strip() != b"alr 2.1.1":
        raise RuntimeError("verified Alire 2.1.1 required")
    directory = args.scratch.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    for sig in (signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, lambda *_: STOP.set())
    done, measurements = threading.Event(), []
    sampler = threading.Thread(target=monitor, args=(directory, done, measurements))
    sampler.start()
    started = time.monotonic()
    summary = dict(source=source, pack=args.pack, concurrency=args.workers, allowed_cpus=cpus,
                   runner_slots=1, workers=[], exit=1)
    try:
        seed = directory / "seed"
        env = environment(seed, real_alr)
        build = execute(["cargo", "+1.95.0", "build", "--locked", "-j", "4", "-p", "ams-gra-codegen-oms"],
                        dict(env, CARGO_NET_OFFLINE="false"), ROOT, directory / "generator-build", 1200)
        summary["generator_build_seconds"] = build["elapsed_seconds"]
        summary["generator_sha256"] = sha(ROOT / "target/debug/ams-gra-codegen-oms")
        summary["guard"] = worker("ada-json-guard", seed, env, cpus[0], source)
        if summary["guard"]["exit"]:
            raise RuntimeError("JSON guard prerequisite failed")
        summary["setup_seconds"] = time.monotonic() - started
        # Copy sequentially before execution: no writable cache or configuration is shared.
        prepared = []
        for name in INVENTORY["packs"][args.pack]:
            workdir = directory / name
            prepared.append((name, workdir, environment(workdir, real_alr, seed)))
        summary["isolation_copy_seconds"] = time.monotonic() - started - summary["setup_seconds"]
        # A fixed CPU belongs to each executor thread even with the two-worker fallback.
        local = threading.local()
        available = iter(cpus[:args.workers])
        lock = threading.Lock()

        def run(item):
            if not hasattr(local, "cpu"):
                with lock:
                    local.cpu = next(available)
            return worker(*item, local.cpu, source)

        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            summary["workers"] = list(executor.map(run, prepared))
        identity(args.expected_sha)
        if STOP.is_set() or any(item["exit"] for item in summary["workers"]):
            raise RuntimeError("one or more mandatory workers failed")
        summary["exit"] = 0
    except Exception as exc:
        summary["error"] = str(exc)
    finally:
        summary["elapsed_seconds"] = time.monotonic() - started
        done.set()
        sampler.join()
        summary["peak_host_memory_used_bytes"] = max((s["host_memory_used_bytes"] for s in measurements), default=0)
        summary["peak_disk_used_bytes"] = max((s["disk_used_bytes"] for s in measurements), default=0)
        summary["minimum_disk_free_bytes"] = min((s["disk_free_bytes"] for s in measurements), default=0)
        summary["child_maxrss_kib"] = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
        summary["mean_host_cpu_busy_fraction"] = sum(s["cpu_busy_fraction"] or 0 for s in measurements) / max(1, len(measurements) - 1)
        write(directory / "summary.json", summary)
    raise SystemExit(summary["exit"])


if __name__ == "__main__":
    main()