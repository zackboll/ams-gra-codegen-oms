#!/usr/bin/env python3
"""Fail closed on selection, per-matrix authoritative outcomes and evidence."""
import json
import os
from pathlib import Path
import urllib.request
from ci_selection import ROOT, from_environment, inventory, ordinary_jobs


def validate(manifest, expected, needs, jobs, artifacts, policy):
    if manifest != expected:
        raise ValueError("selection differs from independently recomputed complete diff")
    required = ordinary_jobs(policy)
    for prerequisite in ("changes", "baseline"):
        if needs.get(prerequisite, {}).get("result") != "success":
            raise ValueError("missing/failed mandatory " + prerequisite)
    for job_id, record in required.items():
        selected = manifest["jobs"][job_id]
        result = needs.get(job_id, {}).get("result")
        if not selected:
            if result != "skipped":
                raise ValueError("unselected job not explicitly skipped: " + job_id)
            continue
        if result != "success":
            raise ValueError("selected job not successful: " + job_id + ": " + str(result))
        if any(not manifest["jobs"].get(p) for p in record["prerequisites"]):
            raise ValueError("missing prerequisite: " + job_id)
        validate_cells(record, jobs, artifacts)
    if manifest["deep"]:
        if needs.get("deep-qualification", {}).get("result") != "success":
            raise ValueError("missing/failed/cancelled Deep CI prerequisite")
        for record in policy["jobs"]:
            if record["workflow"] == "deep-ci.yml":
                validate_cells(record, jobs, artifacts, prefix="deep-qualification / ")
    elif needs.get("deep-qualification", {}).get("result") != "skipped":
        raise ValueError("unselected Deep CI not explicitly skipped")


def validate_cells(record, jobs, artifacts, prefix=""):
    for cell in record["cells"]:
        matches = [j for j in jobs if j["name"] == prefix + cell["name"]]
        if len(matches) != 1:
            raise ValueError("missing/ambiguous authoritative matrix cell: " + cell["name"])
        job = matches[0]
        if job["status"] != "completed" or job["conclusion"] != "success":
            raise ValueError("cell not successful: " + cell["name"])
        # A successful job with skipped qualification steps is NOT qualification.
        steps = job.get("steps", [])
        if len(steps) < len(record["original_steps"]) or any(s["conclusion"] != "success" for s in steps):
            raise ValueError("missing/skipped/failed qualification step: " + cell["name"])
        for name in cell["artifacts"]:
            evidence = [a for a in artifacts if a["name"] == name and not a["expired"]
                        and a["size_in_bytes"] > 0 and a["created_at"] >= job["started_at"]]
            if len(evidence) != 1:
                raise ValueError("missing/expired/stale required evidence: " + name)


def api(path):
    url = "https://api.github.com/repos/" + os.environ["GITHUB_REPOSITORY"] + "/" + path
    request = urllib.request.Request(url, headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"],
        "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"})
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def pages(path, key):
    items = []
    for page in range(1, 1001):
        data = api(path + ("&" if "?" in path else "?") + f"per_page=100&page={page}")
        items.extend(data[key])
        if len(items) == data["total_count"]:
            return items
        if not data[key] or len(items) > data["total_count"]:
            break
    raise ValueError("incomplete or changing API pagination: " + path)


def evidence(run_id, attempt):
    return dict(jobs=pages(f"actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs"),
                artifacts=pages(f"actions/runs/{run_id}/artifacts", "artifacts"))


def main():
    manifest = json.loads(os.environ["SELECTION_MANIFEST"])
    expected = from_environment()
    run = os.environ["GITHUB_RUN_ID"]
    own = evidence(run, os.environ["GITHUB_RUN_ATTEMPT"])
    validate(manifest, expected, json.loads(os.environ["NEEDS_JSON"]), own["jobs"], own["artifacts"], inventory())
    report = dict(identities=manifest["identities"], selected_cells=manifest["selected_cells"],
                  skipped_cells=manifest["skipped_cells"], deep=manifest["deep"], conclusion="success")
    Path("ci-required-result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("PR_CI_REQUIRED_COMPLETE", json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()