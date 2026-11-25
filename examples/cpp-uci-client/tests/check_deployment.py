#!/usr/bin/env python3
"""Deployment integrity controls, using real build.sh artifacts, no broker."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
DEPLOY = Path(__file__).resolve().parents[1] / "deploy"
sys.path.insert(0, str(DEPLOY))
import check


def run(script, *args, code=0, message=None, prefix=()):
    result = subprocess.run([*prefix, sys.executable, str(script), *args], env=check.TOOLS_ENV,
                            cwd="/", text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=20)
    assert result.returncode == code, result.stdout
    assert message is None or message in result.stdout, result.stdout
    if message:
        print("rejected:", message)


def controls(build, out, missing_libs):
    plain = ("--release", "2.5", "--profile", "A", "--variant", "plain")
    tls = (*plain[:-1], "tls")
    stage = DEPLOY / "stage.py"
    bundle = out / "plain"
    run(stage, *plain, "--build", build, "--out", bundle)
    run(stage, *plain, "--build", build, "--out", bundle, code=1, message="File exists")
    repeat = out / "repeat"
    run(stage, *plain, "--build", build, "--out", repeat)
    expected = {"README.txt", "LICENSE", "check.py", check.BUILD, check.DEPLOY, check.SUMS, "bin/cpp-uci-client-uci25"}
    assert {str(p.relative_to(bundle)) for p in bundle.rglob("*") if p.is_file()} == expected
    assert all((bundle / name).read_bytes() == (repeat / name).read_bytes() for name in expected)

    run(bundle / "check.py", *plain, bundle)
    run(bundle / "check.py", *plain[:3], "B", *plain[4:], bundle, code=1, message="contract/profile mismatch")
    run(bundle / "check.py", "--release", "2.7", *plain[2:], bundle, code=2, message="invalid choice")
    for name, message in (("bin/cpp-uci-client-uci25", "binary SHA-256 mismatch"),
                          (check.SUMS, "SHA256SUMS mismatch"), (check.DEPLOY, "deployment manifest mismatch")):
        with (bundle / name).open("ab") as stream:
            stream.write(b"tampered\n")
        run(bundle / "check.py", *plain, bundle, code=1, message=message)
        shutil.copyfile(repeat / name, bundle / name)
    (bundle / "bin/cpp-uci-client-uci25").unlink()
    run(bundle / "check.py", *plain, bundle, code=1, message="No such file")

    # Reproduce manifest drift in a scratch copy, never the qualified build.
    copy = out / "build"
    gen = "generated/uci25"
    manifest = copy / "build-uci25.manifest"
    source = (build / manifest.name).read_text()
    shutil.copytree(build / gen, copy / gen)
    shutil.copytree(build / "bin", copy / "bin", ignore=lambda _dir, files: set(files) - {"cpp-uci-client-uci25"})
    for text, message in ((source + "release=2.5\n", "duplicate field"),
                          (source + "command=$(touch should-not-run)\n", "field set"),
                          (source.replace(check.PINS["2.5"][0], "f" * 64), "schema pin mismatch"),
                          (source.replace(check.sha(DEPLOY.parents[2] / "examples/cpp-uci-client/contracts/uci25-service.yaml"),
                                          "0" * 64), "contract SHA-256 mismatch"),
                          (source.replace("release=2.5", "release=2.6"), "build release mismatch")):
        manifest.write_text(text)
        run(stage, *plain, "--build", copy, "--out", out / "rejected", code=1, message=message)
    manifest.write_text(source)
    binary = copy / "bin/cpp-uci-client-uci25"
    with (copy / gen / "service_codec.hpp").open("ab") as stream:
        stream.write(b"// tampered\n")
    run(stage, *plain, "--build", copy, "--out", out / "rejected", code=1, message="generated service_codec.hpp SHA-256 mismatch")
    shutil.copyfile(build / gen / "service_codec.hpp", copy / gen / "service_codec.hpp")

    for wrong in ("cpp-uci-client-uci25-log-rdma", "cpp-uci-client-uci25-tls"):
        shutil.copyfile(build / "bin" / wrong, binary)
        digest = next(line for line in source.splitlines() if line.startswith("binary_sha256="))
        manifest.write_text(source.replace(digest, "binary_sha256=" + check.sha(binary)))
        reason = "--help build profile or option set mismatch" if wrong.endswith("rdma") else "unexpected or missing direct OpenSSL"
        run(stage, *plain, "--build", copy, "--out", out / "rejected", code=1, message=reason)
        assert not (out / "rejected").exists()

    if missing_libs:
        bundle = out / "tls"
        run(stage, *tls, "--build", build, "--out", bundle)
        for soname in ("libssl.so.3", "libcrypto.so.3"):
            # Hide only this SONAME inside an unprivileged mount namespace.
            library = Path("/usr/lib/x86_64-linux-gnu") / soname
            assert library.is_file()
            prefix = ("/usr/bin/bwrap", "--ro-bind", "/", "/", "--dev", "/dev",
                      "--ro-bind", "/dev/null", str(library))
            run(bundle / "check.py", *tls, bundle, prefix=prefix, code=1,
                message=f"error while loading shared libraries: {soname}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True, help="qualified UCI 2.5 plain A/B and TLS A artifacts")
    parser.add_argument("--scratch", type=Path, required=True, help="existing external scratch directory")
    parser.add_argument("--missing-libs", action="store_true", help="Debian x86-64 bubblewrap control (requires user namespaces)")
    args = parser.parse_args()
    if not __debug__ or args.scratch.resolve().is_relative_to(DEPLOY.parents[2]):
        parser.error("enable assertions and use external scratch storage")
    with tempfile.TemporaryDirectory(dir=args.scratch) as temp:
        controls(args.build.resolve(strict=True), Path(temp), args.missing_libs)
    if not args.missing_libs:
        print("SKIPPED: missing shared library control (opt in with --missing-libs)")
    print("CPP_OMS_013_DEPLOYMENT_CONTROLS PASS")


if __name__ == "__main__":
    main()