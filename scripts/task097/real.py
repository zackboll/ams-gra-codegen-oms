"""Complete unchanged newly READY UCI services, independently authored JSON."""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--uci25", type=Path, required=True)
    parser.add_argument("--uci26", type=Path, required=True)
    args = parser.parse_args()
    scratch, setup = args.scratch.resolve(), args.setup.resolve()
    scratch.mkdir(parents=True, exist_ok=False)

    def run(label, command, cwd=ROOT, reject=False):
        prefix = scratch / label
        prefix.with_suffix(".command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}, indent=2) + "\n")
        with prefix.with_suffix(".stdout").open("wb") as out, prefix.with_suffix(".stderr").open("wb") as err:
            process = subprocess.run(command, cwd=cwd, stdout=out, stderr=err)
        prefix.with_suffix(".exit").write_text(str(process.returncode) + "\n")
        if (process.returncode == 0) == reject:
            raise RuntimeError(f"failed {label}: {process.returncode}")
        return prefix.with_suffix(".stdout").read_text()

    # Reuse verified provisioning, but copy every writable provider build tree.
    environment = run("printenv", ["alr", "-s", str(setup / "settings"), "-n", "printenv"], setup / "consumer")
    provider = setup / "consumer/alire/cache/dependencies"
    shutil.copytree(provider, scratch / "provider")
    environment = environment.replace(str(provider), str(scratch / "provider"))
    environment_file = scratch / "environment.sh"
    environment_file.write_text(environment)
    identity = {"UUID": "12345678-1234-1234-8234-123456789abc"}
    date = "2026-01-02T03:04:05Z"
    frequency = {"Min": 1.0000000000000002, "Max": 100.0}
    data = {
        "EntityLost": {"EntityID": identity, "LastObservationTime": date, "MissingCount": 1,
                       "Probability": 1.0000000000000002, "PreviousProbability": -0.0, "Source": {"SystemID": identity}},
        "EntityConfirmation": {"EntityID": identity, "ConfirmationProbability": 1.0000000000000002, "ConfirmationLevel": "CONFIRMED"},
        "EarthOrientationParameters": {"EarthOrientationParametersID": identity, "CreationTime": date, "Epoch": {},
            "Parameters": [{"TAI_UTC_Difference": 37, "UT1_UTC_Difference": 1.0000000000000002,
                            "UT1_Rate": -0.0, "PolarMotion": {"X": 0.5, "Y": -0.5}, "NutationModelTerms": 1, "Epoch": {}}]},
        "SpectralDensityReport": {"SpectralDensityReportID": identity, "StartTimestamp": date,
            "SpectralBand": [{"Frequency": frequency, "SpectralDensity": [1.0000000000000002, -0.0], "SpectralImage": "80FF"}]},
        "SubsystemCalibrationStatus": {"SubsystemID": identity,
            "ActiveCalibration": [{"CalibrationID": identity, "EstimatedPercentComplete": 1.0000000000000002}]},
        "LAR_Report": {"LAR_ID": identity, "InsideLAR": True,
            "LAR_Analysis": {"LAR_EntranceTime": date, "DistanceToEnterLAR": 1.0000000000000002,
                             "LAR_ExitTime": date, "DistanceToExitLAR": 100.0},
            "ReleaseConditions": {"ValidAltitude": True, "ValidVelocity": False, "ValidHeading": True, "AltitudeDelta": -0.0}},
        "RF_Profile": {"RF_ProfileID": identity, "ProfileBand": [{"Frequency": frequency, "ReceiveAllowed": True,
            "TransmitAllowed": False, "MaximumTransmitPower": 1.0000000000000002, "DutyCycle": 0.5}]},
    }
    cells = []
    for release, schema in [("25", args.uci25.resolve()), ("26", args.uci26.resolve())]:
        expected = {"25": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27", "26": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b"}[release]
        if hashlib.sha256(schema.read_bytes()).hexdigest() != expected:
            raise RuntimeError("root drift")
        tree = ET.parse(schema).getroot()
        rows = json.loads((args.inventory / f"{release}-selections.json").read_text())
        for row in rows:
            name = row.get("message")
            profiles = row["outcome"].get("profiles", {})
            if not name or not profiles.get("lossless-v1", {}).get("ready") or profiles.get("float32", {}).get("ready"):
                continue
            label = f"{release}-{name}"
            consumer = scratch / label / "consumer"
            consumer.parent.mkdir()
            contract = args.inventory / f"{release}-{name}.yaml"
            common = ["--schema", str(schema), "--contract", str(contract), "--language", "ada", "--world", "closed-schema"]
            for variant, generator, runtime in [("baseline-portable", args.baseline.resolve(), []),
                ("baseline-float32", args.baseline.resolve(), ["--ada-codec-runtime", "linux-x86_64-glibc-2.41"]),
                ("current-lossless", args.generator.resolve(), ["--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1"])]:
                reject = variant != "current-lossless"
                run(f"{label}-{variant}-check", [str(generator), "service-check", *common, "--with-codec", *runtime], reject=reject)
                output = consumer if not reject else consumer.parent / variant
                run(f"{label}-{variant}-generate", [str(generator), "service-generate", *common, "--with-codec", *runtime, "--output", str(output)], reject=reject)
                if reject and output.exists():
                    raise RuntimeError("non-atomic real generation rejection")
            element = tree.find(f"{{http://www.w3.org/2001/XMLSchema}}element[@name='{name}']")
            payload = element.attrib["type"].split(":")[-1]
            source = (ROOT / "tests/ada-float64-codec/real_consumer.adb").read_text().replace("TASK097_PAYLOAD", payload)
            (consumer / "real_consumer.adb").write_text(source)
            shutil.copytree(ROOT / "ada/oms-json-support", consumer.parent / "support")
            (consumer / "consumer.gpr").write_text(
                'with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
                'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
                'for Main use ("real_consumer.adb");\n'
                'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
                'end Compiler; end Consumer;\n')
            document = {"SecurityInformation": {"Classification": "U", "OwnerProducer": [{"GovernmentIdentifier": "USA"}]},
                "MessageHeader": {"SystemID": identity, "Timestamp": date, "SchemaVersion": f"002.{release[-1]}.0", "Mode": "LIVE"},
                "MessageData": data[name]}
            valid = consumer.parent / "valid.json"
            valid.write_text(json.dumps(document, ensure_ascii=False) + "\n")
            invalid = consumer.parent / "invalid.json"
            invalid.write_text(json.dumps(document | {"Unknown": 0}) + "\n")
            nested = consumer.parent / "nested-invalid.json"
            nested.write_text(json.dumps(document | {"MessageData": data[name] | {"Unknown": 0}}) + "\n")
            for policy in ["Normal", "Check", "Ignore"]:
                command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
                if policy != "Normal":
                    adc = consumer.parent / f"{policy}.adc"
                    adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                    command += ["-cargs:Ada", f"-gnatec={adc}"]
                run(f"{label}-{policy}-build", ["sh", "-ec", f". {shlex.quote(str(environment_file))}; exec " + shlex.join(command)], consumer)
                output = run(f"{label}-{policy}-valid", [str(consumer / "bin/real_consumer"), "accept", str(valid)], consumer)
                lines = output.splitlines()
                if len(lines) != 2 or lines[-1] != "TASK097_REAL_ACCEPT_COMPLETE":
                    raise RuntimeError("incomplete real valid execution")
                # Ordinary JSON decoding preserves kind: quoted numerals are str,
                # finite model numbers remain float/int. Also compare every leaf.
                decoded = json.loads(lines[0])
                if decoded != document:
                    raise RuntimeError(f"real semantic mismatch {label}")
                for index, path in enumerate([invalid, nested]):
                    output = run(f"{label}-{policy}-invalid-{index}", [str(consumer / "bin/real_consumer"), "reject", str(path)], consumer)
                    if output != "TASK097_REAL_REJECT_ATOMIC\n":
                        raise RuntimeError("real invalid document publication")
                cells.append({"release": release, "message": name, "payload": payload, "policy": policy})
            print("TASK097_REAL_SERVICE_COMPLETE", release, name, flush=True)
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True, "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_REAL_MATRIX_COMPLETE cells=" + str(len(cells)), flush=True)


if __name__ == "__main__":
    main()