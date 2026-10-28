"""Synthetic evidence is ONLY verifier test data, never qualification evidence."""
import copy
import importlib.util
import json
import io
import zipfile
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import ci_policy
import real_uci_evidence as evidence
from test_ci_select import GateTests, gate


def bundle(shard):
    value = evidence.contract()
    files = {}
    files['identity.json'] = evidence.encode(dict(sha='a'*40, tree='b'*40, shard=shard,
        run_id='123', attempt='1', started=1.0))
    sources = {p: evidence.digest((evidence.ROOT / p).read_bytes()) for p in value['source_files']}
    sources['scripts/real-uci-contract.json'] = evidence.digest(evidence.CONTRACT_PATH.read_bytes())
    files['inputs.json'] = evidence.encode(dict(pins=evidence.PINS, source_hashes=sources,
        toolchains={t: 'fixture verifier test, NOT hosted qualification' for t in
                    ('rustc', 'cargo', 'rustup', 'gnatmake', 'g++', 'python', 'git')}))
    for n, name in enumerate(('gnat', 'fetch', 'build')):
        commands = {'gnat': 'sudo apt-get update\nsudo apt-get install -y gnat',
                    'fetch': 'scripts/fetch-pinned-uci-2.5.sh\nscripts/fetch-pinned-uci-2.6.sh',
                    'build': 'cargo fetch --locked\ncargo build --release --locked -p ams-gra-codegen-oms'}
        log = b'verifier fixture\n'
        files['setup/'+name+'.log'] = log
        files['setup/'+name+'.json'] = evidence.encode(dict(complete=True, exit_status=0,
            log_sha256=evidence.digest(log), log_bytes=len(log), elapsed_seconds=1.0,
            started=2.0+n, ended=3.0+n, command=commands[name], task=name))
    for n, task in enumerate(evidence.SHARDS[shard]):
        item = value['tasks'][task]
        lines = [marker for marker, count in item['markers'].items() for _ in range(count)]
        for row in item['results']:
            lines.append('test result: ok. %d passed; %d failed; %d ignored; %d measured; %d filtered out;' % tuple(row))
        if task == '065':
            for release in ('2.5', '2.6'):
                lines.extend(f'FORMER-GAP-PROJECTION\t{release}\t{i}' for i in range(33))
                lines.extend(f'GAP-COUNT\t{release}\t{i}\t0' for i in range(2))
        log = ('\n'.join(lines)+'\n').encode()
        files['task/'+task+'.log'] = log
        files['task/'+task+'.json'] = evidence.encode(dict(task=task, command=item['command'],
            complete=True, exit_status=0, started=10.0+n*2, ended=11.0+n*2, elapsed_seconds=1.0,
            log_sha256=evidence.digest(log), log_bytes=len(log), **evidence.validate_output(task, log, value)))
    files['completion.json'] = evidence.encode(dict(status='success', tasks=evidence.SHARDS[shard],
        files={}, ended=30.0, elapsed_seconds=29.0, setup_seconds=9.0))
    seal(files)
    return files


def seal(files):
    completion = json.loads(files['completion.json'])
    completion['files'] = {name: dict(sha256=evidence.digest(data), bytes=len(data))
                           for name, data in files.items() if not name.startswith('completion.')}
    files['completion.json'] = evidence.encode(completion)
    files['completion.sha256'] = (evidence.digest(files['completion.json'])+'\n').encode()


def change_json(files, path, **values):
    data = json.loads(files[path])
    data.update(values)
    files[path] = evidence.encode(data)
    seal(files)


class RuntimeEvidence(unittest.TestCase):
    def verify(self, files, shard):
        return evidence.verify_bundle(files, shard, 'a'*40, 'b'*40, '123', '1')

    def test_all_shards_and_exact_union(self):
        observed = []
        for shard in evidence.SHARDS:
            observed.extend(self.verify(bundle(shard), shard)['tasks'])
        self.assertCountEqual(observed, evidence.ORIGINAL)
        artifacts = [dict(name='perf007-'+s, id=s, expired=False, size_in_bytes=1) for s in evidence.SHARDS]
        result = gate.reconcile_primary(artifacts, 'a'*40, 'b'*40, '123', '1', lambda a: bundle(a['id']))
        self.assertEqual(set(result), set(evidence.SHARDS))
        for missing in artifacts:
            with self.assertRaises(ValueError):
                gate.reconcile_primary([a for a in artifacts if a != missing], 'a'*40, 'b'*40, '123', '1', lambda a: bundle(a['id']))

    def test_missing_duplicate_failed_skipped_truncated_stale_and_hash_mutations(self):
        for shard, tasks in evidence.SHARDS.items():
            healthy = bundle(shard)
            mutations = []
            for path in ('task/'+tasks[0]+'.json', 'task/'+tasks[0]+'.log', 'inputs.json', 'completion.sha256'):
                files = copy.deepcopy(healthy)
                del files[path]
                mutations.append(('missing '+path, files))
            files = copy.deepcopy(healthy)
            files['task/999.json'] = files['task/'+tasks[0]+'.json']
            seal(files)
            mutations.append(('duplicate extra task', files))
            for key, val in (('complete', False), ('exit_status', 23), ('command', 'true'), ('task', '999')):
                files = copy.deepcopy(healthy)
                change_json(files, 'task/'+tasks[0]+'.json', **{key: val})
                mutations.append((key, files))
            files = copy.deepcopy(healthy)
            files['task/'+tasks[0]+'.log'] = b'test result: ok. 0 passed; 0 failed; 1 ignored;\n'
            seal(files)
            mutations.append(('skipped test', files))
            for key, val in (('sha', 'c'*40), ('tree', 'c'*40), ('run_id', '122'), ('attempt', '2')):
                files = copy.deepcopy(healthy)
                change_json(files, 'identity.json', **{key: val})
                mutations.append(('stale '+key, files))
            files = copy.deepcopy(healthy)
            data = json.loads(files['inputs.json'])
            data['pins']['2.6']['sha256'] = '0'*64
            files['inputs.json'] = evidence.encode(data)
            seal(files)
            mutations.append(('changed pinned hash', files))
            for task in tasks:
                for marker in evidence.contract()['tasks'][task]['markers']:
                    files = copy.deepcopy(healthy)
                    path = 'task/'+task+'.log'
                    files[path] = files[path].replace((marker+'\n').encode(), b'', 1)
                    seal(files)
                    mutations.append(('removed marker '+marker, files))
            for label, files in mutations:
                with self.subTest(shard=shard, mutation=label), self.assertRaises((ValueError, KeyError)):
                    self.verify(files, shard)

    def test_real_executor_nonzero_propagates_and_writes_diagnostics(self):
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp)/'command.sh'
            script.write_text('echo required-diagnostic; exit 23\n')
            out = Path(tmp)/'evidence'
            out.mkdir()
            with patch.object(evidence, 'directory', return_value=out):
                with self.assertRaises(SystemExit) as raised:
                    evidence.execute('setup', 'gnat', str(script))
                self.assertEqual(raised.exception.code, 23)
                self.assertIn('required-diagnostic', (out/'setup/gnat.log').read_text())
                self.assertEqual(json.loads((out/'setup/gnat.json').read_text())['exit_status'], 23)

    def test_archive_paths_and_duplicate_entries_fail_without_extraction(self):
        for names in (['../escape.json'], ['/absolute.json'], ['same.json', 'same.json']):
            stream = io.BytesIO()
            import warnings
            with warnings.catch_warnings():
                warnings.simplefilter('ignore', UserWarning)
                with zipfile.ZipFile(stream, 'w') as archive:
                    for name in names:
                        archive.writestr(name, b'{}')
            with patch.dict(gate.os.environ, GITHUB_REPOSITORY='fixture/repo', GH_TOKEN='fixture'), \
                    patch.object(gate.urllib.request, 'build_opener') as opener:
                opener.return_value.open.return_value = io.BytesIO(stream.getvalue())
                with self.assertRaises(ValueError):
                    gate.download_bundle(dict(id=1))

    def test_incomplete_finalization_writes_failed_completion(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'out'
            out.mkdir()
            shard = next(iter(evidence.SHARDS))
            (out/'identity.json').write_bytes(evidence.encode(dict(sha='a'*40, tree='b'*40,
                shard=shard, run_id='123', attempt='1', started=1.0)))
            with patch.object(evidence, 'directory', return_value=out), \
                    patch.dict(evidence.os.environ, GITHUB_JOB=shard):
                with self.assertRaises((ValueError, KeyError)):
                    evidence.finish()
            self.assertEqual(json.loads((out/'completion.json').read_bytes())['status'], 'failed')
            self.assertTrue((out/'completion.sha256').is_file())


class ShardJobs(GateTests):
    def test_every_actual_shard_and_evidence_is_required(self):
        for shard in evidence.SHARDS:
            index = next(n for n, j in enumerate(self.jobs) if j['name'] == 'deep-qualification / '+shard)
            original = copy.deepcopy(self.jobs[index])
            for conclusion in ('failure', 'skipped', 'cancelled'):
                self.jobs[index]['conclusion'] = conclusion
                with self.assertRaises(ValueError):
                    self.validate()
            self.jobs[index] = copy.deepcopy(original)
            self.jobs[index]['steps'] = [dict(name='successful placeholder', conclusion='success')]
            with self.assertRaises(ValueError):
                self.validate()
            self.jobs.pop(index)
            with self.assertRaises(ValueError):
                self.validate()
            self.jobs.insert(index, original)
            artifact = next(a for a in self.artifacts if a['name'] == 'perf007-'+shard)
            artifact['created_at'] = '2000-01-01T00:00:00Z'
            with self.assertRaises(ValueError):
                self.validate()
            artifact['created_at'] = '2026-10-27T02:00:00Z'


class StaticPartition(unittest.TestCase):
    def test_omission_duplication_missing_shard_and_placeholder(self):
        text = (evidence.ROOT/'.github/workflows/deep-ci.yml').read_text()
        jobs = ci_policy.blocks(text)[1]
        evidence.check_layout(jobs)
        for shard, tasks in evidence.SHARDS.items():
            for task in tasks:
                changed = copy.deepcopy(jobs)
                cmd = evidence.contract()['tasks'][task]['command']
                changed[shard] = changed[shard].replace('        run: '+cmd, '        run: true')
                with self.assertRaises(ValueError):
                    evidence.check_layout(changed)
                changed = copy.deepcopy(jobs)
                other = next(s for s in evidence.SHARDS if s != shard)
                changed[other] += '\n      - run: '+cmd+'\n'
                with self.assertRaises(ValueError):
                    evidence.check_layout(changed)
            changed = copy.deepcopy(jobs)
            del changed[shard]
            with self.assertRaises(ValueError):
                evidence.check_layout(changed)


if __name__ == '__main__':
    unittest.main()