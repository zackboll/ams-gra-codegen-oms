#!/usr/bin/env python3
"""Read-only, offline preflight of a trusted CPP-OMS-013 bundle."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys


# The MessageDefinitions and SecurityMarkings pins from build.sh (Task 102).
PINS = {
    "2.5": ("ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
            "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b"),
    "2.6": ("af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
            "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88"),
}
PROFILES = {"A": "", "B": "-log-rdma", "C": "-log-rdma-survivability"}
HEADERS = ("oam", "service_api", "service_codec")
BUILD = "manifest/build-manifest.txt"
DEPLOY = "manifest/deployment-manifest.json"
SUMS = "manifest/SHA256SUMS"
LOADER = "/lib64/ld-linux-x86-64.so.2"
TOOLS_ENV = {"PATH": "/usr/bin:/bin", "LC_ALL": "C"}


class Invalid(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Invalid(message)


def regular(path):
    require(stat.S_ISREG(path.lstat().st_mode), f"not a regular file: {path}")
    return path


def sha(path):
    with regular(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def selection(parser):
    parser.add_argument("--release", choices=PINS, required=True)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    parser.add_argument("--variant", choices=("plain", "tls"), required=True)


def artifact(args):
    return "uci" + args.release.replace(".", "") + PROFILES[args.profile] + ("-tls" if args.variant == "tls" else "")


def binary_path(args):
    return "bin/cpp-uci-client-" + artifact(args)


def schema(args):
    return f"002.{args.release[-1]}.0"


def contract_path(args):
    name = f"uci{args.release.replace('.', '')}{PROFILES[args.profile]}-service.yaml"
    base = "tests/fixtures/cpp-codec/addresses" if args.profile == "B" else "examples/cpp-uci-client/contracts"
    return f"{base}/{name}"


def build_manifest(path, args):
    data = regular(path).read_bytes().decode("utf-8")
    result = {}
    for line in data.split("\n")[:-1]:
        key, sep, value = line.partition("=")
        require(sep and value.strip() and value.isprintable() and key not in result,
                "malformed build manifest (invalid or duplicate field)")
        result[key] = value
    hashes = {"schema_root_sha256", "security_markings_sha256", "contract_sha256", "binary_sha256",
              *(f"generated_{name}_sha256" for name in HEADERS)}
    keys = hashes | {"release", "schema_version", "contract", "compiler", "boost", "flags", "binary"}
    tls = args.variant == "tls"
    if tls:
        keys |= {"openssl", "link_libraries"}
    require(data.endswith("\n") and result.keys() == keys, "malformed build manifest (field set or TLS selection)")
    require(all(re.fullmatch(r"[0-9a-f]{64}", result[key]) for key in hashes), "malformed build manifest SHA-256")
    require((result["release"], result["schema_version"]) == (args.release, schema(args)), "build release mismatch")
    require((result["schema_root_sha256"], result["security_markings_sha256"]) == PINS[args.release], "schema pin mismatch")
    require(result["contract"] == contract_path(args), "build contract/profile mismatch")
    built = Path(result["binary"])
    require(built.is_absolute() and built.parts[-2:] == Path(binary_path(args)).parts, "build binary/profile mismatch")
    boost = re.fullmatch(r"Boost (\d{6}) \([0-9_]+\), Boost\.Beast/\d+", result["boost"])
    require(boost and int(boost[1]) >= 108100, "missing/unsupported Boost build metadata")
    require(re.search(r"\b\d+\.\d+\.\d+\b", result["compiler"]), "missing compiler version metadata")
    flags = result["flags"].split()
    require([f for f in flags if f.startswith("-std=")] == ["-std=c++17"] and "-pthread" in flags,
            "build manifest does not describe the C++17/pthread build")
    tls_flags = [f for f in flags if "AMS_GRA_LACAL_ENABLE_TLS" in f]
    require(tls_flags == (["-DAMS_GRA_LACAL_ENABLE_TLS=1"] if tls else []), "build TLS flags mismatch")
    if tls:
        require(result["link_libraries"] == "-lssl -lcrypto" and re.fullmatch(
            r"headers: OpenSSL 3\.[^;]+; library: OpenSSL 3\..+", result["openssl"]),
            "deployment expects OpenSSL 3 build metadata and -lssl -lcrypto")
    return result


def run(argv):
    return subprocess.run(argv, env=TOOLS_ENV, cwd="/", text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, timeout=10, check=False)


def elf(path, variant):
    result = run(["/usr/bin/readelf", "-hldW", path])
    require(result.returncode == 0 and not re.search(r"^readelf:", result.stdout, re.M),
            f"cannot inspect ELF: {result.stdout.strip()}")
    text = result.stdout
    for field, expected in (("Class", "ELF64"), ("Data", "2's complement, little endian"),
                            ("Machine", "Advanced Micro Devices X86-64")):
        require(re.findall(rf"^\s*{field}:\s+(.+)$", text, re.M) == [expected], f"unsupported ELF {field}")
    require(re.findall(r"\[Requesting program interpreter: (.+)\]", text) == [LOADER], "unsupported ELF interpreter")
    require(not re.search(r"\((?:RPATH|RUNPATH|AUDIT|DEPAUDIT|FILTER|AUXILIARY)\)", text),
            "private ELF library search paths or loader hooks are not supported")
    needed = sorted(re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", text))
    require(needed and len(needed) == len(set(needed)) == text.count("(NEEDED)") and
            all(re.fullmatch(r"[A-Za-z0-9_.+-]+\.so(?:\.[0-9]+)*", lib) for lib in needed),
            "invalid ELF shared library requirements")
    openssl = {lib for lib in needed if re.match(r"lib(?:ssl|crypto)\.so(?:\.|$)", lib)}
    require(openssl == ({"libssl.so.3", "libcrypto.so.3"} if variant == "tls" else set()),
            "unexpected or missing direct OpenSSL dependencies")
    return {"architecture": "linux-x86_64", "interpreter": LOADER, "needed": needed}


def deployment(args, source, build_sha, linkage):
    fields = {k: v for k, v in source.items() if k != "binary"}
    return json.dumps(dict(fields, format=1, profile=args.profile, variant=args.variant,
                           binary=binary_path(args), build_manifest_sha256=build_sha,
                           elf=linkage), indent=2, sort_keys=True) + "\n"


def checksums(bundle, args):
    return "".join(f"{sha(bundle / name)}  {name}\n"
                   for name in (binary_path(args), BUILD, DEPLOY, "check.py", "README.txt", "LICENSE"))


def host_checks(path, args):
    require(sys.platform == "linux" and os.uname().machine == "x86_64", "requires a Linux x86-64 host")
    require(os.access(path, os.X_OK) and not path.stat().st_mode & (stat.S_ISUID | stat.S_ISGID),
            "binary is not executable or has a set-ID bit")
    require(os.access(LOADER, os.X_OK), f"missing runtime interpreter {LOADER}")
    # Use the known system loader only, not ldd or an interpreter taken from the manifest.
    libs = run([LOADER, "--list", path])
    require(libs.returncode == 0 and "not found" not in libs.stdout, f"runtime dependencies unavailable:\n{libs.stdout.strip()}")
    help_text = run([path, "--help"])
    require(help_text.returncode == 0 and f"BUILT FOR UCI {args.release} (OWP INIT schema {schema(args)})." in help_text.stdout,
            "--help failed or does not identify the selected build release")
    expected = {"url", "service-id", "duration-seconds", "max-messages", "help"}
    expected |= {"ca-file"} if args.variant == "tls" else set()
    expected |= {"with-rdma"} if args.profile in ("B", "C") else set()
    expected |= {"with-survivability"} if args.profile == "C" else set()
    advertised = re.findall(r"^  --(\S+)", help_text.stdout, re.M)
    url = "{ws,wss}://" if args.variant == "tls" else "ws://"
    require(set(advertised) == expected and len(advertised) == len(expected) and
            help_text.stdout.startswith(f"cpp-uci-client --url {url}"), "--help build profile or option set mismatch")
    for flag in sorted({"release", "message", "insecure", "with-rdma", "with-survivability", "ca-file"} - expected):
        rejected = run([path, "--" + flag])
        reason = {"with-rdma": "--with-rdma requires a Log + RDMA build",
                  "ca-file": "--ca-file requires a TLS-enabled build"}.get(flag, f"unknown argument '--{flag}'")
        require(rejected.returncode == 2 and rejected.stdout.startswith("ERROR " + reason),
                f"unsupported --{flag} was not rejected locally")
    return libs.stdout.strip()


def check(bundle, args):
    for name in ("bin", "manifest"):
        require(stat.S_ISDIR((bundle / name).lstat().st_mode), f"not a bundle directory: {name}")
    source = build_manifest(bundle / BUILD, args)
    path = bundle / binary_path(args)
    require(sha(path) == source["binary_sha256"], "binary SHA-256 mismatch")
    linkage = elf(path, args.variant)
    require(regular(bundle / DEPLOY).read_bytes() == deployment(args, source, sha(bundle / BUILD), linkage).encode(),
            "deployment manifest mismatch")
    require(regular(bundle / SUMS).read_bytes() == checksums(bundle, args).encode(), "SHA256SUMS mismatch")
    return host_checks(path, args)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    selection(parser)
    parser.add_argument("bundle", type=Path, help="root of the staged bundle")
    args = parser.parse_args()
    try:
        libs = check(args.bundle.resolve(strict=True), args)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"CHECK FAILED: {exc}", file=sys.stderr)
        return 1
    print(f"PREFLIGHT OK: UCI {args.release} profile {args.profile} {args.variant} ({binary_path(args)})")
    print(libs)
    print("Offline only: certificate trust and broker availability were not tested.")
    return 0


if __name__ == "__main__":
    sys.exit(main())