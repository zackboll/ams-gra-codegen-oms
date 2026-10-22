#!/usr/bin/env python3
"""Bounded production service-check qualification; retain ordered, lossless evidence."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parent.parent
MARKER = "TASK062 ALL 54 NEWLY READY CLI TUPLES: PASSED"
# Qualified on four-logical-CPU affinity; one worker remains the reference mode.
DEFAULT_WORKERS = 4
MAX_WORKERS = 4
TIMEOUT_SECONDS = 600
WORLDS = {"ClosedSchemaSet": "closed-schema", "OpenExtensions": "open-extensions"}


def worker_count(value):
    if not re.fullmatch(r"[1-9][0-9]*", value) or not 1 <= int(value) <= MAX_WORKERS:
        raise ValueError(f"AMS_GRA_TASK062_WORKERS must be an integer from 1 to {MAX_WORKERS}")
    return int(value)


def selections():
    rows = (REPO / "tests/fixtures/string/task062-new-ready.tsv").read_text().splitlines()
    validate_selections(rows)
    return rows


def validate_selections(rows):
    if len(rows) != 54 or len(set(rows)) != 54:
        raise ValueError("expected exactly 54 distinct Task 062 selections")
    parsed = [row.split('\t') for row in rows]
    for fields in parsed:
        if (len(fields) != 4 or fields[0] not in ("2.5", "2.6")
                or fields[1] not in WORLDS or fields[2] not in ("Ada", "Cpp", "Rust")
                or not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", fields[3])):
            raise ValueError(f"invalid Task 062 selection: {fields!r}")
    if len({(r[0], r[2], r[3]) for r in parsed}) != 30:
        raise ValueError("expected exactly 30 release/backend/message identities")


def contract_text(row):
    release, _, _, message = row.split('\t')
    return f'''contract_version: "0.1"
service:
  name: task062
  version: "0.1.0"
  kind: service
standards:
  oms_version: "{release}"
  uci_schema_version: "{release}"
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
'''


def prepare(binary, rows, scratch):
    validate_selections(rows)
    items = []
    for index, row in enumerate(rows):
        release, world, backend, _ = row.split('\t')
        directory = scratch / f"{index:02d}"
        directory.mkdir()
        contract = directory / "contract.yaml"
        contract.write_text(contract_text(row))
        root = os.environ[f"AMS_GRA_UCI_{release.replace('.', '_')}_ROOT"]
        if not Path(root).is_file():
            raise ValueError(f"missing pinned schema: {root}")
        items.append({"index": index, "identity": row,
                      "command": [str(binary), "service-check", "--schema", root,
                                  "--contract", str(contract), "--language", backend.lower(),
                                  "--world", WORLDS[world]],
                      "contract": contract_text(row),
                      "log_path": str(directory / "diagnostic.json")})
    return items


def save_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


def execute(item):
    """One subprocess owns one contract and three diagnostic files, even on failure."""
    result = dict(item, exit_status=None, duration_seconds=0, error=None)
    log = Path(item["log_path"])
    save_json(log, result)
    start = time.monotonic()
    stdout = stderr = b""
    try:
        completed = subprocess.run(item["command"], capture_output=True, timeout=TIMEOUT_SECONDS)
        stdout, stderr = completed.stdout, completed.stderr
        result["exit_status"] = completed.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        result["error"] = f"subprocess timed out after {TIMEOUT_SECONDS}s"
    except Exception as error:
        result["error"] = f"{type(error).__name__}: {error}"
    finally:
        result["duration_seconds"] = time.monotonic() - start
        log.with_suffix(".stdout").write_bytes(stdout)
        log.with_suffix(".stderr").write_bytes(stderr)
        save_json(log, result)
    return result


def ready_report(data, identity):
    """Validate the complete deterministic production READY report, not a substring."""
    _, world, backend, _ = identity.split('\t')
    pattern = (r"service contract valid\nservice: task062\n"
               + f"language: {backend.lower()}\ngeneration world: {WORLDS[world]}\n\n"
               + r"selected oms messages: 1\nrenderable selected oms messages: 1\n"
               r"selected type closure: ([1-9][0-9]*)\nrenderable selected types: ([1-9][0-9]*)\n"
               r"(?:generated support types: ([1-9][0-9]*)\n"
               r"renderable generated support types: ([1-9][0-9]*)\n)?status: READY\n")
    match = re.fullmatch(pattern.encode(), data)
    if not match or match[1] != match[2] or match[3] != match[4]:
        raise ValueError(f"incomplete, malformed or non-READY report: {identity}")


def validate_results(items, results):
    expected = [item["identity"] for item in items]
    validate_selections(expected)
    if (len({item["log_path"] for item in items}) != 54
            or len({item["command"][5] for item in items}) != 54):
        raise ValueError("work items must own distinct contracts and logs")
    actual = [result["identity"] for result in results]
    if actual != expected or len(set(actual)) != len(expected):
        raise ValueError("missing, duplicate, extra, reordered or wrong selection results")
    failures = []
    for item, result in zip(items, results):
        try:
            _, world, backend, _ = item["identity"].split('\t')
            command = item["command"]
            if (item["contract"] != contract_text(item["identity"])
                    or command[1:3] != ["service-check", "--schema"]
                    or command[4] != "--contract"
                    or command[6:] != ["--language", backend.lower(), "--world", WORLDS[world]]):
                raise ValueError("work item does not match its frozen identity")
            for key in ("index", "identity", "command", "contract", "log_path"):
                if result[key] != item[key]:
                    raise ValueError(f"wrong {key}")
            if (result["error"] is not None or result["exit_status"] != 0
                    or not isinstance(result["duration_seconds"], (int, float))
                    or not 0 <= result["duration_seconds"] < float('inf')):
                raise ValueError(f"failed execution: {result}")
            log = Path(result["log_path"])
            if json.loads(log.read_text()) != result:
                raise ValueError("missing or malformed diagnostic completion")
            contract = Path(result["command"][5])
            if contract.read_text() != result["contract"]:
                raise ValueError("wrong contract content")
            ready_report(log.with_suffix(".stdout").read_bytes(), result["identity"])
            # Read the entire diagnostic stream too; absent logs must never pass.
            log.with_suffix(".stderr").read_bytes()
        except (ValueError, OSError, KeyError, TypeError, IndexError) as error:
            failures.append(f"{item['identity']}: {error}")
    if failures:
        raise ValueError('\n'.join(failures))


def campaign(items, workers, scratch):
    worker_count(str(workers))
    validate_selections([item["identity"] for item in items])
    results = []
    errors = []
    manifest = {"complete": False, "workers": workers, "expected": items, "results": results}
    save_json(scratch / "manifest.json", manifest)
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = [pool.submit(execute, item) for item in items]
        # Retrieving in fixture order does not serialize execution.
        for item, future in zip(items, futures):
            try:
                result = future.result()
                results.append(result)
                log = Path(result["log_path"])
                print(result["identity"], "exit", result["exit_status"],
                      "seconds", f"{result['duration_seconds']:.6f}", "log", log, flush=True)
                for suffix, stream in ((".stdout", sys.stdout), (".stderr", sys.stderr)):
                    stream.flush()
                    stream.buffer.write(log.with_suffix(suffix).read_bytes())
                    stream.flush()
            except Exception as error:
                errors.append(f"worker/publication exception for {item['identity']}: {error!r}")
                print(errors[-1], file=sys.stderr, flush=True)
    save_json(scratch / "manifest.json", manifest)
    validate_results(items, results)
    if errors:
        raise ValueError('\n'.join(errors))
    manifest["complete"] = True
    save_json(scratch / "manifest.json", manifest)
    print(MARKER, flush=True)


def compare(reference, candidate):
    """No report/diagnostic normalization. Only the --contract path may differ."""
    manifests = [json.loads(Path(path).read_text()) for path in (reference, candidate)]
    for manifest in manifests:
        if manifest["complete"] is not True:
            raise ValueError("incomplete campaign")
        if [item["identity"] for item in manifest["expected"]] != selections():
            raise ValueError("campaign does not match the frozen fixture order")
        validate_results(manifest["expected"], manifest["results"])
    if manifests[0]["workers"] != 1:
        raise ValueError("reference must use one worker")
    for left, right in zip(manifests[0]["results"], manifests[1]["results"]):
        for key in ("identity", "index", "contract", "exit_status", "error"):
            if left[key] != right[key]:
                raise ValueError(f"non-equivalent {key}: {left['identity']}")
        commands = [list(result["command"]) for result in (left, right)]
        for command in commands:
            if command[4] != "--contract":
                raise ValueError("unexpected CLI argument structure")
            command[5] = "<selection-contract>"
        if commands[0] != commands[1]:
            raise ValueError(f"non-equivalent CLI arguments: {left['identity']}")
        for suffix in (".stdout", ".stderr"):
            if (Path(left["log_path"]).with_suffix(suffix).read_bytes()
                    != Path(right["log_path"]).with_suffix(suffix).read_bytes()):
                raise ValueError(f"non-equivalent {suffix}: {left['identity']}")
        print(f"EQUIVALENT\t{left['identity']}")
    print("TASK062 SERIAL/PARALLEL EQUIVALENCE: PASSED (54 selections, 30 identities)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?")
    parser.add_argument("--compare", nargs=2, metavar=("REFERENCE", "CANDIDATE"))
    args = parser.parse_args()
    if args.compare:
        compare(*args.compare)
        return
    if not args.binary:
        parser.error("binary is required unless --compare is used")
    workers = worker_count(os.environ.get("AMS_GRA_TASK062_WORKERS", str(DEFAULT_WORKERS)))
    binary = Path(args.binary).resolve(strict=True)
    scratch = Path(tempfile.mkdtemp(prefix="task062-service-impact-", dir=os.environ["TMPDIR"]))
    print(f"TASK062 evidence: {scratch / 'manifest.json'} workers={workers}", flush=True)
    campaign(prepare(binary, selections(), scratch), workers, scratch)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, IndexError) as error:
        print(f"TASK062 service qualification FAILED: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
