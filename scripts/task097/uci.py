"""Fresh focused real-UCI inventory using one normalized parse per root/variant."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--variant", choices=["baseline", "current"], required=True)
    parser.add_argument("--uci25", type=Path, required=True)
    parser.add_argument("--uci26", type=Path, required=True)
    args = parser.parse_args()
    scratch, source = args.scratch.resolve(), args.source.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    identities = {}
    for release, root, expected, security in [
        ("5", args.uci25.resolve(), "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27", "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b"),
        ("6", args.uci26.resolve(), "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b", "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88"),
    ]:
        for path, digest in [(root, expected), (root.with_name(f"UCI_SecurityMarkings_v2_{release}_0.xsd"), security)]:
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != digest:
                raise RuntimeError("pinned schema drift")
            identities[str(path)] = actual
    harness = scratch / "harness"
    (harness / "src").mkdir(parents=True)
    text = (ROOT / "scripts/task097/uci.rs").read_text()
    if args.variant == "baseline":
        text = text.replace('if lossless { profiles.push(("lossless-v1", AdaCodecRuntime::LinuxX86_64Glibc241LosslessV1)); }',
                            'assert!(!lossless);')
    (harness / "src/main.rs").write_text(text)
    dependencies = ["codegen-core", "ir", "xsd-frontend", "service-contract", "backend-ada"]
    names = ["ams-gra-oms-codegen-core", "ams-gra-oms-ir", "ams-gra-oms-xsd-frontend", "ams-gra-oms-service-contract", "ams-gra-oms-backend-ada"]
    (harness / "Cargo.toml").write_text('[package]\nname="task097-uci"\nversion="0.1.0"\nedition="2024"\n[workspace]\n[dependencies]\nserde_json="1"\n' +
        "".join(f'{name} = {{ path = "{source / "crates" / directory}" }}\n' for name, directory in zip(names, dependencies, strict=True)))

    def run(label, command, cwd):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            process = subprocess.run(command, cwd=cwd, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed {label}")

    run("build", ["cargo", "+1.95.0", "build", "--release", "--offline"], harness)
    run("analysis", [str(harness / "target/release/task097-uci"), str(scratch / "analysis"),
        str(args.uci25.resolve()), str(args.uci26.resolve()), args.variant], ROOT)
    paths = [Path(__file__), ROOT / "scripts/task097/uci.rs", harness / "src/main.rs", harness / "Cargo.lock", harness / "target/release/task097-uci"]
    identities.update({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths})
    (scratch / "sources.json").write_text(json.dumps(identities, indent=2) + "\n")
    shutil.copy2(scratch / "analysis/result.json", scratch / "result.json")
    print("TASK097_UCI_VARIANT_COMPLETE", args.variant, flush=True)


if __name__ == "__main__":
    main()