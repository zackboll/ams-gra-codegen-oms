"""Fail-closed effective compiler-input validation for retained PERF-005 traces."""
import hashlib
import json
from pathlib import Path
import re
import shlex

from perf005_profile import executions


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_profile(path):
    data = json.loads(path.read_text())
    if data.get("exit") != 0:
        raise RuntimeError(f"failed/incomplete build evidence: {path}")
    raw = Path(str(path).replace(".profile.json", ".strace"))
    if raw.exists() and executions(raw) != data.get("executions"):
        raise RuntimeError("profile executions differ from raw trace")
    return data


def input_text(data, name):
    # Temporary inputs MUST have been retained; never consult a recycled /tmp.
    retained = data.get("gpr_temporary_inputs", {})
    if name in retained:
        return retained[name]
    if name.startswith("/tmp/"):
        raise RuntimeError(f"incomplete evidence: unretained compiler input {name}")
    path = Path(name)
    if not path.is_absolute() or not path.is_file():
        raise RuntimeError(f"incomplete evidence: missing compiler input {name}")
    return path.read_text()


def canonical(text, root):
    text = text.replace(str(root) + "-state", "<STATE>").replace(str(root), "<CAMPAIGN>")
    return text


def effective_args(data, argv, root, active=(), source=None):
    """Retain every option and recursively substitute semantic input contents.

    No compiler response/config/specs/ADC/source-map input is dropped. Only
    campaign/state roots and the names of expanded input files are abstracted.
    """
    result = []
    for arg in argv:
        prefix = next((p for p in ("@", "-specs=", "-gnatec=", "-gnatem=") if arg.startswith(p)), None)
        if prefix:
            name = arg[len(prefix):]
            if name in active:
                raise RuntimeError("cyclic compiler input")
            if prefix == "-gnatem=":
                key = str(source) + "\n" + name
                if key not in data.get("pre_execution_mapping_inputs", {}):
                    raise RuntimeError(f"incomplete evidence: missing pre-execution mapping {key}")
                text = data["pre_execution_mapping_inputs"][key]
            else:
                text = input_text(data, name)
            if prefix == "@":
                value = effective_args(data, shlex.split(text), root, (*active, name), source)
            elif prefix == "-specs=":
                # GCC specs may reference response files; preserve literal specs
                # syntax and recursively preserve each referenced file's input.
                value = re.sub(r"@([^\s]+)", lambda m: "@" + json.dumps(
                    effective_args(data, ["@" + m[1]], root, (*active, name), source)), text)
                value = canonical(value, root)
            elif prefix == "-gnatem=":
                # GNAT mapping files are unordered unit/kind, filename, path
                # triples. GPR's concurrent completion order is not semantic.
                lines = canonical(text, root).splitlines()
                if len(lines) % 3:
                    raise RuntimeError("invalid GNAT source mapping input")
                triples = [lines[i:i + 3] for i in range(0, len(lines), 3)]
                if len({row[0] for row in triples}) != len(triples):
                    raise RuntimeError("ambiguous GNAT source mapping input")
                value = sorted(triples)
            else:
                value = canonical(text, root)
            result.append([prefix, value])
        else:
            result.append(canonical(arg, root))
    return result


def successful(record):
    if record.get("exit") != 0 or record.get("inclusive_elapsed_seconds", -1) < 0:
        raise RuntimeError("failed/incomplete compiler execution")


def drivers(data):
    return [r for r in data["executions"] if "-c" in r["argv"] and
            re.search(r"(?:^|-)gcc(?:-\d+)?$", Path(r["executable"]).name)]


def compiler_identity(root, label):
    text = (root / f"{label}-provider-identity.log").read_text()
    path = text.splitlines()[0]
    match = re.search(r"(?m)^([0-9a-f]{64})  " + re.escape(path) + r"$", text)
    if not match:
        raise RuntimeError("missing qualified compiler hash")
    version = json.loads((root / "result.json").read_text())["records"][0]["compiler"]
    qualified = {"14.2.0": "a23ecab8ff08f09ad8c80602c2c5df7f49e09c25905cb8975902e101bf72635f",
                 "16.1.0": "238f4adafcb1425c84da6519068fdfcf33e57e406fc793e2958a9238789900de"}
    if match[1] != qualified.get(version) or f"GNAT {version}\n" not in text:
        raise RuntimeError("compiler hash/version differs from qualified seed")
    executable = Path(path)
    if executable.is_file() and sha(executable) != match[1]:
        raise RuntimeError("compiler binary identity changed")
    return path, match[1]


def source_manifest(root):
    return {name: digest for digest, name in
            (line.split("  ", 1) for line in (root / "sources.sha256").read_text().splitlines())}


def expected_sources(root, label):
    consumer = root / label
    support = root / (label + "-support")
    # All generated bodies plus bodyless namespace specs; service_api and codec
    # specs are dependencies of bodies, not separate compilation-driver inputs.
    paths = list(consumer.glob("*.adb")) + list(consumer.glob("programs*.ads"))
    paths += list((support / "src").glob("*.adb"))
    paths += list((support / "float32").glob("*.adb")) + list((support / "float32").glob("*.c"))
    return {p.resolve() for p in paths}


def validate_cell(root, label, policy, data=None):
    cell = f"{label}-{policy}"
    data = data or load_profile(root / f"{cell}-build.profile.json")
    compiler, digest = compiler_identity(root, label)
    # Executable may live only in the qualified container; the identity log
    # binds its exact absolute path and hash, checked against qualification seed.
    expected = expected_sources(root, label)
    provenance = source_manifest(root)
    seen = set()
    for record in drivers(data):
        successful(record)
        if record["executable"] != compiler or record["argv"][0] != compiler:
            raise RuntimeError("compiler path differs from qualified identity")
        argv = record["argv"]
        inputs = [a for a in argv if a.endswith((".adb", ".ads", ".c", ".S", ".s"))]
        if len(inputs) != 1:
            raise RuntimeError("ambiguous compilation source")
        source = Path(inputs[0])
        if source.name.startswith(("b__", "b~")) and not source.is_absolute():
            continue  # binder-generated compilation, not a project-owned input
        if not source.is_absolute() or source.is_symlink() or source.resolve() not in expected:
            raise RuntimeError(f"unexpected/provider/unowned compilation source: {source}")
        if source.resolve() in seen or provenance.get(str(source)) != sha(source):
            raise RuntimeError("duplicate compilation or source identity mismatch")
        backend_name = "cc1" if source.suffix == ".c" else "gnat1"
        backends = [r for r in data["executions"] if Path(r["executable"]).name == backend_name
                    and str(source) in r["argv"]]
        if len(backends) != 1:
            raise RuntimeError("missing/ambiguous compiler backend execution")
        successful(backends[0])
        backend = Path(backends[0]["executable"])
        if compiler == "/usr/bin/gcc":
            qualified_backend = Path("/usr/libexec/gcc/x86_64-linux-gnu/14") / backend_name
        else:
            qualified_backend = Path(compiler).parent.parent / "libexec/gcc/x86_64-pc-linux-gnu/16.1.0" / backend_name
        if backend.resolve() != qualified_backend.resolve():
            raise RuntimeError("compiler backend differs from qualified toolchain")
        seen.add(source.resolve())
        effective_args(data, argv[1:], root, source=source)  # require every indirect input
        if source.suffix in (".adb", ".ads"):
            adcs = [input_text(data, a.split("=", 1)[1]) for a in argv if a.startswith("-gnatec=")]
            policies = [s for s in adcs if re.search(r"\bAssertion_Policy\b", s, re.I)]
            wanted = [] if policy == "normal" else [f"pragma Assertion_Policy ({policy});\n"]
            if policies != wanted:
                raise RuntimeError("effective assertion policy differs")
            backend_policies = [input_text(data, a.split("=", 1)[1]) for a in backends[0]["argv"]
                                if a.startswith("-gnatec=")]
            if [s for s in backend_policies if re.search(r"\bAssertion_Policy\b", s, re.I)] != wanted:
                raise RuntimeError("backend assertion policy differs")
            if not {"-gnat2022", "-gnato", "-O0"} <= set(argv):
                raise RuntimeError("project-owned Ada options differ")
    if seen != expected:
        raise RuntimeError(f"incomplete project-owned compilation inventory: {expected - seen}")
    return dict(count=len(seen), compiler=compiler, compiler_sha256=digest)


def provider_inputs(profile, root):
    data = load_profile(profile)
    result = []
    compiler, _ = compiler_identity(root, "oam-ArraySample")
    for record in drivers(data):
        argv = record["argv"]
        sources = [a for a in argv if "/alire/cache/dependencies/gnatcoll_" in a and
                   a.endswith((".adb", ".ads", ".c", ".s", ".S"))]
        if not sources:
            continue
        successful(record)
        if record["executable"] != compiler:
            raise RuntimeError("provider compiler differs from qualified compiler")
        provenance = source_manifest(root)
        for source in sources:
            if provenance.get(source) != sha(Path(source)):
                raise RuntimeError("provider compiler source identity differs")
        result.append(effective_args(data, argv[1:], root, source=sources[0]))
    return sorted(result, key=lambda v: json.dumps(v, sort_keys=True))