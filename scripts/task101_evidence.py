"""Source-bound Task101 evidence verifier; expectations are not artifact-owned."""
import hashlib
import itertools
import json
import re
from pathlib import Path

JOB = 'ada-real-uci-dlz-sleet'
PINS = {'25': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
        '26': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
INCLUDES = {'25': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
            '26': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'}
ROOT = Path(__file__).resolve().parents[1]
BROKER_ERRORS = {
    'missing-time': ('Invalid-Message', 'MissingRequiredMember'),
    'offset-time': ('Invalid-Message', 'FacetViolation'),
    'null-time': ('Invalid-Message', 'TypeMismatch'),
    'number-time': ('Invalid-Message', 'TypeMismatch'),
    'quoted-float64': ('Invalid-Message', 'TypeMismatch'),
    'choice-two-arms': ('Invalid-Message', 'ChoiceViolation'),
    'wrong-global': ('Invalid-Message', 'UnknownGlobalElement'),
    'wrong-topic': ('Illegal-State', 'not authorized to publish to topic forbidden'),
    'unknown-sid': ('Illegal-State', 'unknown subscription id 987'),
    'duplicate-invalid-last': ('Invalid-Message', 'TypeMismatch'),
}


def verify_bundle(files, compiler, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [value for name, value in files.items() if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task101 evidence: ' + suffix)
        return matches[0]
    def text(suffix):
        return raw(suffix).decode('utf-8', errors='strict')
    def doc(suffix):
        return json.loads(text(suffix))
    if doc('integration-identity.json') != dict(sha=sha, tree=tree, run_id=str(run_id),
                                              run_attempt=str(attempt), compiler=compiler):
        raise ValueError('Task101 source/tree/run/attempt/compiler mismatch')
    if text('campaign/source.stdout').splitlines() != [sha, tree] or text('campaign/source-status.stdout').strip():
        raise ValueError('Task101 dirty/wrong source identity')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    if not text('campaign/gnat-version.stdout').startswith('GNAT ' + version + '\n'):
        raise ValueError('Task101 wrong compiler')
    if not re.fullmatch('[0-9a-f]{64}', text('campaign/generator.sha256').strip()):
        raise ValueError('Task101 generator identity missing')
    inputs = doc('campaign/input-sources.json')
    required_inputs = {'scripts/task101/qualify.py', 'scripts/task101/sleet.py', 'scripts/task101/cases.py',
                       'tests/ada-real-uci-dlz/dlz_consumer.adb', 'scripts/task101_evidence.py',
                       'scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
                       'tests/ada-time-zulu-codec/time_consumer.adb'}
    required_inputs |= {f'tests/fixtures/task101/uci{r}-{s}' for r in PINS for s in ('full-dlz.json', 'service.yaml')}
    if not required_inputs <= inputs.keys():
        raise ValueError('Task101 mandatory input sources absent')
    for path, digest in inputs.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('Task101 qualification inputs differ from exact source')
    result = doc('campaign/result.json')
    expected = set(itertools.product(('25', '26'), ('Normal', 'Check', 'Ignore'), ('0', '2')))
    actual = [(c['release'], c['policy'], c['optimization']) for c in result['cells']]
    if (result.get('completed') is not True or result.get('compiler') != version
            or result.get('real_sleet') is not True or result.get('real_uci') is not True
            or len(actual) != 12 or set(actual) != expected):
        raise ValueError('Task101 missing/duplicate/wrong release/policy/optimization cell')
    steps = result['steps']
    labels = [step['label'] for step in steps]
    if not labels or len(labels) != len(set(labels)) or any(s['exit'] != 0 for s in steps):
        raise ValueError('Task101 failed/duplicate execution step')
    mandatory = {'source', 'source-status', 'platform', 'gnat-version', 'gprbuild-version', 'printenv'}
    mandatory |= {f'{r}-{s}' for r in PINS for s in ('check', 'generate', 'original', 'model')}
    mandatory |= {f'uci{r}-{p}-O{o}-{s}' for r, p, o in expected for s in ('build', 'sleet')}
    mandatory |= {f'{r}-dlz-{p}-{s}' for r in PINS for p in ('Normal', 'Check', 'Ignore') for s in ('build', 'execute')}
    if not mandatory <= set(labels):
        raise ValueError('Task101 mandatory execution step absent')
    for label in labels:
        if text('campaign/' + label + '.exit').strip() != '0':
            raise ValueError('Task101 execution exit mismatch')
    log = text('qualify.log').splitlines()
    if (sum(line.startswith('TASK101_CELL_COMPLETE ') for line in log) != 12
            or f'TASK101_QUALIFICATION_COMPLETE {version} 12' not in log):
        raise ValueError('Task101 completion marker absent')
    sources = doc('campaign/sources.json')
    if not sources:
        raise ValueError('Task101 source hashes absent')
    for path, digest in sources.items():
        if path.startswith('/') or '..' in path.split('/') or hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task101 generated/fixture provenance mismatch')
    for release in PINS:
        if not raw(f'campaign/{release}-original-generated/service_api.ads'):
            raise ValueError('Task101 original generated API absent')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_codec.ads', 'service_codec.adb'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-original-generated/{name}'):
                raise ValueError('Task101 model/codec original selection byte parity mismatch')
        for name in ('programs.ads', 'programs-oam.ads', 'programs-oam.adb', 'service_api.ads'):
            if raw(f'campaign/{release}-generated/{name}') != raw(f'campaign/{release}-model/{name}'):
                raise ValueError('Task101 model/API codec opt-in byte parity mismatch')
    for release, digest in PINS.items():
        path = f'uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest:
            raise ValueError('Task101 wrong UCI release/root')
        include = f'campaign/uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
        if hashlib.sha256(raw(include)).hexdigest() != INCLUDES[release]:
            raise ValueError('Task101 authentic schema include mismatch')
        report = text(f'campaign/{release}-check.stdout')
        if 'selected type closure: 54\n' not in report or 'codec renderable emitted declarations: 53/53\ncodec status: READY\n' not in report:
            raise ValueError('Task101 actual DLZ readiness absent')
        for policy in ('Normal', 'Check', 'Ignore'):
            output = text(f'campaign/{release}-dlz-{policy}-execute.stdout').splitlines()
            if (len(output) != 27 or output[-1] != 'TASK098_CONSUMER_COMPLETE count= 26 accepted= 10 rejected= 16'
                    or sum(s.startswith('ACCEPT\t') for s in output) != 10
                    or sum(s.startswith('REJECT\t') for s in output) != 16):
                raise ValueError('Task101 mandatory real codec corpus incomplete')
    for release, policy, optimization in expected:
        cell = f'uci{release}-{policy}-O{optimization}'
        prefix = f'campaign/{cell}/sleet/'
        evidence = doc(prefix + 'result.json')
        if evidence != dict(completed=True, release=release, revision=REV, tree=TREE, handlers=11,
                            witness_handlers=1, decode_rejected=1, published=10, broker_rejected=10,
                            consumer_exit=0, server_exit=0, cleanup=True):
            raise ValueError('Task101 real-Sleet handler/count/cleanup mismatch')
        if text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE]:
            raise ValueError('Task101 Sleet source mismatch')
        if not re.fullmatch('[0-9a-f]{64}', text(prefix + 'sleet-binary.sha256').strip()):
            raise ValueError('Task101 Sleet binary identity missing')
        if text(prefix + 'consumer.exit').strip() != '0' or text(prefix + 'server.exit').strip() != '0':
            raise ValueError('Task101 consumer/server failed')
        output = text(prefix + 'consumer.stdout').splitlines()
        for marker in ['TASK101_ADA_REAL_DLZ_COMPLETE handlers= 11 rejected= 1',
                       'TASK101_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1',
                       'TASK101_CLEANUP_COMPLETE', 'TASK101_DECODE_RECOVERY_COMPLETE',
                       'TASK101_UNKNOWN_BINDING_SID_REJECTED', 'TASK101_UPGRADE_OWP_COMPLETE',
                       'TASK101_INFO_VALIDATED', 'TASK101_SUB_ACCEPTED DLZ task098-dlz']:
            if output.count(marker) != 1:
                raise ValueError('Task101 interoperability marker missing/duplicate')
        if (sum(s.startswith('TASK101_TYPED_HANDLER ') for s in output) != 11
                or sum(s.startswith('TASK101_WITNESS_HANDLER ') for s in output) != 1
                or sum(s.startswith('TASK101_PUBLISHED ') for s in output) != 10
                or output.count('TASK101_EVENT DECODE_ERROR invalid named Time') != 1):
            raise ValueError('Task101 actual handler/codec/publication count mismatch')
        wire = doc(prefix + 'wire.json')
        rejections = [item for item in wire if item.get('response', '').startswith('-ERR Invalid-Message')
                      or item.get('label') in ('wrong-topic', 'unknown-sid')]
        if len(rejections) != 10:
            raise ValueError('Task101 broker rejection observations absent')
        for label, (code, detail) in BROKER_ERRORS.items():
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or not matches[0]['response'].startswith('-ERR ' + code) or detail not in matches[0]['response']:
                raise ValueError('Task101 expected broker error absent: ' + label)
        for label in ('broker-accepted-codec-invalid-time', 'duplicate-valid-last-recovery'):
            matches = [item for item in wire if item.get('label') == label]
            if len(matches) != 1 or matches[0]['response'] != '+OK':
                raise ValueError('Task101 broker/codec stage separation absent')
        if sum(item.get('label') == 'wrong-service-closed' for item in wire) != 1:
            raise ValueError('Task101 required service-error connection closure absent')
        server = text(prefix + 'server.stdout')
        if not all(marker in server for marker in ('schema loaded', 'route PUB message', 'shutdown complete')):
            raise ValueError('Task101 schema/routing/cleanup evidence absent')
    return dict(compiler=version, cells=12, releases=['25', '26'], handlers=132, codec_rejected=12,
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))