#!/usr/bin/env python3
"""Validate frozen evidence accounting; no production or vendor input changes."""
import csv
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/fixtures/open-world"


def read(name):
    with (FIXTURES / name).open() as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


paths = read("task068-open-projection.tsv")
impact = read("task068-impact.tsv")
targets = read("task068-targets.tsv")
coverage = read("task068-coverage.tsv")
assert paths == sorted(
    paths, key=lambda row: (row["release"], row["message"], row["target"], row["path"])
)
assert len(paths) == 1910
assert len(impact) == 302
assert len(targets) == 104
assert len(coverage) == 6
closure_sets = read("task068-closure-failures.tsv")
assert len(closure_sets) == 6
readiness = (FIXTURES / "task068-open-readiness.tsv").read_text().splitlines()
for release, row_count in [("2.5", 954), ("2.6", 956)]:
    release_paths = [row for row in paths if row["release"] == release]
    release_impact = [row for row in impact if row["release"] == release]
    messages = {row["message"] for row in release_impact}
    assert len(messages) == 151
    assert len(release_paths) == row_count
    assert len({row["target"] for row in release_paths}) == 52
    assert len({row["first_projection_target"] for row in release_impact}) == 34
    assert {row["message"].split("}")[-1] for row in release_paths} == messages
    assert release_impact == sorted(
        release_impact, key=lambda row: (int(row["total_cost"]), row["message"])
    )
    for row in release_impact:
        assert int(row["selected"]) + int(row["closed_support_cost"]) == int(
            row["total_cost"]
        )
        assert any(
            path["message"].endswith("}" + row["message"])
            and path["target"].endswith("}" + row["first_projection_target"])
            for path in release_paths
        )
    for language in ["Ada", "Rust", "Cpp"]:
        closure = next(row for row in closure_sets if row["release"] == release and row["language"] == language)
        assert set(closure["failed_message_set"].split(",")) == messages
        prefix = f"BLOCK\t{release}\t{language}\t"
        assert {
            line.split("\t")[3] for line in readiness if line.startswith(prefix)
        } == messages
    assert sum(row["closed_readiness_all_backends"] == "READY" for row in release_impact) == 94
    assert sum(row["recursive"] == "true" for row in targets if row["release"] == release) == 10
print("Task068 deterministic fixtures and six readiness message sets verified")