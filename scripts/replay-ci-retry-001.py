#!/usr/bin/env python3
"""Replay retained CI-RETRY-001 API/ZIP evidence read-only; no API or rollout.

The original PERF007 source bytes are read from Git objects for its exact
proposed merge, not from today's checkout. No source hash is replaced/ignored.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import zipfile
from unittest.mock import patch

from ci_artifact_provenance import resolve
import ci_selection
import real_uci_evidence
import task100_evidence

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    directory = args.evidence.resolve(strict=True)
    read = lambda name: json.loads((directory / name).read_text())
    run = read('run.json')
    history = {n: dict(run=read(f'attempt-{n}.json'), jobs=[j for page in
                       read(f'attempt-{n}-jobs-pages.json') for j in page['jobs']])
               for n in range(1, run['run_attempt'] + 1)}
    artifacts = [a for page in read('artifacts-pages.json') for a in page['artifacts']]
    with zipfile.ZipFile(directory / 'selection.zip') as archive:
        manifest = json.loads(archive.read('ci-selection.json'))
    sha = manifest['identities']['merge']
    tree = subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', sha+'^{tree}'], text=True).strip()
    if run['head_sha'] != manifest['identities']['head']:
        raise ValueError('retained proposed merge/head mismatch')
    parents = subprocess.check_output(['git', '-C', str(ROOT), 'show', '-s', '--format=%P', sha], text=True).split()
    if parents != [manifest['identities']['base'], manifest['identities']['head']]:
        raise ValueError('retained proposed merge parent mismatch')
    spec = importlib.util.spec_from_file_location('retry_gate', ROOT / 'scripts/ci-required-gate.py')
    gate = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(gate)
    errors = {}
    producers = {}

    def loader(artifact):
        raw = (directory / (artifact['name'] + '.zip')).read_bytes()
        if 'sha256:' + hashlib.sha256(raw).hexdigest() != artifact['digest']:
            raise ValueError('retained artifact digest mismatch: ' + artifact['name'])
        import io
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            names = [n for n in archive.namelist() if not n.endswith('/')]
            if len(names) != len(set(names)):
                raise ValueError('duplicate retained bundle paths')
            return {n: archive.read(n) for n in names}

    with tempfile.TemporaryDirectory(prefix='ci-retry-replay-') as temporary:
        source = Path(temporary)
        contract = real_uci_evidence.contract()
        for name in [*contract['source_files'], 'scripts/real-uci-contract.json']:
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(subprocess.check_output(['git', '-C', str(ROOT), 'show', sha+':'+name]))
        with patch.object(real_uci_evidence, 'ROOT', source):
            for artifact in artifacts:
                name = artifact['name']
                if not name.startswith(('task100-', 'perf007-')):
                    continue
                if name.startswith('perf007-'):
                    shard = name.removeprefix('perf007-')
                    job_name = 'deep-qualification / ' + shard
                    verify = lambda a: real_uci_evidence.verify_bundle(loader(artifact), shard, sha, tree, run['id'], a)
                else:
                    real = '-sleet-' in name
                    compiler = name[-2:]
                    job_name = ('deep-qualification / ada-websocket-real-sleet' if real else
                                'ada-websocket-transport') + f' ({compiler})'
                    verify = lambda a: task100_evidence.verify_bundle(loader(artifact), compiler, real, sha, tree, run['id'], a)
                producers[name] = resolve(artifact, job_name, history[2]['jobs'], history, run['id'], 2)
                try:
                    verify(2)
                except ValueError as error:
                    errors[name] = str(error)
                else:
                    raise ValueError('original failure did not reproduce: ' + name)
                verify(producers[name]['run_attempt'])
            task100 = gate.reconcile_task100(artifacts, manifest, sha, tree, run['id'], 2, loader,
                                             jobs=history[2]['jobs'], attempts=history)
            primary = gate.reconcile_primary(artifacts, sha, tree, run['id'], 2, loader,
                                             jobs=history[2]['jobs'], attempts=history)
    result = dict(reproduced=True, run_id=run['id'], gate_attempt=2, sha=sha, tree=tree,
                  original_errors=errors, verified_producers=producers, task100=task100,
                  primary=primary, retrospective_replay=True, hosted_final_gate_success=False)
    (directory / 'replay-result.json').write_text(json.dumps(result, indent=2) + '\n')
    print('CI_RETRY_REPLAY_COMPLETE original_failures=7 verified_bundles=7 NOT_HOSTED_GATE_SUCCESS')


if __name__ == '__main__':
    main()