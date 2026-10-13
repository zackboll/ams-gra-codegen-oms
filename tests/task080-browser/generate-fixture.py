"""Generate the task-local IR fixture with the current Rust 1.95 exporter."""
import json
import pathlib
import subprocess
import sys

repo = pathlib.Path(__file__).resolve().parents[2]
scratch = repo / 'target/task080a-scratch'
scratch.mkdir(parents=True, exist_ok=True)
build = subprocess.run(['cargo', '+1.95.0', 'build', '--locked', '-p',
    'ams-gra-oms-schema-docs', '-p', 'ams-gra-oms-xsd-frontend', '--message-format=json'], cwd=repo,
    text=True, stdout=subprocess.PIPE, check=True)
libraries = {}
for line in build.stdout.splitlines():
    message = json.loads(line)
    if message.get('reason') == 'compiler-artifact':
        for filename in message['filenames']:
            if filename.endswith('.rlib'):
                libraries[message['target']['name']] = filename
command = ['rustc', '+1.95.0', '--edition=2024', str(repo / 'tests/task080-browser/generate-fixture.rs'),
    '-L', 'dependency=' + str(repo / 'target/debug/deps'), '-o', str(scratch / 'generate-fixture')]
for name in ['ams_gra_oms_ir', 'ams_gra_oms_schema_docs', 'ams_gra_oms_xsd_frontend']:
    command.extend(['--extern', name + '=' + libraries[name]])
subprocess.run(command, cwd=repo, check=True)
subprocess.run([str(scratch / 'generate-fixture'), 'crates/schema-docs/tests/explorer.xsd',
    sys.argv[1] if len(sys.argv) > 1 else 'target/task080a-fixture'], cwd=repo, check=True)