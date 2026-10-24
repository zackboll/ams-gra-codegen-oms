#!/usr/bin/env python3
"""Create private Alire state from an index and exact toolchain-only seeds.

No provider objects/releases or mutable settings are imported. The source index
and toolchain files are copied, not linked. Run outside measured campaign time;
use the same seed condition on both sides. Provider preparation is NOT a seed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess


def create_state(index, toolchains, root, alr):
    """Copy only index/toolchains; never import provider objects or releases."""
    alr = alr.resolve(strict=True)
    if subprocess.check_output([str(alr), "--version"], text=True).strip() != "alr 2.1.1":
        raise RuntimeError("Alire 2.1.1 required")
    if hashlib.sha256(alr.read_bytes()).hexdigest() != "2e2b8de009b793ad2aed10096e162649bbcc5b0711e3c21785ebe8a1261c5d64":
        raise RuntimeError("Alire executable differs from verified release")
    root = root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    (root / "home").mkdir()
    (root / "bin").mkdir()
    shutil.copytree(index, root / "settings/indexes/community")
    identities = {}
    for name in ("gnat_native_16.1.0_9f74f58a", "gprbuild_26.0.1_e3f27f25"):
        source = toolchains / name
        shutil.copytree(source, root / "cache/toolchains" / name, symlinks=True)
        identities[name] = {str(p.relative_to(source)): hashlib.sha256(p.read_bytes()).hexdigest()
                            for p in source.rglob("*") if p.is_file()}
    (root / "settings/settings.toml").write_text(
        f'[cache]\ndir = "{root}/cache"\n[index]\nauto_update = 0\n[toolchain]\nassistant = false\n')
    (root / "bin/alr").write_text(f'#!/bin/sh\nexec {shlex.quote(str(alr))} -s {shlex.quote(str(root / "settings"))} "$@"\n')
    (root / "bin/alr").chmod(0o755)
    index_commit = subprocess.check_output(["git", "-C", str(index / "repo"), "rev-parse", "HEAD"], text=True).strip()
    record = dict(index_commit=index_commit, alire_executable=str(alr),
                  alire_sha256=hashlib.sha256(alr.read_bytes()).hexdigest(), toolchain_files=identities)
    (root / "seed.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--index", type=Path, required=True, help="community directory containing index.toml and repo")
    parser.add_argument("--toolchains", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--alr", type=Path, default=shutil.which("alr"), help="verified Alire selected by the caller")
    args = parser.parse_args()
    root = args.output.resolve()
    if args.alr is None:
        parser.error("verified Alire executable required")
    create_state(args.index, args.toolchains, root, args.alr)
    print(f"HOME={root}/home PATH={root}/bin:$PATH")


if __name__ == "__main__":
    main()