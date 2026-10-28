"""Every portable naming/policy reuse boundary, using actual read-time inputs."""
import json
from pathlib import Path

from perf005_evidence import drivers, effective_args, successful
from perf005_mapping import conventional_runtime, invocation_reads, parse_mapping, runtime_root
from perf005_provider import digest
from perf009_evidence import verify_trace
from perf009_portable import cells


def boundary(original, prepared, label, policy):
    paths = [original / f"{label}-{policy}-build.profile.json",
             prepared / f"provider-{policy}-build.profile.json"]
    roots, labels = (original, prepared), (label, "oam-ArraySample")
    datasets = [verify_trace(path) for path in paths]

    def normalize(text, root, name):
        return text.replace(str(root) + "-state", "<STATE>").replace(str(root), "<CAMPAIGN>").replace(name, "<CONSUMER>")

    inventories = []
    for data, root, name in zip(datasets, roots, labels):
        inventory = {}
        for row in drivers(data):
            sources = [s for s in row["argv"] if "/alire/cache/dependencies/gnatcoll_" in s
                       and s.endswith((".ads", ".adb", ".c", ".s", ".S"))]
            if not sources:
                continue
            successful(row)
            if len(sources) != 1:
                raise RuntimeError("ambiguous provider source")
            key = normalize(sources[0], root, name)
            if key in inventory:
                raise RuntimeError("duplicate provider input")
            inventory[key] = (row, sources[0])
        inventories.append(inventory)
    if inventories[0].keys() != inventories[1].keys() or len(inventories[0]) != 106:
        raise RuntimeError("portable provider boundary inventory differs")
    reports, witnessed, extras = [], {}, []
    for source in sorted(inventories[0]):
        effective, maps, reads = [], [], []
        for inventory, data, root, name, path in zip(inventories, datasets, roots, labels, paths):
            row, actual_source = inventory[source]
            args = effective_args(data, row["argv"][1:], root, source=actual_source)
            effective.append(json.loads(json.dumps([a for a in args if not isinstance(a, list) or a[0] != "-gnatem="]).replace(name, "<CONSUMER>")))
            mapping = [a.split("=", 1)[1] for a in row["argv"] if a.startswith("-gnatem=")]
            if actual_source.endswith((".ads", ".adb")):
                if len(mapping) != 1:
                    raise RuntimeError("missing portable read-time mapping")
                text = data["pre_execution_mapping_inputs"][actual_source + "\n" + mapping[0]]
                maps.append({tuple(normalize(v, root, name) for v in r): r for r in parse_mapping(text)})
                actual_reads = invocation_reads(data, path.with_name(path.name.replace(".profile.json", ".strace")), actual_source, root)
                reads.append({p.replace(name, "<CONSUMER>"): h for p, h in actual_reads.items()})
        if effective[0] != effective[1]:
            raise RuntimeError("portable provider effective configuration/response differs")
        if maps:
            if not reads[0] or reads[0] != reads[1]:
                raise RuntimeError("portable provider compiler source reads differ")
            witnessed.update(reads[0])
            for side in (0, 1):
                other = 1 - side
                for key in maps[side].keys() - maps[other].keys():
                    if any(key[0] == k[0] or key[1] == k[1] for k in maps[other].keys() - maps[side].keys()):
                        raise RuntimeError("conflicting portable read-time mapping")
                    unit, filename, actual = maps[side][key]
                    compiler = inventories[side][source][0]["executable"]
                    runtime = runtime_root(roots[side])
                    h = conventional_runtime(unit, filename, Path(actual), runtime, compiler)
                    other_runtime = runtime_root(roots[other])
                    if h != conventional_runtime(unit, filename, other_runtime / filename, other_runtime,
                                                  inventories[other][source][0]["executable"]):
                        raise RuntimeError("portable mapped runtime differs")
                    extras.append((key[2], h))
                for key in maps[0].keys() & maps[1].keys():
                    if digest(Path(maps[0][key][2])) != digest(Path(maps[1][key][2])):
                        raise RuntimeError("portable mapped source differs")
        reports.append(dict(source=source, mapping=bool(maps), equivalent=True))
    if any(witnessed.get(p) != h for p, h in extras):
        raise RuntimeError("unwitnessed portable runtime cache difference")
    return dict(label=label, policy=policy, provider_invocations=len(reports),
                ada_mappings=sum(r["mapping"] for r in reports), equivalent=True,
                original_profile_sha256=digest(paths[0]), preparation_profile_sha256=digest(paths[1]))


def all_boundaries(original, prepared):
    return [boundary(original, prepared, f"{profile}-{name}", policy)
            for profile, name, policy in cells()]