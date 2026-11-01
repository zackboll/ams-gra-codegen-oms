"""Source-bound Task102 evidence verifier; expectations are not artifact-owned."""
import hashlib
import itertools
import json
import re
import importlib.util
from pathlib import Path

JOB = 'ada-real-uci-log-sleet'
PINS = {'25': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
        '26': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
INCLUDES = {'25': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
            '26': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
ROOT = Path(__file__).resolve().parents[1]
BROKER_ERRORS = {
    **{kind + '-data': ('Invalid-Message', 'TypeMismatch') for kind in ('null', 'number', 'array', 'object')},
    'array-data': ('Invalid-Message', 'OccurrenceMismatch'),
    'choice-two-arms': ('Invalid-Message', 'ChoiceViolation'),
    'missing-header': ('Invalid-Message', 'MissingRequiredMember'),
    'wrong-global': ('Invalid-Message', 'UnknownGlobalElement'),
    'wrong-topic': ('Illegal-State', 'not authorized'),
    'unknown-sid': ('Illegal-State', 'unknown subscription id 987'),
    'malformed-document': ('Invalid-Message', 'invalid JSON payload'),
}


def verify_bundle(files, compiler, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [value for name, value in files.items() if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task102 evidence: ' + suffix)
        return matches[0]
    def text(suffix):
        return raw(suffix).decode('utf-8', errors='strict')
    def doc(suffix):
        return json.loads(text(suffix))
    if doc('integration-identity.json') != dict(sha=sha, tree=tree, run_id=str(run_id),
                                              run_attempt=str(attempt), compiler=compiler):
        raise ValueError('Task102 source/tree/run/attempt/compiler mismatch')
    if text('campaign/source.stdout').splitlines() != [sha, tree] or text('campaign/source-status.stdout').strip():
        raise ValueError('Task102 dirty/wrong source identity')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    if not text('campaign/gnat-version.stdout').startswith('GNAT ' + version + '\n'):
        raise ValueError('Task102 wrong compiler')
    if not re.fullmatch('[0-9a-f]{64}', text('campaign/generator.sha256').strip()):
        raise ValueError('Task102 generator identity missing')
    inputs = doc('campaign/input-sources.json')
    required_inputs = {'scripts/task102/qualify.py', 'scripts/task102/sleet.py', 'scripts/task102/cases.py',
                       'tests/ada-real-uci-log/log_consumer.adb', 'scripts/task102_evidence.py',
                       'scripts/task098/qualify.py', 'tests/ada-generated-codec/hexbinary_log_consumer.adb', 'scripts/check-task093-log.py'}
    required_inputs |= {f'tests/fixtures/task102/uci{r}-service.yaml' for r in PINS}
    required_inputs |= {'scripts/task102/admission.py', 'tests/fixtures/service-generate/real-duration-log.yaml'}
    required_inputs |= {'scripts/task102/constrained.py', 'tests/ada-real-uci-log/hash_probe.adb'}
    required_inputs |= {f'tests/fixtures/service-generate/task093-real-log{r}.json' for r in PINS}
    if not required_inputs <= inputs.keys():
        raise ValueError('Task102 mandatory input sources absent')
    for path, digest in inputs.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('Task102 qualification inputs differ from exact source')
    result = doc('campaign/result.json')
    expected = set(itertools.product(('25', '26'), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    actual = [(c['release'], c['policy'], c['optimization']) for c in result['cells']]
    if (result.get('completed') is not True or result.get('compiler') != version
            or result.get('real_sleet') is not True or result.get('real_uci') is not True
            or len(actual) != 12 or set(actual) != expected):
        raise ValueError('Task102 missing/duplicate/wrong release/policy/optimization cell')
    steps = result['steps']
    labels = [step['label'] for step in steps]
    intentional = {f'{r}-direction-{d}' for r in PINS for d in ('input', 'output')}
    if not labels or len(labels) != len(set(labels)) or any(s['exit'] != 0 for s in steps if s['label'] not in intentional):
        raise ValueError('Task102 failed/duplicate execution step')
    mandatory = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv', 'admission'}
    mandatory |= {f'{r}-{s}' for r in PINS for s in ('check', 'generate', 'original', 'model')}
    mandatory |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    mandatory |= {f'{r}-codec-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    mandatory |= {f'{r}-constrained-ada' for r in PINS}
    mandatory |= {f'{r}-hash-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    if not mandatory <= set(labels):
        raise ValueError('Task102 mandatory execution step absent')
    for label in labels:
        observed = text('campaign/' + label + '.exit').strip()
        recorded = next(s['exit'] for s in steps if s['label'] == label)
        if observed != str(recorded) or (label not in intentional and observed != '0') or (label in intentional and observed == '0'):
            raise ValueError('Task102 execution exit mismatch')
    if not intentional <= set(labels):
        raise ValueError('Task102 static endpoint direction probes absent')
    for release in PINS:
        for direction, forbidden in [('input', 'Publisher'), ('output', 'Subscriber')]:
            if f'"{forbidden}" not declared' not in text(f'campaign/{release}-direction-{direction}.stderr'):
                raise ValueError('Task102 endpoint probe failed for wrong reason')
        admission = doc(f'campaign/admission/phase-a-{release}/result.json')
        if admission != dict(release=release, admitted=True, delivered=True, cleanup=True,
                             publication_response='+OK', server_exit=0, socket_closed=True, status='PASS'):
            raise ValueError('Task102 independent broker admission/cleanup missing')
        prefix = f'campaign/admission/phase-a-{release}/'
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE] or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task102 independent admission identity/exit mismatch')
        constrained = f'campaign/{release}-constrained-broker/'
        if doc(constrained + 'result.json') != dict(
                scope='unchanged extracted authentic Binary declarations; synthetic envelope, NOT Log',
                ada_accepts_octets=32, broker_accepts_octets=16, broker_rejects_octets=[32, 31, 33],
                server_exit=0, cleanup=True):
            raise ValueError('Task102 separate constrained binary observation absent')
        constrained_wire = doc(constrained + 'wire.json')
        for octets in (32, 16, 31, 33):
            entries = [item for item in constrained_wire if item.get('label') == f'hash-{octets}-octets']
            if len(entries) != 1 or not entries[0]['response'].startswith('+OK' if octets == 16 else '-ERR Invalid-Message'):
                raise ValueError('Task102 broker constrained length stage mismatch')
            value = json.loads(entries[0]['command'].removeprefix('PUB task102-constrained '))
            if value != {'BinaryNotice': {'Data': '0080FF', 'SHA_2_Hash': '0080FF' + '00' * (octets - 3)}}:
                raise ValueError('Task102 broker constrained length input mismatch')
        if text(constrained + 'server.exit').strip() != '0' or 'shutdown complete' not in text(constrained + 'server.stdout'):
            raise ValueError('Task102 constrained broker cleanup absent')
        for policy in ('Normal', 'Check', 'Ignore'):
            if text(f'campaign/{release}-hash-{policy}-execute.stdout') != 'TASK102_HASH_OCTETS_COMPLETE accepted=1 rejected=3\n':
                raise ValueError('Task102 Ada constrained octet enforcement absent')
        extracted = doc(f'campaign/{release}-constrained-ada/qualification.json')
        if extracted != dict(compiler=version, uci='2.' + release[-1], policies=['normal', 'Check', 'Ignore'],
                             checks_per_cell=14, status='PASS', full_service_qualified=False):
            raise ValueError('Task102 Task093 extracted carrier qualification incomplete')
        for policy in ('normal', 'Check', 'Ignore'):
            if text(f'campaign/{release}-constrained-ada/{policy}-run.log') != 'TASK093_REAL_COMPLETE cases= 14\n':
                raise ValueError('Task102 extracted Ada consumer completion absent')
        extracted_schema = text(f'campaign/{release}-constrained-ada/real-binary.xsd')
        broker_schema = text(constrained + 'extracted-binary.xsd')
        if broker_schema.replace('\r\n', '\n') != extracted_schema.replace('\r\n', '\n').replace('<xs:schema ', f'<xs:schema version="002.{release[-1]}.0" ', 1):
            raise ValueError('Task102 constrained broker changed extracted declarations/facets')
        authentic = text(f'campaign/uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd')
        for name in ('SHA_2_256_HashType', *(['HexBinaryType'] if release == '26' else [])):
            match = re.search(r'<xs:simpleType\b[^>]*name="' + name + r'"[^>]*>.*?</xs:simpleType>', authentic, re.S)
            if match is None or match.group().replace('\r\n', '\n') not in extracted_schema.replace('\r\n', '\n'):
                raise ValueError('Task102 constrained authentic declaration altered')
    log = text('qualify.log').splitlines()
    if (sum(line.startswith('TASK102_CELL_COMPLETE ') for line in log) != 12
            or f'TASK102_QUALIFICATION_COMPLETE {version} 12' not in log):
        raise ValueError('Task102 completion marker absent')
    sources = doc('campaign/sources.json')
    if not sources:
        raise ValueError('Task102 source hashes absent')
    for path, digest in sources.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task102 generated/fixture provenance mismatch')
    for release in PINS:
        if not raw(f'campaign/{release}-original-generated/service_api.ads'):
            raise ValueError('Task102 original generated API absent')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-original-generated/{name}'):
                raise ValueError('Task102 model/codec original selection byte parity mismatch')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-model/{name}'):
                raise ValueError('Task102 model/API codec opt-in byte parity mismatch')
    for release, digest in PINS.items():
        path = f'uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task102 wrong UCI release/root')
        include = f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw(include)).hexdigest() != INCLUDES[release]:
            raise ValueError('Task102 authentic schema include mismatch')
        for kind, pin in [('MessageDefinitions', digest), ('SecurityMarkings', INCLUDES[release])]:
            if hashlib.sha256(raw(f'campaign/admission/phase-a-{release}/schema/UCI_{kind}_v2_{release[-1]}_0.xsd')).hexdigest() != pin:
                raise ValueError('Task102 admission authentic schema hash mismatch')
        report = text(f'campaign/{release}-check.stdout')
        selected = 42 if release == '25' else 44
        if f'selected type closure: {selected}\n' not in report or f'codec renderable emitted declarations: {selected-1}/{selected-1}\ncodec status: READY\n' not in report:
            raise ValueError('Task102 actual Log readiness absent')
        for policy in ('Normal', 'Check', 'Ignore'):
            output = text(f'campaign/{release}-codec-{policy}-execute.stdout')
            if output != 'TASK093_LOG_COMPLETE checks= 29 rejections= 9\n':
                raise ValueError('Task102 mandatory Task093 real Log codec corpus incomplete')
    spec = importlib.util.spec_from_file_location('task102_independent_cases', ROOT / 'scripts/task102/cases.py')
    cases = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cases)
    for release in PINS:
        prefix = f'campaign/admission/phase-a-{release}/'
        if doc(prefix + f'task093-real-log{release}.json') != cases.golden(release):
            raise ValueError('Task102 admission golden differs')
        admission_wire = doc(prefix + 'wire.json')
        deliveries = [item['receive'] for item in admission_wire if item.get('receive', '').startswith('MSG 1 ')]
        if len(deliveries) != 1 or json.loads(deliveries[0].split(' ', 2)[2]) != {'Log': cases.golden(release)}:
            raise ValueError('Task102 admission real broker delivery missing')
    for release, policy, optimization in expected:
        cell = f'uci{release}-{policy}-O{optimization}'
        prefix = f'campaign/{cell}/sleet/'
        evidence = doc(prefix + 'result.json')
        if evidence != dict(completed=True, release=release, revision=REV, tree=TREE, handlers=7,
                            witness_handlers=1, decode_rejected=3, published=6, broker_rejected=10,
                            consumer_exit=0, server_exit=0, cleanup=True):
            raise ValueError('Task102 real-Sleet handler/count/cleanup mismatch')
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE]:
            raise ValueError('Task102 Sleet source mismatch')
        if not re.fullmatch('[0-9a-f]{64}', text(prefix + 'sleet-binary.sha256').strip()):
            raise ValueError('Task102 Sleet binary identity missing')
        if text(prefix + 'consumer.exit').strip() != '0' or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task102 consumer/server failed')
        output = text(prefix + 'consumer.stdout').splitlines()
        for marker in ['TASK102_ADA_REAL_LOG_COMPLETE handlers= 7 rejected= 3',
                       'TASK102_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1',
                       'TASK102_CLEANUP_COMPLETE', 'TASK102_DECODE_RECOVERY_COMPLETE',
                       'TASK102_UNKNOWN_BINDING_SID_REJECTED', 'TASK102_UPGRADE_OWP_COMPLETE',
                       'TASK102_INFO_VALIDATED', 'TASK102_SUB_ACCEPTED Log task102-log']:
            if output.count(marker) != 1:
                raise ValueError('Task102 interoperability marker missing/duplicate')
        if (sum(s.startswith('TASK102_TYPED_HANDLER ') for s in output) != 7
                or sum(s.startswith('TASK102_WITNESS_HANDLER ') for s in output) != 1
                or sum(s.startswith('TASK102_PUBLISHED ') for s in output) != 6
                or sum(s.startswith('TASK102_EVENT DECODE_ERROR ') for s in output) != 3):
            raise ValueError('Task102 actual handler/codec/publication count mismatch')
        published = [json.loads(s.removeprefix('TASK102_PUBLISHED ')) for s in output if s.startswith('TASK102_PUBLISHED ')]
        positive = cases.positives(release)
        if published != [expected for _, _, expected in positive]:
            raise ValueError('Task102 publications differ from independent real Log oracle')
        received = [json.loads(s.removeprefix('TASK102_WIRE_RECEIVED ')) for s in output if s.startswith('TASK102_WIRE_RECEIVED ')]
        expected_wire = [{'Log': doc} for _, doc in cases.decoder_invalid(release)]
        expected_wire += [{'Log': positive[0][1]}] + [{'Log': expected} for _, _, expected in positive] + [{'Log': positive[0][1]}]
        if received != expected_wire:
            raise ValueError('Task102 broker delivery wire identity/value/order mismatch')
        errors = [s.removeprefix('TASK102_EVENT DECODE_ERROR ') for s in output if s.startswith('TASK102_EVENT DECODE_ERROR ')]
        if errors != ['odd hexBinary digit count', 'invalid hexBinary digit', 'odd hexBinary digit count']:
            raise ValueError('Task102 decoder rejection diagnostic/stage mismatch')
        wire = doc(prefix + 'wire.json')
        for label, (code, detail) in BROKER_ERRORS.items():
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or not matches[0]['response'].startswith('-ERR ' + code) or detail not in matches[0]['response']:
                raise ValueError('Task102 expected broker error absent: ' + label)
        for label in ('odd-hex', 'invalid-digit', 'internal-space', 'same-subscription-recovery'):
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or matches[0]['response'] != '+OK':
                raise ValueError('Task102 broker/codec stage separation absent')
        for label, value in cases.decoder_invalid(release):
            observed = next(item for item in wire if item.get('label') == label)
            if json.loads(observed.get('command', '').removeprefix('PUB task102-log ')) != {'Log': value}:
                raise ValueError('Task102 decoder input differs from independent invalid corpus')
        for label, value, _ in positive[-2:]:
            admission = [item for item in wire if item.get('label') == 'broker-' + label]
            delivery = [item for item in wire if item.get('label') == 'broker-' + label + '-delivery']
            if (len(admission) != 1 or admission[0]['response'] != '+OK' or len(delivery) != 1
                    or not delivery[0]['response'].startswith('MSG 99 ')
                    or json.loads(delivery[0]['response'].split(' ', 2)[2]) != {'Log': value}):
                raise ValueError('Task102 independent broker lexical observation absent')
        if sum(item.get('label') == 'wrong-service-closed' for item in wire) != 1:
            raise ValueError('Task102 required service-error connection closure absent')
        server = text(prefix + 'server.stdout')
        if not all(marker in server for marker in ('schema loaded', 'route PUB message', 'shutdown complete')):
            raise ValueError('Task102 schema/routing/cleanup evidence absent')
    return dict(compiler=version, cells=12, releases=['25', '26'], handlers=84, codec_rejected=36,
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))