"""Compare baseline/candidate executables without changing shared inventories.

Usage: python3 <this-file> BASELINE_BINARY CANDIDATE_BINARY [EVIDENCE_JSON]
Both binaries use the SAME absolute fixture paths, so report bytes compare
directly. This is a delivery check, not a dependency of Python model generation.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory


def snapshot(binary: Path, output: Path) -> dict:
    root = Path(__file__).resolve().parents[3]
    track = root / "crates/xsd-frontend/tests/fixtures/track.xsd"
    fixtures = root / "tests/fixtures/service-check"
    evidence = {}
    for world in ("closed-schema", "open-extensions"):
        for language in ("ada", "rust", "cpp"):
            dest = output / world / language
            subprocess.run([
                str(binary), "generate", "--schema", str(track), "--world", world,
                "--language", language, "--output", str(dest),
            ], capture_output=True, check=True)
            evidence[f"generate/{world}/{language}"] = {
                str(f.relative_to(dest)): hashlib.sha256(f.read_bytes()).hexdigest()
                for f in sorted(dest.rglob("*")) if f.is_file()
            }
            for contract in ("ready.yaml", "unready.yaml"):
                result = subprocess.run([
                    str(binary), "service-check", "--schema", str(fixtures / "root.xsd"),
                    "--contract", str(fixtures / contract), "--world", world,
                    "--language", language,
                ], capture_output=True, check=False)
                evidence[f"readiness/{world}/{language}/{contract}"] = {
                    "exit": result.returncode,
                    "stdout": result.stdout.decode(),
                    "stderr": result.stderr.decode(),
                }
                expected = 0 if contract == "ready.yaml" else 1
                if result.returncode != expected:
                    raise RuntimeError(f"unexpected readiness exit: {result}")
        result = subprocess.run([
            str(binary), "coverage", "--schema", str(track), "--world", world,
        ], capture_output=True, check=True)
        evidence[f"coverage/{world}"] = result.stdout.decode()
    return evidence


def main() -> None:
    if len(sys.argv) not in (3, 4):
        raise SystemExit(__doc__)
    with TemporaryDirectory(prefix="py001-parity-") as temp:
        results = {
            label: snapshot(Path(binary).resolve(), Path(temp) / label)
            for label, binary in zip(("before", "after"), sys.argv[1:3])
        }
    if results["before"] != results["after"]:
        raise RuntimeError("baseline source/report parity mismatch")
    if len(sys.argv) == 4:
        Path(sys.argv[3]).write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    digest = hashlib.sha256(json.dumps(results["after"], sort_keys=True).encode()).hexdigest()
    print("PY001 BASELINE PARITY: PASSED (6 generation runs, 12 readiness reports, 2 coverage reports)")
    print(f"Canonical candidate evidence SHA256: {digest}")


if __name__ == "__main__":
    main()