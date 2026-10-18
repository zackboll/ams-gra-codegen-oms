#!/usr/bin/env python3
"""Qualify all independent x87/MXCSR pairs at O0/O2, plus C2 controls."""
import argparse
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)

    def checked(command, label):
        (scratch / f"{label}.command.json").write_text(json.dumps({"command": command, "cwd": str(ROOT)}, indent=2) + "\n")
        with (scratch / f"{label}.log").open("wb") as output:
            process = subprocess.run(command, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT)
        (scratch / f"{label}.exit").write_text(f"{process.returncode}\n")
        if process.returncode:
            raise RuntimeError(f"failed {label}")
        return (scratch / f"{label}.log").read_text()

    checked(["sh", "-ec", "uname -m; getconf GNU_LIBC_VERSION; dpkg-query -W libc6; gcc --version; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6"], "runtime")
    support = ROOT / "ada/oms-json-support/float32/ams_gra_float32.c"
    tests = ROOT / "tests/ada-float32-probe"
    pairs = []
    for optimization in ["0", "2"]:
        binary = scratch / f"rounding-O{optimization}"
        checked(["gcc", "-std=c11", f"-O{optimization}", "-Wall", "-Wextra", "-Werror",
                 str(support), str(tests / "rounding_controls.c"), str(tests / "rounding_harness.c"), "-lm", "-o", str(binary)], f"build-O{optimization}")
        for x87 in range(4):
            for mxcsr in range(4):
                label = f"O{optimization}-{x87}-{mxcsr}"
                output = checked([str(binary), str(x87), str(mxcsr)], label)
                marker = f"TASK095_C3_C_ROUNDING_COMPLETE x87={x87} mxcsr={mxcsr} checks=57 red=0"
                if output.splitlines()[-1] != marker or len(output.splitlines()) != 8:
                    raise RuntimeError(f"incomplete pair {label}")
                pairs.append({"optimization": optimization, "x87": x87, "mxcsr": mxcsr,
                              "supported": x87 == 0 and mxcsr == 0})
    if len(pairs) != 32:
        raise RuntimeError("missing rounding pairs")
    environment = scratch / "environment"
    checked(["gcc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", str(support),
             str(tests / "environment_controls.c"), str(tests / "environment_harness.c"), "-lm", "-o", str(environment)], "build-environment")
    for mode in range(4):
        output = checked([str(environment), str(mode)], f"environment-{mode}")
        if output != f"TASK095_C2_C_ENVIRONMENT_COMPLETE mode={mode} checks={32 if mode == 0 else 35}\n":
            raise RuntimeError("incomplete C2 controls")
    bridge = scratch / "bridge"
    checked(["gcc", "-std=c11", "-O0", "-Wall", "-Wextra", "-Werror", str(support), str(tests / "bridge_controls.c"),
             "-Wl,--wrap=newlocale", "-Wl,--wrap=gnu_get_libc_version", "-lm", "-o", str(bridge)], "build-bridge")
    if checked([str(bridge)], "bridge") != "TASK095_BRIDGE_CONTROLS_COMPLETE checks=20\n":
        raise RuntimeError("incomplete bridge controls")
    paths = [support, pathlib.Path(__file__).resolve(), *tests.glob("*.c"), *scratch.glob("*.log"), *scratch.glob("*.exit")]
    (scratch / "sources.sha256").write_text("".join(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n" for path in sorted(paths)))
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "pairs": pairs, "c2_modes": 4, "bridge_checks": 20}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK095_C3_C_QUALIFICATION_COMPLETE pairs=32 optimizations=2")


if __name__ == "__main__":
    main()