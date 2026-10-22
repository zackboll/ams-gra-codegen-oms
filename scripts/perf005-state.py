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
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--index", type=Path, required=True, help="community directory containing index.toml and repo")
    parser.add_argument("--toolchains", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=False)
    (root / "home").mkdir()
    (root / "bin").mkdir()
    shutil.copytree(args.index, root / "settings/indexes/community")
    identities = {}
    for name in ("gnat_native_16.1.0_9f74f58a", "gprbuild_26.0.1_e3f27f25"):
        source = args.toolchains / name
        shutil.copytree(source, root / "cache/toolchains" / name, symlinks=True)
        identities[name] = {str(p.relative_to(source)): hashlib.sha256(p.read_bytes()).hexdigest()
                            for p in source.rglob("*") if p.is_file()}
    (root / "settings/settings.toml").write_text(
        f'[cache]\ndir = "{root}/cache"\n[index]\nauto_update = 0\n[toolchain]\nassistant = false\n')
    (root / "bin/alr").write_text(f'#!/bin/sh\nexec /usr/local/bin/alr -s "{root}/settings" "$@"\n')
    (root / "bin/alr").chmod(0o755)
    index_commit = subprocess.check_output(["git", "-C", str(args.index / "repo"), "rev-parse", "HEAD"], text=True).strip()
    (root / "seed.json").write_text(json.dumps(dict(index_commit=index_commit, toolchain_files=identities), indent=2) + "\n")
    print(f"HOME={root}/home PATH={root}/bin:$PATH")


if __name__ == "__main__":
    main()