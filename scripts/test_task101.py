"""Task101 independently constructed verifier and selector rejection controls."""
import copy
import hashlib
import itertools
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import ci_selection
from ci_retry_test_data import metadata
from test_ci_select import change, gate, GateTests
import task101_evidence as evidence


def bundle(compiler='14'):
    """Synthetic verifier input ONLY; never real interoperability evidence."""
    files = {}
    def put(name, value):
        files[name] = (json.dumps(value) if not isinstance(value, str) else value).encode()
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    put('integration-identity.json', dict(sha='a'*40, tree='b'*40, run_id='123', run_attempt='2', compiler=compiler))
    put('campaign/source.stdout', 'a'*40 + '\n' + 'b'*40 + '\n')
    put('campaign/source-status.stdout', '')
    put('campaign/gnat-version.stdout', 'GNAT ' + version + '\n')
    put('campaign/generator.sha256', 'c'*64)
    inputs = ['scripts/task101/qualify.py', 'scripts/task101/sleet.py', 'scripts/task101/cases.py',
              'tests/ada-real-uci-dlz/dlz_consumer.adb', 'scripts/task101_evidence.py',
              'scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
              'tests/ada-time-zulu-codec/time_consumer.adb']
    inputs += [f'tests/fixtures/task101/uci{r}-{s}' for r in evidence.PINS for s in ('full-dlz.json', 'service.yaml')]
    put('campaign/input-sources.json', {path: hashlib.sha256((evidence.ROOT / path).read_bytes()).hexdigest() for path in inputs})
    expected = list(itertools.product(('25', '26'), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    labels = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv'}
    labels |= {f'{r}-{s}' for r in evidence.PINS for s in ('check', 'generate', 'original', 'model')}
    labels |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    labels |= {f'{r}-dlz-{p}-{s}' for r in evidence.PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    for label in labels:
        put('campaign/' + label + '.exit', '0\n')
    put('campaign/result.json', dict(completed=True, compiler=version, real_sleet=True, real_uci=True,
        cells=[dict(release=r, policy=p, optimization=o) for r, p, o in expected],
        steps=[dict(label=label, exit=0) for label in sorted(labels)]))
    put('qualify.log', ''.join(f'TASK101_CELL_COMPLETE {version} uci{r}-{p}-O{o}\n' for r,p,o in expected)
        + f'TASK101_QUALIFICATION_COMPLETE {version} 12\n')
    put('campaign/fixture.ads', 'synthetic source')
    put('campaign/sources.json', {'fixture.ads': hashlib.sha256(files['campaign/fixture.ads']).hexdigest()})
    for release in evidence.PINS:
        # Actual pinned source bytes are optional external input in unit tests;
        # the hash operation for these two roots is isolated in this fixture.
        put(f'campaign/uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd', 'synthetic-root-' + release)
        put(f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd', 'synthetic-include-' + release)
        put(f'campaign/{release}-check.stdout', 'selected type closure: 54\ncodec renderable emitted declarations: 53/53\ncodec status: READY\n')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb', 'service_api.ads'):
            for variant in ('generated', 'original-generated', 'model'):
                if variant == 'model' and name.startswith('service_codec'):
                    continue
                put(f'campaign/{release}-{variant}/{name}', 'synthetic unchanged ' + name)
        for policy in ('Normal', 'Check', 'Ignore'):
            put(f'campaign/{release}-dlz-{policy}-execute.stdout', 'ACCEPT\t{}\n'*10 + 'REJECT\tinvalid named Time\n'*16
                + 'TASK098_CONSUMER_COMPLETE count= 26 accepted= 10 rejected= 16\n')
    for release, policy, optimization in expected:
        prefix = f'campaign/uci{release}-{policy}-O{optimization}/sleet/'
        put(prefix + 'result.json', dict(completed=True, release=release, revision=evidence.REV, tree=evidence.TREE,
            handlers=11, witness_handlers=1, decode_rejected=1, published=10, broker_rejected=10,
            consumer_exit=0, server_exit=0, cleanup=True))
        put(prefix + 'sleet-source.txt', evidence.REV + '\n' + evidence.TREE + '\n')
        put(prefix + 'sleet-binary.sha256', 'd'*64)
        for phase in ('consumer', 'server'):
            put(prefix + phase + '.exit', '0\n')
        put(prefix + 'consumer.stdout', '\n'.join([
            'TASK101_ADA_REAL_DLZ_COMPLETE handlers= 11 rejected= 1',
            'TASK101_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1', 'TASK101_CLEANUP_COMPLETE',
            'TASK101_DECODE_RECOVERY_COMPLETE', 'TASK101_UNKNOWN_BINDING_SID_REJECTED',
            'TASK101_UPGRADE_OWP_COMPLETE', 'TASK101_INFO_VALIDATED', 'TASK101_SUB_ACCEPTED DLZ task098-dlz',
            'TASK101_EVENT DECODE_ERROR invalid named Time',
            *['TASK101_TYPED_HANDLER ' + str(n) for n in range(11)], 'TASK101_WITNESS_HANDLER 1',
            *['TASK101_PUBLISHED {}']*10]) + '\n')
        put(prefix + 'wire.json', [dict(label=label, response='-ERR ' + code + ' ' + detail)
                                 for label, (code, detail) in evidence.BROKER_ERRORS.items()]
            + [dict(label=label, response='+OK') for label in ('broker-accepted-codec-invalid-time', 'duplicate-valid-last-recovery')]
            + [dict(label='wrong-service-closed')])
        put(prefix + 'server.stdout', 'schema loaded\nroute PUB message\nshutdown complete\n')
    return files


class EvidenceTests(unittest.TestCase):
    def test_per_tool_identities_under_posix_sh(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('task101_qualification_test',
            evidence.ROOT / 'scripts/task101/qualify.py')
        qualify = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(qualify)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            for tool in ('gcc', 'gnat', 'gprbuild', 'ld'):
                path = directory / tool
                path.write_text('#!/bin/sh\nprintf "linker-path\\n"\n')
                path.chmod(0o755)
            from unittest.mock import patch
            import os
            with patch.dict(os.environ, PATH=str(directory) + ':' + os.environ['PATH']):
                output = subprocess.check_output(['sh', '-ec', qualify.TOOL_IDENTITIES], text=True)
            for tool in ('gcc', 'gnat', 'gprbuild', 'ld'):
                path = directory / tool
                self.assertIn(str(path) + '\n', output)
                self.assertIn(hashlib.sha256(path.read_bytes()).hexdigest() + '  ' + str(path), output)

    def verify(self, files, compiler='14'):
        # Only root bytes are synthetic. All SHA checks still execute, including
        # a wrong-root mutation; production never substitutes these pins.
        from unittest.mock import patch
        pins = {r: hashlib.sha256(('synthetic-root-' + r).encode()).hexdigest() for r in evidence.PINS}
        includes = {r: hashlib.sha256(('synthetic-include-' + r).encode()).hexdigest() for r in evidence.PINS}
        with patch.object(evidence, 'PINS', pins), patch.object(evidence, 'INCLUDES', includes):
            return evidence.verify_bundle(files, compiler, 'a'*40, 'b'*40, '123', '2')

    def test_both_compilers_complete_and_each_file_missing_or_duplicated(self):
        for compiler in ('14', '16'):
            files = bundle(compiler)
            self.assertEqual(self.verify(files, compiler)['cells'], 12)
        files = bundle()
        for path in files:
            with self.subTest(missing=path):
                mutated = dict(files)
                del mutated[path]
                with self.assertRaises((ValueError, KeyError)):
                    self.verify(mutated)
            with self.subTest(duplicate=path):
                mutated = dict(files)
                mutated['duplicate/' + path] = mutated[path]
                with self.assertRaises((ValueError, KeyError)):
                    self.verify(mutated)

    def test_identity_matrix_exits_markers_schema_compiler_and_sources(self):
        files = bundle()
        mutations = [
            ('campaign/source.stdout', b'wrong source\n'), ('campaign/source-status.stdout', b'M dirty'),
            ('campaign/gnat-version.stdout', b'GNAT 16.1.0\n'), ('campaign/generator.sha256', b''),
            ('campaign/uci25/UCI_MessageDefinitions_v2_5_0.xsd', b'wrong authentic release'),
            ('campaign/fixture.ads', b'stale source'), ('qualify.log', b''),
            ('campaign/uci25-Normal-O0-build.exit', b'1'),
            ('campaign/uci25-Normal-O0/sleet/consumer.stdout', b'crashed'),
            ('campaign/uci25-Normal-O0/sleet/wire.json', b'[]'),
            ('campaign/uci25-Normal-O0/sleet/server.exit', b'137'),
        ]
        for path, value in mutations:
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: value}))
        for field in ('sha', 'tree', 'run_id', 'run_attempt', 'compiler'):
            mutated = dict(files)
            identity = json.loads(mutated['integration-identity.json'])
            identity[field] = 'wrong'
            mutated['integration-identity.json'] = json.dumps(identity).encode()
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.verify(mutated)
        for mutation in ('missing', 'duplicate', 'release', 'skipped-step', 'failed-step'):
            mutated = dict(files)
            result = json.loads(mutated['campaign/result.json'])
            if mutation == 'missing': result['cells'].pop()
            elif mutation == 'duplicate': result['cells'][1] = result['cells'][0]
            elif mutation == 'release': result['cells'][0]['release'] = '27'
            elif mutation == 'skipped-step': result['steps'].pop()
            else: result['steps'][0]['exit'] = 1
            mutated['campaign/result.json'] = json.dumps(result).encode()
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.verify(mutated)


class SelectionAndAggregate(GateTests):
    def test_task101_surfaces_select_both_families_and_deep(self):
        policy = ci_selection.inventory()
        for path in ('scripts/task101/qualify.py', 'scripts/task101/sleet.py', 'scripts/task101_evidence.py',
                     'tests/ada-real-uci-dlz/dlz_consumer.adb', 'tests/fixtures/task101/uci25-full-dlz.json'):
            result = ci_selection.select([change(path)], policy)
            self.assertIn(evidence.JOB, result['deep_jobs'])
            self.assertTrue({'ada-websocket-transport-14', 'ada-websocket-transport-16'} <= set(result['cells']))

    def test_missing_duplicate_expired_stale_and_skipped_cells(self):
        self.validate()
        for compiler in ('14', '16'):
            name = 'task101-ada-real-uci-dlz-' + compiler
            item = next(a for a in self.artifacts if a['name'] == name)
            for field, value in [('expired', True), ('created_at', '2000-01-01T00:00:00Z'), ('size_in_bytes', 0)]:
                old = item[field]
                item[field] = value
                with self.subTest(field=field), self.assertRaises(ValueError): self.validate()
                item[field] = old
            self.artifacts.append(copy.deepcopy(item))
            with self.assertRaises(ValueError): self.validate()
            self.artifacts.pop()
            self.artifacts.remove(item)
            with self.assertRaises(ValueError): self.validate()
            self.artifacts.append(item)
            job = next(j for j in self.jobs if j['name'] == f'deep-qualification / {evidence.JOB} ({compiler})')
            job['steps'][0]['conclusion'] = 'skipped'
            with self.assertRaises(ValueError): self.validate()
            job['steps'][0]['conclusion'] = 'success'

    def test_artifact_producer_provenance_and_inner_verifier_are_required(self):
        from unittest.mock import patch
        artifacts = [dict(name='task101-ada-real-uci-dlz-' + c) for c in ('14', '16')]
        jobs, attempts = metadata(artifacts)
        manifest = dict(deep_jobs=[evidence.JOB])
        with patch.object(gate, 'verify_task101_bundle', return_value=dict(cells=12)) as verify:
            result = gate.reconcile_task101(artifacts, manifest, 'a'*40, 'b'*40, '123', '2',
                                           lambda item: {}, jobs=jobs, attempts=attempts)
            self.assertEqual(len(result), 2)
            self.assertEqual(verify.call_count, 2)
        for mutation in ('stale', 'expired', 'mismatched-head', 'skipped-step'):
            changed, history, current = copy.deepcopy(artifacts), copy.deepcopy(attempts), copy.deepcopy(jobs)
            if mutation == 'stale': changed[0]['created_at'] = '2000-01-01T00:00:00Z'
            elif mutation == 'expired': changed[0]['expired'] = True
            elif mutation == 'mismatched-head': changed[0]['workflow_run']['head_sha'] = '0'*40
            else: current[0]['steps'][0]['conclusion'] = 'skipped'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                gate.reconcile_task101(changed, manifest, 'a'*40, 'b'*40, '123', '2',
                                      lambda item: {}, jobs=current, attempts=history)
        with patch.object(gate, 'verify_task101_bundle', side_effect=ValueError('invalid source-bound bundle')), self.assertRaises(ValueError):
            gate.reconcile_task101(artifacts, manifest, 'a'*40, 'b'*40, '123', '2',
                                  lambda item: {}, jobs=jobs, attempts=attempts)


if __name__ == '__main__':
    unittest.main()