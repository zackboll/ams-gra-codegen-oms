#!/usr/bin/env python3
"""Stage one already-qualified build.sh artifact; never build or regenerate."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
import check as deploy

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def stage(build, out, args):
    deploy.require(not out.is_relative_to(REPO) and not out.is_relative_to(build),
                   "bundle destination must be outside the source checkout and build directory")
    manifest = build / f"build-{deploy.artifact(args)}.manifest"
    source = deploy.build_manifest(manifest, args)
    build_sha = deploy.sha(manifest)
    src = build / deploy.binary_path(args)
    deploy.require(deploy.sha(src) == source["binary_sha256"], "binary SHA-256 mismatch")
    deploy.require(deploy.sha(REPO / source["contract"]) == source["contract_sha256"], "contract SHA-256 mismatch")
    generated = build / "generated" / deploy.artifact(args)
    for name in deploy.HEADERS:
        deploy.require(deploy.sha(generated / f"{name}.hpp") == source[f"generated_{name}_sha256"],
                       f"generated {name}.hpp SHA-256 mismatch")
    linkage = deploy.elf(src, args.variant)

    # Exclusive mkdir: never replace an existing path (including a symlink).
    # The parent is trusted and must exist. Deliver only after STAGED succeeds.
    out.mkdir(mode=0o755)
    try:
        (out / "bin").mkdir()
        (out / "manifest").mkdir()
        binary = out / deploy.binary_path(args)
        for origin, target, mode in ((src, binary, 0o755), (manifest, out / deploy.BUILD, 0o644),
                                     (HERE / "check.py", out / "check.py", 0o644),
                                     (HERE / "README.txt", out / "README.txt", 0o644),
                                     (REPO / "LICENSE", out / "LICENSE", 0o644)):
            shutil.copyfile(origin, target)
            target.chmod(mode)
        (out / deploy.DEPLOY).write_text(deploy.deployment(args, source, build_sha, linkage), encoding="utf-8")
        (out / deploy.SUMS).write_text(deploy.checksums(out, args), encoding="ascii")
        deploy.check(out, args)
    except (Exception, KeyboardInterrupt):
        shutil.rmtree(out)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    deploy.selection(parser)
    parser.add_argument("--build", required=True, type=Path, help="read-only qualified build directory")
    parser.add_argument("--out", required=True, type=Path, help="new bundle path (parent must exist)")
    args = parser.parse_args()
    try:
        out = args.out.parent.resolve(strict=True) / args.out.name
        stage(args.build.resolve(strict=True), out, args)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"STAGE FAILED: {exc}", file=sys.stderr)
        return 1
    print(f"STAGED: {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())