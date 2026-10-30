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


def deep_jobs(policy):
    return {j["job_id"]: j for j in policy["jobs"] if j["workflow"] == "deep-ci.yml"}


def deep_condition(job_id):
    return ("github.event_name != 'pull_request' || inputs.selection == '' || "
            "fromJSON(inputs.selection).version != 2 || "
            "contains(fromJSON(inputs.selection).deep_jobs, '" + job_id + "')")


def validate_deep(manifest, policy):
    """Validate the v2 wire contract, independently of path classification."""
    registered = deep_jobs(policy)
    selected = manifest.get('deep_jobs')
    if (manifest.get('version') != 2 or not isinstance(selected, list)
            or any(not isinstance(j, str) or not re.fullmatch(r'[a-z][a-z0-9-]*', j)
                   or j not in registered for j in selected)
            or selected != sorted(set(selected))):
        raise ValueError('invalid canonical Deep job selection')
    if type(manifest.get('deep')) is not bool or manifest['deep'] != bool(selected):
        raise ValueError('inconsistent Deep boolean')
    if close_prerequisites(selected, registered) != set(selected):
        raise ValueError('missing Deep dependency closure')
    for field, enabled in (('deep_selected_cells', True), ('deep_skipped_cells', False)):
        wanted = sum(len(r['cells']) for j, r in registered.items() if (j in selected) == enabled)
        if type(manifest.get(field)) is not int or manifest[field] != wanted:
            raise ValueError('incorrect Deep matrix cell count: ' + field)
    if manifest.get('mode') in ('full-main', 'full-pr') and set(selected) != set(registered):
        raise ValueError('full qualification omitted Deep jobs')


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


def _select(changes, policy, full_reason=None, full_main=False):
    jobs = ordinary_jobs(policy)
    registered = deep_jobs(policy)
    selected, reasons = set(), []
    selected_deep = set()
    full = bool(full_reason) or full_main
    deep = full
    if full_reason:
        reasons.append(dict(category="fallback", reason=full_reason))
    for change in changes:
        path = change["paths"][-1]
        deep |= any(fnmatch.fnmatchcase(p, pattern) for p in change["paths"]
                    for pattern in policy["deep_pr_paths"])
        status = change["status"][0]
        if (status not in "AM" or change["new_mode"] not in ("100644", "100755")
                or (status == 'M' and change['old_mode'] != change['new_mode'])):
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
        # Only reviewed Task099 runtime/fixture roots may use the narrow rule.
        # New similarly named paths must not fall through to generic Ada rules.
        task100_path = ('websocket' in path.lower() or 'task100' in path.lower())
        if task100_path and not any(
                fnmatch.fnmatchcase(path, p) for r in policy['rules']
                if r['category'] in ('ada-websocket', 'ada-websocket-implementation', 'ada-lacal-runtime') for p in r['paths']):
            full = deep = True
            reasons.append(dict(path=path, category='fallback', reason='unmapped Task100 path'))
            continue
        task099_path = ((path.startswith(('ada/', 'tests/ada-')) and 'lacal' in path.lower())
                        or 'task099' in path.lower())
        if task099_path and not any(
                fnmatch.fnmatchcase(path, p) for r in policy["rules"]
                if r['category'] == 'ada-lacal-runtime' for p in r['paths']):
            full = deep = True
            reasons.append(dict(path=path, category="fallback", reason="unmapped Task099 path"))
            continue
        rule = next((r for r in policy["rules"] if any(fnmatch.fnmatchcase(path, p) for p in r["paths"])), None)
        if rule is None:
            full = deep = True
            reasons.append(dict(path=path, category="fallback", reason="unknown/unmapped path"))
        else:
            selected.update(rule["jobs"])
            full |= set(rule["jobs"]) == set(jobs)
            deep |= rule["deep"]
            if type(rule['deep']) is not bool or bool(rule['deep_jobs']) != rule['deep']:
                raise ValueError('incomplete Deep ownership: ' + rule['category'])
            owned = close_prerequisites(rule['deep_jobs'], registered)
            selected_deep.update(owned)
            reasons.append(dict(path=path, category=rule["category"], reason=rule["rationale"],
                                deep_jobs=sorted(owned)))
    if not changes and not full_main:
        # An empty verified diff has only baseline qualification.
        reasons.append(dict(category="empty-verified-diff"))
    if full_main:
        reasons.append(dict(category='full-main', reason='non-PR events require complete qualification'))
    deep |= full
    # Legacy Deep trigger paths not covered by explicit reviewed ownership stay
    # broad. Never infer ownership from a filename or silently lose a trigger.
    if deep and not selected_deep:
        selected_deep = set(registered)
    if full:
        selected_deep = set(registered)
    for reason in reasons:
        if full or 'deep_jobs' not in reason:
            reason['deep_jobs'] = sorted(selected_deep) if full else []
    selected = close_prerequisites(jobs if full else selected, jobs)
    return dict(version=2, mode="full-main" if full_main else "full-pr" if full else "selective-pr",
                baseline=True, deep=bool(selected_deep), deep_jobs=sorted(selected_deep),
                deep_selected_cells=sum(len(registered[j]['cells']) for j in selected_deep),
                deep_skipped_cells=sum(len(r['cells']) for j, r in registered.items() if j not in selected_deep),
                reasons=reasons, changes=changes,
                jobs={j: j in selected for j in sorted(jobs)},
                cells=[c["key"] for j in sorted(selected) for c in jobs[j]["cells"]],
                selected_cells=sum(len(jobs[j]["cells"]) for j in selected),
                skipped_cells=sum(len(jobs[j]["cells"]) for j in jobs if j not in selected))


def select(changes, policy, full_reason=None, full_main=False):
    try:
        result = _select(changes, policy, full_reason, full_main)
        validate_deep(result, policy)
        return result
    except (KeyError, TypeError, ValueError, IndexError) as error:
        # Classification uncertainty cannot authorize a narrow run. Inventory
        # corruption is separately fatal in the mandatory baseline policy gate.
        result = _select([], policy, 'incomplete classification: ' + str(error), full_main)
        validate_deep(result, policy)
        return result


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