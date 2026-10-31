"""Task105 independently constructed verifier and selector rejection controls."""
import copy
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

import ci_selection
from ci_retry_test_data import metadata
from test_ci_select import change, gate, GateTests
import task105_evidence as evidence
spec = importlib.util.spec_from_file_location('task105_test_cases', evidence.ROOT / 'scripts/task105/cases.py')
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
    put('campaign/source.stdout', 'a'*40 + '\n' + 'b'*40 + '\n')
    put('campaign/source-status.stdout', '')
    put('campaign/gnat-version.stdout', 'GNAT ' + version + '\n')
    put('campaign/gprbuild-version.stdout', 'GPRBUILD Pro 26.0.0\n')
    put('campaign/generator.sha256', 'c'*64)
    hashes = dict(evidence.TOOL_HASHES[compiler], gprbuild='aff9fde5ca766d5497da9dec185cb8b96878206a0d02512aa5685e1e56d0a4d3')
    put('campaign/platform.stdout', 'x86_64\nglibc 2.41\n' + ''.join(digest + '  /tools/' + tool + '\n' for tool, digest in hashes.items()))
    put('campaign/alire-version.stdout', 'alr 2.1.1\n')
    put('campaign/rustc-version.stdout', 'rustc 1.95.0 (synthetic)\n')
    put('campaign/cargo-version.stdout', 'cargo 1.95.0 (synthetic)\n')
    guard_spec = importlib.util.spec_from_file_location('task105_test_lock', evidence.ROOT / 'scripts/check-task087a.py')
    guard = importlib.util.module_from_spec(guard_spec)
    guard_spec.loader.exec_module(guard)
    put('campaign/toolchain-lock.json', {'solution': {'state': [
        {'release': {'name': name, 'version': v, 'origin': {'hashes': [guard.PROVIDER_HASH,
            'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b',
            'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703']}}}
        for name, v in [('gnat_external' if compiler == '14' else 'gnat_native', version),
                        ('gnatcoll', '26.0.0'), ('gnatcoll_minimal', '26.0.0'), ('gprbuild', '26.0.1')]]}})
    inputs = ['scripts/task105/qualify.py', 'scripts/task105/sleet.py', 'scripts/task105/cases.py',
              'tests/ada-real-uci-survivability26/survivability_consumer.adb', 'scripts/task105_evidence.py',
              'tests/ada-real-uci-survivability26/codec_probe.adb', 'scripts/check-task087a.py',
              'scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
              'tests/ada-real-uci-survivability26/codec_consumer.adb', 'scripts/check-task092-real.py',
              'scripts/task105/admission.py', 'tests/fixtures/service-generate/codec-ada-minimal.yaml',
              'tests/fixtures/task105/survivability26.json', 'tests/fixtures/task105/uci26-input-service.yaml', 'scripts/task105/schema.py', 'scripts/task104_evidence.py', 'tests/fixtures/task104/uci25-service.yaml',
              ]
    inputs += [f'tests/fixtures/task105/uci{r}-service.yaml' for r in evidence.PINS]
    put('campaign/input-sources.json', {path: hashlib.sha256((evidence.ROOT / path).read_bytes()).hexdigest() for path in inputs})
    expected = list(itertools.product(('26',), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    labels = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission', 'schema-comparison', 'inherited25-generate', 'alire-version', 'rustc-version', 'cargo-version'}
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
    put('qualify.log', ''.join(f'TASK105_CELL_COMPLETE {version} uci{r}-{p}-O{o}\n' for r,p,o in expected)
        + f'TASK105_QUALIFICATION_COMPLETE {version} 6\n')
    put('campaign/schema-comparison.json', {'synthetic': True})
    for kind in ('MessageDefinitions', 'SecurityMarkings'):
        put(f'campaign/inherited25/UCI_{kind}_v2_5_0.xsd', 'synthetic inherited schema')
    for name in evidence.BASELINE:
        put('campaign/inherited25-generated/' + name, 'synthetic unchanged ' + name)
    for label in labels:
        files.setdefault('campaign/' + label + '.stdout', b'')
        files.setdefault('campaign/' + label + '.stderr', b'')
        argv = ['recorded-tool', label]
        if label.endswith(('-build', '-input', '-output')):
            argv = ['gprbuild', '-p', '-P', 'consumer.gpr']
        if label.startswith('uci26-') and label.endswith('-build'):
            argv += ['-cargs:Ada', '-gnato', '-' + label.split('-')[2], '-gnatec=/build/policy.adc']
        if label in ('26-check', '26-generate', '26-original', '26-model', 'inherited25-generate'):
            argv = ['generator', 'service-check' if label == '26-check' else 'service-generate',
                    '--schema', '/schema.xsd', '--contract', '/contract.yaml', '--language', 'ada',
                    '--world', 'closed-schema']
            if label != '26-model': argv.append('--with-codec')
            if label != '26-check': argv += ['--output', '/generated']
        if label.endswith('-sleet'):
            argv = ['python', '/source/scripts/task105/sleet.py', '--checkout', '/sleet',
                    '--consumer', '/consumer', '--schema', '/schema.xsd', '--release', '26', '--evidence', '/evidence']
        if '-independent-' in label and label.endswith('-execute'):
            argv = ['/build/bin/codec_probe', '/invalid.jsonl', '/reasons.txt']
        if label.startswith('26-codec-') and label.endswith('-execute'):
            argv = ['/build/bin/codec_consumer', '/source/tests/fixtures/task105/survivability26.json']
        if label == 'admission':
            argv = ['python', '/source/scripts/task105/admission.py', '--evidence', '/evidence',
                    '--sleet', '/sleet', '--uci26', '/schema.xsd']
        put('campaign/' + label + '.command.json', dict(argv=argv, cwd='/source'))
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
        files[prefix + 'survivability26.json'] = (
            evidence.ROOT / 'tests/fixtures/task105/survivability26.json').read_bytes()
        put(prefix + 'wire.json', [dict(receive='MSG 1 ' + json.dumps({'SurvivabilityRiskLevel': cases.golden(release)}))])
        put(f'campaign/{release}-check.stdout', 'selected type closure: 45\ncodec renderable emitted declarations: 44/44\ncodec status: READY\n')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb', 'service_api.ads'):
            for variant in ('generated', 'original-generated', 'model'):
                if variant == 'model' and name.startswith('service_codec'):
                    continue
                put(f'campaign/{release}-{variant}/{name}', 'synthetic unchanged ' + name)
        for policy in ('Normal', 'Check', 'Ignore'):
            put(f'campaign/{release}-codec-{policy}-execute.stdout', 'TASK105_CODEC_PASS checks= 8\n')
            put(f'campaign/{release}-independent-{policy}-execute.stdout', '\n'.join([
                *['TASK105_CODEC_REJECTED ' + cases.CODEC_REASONS[label]
                  for label, _ in cases.invalid_documents(release) + cases.decoder_invalid(release)],
                *['TASK105_MODEL_CONSTRAINT_REJECTED  ' + str(n) + ' ' + reason for n, reason in enumerate((
                    'invalid UUID', 'invalid schema version',
                    'SurvivabilityRiskSettingType_RiskLevel_Sequence requires 1 .. 5 values',
                    'SurvivabilityRiskLevelMDT_RiskSetting_Sequence requires 1 .. 4 values',
                    'invalid whitespace-visible string', 'invalid whitespace-visible string',
                    'not a valid XML Schema dateTime in the Zulu timezone'), 1)],
                'TASK105_INDEPENDENT_CODEC_COMPLETE rejected=23 model_constraints=7']) + '\n')
    original_api = 'package Service_API is\nend Service_API;\n'
    output_api = '      package Exchange_Publish is\n      end Exchange_Publish;\n\n'
    api = original_api.replace('end Service_API;', output_api + 'end Service_API;')
    for variant in ('generated', 'model'):
        put('campaign/26-' + variant + '/service_api.ads', api)
    put('campaign/26-original-generated/service_api.ads', original_api)
    put('campaign/service-api-difference.json', dict(baseline='uci26-input-service.yaml',
        added_package='Exchange_Publish', added_source=output_api, inherited_input_unchanged=True))
    for release, policy, optimization in expected:
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads', 'service_codec.ads', 'service_codec.adb'):
            put(f'campaign/uci{release}-{policy}-O{optimization}/generated/{name}', api if name == 'service_api.ads' else 'synthetic unchanged ' + name)
        prefix = f'campaign/uci{release}-{policy}-O{optimization}/sleet/'
        put(prefix + 'result.json', dict(completed=True, release=release, revision=evidence.REV, tree=evidence.TREE,
            handlers=12, witness_handlers=1, decode_rejected=1, published=6, broker_rejected=26,
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
                    risk_observations.append(f'TASK105_RISK_OBSERVED setting= {index}'
                        f' priority={setting["RiskPrioritization"]}'
                        f' allowed={"VALUE_ALL" if allowed == "ALL" else allowed}'
                        f' levels= {len(setting["RiskLevel"])}')
        received = [{'SurvivabilityRiskLevel': doc} for _, doc in cases.decoder_invalid(release)]
        received += [{cases.MESSAGE: positive[0][1]}, {cases.MESSAGE: positive[0][2]}]
        for _, value, canonical in positive[1:]:
            received += [{cases.MESSAGE: value}, {cases.MESSAGE: canonical}]
        received += [{cases.MESSAGE: positive[0][1]}]
        put(prefix + 'consumer.stdout', '\n'.join([
            'TASK105_ADA_REAL_SURVIVABILITY_COMPLETE handlers= 12 rejected= 1',
            'TASK105_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1', 'TASK105_CLEANUP_COMPLETE',
            'TASK105_DECODE_RECOVERY_COMPLETE', 'TASK105_UNKNOWN_BINDING_SID_REJECTED',
            'TASK105_UPGRADE_OWP_COMPLETE', 'TASK105_INFO_VALIDATED', 'TASK105_SUB_ACCEPTED SurvivabilityRiskLevel task105-survivability',
            'TASK105_EVENT DECODE_ERROR invalid named DateTime',
            *['TASK105_EVENT COMMAND_ACCEPTED +OK'] * 12,
            *['TASK105_MODEL_CHECK_COMPLETE case= ' + str(n) for n in (0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 0)],
            *['TASK105_TYPED_HANDLER ' + str(n) for n in range(12)], 'TASK105_WITNESS_HANDLER 1',
            *risk_observations,
            *['TASK105_PUBLISHED ' + json.dumps(expected) for _, _, expected in positive],
            *['TASK105_WIRE_RECEIVED ' + json.dumps(doc) for doc in received]]) + '\n')
        put(prefix + 'wire.json', [dict(label=label, command=command,
                response=code + ' ' + detail + ' ' + (cases.BROKER_DETAILS.get(label, ('', ''))[1]))
                for label, command, code, detail in cases.broker_cases(release)]
            + [dict(label=label, command=cases.publication(doc), response='+OK') for label, doc in cases.decoder_invalid(release)]
            + [dict(label='same-subscription-recovery', command=cases.publication(positive[0][1]), response='+OK')]
            + [dict(label='external-' + label, command=cases.publication(value), response='+OK') for label, value, _ in positive[1:]]
            + [dict(label='wrong-service-closed')]
            + [dict(label='wrong-init-schema-' + version, response='-ERR Unsupported-Schema does not match 002.6.0')
               for version in ('002.5.0', '002.6')]
            + [dict(label='wrong-init-schema-closed-' + version) for version in ('002.5.0', '002.6')]
            + [dict(label='other-release-payload-version', response='+OK',
                    command=cases.publication(dict(cases.golden(release), MessageHeader=dict(
                        cases.golden(release)['MessageHeader'], SchemaVersion='002.5.0'))))]
            + [dict(label='other-release-version-delivery', response='MSG 9 ' + json.dumps({cases.MESSAGE:
                    dict(cases.golden(release), MessageHeader=dict(cases.golden(release)['MessageHeader'],
                        SchemaVersion='002.5.0'))}))])
        put(prefix + 'server.stdout', 'schema loaded\nroute PUB message\nshutdown complete\n')
    return files


class GitTrustBoundary(unittest.TestCase):
    def check_boundary(self, actual_owner=False):
        def load(name, path):
            spec = importlib.util.spec_from_file_location(name, evidence.ROOT / path)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module
        qualify = load('task105_git_trust', 'scripts/task105/qualify.py')
        shared = load('task105_git_campaign', 'scripts/task098/qualify.py')
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

    @unittest.skipUnless(os.environ.get('TASK105_OWNERSHIP_FIXTURE') == 'podman',
                         'opt-in disposable UID fixture: TASK105_OWNERSHIP_FIXTURE=podman')
    def test_git_trust_boundary_different_owner(self):
        self.check_boundary(actual_owner=True)


class EvidenceTests(unittest.TestCase):
    def test_incomplete_commands_tool_drift_and_false_completion_fail_closed(self):
        files = bundle()
        for label in ('26-check', '26-generate', '26-model', 'uci26-Normal-O0-build',
                      'uci26-Normal-O0-sleet', '26-independent-Normal-execute'):
            path = 'campaign/' + label + '.command.json'
            for argv in ([], ['true'], ['sh', '-ec', 'true']):
                with self.subTest(label=label, argv=argv), self.assertRaises(ValueError):
                    self.verify(dict(files, **{path: json.dumps(dict(argv=argv, cwd='/source')).encode()}))
        for name in ('platform', 'alire-version', 'rustc-version', 'cargo-version'):
            path = 'campaign/' + name + '.stdout'
            with self.subTest(tool=name), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: b'wrong toolchain'}))
        result = json.loads(files['campaign/result.json'])
        result['completed'] = False
        with self.assertRaises(ValueError):
            self.verify(dict(files, **{'campaign/result.json': json.dumps(result).encode()}))
        prefix = 'campaign/uci26-Normal-O0/sleet/'
        for label in ('other-release-version-delivery', 'wrong-init-schema-002.6',
                      'wrong-init-schema-closed-002.5.0'):
            wire = json.loads(files[prefix + 'wire.json'])
            wire = [row for row in wire if row.get('label') != label]
            with self.subTest(label=label), self.assertRaises(ValueError):
                self.verify(dict(files, **{prefix + 'wire.json': json.dumps(wire).encode()}))

    def test_per_tool_identities_under_posix_sh(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('task105_qualification_test',
            evidence.ROOT / 'scripts/task105/qualify.py')
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
                patch.object(evidence, 'BASELINE', baseline), \
                patch('task104_evidence.BASELINE', baseline), \
                patch.object(evidence.schema_oracle, 'compare', return_value={'synthetic': True}):
            return evidence.verify_bundle(files, compiler, 'a'*40, 'b'*40, '123', '2')

    def test_both_compilers_complete_and_each_file_missing_or_duplicated(self):
        for compiler in ('14', '16'):
            files = bundle(compiler)
            summary = self.verify(files, compiler)
            self.assertEqual(summary['cells'], 6)
            self.assertEqual(summary['compiler'], {'14': '14.2.0', '16': '16.1.0'}[compiler])
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
            ('campaign/uci26/UCI_MessageDefinitions_v2_6_0.xsd', b'wrong authentic release'),
            ('campaign/fixture.ads', b'stale source'), ('qualify.log', b''),
            ('campaign/uci26-Normal-O0-build.exit', b'1'),
            ('campaign/uci26-Normal-O0/sleet/consumer.stdout', b'crashed'),
            ('campaign/uci26-Normal-O0/sleet/wire.json', b'[]'),
            ('campaign/uci26-Normal-O0/sleet/server.exit', b'137'),
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
        prefix = 'campaign/uci26-Normal-O0/sleet/'
        original = files[prefix + 'consumer.stdout'].decode()
        for before, after in [
                ('NATO:EXAMPLE', 'NATO:FORGED'),
                ('TASK105_WIRE_RECEIVED {"SurvivabilityRiskLevel":', 'TASK105_WIRE_RECEIVED {"SurvivabilityRiskLevelMT":'),
                ('TASK105_PUBLISHED ', 'NOT_A_PUBLICATION '),
                ('DECODE_ERROR invalid named DateTime', 'SERVER_ERROR invalid named DateTime'),
                ('TASK105_CLEANUP_COMPLETE', 'CLEANUP_MISSING'),
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
            ('campaign/uci26-Normal-O0/generated/programs-oam.ads', b'forged compiled model'),
            ('campaign/uci26/UCI_SecurityMarkings_v2_6_0.xsd', b'wrong authentic include'),
            ('campaign/uci26-Normal-O0/sleet/sleet-source.txt', b'wrong broker pin'),
        ]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: value}))
        for path, before, after in [
            ('campaign/26-independent-Normal-execute.stdout', b'RiskSetting item 1', b'wrong rejection'),
            ('campaign/26-independent-Normal-execute.stdout', b'REJECTED  4', b'REJECTED  3'),
            ('campaign/uci26-Normal-O0/sleet/consumer.stdout', b'TASK105_MODEL_CHECK_COMPLETE', b'FORGED_MODEL_CHECK'),
            ('campaign/uci26-Normal-O0/sleet/consumer.stdout', b'RECOVERY_COMPLETE', b'RECOVERY_SKIPPED'),
        ]:
            self.assertIn(before, files[path])
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(dict(files, **{path: files[path].replace(before, after)}))



class SelectionAndAggregate(GateTests):
    def test_task105_and_manual_benchmark_integration_preserves_both_policies(self):
        import ci_policy
        from ci_bench import policy as benchmark
        policy = ci_selection.inventory()
        workflows = {p.name: p.read_text() for p in (evidence.ROOT / '.github/workflows').glob('*.yml')}
        self.assertEqual(ci_policy.check(policy, workflows), 50)
        registered = ci_selection.deep_jobs(policy)
        self.assertEqual([c['variant']['compiler'] for c in registered[evidence.JOB]['cells']], ['14', '16'])
        self.assertEqual(policy['manual_benchmark']['events'], ['workflow_dispatch'])
        self.assertEqual(policy['manual_benchmark']['sources_sha256']['scripts/ci_artifact_provenance.py'],
                         'f2f7cfe5dcde40cbd64832f8ac8be83ff229cfd760dc955364ec4f9b850158b2')
        for path in ('scripts/task105/qualify.py', 'scripts/ci_bench/controller.py'):
            selection = ci_selection.select([change(path)], policy)
            self.assertIn(evidence.JOB, selection['deep_jobs'])
            self.assertFalse(set(benchmark.JOBS) & (set(selection['jobs']) | set(selection['deep_jobs'])))
        for mutation in ('remove-task105', 'automatic-benchmark', 'wildcard-trust'):
            modified_policy, modified_workflows = copy.deepcopy(policy), dict(workflows)
            if mutation == 'remove-task105':
                modified_policy['jobs'] = [r for r in modified_policy['jobs'] if r['job_id'] != evidence.JOB]
            elif mutation == 'automatic-benchmark':
                modified_policy['manual_benchmark']['events'].append('pull_request')
            else:
                modified_workflows['portable-provider-benchmark.yml'] += '\ninvalid-trust: safe.directory=*\n'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                ci_policy.check(modified_policy, modified_workflows)

    def test_hashed_hidden_provider_sources_cannot_be_omitted(self):
        import ci_policy
        workflows = {p.name: p.read_text() for p in (evidence.ROOT / '.github/workflows').glob('*.yml')}
        addition = ('          name: task105-ada-survivability-26-${{ matrix.compiler }}\n'
                    '          include-hidden-files: true\n')
        self.assertEqual(workflows['deep-ci.yml'].count(addition), 1)
        for replacement in (addition.replace('          include-hidden-files: true\n', ''),
                            addition.replace('true', 'false')):
            modified = dict(workflows)
            modified['deep-ci.yml'] = modified['deep-ci.yml'].replace(addition, replacement)
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                ci_policy.check(workflows=modified)
        files = bundle()
        hidden = 'campaign/provider/gnatcoll/.github/workflows/main.yaml'
        files[hidden] = b'name: provider fixture\n'
        sources = json.loads(files['campaign/sources.json'])
        sources[hidden.removeprefix('campaign/')] = hashlib.sha256(files[hidden]).hexdigest()
        files['campaign/sources.json'] = json.dumps(sources).encode()
        EvidenceTests().verify(files)
        del files[hidden]
        with self.assertRaisesRegex(ValueError, 'missing/duplicate Task105 evidence'):
            EvidenceTests().verify(files)

    def test_task105_surfaces_select_both_families_and_deep(self):
        policy = ci_selection.inventory()
        for path in ('scripts/task105/qualify.py', 'scripts/task105/sleet.py', 'scripts/task105_evidence.py',
                     'tests/ada-real-uci-survivability26/survivability_consumer.adb', 'tests/fixtures/task105/uci26-service.yaml'):
            result = ci_selection.select([change(path)], policy)
            self.assertIn(evidence.JOB, result['deep_jobs'])
            self.assertTrue({'ada-websocket-transport-14', 'ada-websocket-transport-16'} <= set(result['cells']))

    def test_missing_duplicate_expired_stale_and_skipped_cells(self):
        self.validate()
        for compiler in ('14', '16'):
            name = 'task105-ada-survivability-26-' + compiler
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
        artifacts = [dict(name='task105-ada-survivability-26-' + c) for c in ('14', '16')]
        jobs, attempts = metadata(artifacts)
        manifest = dict(deep_jobs=[evidence.JOB])
        with patch.object(gate, 'verify_task105_bundle', return_value=dict(cells=12)) as verify:
            result = gate.reconcile_task105(artifacts, manifest, 'a'*40, 'b'*40, '123', '2',
                                           lambda item: {}, jobs=jobs, attempts=attempts)
            self.assertEqual(len(result), 2)
            self.assertEqual(verify.call_count, 2)
        for mutation in ('stale', 'expired', 'mismatched-head', 'skipped-step', 'wrong-producer-attempt'):
            changed, history, current = copy.deepcopy(artifacts), copy.deepcopy(attempts), copy.deepcopy(jobs)
            if mutation == 'stale': changed[0]['created_at'] = '2000-01-01T00:00:00Z'
            elif mutation == 'expired': changed[0]['expired'] = True
            elif mutation == 'mismatched-head': changed[0]['workflow_run']['head_sha'] = '0'*40
            elif mutation == 'skipped-step': current[0]['steps'][0]['conclusion'] = 'skipped'
            else: history[2]['jobs'][0]['run_attempt'] = 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                gate.reconcile_task105(changed, manifest, 'a'*40, 'b'*40, '123', '2',
                                      lambda item: {}, jobs=current, attempts=history)
        with patch.object(gate, 'verify_task105_bundle', side_effect=ValueError('invalid source-bound bundle')), self.assertRaises(ValueError):
            gate.reconcile_task105(artifacts, manifest, 'a'*40, 'b'*40, '123', '2',
                                  lambda item: {}, jobs=jobs, attempts=attempts)


if __name__ == '__main__':
    unittest.main()