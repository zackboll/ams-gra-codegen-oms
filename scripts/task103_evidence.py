"""Source-bound Task103 evidence verifier; expectations are not artifact-owned."""
import hashlib
import itertools
import json
import re
import importlib.util
from pathlib import Path

JOB = 'ada-real-uci-rdma-sleet'
PINS = {'25': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
        '26': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
INCLUDES = {'25': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
            '26': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
ROOT = Path(__file__).resolve().parents[1]



def verify_bundle(files, compiler, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [value for name, value in files.items() if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task103 evidence: ' + suffix)
        return matches[0]
    def text(suffix):
        return raw(suffix).decode('utf-8', errors='strict')
    def doc(suffix):
        return json.loads(text(suffix))
    if doc('integration-identity.json') != dict(sha=sha, tree=tree, run_id=str(run_id),
                                              run_attempt=str(attempt), compiler=compiler):
        raise ValueError('Task103 source/tree/run/attempt/compiler mismatch')
    if text('campaign/source.stdout').splitlines() != [sha, tree] or text('campaign/source-status.stdout').strip():
        raise ValueError('Task103 dirty/wrong source identity')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    lock_spec = importlib.util.spec_from_file_location('task103_lock_guard', ROOT / 'scripts/check-task087a.py')
    lock_guard = importlib.util.module_from_spec(lock_spec)
    lock_spec.loader.exec_module(lock_guard)
    lock = doc('campaign/toolchain-lock.json')
    try:
        lock_guard.validate_solution(lock,
                                     'gnat_external' if compiler == '14' else 'gnat_native')
    except RuntimeError as exc:
        raise ValueError('Task103 unqualified toolchain lock') from exc
    archive_pins = {'gprbuild': 'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b'}
    if compiler == '16':
        archive_pins['gnat_native'] = 'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703'
    for name, digest in archive_pins.items():
        releases = [s['release'] for s in lock['solution']['state'] if s.get('release', {}).get('name') == name]
        if not releases or any(digest not in r.get('origin', {}).get('hashes', []) for r in releases):
            raise ValueError('Task103 compiler/builder archive pin mismatch')
    if not text('campaign/gnat-version.stdout').startswith('GNAT ' + version + '\n'):
        raise ValueError('Task103 wrong compiler')
    if '26.0.0' not in text('campaign/gprbuild-version.stdout'):
        raise ValueError('Task103 wrong GPRbuild identity')
    if not re.fullmatch('[0-9a-f]{64}', text('campaign/generator.sha256').strip()):
        raise ValueError('Task103 generator identity missing')
    inputs = doc('campaign/input-sources.json')
    required_inputs = {'scripts/task103/qualify.py', 'scripts/task103/sleet.py', 'scripts/task103/cases.py',
                       'tests/ada-real-uci-rdma/rdma_consumer.adb', 'scripts/task103_evidence.py',
                       'scripts/task098/qualify.py', 'tests/ada-generated-codec/task092_real_rdma.adb', 'scripts/check-task092-real.py'}
    required_inputs |= {'tests/ada-real-uci-rdma/codec_probe.adb', 'scripts/check-task087a.py'}
    required_inputs |= {f'tests/fixtures/task103/uci{r}-service.yaml' for r in PINS}
    required_inputs |= {'scripts/task103/admission.py', 'tests/fixtures/service-generate/codec-ada-minimal.yaml'}
    required_inputs |= {f'tests/fixtures/service-generate/task092-real-rdma{r}.json' for r in PINS}
    if not required_inputs <= inputs.keys():
        raise ValueError('Task103 mandatory input sources absent')
    for path, digest in inputs.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('Task103 qualification inputs differ from exact source')
    result = doc('campaign/result.json')
    expected = set(itertools.product(('25', '26'), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    actual = [(c['release'], c['policy'], c['optimization']) for c in result['cells']]
    if (result.get('completed') is not True or result.get('compiler') != version
            or result.get('real_sleet') is not True or result.get('real_uci') is not True
            or len(actual) != 12 or set(actual) != expected):
        raise ValueError('Task103 missing/duplicate/wrong release/policy/optimization cell')
    steps = result['steps']
    labels = [step['label'] for step in steps]
    intentional = {f'{r}-direction-{d}' for r in PINS for d in ('input', 'output')}
    if not labels or len(labels) != len(set(labels)) or any(s['exit'] != 0 for s in steps if s['label'] not in intentional):
        raise ValueError('Task103 failed/duplicate execution step')
    mandatory = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission'}
    mandatory |= {f'{name}-{step}' for name in ('aht', 'websocket-ada')
                  for step in ('clone', 'pin', 'identity', 'clean')}
    mandatory |= {f'{r}-{s}' for r in PINS for s in ('check', 'generate', 'original', 'model')}
    mandatory |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    mandatory |= {f'{r}-codec-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    mandatory |= {f'{r}-independent-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    if not mandatory <= set(labels):
        raise ValueError('Task103 mandatory execution step absent')
    for label in labels:
        observed = text('campaign/' + label + '.exit').strip()
        recorded = next(s['exit'] for s in steps if s['label'] == label)
        if observed != str(recorded) or (label not in intentional and observed != '0') or (label in intentional and observed == '0'):
            raise ValueError('Task103 execution exit mismatch')
    for name, revision in [('aht', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
                           ('websocket-ada', '765d9cd612838eb00645037977c8cc9af7171c3e')]:
        if text(f'campaign/{name}-identity.stdout').strip() != revision or text(f'campaign/{name}-clean.stdout').strip():
            raise ValueError('Task103 transport dependency source pin/cleanliness mismatch')
    if not intentional <= set(labels):
        raise ValueError('Task103 static endpoint direction probes absent')
    for release in PINS:
        for direction, forbidden in [('input', 'Publisher'), ('output', 'Subscriber')]:
            if f'"{forbidden}" not declared' not in text(f'campaign/{release}-direction-{direction}.stderr'):
                raise ValueError('Task103 endpoint probe failed for wrong reason')
        admission = doc(f'campaign/admission/phase-a-{release}/result.json')
        if any(admission.get(k) is not True for k in (
                'schema_admitted', 'owp_negotiated', 'initialized', 'subscribed',
                'admitted', 'delivered', 'unsubscribed', 'cleanup', 'socket_closed', 'websocket_closed')) or (
                admission.get('publication_response') != '+OK' or admission.get('server_exit') != 0
                or admission.get('status') != 'PASS' or admission.get('generated_ada_decoder_executed') is not False):
            raise ValueError('Task103 independent broker admission/cleanup missing')
        prefix = f'campaign/admission/phase-a-{release}/'
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE] or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task103 independent admission identity/exit mismatch')
    log = text('qualify.log').splitlines()
    if (sum(line.startswith('TASK103_CELL_COMPLETE ') for line in log) != 12
            or f'TASK103_QUALIFICATION_COMPLETE {version} 12' not in log):
        raise ValueError('Task103 completion marker absent')
    sources = doc('campaign/sources.json')
    if not sources:
        raise ValueError('Task103 source hashes absent')
    for path, digest in sources.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task103 generated/fixture provenance mismatch')
    for release in PINS:
        if not raw(f'campaign/{release}-original-generated/service_api.ads'):
            raise ValueError('Task103 original generated API absent')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-original-generated/{name}'):
                raise ValueError('Task103 model/codec original selection byte parity mismatch')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-model/{name}'):
                raise ValueError('Task103 model/API codec opt-in byte parity mismatch')
        for policy, optimization in itertools.product(('Normal', 'Check', 'Ignore'), ('0', '2')):
            for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads', 'service_codec.ads', 'service_codec.adb'):
                if raw(f'campaign/uci{release}-{policy}-O{optimization}/generated/{name}') != raw(f'campaign/{release}-generated/{name}'):
                    raise ValueError('Task103 compiled cell generated source differs')
    for release, digest in PINS.items():
        path = f'uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task103 wrong UCI release/root')
        include = f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw(include)).hexdigest() != INCLUDES[release]:
            raise ValueError('Task103 authentic schema include mismatch')
        for kind, pin in [('MessageDefinitions', digest), ('SecurityMarkings', INCLUDES[release])]:
            if hashlib.sha256(raw(f'campaign/admission/phase-a-{release}/schema/UCI_{kind}_v2_{release[-1]}_0.xsd')).hexdigest() != pin:
                raise ValueError('Task103 admission authentic schema hash mismatch')
        report = text(f'campaign/{release}-check.stdout')
        selected = 50
        if f'selected type closure: {selected}\n' not in report or f'codec renderable emitted declarations: {selected-1}/{selected-1}\ncodec status: READY\n' not in report:
            raise ValueError('Task103 actual RDMA_InitializeSetup readiness absent')
        for policy in ('Normal', 'Check', 'Ignore'):
            output = text(f'campaign/{release}-codec-{policy}-execute.stdout')
            if output != 'TASK092_REAL_PASS checks= 12\n':
                raise ValueError('Task103 mandatory Task092 real RDMA_InitializeSetup codec corpus incomplete')
            independent = text(f'campaign/{release}-independent-{policy}-execute.stdout').splitlines()
            if (len(independent) != 14
                    or independent[-1] != 'TASK103_INDEPENDENT_CODEC_COMPLETE rejected=9 model_constraints=4'
                    or sum(line.startswith('TASK103_CODEC_REJECTED ') for line in independent) != 9
                    or sum(line.startswith('TASK103_MODEL_CONSTRAINT_REJECTED ') for line in independent) != 4):
                raise ValueError('Task103 independent codec/model constraints incomplete')
    spec = importlib.util.spec_from_file_location('task103_independent_cases', ROOT / 'scripts/task103/cases.py')
    cases = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cases)
    for release in PINS:
        prefix = f'campaign/admission/phase-a-{release}/'
        for policy in ('Normal', 'Check', 'Ignore'):
            lines = text(f'campaign/{release}-independent-{policy}-execute.stdout').splitlines()
            reasons = ['TASK103_CODEC_REJECTED ' + cases.CODEC_REASONS[label]
                       for label, _ in cases.invalid_documents(release) + cases.decoder_invalid(release)]
            if lines[:9] != reasons:
                raise ValueError('Task103 independent codec rejected for wrong reason')
            if lines[9:13] != ['TASK103_MODEL_CONSTRAINT_REJECTED  ' + str(n) for n in range(1, 5)]:
                raise ValueError('Task103 independent model constraint coverage incomplete')
        if doc(prefix + f'task092-real-rdma{release}.json') != cases.golden(release):
            raise ValueError('Task103 admission golden differs')
        admission_wire = doc(prefix + 'wire.json')
        deliveries = [item['receive'] for item in admission_wire if item.get('receive', '').startswith('MSG 1 ')]
        if len(deliveries) != 1 or json.loads(deliveries[0].split(' ', 2)[2]) != {'RDMA_InitializeSetup': cases.golden(release)}:
            raise ValueError('Task103 admission real broker delivery missing')
    for release, policy, optimization in expected:
        cell = f'uci{release}-{policy}-O{optimization}'
        prefix = f'campaign/{cell}/sleet/'
        evidence = doc(prefix + 'result.json')
        if evidence != dict(completed=True, release=release, revision=REV, tree=TREE, handlers=7,
                            witness_handlers=1, decode_rejected=1, published=6, broker_rejected=12,
                            consumer_exit=0, server_exit=0, cleanup=True):
            raise ValueError('Task103 real-Sleet handler/count/cleanup mismatch')
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE]:
            raise ValueError('Task103 Sleet source mismatch')
        if not re.fullmatch('[0-9a-f]{64}', text(prefix + 'sleet-binary.sha256').strip()):
            raise ValueError('Task103 Sleet binary identity missing')
        if text(prefix + 'consumer.exit').strip() != '0' or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task103 consumer/server failed')
        output = text(prefix + 'consumer.stdout').splitlines()
        if output.count('TASK103_EVENT COMMAND_ACCEPTED +OK') != 12:
            raise ValueError('Task103 generated publications/subscriptions not broker accepted')
        for marker in ['TASK103_ADA_REAL_RDMA_COMPLETE handlers= 7 rejected= 1',
                       'TASK103_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1',
                       'TASK103_CLEANUP_COMPLETE', 'TASK103_DECODE_RECOVERY_COMPLETE',
                       'TASK103_UNKNOWN_BINDING_SID_REJECTED', 'TASK103_UPGRADE_OWP_COMPLETE',
                       'TASK103_INFO_VALIDATED', 'TASK103_SUB_ACCEPTED RDMA_InitializeSetup task103-rdma']:
            if output.count(marker) != 1:
                raise ValueError('Task103 interoperability marker missing/duplicate')
        if (sum(s.startswith('TASK103_TYPED_HANDLER ') for s in output) != 7
                or sum(s.startswith('TASK103_WITNESS_HANDLER ') for s in output) != 1
                or sum(s.startswith('TASK103_PUBLISHED ') for s in output) != 6
                or sum(s.startswith('TASK103_EVENT DECODE_ERROR ') for s in output) != 1):
            raise ValueError('Task103 actual handler/codec/publication count mismatch')
        model_checks = [s for s in output if s.startswith('TASK103_MODEL_CHECK_COMPLETE case=')]
        if model_checks != ['TASK103_MODEL_CHECK_COMPLETE case= ' + str(n) for n in (0, 0, 1, 2, 3, 4, 5, 0)]:
            raise ValueError('Task103 decoded model discriminant/value/order observations absent')
        published = [json.loads(s.removeprefix('TASK103_PUBLISHED ')) for s in output if s.startswith('TASK103_PUBLISHED ')]
        positive = cases.positives(release)
        if published != [expected for _, _, expected in positive]:
            raise ValueError('Task103 publications differ from independent real RDMA_InitializeSetup oracle')
        received = [json.loads(s.removeprefix('TASK103_WIRE_RECEIVED ')) for s in output if s.startswith('TASK103_WIRE_RECEIVED ')]
        expected_wire = [{'RDMA_InitializeSetup': doc} for _, doc in cases.decoder_invalid(release)]
        expected_wire += [{'RDMA_InitializeSetup': positive[0][1]}] + [{'RDMA_InitializeSetup': expected} for _, _, expected in positive] + [{'RDMA_InitializeSetup': positive[0][1]}]
        if received != expected_wire:
            raise ValueError('Task103 broker delivery wire identity/value/order mismatch')
        errors = [s.removeprefix('TASK103_EVENT DECODE_ERROR ') for s in output if s.startswith('TASK103_EVENT DECODE_ERROR ')]
        if errors != ['invalid named DateTime']:
            raise ValueError('Task103 decoder rejection diagnostic/stage mismatch')
        wire = doc(prefix + 'wire.json')
        for label, command, code, detail in cases.broker_cases(release):
            matches = [item for item in wire if item.get('label') == label]
            if (len(matches) != 1 or matches[0].get('command') != command
                    or not matches[0]['response'].startswith(code) or detail not in matches[0]['response']):
                raise ValueError('Task103 expected broker error absent/wrong reason: ' + label)
            if label in cases.BROKER_DETAILS and cases.BROKER_DETAILS[label][1] not in matches[0]['response']:
                raise ValueError('Task103 broker rejection path/detail mismatch: ' + label)
        for label, value in cases.decoder_invalid(release):
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or matches[0]['response'] != '+OK' or matches[0]['command'] != cases.publication(value):
                raise ValueError('Task103 real broker/codec stage separation absent')
        recovery = [item for item in wire if item.get('label') == 'same-subscription-recovery']
        if len(recovery) != 1 or recovery[0]['response'] != '+OK' or recovery[0]['command'] != cases.publication(positive[0][1]):
            raise ValueError('Task103 same subscription valid recovery absent')
        for label, value, _ in positive[-2:]:
            admission = [item for item in wire if item.get('label') == 'broker-' + label]
            delivery = [item for item in wire if item.get('label') == 'broker-' + label + '-delivery']
            if (len(admission) != 1 or admission[0]['response'] != '+OK' or len(delivery) != 1
                    or not delivery[0]['response'].startswith('MSG 99 ')
                    or json.loads(delivery[0]['response'].split(' ', 2)[2]) != {cases.MESSAGE: value}):
                raise ValueError('Task103 independent broker endpoint observation absent')
        if sum(item.get('label') == 'wrong-service-closed' for item in wire) != 1:
            raise ValueError('Task103 required service-error connection closure absent')
        server = text(prefix + 'server.stdout')
        if not all(marker in server for marker in ('schema loaded', 'route PUB message', 'shutdown complete')):
            raise ValueError('Task103 schema/routing/cleanup evidence absent')
    return dict(compiler=version, cells=12, releases=['25', '26'], handlers=84, codec_rejected=12,
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))