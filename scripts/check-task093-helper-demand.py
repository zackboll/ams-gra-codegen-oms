#!/usr/bin/env python3
"""C4 production CLI/GNAT regression for semantic Binary helper demand."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "scripts"))
from task093_resources import Measurement
spec = importlib.util.spec_from_file_location("temporal", ROOT / "scripts/check-task094.py")
temporal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(temporal)
NAMES = ["ArraySample", "Array_Encode_Hex_Binary", "Encode_Hex_Binary",
         "Array_encode_hex_binary", "Array_Decode_Hex_Binary"]
QUALIFIED = "https://www.vdl.afrl.af.mil/programs/task093"
RUNTIME = "linux-x86_64-glibc-2.41"


def helper_definitions(body, identifier):
    """Match the complete Ada identifier, not a suffix in a model operation."""
    return re.findall(r"(?im)^\s*function\s+" + re.escape(identifier) + r"\s*\([^;]*?\)\s*return\s+[^;\n]+\s+is\s*$", body)


def checked(command, cwd, stem, expected=0):
    started = time.time()
    clock = time.monotonic()
    with stem.with_suffix(".log").open("xb") as output:
        process = subprocess.Popen(command, cwd=cwd, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        _, status, usage = os.wait4(process.pid, 0)
        process.returncode = os.waitstatus_to_exitcode(status)
    stem.with_suffix(".command.json").write_text(json.dumps(dict(command=command, cwd=str(cwd), exit=process.returncode, expected=expected, started=started, elapsed_seconds=time.monotonic() - clock, cpu_seconds=usage.ru_utime + usage.ru_stime, peak_rss_kib=usage.ru_maxrss), indent=2) + "\n")
    text = stem.with_suffix(".log").read_bytes()
    if expected is not None and process.returncode != expected:
        # Preserve the complete subprocess log and expose its diagnostic in CI.
        print(text.decode("utf-8", errors="replace"), file=sys.stderr, flush=True)
        raise RuntimeError(f"exit {process.returncode}, expected {expected}: {stem}")
    return process.returncode, text


def worker_count(value):
    if value not in ("1", "2", "3", "4"):
        raise ValueError("AMS_GRA_TASK093_WORKERS must be an integer from 1 to 4")
    return int(value)


def verify_sources(scratch):
    entries = (scratch / "sources.sha256").read_text().splitlines()
    if not entries or len(entries) != len(set(entries)):
        raise RuntimeError(f"missing/duplicate provenance: {scratch}")
    for entry in entries:
        digest, path = entry.split("  ", 1)
        if hashlib.sha256(Path(path).read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"source provenance mismatch: {path}")


def validate_case(result, args, profile, name):
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    if type(result) is not dict or set(result) != {"completed", "red", "records"} or result["completed"] is not True or result["red"] is not args.red:
        raise RuntimeError(f"incomplete/malformed result: {profile}/{name}")
    records = result["records"]
    if type(records) is not list or len(records) != len(args.policies):
        raise RuntimeError(f"missing/duplicate policy: {profile}/{name}")
    for record, policy in zip(records, args.policies, strict=True):
        if type(record) is not dict or type(record.get("compiler_exit")) is not int or type(record.get("intentional_red")) is not bool:
            raise RuntimeError(f"malformed record: {profile}/{name}/{policy}")
        expected = dict(compiler=version, family=args.family, profile=profile,
                        name=name, policy=policy, intentional_red=args.red,
                        compiler_exit=record["compiler_exit"])
        if record != expected or (record["compiler_exit"] != 0) != args.red:
            raise RuntimeError(f"incorrect case identity/outcome: {profile}/{name}/{policy}")
    return records


def aggregate(args, selections, completed):
    if len(selections) != len(set(selections)) or len(completed) != len(selections):
        raise RuntimeError("missing/duplicate naming selections")
    results = {}
    for selection, result in completed:
        if selection not in selections or selection in results:
            raise RuntimeError(f"unexpected/duplicate selection: {selection}")
        results[selection] = validate_case(result, args, *selection)
    return [record for selection in selections for record in results[selection]]


def verify_case_evidence(scratch, records):
    """Do not trust completion JSON without its compiler/runtime evidence."""
    verify_sources(scratch)
    for record in records:
        cell = f'{record["profile"]}-{record["name"]}-{record["policy"]}'
        build = json.loads((scratch / f"{cell}-build.command.json").read_text())
        if build["exit"] != record["compiler_exit"] or build["expected"] != (None if record["intentional_red"] else 0) or "-f" not in build["command"]:
            raise RuntimeError(f"missing/inconsistent forced compiler evidence: {cell}")
        if record["policy"] != "normal":
            adc = scratch / f"{cell}.adc"
            if adc.read_text() != f'pragma Assertion_Policy ({record["policy"]});\n' or f"-gnatec={adc}" not in build["command"]:
                raise RuntimeError(f"missing assertion policy evidence: {cell}")
        compiler = "gnat_external" if record["compiler"] == "14.2.0" else "gnat_native"
        temporal.guard.validate_solution(tomllib.loads((scratch / f"{cell}-alire.lock").read_text()), compiler)
        if f'GNAT {record["compiler"]}'.encode() not in (scratch / f"{cell}-gnat.log").read_bytes():
            raise RuntimeError(f"missing compiler identity evidence: {cell}")
        if record["intentional_red"]:
            if b'"Binary_Vectors" not declared in "Oam"' not in (scratch / f"{cell}-build.log").read_bytes():
                raise RuntimeError(f"missing compiler rejection evidence: {cell}")
        elif (scratch / f"{cell}-execute.log").read_bytes() != b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n":
            raise RuntimeError(f"missing runtime evidence: {cell}")


def alire_settings(destination):
    return (
        '[index]\nauto_update = 0\nauto_community = false\n'
        '[dependencies]\nshared = false\n'
        f'[cache]\ndir = {json.dumps(str(destination / "cache"))}\n'
        f'[toolchain]\nassistant = false\ndir = {json.dumps(str(destination / "toolchains"))}\n')


def clone_alire(source, destination):
    """Copy mutable metadata, never share a toolchain directory or cache.

    Only deployed toolchain payload subdirectories are symlinked. Alire's
    must_reload flag lives in the private containing directory. The selected
    releases are pre-deployed serially; no worker installs into the shared tree.
    """
    shutil.copytree(source, destination, symlinks=True, ignore=shutil.ignore_patterns("toolchains"))
    tools = destination / "toolchains"
    tools.mkdir()
    for tool in (source / "toolchains").iterdir():
        if tool.is_dir():
            (tools / tool.name).symlink_to(tool.resolve(), target_is_directory=True)
    (destination / "settings.toml").write_text(alire_settings(destination))


def prepare_alire(scratch, run):
    """Snapshot the existing index; do not add, refresh, or publish an index."""
    _, details = run(["alr", "version"], ROOT, "alire-paths")
    def folder(label):
        match = re.search(r"(?m)^" + re.escape(label) + r":\s+(.+)$", details.decode())
        if not match:
            raise RuntimeError(f"Alire path missing: {label}")
        return Path(match[1].strip())
    # upload-artifact follows symlinks. Keep ephemeral settings/tool payload
    # links hidden (its default exclusion), not in recursive visible evidence.
    seed = scratch / ".alire-seed"
    seed.mkdir()
    shutil.copytree(folder("indexes folder"), seed / "indexes", ignore=shutil.ignore_patterns(".git"))
    tools = seed / "toolchains"
    tools.mkdir()
    for tool in folder("toolchain folder").iterdir():
        if tool.is_dir():
            (tools / tool.name).symlink_to(tool.resolve(), target_is_directory=True)
    (seed / "settings.toml").write_text(alire_settings(seed))
    return seed


def run_campaign(args, scratch, seed, workers):
    selections = [(profile, name) for profile in args.profiles
                  for name in (["Array_Encode_Hex_Binary"] if args.red else NAMES)]
    completed, failures = [], []

    def execute(selection):
        profile, name = selection
        label = f"{profile}-{name}"
        private = scratch / f".{label}-alire"
        clone_alire(seed, private)
        command = [sys.executable, str(Path(__file__).resolve()), "--compiler", args.compiler,
                   "--family", args.family, "--generator", str(args.generator.resolve()),
                   "--scratch", str(scratch / label), "--profiles", profile,
                   "--policies", *args.policies, "--case", name,
                   "--alire-settings", str(private), "--build-jobs", str(4 // workers)]
        if args.red:
            command.append("--red")
        (scratch / f"{label}-worker.command.json").write_text(json.dumps(dict(command=command, cwd=str(ROOT)), indent=2) + "\n")
        with (scratch / f"{label}-worker.log").open("xb") as output:
            process = subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        (scratch / f"{label}-worker.exit").write_text(f"{process.returncode}\n")
        if process.returncode:
            raise RuntimeError(f"worker exit {process.returncode}; see {scratch / (label + '-worker.log')}")
        result = json.loads((scratch / label / "result.json").read_text())
        records = validate_case(result, args, *selection)
        verify_case_evidence(scratch / label, records)
        return selection, result

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(execute, selection): selection for selection in selections}
        for future in as_completed(futures):
            selection = futures[future]
            try:
                completed.append(future.result())
                print("COMPLETE CASE", *selection, flush=True)
            except Exception as error:
                failures.append(f"{selection}: {error}")
                print("FAILED CASE", *selection, error, file=sys.stderr, flush=True)
    # Drain every worker, preserving diagnostics, but never publish success if
    # any exit, identity, policy, result, or provenance check failed.
    if failures:
        raise RuntimeError("qualification workers failed:\n" + "\n".join(failures))
    return aggregate(args, selections, completed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--family", choices=["portable", "float32"], required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--profiles", nargs="+", choices=["oam", "qualified"], default=["oam", "qualified"])
    parser.add_argument("--policies", nargs="+", choices=["normal", "Check", "Ignore"], default=["normal", "Check", "Ignore"])
    parser.add_argument("--red", action="store_true", help="record intentional reviewed-head compile failures for the trigger")
    parser.add_argument("--case", choices=NAMES, help=argparse.SUPPRESS)
    parser.add_argument("--alire-settings", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--build-jobs", type=int, choices=[1, 2, 4], default=1, help=argparse.SUPPRESS)
    args = parser.parse_args()
    try:
        workers = worker_count(os.environ.get("AMS_GRA_TASK093_WORKERS", "2"))
    except ValueError as error:
        parser.error(str(error))
    if len(args.profiles) != len(set(args.profiles)) or len(args.policies) != len(set(args.policies)):
        parser.error("duplicate profiles/policies are not qualification selections")
    if bool(args.case) != bool(args.alire_settings) or (args.case and len(args.profiles) != 1):
        parser.error("internal worker requires one profile, case and private Alire settings")
    measurement = Measurement()
    phases = {}
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    floating = args.family == "float32"
    if floating and (subprocess.check_output(["getconf", "GNU_LIBC_VERSION"]).strip() != b"glibc 2.41" or subprocess.check_output(["uname", "-m"]).strip() != b"x86_64"):
        raise RuntimeError("unqualified Float32 runtime")
    records = []

    def run(command, cwd, label, expected=0):
        if command[0] == "alr" and args.alire_settings:
            command = ["alr", "-s", str(args.alire_settings), *command[1:]]
        return checked(command, cwd, scratch / label, expected)

    run(["sh", "-ec", "uname -m; getconf GNU_LIBC_VERSION; dpkg-query -W libc6; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6; alr --version"], ROOT, "runtime")
    if not args.case:
        started = time.monotonic()
        args.alire_settings = prepare_alire(scratch, run)
        phases["alire_snapshot_seconds"] = time.monotonic() - started
    # Fail before any long naming campaign if this environment cannot supply
    # the requested release. This task-local probe never changes global setup.
    probe = scratch / "toolchain-probe"
    probe.mkdir()
    (probe / "alire.toml").write_text(
        'name = "task093_probe"\nversion = "0.1.0-dev"\ndescription = "Compiler availability"\nlicenses = "MIT"\n[[depends-on]]\n'
        f'{compiler} = "={version}"\ngprbuild = "=26.0.1"\n')
    run(["alr", "-n", "--force", "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"], probe, "toolchain-preflight")
    _, identity = run(["alr", "-n", "exec", "--", "sh", "-ec", "command -v gnat; gnat --version"], probe, "compiler-preflight")
    if f"GNAT {version}".encode() not in identity:
        raise RuntimeError("compiler preflight identity mismatch")
    probe_lock = probe / "alire/alire.lock"
    releases = [state.get("release", {}) for state in tomllib.loads(probe_lock.read_text())["solution"]["state"]]
    for name, required in [(compiler, version), ("gprbuild", "26.0.1")]:
        if not any(release.get("name") == name and release.get("version") == required for release in releases):
            raise RuntimeError(f"compiler preflight solution mismatch: {name}={required}")
    shutil.copy2(probe_lock, scratch / "toolchain-preflight.lock")
    fixtures = ROOT / "tests/fixtures/service-generate"
    suffix = "-float32" if floating else ""
    original = (fixtures / f"codec-ada-helper-demand{suffix}.xsd").read_text()
    golden = (fixtures / f"task093-helper-demand{suffix}.json").read_text()
    consumer_source = (ROOT / "tests/ada-generated-codec/helper_demand_consumer.adb").read_text()
    if floating:
        consumer_source = consumer_source.replace("with Ada.Command_Line;", "with Ada.Command_Line;\nwith Ada.Unchecked_Conversion;\nwith Interfaces; use type Interfaces.Unsigned_32;")
        consumer_source = consumer_source.replace("   File :", "   use type Interfaces.IEEE_Float_32;\n   function Bits is new Ada.Unchecked_Conversion (Interfaces.IEEE_Float_32, Interfaces.Unsigned_32);\n   File :")
        consumer_source = consumer_source.replace("Item.Number = 42", "Bits (Item.Number) = 16#3F80_0001#")
    if not args.case:
        records = run_campaign(args, scratch, args.alire_settings, workers)
    if not args.case:
        args.profiles = []  # Only worker subprocesses execute naming cases.
    for profile in args.profiles:
        for name in [args.case]:
            label = f"{profile}-{name}"
            source = original.replace("ArraySample", name)
            text = golden
            consumer = consumer_source.replace("ArraySample", name)
            if profile == "qualified":
                temporal.QUALIFIED = QUALIFIED
                source = temporal.qualified_schema(source, scratch / f"{label}-namespace.json")
                text = text.replace('"Number":', f'"{{{QUALIFIED}}}Number":').replace('"Text":', f'"{{{QUALIFIED}}}Text":')
                consumer = consumer.replace("Programs.Oam", "Programs.Task093")
            schema = scratch / f"{label}.xsd"
            schema.write_text(source)
            input_path = scratch / f"{label}.json"
            input_path.write_text(text)
            common = [str(args.generator.resolve()), "service-check", "--schema", str(schema), "--contract", str(fixtures / "codec-ada-helper-demand.yaml"), "--language", "ada", "--world", "closed-schema"]
            run(common, ROOT, f"{label}-model-check")
            plain = scratch / f"{label}-plain"
            generate = [*common]
            generate[1] = "service-generate"
            run([*generate, "--output", str(plain)], ROOT, f"{label}-model-generate")
            runtime = ["--ada-codec-runtime", RUNTIME] if floating else []
            _, report = run([*common, "--with-codec", *runtime], ROOT, f"{label}-codec-check")
            if b"codec status: READY" not in report:
                raise RuntimeError("missing readiness result")
            if floating:
                run([*common, "--with-codec"], ROOT, f"{label}-portable-reject", expected=1)
                rejected = scratch / f"{label}-rejected"
                run([*generate, "--with-codec", "--output", str(rejected)], ROOT, f"{label}-portable-generate-reject", expected=1)
                if rejected.exists():
                    raise RuntimeError("non-atomic Float32 rejection")
            directory = scratch / label
            run([*generate, "--with-codec", *runtime, "--output", str(directory)], ROOT, f"{label}-codec-generate")
            for path in plain.iterdir():
                if path.read_bytes() != (directory / path.name).read_bytes():
                    raise RuntimeError("model/API bytes changed")
            body = (directory / "service_codec.adb").read_text()
            model = "".join(p.read_text() for p in plain.glob("*.ad?"))
            if "Binary_Vectors" in model or "UnselectedBinary" in model:
                raise RuntimeError("unselected Binary leaked through projection")
            for operation in ["To_JSON_", "From_JSON_", "Encode_", "Decode_"]:
                if f"function {operation}{name} (" not in body or f"end {operation}{name};" not in body:
                    raise RuntimeError("schema-derived identifier corrupted")
            actual = [len(helper_definitions(body, h)) for h in ["Encode_Hex_Binary", "Decode_Hex_Binary"]]
            if args.red:
                if actual != [1, 1] or "Model.Binary_Vectors.Vector" not in body:
                    raise RuntimeError("intended red helper/model mismatch not reproduced")
            elif actual != [0, 0] or "Model.Binary_Vectors" in body or "with Ada.Containers;" in body:
                raise RuntimeError("unwanted Binary helper demand")
            if ("Require_Platform" in body) != floating or ("with AMS_GRA_OMS_JSON_Occurrences;" in body) != floating:
                raise RuntimeError("runtime representation leakage")
            if not floating:
                opted = scratch / f"{label}-opted"
                run([*generate, "--with-codec", "--ada-codec-runtime", RUNTIME, "--output", str(opted)], ROOT, f"{label}-opted-generate")
                if any(p.read_bytes() != (opted / p.name).read_bytes() for p in directory.iterdir()):
                    raise RuntimeError("runtime option changes portable output")
            (directory / "helper_demand_consumer.adb").write_text(consumer)
            # Each independent naming case owns its support/provider build area.
            support = scratch / f"{label}-support"
            started = time.monotonic()
            shutil.copytree(ROOT / "ada/oms-json-support", support)
            phases["support_copy_seconds"] = time.monotonic() - started
            (directory / "alire.toml").write_text(
                'name = "task093_c4"\nversion = "0.1.0-dev"\ndescription = "Semantic helper demand"\nlicenses = "MIT"\nproject-files = ["consumer.gpr"]\n[[depends-on]]\n'
                f'{compiler} = "={version}"\ngprbuild = "=26.0.1"\nams_gra_oms_json = "=0.1.0-dev"\n[[pins]]\nams_gra_oms_json = {{ path = "../{support.name}" }}\n')
            project = "ams_gra_oms_json_float32" if floating else "ams_gra_oms_json"
            (directory / "consumer.gpr").write_text(
                f'with "../{support.name}/{project}";\nproject Consumer is\nfor Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\nfor Main use ("helper_demand_consumer.adb");\npackage Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
            run(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], directory, f"{label}-isolation")
            run(["alr", "-n", "--force", "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"], directory, f"{label}-toolchain")
            for policy in args.policies:
                cell = f"{label}-{policy}"
                command = ["alr", "-n", "build", "--", "-f", f"-j{args.build_jobs}"]
                if policy != "normal":
                    adc = scratch / f"{cell}.adc"
                    adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                    command += ["-cargs:Ada", f"-gnatec={adc}"]
                exit_code, diagnostic = run(command, directory, f"{cell}-build", expected=None if args.red else 0)
                lock = directory / "alire/alire.lock"
                temporal.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
                shutil.copy2(lock, scratch / f"{cell}-alire.lock")
                _, identity = run(["alr", "-n", "exec", "--", "sh", "-ec", "command -v gnat; gnat --version"], directory, f"{cell}-gnat")
                if f"GNAT {version}".encode() not in identity:
                    raise RuntimeError("compiler identity mismatch")
                if args.red:
                    if exit_code == 0 or b'"Binary_Vectors" not declared in "Oam"' not in diagnostic:
                        raise RuntimeError("expected actual Binary_Vectors compiler diagnostic missing")
                else:
                    _, output = run([str(directory / "bin/helper_demand_consumer"), str(input_path)], directory, f"{cell}-execute")
                    if output != b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n":
                        raise RuntimeError("incomplete naming execution")
                    # Retain an independently runnable consumer for every
                    # policy; subsequent forced builds replace bin/ in place.
                    archive = scratch / f"{cell}-bin"
                    archive.mkdir()
                    shutil.copy2(directory / "bin/helper_demand_consumer", archive)
                    (archive / "executable.sha256").write_text(
                        f"{hashlib.sha256((archive / 'helper_demand_consumer').read_bytes()).hexdigest()}  {archive / 'helper_demand_consumer'}\n")
                records.append(dict(compiler=version, family=args.family, profile=profile, name=name, policy=policy, intentional_red=args.red, compiler_exit=exit_code))
                print("COMPLETE", cell, flush=True)
    (scratch / "resources.json").write_text(json.dumps(dict(workers=1 if args.case else workers, phases=phases, **measurement.finish()), indent=2) + "\n")
    hash_started = time.monotonic()
    paths = [p for p in scratch.rglob("*") if p.is_file() and not p.is_symlink() and p.suffix in [".ads", ".adb", ".c", ".gpr", ".lock", ".xsd", ".json"]]
    paths += [args.generator.resolve(), Path(__file__).resolve(), ROOT / "scripts/task093_resources.py", ROOT / "tests/ada-generated-codec/helper_demand_consumer.adb", fixtures / "codec-ada-helper-demand.yaml", fixtures / f"codec-ada-helper-demand{suffix}.xsd", fixtures / f"task093-helper-demand{suffix}.json"]
    (scratch / "sources.sha256").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    verify_sources(scratch)
    # Kept outside sources.sha256 to avoid a self-referential mutable digest.
    (scratch / "provenance-timing.txt").write_text(f"hash_and_verify_seconds={time.monotonic() - hash_started:.6f}\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps(dict(completed=True, red=args.red, records=records), indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print(f"TASK093_C4_HELPER_QUALIFICATION_COMPLETE family={args.family} compiler={version} naming_executions={len(records)} red={args.red}")


if __name__ == "__main__":
    main()