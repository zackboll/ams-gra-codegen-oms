#!/usr/bin/env python3
"""PERF-007: execute unchanged workflow run bodies and reconcile their evidence.

Only the stdlib is used. No test-output truncation, background qualification,
artifact execution, mutable shared build cache, or successful fallback exists.
"""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import shutil
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = ROOT / 'scripts/real-uci-contract.json'
# Reviewed original marker/count inventory captured before editing the workflow.
CONTRACT_SHA256 = '56819635c5548a74d5ac95bc456b9e9d718673478a972f65e31469f47b544428'
SHARDS = {
    'real-uci-temporal-naming': ['061', '071', '065'],
    'real-uci-schema-ipv6': ['069', '062'],
    'real-uci-integral-unicode': ['063', '066'],
}
ORIGINAL = ['061', '071', '069', '062', '063', '065', '066']
PINS = {
    '2.5': dict(tag='v2.5', revision='093610b7753944059360d3236770ab446d039556',
                sha256='ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27'),
    '2.6': dict(tag='v2.6', revision='78eb61b6112c8bffa40820c33124b57787fc5bd9',
                sha256='af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encode(value):
    return (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, stderr=subprocess.STDOUT).decode().strip()


def contract():
    raw = CONTRACT_PATH.read_bytes()
    if digest(raw) != CONTRACT_SHA256:
        raise ValueError('original qualification marker/count contract changed')
    value = json.loads(raw)
    reconcile(value)
    return value


def evidence_steps(job):
    return f'''      - name: Reconcile complete shard evidence
        if: always()
        run: python3 scripts/real_uci_evidence.py finish
      - name: Preserve mandatory shard evidence and failure diagnostics
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: perf007-{job}
          path: ${{{{ runner.temp }}}}/perf007/{job}/**
          if-no-files-found: error
'''


def check_layout(jobs):
    """Independent static original == disjoint union check, before execution."""
    value = contract()
    if 'real-uci' in jobs or not set(SHARDS) <= jobs.keys():
        raise ValueError('missing primary shard or surviving serial job')
    commands = []
    for job, tasks in SHARDS.items():
        body = jobs[job]
        expected = [value['tasks'][t]['command'] for t in tasks]
        actual = re.findall(r'^        run: (bash scripts/check-task(?:061|071|069|062|063|065|066)-(?:pinned|uci)\.sh)$', body, re.M)
        if actual != expected:
            raise ValueError('omitted/duplicated/reordered primary task: ' + job)
        for task in tasks:
            step = ('        shell: python3 scripts/real_uci_evidence.py task ' + task + ' {0}\n'
                    '        run: ' + value['tasks'][task]['command'])
            if body.count(step) != 1:
                raise ValueError('missing evidence shell: ' + task)
        if (body.count(evidence_steps(job).rstrip()) != 1 or '    needs:' in body
                or re.search(r'actions/cache|download-artifact|CARGO_TARGET_DIR|target-directory', body)):
            raise ValueError('missing evidence or non-independent primary shard: ' + job)
        for action in ('init', 'inputs', 'finish'):
            if body.count('run: python3 scripts/real_uci_evidence.py ' + action) != 1:
                raise ValueError('missing/duplicated evidence action: ' + action)
        for name in ('gnat', 'fetch', 'build'):
            if body.count('shell: python3 scripts/real_uci_evidence.py setup ' + name + ' {0}') != 1:
                raise ValueError('missing audited setup: ' + name)
        if body.count('cargo fetch --locked') != 1 or body.count('cargo build --release --locked -p ams-gra-codegen-oms') != 1:
            raise ValueError('missing independent locked release preparation')
        commands.extend(actual)
    # Look across ALL jobs: extra invocations in a renamed/legacy job are invalid.
    all_commands = re.findall(r'bash scripts/check-task(?:061|071|069|062|063|065|066)-(?:pinned|uci)\.sh', '\n'.join(jobs.values()))
    if Counter(commands) != Counter(all_commands):
        raise ValueError('original task executes more than once across Deep jobs')


def reconcile(value):
    if value['original_order'] != ORIGINAL or value['shards'] != SHARDS or value['pins'] != PINS:
        raise ValueError('original/shard/pinned identity contract changed')
    union = [task for tasks in value['shards'].values() for task in tasks]
    if Counter(union) != Counter(ORIGINAL) or set(value['tasks']) != set(ORIGINAL):
        raise ValueError('missing/duplicate original qualification')
    for task, item in value['tasks'].items():
        if digest((ROOT / item['wrapper']).read_bytes()) != item['wrapper_sha256']:
            raise ValueError('inherited qualification wrapper changed: ' + task)


def validate_output(task, data, value):
    text = data.decode('utf-8', errors='strict')
    item = value['tasks'][task]
    # Include multiplicities (e.g. one Rust codec marker for each release).
    actual = Counter(text.splitlines())
    for marker, count in item['markers'].items():
        if actual[marker] != count:
            raise ValueError('missing/duplicate mandatory marker: ' + marker)
    results = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    if [[int(n) for n in row] for row in results] != item['results']:
        raise ValueError('missing/skipped/duplicate/changed exact test results: ' + task)
    for pattern, count in item.get('patterns', {}).items():
        if len(re.findall(pattern, text, re.M)) != count:
            raise ValueError('missing mandatory naming/first-blocker evidence: ' + task)
    if re.search(r'^test result: (FAILED|ignored)|^test .* \.\.\. (FAILED|ignored)$', text, re.M):
        raise ValueError('failed or skipped test: ' + task)
    return dict(markers=item['markers'], results=item['results'])


def directory():
    job = os.environ['GITHUB_JOB']
    if job not in SHARDS:
        raise ValueError('unmapped shard')
    return Path(os.environ['RUNNER_TEMP']) / 'perf007' / job


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive creation prevents repeated task execution from overwriting proof.
    with path.open('xb') as stream:
        stream.write(encode(value))


def initialize():
    out = directory()
    out.mkdir(parents=True, exist_ok=False)
    sha = command('git', 'rev-parse', 'HEAD')
    if sha != os.environ['GITHUB_SHA']:
        raise ValueError('workflow source mismatch')
    write(out / 'identity.json', dict(sha=sha, tree=command('git', 'rev-parse', 'HEAD^{tree}'),
          shard=os.environ['GITHUB_JOB'], run_id=os.environ['GITHUB_RUN_ID'],
          attempt=os.environ['GITHUB_RUN_ATTEMPT'], started=time.time()))
    scratch = out.parent / (out.name + '-scratch')
    scratch.mkdir(exist_ok=False)
    with open(os.environ['GITHUB_ENV'], 'a') as env:
        env.write(f'TMPDIR={scratch}\nAMS_GRA_REQUIRE_GNAT=1\n')


def execute(kind, task, script):
    value = contract()
    if kind == 'task' and task not in SHARDS[os.environ['GITHUB_JOB']]:
        raise ValueError('task belongs to another shard')
    out = directory() / kind
    out.mkdir(exist_ok=True)
    body = Path(script).read_text().strip()
    if kind == 'task' and body != value['tasks'][task]['command']:
        raise ValueError('qualification command changed')
    started = time.time()
    with (out / (task + '.log')).open('xb') as log:
        process = subprocess.Popen(['bash', '--noprofile', '--norc', '-e', '-o', 'pipefail', script],
                                   cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        while True:
            chunk = process.stdout.read1(65536)
            if not chunk:
                break
            log.write(chunk)
            log.flush()
            sys.stdout.buffer.write(chunk)
            sys.stdout.buffer.flush()
        status = process.wait()
        process.stdout.close()
    ended = time.time()
    record = dict(task=task, command=body, exit_status=status, started=started, ended=ended,
                  elapsed_seconds=ended-started, complete=False)
    data = (out / (task + '.log')).read_bytes()
    record.update(log_sha256=digest(data), log_bytes=len(data))
    try:
        if status:
            raise SystemExit(status if status > 0 else 128-status)
        if kind == 'task':
            record.update(validate_output(task, data, value))
        record['complete'] = True
    finally:
        write(out / (task + '.json'), record)


def inputs():
    pins = {}
    for release, pin in PINS.items():
        root = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
        checkout = Path(os.environ['RUNNER_TEMP']) / ('uci-' + release)
        actual = dict(tag=pin['tag'], revision=command('git', '-C', str(checkout), 'rev-parse', pin['tag'] + '^{commit}'),
                      sha256=digest(root.read_bytes()))
        if actual != pin or command('git', '-C', str(checkout), 'rev-parse', 'HEAD') != pin['revision']:
            raise ValueError('pinned UCI identity/hash mismatch')
        pins[release] = actual
    tools = {name: command(*args) for name, args in {
        'rustc': ['rustc', '-Vv'], 'cargo': ['cargo', '-Vv'], 'rustup': ['rustup', 'show', 'active-toolchain'],
        'gnatmake': ['gnatmake', '--version'], 'g++': ['g++', '--version'],
        'python': ['python3', '--version'], 'git': ['git', '--version'],
    }.items()}
    sources = {p: digest((ROOT / p).read_bytes()) for p in json.loads(CONTRACT_PATH.read_bytes())['source_files']}
    sources['scripts/real-uci-contract.json'] = digest(CONTRACT_PATH.read_bytes())
    write(directory() / 'inputs.json', dict(pins=pins, toolchains=tools, source_hashes=sources))


def verify_bundle(files, shard, sha, tree, run_id, attempt, value=None):
    value = value or contract()
    completion = json.loads(files['completion.json'])
    if files['completion.sha256'].decode().strip() != digest(files['completion.json']):
        raise ValueError('completion hash mismatch')
    expected_files = set(completion['files']) | {'completion.json', 'completion.sha256'}
    if set(files) != expected_files:
        raise ValueError('missing/duplicate/unlisted evidence files')
    for name, entry in completion['files'].items():
        if dict(sha256=digest(files[name]), bytes=len(files[name])) != entry:
            raise ValueError('truncated/changed evidence: ' + name)
    identity = json.loads(files['identity.json'])
    for key, expected in dict(sha=sha, tree=tree, shard=shard, run_id=str(run_id), attempt=str(attempt)).items():
        if identity[key] != expected:
            raise ValueError('stale/incorrect evidence ' + key)
    if completion['status'] != 'success' or completion['tasks'] != SHARDS[shard]:
        raise ValueError('incomplete/incorrect shard task inventory')
    task_records = sorted(n for n in files if n.startswith('task/'))
    if task_records != sorted('task/' + t + ext for t in SHARDS[shard] for ext in ('.json', '.log')):
        raise ValueError('missing/duplicate task evidence')
    previous = identity['started']
    for task in SHARDS[shard]:
        record = json.loads(files['task/' + task + '.json'])
        log = files['task/' + task + '.log']
        observed = validate_output(task, log, value)
        if (record['task'] != task or record['command'] != value['tasks'][task]['command']
                or record['exit_status'] != 0 or record['complete'] is not True
                or record['log_sha256'] != digest(log) or record['log_bytes'] != len(log)
                or any(record[k] != v for k, v in observed.items())
                or not previous <= record['started'] < record['ended'] <= completion['ended']
                or abs(record['elapsed_seconds'] - (record['ended']-record['started'])) > 0.001):
            raise ValueError('failed/truncated/invalid task record: ' + task)
        previous = record['ended']
    if sorted(n for n in files if n.startswith('setup/')) != sorted(
            'setup/' + t + ext for t in ('gnat', 'fetch', 'build') for ext in ('.json', '.log')):
        raise ValueError('missing/duplicate setup evidence')
    for setup in ('gnat', 'fetch', 'build'):
        record = json.loads(files['setup/' + setup + '.json'])
        log = files['setup/' + setup + '.log']
        if (not record['complete'] or record['exit_status'] != 0 or record['log_sha256'] != digest(log)
                or record['log_bytes'] != len(log) or not 0 < record['elapsed_seconds']
                or record['task'] != setup
                or not identity['started'] <= record['started'] < record['ended'] <=
                    json.loads(files['task/' + SHARDS[shard][0] + '.json'])['started']
                or abs(record['elapsed_seconds']-(record['ended']-record['started'])) > 0.001):
            raise ValueError('failed/missing setup: ' + setup)
        required_commands = {'gnat': ['sudo apt-get update', 'sudo apt-get install -y gnat'],
                             'fetch': ['scripts/fetch-pinned-uci-2.5.sh', 'scripts/fetch-pinned-uci-2.6.sh'],
                             'build': ['cargo fetch --locked', 'cargo build --release --locked -p ams-gra-codegen-oms']}
        if not all(c in record['command'] for c in required_commands[setup]):
            raise ValueError('setup replaced by placeholder: ' + setup)
    prepared = json.loads(files['inputs.json'])
    if prepared['pins'] != PINS or not all(prepared['toolchains'].get(t) for t in
            ('rustc', 'cargo', 'rustup', 'gnatmake', 'g++', 'python', 'git')):
        raise ValueError('missing toolchain/pinned identity')
    expected = {p: digest((ROOT / p).read_bytes()) for p in value['source_files']}
    expected['scripts/real-uci-contract.json'] = digest(CONTRACT_PATH.read_bytes())
    if prepared['source_hashes'] != expected:
        raise ValueError('evidence/source hash mismatch')
    first = json.loads(files['task/' + SHARDS[shard][0] + '.json'])['started']
    if (abs(completion['setup_seconds'] - (first-identity['started'])) > 0.001
            or abs(completion['elapsed_seconds'] - (completion['ended']-identity['started'])) > 0.001):
        raise ValueError('invalid shard elapsed time')
    return completion


def finish():
    out = directory()
    value = contract()
    identity = json.loads((out / 'identity.json').read_bytes())
    ended = time.time()
    # Preserve generated diagnostics without uploading mutable Cargo targets or
    # executable build artifacts. Logs retain the complete compiler output.
    scratch = out.parent / (out.name + '-scratch')
    for parent, dirs, names in os.walk(scratch, followlinks=False):
        dirs[:] = [name for name in dirs if name not in ('target', 'cargo-target', '.git')
                   and not (Path(parent) / name).is_symlink()]
        for name in names:
            path = Path(parent) / name
            if (path.is_file() and not path.is_symlink() and
                    path.suffix in ('.log', '.json', '.tsv', '.sha256', '.lock', '.yaml', '.toml')):
                dest = out / 'diagnostics' / path.relative_to(scratch)
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, dest)
    records = {}
    for path in out.rglob('*'):
        if path.is_file():
            data = path.read_bytes()
            records[path.relative_to(out).as_posix()] = dict(sha256=digest(data), bytes=len(data))
    tasks = SHARDS[os.environ['GITHUB_JOB']]
    first = out / 'task' / (tasks[0] + '.json')
    complete = all((out / 'task' / (t + '.json')).is_file() and
                   json.loads((out / 'task' / (t + '.json')).read_bytes())['complete'] for t in tasks)
    completion = dict(status='success' if complete else 'failed', tasks=tasks, files=records, ended=ended,
                      setup_seconds=(json.loads(first.read_bytes())['started'] if first.exists() else ended)-identity['started'],
                      elapsed_seconds=ended-identity['started'])
    write(out / 'completion.json', completion)
    (out / 'completion.sha256').write_text(digest((out / 'completion.json').read_bytes()) + '\n')
    try:
        verify_bundle({p.relative_to(out).as_posix(): p.read_bytes() for p in out.rglob('*') if p.is_file()},
                      identity['shard'], identity['sha'], identity['tree'], identity['run_id'], identity['attempt'], value)
    except (ValueError, KeyError) as error:
        completion.update(status='failed', error=str(error))
        (out / 'completion.json').write_bytes(encode(completion))
        (out / 'completion.sha256').write_text(digest((out / 'completion.json').read_bytes()) + '\n')
        raise
    print('PERF007_SHARD_COMPLETE', json.dumps(completion, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['init', 'task', 'setup', 'inputs', 'finish', 'check'])
    parser.add_argument('rest', nargs='*')
    args = parser.parse_args()
    if args.action in ('task', 'setup'):
        execute(args.action, *args.rest)
    else:
        {'init': initialize, 'inputs': inputs, 'finish': finish, 'check': contract}[args.action]()


if __name__ == '__main__':
    main()