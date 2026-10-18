"""Byte preservation against the exact merged-main generator, including models/APIs."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    stems = ["codec-ada-minimal", "codec-ada-choice", "codec-ada-repeated", "codec-ada-datetime-combined",
             "codec-ada-duration", "codec-ada-repeated-choice", "codec-ada-repeated-temporal",
             "codec-ada-repeated-temporal-composition", "codec-ada-float32", "codec-ada-float32-identifiers",
             "codec-ada-hexbinary-single", "codec-ada-hexbinary-combined", "codec-ada-hexbinary-float32",
             "codec-ada-helper-demand", "codec-ada-helper-demand-float32"]
    rows = []
    for stem in stems:
        schema, contract = fixtures / (stem + ".xsd"), fixtures / (stem + ".yaml")
        if "identifiers" in stem:
            contract = fixtures / "codec-ada-float32-identifiers.yaml"
        if stem == "codec-ada-helper-demand-float32":
            contract = fixtures / "codec-ada-helper-demand.yaml"
        if stem == "codec-ada-repeated-temporal-composition":
            contract = fixtures / "codec-ada-repeated-temporal.yaml"
        if not schema.exists() or not contract.exists():
            raise RuntimeError(f"missing preservation fixture {stem}")
        for language in ["ada", "rust", "cpp"]:
            for codec in [False, True]:
                profiles = [None, "linux-x86_64-glibc-2.41"] if codec and language == "ada" else [None]
                for profile in profiles:
                    label = f"{stem}-{language}-{codec}-{profile}"
                    variants = {}
                    for variant, generator in [("baseline", args.baseline.resolve()), ("current", args.generator.resolve())]:
                        output = scratch / f"{label}-{variant}"
                        command = [str(generator), "service-generate", "--schema", str(schema), "--contract", str(contract),
                            "--language", language, "--world", "closed-schema", "--output", str(output)]
                        if codec:
                            command += ["--with-codec"]
                        if profile:
                            command += ["--ada-codec-runtime", profile]
                        prefix = scratch / f"{label}-{variant}"
                        prefix.with_suffix(".command.json").write_text(json.dumps({"argv": command, "cwd": str(ROOT)}) + "\n")
                        with prefix.with_suffix(".stdout").open("wb") as out, prefix.with_suffix(".stderr").open("wb") as err:
                            result = subprocess.run(command, cwd=ROOT, stdout=out, stderr=err)
                        prefix.with_suffix(".exit").write_text(str(result.returncode) + "\n")
                        files = {str(p.relative_to(output)): hashlib.sha256(p.read_bytes()).hexdigest() for p in output.rglob("*") if p.is_file()}
                        if result.returncode and output.exists():
                            raise RuntimeError("non-atomic rejected generation")
                        variants[variant] = {"exit": result.returncode, "files": files}
                    if variants["baseline"] != variants["current"]:
                        raise RuntimeError(f"preservation mismatch {label}")
                    rows.append({"fixture": stem, "language": language, "codec": codec, "runtime": profile,
                                 "outcome": "byte-identical" if variants["current"]["exit"] == 0 else "atomic-rejection",
                                 "result": variants["current"]})
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "baseline_sha": "5f6800d505c6501638c322d20b23cfc36b09d7d6", "comparisons": rows}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_PRESERVATION_COMPLETE comparisons=" + str(len(rows)), flush=True)


if __name__ == "__main__":
    main()