"""Campaign-local GNATCOLL reuse experiment, never enabled by ordinary runners."""
import hashlib
import json
from pathlib import Path
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest(root):
    if any(path.is_symlink() for path in root.rglob("*")):
        raise RuntimeError("provider artifact symlink is not an immutable artifact")
    return {str(path.relative_to(root)): digest(path)
            for path in sorted(root.rglob("*")) if path.is_file()}


def sources(directory):
    dependencies = directory / "alire/cache/dependencies"
    result = {}
    for name in ("gnatcoll", "gnatcoll_minimal"):
        matches = list(dependencies.glob(name + "_26.0.0_*"))
        if len(matches) != 1:
            raise RuntimeError(f"missing/ambiguous resolved provider: {name}")
        result[name] = matches[0]
    return result


def source_identity(roots):
    suffixes = {".ads", ".adb", ".c", ".h", ".s", ".gpr", ".toml"}
    return {name: {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob("*"))
                   if p.is_file() and p.suffix.lower() in suffixes
                   and not any(part in ("obj", "lib", ".git", "alire") for part in p.relative_to(root).parts)}
            for name, root in roots.items()}


def verify_variant(root, expected):
    record = json.loads((root / "complete.json").read_text())
    if record.get("identity") != expected or record.get("completed") is not True:
        raise RuntimeError("provider identity/incomplete preparation mismatch")
    if "assertion_policy" in expected and (root / "policy.adc").read_text() != expected["assertion_policy"]:
        raise RuntimeError("provider assertion-policy contents changed")
    actual = manifest(root / "artifacts")
    if not actual or actual != record["artifacts"]:
        raise RuntimeError("missing/corrupt provider artifact")
    for library in ("libgnatcoll_core.a", "libgnatcoll_minimal.a"):
        if not any(Path(p).name == library for p in actual):
            raise RuntimeError(f"incomplete provider preparation: {library}")
    return record


class Pilot:
    def __init__(self, scratch, compiler, checked):
        self.root = scratch / "perf005-providers"
        self.scratch = scratch
        self.compiler = compiler
        self.checked = checked
        self.identity = None
        self.provenance = []
        self.validated_directories = set()

    def configuration(self, directory, roots):
        _, output = self.checked(["alr", "-n", "exec", "--", "sh", "-ec",
                                  'command -v gcc; gcc --version; gcc -dumpmachine; '
                                  'gcc -print-file-name=adainclude; gcc -print-file-name=adalib; '
                                  'sha256sum "$(gcc -print-file-name=adainclude)/system.ads" "$(gcc -print-file-name=adalib)/libgnat.a"; '
                                  'command -v gnat; gnat --version; '
                                  'command -v gprbuild; gprbuild --version; '
                                  'sha256sum "$(command -v gcc)" "$(command -v gnat)" "$(command -v gprbuild)"; '
                                  'getconf GNU_LIBC_VERSION; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6; '
                                  'env | sort | grep -E "^(GNATCOLL|LIBRARY_TYPE|BUILD|GPR_|ADA_|CFLAGS|LDFLAGS)"'],
                                 directory, self.scratch / f"{directory.name}-provider-identity")
        # GPR_PROJECT_PATH is deployment-specific; all other recorded lines must
        # match exactly. Sources include upstream projects/constants/options.
        lines = [line for line in output.decode().splitlines()
                 if not line.startswith(("GPR_PROJECT_PATH=", "GNATCOLL_ALIRE_PREFIX=", "GNATCOLL_MINIMAL_ALIRE_PREFIX="))]
        identity = dict(compiler=self.compiler, toolchain_runtime_environment=lines,
                        sources=source_identity(roots), alire="2.1.1", gprbuild="26.0.1")
        expected = "14.2.0" if self.compiler == "14" else "16.1.0"
        if f"GNAT {expected}" not in lines:
            raise RuntimeError("provider compiler identity mismatch")
        return identity

    def prepare(self, directory, roots, identity):
        self.root.mkdir(exist_ok=False)
        for policy in ("normal", "Check", "Ignore"):
            variant = self.root / policy
            variant.mkdir()
            artifacts = variant / "artifacts"
            artifacts.mkdir()
            adc = variant / "policy.adc"
            contents = "" if policy == "normal" else f"pragma Assertion_Policy ({policy});\n"
            adc.write_text(contents)
            command = ["alr", "-n", "exec", "--", "gprbuild", "-p", "-f", "-j0",
                       "-P", str(roots["gnatcoll"] / "core/gnatcoll_core.gpr"),
                       f"-XGNATCOLL_OBJECT_ROOT={artifacts}/", "-XEXTERNALLY_BUILT=false"]
            if contents:
                command += ["-cargs:Ada", f"-gnatec={adc}"]
            self.checked(command, directory, self.scratch / f"provider-{policy}-build")
            key = dict(identity, policy=policy, assertion_policy=contents)
            record = dict(completed=True, identity=key, command=command, artifacts=manifest(artifacts))
            (variant / "complete.json").write_text(json.dumps(record, indent=2) + "\n")
            verify_variant(variant, key)
            for path in artifacts.rglob("*"):
                path.chmod(0o555 if path.is_dir() else 0o444)
            artifacts.chmod(0o555)
        self.identity = identity

    def build(self, command, directory, stem, expected):
        self.checked(["alr", "-n", "build", "--stop-after=generation"], directory,
                     self.scratch / f"{stem.name}-resolve")
        lock = tomllib.loads((directory / "alire/alire.lock").read_text())
        releases = [s.get("release", {}) for s in lock["solution"]["state"]]
        required_hash = "sha512:cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b"
        for name in ("gnatcoll", "gnatcoll_minimal"):
            matches = [r for r in releases if r.get("name") == name]
            if len(matches) != 1 or matches[0].get("version") != "26.0.0" or required_hash not in matches[0].get("origin", {}).get("hashes", []):
                raise RuntimeError("provider archive identity mismatch")
        roots = sources(directory)
        if directory not in self.validated_directories:
            identity = self.configuration(directory, roots)
            if self.identity is None:
                self.prepare(directory, roots, identity)
            elif identity != self.identity:
                raise RuntimeError("consumer source/compiler/configuration differs from provider")
            self.validated_directories.add(directory)
        elif source_identity(roots) != self.identity["sources"]:
            raise RuntimeError("provider sources changed within consumer")
        policy = "normal"
        for arg in command:
            if arg.startswith("-gnatec="):
                contents = Path(arg.split("=", 1)[1]).read_text()
                choices = {f"pragma Assertion_Policy ({p});\n": p for p in ("Check", "Ignore")}
                if contents not in choices:
                    raise RuntimeError("unrecognized assertion policy")
                policy = choices[contents]
        key = dict(self.identity, policy=policy,
                   assertion_policy="" if policy == "normal" else f"pragma Assertion_Policy ({policy});\n")
        variant = self.root / policy
        verify_variant(variant, key)
        modified = command[:4] + [f"-XGNATCOLL_OBJECT_ROOT={variant}/artifacts/", "-XEXTERNALLY_BUILT=true"] + command[4:]
        result = self.checked(modified, directory, stem, expected)
        verify_variant(variant, key)
        self.provenance.append(dict(cell=stem.name, provider=str(variant),
                                    manifest_sha256=digest(variant / "complete.json"), policy=policy))
        (self.scratch / "perf005-provider-consumers.json").write_text(json.dumps(self.provenance, indent=2) + "\n")
        return result