"""Independent Task104 execution-verifier and Git trust controls; no PR gates."""
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

import task104_evidence as evidence
from workflow_text import blocks
spec = importlib.util.spec_from_file_location('task104_test_cases', evidence.ROOT / 'scripts/task104/cases.py')
cases = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cases)


def schema_fixture(release):
    return 'synthetic authentic-root-' + release


def bundle(compiler='14'):
    """Synthetic verifier input ONLY; never real interoperability evidence."""
    files = {}
    def put(name, value):
        files[name] = (json.dumps(value) if not isinstance(value, str) else value).encode()
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    put('integration-identity.json', dict(sha='a'*40, tree='b'*40, run_id='123', run_attempt='2', compiler=compiler))
    put('uci26-inventory.json', dict(release='2.6', global_element='SurvivabilityRiskLevel',
        payload_type='uci:SurvivabilityRiskLevelMT', exit=0, compiler_qualified=False,
        transport_qualified=False, schema_sources={
            'UCI_MessageDefinitions_v2_6_0.xsd': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
            'UCI_SecurityMarkings_v2_6_0.xsd': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'},
        stdout='selected type closure: 45\ncodec renderable emitted declarations: 44/44\ncodec status: READY\n'))
    put('campaign/source.stdout', 'a'*40 + '\n' + 'b'*40 + '\n')
    put('campaign/source-status.stdout', '')
    put('campaign/gnat-version.stdout', 'GNAT ' + version + '\n')
    put('campaign/gprbuild-version.stdout', 'GPRBUILD Pro 26.0.0\n')
    put('campaign/generator.sha256', 'c'*64)
    guard_spec = importlib.util.spec_from_file_location('task104_test_lock', evidence.ROOT / 'scripts/check-task087a.py')
    guard = importlib.util.module_from_spec(guard_spec)
    guard_spec.loader.exec_module(guard)
    put('campaign/toolchain-lock.json', {'solution': {'state': [
        {'release': {'name': name, 'version': v, 'origin': {'hashes': [guard.PROVIDER_HASH,
            'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b',
            'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703']}}}
        for name, v in [('gnat_external' if compiler == '14' else 'gnat_native', version),
                        ('gnatcoll', '26.0.0'), ('gnatcoll_minimal', '26.0.0'), ('gprbuild', '26.0.1')]]}})
    inputs = ['scripts/task104/qualify.py', 'scripts/task104/sleet.py', 'scripts/task104/cases.py',
              'tests/ada-real-uci-survivability/survivability_consumer.adb', 'scripts/task104_evidence.py',
              'tests/ada-real-uci-survivability/codec_probe.adb', 'scripts/check-task087a.py',
              'scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
              'tests/ada-generated-codec/task092_real_survivability.adb', 'scripts/check-task092-real.py',
              'scripts/task104/admission.py', 'tests/fixtures/service-generate/codec-ada-minimal.yaml',
              'tests/fixtures/service-generate/task092-real-survivability25.json',
              ]
    inputs += [f'tests/fixtures/task104/uci{r}-service.yaml' for r in evidence.PINS]
    put('campaign/input-sources.json', {path: hashlib.sha256((evidence.ROOT / path).read_bytes()).hexdigest() for path in inputs})
    expected = list(itertools.product(('25',), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    labels = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission'}
    labels |= {f'{name}-{step}' for name in ('aht', 'websocket-ada')
               for step in ('clone', 'pin', 'identity', 'clean')}
    for name, revision in [('aht', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
                           ('websocket-ada', '765d9cd612838eb00645037977c8cc9af7171c3e')]:
        put(f'campaign/{name}-identity.stdout', revision + '\n')
        put(f'campaign/{name}-clean.stdout', '')
    labels |= {f'{r}-{s}' for r in evidence.PINS for s in ('check', 'generate', 'original', 'model')}
    labels |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    labels |= {f'{r}-codec-{p}-{s}' for r in evidence.PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    labels |= {f'{r}-independent-{p}-{s}' for r in evidence.PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
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
    put('qualify.log', ''.join(f'TASK104_CELL_COMPLETE {version} uci{r}-{p}-O{o}\n' for r,p,o in expected)
        + f'TASK104_QUALIFICATION_COMPLETE {version} 6\n')
    put('campaign/fixture.ads', 'synthetic source')
    put('campaign/sources.json', {'fixture.ads': hashlib.sha256(files['campaign/fixture.ads']).hexdigest()})
    for release in evidence.PINS:
        # Actual pinned source bytes are optional external input in unit tests;
        # the hash operation for these two roots is isolated in this fixture.
        put(f'campaign/uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd', schema_fixture(release))
        put(f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd', 'synthetic-include-' + release)
        prefix = f'campaign/admission/phase-a-{release}/'
        put(prefix + 'result.json', dict(release=release, admitted=True, delivered=True, cleanup=True,
            publication_response='+OK', server_exit=0, socket_closed=True, status='PASS',
            schema_admitted=True, owp_negotiated=True, initialized=True, subscribed=True,
            unsubscribed=True, websocket_closed=True, generated_ada_decoder_executed=False))
        put(prefix + 'sleet-source.txt', evidence.REV + '\n' + evidence.TREE + '\n')
        put(prefix + 'server.exit', '0\n')
        put(prefix + f'schema/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd', schema_fixture(release))
        put(prefix + f'schema/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd', 'synthetic-include-' + release)
        files[prefix + f'task092-real-survivability{release}.json'] = (
            evidence.ROOT / 'tests/fixtures/service-generate/task092-real-survivability25.json').read_bytes()
        put(prefix + 'wire.json', [dict(receive='MSG 1 ' + json.dumps({'SurvivabilityRiskLevel': cases.golden(release)}))])
        put(f'campaign/{release}-check.stdout', 'selected type closure: 45\ncodec renderable emitted declarations: 44/44\ncodec status: READY\n')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb', 'service_api.ads'):
            for variant in ('generated', 'original-generated', 'model'):
                if variant == 'model' and name.startswith('service_codec'):
                    continue
                put(f'campaign/{release}-{variant}/{name}', 'synthetic unchanged ' + name)
        for policy in ('Normal', 'Check', 'Ignore'):
            put(f'campaign/{release}-codec-{policy}-execute.stdout', 'TASK092_REAL_PASS checks= 8\n')
            put(f'campaign/{release}-independent-{policy}-execute.stdout', '\n'.join([
                *['TASK104_CODEC_REJECTED ' + cases.CODEC_REASONS[label]
                  for label, _ in cases.invalid_documents(release) + cases.decoder_invalid(release)],
                *['TASK104_MODEL_CONSTRAINT_REJECTED  ' + str(n) + ' ' + reason for n, reason in enumerate((
                    'invalid UUID', 'invalid schema version',
                    'SurvivabilityRiskSettingType_RiskLevel_Sequence requires 1 .. 5 values',
                    'SurvivabilityRiskLevelMDT_RiskSetting_Sequence requires 1 .. 4 values'), 1)],
                'TASK104_INDEPENDENT_CODEC_COMPLETE rejected=14 model_constraints=4']) + '\n')
    for release, policy, optimization in expected:
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads', 'service_codec.ads', 'service_codec.adb'):
            put(f'campaign/uci{release}-{policy}-O{optimization}/generated/{name}', 'synthetic unchanged ' + name)
        prefix = f'campaign/uci{release}-{policy}-O{optimization}/sleet/'
        put(prefix + 'result.json', dict(completed=True, release=release, revision=evidence.REV, tree=evidence.TREE,
            handlers=12, witness_handlers=1, decode_rejected=1, published=6, broker_rejected=17,
            consumer_exit=0, server_exit=0, cleanup=True))
        put(prefix + 'sleet-source.txt', evidence.REV + '\n' + evidence.TREE + '\n')
        put(prefix + 'sleet-binary.sha256', 'd'*64)
        for phase in ('consumer', 'server'):
            put(prefix + phase + '.exit', '0\n')
        positive = cases.positives(release)
        risk_observations = []
        for case, repetitions in ((0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (0, 1)):
            for _ in range(repetitions):
                for index, setting in enumerate(positive[case][1]['MessageData']['RiskSetting'], 1):
                    allowed = setting['RiskLevel'][0]['AllowedRisk']
                    risk_observations.append(f'TASK104_RISK_OBSERVED setting= {index}'
                        f' priority={setting["RiskPrioritization"]}'
                        f' allowed={"VALUE_ALL" if allowed == "ALL" else allowed}'
                        f' levels= {len(setting["RiskLevel"])}')
        received = [{'SurvivabilityRiskLevel': doc} for _, doc in cases.decoder_invalid(release)]
        received += [{cases.MESSAGE: positive[0][1]}, {cases.MESSAGE: positive[0][2]}]
        for _, value, canonical in positive[1:]:
            received += [{cases.MESSAGE: value}, {cases.MESSAGE: canonical}]
        received += [{cases.MESSAGE: positive[0][1]}]
        put(prefix + 'consumer.stdout', '\n'.join([
            'TASK104_ADA_REAL_SURVIVABILITY_COMPLETE handlers= 12 rejected= 1',
            'TASK104_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1', 'TASK104_CLEANUP_COMPLETE',
            'TASK104_DECODE_RECOVERY_COMPLETE', 'TASK104_UNKNOWN_BINDING_SID_REJECTED',
            'TASK104_UPGRADE_OWP_COMPLETE', 'TASK104_INFO_VALIDATED', 'TASK104_SUB_ACCEPTED SurvivabilityRiskLevel task104-survivability',
            'TASK104_EVENT DECODE_ERROR invalid named DateTime',
            *['TASK104_EVENT COMMAND_ACCEPTED +OK'] * 12,
            *['TASK104_MODEL_CHECK_COMPLETE case= ' + str(n) for n in (0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 0)],
            *['TASK104_TYPED_HANDLER ' + str(n) for n in range(12)], 'TASK104_WITNESS_HANDLER 1',
            *risk_observations,
            *['TASK104_PUBLISHED ' + json.dumps(expected) for _, _, expected in positive],
            *['TASK104_WIRE_RECEIVED ' + json.dumps(doc) for doc in received]]) + '\n')
        put(prefix + 'wire.json', [dict(label=label, command=command,
                response=code + ' ' + detail + ' ' + (cases.BROKER_DETAILS.get(label, ('', ''))[1]))
                for label, command, code, detail in cases.broker_cases(release)]
            + [dict(label=label, command=cases.publication(doc), response='+OK') for label, doc in cases.decoder_invalid(release)]
            + [dict(label='same-subscription-recovery', command=cases.publication(positive[0][1]), response='+OK')]
            + [dict(label='external-' + label, command=cases.publication(value), response='+OK') for label, value, _ in positive[1:]]
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
        qualify = load('task104_git_trust', 'scripts/task104/qualify.py')
        shared = load('task104_git_campaign', 'scripts/task098/qualify.py')
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

    @unittest.skipUnless(os.environ.get('TASK104_OWNERSHIP_FIXTURE') == 'podman',
                         'opt-in disposable UID fixture: TASK104_OWNERSHIP_FIXTURE=podman')
    def test_git_trust_boundary_different_owner(self):
        self.check_boundary(actual_owner=True)


class EvidenceTests(unittest.TestCase):
    def test_per_tool_identities_under_posix_sh(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('task104_qualification_test',
            evidence.ROOT / 'scripts/task104/qualify.py')
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
        baseline = {name: hashlib.sha256(('synthetic unchanged ' + name).encode()).hexdigest()
                    for name in evidence.BASELINE}
        with patch.object(evidence, 'PINS', pins), patch.object(evidence, 'INCLUDES', includes), \
                patch.object(evidence, 'BASELINE', baseline):
            return evidence.verify_bundle(files, compiler, 'a'*40, 'b'*40, '123', '2')

    def test_both_compilers_complete_and_each_file_missing_or_duplicated(self):
        for compiler in ('14', '16'):
            files = bundle(compiler)
            self.assertEqual(self.verify(files, compiler)['cells'], 6)
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
                ('NATO:EXAMPLE', 'NATO:FORGED'),
                ('TASK104_WIRE_RECEIVED {"SurvivabilityRiskLevel":', 'TASK104_WIRE_RECEIVED {"SurvivabilityRiskLevelMT":'),
                ('TASK104_PUBLISHED ', 'NOT_A_PUBLICATION '),
                ('DECODE_ERROR invalid named DateTime', 'SERVER_ERROR invalid named DateTime'),
                ('TASK104_CLEANUP_COMPLETE', 'CLEANUP_MISSING'),
                ('primary_unchanged witness=1', 'primary_changed witness=1')]:
            self.assertIn(before, original)
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.verify(dict(files, **{prefix + 'consumer.stdout': original.replace(before, after).encode()}))
        for label in ('invalid-timestamp', 'same-subscription-recovery',
                      'external-maximum', 'external-optional-empty'):
            wire = json.loads(files[prefix + 'wire.json'])
            next(item for item in wire if item.get('label') == label)['response'] = '-ERR Invalid-Message'
            with self.subTest(label=label), self.assertRaises(ValueError):
                self.verify(dict(files, **{prefix + 'wire.json': json.dumps(wire).encode()}))

    def test_forged_completion_lock_pins_and_compiled_source(self):
        files = bundle()
        for path, value in [
            ('campaign/toolchain-lock.json', b'{"solution":{"state":[]}}'),
            ('campaign/gprbuild-version.stdout', b'wrong compiler'),
            ('campaign/uci25-Normal-O0/generated/programs-oam.ads', b'forged compiled model'),
            ('campaign/uci25/UCI_SecurityMarkings_v2_5_0.xsd', b'wrong authentic include'),
            ('campaign/uci25-Normal-O0/sleet/sleet-source.txt', b'wrong broker pin'),
        ]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: value}))
        for path, before, after in [
            ('campaign/25-independent-Normal-execute.stdout', b'RiskSetting item 1', b'wrong rejection'),
            ('campaign/25-independent-Normal-execute.stdout', b'REJECTED  4', b'REJECTED  3'),
            ('campaign/uci25-Normal-O0/sleet/consumer.stdout', b'TASK104_MODEL_CHECK_COMPLETE', b'FORGED_MODEL_CHECK'),
            ('campaign/uci25-Normal-O0/sleet/consumer.stdout', b'RECOVERY_COMPLETE', b'RECOVERY_SKIPPED'),
        ]:
            self.assertIn(before, files[path])
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: files[path].replace(before, after)}))


    def test_hashed_hidden_provider_sources_cannot_be_omitted(self):
        files = bundle()
        hidden = 'campaign/provider/gnatcoll/.github/workflows/main.yaml'
        files[hidden] = b'name: provider fixture\n'
        sources = json.loads(files['campaign/sources.json'])
        sources[hidden.removeprefix('campaign/')] = hashlib.sha256(files[hidden]).hexdigest()
        files['campaign/sources.json'] = json.dumps(sources).encode()
        EvidenceTests().verify(files)
        del files[hidden]
        with self.assertRaisesRegex(ValueError, 'missing/duplicate Task104 evidence'):
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
        self.assertIn("          - compiler: '16'\n            slot: 4\n", job)
        for forbidden in ('pull_request', 'inputs.selection', 'oms-pr-', 'continue-on-error'):
            self.assertNotIn(forbidden, header + job)
        for required in (evidence.REV,
                         'scripts/fetch-pinned-uci-2.5.sh', 'scripts/fetch-pinned-uci-2.6.sh',
                         'scripts/task104/qualify.py', 'from task104_evidence import verify_bundle',
                         "print('TASK104_HOSTED_VERIFIED', verify_bundle(",
                         'include-hidden-files: true'):
            self.assertIn(required, job)
        # Only diagnostics are unconditional; execution/verification must succeed.
        self.assertEqual(job.count('        if:'), 1)
        self.assertIn('name: Upload Task104 qualification diagnostics\n        if: always()', job)


if __name__ == '__main__':
    unittest.main()
