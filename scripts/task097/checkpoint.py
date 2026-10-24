"""Small actual generated-code gate; no research boundary or adapter is linked."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    setup = args.setup.resolve()
    env = os.environ.copy()
    env.update(HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"),
               XDG_DATA_HOME=str(setup / "home/.local/share"))

    def run(label, command, cwd=ROOT, expected=0):
        (scratch / f"{label}.command.json").write_text(json.dumps({
            "argv": list(map(str, command)), "cwd": str(cwd)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=cwd, env=env, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        if (expected == 0 and result.returncode != 0) or (expected != 0 and result.returncode == 0):
            raise RuntimeError(f"{label}: unexpected exit {result.returncode}")
        return (scratch / f"{label}.stdout").read_text()

    fixture = ROOT / "tests/fixtures/service-generate/codec-ada-float64"
    common = [str(args.generator.resolve()), "service-generate", "--schema", str(fixture.with_suffix(".xsd")),
              "--contract", str(fixture.with_suffix(".yaml")), "--language", "ada", "--world", "closed-schema"]
    for label, option in [("portable", []), ("old", ["--ada-codec-runtime", "linux-x86_64-glibc-2.41"])]:
        output = scratch / label
        run(label, [*common, "--with-codec", *option, "--output", str(output)], expected=1)
        if output.exists():
            raise RuntimeError("non-atomic rejection")
    model = scratch / "model"
    generated = scratch / "generated"
    run("model", [*common, "--output", str(model)])
    run("lossless", [*common, "--with-codec", "--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1", "--output", str(generated)])
    for path in model.iterdir():
        if path.is_file() and path.read_bytes() != (generated / path.name).read_bytes():
            raise RuntimeError(f"model/API changed: {path.name}")
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    shutil.copy2(ROOT / "tests/ada-float64-codec/checkpoint.adb", generated)
    (generated / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("checkpoint.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
        'end Compiler; end Consumer;\n')
    environment = run("printenv", ["alr", "-s", str(setup / "settings"), "-n", "printenv"], setup / "consumer")
    environment_file = scratch / "environment.sh"
    environment_file.write_text(environment)
    run("build", ["sh", "-ec", f". {shlex.quote(str(environment_file))}; exec gprbuild -P consumer.gpr -p"], generated)
    record = ('{"Direct":1.00000000000000011102230246251565404236316680908203126,'
              '"Optional":-0,"Named":5e-324,"Checked":0.5,"Data":"80FF",'
              '"Single":1.00000005960464477539062500000000001,'
              '"Samples":[-0.0,5e-324,1.0000000000000002],"NamedSamples":[0.5,-1]}')
    text = '{"Record":' + record + ',"Pick":{"Nested":' + record + '},"Picks":[{"Nested":' + record + '}]}'
    input_path = scratch / "valid.json"
    input_path.write_text(text + "\n")
    output = run("valid", [str(generated / "bin/checkpoint"), str(input_path)], generated)
    lines = output.splitlines()
    if len(lines) != 2 or lines[-1] != "TASK097_GENERATED_CHECKPOINT_COMPLETE":
        raise RuntimeError("missing generated checkpoint marker")
    spec = importlib.util.spec_from_file_location("oracle", ROOT / "tests/ada-float64-probe/oracle.py")
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    # Explicit kind tag: a quoted decimal remains str, never a Number.
    class Number(str):
        pass
    document = json.loads(lines[0], parse_float=Number, parse_int=Number)
    records = [document["Record"], document["Pick"]["Nested"], document["Picks"][0]["Nested"]]
    for item in records:
        expected = {"Direct": "3ff0000000000001", "Optional": "8000000000000000",
                    "Named": "0000000000000001", "Checked": "3fe0000000000000"}
        for name, bits in expected.items():
            if type(item[name]) is not Number or oracle.oracle(item[name]).lower() != bits:
                raise RuntimeError(f"wrong number kind/value: {name}")
        for token, bits in zip(item["Samples"], ["8000000000000000", "0000000000000001", "3ff0000000000001"], strict=True):
            if type(token) is not Number or oracle.oracle(token).lower() != bits:
                raise RuntimeError("repeated number kind/value")
    for index, invalid in enumerate([
        text[:-1] + ',"Unknown":0}',
        text.replace('"Direct":', '"Unknown":0,"Direct":', 1),
        text.replace('"Pick":{"Nested":', '"Pick":{"Unknown":', 1),
        text.replace('"Direct":1.00000000000000011102230246251565404236316680908203126', '"Direct":"1.5"', 1),
        text.replace('"Checked":0.5', '"Checked":1', 1),
        text.replace('"Named":5e-324', '"Named":1e999', 1),
    ]):
        path = scratch / f"invalid-{index}.json"
        path.write_text(invalid + "\n")
        run(f"invalid-{index}", [str(generated / "bin/checkpoint"), str(path)], generated, expected=1)
        if (scratch / f"invalid-{index}.stdout").read_text():
            raise RuntimeError("invalid decode published output")
    paths = [p for parent in [generated, scratch / "support"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".adb", ".ads", ".c", ".h", ".gpr"]]
    paths += [Path(__file__), args.generator.resolve(), fixture.with_suffix(".xsd"), fixture.with_suffix(".yaml")]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True,
                                    "record_positions": 3, "negative_cases": 6}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_CHECKPOINT_COMPLETE", flush=True)


if __name__ == "__main__":
    main()