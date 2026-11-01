"""Independent Task102 execution-verifier and qualification controls; no PR gates."""
import hashlib
import importlib.util
import itertools
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import task102_evidence as evidence
from workflow_text import blocks
spec = importlib.util.spec_from_file_location('task102_test_cases', evidence.ROOT / 'scripts/task102/cases.py')
cases = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cases)


def schema_fixture(release):
    return ('<xs:simpleType name="SHA_2_256_HashType"><xs:restriction base="xs:hexBinary">'
            '<xs:length value="32"/></xs:restriction></xs:simpleType>'
            + ('<xs:simpleType name="HexBinaryType"><xs:restriction base="xs:hexBinary"/></xs:simpleType>'
               if release == '26' else ''))


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
    inputs = ['scripts/task102/qualify.py', 'scripts/task102/sleet.py', 'scripts/task102/cases.py',
              'tests/ada-real-uci-log/log_consumer.adb', 'scripts/task102_evidence.py',
              'scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
              'tests/ada-generated-codec/hexbinary_log_consumer.adb', 'scripts/check-task093-log.py',
              'scripts/task102/admission.py', 'tests/fixtures/service-generate/real-duration-log.yaml',
              'scripts/task102/constrained.py', 'tests/ada-real-uci-log/hash_probe.adb',
              'tests/fixtures/service-generate/task093-real-log25.json',
              'tests/fixtures/service-generate/task093-real-log26.json']
    inputs += [f'tests/fixtures/task102/uci{r}-service.yaml' for r in evidence.PINS]
    put('campaign/input-sources.json', {path: hashlib.sha256((evidence.ROOT / path).read_bytes()).hexdigest() for path in inputs})
    expected = list(itertools.product(('25', '26'), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    labels = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission'}
    labels |= {f'{r}-{s}' for r in evidence.PINS for s in ('check', 'generate', 'original', 'model')}
    labels |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    labels |= {f'{r}-codec-{p}-{s}' for r in evidence.PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    labels |= {f'{r}-constrained-ada' for r in evidence.PINS}
    labels |= {f'{r}-hash-{p}-{s}' for r in evidence.PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    for label in labels:
        put('campaign/' + label + '.exit', '0\n')
    for release in evidence.PINS:
        for direction, forbidden in [('input', 'Publisher'), ('output', 'Subscriber')]:
            label = f'{release}-direction-{direction}'
            labels.add(label)
            put('campaign/' + label + '.exit', '1\n')
            put('campaign/' + label + '.stderr', f'"{forbidden}" not declared')
    put('campaign/result.json', dict(completed=True, compiler=version, real_sleet=True, real_uci=True,
        cells=[dict(release=r, policy=p, optimization=o) for r, p, o in expected],
        steps=[dict(label=label, exit=1 if '-direction-' in label else 0) for label in sorted(labels)]))
    put('qualify.log', ''.join(f'TASK102_CELL_COMPLETE {version} uci{r}-{p}-O{o}\n' for r,p,o in expected)
        + f'TASK102_QUALIFICATION_COMPLETE {version} 12\n')
    put('campaign/fixture.ads', 'synthetic source')
    put('campaign/sources.json', {'fixture.ads': hashlib.sha256(files['campaign/fixture.ads']).hexdigest()})
    for release in evidence.PINS:
        # Actual pinned source bytes are optional external input in unit tests;
        # the hash operation for these two roots is isolated in this fixture.
        put(f'campaign/uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd', schema_fixture(release))
        put(f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd', 'synthetic-include-' + release)
        prefix = f'campaign/admission/phase-a-{release}/'
        put(prefix + 'result.json', dict(release=release, admitted=True, delivered=True, cleanup=True,
            publication_response='+OK', server_exit=0, socket_closed=True, status='PASS'))
        put(prefix + 'sleet-source.txt', evidence.REV + '\n' + evidence.TREE + '\n')
        put(prefix + 'server.exit', '0\n')
        put(prefix + f'schema/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd', schema_fixture(release))
        put(prefix + f'schema/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd', 'synthetic-include-' + release)
        put(prefix + f'task093-real-log{release}.json', cases.golden(release))
        put(prefix + 'wire.json', [dict(receive='MSG 1 ' + json.dumps({'Log': cases.golden(release)}))])
        prefix = f'campaign/{release}-constrained-broker/'
        put(prefix + 'result.json', dict(scope='unchanged extracted authentic Binary declarations; synthetic envelope, NOT Log',
            ada_accepts_octets=32, broker_accepts_octets=16, broker_rejects_octets=[32, 31, 33], server_exit=0, cleanup=True))
        put(prefix + 'wire.json', [dict(label=f'hash-{n}-octets', response='+OK' if n == 16 else '-ERR Invalid-Message FacetViolation',
            command='PUB task102-constrained ' + json.dumps({'BinaryNotice': {'Data': '0080FF', 'SHA_2_Hash': '0080FF' + '00' * (n - 3)}})) for n in (32, 16, 31, 33)])
        put(prefix + 'server.exit', '0\n')
        put(prefix + 'server.stdout', 'shutdown complete\n')
        extracted_schema = '<xs:schema >' + schema_fixture(release) + '</xs:schema>'
        put(f'campaign/{release}-constrained-ada/real-binary.xsd', extracted_schema)
        put(prefix + 'extracted-binary.xsd', extracted_schema.replace('<xs:schema ', f'<xs:schema version="002.{release[-1]}.0" ', 1))
        for policy in ('Normal', 'Check', 'Ignore'):
            put(f'campaign/{release}-hash-{policy}-execute.stdout', 'TASK102_HASH_OCTETS_COMPLETE accepted=1 rejected=3\n')
        put(f'campaign/{release}-constrained-ada/qualification.json', dict(compiler=version, uci='2.' + release[-1],
            policies=['normal', 'Check', 'Ignore'], checks_per_cell=14, status='PASS', full_service_qualified=False))
        for policy in ('normal', 'Check', 'Ignore'):
            put(f'campaign/{release}-constrained-ada/{policy}-run.log', 'TASK093_REAL_COMPLETE cases= 14\n')
        put(f'campaign/{release}-check.stdout', f'selected type closure: {42 if release == "25" else 44}\ncodec renderable emitted declarations: {41 if release == "25" else 43}/{41 if release == "25" else 43}\ncodec status: READY\n')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb', 'service_api.ads'):
            for variant in ('generated', 'original-generated', 'model'):
                if variant == 'model' and name.startswith('service_codec'):
                    continue
                put(f'campaign/{release}-{variant}/{name}', 'synthetic unchanged ' + name)
        for policy in ('Normal', 'Check', 'Ignore'):
            put(f'campaign/{release}-codec-{policy}-execute.stdout', 'TASK093_LOG_COMPLETE checks= 29 rejections= 9\n')
    for release, policy, optimization in expected:
        prefix = f'campaign/uci{release}-{policy}-O{optimization}/sleet/'
        put(prefix + 'result.json', dict(completed=True, release=release, revision=evidence.REV, tree=evidence.TREE,
            handlers=7, witness_handlers=1, decode_rejected=3, published=6, broker_rejected=10,
            consumer_exit=0, server_exit=0, cleanup=True))
        put(prefix + 'sleet-source.txt', evidence.REV + '\n' + evidence.TREE + '\n')
        put(prefix + 'sleet-binary.sha256', 'd'*64)
        for phase in ('consumer', 'server'):
            put(prefix + phase + '.exit', '0\n')
        positive = cases.positives(release)
        received = [{'Log': doc} for _, doc in cases.decoder_invalid(release)]
        received += [{'Log': positive[0][1]}] + [{'Log': expected} for _, _, expected in positive] + [{'Log': positive[0][1]}]
        put(prefix + 'consumer.stdout', '\n'.join([
            'TASK102_ADA_REAL_LOG_COMPLETE handlers= 7 rejected= 3',
            'TASK102_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1', 'TASK102_CLEANUP_COMPLETE',
            'TASK102_DECODE_RECOVERY_COMPLETE', 'TASK102_UNKNOWN_BINDING_SID_REJECTED',
            'TASK102_UPGRADE_OWP_COMPLETE', 'TASK102_INFO_VALIDATED', 'TASK102_SUB_ACCEPTED Log task102-log',
            'TASK102_EVENT DECODE_ERROR odd hexBinary digit count',
            'TASK102_EVENT DECODE_ERROR invalid hexBinary digit',
            'TASK102_EVENT DECODE_ERROR odd hexBinary digit count',
            *['TASK102_TYPED_HANDLER ' + str(n) for n in range(7)], 'TASK102_WITNESS_HANDLER 1',
            *['TASK102_PUBLISHED ' + json.dumps(expected) for _, _, expected in positive],
            *['TASK102_WIRE_RECEIVED ' + json.dumps(doc) for doc in received]]) + '\n')
        put(prefix + 'wire.json', [dict(label=label, response='-ERR ' + code + ' ' + detail)
                                 for label, (code, detail) in evidence.BROKER_ERRORS.items()]
            + [dict(label=label, command='PUB task102-log ' + json.dumps({'Log': doc}), response='+OK') for label, doc in cases.decoder_invalid(release)]
            + [dict(label='same-subscription-recovery', response='+OK')]
            + [dict(label='broker-' + label, response='+OK') for label, _, _ in positive[-2:]]
            + [dict(label='broker-' + label + '-delivery', response='MSG 99 ' + json.dumps({'Log': doc})) for label, doc, _ in positive[-2:]]
            + [dict(label='wrong-service-closed')])
        put(prefix + 'server.stdout', 'schema loaded\nroute PUB message\nshutdown complete\n')
    return files


class GitTrustBoundary(unittest.TestCase):
    def check_boundary(self, actual_owner=False):
        def load(name, path):
            spec = importlib.util.spec_from_file_location(name, evidence.ROOT / path)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module
        qualify = load('task102_git_trust', 'scripts/task102/qualify.py')
        shared = load('task102_git_campaign', 'scripts/task098/qualify.py')
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            repo = base / 'checkout'
            home = base / 'setup/home'
            home.mkdir(parents=True)
            # No inherited global/system trust, SUDO_UID, or Git command config.
            env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_') and k != 'SUDO_UID'}
            env.update(HOME=str(home), XDG_CONFIG_HOME=str(home / '.config'), GIT_CONFIG_NOSYSTEM='1')
            def git(*args):
                return subprocess.run(['git', '-C', str(repo), *args], env=env,
                                      text=True, capture_output=True, check=True).stdout
            subprocess.run(['git', 'init', '--quiet', str(repo)], env=env, check=True)
            (repo / 'tracked').write_text('original\n')
            git('add', 'tracked')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                'commit', '--quiet', '-m', 'fixture')
            expected = git('rev-parse', 'HEAD', 'HEAD^{tree}').splitlines()
            if actual_owner:
                subprocess.run(['podman', 'unshare', 'chown', '-R', '1', str(repo)], check=True)
                self.assertNotEqual(repo.stat().st_uid, os.getuid())
            else:
                # Portable exercise of Git's real ownership validator. The opt-in
                # test below additionally changes the disposable checkout's UID.
                env['GIT_TEST_ASSUME_DIFFERENT_OWNER'] = '1'
            try:
                with patch.dict(os.environ, env, clear=True), patch.object(qualify, 'ROOT', repo):
                    def campaign(label):
                        directory = base / label
                        directory.mkdir()
                        return shared.Campaign(directory, base / 'setup', '14')
                    for label, command in (
                            ('untrusted', ['git', 'rev-parse', 'HEAD', 'HEAD^{tree}']),
                            ('wrong-path', ['git', '-c', f'safe.directory={base / "other"}',
                                            'rev-parse', 'HEAD', 'HEAD^{tree}'])):
                        failed = campaign(label)
                        with self.assertRaisesRegex(RuntimeError, 'nonzero exit'):
                            failed.run('source', command, repo)
                        self.assertEqual((failed.scratch / 'source.exit').read_text(), '128\n')
                        self.assertIn('detected dubious ownership', (failed.scratch / 'source.stderr').read_text())
                        self.assertEqual(failed.steps, [dict(label='source', exit=128)])
                        self.assertFalse((failed.scratch / 'result.json').exists())
                    clean = campaign('clean')
                    qualify.record_source(clean)
                    self.assertEqual((clean.scratch / 'source.stdout').read_text().splitlines(), expected)
                    self.assertEqual((clean.scratch / 'source-status.stdout').read_text(), '')
                    for label in ('source', 'source-status'):
                        self.assertEqual((clean.scratch / f'{label}.exit').read_text(), '0\n')
                        argv = json.loads((clean.scratch / f'{label}.command.json').read_text())['argv']
                        self.assertEqual(argv[:3], ['git', '-c', f'safe.directory={repo.resolve()}'])
                        self.assertNotIn('safe.directory=*', argv)
                    (repo / 'untracked').write_text('dirty\n')
                    dirty = campaign('dirty')
                    with self.assertRaisesRegex(RuntimeError, 'dirty source checkout'):
                        qualify.record_source(dirty)
                    self.assertIn('?? untracked', (dirty.scratch / 'source-status.stdout').read_text())
                    self.assertFalse((dirty.scratch / 'result.json').exists())
                    self.assertEqual(clean.env['HOME'], str(home))
                    self.assertFalse((home / '.gitconfig').exists())
            finally:
                if actual_owner:
                    subprocess.run(['podman', 'unshare', 'chown', '-R', '0', str(repo)], check=True)

    def test_git_trust_boundary_portable(self):
        self.check_boundary()

    @unittest.skipUnless(os.environ.get('TASK102_OWNERSHIP_FIXTURE') == 'podman',
                         'opt-in disposable UID fixture: TASK102_OWNERSHIP_FIXTURE=podman')
    def test_git_trust_boundary_different_owner(self):
        self.check_boundary(actual_owner=True)


class EvidenceTests(unittest.TestCase):
    def test_per_tool_identities_under_posix_sh(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('task102_qualification_test',
            evidence.ROOT / 'scripts/task102/qualify.py')
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
        pins = {r: hashlib.sha256(schema_fixture(r).encode()).hexdigest() for r in evidence.PINS}
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
            ('campaign/source.exit', b'128\n'), ('campaign/source-status.exit', b'128\n'),
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

    def test_publication_wire_identity_and_rejection_stages_are_independent(self):
        files = bundle()
        prefix = 'campaign/uci25-Normal-O0/sleet/'
        original = files[prefix + 'consumer.stdout'].decode()
        for before, after in [
                ('"Data": "0080FF"', '"Data": "0080FE"'),
                ('TASK102_WIRE_RECEIVED {"Log":', 'TASK102_WIRE_RECEIVED {"LogMT":'),
                ('TASK102_PUBLISHED ', 'NOT_A_PUBLICATION '),
                ('DECODE_ERROR invalid hexBinary digit', 'SERVER_ERROR invalid hexBinary digit'),
                ('TASK102_CLEANUP_COMPLETE', 'CLEANUP_MISSING'),
                ('primary_unchanged witness=1', 'primary_changed witness=1')]:
            self.assertIn(before, original)
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.verify(dict(files, **{prefix + 'consumer.stdout': original.replace(before, after).encode()}))
        for label in ('odd-hex', 'invalid-digit', 'internal-space', 'same-subscription-recovery',
                      'broker-lowercase', 'broker-xml-whitespace'):
            wire = json.loads(files[prefix + 'wire.json'])
            next(item for item in wire if item.get('label') == label)['response'] = '-ERR Invalid-Message'
            with self.subTest(label=label), self.assertRaises(ValueError):
                self.verify(dict(files, **{prefix + 'wire.json': json.dumps(wire).encode()}))

    def test_constrained_and_admission_probes_cannot_replace_full_log(self):
        files = bundle()
        paths = [
            'campaign/admission/phase-a-25/result.json',
            'campaign/25-constrained-broker/result.json',
            'campaign/25-constrained-ada/qualification.json',
        ]
        for path in paths:
            value = json.loads(files[path])
            value['cleanup' if 'cleanup' in value else 'status'] = False
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: json.dumps(value).encode()}))
        wire = json.loads(files['campaign/25-constrained-broker/wire.json'])
        next(item for item in wire if item['label'] == 'hash-32-octets')['response'] = '+OK'
        with self.assertRaises(ValueError):
            self.verify(dict(files, **{'campaign/25-constrained-broker/wire.json': json.dumps(wire).encode()}))
        for path in ('campaign/25-constrained-broker/extracted-binary.xsd',
                     'campaign/25-constrained-ada/real-binary.xsd'):
            modified = files[path].replace(b'value="32"', b'value="16"')
            self.assertNotEqual(modified, files[path])
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: modified}))

    def test_hashed_hidden_provider_sources_cannot_be_omitted(self):
        files = bundle()
        hidden = 'campaign/provider/gnatcoll/.github/workflows/main.yaml'
        files[hidden] = b'name: provider fixture\n'
        sources = json.loads(files['campaign/sources.json'])
        sources[hidden.removeprefix('campaign/')] = hashlib.sha256(files[hidden]).hexdigest()
        files['campaign/sources.json'] = json.dumps(sources).encode()
        EvidenceTests().verify(files)
        del files[hidden]
        with self.assertRaisesRegex(ValueError, 'missing/duplicate Task102 evidence'):
            EvidenceTests().verify(files)


class DeepQualification(unittest.TestCase):
    def test_main_only_full_compiler_qualification(self):
        header, jobs = blocks((evidence.ROOT / '.github/workflows/deep-ci.yml').read_text())
        job = jobs[evidence.JOB]
        self.assertIn("    if: github.ref == 'refs/heads/main'\n", job)
        self.assertIn("group: ${{ format('oms-main-slot-{0}', matrix.slot) }}", job)
        self.assertIn('      queue: max\n', job)
        self.assertIn("        compiler: ['14', '16']\n", job)
        self.assertIn("          - compiler: '14'\n            slot: 5\n", job)
        self.assertIn("          - compiler: '16'\n            slot: 2\n", job)
        for forbidden in ('pull_request', 'inputs.selection', 'oms-pr-', 'continue-on-error'):
            self.assertNotIn(forbidden, header + job)
        for required in ('e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b',
                         'scripts/fetch-pinned-uci-2.5.sh', 'scripts/fetch-pinned-uci-2.6.sh',
                         'scripts/task102/qualify.py', 'from task102_evidence import verify_bundle',
                         "print('TASK102_HOSTED_VERIFIED', verify_bundle(",
                         'include-hidden-files: true'):
            self.assertIn(required, job)
        # Only the diagnostic upload is unconditional; execution/verification must succeed.
        self.assertEqual(job.count('        if:'), 1)
        self.assertIn('name: Upload Task102 qualification diagnostics\n        if: always()', job)


if __name__ == '__main__':
    unittest.main()
