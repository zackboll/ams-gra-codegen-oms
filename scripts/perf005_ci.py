"""Lightweight fail-closed CI checks for the R2-qualified reuse mechanism.

No strace or mapping filtering in timed CI. R2 establishes the versioned mapping
semantics; this path checks pinned identities and every forced build/output.
"""
import json
import os
from pathlib import Path
import re
import subprocess

from perf005_provider import digest, verify_variant

COMPILERS = {
    "14": ("14.2.0", "a23ecab8ff08f09ad8c80602c2c5df7f49e09c25905cb8975902e101bf72635f",
           "67082a9217f83ff443253d5d02973615b3f2cc01d47e527d857d87229f7c2c4f",
           "0730ad8a2656bc534b7f22aa8ba70901e683dd4ed1ed86458768ea4de37e3105",
           "b3f3af059c413cf9520c0c5e21df53d9f3ff0823b9f87f89cb936de3b196fa37"),
    "16": ("16.1.0", "238f4adafcb1425c84da6519068fdfcf33e57e406fc793e2958a9238789900de",
           "fa15577ec622065f1e80d18f14d315d8b3d9a6ff17cf195ffe4fe3895c1f84d9",
           "545339a0b49ad059358dcfcc69d47d04dfb3e5acb245cb39a1d590b13d693c95",
           "0ad4faf3482b659765f96e1cc6fc3dcb13c7206b6f366d565a5d3d04a78a756d"),
}
GPRBUILD_SHA256 = "aff9fde5ca766d5497da9dec185cb8b96878206a0d02512aa5685e1e56d0a4d3"
POLICIES = ("normal", "Check", "Ignore")
NAMES = ("ArraySample", "Array_Encode_Hex_Binary", "Encode_Hex_Binary",
         "Array_encode_hex_binary", "Array_Decode_Hex_Binary")


def available_jobs():
    jobs = len(os.sched_getaffinity(0))
    # Hosted container affinity can exceed its cgroup CPU quota.
    quota = Path("/sys/fs/cgroup/cpu.max")
    if quota.is_file():
        amount, period = quota.read_text().split()
        if amount != "max":
            jobs = min(jobs, max(1, int(amount) // int(period)))
    return max(1, jobs)


def validate_toolchain(output, compiler):
    lines = output.decode().splitlines()
    version, gcc_hash, gnat_hash, system_hash, runtime_hash = COMPILERS[compiler]
    hashes = {Path(name).name: value for value, name in re.findall(r"(?m)^([0-9a-f]{64})  (.+)$", output.decode())}
    expected = {"gcc": gcc_hash, "gnat": gnat_hash, "gprbuild": GPRBUILD_SHA256,
                "system.ads": system_hash, "libgnat.a": runtime_hash}
    if f"GNAT {version}" not in lines or any(hashes.get(name) != value for name, value in expected.items()):
        raise RuntimeError("CI toolchain/runtime identity differs from R2 qualification")


def validate_forced_build(command, consumer, text, started_ns):
    if "-f" not in command or "-XEXTERNALLY_BUILT=true" not in command:
        raise RuntimeError("CI forced external-provider build required")
    support = consumer.parent / (consumer.name + "-support")
    paths = list(consumer.glob("*.adb")) + list(consumer.glob("programs*.ads"))
    paths += list((support / "src").glob("*.adb")) + list((support / "float32").glob("*.adb"))
    paths += list((support / "float32").glob("*.c"))
    expected = {p.name for p in paths}
    units = re.findall(rb"\[(?:Ada|C)\]\s+(\S+\.(?:adb|ads|c))", text)
    units = [u.decode() for u in units]
    if len(paths) != 8 or len(expected) != 8 or len(units) != 8 or set(units) != expected:
        raise RuntimeError(f"CI generated/support compilation missing/extra: {units}")
    if not all(marker in text for marker in (b"[gprbind]", b"[link]")):
        raise RuntimeError("CI binding/linking evidence missing")
    artifacts = {}
    for source in paths:
        objdir = consumer / "obj" if source.parent == consumer else support / ("obj" if source.parent.name == "src" else "obj-float32")
        obj = objdir / (source.stem + ".o")
        if not obj.is_file() or obj.is_symlink() or obj.stat().st_mtime_ns < started_ns:
            raise RuntimeError(f"CI fresh generated/support object missing: {obj}")
        artifacts[str(obj)] = digest(obj)
        if source.suffix != ".c":
            ali = obj.with_suffix(".ali")
            if not ali.is_file() or ali.stat().st_mtime_ns < started_ns:
                raise RuntimeError(f"CI fresh ALI missing: {ali}")
            artifacts[str(ali)] = digest(ali)
    executable = consumer / "bin/helper_demand_consumer"
    if not executable.is_file() or executable.stat().st_mtime_ns < started_ns:
        raise RuntimeError("CI fresh linked consumer missing")
    artifacts[str(executable)] = digest(executable)
    return dict(compiled_sources={str(p): digest(p) for p in paths}, artifacts=artifacts,
                forced=True, bind=True, link=True)


def verify_campaign(root, compiler):
    result = json.loads((root / "result.json").read_text())
    expected = [(profile, name, policy) for profile in ("oam", "qualified") for name in NAMES for policy in POLICIES]
    records = result.get("records", [])
    if (result.get("completed") is not True or result.get("red") is not False
            or [(r["profile"], r["name"], r["policy"]) for r in records] != expected
            or any(r["compiler"] != COMPILERS[compiler][0] or r["compiler_exit"] != 0 for r in records)):
        raise RuntimeError("CI incomplete/wrong helper-demand campaign")
    provenance = json.loads((root / "perf005-provider-consumers.json").read_text())
    if [r["cell"] for r in provenance] != [f"{p}-{n}-{a}-build" for p, n, a in expected]:
        raise RuntimeError("CI missing provider-consumer provenance")
    prep = []
    for policy in POLICIES:
        variant = root / "perf005-providers" / policy
        record = json.loads((variant / "complete.json").read_text())
        verify_variant(variant, record["identity"])
        if record["identity"]["compiler"] != compiler or record["identity"]["policy"] != policy:
            raise RuntimeError("CI wrong provider policy/compiler")
        if any(p.stat().st_mode & 0o222 for p in (variant / "artifacts").rglob("*")):
            raise RuntimeError("CI provider artifacts are not immutable")
        profile = json.loads((root / f"provider-{policy}-build.profile.json").read_text())
        log = (root / f"provider-{policy}-build.log").read_bytes()
        if (profile["exit"] != 0 or "-f" not in profile["command"]
                or len(re.findall(rb"\[(?:Ada|C|Asm)\]\s+", log)) != 106
                or not all(s in log for s in (b"libgnatcoll_core.a", b"libgnatcoll_minimal.a"))):
            raise RuntimeError("CI incomplete fresh provider preparation")
        prep.append(dict(policy=policy, elapsed_seconds=profile["elapsed_seconds"],
                         cpu_seconds=profile["cpu_seconds"], child_maxrss_kib=profile["child_maxrss_kib"]))
    for index, (profile, name, policy) in enumerate(expected):
        cell = f"{profile}-{name}-{policy}"
        invocation = json.loads((root / f"{cell}-build.profile.json").read_text())
        verified = json.loads((root / f"{cell}-build.verification.json").read_text())
        variant = root / "perf005-providers" / policy
        if (invocation["exit"] != 0 or len(verified["compiled_sources"]) != 8
                or provenance[index]["manifest_sha256"] != digest(variant / "complete.json")
                or provenance[index]["policy"] != policy):
            raise RuntimeError("CI missing forced compilation/provider evidence")
        if (root / f"{cell}-execute.log").read_bytes() != b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n":
            raise RuntimeError("CI helper execution marker differs")
    for profile in ("oam", "qualified"):
        for name in NAMES:
            label = f"{profile}-{name}"
            for suffix in ("portable-reject", "portable-generate-reject"):
                invocation = json.loads((root / f"{label}-{suffix}.profile.json").read_text())
                if invocation["exit"] != 1 or not (root / f"{label}-{suffix}.log").is_file():
                    raise RuntimeError("CI missing negative CLI case")
            if (root / f"{label}-rejected").exists():
                raise RuntimeError("CI non-atomic negative CLI rejection")
    subprocess.run(["sha256sum", "--check", "--quiet", str(root / "sources.sha256")], check=True)
    total = json.loads((root / "perf005-total.json").read_text())
    report = dict(completed=True, compiler=COMPILERS[compiler][0], provider_constructions=3,
                  positive_cells=30, negative_cli_cases=20, generated_support_compilations=240,
                  bind_link_cells=30, execution_cells=30, preparation=prep, campaign=total,
                  mapping_qualification="PERF-005-R2: concrete runtime-cache equivalence; no filtering",
                  detailed_trace=False)
    (root / "ci-verification.json").write_text(json.dumps(report, indent=2) + "\n")
    return report