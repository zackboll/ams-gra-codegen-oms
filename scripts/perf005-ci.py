#!/usr/bin/env python3
"""Run only the qualified Float32 helper workload with campaign-local reuse.

One private mutable Alire state, two sequential compiler campaigns, no compiled
provider seeds. Ordinary Task093/095 entry points are unchanged.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import resource
import shutil
import subprocess
import sys
import tarfile
import time

from perf005_ci import available_jobs
from perf005_provider import digest
from perf005_sandbox import validate_layout

ROOT = Path(__file__).resolve().parents[1]


def source_identity(root):
    # Container checkout ownership may differ from the process user. Trust only
    # this exact caller-selected checkout for these read-only queries, not '*'
    # or a persistent global exception. Query before expensive qualification.
    command = ["git", "-c", f"safe.directory={root}", "rev-parse"]
    return tuple(subprocess.check_output([*command, ref], cwd=root, text=True).strip()
                 for ref in ("HEAD", "HEAD^{tree}"))


def alire_locations(alr):
    text = subprocess.check_output([str(alr), "version"], text=True)
    locations = {}
    for key in ("settings folder", "toolchain folder"):
        values = [line.split(":", 1)[1].strip() for line in text.splitlines() if line.startswith(key + ":")]
        if len(values) != 1 or not Path(values[0]).is_dir():
            raise RuntimeError(f"missing Alire {key}")
        locations[key] = Path(values[0])
    return locations


def archive_campaign(campaign, destination):
    # tar preserves case-sensitive naming cases that upload-artifact merges.
    with tarfile.open(destination, "w:gz", compresslevel=1) as archive:
        archive.add(campaign, arcname=campaign.name)
    if not destination.is_file() or destination.stat().st_size == 0:
        raise RuntimeError("missing campaign evidence archive")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compilers", nargs="+", choices=("14", "16"), required=True)
    parser.add_argument("--runtime-root", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--alr", type=Path, required=True)
    args = parser.parse_args()
    if args.compilers != ["14", "16"]:
        parser.error("CI requires both compilers, in order: --compilers 14 16")
    commit, tree = source_identity(ROOT)
    runtime = args.runtime_root.resolve()
    evidence = args.evidence.resolve()
    state = runtime / "state"
    campaigns = runtime / "campaigns"
    validate_layout([state, state / "home", Path("/tmp"), Path("/dev"), evidence],
                    [campaigns / f"task093-c4-{c}/perf005-providers" for c in args.compilers])
    runtime.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    locations = alire_locations(args.alr)
    spec = importlib.util.spec_from_file_location("perf005_state", ROOT / "scripts/perf005-state.py")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    seed = helper.create_state(locations["settings folder"] / "indexes/community",
                               locations["toolchain folder"], state, args.alr)
    shutil.copy2(state / "seed.json", evidence / "seed.json")
    setup_seconds = time.monotonic() - started
    setup_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
    setup_self_cpu = time.process_time()
    env = dict(os.environ, HOME=str(state / "home"), PATH=str(state / "bin") + ":" + os.environ["PATH"],
               PYTHONDONTWRITEBYTECODE="1")
    reports = []
    try:
        for compiler in args.compilers:
            campaign = campaigns / f"task093-c4-{compiler}"
            command = [sys.executable, str(ROOT / "scripts/perf005-float32.py"), "--ci", "--reuse-provider", "--",
                       "--family", "float32", "--compiler", compiler, "--scratch", str(campaign),
                       "--generator", str(args.generator.resolve())]
            subprocess.run(command, env=env, cwd=ROOT, check=True)
            report = json.loads((campaign / "ci-verification.json").read_text())
            reports.append(report)
            for filename in ("ci-verification.json", "perf005-total.json", "result.json", "sources.sha256"):
                shutil.copy2(campaign / filename, evidence / f"{compiler}-{filename}")
    finally:
        # Keep all diagnostics even on failure; do not turn archival into success.
        for campaign in sorted(campaigns.glob("task093-c4-*")):
            archive_campaign(campaign, evidence / f"{campaign.name}.tar.gz")
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    summary = dict(completed=True, implementation_commit=commit, source_tree=tree,
        runner=dict(name=os.environ.get("RUNNER_NAME"), os=platform.platform(),
                    cpu_affinity=sorted(os.sched_getaffinity(0)), jobs=available_jobs(),
                    cpu_model=next((s for s in Path("/proc/cpuinfo").read_text().splitlines() if s.startswith("model name")), None)),
        setup_seconds=setup_seconds,
        setup_self_cpu_seconds=setup_self_cpu,
        setup_child_cpu_seconds=setup_cpu.ru_utime + setup_cpu.ru_stime - before.ru_utime - before.ru_stime,
        elapsed_seconds_including_setup_and_archival=time.monotonic() - started,
        helper_campaign_seconds=sum(r["campaign"]["elapsed_seconds"] for r in reports),
        child_cpu_seconds=after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
        child_maxrss_kib=after.ru_maxrss, self_maxrss_kib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
        memory_method="kernel largest-child high-water RSS, not simultaneous process-tree RSS",
        provider_constructions=sum(r["provider_constructions"] for r in reports),
        positive_cells=sum(r["positive_cells"] for r in reports),
        negative_cli_cases=sum(r["negative_cli_cases"] for r in reports),
        generated_support_compilations=sum(r["generated_support_compilations"] for r in reports),
        execution_cells=sum(r["execution_cells"] for r in reports),
        index_commit=seed["index_commit"], alire_sha256=seed["alire_sha256"], compilers=reports)
    if (summary["provider_constructions"], summary["positive_cells"], summary["negative_cli_cases"],
            summary["generated_support_compilations"], summary["execution_cells"]) != (6, 60, 40, 480, 60):
        raise RuntimeError("incomplete combined CI provider-reuse qualification")
    (evidence / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    (evidence / "evidence.sha256").write_text("".join(f"{digest(p)}  {p}\n" for p in sorted(evidence.iterdir()) if p.is_file()))
    subprocess.run(["sha256sum", "--check", "--quiet", str(evidence / "evidence.sha256")], check=True)
    lines = ["## PERF-005 Float32 provider reuse", "", "| Compiler | Helper seconds | Preparation seconds | Preparation CPU seconds |",
             "|---|---:|---:|---:|"]
    for report in reports:
        lines.append(f"| {report['compiler']} | {report['campaign']['elapsed_seconds']:.3f} | "
                     f"{sum(p['elapsed_seconds'] for p in report['preparation']):.3f} | "
                     f"{sum(p['cpu_seconds'] for p in report['preparation']):.3f} |")
    lines += ["", f"Providers: **60 → 6**; positive/execution cells: **60**; negative CLI cases: **40**; "
              f"forced generated/support compilations: **480**.",
              f"Setup: {setup_seconds:.3f}s; setup + campaigns + archival: {summary['elapsed_seconds_including_setup_and_archival']:.3f}s.",
              f"Largest child RSS: {after.ru_maxrss} KiB (not simultaneous tree RSS).",
              f"Implementation: `{summary['implementation_commit']}`; jobs: {summary['runner']['jobs']}.",
              "Complete job duration includes the unchanged campaigns, upload and teardown; use the hosted Jobs API after completion.",
              "Single hosted observation, not a controlled statistical benchmark."]
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with Path(os.environ["GITHUB_STEP_SUMMARY"]).open("a") as output:
            output.write("\n".join(lines) + "\n")
    print("PERF005_CI_COMPLETE providers=6 positive=60 negative=40 compiled=480 executed=60", flush=True)


if __name__ == "__main__":
    main()