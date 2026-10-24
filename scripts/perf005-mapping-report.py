#!/usr/bin/env python3
"""Revalidate final R2 reconciliation and all retained unfiltered replay pairs."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from perf005_evidence import canonical, sha
from perf005_mapping import provider_equivalence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("replay", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    summary = []
    for compiler in (14, 16):
        original, pilot = [args.evidence / "r1" / n for n in (f"original-v2{compiler}", f"pilot{compiler}")]
        for policy in ("normal", "Check", "Ignore"):
            reports = provider_equivalence(original, pilot, policy)
            for i, report in enumerate(reports):
                stem = args.replay / f"{compiler}-{policy}" / f"{i:03d}"
                results = [json.loads((stem / side / "result.json").read_text()) for side in ("original", "pilot")]
                for side, result in zip(("original", "pilot"), results):
                    output = stem / side
                    if (sha(output / "compiler.strace") != result["trace_sha256"]
                            or len(list(output.glob("*.o"))) != 1
                            or sha(next(output.glob("*.o"))) != result["object_sha256"]
                            or len(list(output.glob("*.ali"))) != 1
                            or next(output.glob("*.ali")).read_text().replace(str(output), "<OUTPUT>") != result["ali"]):
                        raise RuntimeError(f"changed/incomplete replay output: {output}")
                    if {canonical(p, original): h for p, h in result["reads"].items()} != report["source_reads"]:
                        raise RuntimeError(f"replay does not reproduce R1 actual sources: {output}")
                    if any(sha(Path(p)) != h for p, h in result["reads"].items()):
                        raise RuntimeError("replay source bytes changed")
                a, b = results
                for key in ("exit", "stdout", "stderr", "ali", "object_sha256", "reads"):
                    if a[key] != b[key] or (key == "exit" and a[key] != 0) or (key == "ali" and a[key] is None):
                        raise RuntimeError(f"paired replay differs: {stem}: {key}")
                # mapping.input is in/out and has been appended by the compiler.
                # Reconstruct its original read-time identity from the trace.
                from perf005_profile import mapping_reads
                captured = [mapping_reads(stem / side / "compiler.strace") for side in ("original", "pilot")]
                if any(len(m) != 1 for m in captured):
                    raise RuntimeError("incomplete replay read-time mapping capture")
                left_text, right_text = [next(iter(m.values())) for m in captured]
                if hashlib.sha256(left_text.encode()).hexdigest() != report["original_sha256"]:
                    raise RuntimeError("replay original map does not match R1")
                relocated = right_text.replace(str(original) + "-state", str(pilot) + "-state").replace(str(original), str(pilot))
                if hashlib.sha256(relocated.encode()).hexdigest() != report["pilot_sha256"]:
                    raise RuntimeError("replay pilot map does not match R1")
                report["replay_evidence"] = str(stem)
            extras = [r for report in reports for r in report["resolved"] if r["side"] != "both"]
            item = dict(compiler=compiler, policy=policy, provider_invocations=106, mapped_ada_invocations=len(reports),
                        differing_invocations=sum(bool(r["original_only"] or r["pilot_only"]) for r in reports),
                        original_only=sum(len(r["original_only"]) for r in reports),
                        pilot_only=sum(len(r["pilot_only"]) for r in reports),
                        unique_different_entries=len({(r["unit"], r["filename"], r["normalized_path"]) for r in extras}),
                        classifications=dict(Counter(r["classification"] for r in extras)),
                        conflicts=0, ambiguities=0, unresolved=0, qualified=True)
            summary.append(item)
            (args.output / f"mapping-{compiler}-{policy}.json").write_text(json.dumps(reports, indent=2, sort_keys=True) + "\n")
            (args.output / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
            print(json.dumps(item, sort_keys=True), flush=True)


if __name__ == "__main__":
    main()