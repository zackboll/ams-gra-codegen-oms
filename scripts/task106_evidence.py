"""Independent source-bound Task106 verifier, with corpus-owned expectations.

Artifact producer attempts are resolved by CI-RETRY-001 before this entry point.
This verifier never accepts a development campaign as clean qualification.
"""
import hashlib
import importlib.util
import itertools
import json
from pathlib import Path
import re
import shlex

from task106 import cases
from task106.qualify import GENERATED, CONSUMERS, INDEPENDENT, inputs

ROOT = Path(__file__).resolve().parents[1]
JOB = 'ada-real-uci-multimessage-sleet'
PINS = {
    'UCI_MessageDefinitions_v2_5_0.xsd': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
    'UCI_SecurityMarkings_v2_5_0.xsd': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
}
REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'


def required_steps():
    labels = {'printenv', 'gnat-version', 'gprbuild-version', 'source', 'source-status',
              'tool-identities', 'alire-version', 'check', 'generate', 'model', 'admission'}
    labels |= {'baseline-' + key for key in cases.MESSAGES}
    labels |= {name + '-' + step for name in ('aht', 'websocket-ada') for step in ('clone', 'pin', 'identity', 'clean')}
    labels |= {'direction-' + key + '-' + direction for key in cases.MESSAGES for direction in ('receive', 'publish')}
    labels |= {'codec-' + policy + '-' + step for policy in ('Normal', 'Check', 'Ignore') for step in ('build', *cases.MESSAGES)}
    labels |= {f'independent-{policy}-{key}-{step}' for policy, key, step in
               itertools.product(('Normal', 'Check', 'Ignore'), ('rdma', 'survivability'), ('build', 'execute'))}
    labels |= {f'uci25-{policy}-O{opt}-{step}' for policy, opt, step in
               itertools.product(('Normal', 'Check', 'Ignore'), ('0', '2'), ('build', 'sleet'))}
    return labels


def verify_cell(raw, prefix):
    def text(name):
        return raw(prefix + '/' + name).decode()
    def doc(name):
        return json.loads(text(name))
    plan = cases.corpus()
    if [json.loads(line) for line in text('plan.jsonl').splitlines()] != plan:
        raise ValueError('Task106 authored plan differs')
    result = doc('result.json')
    expected = dict(completed=True, cleanup=True, counts=cases.expected_counts(),
                    decode_errors=cases.expected_errors(), broker_rejections=len(cases.broker_cases()),
                    steps=len(plan), consumer_exit=0, server_exit=0, witness_deliveries=cases.expected_witnesses())
    if 'error' in result or any(result.get(k) != v for k, v in expected.items()):
        raise ValueError('Task106 failed/incomplete live cell')
    if text('consumer.exit').strip() != '0' or text('server.exit').strip() != '0':
        raise ValueError('Task106 consumer/server exit mismatch')
    output = text('consumer.stdout')
    if ('TASK106_FAILED' in output or output.count('TASK106_UPGRADE_OWP_COMPLETE') != 1
            or output.count('TASK106_SIDS 1 2 3') != 1
            or output.count('TASK106_CLEANUP_COMPLETE lost_events=0') != 1):
        raise ValueError('Task106 missing/duplicate session/SIDs/cleanup')
    deliveries = [line for line in output.splitlines() if line.startswith('TASK106_DELIVERY ')]
    expected_deliveries = [p for p in plan if p['action'] in ('external', 'publish')]
    if len(deliveries) != len(expected_deliveries):
        raise ValueError('Task106 missing/duplicate broker delivery')
    for line, item in zip(deliveries, expected_deliveries, strict=True):
        _, sid, name, document = line.split(' ', 3)
        key = item['function']
        payload = item['expected'] if item['action'] == 'publish' else item['document']
        if sid != str(1 + list(cases.MESSAGES).index(key)) or name != cases.MESSAGES[key] or json.loads(document) != {name: payload}:
            raise ValueError('Task106 incorrect SID/global/document/order')
    counts = dict.fromkeys(cases.MESSAGES, 0)
    errors = 0
    done = [line for line in output.splitlines() if line.startswith('TASK106_DONE ')]
    if len(done) != len(plan):
        raise ValueError('Task106 missing cross-message isolation observation')
    for index, (item, line) in enumerate(zip(plan, done, strict=True), 1):
        if 'expected' in item:
            counts[item['function']] += 1
        errors += 'error' in item
        want = f'TASK106_DONE {index} counts' + ''.join(' ' + str(counts[k]) for k in cases.MESSAGES) + f' errors {errors}'
        if line != want:
            raise ValueError('Task106 incorrect cross-message isolation/recovery counts')
    for key, count in counts.items():
        actual = [line for line in output.splitlines() if line.startswith('TASK106_HANDLER ' + key + ' ')]
        if actual != [f'TASK106_HANDLER {key} {i}' for i in range(1, count + 1)]:
            raise ValueError('Task106 wrong typed handler sequence')
    rejected = [line for line in output.splitlines() if line.startswith('TASK106_DECODE_ERROR ')]
    want_errors = ['TASK106_DECODE_ERROR ' + str(1 + list(cases.MESSAGES).index(p['function'])) + ' ' +
                   cases.MESSAGES[p['function']] for p in plan if 'error' in p]
    if rejected != want_errors:
        raise ValueError('Task106 missing/wrong codec error SID/global')
    if [line for line in output.splitlines() if line.startswith('TASK106_OWNERSHIP_REJECTED')] != [
            'TASK106_OWNERSHIP_REJECTED ' + str(i) for i in range(1, 10)]:
        raise ValueError('Task106 binding ownership controls missing')
    events = [line for line in output.splitlines() if line.startswith('TASK106_EVENT ')]
    if sum(line.startswith('TASK106_EVENT INITIALIZED ') for line in events) != 1 or any(
            not line.startswith(('TASK106_EVENT INITIALIZED ', 'TASK106_EVENT COMMAND_ACCEPTED ', 'TASK106_EVENT DECODE_ERROR ')) for line in events):
        raise ValueError('Task106 ignored/unexpected connection event')
    if text('sleet-source.txt').splitlines() != [REV, TREE]:
        raise ValueError('Task106 broker pin changed')
    if not re.fullmatch('[0-9a-f]{64}', text('sleet-binary.sha256').strip()):
        raise ValueError('Task106 missing broker binary identity')
    log = text('server.stdout')
    registrations = re.findall(r'connection\{id=(\d+)[^\n]*subscription registered sid=([123]) topic=(\S+) message_name=(\S+)', log)
    if len(registrations) != 3 or len({r[0] for r in registrations}) != 1:
        raise ValueError('Task106 subscriptions do not share one connection')
    connection = registrations[0][0]
    if [(r[1], r[2], r[3]) for r in registrations] != [(str(i), cases.TOPICS[k], cases.MESSAGES[k]) for i, k in enumerate(cases.MESSAGES, 1)]:
        raise ValueError('Task106 broker registration mismatch')
    if len(re.findall(r'connection\{id=' + connection + r'[^\n]*recv operation="INIT"', log)) != 1 or len(
            re.findall(r'connection\{id=' + connection + r'[^\n]*recv operation="UNSUB"', log)) != 3:
        raise ValueError('Task106 unexpected INIT/UNSUB wire traffic')
    if 'shutdown complete' not in log:
        raise ValueError('Task106 broker shutdown missing')
    observations = doc('observations.json')
    negatives = observations[:len(cases.broker_cases())]
    for observed, (label, _, error_prefix, detail) in zip(negatives, cases.broker_cases(), strict=True):
        stage = 'OWP authorization/state rejection' if error_prefix == '-ERR Illegal-State' else 'Broker/XSD rejection'
        if observed.get('label') != label or observed.get('stage') != stage or not observed.get('response', '').startswith(error_prefix) or detail not in observed['response']:
            raise ValueError('Task106 wrong rejection stage/diagnostic')
    wire = doc('wire.json')
    witnessed = [item['receive'] for item in wire if item.get('receive', '').startswith('MSG ')]
    expected_witnessed = [p for p in plan if p['action'] != 'remove']
    if len(witnessed) != len(expected_witnessed):
        raise ValueError('Task106 missing actual witness delivery')
    for frame, item in zip(witnessed, expected_witnessed, strict=True):
        _, sid, document = frame.split(' ', 2)
        key = item['function']
        payload = item['expected'] if item['action'] == 'publish' else item['document']
        if sid != str(101 + list(cases.MESSAGES).index(key)) or json.loads(document) != {cases.MESSAGES[key]: payload}:
            raise ValueError('Task106 witness wrong global/SID/value/order')
    actual_sends = [entry['send'] for entry in wire if 'send' in entry]
    expected_sends = [command for _, command, _, _ in cases.broker_cases()]
    expected_sends += ['PUB ' + cases.TOPICS[p['function']] + ' ' + json.dumps({cases.MESSAGES[p['function']]: p['document']})
                       for p in plan if p['action'] == 'external']
    if actual_sends[4:-3] != expected_sends or actual_sends[-3:] != ['UNSUB 101', 'UNSUB 102', 'UNSUB 103']:
        raise ValueError('Task106 missing/changed independent sends')
    if sum(entry.get('receive') == '+OK' for entry in wire) != 7 + sum(p['action'] == 'external' for p in plan):
        raise ValueError('Task106 failed/missing publisher acceptance')
    return expected


def verify_bundle(files, compiler, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [v for name, v in files.items() if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task106 evidence: ' + suffix)
        return matches[0]
    def text(suffix):
        return raw(suffix).decode('utf-8', errors='strict')
    def doc(suffix):
        return json.loads(text(suffix))
    if doc('integration-identity.json') != dict(sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt), compiler=compiler):
        raise ValueError('Task106 source/tree/run/attempt/compiler mismatch')
    if text('campaign/source.stdout').splitlines() != [sha, tree] or text('campaign/source-status.stdout').strip():
        raise ValueError('Task106 dirty/wrong source')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    if not text('campaign/gnat-version.stdout').startswith('GNAT ' + version + '\n') or text('campaign/alire-version.stdout').strip() != 'alr 2.1.1':
        raise ValueError('Task106 wrong compiler/Alire')
    from task105_evidence import TOOL_HASHES
    tool_output = text('campaign/tool-identities.stdout')
    for pin in [*TOOL_HASHES[compiler].values(), 'aff9fde5ca766d5497da9dec185cb8b96878206a0d02512aa5685e1e56d0a4d3']:
        if pin not in tool_output:
            raise ValueError('Task106 wrong compiler/builder/linker binary identity')
    spec = importlib.util.spec_from_file_location('task106_lock_guard', ROOT / 'scripts/check-task087a.py')
    guard = importlib.util.module_from_spec(spec); spec.loader.exec_module(guard)
    try:
        lock = doc('campaign/toolchain-lock.json')
        guard.validate_solution(lock, 'gnat_external' if compiler == '14' else 'gnat_native')
    except RuntimeError as error:
        raise ValueError('Task106 wrong toolchain lock') from error
    archive_pins = {'gprbuild': 'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b'}
    if compiler == '16':
        archive_pins['gnat_native'] = 'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703'
    for name, pin in archive_pins.items():
        releases = [entry['release'] for entry in lock['solution']['state'] if entry.get('release', {}).get('name') == name]
        # Alire records both the compiler dependency and its selected toolchain
        # alias. Every matching release must carry the same immutable archive.
        if not releases or any(pin not in release.get('origin', {}).get('hashes', []) for release in releases):
            raise ValueError('Task106 compiler/builder archive pin changed')
    if doc('campaign/input-sources.json') != inputs():
        raise ValueError('Task106 inputs differ from authoritative source')
    result = doc('campaign/result.json')
    wanted = set(itertools.product(('Normal', 'Check', 'Ignore'), ('0', '2')))
    cells = [(c['policy'], c['optimization']) for c in result['cells'] if c.get('release') == '25']
    if result.get('completed') is not True or result.get('development') is not False or result.get('compiler') != version or len(cells) != 6 or set(cells) != wanted or len(result['cells']) != 6:
        raise ValueError('Task106 missing/duplicate/wrong matrix cells')
    steps = result['steps']
    labels = [s['label'] for s in steps]
    if len(labels) != len(set(labels)) or set(labels) != required_steps():
        raise ValueError('Task106 skipped/duplicate mandatory step')
    for step in steps:
        label = step['label']
        exit_code = text('campaign/' + label + '.exit').strip()
        negative = label.startswith('direction-')
        if exit_code != str(step['exit']) or (int(exit_code) == 0) == negative:
            raise ValueError('Task106 failed mandatory/negative step')
        command = doc('campaign/' + label + '.command.json')
        argv = command.get('argv')
        if not isinstance(argv, list) or not argv or not all(isinstance(a, str) and a for a in argv) or not command.get('cwd', '').startswith('/'):
            raise ValueError('Task106 incomplete command record')
        if argv[:2] == ['sh', '-ec']:
            if len(argv) != 3 or not argv[2].startswith('. ') or '; exec ' not in argv[2]:
                raise ValueError('Task106 missing pinned environment')
            argv = shlex.split(argv[2].split('; exec ', 1)[1])
        if label.endswith('-build') or negative:
            if not argv or argv[0] != 'gprbuild' or not {'-p', '-P'} <= set(argv):
                raise ValueError('Task106 actual builder command absent')
        if label.startswith('uci25-') and label.endswith('-build'):
            if not {'gprbuild', '-p', '-P', '-gnato', '-' + label.split('-')[2]} <= set(argv) or not any(a.startswith('-gnatec=') for a in argv):
                raise ValueError('Task106 missing actual matrix compile flags')
        if label in ('check', 'generate', 'model') or label.startswith('baseline-'):
            operation = 'service-check' if label == 'check' else 'service-generate'
            if (len(argv) < 10 or argv[1] != operation or not {'--schema', '--contract', '--language', '--world'} <= set(argv)
                    or argv[argv.index('--language') + 1] != 'ada' or argv[argv.index('--world') + 1] != 'closed-schema'
                    or ('--with-codec' in argv) != (label != 'model')):
                raise ValueError('Task106 generation command changed/skipped')
        if label.endswith('-sleet') and (not any(a.endswith('/scripts/task106/sleet.py') for a in argv)
                or not {'--evidence', '--checkout', '--schema', '--consumer'} <= set(argv)):
            raise ValueError('Task106 actual Sleet execution command missing')
        if label == 'admission' and (not any(a.endswith('/scripts/task106/admission.py') for a in argv)
                or not {'--evidence', '--sleet', '--uci25'} <= set(argv)):
            raise ValueError('Task106 admission command missing')
        if label.startswith('codec-') and not label.endswith('-build'):
            key = label.split('-')[-1]
            _, executable, fixture, _ = next(c for c in CONSUMERS if c[0] == key)
            if len(argv) != 2 or not argv[0].endswith('/bin/' + executable) or not argv[1].endswith('/tests/fixtures/service-generate/' + fixture):
                raise ValueError('Task106 original codec consumer execution missing')
        if label.startswith('independent-') and label.endswith('-execute') and (
                len(argv) != 3 or not argv[0].endswith('/bin/codec_probe')
                or not argv[1].endswith('/documents.jsonl') or not argv[2].endswith('/reasons.txt')):
            raise ValueError('Task106 independent codec/model execution missing')
        if negative:
            forbidden = 'Publisher' if label.endswith('-receive') else 'Subscriber'
            if f'"{forbidden}" not declared' not in text('campaign/' + label + '.stderr'):
                raise ValueError('Task106 incorrect direction diagnostic')
    for name, pin in GENERATED.items():
        if hashlib.sha256(raw('campaign/generated/' + name)).hexdigest() != pin:
            raise ValueError('Task106 forged generated-source parity')
        if name.startswith('programs') or name == 'service_api.ads':
            if raw('campaign/model/' + name) != raw('campaign/generated/' + name):
                raise ValueError('Task106 model byte parity mismatch')
    for name, pin in PINS.items():
        if hashlib.sha256(raw('campaign/uci25/' + name)).hexdigest() != pin:
            raise ValueError('Task106 authentic schema changed')
    representations = doc('campaign/representation-parity.json')
    from task104_evidence import BASELINE as SURVIVABILITY_BASELINE
    for key in cases.MESSAGES:
        pins = SURVIVABILITY_BASELINE if key == 'survivability' else INDEPENDENT[key]
        for name, pin in pins.items():
            if hashlib.sha256(raw('campaign/baseline-' + key + '/' + name)).hexdigest() != pin:
                raise ValueError('Task106 independent single-message baseline changed')
        payload = {'log': 'LogMT', 'rdma': 'RDMA_InitializeSetupMT', 'survivability': 'SurvivabilityRiskLevelMT'}[key]
        pattern = r'^   type ' + payload + r' is record\n.*?^   end record;'
        original = re.findall(pattern, text('campaign/baseline-' + key + '/programs-oam.ads'), re.M | re.S)
        combined = re.findall(pattern, text('campaign/generated/programs-oam.ads'), re.M | re.S)
        if len(original) != 1 or original != combined or representations.get(key) != dict(
                payload=payload, representation_sha256=hashlib.sha256(original[0].encode()).hexdigest(), independent_baseline_hashes=pins):
            raise ValueError('Task106 forged per-type representation parity')
    source_hashes = doc('campaign/sources.json')
    for relative, pin in source_hashes.items():
        if relative.startswith('/') or '..' in relative.split('/') or hashlib.sha256(raw('campaign/' + relative)).hexdigest() != pin:
            raise ValueError('Task106 source manifest mismatch')
    for policy in ('Normal', 'Check', 'Ignore'):
        for key, _, _, marker in CONSUMERS:
            if text('campaign/codec-' + policy + '-' + key + '.stdout') != marker:
                raise ValueError('Task106 original codec consumer controls missing')
        for key, number, count in [('rdma', 103, 9), ('survivability', 104, 14)]:
            if not text(f'campaign/independent-{policy}-{key}-execute.stdout').endswith(
                    f'TASK{number}_INDEPENDENT_CODEC_COMPLETE rejected={count} model_constraints=4\n'):
                raise ValueError('Task106 independent codec/model rejection missing')
    admission = doc('campaign/admission/result.json')
    if admission.get('status') != 'PASS' or admission.get('cleanup') is not True or admission.get('generated_ada_decoder_executed') is not False:
        raise ValueError('Task106 independent admission missing')
    if admission.get('subscriptions') != [dict(sid=i, name=cases.MESSAGES[key], topic=cases.TOPICS[key])
            for i, key in enumerate(cases.MESSAGES, 1)] or admission.get('init_count') != 1 or admission.get('info_validated') is not True:
        raise ValueError('Task106 independent session/subscription identity missing')
    for name, revision in [('aht', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
                           ('websocket-ada', '765d9cd612838eb00645037977c8cc9af7171c3e')]:
        if text('campaign/' + name + '-identity.stdout').strip() != revision or text('campaign/' + name + '-clean.stdout').strip():
            raise ValueError('Task106 transport source pin/cleanliness changed')
    for policy, opt in sorted(wanted):
        prefix = f'campaign/uci25-{policy}-O{opt}'
        for name in GENERATED:
            if raw(prefix + '/generated/' + name) != raw('campaign/generated/' + name):
                raise ValueError('Task106 cell generated source changed')
        if raw(prefix + '/multimessage_consumer.adb') != (ROOT / 'tests/ada-real-uci-multimessage/multimessage_consumer.adb').read_bytes():
            raise ValueError('Task106 cell consumer differs from source')
        if text(prefix + '/policy.adc') != ('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n'):
            raise ValueError('Task106 wrong assertion policy')
        for source in (ROOT / 'ada').rglob('*'):
            if source.is_file() and source.suffix in ('.ads', '.adb', '.gpr', '.c', '.h'):
                relative = str(source.relative_to(ROOT))
                if raw(prefix + '/' + relative) != source.read_bytes():
                    raise ValueError('Task106 cell inherited runtime differs from source')
        verify_cell(raw, prefix + '/sleet')
        if text(prefix + '/sleet/sleet-binary.sha256') != text('campaign/admission/sleet-binary.sha256'):
            raise ValueError('Task106 broker binary changed between admission and execution')
    return dict(compiler=compiler, cells=6, counts=cases.expected_counts(), decode_errors=cases.expected_errors())