"""Production portable provider reuse using the qualified PERF-005 mechanism.

No cross-campaign cache, fallback, representative matrix or benchmark gate.
Detailed compiler/read-time evidence is mandatory, not a timing certificate.
"""
import json
import os
from pathlib import Path
import time

from perf005_profile import run
from perf005_sandbox import consumer_layout
from perf009_evidence import verify_execution, verify_preparation, verify_trace
from perf009_portable import MARKER, cells, inventory, validate_build, validate_result
from perf009_provider import PortablePilot, validate_lock, verify_live_identity
from perf005_provider import Pilot, digest, sources


def verify_consumer_reads(data, trace, consumer, identity):
    """Reconcile compiler read-time mappings against exact deployed provider ABI."""
    from perf005_mapping import parse_mapping, source_reads
    pids = frozenset(r["pid"] for r in data["executions"] if Path(r["executable"]).name == "gnat1"
                     and any(str(p) in r["argv"] for p in inventory(consumer)))
    reads = source_reads(trace, pids)
    if set(reads) != set(pids):
        raise RuntimeError("missing consumer compiler source-read evidence")
    roots = sources(consumer)
    providers = {str(root / relative): h for name, root in roots.items()
                 for relative, h in identity["sources"][name].items()}
    checked = set()
    for pid, files in reads.items():
        for path, digest_read in files.items():
            if "/alire/cache/dependencies/gnatcoll_" in path:
                if providers.get(path) != digest_read:
                    raise RuntimeError("consumer read changed provider source")
                checked.add(path)
    if not checked:
        raise RuntimeError("no provider ABI source reads witnessed")
    for mapping in data["pre_execution_mapping_inputs"].values():
        for unit, filename, path in parse_mapping(mapping):
            if "/alire/cache/dependencies/gnatcoll_" in path:
                if path not in providers or digest(Path(path)) != providers[path]:
                    raise RuntimeError("consumer read-time provider mapping changed")
    return dict(compiler_backends=len(pids), provider_source_reads=len(checked))


class Campaign:
    def __init__(self, scratch, compiler, state, jobs, reuse):
        self.scratch, self.compiler, self.state = scratch, compiler, state
        self.jobs, self.reuse = jobs, reuse
        self.pilot = PortablePilot(scratch, compiler, self.checked, jobs=jobs)

    def checked(self, command, cwd, stem, expected=0):
        forced = "-XEXTERNALLY_BUILT=true" in command
        building = "gprbuild" in command or command[:4] == ["alr", "-n", "build", "--"]
        executing = stem.name.endswith("-execute")
        if command[:4] == ["alr", "-n", "build", "--"]:
            index = command.index("-cargs:Ada") if "-cargs:Ada" in command else len(command)
            command = command[:index] + [f"-j{self.jobs}"] + command[index:]
        writable, protected = None, ()
        if forced:
            writable, protected = consumer_layout(command, cwd, self.state, self.state / "home")
        started_ns = time.time_ns()
        code, text = run(command, cwd, stem, trace=building or executing,
                         writable=writable, protected=protected)
        if expected is not None and code != expected:
            raise RuntimeError(f"portable command exit={code}, expected={expected}: {stem}")
        if stem.name.endswith("-provider-identity"):
            verify_live_identity(text, self.compiler)
        if forced:
            data = verify_trace(Path(str(stem) + ".profile.json"))
            policy = stem.name.removesuffix("-build").rsplit("-", 1)[1]
            record = validate_build(command, cwd, data, self.compiler, policy, started_ns)
            record["source_reads"] = verify_consumer_reads(
                data, Path(str(stem) + ".strace"), cwd, self.pilot.identity)
            record["started_ns"] = started_ns
            Path(str(stem) + ".verification.json").write_text(json.dumps(record, indent=2) + "\n")
        if executing:
            verify_execution(Path(str(stem) + ".profile.json"), Path(command[0]), MARKER)
        if building and not self.reuse and stem.name.endswith("-build"):
            if stem.name.endswith("-normal-build"):
                self.pilot.configuration(cwd, sources(cwd))
            data = verify_trace(Path(str(stem) + ".profile.json"))
            policy = stem.name.removesuffix("-build").rsplit("-", 1)[1]
            record = validate_build(command, cwd, data, self.compiler, policy, started_ns, original=True)
            Path(str(stem) + ".verification.json").write_text(json.dumps(record, indent=2) + "\n")
        return code, text

    def __call__(self, command, cwd, stem, expected=0):
        if self.reuse and command[:5] == ["alr", "-n", "build", "--", "-f"]:
            return self.pilot.build(command, cwd, stem, expected)
        return self.checked(command, cwd, stem, expected)

    def finish(self):
        validate_result(self.scratch, self.compiler)
        counts = []
        for profile, name, policy in cells():
            consumer = self.scratch / f"{profile}-{name}"
            validate_lock(consumer / "alire/alire.lock", self.compiler)
            path = self.scratch / f"{profile}-{name}-{policy}-build.profile.json"
            data = verify_trace(path)
            from perf005_evidence import drivers, successful
            rows = [r for r in drivers(data) if any("/alire/cache/dependencies/gnatcoll_" in a
                                                    for a in r["argv"])]
            for row in rows:
                successful(row)
            if len(rows) != (0 if self.reuse else 106):
                raise RuntimeError("unexpected actual portable provider compilation count")
            counts.append(len(rows))
        preparation = []
        if self.reuse:
            for policy in ("normal", "Check", "Ignore"):
                preparation.append(verify_preparation(self.scratch / f"provider-{policy}-build.profile.json"))
            provenance = json.loads((self.scratch / "perf005-provider-consumers.json").read_text())
            if [r["cell"] for r in provenance] != [f"{p}-{n}-{a}-build" for p, n, a in cells()]:
                raise RuntimeError("missing/duplicate portable consumer provenance")
        report = dict(completed=True, compiler=self.compiler, reuse=self.reuse,
                      positive_cells=30, execution_cells=30, bind_link_cells=30,
                      generated_support_compilations=150,
                      provider_compilations=sum(counts) + 106 * len(preparation),
                      provider_constructions=len(preparation),
                      memory_method="sampled simultaneous process-tree RSS from mandatory per-command traces",
                      peak_tree_rss_kib=max(json.loads(p.read_text()).get("sampled_peak_tree_rss_kib") or 0
                                            for p in self.scratch.glob("*.profile.json")))
        (self.scratch / "portable-ci-verification.json").write_text(json.dumps(report, indent=2) + "\n")
        return report