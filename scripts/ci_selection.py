"""CI-SELECT-001: deterministic, stdlib-only source qualification policy.

The synthetic merge commit is the proposed source. Its first/second parents
must exactly match the event's base/head, not a merge-base or a moving branch.
Never use the REST files endpoint (which is capped) to establish completeness.
"""
import fnmatch
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "scripts/ci-qualification-inventory.json"


def inventory():
    return json.loads(INVENTORY.read_text())


def ordinary_jobs(policy):
    return {j["job_id"]: j for j in policy["jobs"] if j["workflow"] == "ci.yml"}


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, timeout=120)
    if result.returncode:
        raise ValueError("git " + args[0] + " failed: " + result.stderr.decode(errors="replace")[:300])
    return result.stdout


def complete_diff(root, base, head, merge):
    for sha in (base, head, merge):
        if not re.fullmatch(r"[0-9a-f]{40}", sha):
            raise ValueError("missing or malformed base/head/merge identity")
        if git(root, "rev-parse", "--verify", sha + "^{commit}").decode().strip() != sha:
            raise ValueError("commit identity mismatch")
    if git(root, "rev-parse", "--is-shallow-repository").strip() != b"false":
        raise ValueError("incomplete shallow history")
    if git(root, "rev-parse", "HEAD").decode().strip() != merge:
        raise ValueError("checkout is not the event merge result")
    parents = git(root, "show", "-s", "--format=%P", merge).decode().strip().split()
    if parents != [base, head]:
        raise ValueError("synthetic merge parents do not match event base/head")
    git(root, "merge-base", "--is-ancestor", base, merge)
    # ls-tree forces both complete trees to be readable before diff collection.
    for sha in (base, head, merge):
        git(root, "ls-tree", "-r", "--full-tree", sha)
    raw = git(root, "diff", "--raw", "-z", "--no-abbrev", "--no-ext-diff", "--no-textconv",
              "--find-renames", base, merge, "--")
    changes = parse_diff(raw)
    # Independently cross-check path completeness, including a truncation at a
    # valid record boundary (which NUL framing alone cannot detect). Disabling
    # renames here exposes BOTH sides of a rename, matching the raw inventory.
    names = git(root, "diff", "--name-only", "-z", "--no-renames", "--no-ext-diff",
                "--no-textconv", base, merge, "--")
    if names and not names.endswith(b"\0"):
        raise ValueError("truncated independent diff path collection")
    names = [p.decode("utf-8", errors="strict") for p in names.split(b"\0")[:-1]]
    if sorted(names) != sorted(p for c in changes for p in c["paths"]):
        raise ValueError("incomplete diff: independent path inventories disagree")
    return changes


def parse_diff(raw):
    if raw and not raw.endswith(b"\0"):
        raise ValueError("truncated NUL-delimited git diff")
    fields = raw.split(b"\0")[:-1]
    changes = []
    while fields:
        header = fields.pop(0).decode("ascii")
        match = re.fullmatch(r":([0-7]{6}) ([0-7]{6}) [0-9a-f]{40} [0-9a-f]{40} ([AMDRCTUXB][0-9]*)", header)
        if not match or not fields:
            raise ValueError("malformed/incomplete git diff")
        old_mode, new_mode, status = match.groups()
        paths = [fields.pop(0).decode("utf-8", errors="strict")]
        if status[0] in "RC":
            if not fields:
                raise ValueError("incomplete renamed/copied diff")
            paths.append(fields.pop(0).decode("utf-8", errors="strict"))
        if any(not p or p.startswith("/") or ".." in p.split("/") for p in paths):
            raise ValueError("invalid diff path")
        changes.append(dict(status=status, paths=paths, old_mode=old_mode, new_mode=new_mode))
    return sorted(changes, key=lambda c: (c["paths"], c["status"]))


def close_prerequisites(selected, jobs):
    selected = set(selected)
    while True:
        expanded = selected | {p for j in selected for p in jobs[j]["prerequisites"]}
        if not expanded <= jobs.keys():
            raise ValueError("unmapped prerequisite")
        if expanded == selected:
            return selected
        selected = expanded


def select(changes, policy, full_reason=None, full_main=False):
    jobs = ordinary_jobs(policy)
    selected, reasons = set(), []
    full = bool(full_reason) or full_main
    deep = full
    if full_reason:
        reasons.append(dict(category="fallback", reason=full_reason))
    for change in changes:
        path = change["paths"][-1]
        deep |= any(fnmatch.fnmatchcase(p, pattern) for p in change["paths"]
                    for pattern in policy["deep_pr_paths"])
        status = change["status"][0]
        if status not in "AM" or change["new_mode"] not in ("100644", "100755"):
            full = deep = True
            reasons.append(dict(path=path, category="fallback", reason="deleted/renamed/copied/type-changed or nonregular path"))
            continue
        # Infrastructure and dependencies win even under a component subtree.
        if (path.endswith("/Cargo.toml") or path.endswith("/Cargo.lock")
                or path.startswith((".github/", ".cargo/", "scripts/"))
                or Path(path).name.startswith(("SECURITY", "rust-toolchain"))):
            full = deep = True
            reasons.append(dict(path=path, category="infrastructure"))
            continue
        docs = policy["documentation"]
        if (any(fnmatch.fnmatchcase(path, p) for p in docs["paths"])
                and path not in docs["exclusions"] and change["new_mode"] == "100644"
                and change["old_mode"] in ("000000", "100644")):
            reasons.append(dict(path=path, category="documentation"))
            continue
        rule = next((r for r in policy["rules"] if any(fnmatch.fnmatchcase(path, p) for p in r["paths"])), None)
        if rule is None:
            full = deep = True
            reasons.append(dict(path=path, category="fallback", reason="unknown/unmapped path"))
        else:
            selected.update(rule["jobs"])
            full |= set(rule["jobs"]) == set(jobs)
            deep |= rule["deep"]
            reasons.append(dict(path=path, category=rule["category"], reason=rule["rationale"]))
    if not changes and not full_main:
        # An empty verified diff has only baseline qualification.
        reasons.append(dict(category="empty-verified-diff"))
    deep |= full
    selected = close_prerequisites(jobs if full else selected, jobs)
    return dict(version=1, mode="full-main" if full_main else "full-pr" if full else "selective-pr",
                baseline=True, deep=bool(deep), reasons=reasons, changes=changes,
                jobs={j: j in selected for j in sorted(jobs)},
                cells=[c["key"] for j in sorted(selected) for c in jobs[j]["cells"]],
                selected_cells=sum(len(jobs[j]["cells"]) for j in selected),
                skipped_cells=sum(len(jobs[j]["cells"]) for j in jobs if j not in selected))


def classify(root, event, event_name, sha, policy=None):
    policy = policy or inventory()
    if event_name != "pull_request":
        manifest = select([], policy, full_main=True)
        manifest["identities"] = dict(merge=sha, base=None, head=None)
        return manifest
    pr = event.get("pull_request", {})
    base, head = pr.get("base", {}).get("sha", ""), pr.get("head", {}).get("sha", "")
    try:
        changes = complete_diff(root, base, head, sha)
        manifest = select(changes, policy)
    except (ValueError, OSError, UnicodeError, subprocess.SubprocessError) as error:
        manifest = select([], policy, full_reason=str(error))
    manifest["identities"] = dict(base=base, head=head, merge=sha)
    return manifest


def from_environment():
    return classify(ROOT, json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text()),
                    os.environ["GITHUB_EVENT_NAME"], os.environ["GITHUB_SHA"])