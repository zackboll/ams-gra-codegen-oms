#!/usr/bin/env python3
"""CPP-OMS-006: whole-release C++ `--with-codec` readiness sweep.

Every global element of each pinned UCI root becomes a one-output-exchange
contract. Production `service-check-batch` (C++, closed-schema, with_codec)
evaluates the contracts, with one schema load per worker batch. A CLI given
as --baseline is swept the same way, and the two are compared per release.

    python3 -I scripts/check-cpp-oms-006-codec-sweep.py --cli CLI [--baseline CLI] \
        --uci-2-5 ROOT --uci-2-6 ROOT --scratch DIR [--workers N] [--require-no-loss]

Output: totals per release and CLI (model READY, codec READY), newly READY and
lost messages, and a histogram of first codec blockers grouped by kind. Raw
batch output and parsed per-message results stay in --scratch. The script is
read-only with respect to the repository and fails closed. Batch output that
is incomplete, malformed or reordered is an error, and so is a request whose
exit code disagrees with its report.
"""
import argparse
from collections import Counter
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

PINS = {
    "2.5": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
    "2.6": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
}
XS = "{http://www.w3.org/2001/XMLSchema}"


def require(condition, message):
    if not condition:
        raise SystemExit(f"check-cpp-oms-006-codec-sweep: {message}")


def global_elements(root):
    tree = ET.parse(root).getroot()
    names = [e.get("name") for e in tree if e.tag == XS + "element"]
    require(len(names) == len(set(names)) and all(names), "global element names are not unique")
    return names


def contract(path, version, message):
    path.write_text(f'''contract_version: "0.1"
service:
  name: cpp-oms-006-sweep
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
      - id: out
        kind: oms_message
        direction: output
        mandate: mandatory
        message: {message}
        topic: t
        timing:
          kind: asynchronous
''')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key {key}")
        result[key] = value
    return result


def field(report, prefix):
    for line in report.splitlines():
        if line.startswith(prefix):
            return line[len(prefix):]
    return None


def classify(blocker):
    """Coarse, documented grouping of a first codec blocker line."""
    if blocker is None:
        return None
    # The "(supported: ...)" suffix lists admitted profiles; never match it.
    blocker = re.sub(r" [(]supported:.*", "", blocker)
    for needle, kind in [
        ("String profile structured ASCII", "structured ASCII"),
        ("String profile bounded ASCII", "bounded ASCII"),
        ("String profile alternating ASCII", "alternating ASCII"),
        ("String profile IPv6", "IPv6 address"),
        ("String profile Unicode 3.1", "Unicode 3.1"),
        ("Float32", "Float32"),
        ("named Time", "named Time"),
        ("hexBinary", "constrained hexBinary"),
        ("uninhabited abstract", "uninhabited abstract member"),
        ("\"$type\" discriminator", "abstract $type"),
        ("Choice alternatives", "optional/repeated Choice alternative"),
    ]:
        if needle in blocker:
            return kind
    return "other"


def batch(cli, root, version, requests, worker_dir):
    worker_dir.mkdir(parents=True, exist_ok=True)
    manifest = worker_dir / "manifest.json"
    manifest.write_text(json.dumps(dict(format="service-check-batch-v1",
                                        schema_version=f"002.{version[-1]}.0", requests=requests)))
    run = subprocess.run([cli, "service-check-batch", "--schema", root, "--manifest", str(manifest)],
                         capture_output=True, check=False)
    (worker_dir / "stdout.json").write_bytes(run.stdout)
    (worker_dir / "stderr.log").write_bytes(run.stderr)
    report = json.loads(run.stdout, object_pairs_hook=unique_object)
    require(report.get("format") == "service-check-batch-results-v1", f"{worker_dir}: format")
    require(report.get("schema_loads") == 1, f"{worker_dir}: schema loads")
    results = report["results"]
    require(report["completed"] == len(results) == len(requests), f"{worker_dir}: incomplete batch")
    require([r["request"]["id"] for r in results] == [r["id"] for r in requests], f"{worker_dir}: reordered")
    failed = any(r["exit_code"] != 0 for r in results)
    require(run.returncode == (1 if failed else 0), f"{worker_dir}: exit status disagrees")
    return results


def sweep(cli, label, roots, scratch, workers):
    out = {}
    for version, root in roots.items():
        names = global_elements(root)
        cdir = scratch / f"contracts-{version}"
        cdir.mkdir(parents=True, exist_ok=True)
        requests = []
        for name in names:
            path = cdir / f"{name}.yaml"
            contract(path, version, name)
            requests.append(dict(id=name, contract=str(path), language="cpp", world="closed-schema",
                                 with_codec=True))
        parts = [requests[i::workers] for i in range(workers)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
            groups = list(pool.map(lambda p: batch(cli, root, version, p[1],
                                                   scratch / f"{label}-{version}-w{p[0]}"),
                                   enumerate(parts)))
        rows = {}
        for group in groups:
            for item in group:
                stdout = item["stdout"]
                renderable = field(stdout, "codec renderable emitted declarations: ")
                rows[item["request"]["id"]] = dict(
                    exit_code=item["exit_code"],
                    contract_valid=stdout.startswith("service contract valid"),
                    closure=field(stdout, "selected type closure: "),
                    model=field(stdout, "status: "),
                    codec=field(stdout, "codec status: "),
                    renderable=renderable,
                    blocker=field(stdout, "service codec boundary: "),
                )
        require(set(rows) == set(names), f"{label} {version}: missing results")
        (scratch / f"{label}-{version}.json").write_text(json.dumps(rows, indent=1, sort_keys=True) + "\n")
        out[version] = rows
    return out


def totals(rows):
    messages = [n for n, r in rows.items() if r["contract_valid"]]
    model = [n for n in messages if rows[n]["model"] == "READY"]
    codec = [n for n in model if rows[n]["codec"] == "READY"]
    return messages, model, codec


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True)
    parser.add_argument("--baseline")
    parser.add_argument("--uci-2-5", required=True)
    parser.add_argument("--uci-2-6", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--require-no-loss", action="store_true")
    args = parser.parse_args()
    require(1 <= args.workers <= 16, "--workers must be in 1..16")
    roots = {"2.5": str(Path(args.uci_2_5).resolve()), "2.6": str(Path(args.uci_2_6).resolve())}
    for version, root in roots.items():
        require(hashlib.sha256(Path(root).read_bytes()).hexdigest() == PINS[version],
                f"UCI {version} root is not the pinned file")
    scratch = Path(args.scratch).resolve()
    scratch.mkdir(parents=True, exist_ok=True)
    after = sweep(args.cli, "after", roots, scratch, args.workers)
    before = sweep(args.baseline, "before", roots, scratch, args.workers) if args.baseline else None
    lost_any = False
    for version in roots:
        messages, model, codec = totals(after[version])
        print(f"UCI {version}: global elements {len(after[version])}, valid single-message contracts "
              f"{len(messages)}, model READY {len(model)}, codec READY {len(codec)} (after)")
        blockers = Counter(classify(after[version][n]["blocker"]) for n in model
                           if after[version][n]["codec"] != "READY")
        print(f"UCI {version}: first codec blockers (after, model READY only): "
              + ", ".join(f"{k} {v}" for k, v in sorted(blockers.items(), key=lambda kv: (-kv[1], kv[0]))))
        if before is None:
            continue
        b_messages, b_model, b_codec = totals(before[version])
        require(b_messages == messages, f"UCI {version}: contract validity changed")
        require(sorted(b_model) == sorted(model), f"UCI {version}: model readiness changed")
        print(f"UCI {version}: model READY {len(b_model)} -> {len(model)}, "
              f"codec READY {len(b_codec)} -> {len(codec)}")
        newly = sorted(set(codec) - set(b_codec))
        lost = sorted(set(b_codec) - set(codec))
        moved = sorted(n for n in model if before[version][n]["renderable"] != after[version][n]["renderable"])
        b_blockers = Counter(classify(before[version][n]["blocker"]) for n in b_model
                             if before[version][n]["codec"] != "READY")
        print(f"UCI {version}: first codec blockers (before, model READY only): "
              + ", ".join(f"{k} {v}" for k, v in sorted(b_blockers.items(), key=lambda kv: (-kv[1], kv[0]))))
        print(f"UCI {version}: newly READY {len(newly)}: {', '.join(newly)}")
        print(f"UCI {version}: lost {len(lost)}: {', '.join(lost) or '-'}")
        print(f"UCI {version}: messages whose codec renderable count moved: {len(moved)}")
        regressions = [n for n in model if before[version][n]["renderable"] and after[version][n]["renderable"]
                       and int(after[version][n]["renderable"].split("/")[0])
                       < int(before[version][n]["renderable"].split("/")[0])]
        print(f"UCI {version}: messages whose renderable count fell: {len(regressions)}"
              + (f": {', '.join(regressions)}" if regressions else ""))
        for n in newly:
            print(f"  newly READY {version} {n}: {before[version][n]['renderable']} -> "
                  f"{after[version][n]['renderable']}; before blocker: "
                  f"{re.sub(r' [(]supported:.*', '', before[version][n]['blocker'] or '')}")
        lost_any = lost_any or bool(lost) or bool(regressions)
    if args.require_no_loss and lost_any:
        raise SystemExit("check-cpp-oms-006-codec-sweep: a message lost codec readiness or renderable declarations")
    print("CPP_OMS_006_SWEEP_COMPLETE")


if __name__ == "__main__":
    sys.exit(main())
