#!/usr/bin/env python3
"""Task 098 broad preservation sweep against the exact Task 097 parent.

For every committed service-generate fixture with a same-stem contract, and
every language/codec/runtime combination the CLI accepts, run both
generators and compare exit status, the complete output file tree and every
rejection diagnostic. The only permitted differences involve an Ada codec run
whose parent's first codec blocker was a named Time declaration in an
xs:time-bearing fixture: it is either newly READY (model/API bytes equal the
parent's model-only output) or still rejected with the first blocker moved
to a pre-existing non-Time construct. Each such change is listed explicitly.
Rust, C++, model-only Ada and every Time-free output must be byte-identical.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/service-generate"
RUNTIMES = [None, "linux-x86_64-glibc-2.41", "linux-x86_64-glibc-2.41-lossless-v1"]


def tree(path):
    if not path.exists():
        return None
    return {str(p.relative_to(path)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(path.rglob("*")) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    rows, changed, moved = [], [], []
    for schema in sorted(FIXTURES.glob("*.xsd")):
        contract = schema.with_suffix(".yaml")
        if not contract.exists():
            continue
        for language in ["ada", "rust", "cpp"]:
            for codec in [False, True]:
                for runtime in RUNTIMES if (language == "ada" and codec) else [None]:
                    label = f"{schema.stem}-{language}-{codec}-{runtime}"
                    result = {}
                    for variant, generator in [("baseline", args.baseline), ("current", args.generator)]:
                        output = scratch / f"{label}-{variant}"
                        command = [str(generator.resolve()), "service-generate", "--schema", str(schema),
                                   "--contract", str(contract), "--language", language,
                                   "--world", "closed-schema", "--output", str(output)]
                        if codec:
                            command.append("--with-codec")
                        if runtime:
                            command += ["--ada-codec-runtime", runtime]
                        process = subprocess.run(command, cwd=ROOT, capture_output=True)
                        (scratch / f"{label}-{variant}.log").write_bytes(process.stdout + process.stderr)
                        if process.returncode and output.exists():
                            raise RuntimeError(f"non-atomic rejection {label} {variant}")
                        result[variant] = {"exit": process.returncode, "files": tree(output),
                                           "diagnostic": (process.stdout + process.stderr).decode(errors="replace")}
                    same = (result["baseline"]["exit"], result["baseline"]["files"]) == (
                        result["current"]["exit"], result["current"]["files"])
                    row = {"fixture": schema.stem, "language": language, "codec": codec, "runtime": runtime,
                           "baseline_exit": result["baseline"]["exit"], "current_exit": result["current"]["exit"],
                           "identical": same}
                    diagnostics = [result[v]["diagnostic"].replace(f"{label}-{v}", "OUT")
                                   for v in ["baseline", "current"]]
                    if same and result["current"]["exit"] and diagnostics[0] != diagnostics[1]:
                        # Still rejected: only the first-reported codec blocker may
                        # move past a now-admitted named Time to a pre-existing one.
                        before = [l for l in diagnostics[0].splitlines() if "service codec boundary" in l]
                        after = [l for l in diagnostics[1].splitlines() if "service codec boundary" in l]
                        if not (language == "ada" and codec and before and after
                                and before[0].endswith("unsupported Ada codec primitive or facets: Time")
                                and "Time" not in after[0] and 'base="xs:time"' in schema.read_text()):
                            raise RuntimeError(f"unexpected rejection diagnostic change: {label}")
                        row["change"] = "first-blocker-moved-past-named-time"
                        row["blockers"] = [before[0], after[0]]
                        moved.append(label)
                    if not same:
                        admitted = (language == "ada" and codec and result["baseline"]["exit"] == 1
                                    and result["current"]["exit"] == 0
                                    and "unsupported Ada codec primitive or facets: Time" in result["baseline"]["diagnostic"]
                                    and 'base="xs:time"' in schema.read_text())
                        if not admitted:
                            raise RuntimeError(f"unexpected preservation change: {label}")
                        model = {k: v for k, v in result["current"]["files"].items()
                                 if not k.startswith("service_codec") and "codec_views" not in k}
                        plain = scratch / f"{label}-model-only"
                        subprocess.run([str(args.baseline.resolve()), "service-generate", "--schema", str(schema),
                                        "--contract", str(contract), "--language", "ada", "--world",
                                        "closed-schema", "--output", str(plain)], cwd=ROOT, check=True,
                                       capture_output=True)
                        if model != tree(plain):
                            raise RuntimeError(f"newly admitted Time codec changed model/API: {label}")
                        row["change"] = "newly-ready-named-time-codec"
                        changed.append(label)
                    rows.append(row)
    summary = {"comparisons": len(rows), "identical": sum(r["identical"] for r in rows),
               "identical_successes": sum(r["identical"] and r["current_exit"] == 0 for r in rows),
               "identical_rejections": sum(r["identical"] and r["current_exit"] != 0 for r in rows),
               "identical_rejection_diagnostics": sum(r["identical"] and r["current_exit"] != 0
                                                      and "change" not in r for r in rows),
               "newly_ready_named_time": changed,
               "rejection_first_blocker_moved_past_named_time": moved}
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "summary": summary, "rows": rows}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK098_SWEEP_COMPLETE " + json.dumps(summary, sort_keys=True), flush=True)


if __name__ == "__main__":
    main()
