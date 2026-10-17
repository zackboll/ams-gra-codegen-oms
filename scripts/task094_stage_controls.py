"""Stage-aware generator rejection evidence shared by temporal regressions."""
import hashlib
import json
import pathlib
import subprocess


def validate_control(generator, common, root, scratch, name, schema, stage, diagnostic):
    if stage not in {"frontend", "model-preflight", "codec-readiness"}:
        raise RuntimeError(f"invalid generator rejection stage: {stage}")
    path = scratch / (name + ".xsd")
    path.write_text(schema)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()

    def invoke(action, codec, expected_exit, phase):
        output = scratch / f"{name}-{phase}-{action}-output"
        options = common if codec else [arg for arg in common if arg != "--with-codec"]
        command = [generator, action, *options, "--schema", str(path)]
        if action == "service-generate":
            command += ["--output", str(output)]
        result = subprocess.run(command, cwd=root, capture_output=True, check=False)
        text = (result.stdout + result.stderr).decode()
        stem = scratch / f"{name}-{phase}-{action}"
        log = pathlib.Path(str(stem) + ".log")
        record = pathlib.Path(str(stem) + ".command.json")
        if log.exists() or record.exists():
            raise RuntimeError(f"evidence collision: {stem}")
        log.write_text(text)
        record.write_text(json.dumps({"command": command, "cwd": str(root),
            "exit": result.returncode, "expected_stage": stage,
            "expected_diagnostic": diagnostic, "actual_diagnostic": text,
            "output_exists": output.exists(), "schema_sha256": digest}, indent=2) + "\n")
        if result.returncode != expected_exit:
            raise RuntimeError(f"{name}: {phase}/{action} wrong exit; see {log}")
        if expected_exit == 0:
            if action == "service-check" and "\nstatus: READY\n" not in text:
                raise RuntimeError(f"{name}: model is not READY; see {log}")
            if action == "service-generate" and not list(output.glob("*.ads")):
                raise RuntimeError(f"{name}: no actual model output")
        else:
            if output.exists() or diagnostic not in text:
                raise RuntimeError(f"{name}: {stage}/{action} wrong diagnostic or partial output; see {log}")
            if stage == "model-preflight" and not (
                "unsupported selected types:" in text and "\nstatus: NOT READY\n" in text
                and "codec: not measured (selected model is NOT READY)" in text
                and "codec status:" not in text
            ):
                raise RuntimeError(f"{name}: rejection was not model-preflight; see {log}")
            if stage == "codec-readiness" and not (
                "\nstatus: READY\n" in text and "codec status: NOT READY" in text
            ):
                raise RuntimeError(f"{name}: rejection was not codec-readiness; see {log}")

    if stage == "codec-readiness":
        for action in ["service-check", "service-generate"]:
            invoke(action, False, 0, "model-proof")
    for action in ["service-check", "service-generate"]:
        invoke(action, True, 1, "rejection")