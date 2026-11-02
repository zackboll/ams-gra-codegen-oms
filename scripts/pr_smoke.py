#!/usr/bin/env python3
"""Small, conservative PR diff router and backend smoke executor (stdlib only)."""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BACKENDS = ("python", "rust", "ada", "cpp")

# CPP-OMS-001R1: strict C++17 flags for the generated-consumer smoke. Sanitizers
# and clang++ stay in the full cpp_codec suite, not in PR smoke.
CPP_STRICT = ("-std=c++17", "-Wall", "-Wextra", "-Wpedantic", "-Werror", "-Wconversion",
              "-Wsign-conversion", "-Wshadow", "-Wold-style-cast", "-Wcast-align",
              "-Wnon-virtual-dtor", "-Woverloaded-virtual", "-Wnull-dereference",
              "-Wdouble-promotion", "-Wformat=2")
# Exact registered tests: golden/determinism, codec-is-a-separate-artifact with
# unchanged no-codec output, and explicit rejection of unsupported constructs.
CPP_SMOKE_TESTS = ("generated_codec_matches_the_golden_and_is_deterministic",
                   "codec_is_a_separate_artifact_and_leaves_the_model_and_api_unchanged",
                   "unsupported_constructs_are_rejected_before_any_output")
# CPP-OMS-002R1: offline regression for the authentic-Log value kinds. Each
# existing synthetic fixture is generated through the production CLI and
# tests/cpp-codec/shapes_consumer.cpp is compiled with CPP_STRICT and run; its
# deterministic summary (positive round trips plus negative rejections) must
# match exactly. No UCI schema is fetched.
CPP_SHAPES = (
    ("service-generate/codec-hexbinary", "HEX", 23, 8),          # hexBinary + hexBinary Choice
    ("service-generate/codec-ada-repeated-temporal", "TEMPORAL", 15, 6),  # dateTime/duration
    ("service-generate/codec-ada-choice", "CHOICE", 36, 13),     # Choice, nested Choice
)


def cpp_shape_marker(shape, checks, rejections):
    return f"CPP_OMS_002_SHAPES_COMPLETE shape={shape} checks={checks} rejections={rejections}"


def classify(paths):
    """Unknown/unsafe paths select every lightweight suite, never Deep CI."""
    selected = set()
    workflows = False
    paths = list(paths)
    if not paths:
        return set(BACKENDS), True
    for path in paths:
        if (not isinstance(path, str) or not path or "\\" in path
                or any(ord(c) < 32 or 0xD800 <= ord(c) <= 0xDFFF for c in path)
                or PurePosixPath(path).is_absolute()
                or any(p in ("", ".", "..") for p in path.split("/"))):
            return set(BACKENDS), True
        if path.startswith(".github/workflows/") and path.endswith((".yml", ".yaml")):
            workflows = True
        elif path.endswith((".md", ".rst", ".txt")) and (
                path.startswith("docs/") or path.startswith(("README.", "CONTRIBUTING.", "CHANGELOG."))):
            continue
        elif path in ("LICENSE", "NOTICE"):
            continue
        elif PurePosixPath(path).name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
            selected.update(BACKENDS)
        elif path.startswith(("crates/backend-python/", "python/", "tests/fixtures/python-",
                              "tests/python-", "crates/cli/tests/python_",
                              "scripts/check-py", "scripts/test-py", "scripts/measure-py")):
            selected.add("python")
        elif path.startswith(("crates/backend-rust/", "crates/runtime-rust/",
                              "crates/runtime-api-rust/", "crates/runtime-rust-facade-tests/",
                              "tests/rust-", "tests/fixtures/rust-")):
            selected.add("rust")
        elif (path.startswith(("crates/backend-ada/", "ada/", "qualification/ada/",
                               "tests/ada-", "tests/fixtures/ada-",
                               "tests/fixtures/service-generate/codec-ada-",
                               "crates/cli/tests/ada_", "scripts/task102/", "scripts/task103/",
                               "scripts/task104/", "scripts/task105/", "scripts/task106/", "tests/fixtures/task102/",
                               "tests/fixtures/task103/", "tests/fixtures/task104/",
                               "tests/fixtures/task105/", "tests/fixtures/task106/"))
              or path in ("scripts/task102_evidence.py", "scripts/task103_evidence.py",
                          "scripts/task104_evidence.py", "scripts/task105_evidence.py",
                          "scripts/task106_evidence.py")
              or (path.startswith(("scripts/test_task102", "scripts/test_task103")) and path.endswith(".py"))
              or (path.startswith(("scripts/test_task104", "scripts/test_task105", "scripts/test_task106"))
                  and "/" not in path.removeprefix("scripts/") and path.endswith(".py"))):
            selected.add("ada")
        elif (path.startswith(("crates/backend-cpp/", "tests/cpp-codec/", "tests/fixtures/cpp-codec/"))
              or path in ("crates/cli/tests/cpp_codec.rs", "scripts/check-cpp-oms-001.sh",
                          "scripts/check-cpp-oms-001-parity.sh",
                          "scripts/check-cpp-oms-002-real-log.sh")):
            selected.add("cpp")
        else:
            selected.update(BACKENDS)
    return selected, workflows


def route(base, head):
    try:
        # --no-renames deliberately reports BOTH sides of every rename as D+A.
        # NUL delimiting handles spaces/newlines; no path is evaluated by a shell.
        for revision in (base, head):
            if not re.fullmatch(r"[0-9a-f]{40}", revision):
                raise ValueError("expected full Git object IDs")
        data = subprocess.check_output(
            ["git", "diff", "--no-ext-diff", "--no-renames", "--name-only", "-z", base, head, "--"],
            cwd=ROOT)
        if data and not data.endswith(b"\0"):
            raise ValueError("unterminated diff")
        return classify(data[:-1].decode("utf-8", errors="strict").split("\0") if data else [])
    except (OSError, subprocess.CalledProcessError, UnicodeError, ValueError) as error:
        print(f"Diff unavailable/unclassifiable: {error}; running all smoke suites", flush=True)
        return set(BACKENDS), True


def checked(command, *, cwd=ROOT, marker=None, tests=False, env=None):
    print("+ " + shlex.join(map(str, command)), flush=True)
    result = subprocess.run(command, cwd=cwd, env=env, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    print(result.stdout, end="", flush=True)
    if result.returncode:
        raise subprocess.CalledProcessError(result.returncode, command)
    if marker and marker not in result.stdout:
        raise RuntimeError(f"Missing execution marker: {marker}")
    if tests:
        counts = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored", result.stdout)
        if not counts or sum(int(row[0]) for row in counts) == 0 or any(
                int(failed) or int(ignored) for _, failed, ignored in counts):
            raise RuntimeError("Smoke tests missing, ignored, or not executed")
        if re.search(r"\bskip(?:ped|ping)?\b", result.stdout, re.I):
            raise RuntimeError("Unexpected smoke test skip")
    return result.stdout


def generate(language, scratch, fixture):
    contract = {"python-codec/oam": "python-codec/service",
                "cpp-codec/oam": "cpp-codec/service"}.get(fixture, fixture)
    checked([ROOT / "target/debug/ams-gra-codegen-oms", "service-generate",
             "--schema", ROOT / f"tests/fixtures/{fixture}.xsd",
             "--contract", ROOT / f"tests/fixtures/{contract}.yaml",
             "--language", language, "--world", "closed-schema", "--with-codec",
             "--output", scratch])


def cpp_smoke(scratch):
    """Generate through the production CLI, compile the real consumer, execute it.

    A missing compiler (FileNotFoundError), compile error, failed consumer or
    consumer that did not actually run its checks all raise, failing the job.
    """
    checked(["g++", "--version"], marker="g++")
    # Exact registered tests; the marker proves all three ran (none skipped).
    checked(["cargo", "test", "--locked", "-p", "ams-gra-codegen-oms", "--test", "cpp_codec",
             "--", "--exact", *CPP_SMOKE_TESTS, "--nocapture"],
            marker="test result: ok. 3 passed; 0 failed", tests=True)
    generate("cpp", scratch, "cpp-codec/oam")
    for name in ("oam.hpp", "service_api.hpp", "service_codec.hpp"):
        if not (scratch / name).is_file():
            raise RuntimeError(f"C++ smoke: production CLI did not generate {name}")
    consumer = scratch / "consumer"
    checked(["g++", *CPP_STRICT, "-I", scratch, ROOT / "tests/cpp-codec/consumer.cpp",
             "-o", consumer])
    output = checked([consumer])
    match = re.search(r"^(\d+) checks, 0 failures$", output, re.M)
    if not match or int(match.group(1)) < 100:
        raise RuntimeError("C++ smoke: consumer did not report its positive and negative checks")
    for fixture, shape, checks, rejections in CPP_SHAPES:
        out = scratch.parent / f"cpp-shape-{shape.lower()}"
        generate("cpp", out, fixture)
        for name in ("oam.hpp", "service_api.hpp", "service_codec.hpp"):
            if not (out / name).is_file():
                raise RuntimeError(f"C++ smoke: production CLI did not generate {name} for {fixture}")
        binary = out / f"shapes-{shape}"
        checked(["g++", *CPP_STRICT, f"-DSHAPE_{shape}", "-I", out,
                 ROOT / "tests/cpp-codec/shapes_consumer.cpp", "-o", binary])
        marker = cpp_shape_marker(shape, checks, rejections)
        if marker not in checked([binary]).splitlines():
            raise RuntimeError(f"C++ smoke: shapes consumer {shape} did not report exactly: {marker}")
    print("CPP OMS-002 SHAPES PASSED", flush=True)
    print("CPP SMOKE PASSED", flush=True)


def smoke(backend):
    started = time.monotonic()
    print(f"SMOKE START {backend}", flush=True)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    if backend == "python":
        checked([sys.executable, "-c", "import sys; assert sys.version_info[:2] == (3, 11); print(sys.version)"])
        env["PYTHON"] = sys.executable
        checked(["cargo", "test", "--locked", "-p", "ams-gra-oms-backend-python",
                 "--test", "models", "--test", "codecs", "--", "--nocapture"], tests=True, env=env)
    else:
        # Focused deterministic renderer tests; no full backend/corpus campaign.
        for name in ("track_matches_golden_and_is_deterministic", "schema_set_matches_dependency_order_golden"):
            checked(["cargo", "test", "--locked", "-p", f"ams-gra-oms-backend-{backend}",
                     "--lib", f"tests::{name}", "--", "--exact", "--nocapture"], tests=True)
    checked(["cargo", "build", "--locked", "-p", "ams-gra-codegen-oms", "--bin", "ams-gra-codegen-oms"])
    with tempfile.TemporaryDirectory(prefix=f"pr-smoke-{backend}-") as directory:
        scratch = Path(directory) / "consumer"
        if backend == "python":
            # Public production CLI generates both the model and JSON codec.
            generate("python", scratch, "python-codec/oam")
            checked([sys.executable, ROOT / "crates/backend-python/tests/codec_runtime.py",
                     scratch, "flat", "https://www.vdl.afrl.af.mil/programs/oam",
                     ROOT / "tests/fixtures/python-codec"], marker="Ran 9 tests", env=env)
        elif backend == "cpp":
            cpp_smoke(scratch)
        elif backend == "rust":
            checked([ROOT / "target/debug/ams-gra-codegen-oms", "generate", "--schema",
                     ROOT / "crates/xsd-frontend/tests/fixtures/track.xsd", "--language", "rust",
                     "--world", "closed-schema", "--output", scratch])
            shutil.copy2(ROOT / "tests/pr-smoke/rust_consumer.rs", scratch / "main.rs")
            checked(["rustc", "--edition=2024", "main.rs", "-o", "consumer"], cwd=scratch)
            checked([scratch / "consumer"], cwd=scratch, marker="RUST SMOKE PASSED")
        else:
            generate("ada", scratch, "service-generate/codec-ada-minimal")
            shutil.copy2(ROOT / "tests/pr-smoke/ada_consumer.adb", scratch)
            shutil.copytree(ROOT / "ada/oms-json-support", Path(directory) / "support")
            (scratch / "alire.toml").write_text(
                'name = "pr_smoke_consumer"\nversion = "0.1.0-dev"\n'
                'description = "PR generated codec smoke"\nlicenses = "MIT"\n'
                'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
                'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
                'gnat_native = "=16.1.0"\n[[pins]]\n'
                'ams_gra_oms_json = { path = "../support" }\n')
            (scratch / "consumer.gpr").write_text(
                'with "ams_gra_oms_json";\nproject Consumer is\n'
                'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
                'for Exec_Dir use "bin"; for Main use ("ada_consumer.adb");\n'
                'package Compiler is for Default_Switches ("Ada") use '
                '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
            checked(["alr", "--version"], marker="alr 2.1.1")
            checked(["alr", "-n", "toolchain", "--select", "--local",
                     "gnat_native=16.1.0", "gprbuild=26.0.1"], cwd=scratch)
            checked(["alr", "-n", "exec", "--", "gnat", "--version"], cwd=scratch, marker="GNAT 16.1.0")
            checked(["alr", "-n", "exec", "--", "gprbuild", "--version"], cwd=scratch, marker="26.0.0")
            checked(["alr", "-n", "build"], cwd=scratch)
            checked([scratch / "bin/ada_consumer"], cwd=scratch, marker="ADA SMOKE PASSED")
    print(f"SMOKE PASS {backend} elapsed_seconds={time.monotonic() - started:.3f}", flush=True)


def execute(backends, workflows, runner=smoke, validator=None):
    if workflows:
        (validator or (lambda: checked(["bash", ROOT / "scripts/check-ci-actionlint.sh"])))()
    for backend in BACKENDS:
        if backend in backends:
            runner(backend)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    router = commands.add_parser("route")
    router.add_argument("--base", required=True)
    router.add_argument("--head", required=True)
    router.add_argument("--github-output", type=Path)
    run = commands.add_parser("run")
    run.add_argument("backend", choices=BACKENDS)
    args = parser.parse_args()
    if args.command == "run":
        smoke(args.backend)
    else:
        selected, workflows = route(args.base, args.head)
        result = {backend: backend in selected for backend in BACKENDS}
        result["workflows"] = workflows
        print(json.dumps(result, sort_keys=True))
        if args.github_output:
            with args.github_output.open("a") as output:
                for key, value in result.items():
                    output.write(f"{key}={str(value).lower()}\n")


if __name__ == "__main__":
    main()