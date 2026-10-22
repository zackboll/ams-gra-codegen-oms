#!/usr/bin/env python3
"""Compare helper-demand evidence without hiding qualification differences."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib

sys.dont_write_bytecode = True


def verify(root):
    lines = (root / "sources.sha256").read_text().splitlines()
    if not lines:
        raise RuntimeError(f"empty provenance: {root}")
    for line in lines:
        digest, path = line.split("  ", 1)
        if hashlib.sha256(Path(path).read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"provenance mismatch: {path}")


def evidence(root):
    verify(root)
    result = json.loads((root / "result.json").read_text())
    if result.get("completed") is not True or not result.get("records"):
        raise RuntimeError(f"incomplete campaign: {root}")
    identities = [tuple(record.items()) for record in result["records"]]
    if len(identities) != len(set(identities)):
        raise RuntimeError(f"duplicate record: {root}")
    names = ["Array_Encode_Hex_Binary"] if result["red"] else [
        "ArraySample", "Array_Encode_Hex_Binary", "Encode_Hex_Binary",
        "Array_encode_hex_binary", "Array_Decode_Hex_Binary"]
    expected = [(profile, name, policy) for profile in ["oam", "qualified"]
                for name in names for policy in ["normal", "Check", "Ignore"]]
    actual = [(record["profile"], record["name"], record["policy"]) for record in result["records"]]
    if actual != expected:
        raise RuntimeError(f"incomplete/reordered campaign inventory: {root}")
    items = {"result.json": result}
    for record in result["records"]:
        label = f'{record["profile"]}-{record["name"]}'
        # Original implementation was flat; optimized workers own subtrees.
        case = root / label if (root / label / "result.json").exists() else root
        generated = case / label
        for area in [generated, case / f"{label}-plain", case / f"{label}-opted", case / f"{label}-support"]:
            if not area.exists():
                continue
            for path in area.rglob("*"):
                # Build configs contain scratch paths; generated Ada, schema,
                # consumer, provider and C sources must be byte-identical.
                if path.is_file() and path.suffix in [".ads", ".adb", ".c"] and not any(part in ["obj", "alire", "config"] for part in path.relative_to(area).parts):
                    items[f"{label}/{area.name}/{path.relative_to(area)}"] = path.read_bytes()
        for extension in ["xsd", "json"]:
            items[f"{label}.{extension}"] = (case / f"{label}.{extension}").read_bytes()
        cell = f'{label}-{record["policy"]}'
        lock = tomllib.loads((case / f"{cell}-alire.lock").read_text())

        def normalize(value):
            if isinstance(value, dict):
                return {key: normalize(item) for key, item in value.items()}
            if isinstance(value, list):
                return [normalize(item) for item in value]
            if isinstance(value, str):
                # The only lockfile path normalization: case-local provider pin.
                return value.replace(str(case), "<CASE>")
            return value

        items[f"{cell}-lock"] = normalize(lock)
        build = json.loads((case / f"{cell}-build.command.json").read_text())
        items[f"{cell}-outcome"] = (build["exit"], build["expected"])
        identity = (case / f"{cell}-gnat.log").read_bytes()
        banners = re.findall(rb"(?m)^GNAT [^\n]+", identity)
        if len(banners) != 1:
            raise RuntimeError(f"missing compiler banner: {cell}")
        items[f"{cell}-compiler"] = banners
        if record["intentional_red"]:
            diagnostic = (case / f"{cell}-build.log").read_bytes()
            items[f"{cell}-diagnostics"] = re.findall(rb"(?m)^.*(?:error:|Binary_Vectors.*not declared).*$", diagnostic.replace(str(case).encode(), b"<CASE>"))
            if not items[f"{cell}-diagnostics"]:
                raise RuntimeError(f"missing red diagnostic: {cell}")
        else:
            items[f"{cell}-execute"] = (case / f"{cell}-execute.log").read_bytes()
        for suffix in ["portable-reject", "portable-generate-reject"]:
            log = case / f"{label}-{suffix}.log"
            if log.exists():
                items[f"{label}-{suffix}"] = log.read_bytes().replace(str(case).encode(), b"<CASE>")
    return items


def compare(reference, candidate):
    left, right = evidence(reference), evidence(candidate)
    if left.keys() != right.keys():
        raise RuntimeError(f"evidence inventory differs: {left.keys() ^ right.keys()}")
    different = [key for key in left if left[key] != right[key]]
    if different:
        raise RuntimeError(f"non-equivalent qualification: {different}")
    return len(left)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidates", type=Path, nargs="+")
    args = parser.parse_args()
    for candidate in args.candidates:
        count = compare(args.reference.resolve(), candidate.resolve())
        print(f"TASK093_EQUIVALENCE_COMPLETE items={count} reference={args.reference} candidate={candidate}")


if __name__ == "__main__":
    main()