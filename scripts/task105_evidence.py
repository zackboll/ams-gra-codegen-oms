"""Source-bound Task105 evidence verifier; expectations are not artifact-owned."""
import hashlib
import itertools
import json
import re
import importlib.util
import shlex
import tempfile
from pathlib import Path
from task105 import schema as schema_oracle

JOB = 'ada-real-uci-survivability-26-sleet'
PINS = {'26': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
INCLUDES = {'25': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
            '26': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
ROOT = Path(__file__).resolve().parents[1]
BASELINE = {
    'programs.ads': 'a5d92d26a18c03999588818e68c5a8b3426ff1c780a7582bf21121727b4a0d16',
    'programs-oam.ads': 'bec55b85b53964b24ad8b8045fbdc65e2df02af0b8cae778db286c7ec96f4832',
    'programs-oam.adb': 'bff53529498906f66edd6fe13735e6837d2240d5f37d5ac093a20e267114e589',
    'service_codec.ads': 'ed7475f3d74fe3e91028f7c3d601ce2f4288393284debb6a723dbc32d6e6266d',
    'service_codec.adb': '17b0874932ab43b8b8ebe84054fc3fb7f2f750f0f66974d490b41e30f0f03637',
}
TOOL_HASHES = {
    '14': {'gcc': 'a23ecab8ff08f09ad8c80602c2c5df7f49e09c25905cb8975902e101bf72635f',
           'gnat': '67082a9217f83ff443253d5d02973615b3f2cc01d47e527d857d87229f7c2c4f',
           'ld': 'd2766988b6a1b1d1518cdc09242e726d1f24e151e16410ada3092b11ffd9081b'},
    '16': {'gcc': '238f4adafcb1425c84da6519068fdfcf33e57e406fc793e2958a9238789900de',
           'gnat': 'fa15577ec622065f1e80d18f14d315d8b3d9a6ff17cf195ffe4fe3895c1f84d9',
           'ld': '5fedbb6086b0234eb990800507c0670cb572b48ad864a4d8e8f961ddddef09a0'},
}



def verify_bundle(files, compiler, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [value for name, value in files.items() if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task105 evidence: ' + suffix)
        return matches[0]
    def text(suffix):
        return raw(suffix).decode('utf-8', errors='strict')
    def doc(suffix):
        return json.loads(text(suffix))
    if doc('integration-identity.json') != dict(sha=sha, tree=tree, run_id=str(run_id),
                                              run_attempt=str(attempt), compiler=compiler):
        raise ValueError('Task105 source/tree/run/attempt/compiler mismatch')
    if text('campaign/source.stdout').splitlines() != [sha, tree] or text('campaign/source-status.stdout').strip():
        raise ValueError('Task105 dirty/wrong source identity')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    lock_spec = importlib.util.spec_from_file_location('task105_lock_guard', ROOT / 'scripts/check-task087a.py')
    lock_guard = importlib.util.module_from_spec(lock_spec)
    lock_spec.loader.exec_module(lock_guard)
    lock = doc('campaign/toolchain-lock.json')
    try:
        lock_guard.validate_solution(lock,
                                     'gnat_external' if compiler == '14' else 'gnat_native')
    except RuntimeError as exc:
        raise ValueError('Task105 unqualified toolchain lock') from exc
    archive_pins = {'gprbuild': 'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b'}
    if compiler == '16':
        archive_pins['gnat_native'] = 'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703'
    for name, digest in archive_pins.items():
        releases = [s['release'] for s in lock['solution']['state'] if s.get('release', {}).get('name') == name]
        if not releases or any(digest not in r.get('origin', {}).get('hashes', []) for r in releases):
            raise ValueError('Task105 compiler/builder archive pin mismatch')
    if not text('campaign/gnat-version.stdout').startswith('GNAT ' + version + '\n'):
        raise ValueError('Task105 wrong compiler')
    if '26.0.0' not in text('campaign/gprbuild-version.stdout'):
        raise ValueError('Task105 wrong GPRbuild identity')
    platform = text('campaign/platform.stdout')
    hashes = dict(TOOL_HASHES[compiler], gprbuild='aff9fde5ca766d5497da9dec185cb8b96878206a0d02512aa5685e1e56d0a4d3')
    for tool, digest in hashes.items():
        if not re.search(r'(?m)^' + digest + r'  /[^\n]*/' + tool + r'$', platform):
            raise ValueError('Task105 toolchain binary drift: ' + tool)
    if not platform.startswith('x86_64\nglibc 2.41\n'):
        raise ValueError('Task105 wrong target platform')
    if (text('campaign/alire-version.stdout') != 'alr 2.1.1\n'
            or not text('campaign/rustc-version.stdout').startswith('rustc 1.95.0 ')
            or not text('campaign/cargo-version.stdout').startswith('cargo 1.95.0 ')):
        raise ValueError('Task105 Alire/Rust build identity drift')
    if not re.fullmatch('[0-9a-f]{64}', text('campaign/generator.sha256').strip()):
        raise ValueError('Task105 generator identity missing')
    inputs = doc('campaign/input-sources.json')
    required_inputs = {'scripts/task105/qualify.py', 'scripts/task105/sleet.py', 'scripts/task105/cases.py',
                       'tests/ada-real-uci-survivability26/survivability_consumer.adb', 'scripts/task105_evidence.py',
                       'scripts/task098/qualify.py', 'tests/ada-real-uci-survivability26/codec_consumer.adb', 'scripts/check-task092-real.py'}
    required_inputs |= {'tests/ada-real-uci-survivability26/codec_probe.adb', 'scripts/check-task087a.py'}
    required_inputs |= {f'tests/fixtures/task105/uci{r}-service.yaml' for r in PINS}
    required_inputs |= {'scripts/task105/admission.py', 'tests/fixtures/service-generate/codec-ada-minimal.yaml'}
    required_inputs |= {'tests/fixtures/task105/survivability26.json', 'tests/fixtures/task105/uci26-input-service.yaml', 'scripts/task105/schema.py'}
    required_inputs |= {'scripts/task104_evidence.py', 'tests/fixtures/task104/uci25-service.yaml'}
    if not required_inputs <= inputs.keys():
        raise ValueError('Task105 mandatory input sources absent')
    for path, digest in inputs.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('Task105 qualification inputs differ from exact source')
    result = doc('campaign/result.json')
    expected = set(itertools.product(('26',), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    actual = [(c['release'], c['policy'], c['optimization']) for c in result['cells']]
    if (result.get('completed') is not True or result.get('compiler') != version
            or result.get('real_sleet') is not True or result.get('real_uci') is not True
            or len(actual) != 6 or set(actual) != expected):
        raise ValueError('Task105 missing/duplicate/wrong release/policy/optimization cell')
    steps = result['steps']
    labels = [step['label'] for step in steps]
    intentional = {f'{r}-direction-{d}' for r in PINS for d in ('input', 'output')}
    if not labels or len(labels) != len(set(labels)) or any(s['exit'] != 0 for s in steps if s['label'] not in intentional):
        raise ValueError('Task105 failed/duplicate execution step')
    mandatory = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission',
                 'schema-comparison', 'inherited25-generate', 'alire-version', 'rustc-version', 'cargo-version'}
    mandatory |= {f'{name}-{step}' for name in ('aht', 'websocket-ada')
                  for step in ('clone', 'pin', 'identity', 'clean')}
    mandatory |= {f'{r}-{s}' for r in PINS for s in ('check', 'generate', 'original', 'model')}
    mandatory |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    mandatory |= {f'{r}-codec-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    mandatory |= {f'{r}-independent-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    if not mandatory <= set(labels):
        raise ValueError('Task105 mandatory execution step absent')
    for label in labels:
        raw('campaign/' + label + '.stdout')
        raw('campaign/' + label + '.stderr')
        observed = text('campaign/' + label + '.exit').strip()
        recorded = next(s['exit'] for s in steps if s['label'] == label)
        if observed != str(recorded) or (label not in intentional and observed != '0') or (label in intentional and observed == '0'):
            raise ValueError('Task105 execution exit mismatch')
        command = doc('campaign/' + label + '.command.json')
        argv = command.get('argv')
        if (not isinstance(argv, list) or not argv or any(not isinstance(arg, str) or not arg for arg in argv)
                or not isinstance(command.get('cwd'), str) or not command['cwd'].startswith('/')):
            raise ValueError('Task105 incomplete command/cwd record')
        if argv[:2] == ['sh', '-ec']:
            if len(argv) != 3 or not argv[2].startswith('. ') or '; exec ' not in argv[2]:
                raise ValueError('Task105 incomplete pinned build environment command')
            argv = shlex.split(argv[2].split('; exec ', 1)[1])
        if label.endswith(('-build', '-input', '-output')) and (
                not argv or argv[0] != 'gprbuild' or '-p' not in argv or '-P' not in argv
                or argv.index('-P') + 1 >= len(argv)):
            raise ValueError('Task105 incomplete builder/project command')
        if label.startswith('uci26-') and label.endswith('-build'):
            optimization = label.split('-')[2]
            if (not {'-gnato', '-' + optimization, '-cargs:Ada'} <= set(argv)
                    or not any(a.startswith('-gnatec=') and a.endswith('/policy.adc') for a in argv)):
                raise ValueError('Task105 missing cell optimization/assertion/overflow command')
        if label in ('26-check', '26-generate', '26-original', '26-model', 'inherited25-generate'):
            operation = 'service-check' if label == '26-check' else 'service-generate'
            if (len(argv) < 10 or argv[1] != operation or '--schema' not in argv
                    or '--contract' not in argv or '--language' not in argv
                    or argv[argv.index('--language') + 1] != 'ada' or '--world' not in argv
                    or argv[argv.index('--world') + 1] != 'closed-schema'
                    or ('--with-codec' in argv) != (label != '26-model')
                    or (operation == 'service-generate' and '--output' not in argv)):
                raise ValueError('Task105 incomplete generation command')
        if label.endswith('-sleet') and (not any(a.endswith('/scripts/task105/sleet.py') for a in argv)
                or not {'--checkout', '--consumer', '--schema', '--release', '--evidence'} <= set(argv)):
            raise ValueError('Task105 actual Sleet execution command absent')
        if '-independent-' in label and label.endswith('-execute') and (
                len(argv) != 3 or not argv[0].endswith('/bin/codec_probe')):
            raise ValueError('Task105 independent codec execution command absent')
        if label.startswith('26-codec-') and label.endswith('-execute') and (
                len(argv) != 2 or not argv[0].endswith('/bin/codec_consumer')
                or not argv[1].endswith('/tests/fixtures/task105/survivability26.json')):
            raise ValueError('Task105 source-generated codec consumer command absent')
        if label == 'admission' and (not any(a.endswith('/scripts/task105/admission.py') for a in argv)
                or not {'--evidence', '--sleet', '--uci26'} <= set(argv)):
            raise ValueError('Task105 independent admission command absent')
    for name, revision in [('aht', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
                           ('websocket-ada', '765d9cd612838eb00645037977c8cc9af7171c3e')]:
        if text(f'campaign/{name}-identity.stdout').strip() != revision or text(f'campaign/{name}-clean.stdout').strip():
            raise ValueError('Task105 transport dependency source pin/cleanliness mismatch')
    if not intentional <= set(labels):
        raise ValueError('Task105 static endpoint direction probes absent')
    for release in PINS:
        for direction, forbidden in [('input', 'Publisher'), ('output', 'Subscriber')]:
            if f'"{forbidden}" not declared' not in text(f'campaign/{release}-direction-{direction}.stderr'):
                raise ValueError('Task105 endpoint probe failed for wrong reason')
        admission = doc(f'campaign/admission/phase-a-{release}/result.json')
        if any(admission.get(k) is not True for k in (
                'schema_admitted', 'owp_negotiated', 'initialized', 'subscribed',
                'admitted', 'delivered', 'unsubscribed', 'cleanup', 'socket_closed', 'websocket_closed')) or (
                admission.get('publication_response') != '+OK' or admission.get('server_exit') != 0
                or admission.get('status') != 'PASS' or admission.get('generated_ada_decoder_executed') is not False):
            raise ValueError('Task105 independent broker admission/cleanup missing')
        prefix = f'campaign/admission/phase-a-{release}/'
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE] or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task105 independent admission identity/exit mismatch')
    log = text('qualify.log').splitlines()
    if (sum(line.startswith('TASK105_CELL_COMPLETE ') for line in log) != 6
            or f'TASK105_QUALIFICATION_COMPLETE {version} 6' not in log):
        raise ValueError('Task105 completion marker absent')
    sources = doc('campaign/sources.json')
    if not sources:
        raise ValueError('Task105 source hashes absent')
    for path, digest in sources.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task105 generated/fixture provenance mismatch')
    for release in PINS:
        if not raw(f'campaign/{release}-original-generated/service_api.ads'):
            raise ValueError('Task105 original generated API absent')
        api = text(f'campaign/{release}-generated/service_api.ads')
        begin = api.index('      package Exchange_Publish is\n')
        end = api.index('      end Exchange_Publish;\n', begin) + len('      end Exchange_Publish;\n\n')
        if api[:begin] + api[end:] != text(f'campaign/{release}-original-generated/service_api.ads'):
            raise ValueError('Task105 input API changed beyond intentional output addition')
        if doc('campaign/service-api-difference.json') != dict(baseline='uci26-input-service.yaml',
                added_package='Exchange_Publish', added_source=api[begin:end], inherited_input_unchanged=True):
            raise ValueError('Task105 intentional service API difference not recorded')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb'):
            if hashlib.sha256(raw(f'campaign/{release}-generated/{name}')).hexdigest() != BASELINE[name]:
                raise ValueError('Task105 generated model/codec differs from reviewed authentic UCI 2.6 generation')
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-original-generated/{name}'):
                raise ValueError('Task105 model/codec original selection byte parity mismatch')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-model/{name}'):
                raise ValueError('Task105 model/API codec opt-in byte parity mismatch')
        for policy, optimization in itertools.product(('Normal', 'Check', 'Ignore'), ('0', '2')):
            for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads', 'service_codec.ads', 'service_codec.adb'):
                if raw(f'campaign/uci{release}-{policy}-O{optimization}/generated/{name}') != raw(f'campaign/{release}-generated/{name}'):
                    raise ValueError('Task105 compiled cell generated source differs')
    from task104_evidence import BASELINE as PARENT_BASELINE
    for name, digest in PARENT_BASELINE.items():
        if hashlib.sha256(raw('campaign/inherited25-generated/' + name)).hexdigest() != digest:
            raise ValueError('Task105 inherited UCI 2.5 model/codec changed')
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        roots = {}
        for release, folder in [('25', 'inherited25'), ('26', 'uci26')]:
            for kind in ('MessageDefinitions', 'SecurityMarkings'):
                name = f'UCI_{kind}_v2_{release[-1]}_0.xsd'
                (directory / name).write_bytes(raw(f'campaign/{folder}/{name}'))
            roots[release] = directory / f'UCI_MessageDefinitions_v2_{release[-1]}_0.xsd'
        if doc('campaign/schema-comparison.json') != schema_oracle.compare(roots['25'], roots['26']):
            raise ValueError('Task105 structural schema comparison differs from authentic sources')
    for release, digest in PINS.items():
        path = f'uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task105 wrong UCI release/root')
        include = f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw(include)).hexdigest() != INCLUDES[release]:
            raise ValueError('Task105 authentic schema include mismatch')
        for kind, pin in [('MessageDefinitions', digest), ('SecurityMarkings', INCLUDES[release])]:
            if hashlib.sha256(raw(f'campaign/admission/phase-a-{release}/schema/UCI_{kind}_v2_{release[-1]}_0.xsd')).hexdigest() != pin:
                raise ValueError('Task105 admission authentic schema hash mismatch')
        report = text(f'campaign/{release}-check.stdout')
        selected = 45
        if f'selected type closure: {selected}\n' not in report or f'codec renderable emitted declarations: {selected-1}/{selected-1}\ncodec status: READY\n' not in report:
            raise ValueError('Task105 actual SurvivabilityRiskLevel readiness absent')
        for policy in ('Normal', 'Check', 'Ignore'):
            output = text(f'campaign/{release}-codec-{policy}-execute.stdout')
            if output != 'TASK105_CODEC_PASS checks= 8\n':
                raise ValueError('Task105 mandatory Task092 real SurvivabilityRiskLevel codec corpus incomplete')
            independent = text(f'campaign/{release}-independent-{policy}-execute.stdout').splitlines()
            if (len(independent) != 31
                    or independent[-1] != 'TASK105_INDEPENDENT_CODEC_COMPLETE rejected=23 model_constraints=7'
                    or sum(line.startswith('TASK105_CODEC_REJECTED ') for line in independent) != 23
                    or sum(line.startswith('TASK105_MODEL_CONSTRAINT_REJECTED ') for line in independent) != 7):
                raise ValueError('Task105 independent codec/model constraints incomplete')
    spec = importlib.util.spec_from_file_location('task105_independent_cases', ROOT / 'scripts/task105/cases.py')
    cases = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cases)
    for release in PINS:
        prefix = f'campaign/admission/phase-a-{release}/'
        for policy in ('Normal', 'Check', 'Ignore'):
            lines = text(f'campaign/{release}-independent-{policy}-execute.stdout').splitlines()
            reasons = ['TASK105_CODEC_REJECTED ' + cases.CODEC_REASONS[label]
                       for label, _ in cases.invalid_documents(release) + cases.decoder_invalid(release)]
            if lines[:23] != reasons:
                raise ValueError('Task105 independent codec rejected for wrong reason')
            if lines[23:30] != ['TASK105_MODEL_CONSTRAINT_REJECTED  ' + str(n) + ' ' + reason for n, reason in enumerate((
                    'invalid UUID', 'invalid schema version',
                    'SurvivabilityRiskSettingType_RiskLevel_Sequence requires 1 .. 5 values',
                    'SurvivabilityRiskLevelMDT_RiskSetting_Sequence requires 1 .. 4 values',
                    'invalid whitespace-visible string', 'invalid whitespace-visible string',
                    'not a valid XML Schema dateTime in the Zulu timezone'), 1)]:
                raise ValueError('Task105 independent model constraint coverage incomplete')
        if doc(prefix + 'survivability26.json') != cases.golden(release):
            raise ValueError('Task105 admission golden differs')
        golden_bytes = raw(prefix + 'survivability26.json')
        if hashlib.sha256(golden_bytes).hexdigest() != '350a884fe52c8513260fe064fe5b527707c755b10536b4a2da4d1b9f30713071':
            raise ValueError('Task105 authored UCI 2.6 fixture bytes changed')
        admission_wire = doc(prefix + 'wire.json')
        deliveries = [item['receive'] for item in admission_wire if item.get('receive', '').startswith('MSG 1 ')]
        if len(deliveries) != 1 or json.loads(deliveries[0].split(' ', 2)[2]) != {'SurvivabilityRiskLevel': cases.golden(release)}:
            raise ValueError('Task105 admission real broker delivery missing')
    for release, policy, optimization in expected:
        cell = f'uci{release}-{policy}-O{optimization}'
        prefix = f'campaign/{cell}/sleet/'
        evidence = doc(prefix + 'result.json')
        if evidence != dict(completed=True, release=release, revision=REV, tree=TREE, handlers=12,
                            witness_handlers=1, decode_rejected=1, published=6, broker_rejected=26,
                            consumer_exit=0, server_exit=0, cleanup=True):
            raise ValueError('Task105 real-Sleet handler/count/cleanup mismatch')
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE]:
            raise ValueError('Task105 Sleet source mismatch')
        if not re.fullmatch('[0-9a-f]{64}', text(prefix + 'sleet-binary.sha256').strip()):
            raise ValueError('Task105 Sleet binary identity missing')
        if text(prefix + 'consumer.exit').strip() != '0' or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task105 consumer/server failed')
        output = text(prefix + 'consumer.stdout').splitlines()
        if output.count('TASK105_EVENT COMMAND_ACCEPTED +OK') != 12:
            raise ValueError('Task105 generated publications/subscriptions not broker accepted')
        for marker in ['TASK105_ADA_REAL_SURVIVABILITY_COMPLETE handlers= 12 rejected= 1',
                       'TASK105_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1',
                       'TASK105_CLEANUP_COMPLETE', 'TASK105_DECODE_RECOVERY_COMPLETE',
                       'TASK105_UNKNOWN_BINDING_SID_REJECTED', 'TASK105_UPGRADE_OWP_COMPLETE',
                       'TASK105_INFO_VALIDATED', 'TASK105_SUB_ACCEPTED SurvivabilityRiskLevel task105-survivability']:
            if output.count(marker) != 1:
                raise ValueError('Task105 interoperability marker missing/duplicate')
        if (sum(s.startswith('TASK105_TYPED_HANDLER ') for s in output) != 12
                or sum(s.startswith('TASK105_WITNESS_HANDLER ') for s in output) != 1
                or sum(s.startswith('TASK105_PUBLISHED ') for s in output) != 6
                or sum(s.startswith('TASK105_EVENT DECODE_ERROR ') for s in output) != 1):
            raise ValueError('Task105 actual handler/codec/publication count mismatch')
        model_checks = [s for s in output if s.startswith('TASK105_MODEL_CHECK_COMPLETE case=')]
        if model_checks != ['TASK105_MODEL_CHECK_COMPLETE case= ' + str(n) for n in (0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 0)]:
            raise ValueError('Task105 decoded model discriminant/value/order observations absent')
        published = [json.loads(s.removeprefix('TASK105_PUBLISHED ')) for s in output if s.startswith('TASK105_PUBLISHED ')]
        positive = cases.positives(release)
        # Observations must come from generated model fields, not just a
        # completion counter or encode/decode equality. Check their exact order.
        risk_observations = []
        for case, repetitions in ((0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (0, 1)):
            settings = positive[case][1]['MessageData']['RiskSetting']
            for _ in range(repetitions):
                for index, setting in enumerate(settings, 1):
                    levels = setting['RiskLevel']
                    allowed = levels[0]['AllowedRisk']
                    if allowed == 'ALL':
                        allowed = 'VALUE_ALL'
                    risk_observations.append(f'TASK105_RISK_OBSERVED setting= {index}'
                        f' priority={setting["RiskPrioritization"]} allowed={allowed} levels= {len(levels)}')
        if [s for s in output if s.startswith('TASK105_RISK_OBSERVED ')] != risk_observations:
            raise ValueError('Task105 nested generated risk observations absent/wrong order/value')
        if published != [expected for _, _, expected in positive]:
            raise ValueError('Task105 publications differ from independent real SurvivabilityRiskLevel oracle')
        received = [json.loads(s.removeprefix('TASK105_WIRE_RECEIVED ')) for s in output if s.startswith('TASK105_WIRE_RECEIVED ')]
        expected_wire = [{'SurvivabilityRiskLevel': doc} for _, doc in cases.decoder_invalid(release)]
        expected_wire += [{cases.MESSAGE: positive[0][1]}, {cases.MESSAGE: positive[0][2]}]
        for _, value, canonical in positive[1:]:
            expected_wire += [{cases.MESSAGE: value}, {cases.MESSAGE: canonical}]
        expected_wire += [{cases.MESSAGE: positive[0][1]}]
        if received != expected_wire:
            raise ValueError('Task105 broker delivery wire identity/value/order mismatch')
        errors = [s.removeprefix('TASK105_EVENT DECODE_ERROR ') for s in output if s.startswith('TASK105_EVENT DECODE_ERROR ')]
        if errors != ['invalid named DateTime']:
            raise ValueError('Task105 decoder rejection diagnostic/stage mismatch')
        wire = doc(prefix + 'wire.json')
        for label, command, code, detail in cases.broker_cases(release):
            matches = [item for item in wire if item.get('label') == label]
            if (len(matches) != 1 or matches[0].get('command') != command
                    or not matches[0]['response'].startswith(code) or detail not in matches[0]['response']):
                raise ValueError('Task105 expected broker error absent/wrong reason: ' + label)
            if label in cases.BROKER_DETAILS and cases.BROKER_DETAILS[label][1] not in matches[0]['response']:
                raise ValueError('Task105 broker rejection path/detail mismatch: ' + label)
        for label, value in cases.decoder_invalid(release):
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or matches[0]['response'] != '+OK' or matches[0]['command'] != cases.publication(value):
                raise ValueError('Task105 real broker/codec stage separation absent')
        recovery = [item for item in wire if item.get('label') == 'same-subscription-recovery']
        if len(recovery) != 1 or recovery[0]['response'] != '+OK' or recovery[0]['command'] != cases.publication(positive[0][1]):
            raise ValueError('Task105 same subscription valid recovery absent')
        for label, value, _ in positive[1:]:
            admission = [item for item in wire if item.get('label') == 'external-' + label]
            if len(admission) != 1 or admission[0]['response'] != '+OK' or admission[0]['command'] != cases.publication(value):
                raise ValueError('Task105 independent broker/typed variant observation absent')
        if sum(item.get('label') == 'wrong-service-closed' for item in wire) != 1:
            raise ValueError('Task105 required service-error connection closure absent')
        for session_version in ('002.5.0', '002.6'):
            rejected = [item for item in wire if item.get('label') == 'wrong-init-schema-' + session_version]
            if (len(rejected) != 1 or not rejected[0]['response'].startswith('-ERR Unsupported-Schema')
                    or 'does not match 002.6.0' not in rejected[0]['response']
                    or sum(item.get('label') == 'wrong-init-schema-closed-' + session_version for item in wire) != 1):
                raise ValueError('Task105 wrong OWP session release rejection/closure absent')
        other = cases.golden(release)
        other['MessageHeader']['SchemaVersion'] = '002.5.0'
        published_version = [item for item in wire if item.get('label') == 'other-release-payload-version']
        delivered_version = [item for item in wire if item.get('label') == 'other-release-version-delivery']
        if (len(published_version) != 1 or published_version[0]['command'] != cases.publication(other)
                or published_version[0]['response'] != '+OK' or len(delivered_version) != 1
                or not delivered_version[0]['response'].startswith('MSG 9 ')
                or json.loads(delivered_version[0]['response'].split(' ', 2)[2]) != {cases.MESSAGE: other}):
            raise ValueError('Task105 payload-version syntax versus session-identity boundary absent')
        server = text(prefix + 'server.stdout')
        if not all(marker in server for marker in ('schema loaded', 'route PUB message', 'shutdown complete')):
            raise ValueError('Task105 schema/routing/cleanup evidence absent')
    return dict(compiler=version, cells=6, releases=['26'], handlers=72, codec_rejected=6,
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))