"""Source-bound Task104 evidence verifier; expectations are not artifact-owned."""
import hashlib
import itertools
import json
import re
import importlib.util
from pathlib import Path

JOB = 'ada-real-uci-survivability-sleet'
PINS = {'25': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27'}
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
INCLUDES = {'25': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
            '26': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
ROOT = Path(__file__).resolve().parents[1]
BASELINE = {
    'programs.ads': 'a5d92d26a18c03999588818e68c5a8b3426ff1c780a7582bf21121727b4a0d16',
    'programs-oam.ads': '1f130f8317cb8454a3c1bde5115da9c692eef1450b8b3b0b8fb46df2517cda48',
    'programs-oam.adb': '5bf263058f24bd3b5ae27db0de59ee38cbd1f9a97510bf82b97cbbeb90924e5b',
    'service_codec.ads': 'ed7475f3d74fe3e91028f7c3d601ce2f4288393284debb6a723dbc32d6e6266d',
    'service_codec.adb': 'dd0fed491b40434fb4def8f4ecf4f404c8cf85d8d04648722666e6eaa2b3a235',
}



def verify_bundle(files, compiler, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [value for name, value in files.items() if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task104 evidence: ' + suffix)
        return matches[0]
    def text(suffix):
        return raw(suffix).decode('utf-8', errors='strict')
    def doc(suffix):
        return json.loads(text(suffix))
    if doc('integration-identity.json') != dict(sha=sha, tree=tree, run_id=str(run_id),
                                              run_attempt=str(attempt), compiler=compiler):
        raise ValueError('Task104 source/tree/run/attempt/compiler mismatch')
    inventory = doc('uci26-inventory.json')
    if (inventory.get('release') != '2.6' or inventory.get('global_element') != 'SurvivabilityRiskLevel'
            or inventory.get('payload_type') != 'uci:SurvivabilityRiskLevelMT'
            or inventory.get('exit') != 0 or inventory.get('compiler_qualified') is not False
            or inventory.get('transport_qualified') is not False
            or inventory.get('schema_sources') != {
                'UCI_MessageDefinitions_v2_6_0.xsd': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
                'UCI_SecurityMarkings_v2_6_0.xsd': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
            or 'selected type closure: 45\n' not in inventory.get('stdout', '')
            or 'codec renderable emitted declarations: 44/44\ncodec status: READY\n' not in inventory.get('stdout', '')):
        raise ValueError('Task104 separate UCI 2.6 static inventory missing/wrong boundary')
    if text('campaign/source.stdout').splitlines() != [sha, tree] or text('campaign/source-status.stdout').strip():
        raise ValueError('Task104 dirty/wrong source identity')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    lock_spec = importlib.util.spec_from_file_location('task104_lock_guard', ROOT / 'scripts/check-task087a.py')
    lock_guard = importlib.util.module_from_spec(lock_spec)
    lock_spec.loader.exec_module(lock_guard)
    lock = doc('campaign/toolchain-lock.json')
    try:
        lock_guard.validate_solution(lock,
                                     'gnat_external' if compiler == '14' else 'gnat_native')
    except RuntimeError as exc:
        raise ValueError('Task104 unqualified toolchain lock') from exc
    archive_pins = {'gprbuild': 'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b'}
    if compiler == '16':
        archive_pins['gnat_native'] = 'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703'
    for name, digest in archive_pins.items():
        releases = [s['release'] for s in lock['solution']['state'] if s.get('release', {}).get('name') == name]
        if not releases or any(digest not in r.get('origin', {}).get('hashes', []) for r in releases):
            raise ValueError('Task104 compiler/builder archive pin mismatch')
    if not text('campaign/gnat-version.stdout').startswith('GNAT ' + version + '\n'):
        raise ValueError('Task104 wrong compiler')
    if '26.0.0' not in text('campaign/gprbuild-version.stdout'):
        raise ValueError('Task104 wrong GPRbuild identity')
    if not re.fullmatch('[0-9a-f]{64}', text('campaign/generator.sha256').strip()):
        raise ValueError('Task104 generator identity missing')
    inputs = doc('campaign/input-sources.json')
    required_inputs = {'scripts/task104/qualify.py', 'scripts/task104/sleet.py', 'scripts/task104/cases.py',
                       'tests/ada-real-uci-survivability/survivability_consumer.adb', 'scripts/task104_evidence.py',
                       'scripts/task098/qualify.py', 'tests/ada-generated-codec/task092_real_survivability.adb', 'scripts/check-task092-real.py'}
    required_inputs |= {'tests/ada-real-uci-survivability/codec_probe.adb', 'scripts/check-task087a.py'}
    required_inputs |= {f'tests/fixtures/task104/uci{r}-service.yaml' for r in PINS}
    required_inputs |= {'scripts/task104/admission.py', 'tests/fixtures/service-generate/codec-ada-minimal.yaml'}
    required_inputs |= {f'tests/fixtures/service-generate/task092-real-survivability{r}.json' for r in PINS}
    if not required_inputs <= inputs.keys():
        raise ValueError('Task104 mandatory input sources absent')
    for path, digest in inputs.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('Task104 qualification inputs differ from exact source')
    result = doc('campaign/result.json')
    expected = set(itertools.product(('25',), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    actual = [(c['release'], c['policy'], c['optimization']) for c in result['cells']]
    if (result.get('completed') is not True or result.get('compiler') != version
            or result.get('real_sleet') is not True or result.get('real_uci') is not True
            or len(actual) != 6 or set(actual) != expected):
        raise ValueError('Task104 missing/duplicate/wrong release/policy/optimization cell')
    steps = result['steps']
    labels = [step['label'] for step in steps]
    intentional = {f'{r}-direction-{d}' for r in PINS for d in ('input', 'output')}
    if not labels or len(labels) != len(set(labels)) or any(s['exit'] != 0 for s in steps if s['label'] not in intentional):
        raise ValueError('Task104 failed/duplicate execution step')
    mandatory = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission'}
    mandatory |= {f'{name}-{step}' for name in ('aht', 'websocket-ada')
                  for step in ('clone', 'pin', 'identity', 'clean')}
    mandatory |= {f'{r}-{s}' for r in PINS for s in ('check', 'generate', 'original', 'model')}
    mandatory |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    mandatory |= {f'{r}-codec-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    mandatory |= {f'{r}-independent-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    if not mandatory <= set(labels):
        raise ValueError('Task104 mandatory execution step absent')
    for label in labels:
        observed = text('campaign/' + label + '.exit').strip()
        recorded = next(s['exit'] for s in steps if s['label'] == label)
        if observed != str(recorded) or (label not in intentional and observed != '0') or (label in intentional and observed == '0'):
            raise ValueError('Task104 execution exit mismatch')
    for name, revision in [('aht', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
                           ('websocket-ada', '765d9cd612838eb00645037977c8cc9af7171c3e')]:
        if text(f'campaign/{name}-identity.stdout').strip() != revision or text(f'campaign/{name}-clean.stdout').strip():
            raise ValueError('Task104 transport dependency source pin/cleanliness mismatch')
    if not intentional <= set(labels):
        raise ValueError('Task104 static endpoint direction probes absent')
    for release in PINS:
        for direction, forbidden in [('input', 'Publisher'), ('output', 'Subscriber')]:
            if f'"{forbidden}" not declared' not in text(f'campaign/{release}-direction-{direction}.stderr'):
                raise ValueError('Task104 endpoint probe failed for wrong reason')
        admission = doc(f'campaign/admission/phase-a-{release}/result.json')
        if any(admission.get(k) is not True for k in (
                'schema_admitted', 'owp_negotiated', 'initialized', 'subscribed',
                'admitted', 'delivered', 'unsubscribed', 'cleanup', 'socket_closed', 'websocket_closed')) or (
                admission.get('publication_response') != '+OK' or admission.get('server_exit') != 0
                or admission.get('status') != 'PASS' or admission.get('generated_ada_decoder_executed') is not False):
            raise ValueError('Task104 independent broker admission/cleanup missing')
        prefix = f'campaign/admission/phase-a-{release}/'
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE] or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task104 independent admission identity/exit mismatch')
    log = text('qualify.log').splitlines()
    if (sum(line.startswith('TASK104_CELL_COMPLETE ') for line in log) != 6
            or f'TASK104_QUALIFICATION_COMPLETE {version} 6' not in log):
        raise ValueError('Task104 completion marker absent')
    sources = doc('campaign/sources.json')
    if not sources:
        raise ValueError('Task104 source hashes absent')
    for path, digest in sources.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task104 generated/fixture provenance mismatch')
    for release in PINS:
        if not raw(f'campaign/{release}-original-generated/service_api.ads'):
            raise ValueError('Task104 original generated API absent')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb'):
            if hashlib.sha256(raw(f'campaign/{release}-generated/{name}')).hexdigest() != BASELINE[name]:
                raise ValueError('Task104 generated model/codec differs from historical Task092 baseline')
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-original-generated/{name}'):
                raise ValueError('Task104 model/codec original selection byte parity mismatch')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-model/{name}'):
                raise ValueError('Task104 model/API codec opt-in byte parity mismatch')
        for policy, optimization in itertools.product(('Normal', 'Check', 'Ignore'), ('0', '2')):
            for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads', 'service_codec.ads', 'service_codec.adb'):
                if raw(f'campaign/uci{release}-{policy}-O{optimization}/generated/{name}') != raw(f'campaign/{release}-generated/{name}'):
                    raise ValueError('Task104 compiled cell generated source differs')
    for release, digest in PINS.items():
        path = f'uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task104 wrong UCI release/root')
        include = f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw(include)).hexdigest() != INCLUDES[release]:
            raise ValueError('Task104 authentic schema include mismatch')
        for kind, pin in [('MessageDefinitions', digest), ('SecurityMarkings', INCLUDES[release])]:
            if hashlib.sha256(raw(f'campaign/admission/phase-a-{release}/schema/UCI_{kind}_v2_{release[-1]}_0.xsd')).hexdigest() != pin:
                raise ValueError('Task104 admission authentic schema hash mismatch')
        report = text(f'campaign/{release}-check.stdout')
        selected = 45
        if f'selected type closure: {selected}\n' not in report or f'codec renderable emitted declarations: {selected-1}/{selected-1}\ncodec status: READY\n' not in report:
            raise ValueError('Task104 actual SurvivabilityRiskLevel readiness absent')
        for policy in ('Normal', 'Check', 'Ignore'):
            output = text(f'campaign/{release}-codec-{policy}-execute.stdout')
            if output != 'TASK092_REAL_PASS checks= 8\n':
                raise ValueError('Task104 mandatory Task092 real SurvivabilityRiskLevel codec corpus incomplete')
            independent = text(f'campaign/{release}-independent-{policy}-execute.stdout').splitlines()
            if (len(independent) != 19
                    or independent[-1] != 'TASK104_INDEPENDENT_CODEC_COMPLETE rejected=14 model_constraints=4'
                    or sum(line.startswith('TASK104_CODEC_REJECTED ') for line in independent) != 14
                    or sum(line.startswith('TASK104_MODEL_CONSTRAINT_REJECTED ') for line in independent) != 4):
                raise ValueError('Task104 independent codec/model constraints incomplete')
    spec = importlib.util.spec_from_file_location('task104_independent_cases', ROOT / 'scripts/task104/cases.py')
    cases = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cases)
    for release in PINS:
        prefix = f'campaign/admission/phase-a-{release}/'
        for policy in ('Normal', 'Check', 'Ignore'):
            lines = text(f'campaign/{release}-independent-{policy}-execute.stdout').splitlines()
            reasons = ['TASK104_CODEC_REJECTED ' + cases.CODEC_REASONS[label]
                       for label, _ in cases.invalid_documents(release) + cases.decoder_invalid(release)]
            if lines[:14] != reasons:
                raise ValueError('Task104 independent codec rejected for wrong reason')
            if lines[14:18] != ['TASK104_MODEL_CONSTRAINT_REJECTED  ' + str(n) + ' ' + reason for n, reason in enumerate((
                    'invalid UUID', 'invalid schema version',
                    'SurvivabilityRiskSettingType_RiskLevel_Sequence requires 1 .. 5 values',
                    'SurvivabilityRiskLevelMDT_RiskSetting_Sequence requires 1 .. 4 values'), 1)]:
                raise ValueError('Task104 independent model constraint coverage incomplete')
        if doc(prefix + f'task092-real-survivability{release}.json') != cases.golden(release):
            raise ValueError('Task104 admission golden differs')
        golden_bytes = raw(prefix + f'task092-real-survivability{release}.json')
        if hashlib.sha256(golden_bytes).hexdigest() != '996ac91fa9eb177d14a6fc3fd7169e781a2bd206b62a410dd908f0d0750f2349':
            raise ValueError('Task104 original Task092 fixture bytes changed')
        admission_wire = doc(prefix + 'wire.json')
        deliveries = [item['receive'] for item in admission_wire if item.get('receive', '').startswith('MSG 1 ')]
        if len(deliveries) != 1 or json.loads(deliveries[0].split(' ', 2)[2]) != {'SurvivabilityRiskLevel': cases.golden(release)}:
            raise ValueError('Task104 admission real broker delivery missing')
    for release, policy, optimization in expected:
        cell = f'uci{release}-{policy}-O{optimization}'
        prefix = f'campaign/{cell}/sleet/'
        evidence = doc(prefix + 'result.json')
        if evidence != dict(completed=True, release=release, revision=REV, tree=TREE, handlers=12,
                            witness_handlers=1, decode_rejected=1, published=6, broker_rejected=17,
                            consumer_exit=0, server_exit=0, cleanup=True):
            raise ValueError('Task104 real-Sleet handler/count/cleanup mismatch')
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE]:
            raise ValueError('Task104 Sleet source mismatch')
        if not re.fullmatch('[0-9a-f]{64}', text(prefix + 'sleet-binary.sha256').strip()):
            raise ValueError('Task104 Sleet binary identity missing')
        if text(prefix + 'consumer.exit').strip() != '0' or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task104 consumer/server failed')
        output = text(prefix + 'consumer.stdout').splitlines()
        if output.count('TASK104_EVENT COMMAND_ACCEPTED +OK') != 12:
            raise ValueError('Task104 generated publications/subscriptions not broker accepted')
        for marker in ['TASK104_ADA_REAL_SURVIVABILITY_COMPLETE handlers= 12 rejected= 1',
                       'TASK104_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1',
                       'TASK104_CLEANUP_COMPLETE', 'TASK104_DECODE_RECOVERY_COMPLETE',
                       'TASK104_UNKNOWN_BINDING_SID_REJECTED', 'TASK104_UPGRADE_OWP_COMPLETE',
                       'TASK104_INFO_VALIDATED', 'TASK104_SUB_ACCEPTED SurvivabilityRiskLevel task104-survivability']:
            if output.count(marker) != 1:
                raise ValueError('Task104 interoperability marker missing/duplicate')
        if (sum(s.startswith('TASK104_TYPED_HANDLER ') for s in output) != 12
                or sum(s.startswith('TASK104_WITNESS_HANDLER ') for s in output) != 1
                or sum(s.startswith('TASK104_PUBLISHED ') for s in output) != 6
                or sum(s.startswith('TASK104_EVENT DECODE_ERROR ') for s in output) != 1):
            raise ValueError('Task104 actual handler/codec/publication count mismatch')
        model_checks = [s for s in output if s.startswith('TASK104_MODEL_CHECK_COMPLETE case=')]
        if model_checks != ['TASK104_MODEL_CHECK_COMPLETE case= ' + str(n) for n in (0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 0)]:
            raise ValueError('Task104 decoded model discriminant/value/order observations absent')
        published = [json.loads(s.removeprefix('TASK104_PUBLISHED ')) for s in output if s.startswith('TASK104_PUBLISHED ')]
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
                    risk_observations.append(f'TASK104_RISK_OBSERVED setting= {index}'
                        f' priority={setting["RiskPrioritization"]} allowed={allowed} levels= {len(levels)}')
        if [s for s in output if s.startswith('TASK104_RISK_OBSERVED ')] != risk_observations:
            raise ValueError('Task104 nested generated risk observations absent/wrong order/value')
        if published != [expected for _, _, expected in positive]:
            raise ValueError('Task104 publications differ from independent real SurvivabilityRiskLevel oracle')
        received = [json.loads(s.removeprefix('TASK104_WIRE_RECEIVED ')) for s in output if s.startswith('TASK104_WIRE_RECEIVED ')]
        expected_wire = [{'SurvivabilityRiskLevel': doc} for _, doc in cases.decoder_invalid(release)]
        expected_wire += [{cases.MESSAGE: positive[0][1]}, {cases.MESSAGE: positive[0][2]}]
        for _, value, canonical in positive[1:]:
            expected_wire += [{cases.MESSAGE: value}, {cases.MESSAGE: canonical}]
        expected_wire += [{cases.MESSAGE: positive[0][1]}]
        if received != expected_wire:
            raise ValueError('Task104 broker delivery wire identity/value/order mismatch')
        errors = [s.removeprefix('TASK104_EVENT DECODE_ERROR ') for s in output if s.startswith('TASK104_EVENT DECODE_ERROR ')]
        if errors != ['invalid named DateTime']:
            raise ValueError('Task104 decoder rejection diagnostic/stage mismatch')
        wire = doc(prefix + 'wire.json')
        for label, command, code, detail in cases.broker_cases(release):
            matches = [item for item in wire if item.get('label') == label]
            if (len(matches) != 1 or matches[0].get('command') != command
                    or not matches[0]['response'].startswith(code) or detail not in matches[0]['response']):
                raise ValueError('Task104 expected broker error absent/wrong reason: ' + label)
            if label in cases.BROKER_DETAILS and cases.BROKER_DETAILS[label][1] not in matches[0]['response']:
                raise ValueError('Task104 broker rejection path/detail mismatch: ' + label)
        for label, value in cases.decoder_invalid(release):
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or matches[0]['response'] != '+OK' or matches[0]['command'] != cases.publication(value):
                raise ValueError('Task104 real broker/codec stage separation absent')
        recovery = [item for item in wire if item.get('label') == 'same-subscription-recovery']
        if len(recovery) != 1 or recovery[0]['response'] != '+OK' or recovery[0]['command'] != cases.publication(positive[0][1]):
            raise ValueError('Task104 same subscription valid recovery absent')
        for label, value, _ in positive[1:]:
            admission = [item for item in wire if item.get('label') == 'external-' + label]
            if len(admission) != 1 or admission[0]['response'] != '+OK' or admission[0]['command'] != cases.publication(value):
                raise ValueError('Task104 independent broker/typed variant observation absent')
        if sum(item.get('label') == 'wrong-service-closed' for item in wire) != 1:
            raise ValueError('Task104 required service-error connection closure absent')
        server = text(prefix + 'server.stdout')
        if not all(marker in server for marker in ('schema loaded', 'route PUB message', 'shutdown complete')):
            raise ValueError('Task104 schema/routing/cleanup evidence absent')
    return dict(compiler=version, cells=6, releases=['25'], handlers=72, codec_rejected=6,
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))