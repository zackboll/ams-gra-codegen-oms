#!/usr/bin/env python3
"""Fail-closed Task 061 production qualification; --mode legacy|batch|equivalence.

Extensive standalone comparisons are opt-in, not an extra baseline on every PR.
Each invocation retains inputs, streams, ordered results and accounting in a new
scratch directory printed on stderr (also on failure).
"""
import argparse
from collections import Counter
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import subprocess
import sys
import tempfile
import time

repo = Path(__file__).resolve().parent.parent
PINS = {
    "2.5": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
    "2.6": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def workers(value):
    require(value.isascii() and value.isdecimal(), "AMS_GRA_TASK061_WORKERS must be an integer in 1..32")
    count = int(value)
    require(1 <= count <= 32, "AMS_GRA_TASK061_WORKERS must be in 1..32")
    return count


def load_rows():
    rows = [tuple(line.split("\t")) for line in
            (repo / "tests/fixtures/temporal/task061-newly-ready.tsv").read_text().splitlines()
            if not line.startswith("#")]
    require(len(rows) == len(set(rows)) == 609, "expected 609 unique world tuples")
    require(len({(v, b, m) for v, _, b, m in rows}) == 444,
            "expected 444 unique release/backend/message tuples")
    for version, world, backend, message in rows:
        require(version in PINS and world in ("ClosedSchemaSet", "OpenExtensions")
                and backend in ("Ada", "Rust", "Cpp")
                and message.isascii() and message.replace("_", "").isalnum(), "invalid fixture row")
    return rows


def request(row, scratch):
    version, world, backend, message = row
    stem = f"{version}-{world}-{backend}-{message}"
    contract = scratch / (stem + ".yaml")
    contract.write_text(f'''contract_version: "0.1"
service:
  name: task061
  version: "0.1.0"
  kind: service
standards:
  oms_version: "{version}"
  uci_schema_version: "{version}"
functions:
  - id: f
    name: F
    category: specific
    applicability: applicable
    exchanges:
      - id: e
        kind: oms_message
        direction: output
        mandate: mandatory
        message: {message}
        topic: t
        timing:
          kind: asynchronous
''')
    return dict(id=stem, contract=str(contract), language=backend.lower(),
                world="closed-schema" if world == "ClosedSchemaSet" else "open-extensions",
                with_codec=False, ada_codec_runtime=None)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def validate_batch(report, requests, root, version):
    require(set(report) == {"format", "schema", "schema_version", "schema_loads", "completed", "results"},
            "unexpected batch report fields")
    require(report["format"] == "service-check-batch-results-v1"
            and report["schema"] == root and report["schema_version"] == f"002.{version[-1]}.0"
            and type(report["schema_loads"]) is int and report["schema_loads"] == 1,
            "batch schema identity mismatch")
    results = report["results"]
    require(type(report["completed"]) is int
            and report["completed"] == len(requests) == len(results), "incomplete batch")
    expected = {r["id"]: r for r in requests}
    seen = set()
    for item in results:
        require(set(item) == {"request", "exit_code", "stdout", "stderr"}, "invalid result fields")
        identity = item["request"]["id"]
        require(identity in expected and identity not in seen, "duplicate or extra result ID")
        require(item["request"] == expected[identity], "mismatched result identity")
        require(type(item["exit_code"]) is int and item["exit_code"] in (0, 1, 2)
                and isinstance(item["stdout"], str) and isinstance(item["stderr"], str), "invalid result")
        seen.add(identity)
    require(seen == set(expected), "missing results")
    require([r["request"] for r in results] == requests, "batch reordered results")
    return results


def validate_exit(returncode, results):
    failed = any(item["exit_code"] != 0 for item in results)
    require(returncode == (1 if failed else 0), "batch exit status disagrees with results")


def verify_roots(roots):
    for version, root in roots.items():
        require(hashlib.sha256(Path(root).read_bytes()).hexdigest() == PINS[version],
                f"wrong pinned schema identity: {version}")


def execute(binary, rows, requests, roots, scratch, mode, count):
    start = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    destination = scratch / mode
    destination.mkdir()

    def legacy(pair):
        row, req = pair
        command = [str(binary), "service-check", "--schema", roots[row[0]], "--contract",
                   req["contract"], "--language", req["language"], "--world", req["world"]]
        result = subprocess.run(command, capture_output=True, check=False)
        (destination / (req["id"] + ".log")).write_bytes(result.stdout + result.stderr)
        return [dict(request=req, exit_code=result.returncode,
                     stdout=result.stdout.decode(), stderr=result.stderr.decode())]

    def batch(partition):
        index, version, selected = partition
        worker = destination / f"worker-{index}"
        worker.mkdir()
        manifest = worker / "manifest.json"
        manifest.write_text(json.dumps(dict(format="service-check-batch-v1",
                                           schema_version=f"002.{version[-1]}.0", requests=selected)))
        result = subprocess.run([str(binary), "service-check-batch", "--schema", roots[version],
                                 "--manifest", str(manifest)], capture_output=True, check=False)
        (worker / "stdout.json").write_bytes(result.stdout)
        (worker / "stderr.log").write_bytes(result.stderr)
        try:
            report = json.loads(result.stdout, object_pairs_hook=unique_object)
            results = validate_batch(report, selected, roots[version], version)
        except (ValueError, RuntimeError, KeyError, TypeError) as error:
            # An interrupted process cannot publish a completion. Attribute its
            # raw diagnostics to EVERY selection; none becomes a passing result.
            for req in selected:
                (destination / (req["id"] + ".log")).write_bytes(
                    f"BATCH EXECUTION FAILED: {error}; exit={result.returncode}\n".encode()
                    + result.stdout + result.stderr)
            raise RuntimeError(f"invalid/incomplete batch; inspect {worker}") from error
        # Persist every selection BEFORE checking the aggregate subprocess status.
        for item in results:
            (destination / (item["request"]["id"] + ".log")).write_text(item["stdout"] + item["stderr"])
        validate_exit(result.returncode, results)
        return results

    if mode == "legacy":
        tasks = list(zip(rows, requests))
        operation = legacy
    else:
        tasks = []
        # Up to `count` concurrently resident schemas total, never count per release.
        for release_index, version in enumerate(PINS):
            selected = [req for row, req in zip(rows, requests) if row[0] == version]
            partitions = max(1, count // 2 + (count % 2 if release_index == 0 else 0))
            for offset in range(min(partitions, len(selected))):
                tasks.append((len(tasks), version, selected[offset::partitions]))
        operation = batch
    with concurrent.futures.ThreadPoolExecutor(max_workers=count) as pool:
        groups = list(pool.map(operation, tasks))
    by_id = {}
    for group in groups:
        for item in group:
            identity = item["request"]["id"]
            require(identity not in by_id, "duplicate global result")
            by_id[identity] = item
            (destination / (identity + ".log")).write_text(item["stdout"] + item["stderr"])
    require(set(by_id) == {req["id"] for req in requests}, "missing or extra global results")
    results = [by_id[req["id"]] for req in requests]
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    wall = time.monotonic() - start
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    summary = dict(mode=mode, workers=count, wall_seconds=wall, cpu_seconds=cpu,
                   user_seconds=after.ru_utime - before.ru_utime,
                   system_seconds=after.ru_stime - before.ru_stime,
                   cpu_percent=100 * cpu / wall, max_child_rss_kib=after.ru_maxrss,
                   production_invocations=len(tasks), schema_loads=len(tasks), completed=len(results),
                   successes=sum(r["exit_code"] == 0 for r in results),
                   failures=sum(r["exit_code"] != 0 for r in results),
                   per_release=dict(Counter(r[0] for r in rows)),
                   per_backend=dict(Counter(r[2] for r in rows)),
                   per_world=dict(Counter(r[1] for r in rows)),
                   per_cell=dict(Counter("/".join(r[:3]) for r in rows)))
    (destination / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    (destination / "metrics.json").write_text(json.dumps(summary, indent=2) + "\n")
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--mode", choices=("batch", "legacy", "equivalence"), default="batch")
    args = parser.parse_args()
    count = workers(os.environ.get("AMS_GRA_TASK061_WORKERS", "4"))
    rows = load_rows()
    roots = {v: str(Path(os.environ[f"AMS_GRA_UCI_2_{v[-1]}_ROOT"]).resolve()) for v in PINS}
    verify_roots(roots)
    scratch = Path(tempfile.mkdtemp(prefix="task061-service-check-", dir=os.environ["TMPDIR"]))
    print(f"Task 061 evidence: {scratch}", file=sys.stderr, flush=True)
    requests = [request(row, scratch) for row in rows]
    # Freeze the executable for this run: a concurrent cargo build must not
    # silently change the revision halfway through standalone equivalence.
    source_binary = args.binary.resolve(strict=True)
    binary = scratch / "production-cli"
    shutil.copyfile(source_binary, binary)
    binary.chmod(0o500)
    provenance = dict(
        source_commit=subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip(),
        source_diff_sha256=hashlib.sha256(subprocess.check_output(["git", "-C", str(repo), "diff", "HEAD"])).hexdigest(),
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), source_binary=str(source_binary),
        fixture_sha256=hashlib.sha256((repo / "tests/fixtures/temporal/task061-newly-ready.tsv").read_bytes()).hexdigest(),
        schema_roots=roots, schema_sha256=PINS, workers=count, mode=args.mode,
        cpu_affinity=sorted(os.sched_getaffinity(0)),
    )
    (scratch / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    mode = "legacy" if args.mode == "legacy" else "batch"
    results = execute(binary, rows, requests, roots, scratch, mode, count)
    if args.mode == "equivalence":
        standalone = execute(binary, rows, requests, roots, scratch, "legacy", count)
        require(results == standalone, "batch and standalone production results differ; inspect evidence")
        (scratch / "equivalence.json").write_text(json.dumps(dict(
            completed=609, exact_equal=True,
            fields=["release", "world", "backend", "message", "exit_code", "stdout", "stderr"],
            binary_sha256=provenance["binary_sha256"],
            ordered_results_sha256=hashlib.sha256(json.dumps(results, sort_keys=True).encode()).hexdigest(),
        ), indent=2) + "\n")
        print("TASK061 EXACT STANDALONE/BATCH EQUIVALENCE: PASSED (609 selections)")
    for row, req, result in zip(rows, requests, results):
        print("SERVICE-CHECK", *row, result["exit_code"], sep="\t")
        if result["exit_code"]:
            print(result["stdout"] + result["stderr"], file=sys.stderr)
        require(result["request"] == req, "result selection mismatch")
    require(all(r["exit_code"] == 0 and "\nstatus: READY\n" in r["stdout"] and not r["stderr"]
                for r in results), "failed selection or absent READY report")
    print("TASK061 UNIQUE NEWLY READY SERVICE-CHECK: PASSED (609 world tuples; 444 release/backend/message tuples)")


if __name__ == "__main__":
    main()