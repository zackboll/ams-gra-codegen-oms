#!/usr/bin/env python3
"""Independently reconcile exact pack inputs, all cells, exits and artifacts."""
import argparse
import glob
import json
import os
from pathlib import Path
import shutil

try:
    from . import run
except ImportError:
    import run


def artifact_files(name, directory, shared, compiler):
    specification = run.INVENTORY["suites"][name]["artifact"]
    patterns = run.render(specification["path"], name, directory, shared, compiler).splitlines()
    files = set()
    for pattern in patterns:
        path = Path(pattern)
        if not path.is_absolute():
            pattern = str(run.ROOT / pattern)
        files.update(Path(p) for p in glob.glob(pattern, recursive=True) if Path(p).is_file())
    if not files:
        raise RuntimeError("original artifact has no files: " + name)
    # Original globs describe optional file kinds. Require their original actual
    # mandatory bytes through the original source-bound evidence verifier.
    return sorted(files)


def verify(root, expected_sha, expected_tree, pack, compiler, main):
    summary = json.loads((root / "summary.json").read_text())
    source = summary["source"]
    if source != dict(sha=expected_sha, tree=expected_tree,
                      run_id=os.environ["GITHUB_RUN_ID"], attempt=os.environ["GITHUB_RUN_ATTEMPT"]):
        raise RuntimeError("pack source/tree/run/attempt mismatch")
    names = run.suites(pack, main)
    if (summary["exit"] != 0 or summary["pack"] != pack or summary["compiler"] != compiler
            or summary["runner_slots"] != 1 or summary["applicable_suites"] != names
            or [w["suite"] for w in summary["workers"]] != names):
        raise RuntimeError("failed/incomplete pack inventory")
    preparation = summary["preparation"]
    if preparation["sleet_commit"] != run.REV or preparation["sleet_tree"] != run.TREE:
        raise RuntimeError("wrong Sleet identity")
    if run.git("rev-parse", "HEAD", "HEAD^{tree}", cwd=root / "shared/sleet").splitlines() != [run.REV, run.TREE]:
        raise RuntimeError("Sleet checkout mismatch")
    if run.manifest(root / "shared") != preparation["shared_manifest"]:
        raise RuntimeError("changed/writable shared inputs")
    if run.manifest(root / "setup/cache/toolchains") != preparation["toolchain_manifest"]:
        raise RuntimeError("changed/writable compiler inputs")
    for profile, digest in preparation["generators"].items():
        if run.sha(run.ROOT / f"target/{profile}/ams-gra-codegen-oms") != digest:
            raise RuntimeError("generator identity mismatch")
    pins = run.importlib.import_module("task101_evidence")
    for release, schema in preparation["schemas"].items():
        path = Path(schema["path"])
        if (schema["root"] != pins.PINS[release] or run.sha(path) != pins.PINS[release]
                or schema["include"] != pins.INCLUDES[release]
                or run.sha(path.with_name(f"UCI_SecurityMarkings_v2_{release[-1]}_0.xsd")) != pins.INCLUDES[release]):
            raise RuntimeError("UCI root/include mismatch")
    for name, recorded in zip(names, summary["workers"]):
        directory = root / name
        if run.manifest(directory / "evidence/setup/cache/toolchains") != preparation["toolchain_manifest"]:
            raise RuntimeError("worker compiler distribution changed")
        completed = json.loads((directory / "completion.json").read_text())
        steps = run.INVENTORY["suites"][name]["steps"]
        if (completed != recorded or completed["exit"] != 0 or completed["source"] != source
                or completed["compiler"] != compiler or len(completed["steps"]) != len(steps)):
            raise RuntimeError("failed/incomplete worker: " + name)
        for index, step in enumerate(steps):
            record = json.loads((directory / f"step-{index}.json").read_text())
            if (record["exit"] != 0 or record["reason"] is not None
                    or (directory / f"step-{index}.exit").read_text() != "0\n"
                    or record["command"][-1] != run.render(step["run"], name, directory, root / "shared", compiler)
                    or completed["steps"][index] != dict(name=step["name"], **record)):
                raise RuntimeError("step failure/mismatch: " + name)
        if run.verify_suite(name, directory, compiler, source) != completed["evidence"]:
            raise RuntimeError("original evidence mismatch: " + name)
        trace = [json.loads(line) for line in (directory / "alr.jsonl").read_text().splitlines()]
        if not trace:
            raise RuntimeError("missing private Alire trace")
        for argv in trace:
            if "build" in argv and argv[argv.index("--") + 1] != "-j1":
                raise RuntimeError("unbounded nested Alire compilation")
        if name == "ada-real-uci-dlz-sleet":
            from owned_ada_evidence import verify_bundle
            verify_bundle(run.bundle(directory / "owned"), "pinned", compiler,
                          source["sha"], source["tree"], source["run_id"], source["attempt"])
        artifact_files(name, directory, root / "shared", compiler)
    samples = [json.loads(line) for line in (root / "resources.jsonl").read_text().splitlines()]
    if len(samples) < 2 or any(row["disk_free_bytes"] <= 0 for row in samples):
        raise RuntimeError("missing/invalid resource diagnostics")
    run.identity(expected_sha)
    return dict(source=source, pack=pack, compiler=compiler, verified_suites=names)


def publish(root, pack, compiler, main):
    """Stage the original artifact payloads even after a failed campaign."""
    for name in run.suites(pack, main):
        directory = root / name
        output = root / "artifacts" / str(run.INVENTORY["suites"][name]["task"])
        output.mkdir(parents=True, exist_ok=False)
        files = set(artifact_files(name, directory, root / "shared", compiler))
        # Original source verifiers also require ADC/license/input bytes. Keep a
        # superset of the original payload, never drop a newly observed suffix.
        files.update(directory / "evidence" / p for p in run.bundle(directory / "evidence"))
        for path in sorted(files):
            # Preserve the absolute path hierarchy used by upload-artifact's
            # original mixed repository + /tmp least-common-ancestor layout.
            destination = output / str(path).lstrip("/")
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--pack", choices=["a", "b"], required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--expected-sha", required=True)
    parser.add_argument("--expected-tree", required=True)
    parser.add_argument("--publish-only", action="store_true")
    args = parser.parse_args()
    main = os.environ["GITHUB_REF"] == "refs/heads/main"
    if args.publish_only:
        publish(args.scratch, args.pack, args.compiler, main)
    else:
        result = verify(args.scratch, args.expected_sha, args.expected_tree, args.pack, args.compiler, main)
        run.write(args.scratch / "independently-verified.json", result)
        print(json.dumps(result))