#!/usr/bin/env python3
"""Task 098 real-UCI DLZ vertical: Task 097 parent vs Task 098 generator.

For each unchanged pinned release root: compare service-check for both
generators under the portable, Float32 and lossless profiles; require every
NOT READY generation to be atomic; generate the selected DLZ closure with the
Task 098 generator; prove the model/API bytes equal the parent's model-only
output; compile the actual generated model, API and codec; and run
independently authored valid, boundary and invalid OMS JSON documents under
Normal/Check/Ignore. Time values must round-trip as exact JSON strings and
Float64 values as JSON numbers with exact binary64 bits.

A successfully generated and executed service is not a provider
interoperability claim.
"""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]


def load(name):
    spec = importlib.util.spec_from_file_location(f"task098_{name}", ROOT / f"scripts/task098/{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


qualify, cases = load("qualify"), load("cases")
ROOTS = {"25": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
         "26": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b"}
RUNTIMES = {"portable": [], "float32": ["--ada-codec-runtime", "linux-x86_64-glibc-2.41"],
            "lossless": ["--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1"]}
CONTRACT = """contract_version: "0.1"
service:
  name: task098-dlz
  version: "0.1.0"
  kind: service
standards:
  oms_version: "2.{minor}"
  uci_schema_version: "2.{minor}"
functions:
  - id: f
    name: F
    category: specific
    applicability: applicable
    exchanges:
      - id: e
        kind: oms_message
        direction: input
        mandate: mandatory
        message: DLZ
        topic: task098-dlz
        timing:
          kind: asynchronous
"""
TIMES = ["TimeOfIntercept", "TimeToRelease", "TimeOfActivation", "TimeOfMPRF", "TimeOfAutonomous"]


def document(release):
    """Independently authored DLZ_MT body using real required/optional members."""
    identity = {"UUID": "550e8400-e29b-41d4-a716-446655440000"}
    return {
        "SecurityInformation": {
            "Classification": "U",
            # Real required repeated Choice: both OwnerProducerChoiceType alternatives.
            "OwnerProducer": [{"GovernmentIdentifier": "USA"}, {"NATO_SpecialWord": "NATO:COSMIC"}],
        },
        "MessageHeader": {"SystemID": identity, "Timestamp": "2026-01-02T03:04:05Z",
                          "SchemaVersion": f"002.{release[-1]}.0", "Mode": "LIVE"},
        "MessageData": {
            "DLZ_ID": identity, "CapabilityID": identity, "DLZ_Type": "DLZ_STAPLE",
            "GeneratedByWeapon": False,
            "DLZ_Data": {
                "RangeMinimum": 1.0, "RangeTurnAndRun": 2.5, "RangeProbabilityOfIntercept": 1.0000000000000002,
                "RangeMaxGuidance": 100.0, "RangeOptimal": 50.0, "RangeMaxAero": 75.0,
                "TargetClosureRate": {"NorthSpeed": -12.5, "EastSpeed": 0.25},
                "TargetRange": 30.0,
                "SteeringUnitVector": {"LOS_ReferenceFrame": "NED",
                                       "LOS_UnitVector": {"X": 0.5, "Y": -0.5, "Z": 1.0}},
                "LoftAngle": -0.0,
                "TimeOfIntercept": "12:34:56.5000Z",
                "TimeToRelease": "24:00:00.000Z",
                "TimeOfActivation": "23:59:60Z",
                "TimeOfMPRF": "00:00:00Z",
                "TimeOfAutonomous": "12:34:56.12345678901234567890Z",
            },
        },
    }


def data_path(*steps):
    return ["MessageData", "DLZ_Data", *steps]


# Accepted boundaries: (path, input value, expected encoded value).
BOUNDARY = [
    (data_path("TimeOfIntercept"), "\t24:00:00.00000000000000000000000000000000000000000000000000Z\n",
     "24:00:00.00000000000000000000000000000000000000000000000000Z"),
    (data_path("TimeToRelease"), "23:59:60.999999Z", "23:59:60.999999Z"),
    (data_path("TimeOfActivation"), "12:00:60.000Z", "12:00:60.000Z"),
    (data_path("RangeMinimum"), 5e-324, 5e-324),
    (data_path("RangeMaxGuidance"), 1.7976931348623157e308, 1.7976931348623157e308),
    (["SecurityInformation", "OwnerProducer"], [{"NATO_SpecialWord": "NATO:ATOMAL"}],
     [{"NATO_SpecialWord": "NATO:ATOMAL"}]),
]
# Rejected: (path, value, reason). Every value is chosen from the real facets.
INVALID = [
    (data_path("TimeOfIntercept"), "12:34:56", "missing terminal Z"),
    (data_path("TimeOfIntercept"), "12:34:56+00:00", "numeric offset, not Z"),
    (data_path("TimeToRelease"), "24:00:00.001Z", "hour 24 with nonzero fraction"),
    (data_path("TimeToRelease"), "12:34:61Z", "second 61"),
    (data_path("TimeOfActivation"), "12:60:00Z", "minute 60 in optional member"),
    (data_path("TimeOfMPRF"), "\u0661" + "2:34:56Z", "non-ASCII digit lookalike"),
    (data_path("TimeOfAutonomous"), "12: 34:56Z", "embedded whitespace"),
    (data_path("TimeOfIntercept"), 123456, "JSON number instead of string"),
    (data_path("TimeToRelease"), None, "JSON null"),
    (data_path("RangeMinimum"), -1.0, "DoubleNonNegativeType minInclusive 0"),
    (data_path("RangeMinimum"), "1.0", "quoted Float64"),
    (["SecurityInformation", "OwnerProducer"], [{"NATO_SpecialWord": "COSMIC"}], "NATO special word prefix"),
    (["SecurityInformation", "OwnerProducer"], [{"GovernmentIdentifier": "USA", "NATO_SpecialWord": "NATO:X"}],
     "two Choice alternatives"),
]


def removed(value, path):
    result = copy.deepcopy(value)
    target = result
    for step in path[:-1]:
        target = target[step]
    del target[path[-1]]
    return result


def readiness_matrix(campaign, release, common, parent, generator):
    """Parent/current x portable/Float32/lossless check + atomic generation."""
    readiness = {}
    for variant, binary in [("parent", parent), ("current", generator)]:
        for runtime, option in RUNTIMES.items():
            label = f"{release}-{variant}-{runtime}"
            output = campaign.scratch / f"{label}-output"
            exits = {}
            for action in ["check", "generate"]:
                command = [binary, f"service-{action}", *common, "--with-codec", *option]
                if action == "generate":
                    command += ["--output", output]
                exits[action] = campaign.run_any(f"{label}-{action}", command)
            text = (campaign.scratch / f"{label}-check.stdout").read_text()
            if exits["check"] != exits["generate"]:
                raise RuntimeError(f"{label}: check/generate disagree")
            if exits["generate"] and output.exists():
                raise RuntimeError(f"{label}: non-atomic rejected generation")
            counts = [line for line in text.splitlines() if line.startswith("codec renderable")]
            blockers = [line for line in text.splitlines() if "service codec boundary" in line]
            readiness[label] = {"exit": exits["check"],
                                "status": "READY" if "codec status: READY\n" in text else "NOT READY",
                                "renderable": counts[0].split(": ")[1] if counts else None,
                                "first_blocker": blockers[0] if blockers else None}
            print("TASK098_REAL_READINESS", label, json.dumps(readiness[label]), flush=True)
    return readiness


def run_documents(campaign, release, generated):
    """Compile the actual DLZ closure and run authored documents under all policies."""
    base = document(release)
    accepted = [(base, base)]
    # Absent optional Time members: each optional TimeOf* individually omitted.
    for name in TIMES[2:]:
        accepted.append((removed(base, data_path(name)), removed(base, data_path(name))))
    for path, value, expected in BOUNDARY:
        accepted.append((cases.replaced(base, path, value), cases.replaced(base, path, expected)))
    rejected = [cases.replaced(base, path, value) for path, value, _ in INVALID]
    rejected.append(removed(base, data_path("TimeOfIntercept")))  # missing required Time
    rejected.append(cases.replaced(base, data_path("Unknown"), "12:34:56Z"))  # unknown member
    pairs = accepted + [(value, None) for value in rejected]
    lines = [json.dumps(value, ensure_ascii=False) for value, _ in pairs]
    lines.append(lines[0][:-7])  # malformed JSON
    documents = campaign.scratch / f"{release}-documents.jsonl"
    documents.write_text("\n".join(lines) + "\n")
    expectations = [expected for _, expected in pairs] + [None]
    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        label = f"{release}-dlz-{policy}"
        count = campaign.execute(label, generated, "Programs.Oam", "DLZ_MT", "ams_gra_oms_json_lossless",
                                 policy, documents, expectations, qualify.OAM)
        # `same` already checked kinds and spellings; also prove every Time is
        # a Zulu JSON string and representative Float64 members JSON numbers.
        for line in (campaign.scratch / f"{label}-execute.stdout").read_text().splitlines()[:-1]:
            kind, _, detail = line.partition("\t")
            if kind != "ACCEPT":
                continue
            data = cases.decode(detail)["MessageData"]["DLZ_Data"]
            for name in TIMES:
                if name in data and (type(data[name]) is not str or not data[name].endswith("Z")):
                    raise RuntimeError(f"{label}: {name} is not an exact Zulu JSON string")
            for name in ["RangeMinimum", "RangeMaxGuidance", "TargetRange"]:
                if not isinstance(data[name], cases.Number):
                    raise RuntimeError(f"{label}: {name} is not a JSON number")
        cells.append({"policy": policy, "documents": len(expectations), "accepted": count,
                      "rejected": len(expectations) - count,
                      "invalid_reasons": [reason for _, _, reason in INVALID]})
        print("TASK098_REAL_CELL_COMPLETE", campaign.version, label, len(expectations), flush=True)
    return cells


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--parent", type=Path, required=True, help="Task 097 parent generator")
    parser.add_argument("--uci25", type=Path, required=True)
    parser.add_argument("--uci26", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    campaign = qualify.Campaign(scratch, args.setup.resolve(), args.compiler)
    campaign.provision()
    generator, parent = args.generator.resolve(), args.parent.resolve()
    result = {"generators": {"current": hashlib.sha256(generator.read_bytes()).hexdigest(),
                             "parent": hashlib.sha256(parent.read_bytes()).hexdigest()},
              "sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [
                  Path(__file__), ROOT / "scripts/task098/qualify.py", ROOT / "scripts/task098/cases.py",
                  qualify.CONSUMER]},
              "releases": {}}
    for release, root in [("25", args.uci25.resolve()), ("26", args.uci26.resolve())]:
        if hashlib.sha256(root.read_bytes()).hexdigest() != ROOTS[release]:
            raise RuntimeError(f"pinned UCI 2.{release[-1]} root drift")
        contract = scratch / f"{release}-contract.yaml"
        contract.write_text(CONTRACT.format(minor=release[-1]))
        common = ["--schema", root, "--contract", contract, "--language", "ada", "--world", "closed-schema"]
        readiness = readiness_matrix(campaign, release, common, parent, generator)
        entry = {"root_sha256": ROOTS[release], "readiness": readiness, "executed": False}
        result["releases"][release] = entry
        if readiness[f"{release}-current-lossless"]["status"] != "READY":
            continue
        # Model/API preservation against the parent's own model-only output.
        model = scratch / f"{release}-parent-model"
        campaign.run(f"{release}-parent-model", [parent, "service-generate", *common, "--output", model])
        generated = scratch / f"{release}-current-lossless-output"
        for path in model.iterdir():
            if path.read_bytes() != (generated / path.name).read_bytes():
                raise RuntimeError(f"{release}: Task 098 changed the real model/API {path.name}")
        entry["executed"] = True
        entry["cells"] = run_documents(campaign, release, generated)
        entry["generated_sha256"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                     for p in sorted(generated.iterdir()) if p.is_file()}
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "compiler": campaign.version, **result,
                                     "steps": campaign.steps}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK098_REAL_DLZ_COMPLETE", campaign.version, flush=True)


if __name__ == "__main__":
    main()
