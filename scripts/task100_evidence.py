"""Fail-closed reconciliation of the original Task100 campaign evidence."""
import itertools
import json
import hashlib
import re

SLEET_REV = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
SLEET_TREE = '7e705f4eb5f658957bd414b4abc8207311f3c1e4'
AHT_SPDX_LICENSE_SHA256 = '074e6e32c86a4c0ef8b3ed25b721ca23aca83df277cd88106ef7177c354615ff'
UPSTREAM_SOURCES = {
    *(f'deps/aht/src/{name}' for name in (
        'aht.ads', 'aht-bytes.ads', 'aht-bytes.adb', 'aht-websocket.ads',
        'aht-websocket-frames.ads', 'aht-websocket-frames.adb',
        'aht-websocket-utf8.ads', 'aht-websocket-utf8.adb',
        'aht-random.ads', 'aht-random.adb', 'aht-random-source.ads')),
    'deps/aht/src/os/posix/aht-random-source.adb',
    *(f'deps/websocket-ada/src/{name}' for name in (
        'websocket.ads', 'websocket-handshake.ads', 'websocket-handshake.adb',
        'websocket-base64.ads', 'websocket-base64.adb')),
}
DEPENDENCIES = {
    'aht': ('664d5ddeb568626b2f9ff66054c7091894b3b8d4',
            'cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30'),
    'websocket-ada': ('765d9cd612838eb00645037977c8cc9af7171c3e',
                      '7202d55f00bedb98266732087979628545c6bd24bd84815597d8d0b45c8249fd'),
}
# Independently frozen from the original local peer; do not derive expected
# cases from the candidate artifact's own result.json.
LOCAL_CASES = {
    'valid', 'tcp-split', 'stalled-partial', 'fragment-ping', 'utf8-fragments',
    'receive-large', 'peer-close', 'abrupt', 'large', 'send-failure',
    'validation', 'lifecycle', 'finalize', 'duplicate-protocol',
    'excessive-headers', 'extensions', 'invalid-upgrade', 'malformed-header',
    'missing-protocol', 'redirect', 'wrong-accept', 'wrong-protocol', 'masked',
    'reserved', 'rsv', 'utf8', 'binary', 'oversized', 'fragment-control',
    'large-control', 'nonminimal', 'length-msb', 'orphan-continuation',
    'new-text-during-fragment', 'bad-close-code', 'bad-close-utf8',
    'fragment-oversized', 'generated',
    *('invalid-url-' + str(n) for n in range(5)),
}


def verify_bundle(files, compiler, real_sleet, sha, tree, run_id, attempt):
    def raw(suffix):
        matches = [value for name, value in files.items()
                   if name == suffix or name.endswith('/' + suffix)]
        if len(matches) != 1:
            raise ValueError('missing/duplicate Task100 evidence: ' + suffix)
        return matches[0]

    def content(suffix):
        return raw(suffix).decode('utf-8', errors='strict')

    def document(suffix):
        return json.loads(content(suffix))

    identity = document('integration-identity.json')
    if identity != dict(sha=sha, tree=tree, run_id=str(run_id),
                        run_attempt=str(attempt), compiler=compiler):
        raise ValueError('stale/wrong Task100 source/run/attempt/compiler identity')
    for suffix in ('source.txt', 'campaign/source.stdout'):
        if content(suffix).splitlines() != [sha, tree]:
            raise ValueError('Task100 campaign source/tree mismatch')
    if content('campaign/source-status.stdout').strip():
        raise ValueError('Task100 campaign source was dirty')
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    if not content('campaign/compiler.stdout').startswith('GNAT ' + version + '\n'):
        raise ValueError('Task100 compiler identity mismatch')
    for name, (revision, license_digest) in DEPENDENCIES.items():
        if (content(f'campaign/{name}-identity.stdout').strip() != revision
                or content(f'campaign/{name}-clean.stdout').strip()
                or hashlib.sha256(raw(f'campaign/deps/{name}/LICENSE')).hexdigest() != license_digest):
            raise ValueError('Task100 pinned dependency/license identity mismatch')
    if hashlib.sha256(raw('campaign/deps/aht/LICENSES/Apache-2.0.txt')).hexdigest() != AHT_SPDX_LICENSE_SHA256:
        raise ValueError('Task100 AHT SPDX license text identity mismatch')
    sources = document('campaign/sources.json')
    if not UPSTREAM_SOURCES <= sources.keys():
        raise ValueError('missing/incomplete Task100 pinned source provenance')
    for path, digest in sources.items():
        if (path.startswith('/') or '..' in path.split('/')
                or hashlib.sha256(raw('campaign/' + path)).hexdigest() != digest):
            raise ValueError('Task100 source provenance mismatch')
    result = document('campaign/result.json')
    expected = set(itertools.product(('oam', 'qualified'), ('portable', 'lossless'),
                                     ('Normal', 'Check', 'Ignore'), ('0', '2')))
    cells = result['cells']
    actual = [(c['namespace'], c['profile'], c['policy'], c['optimization']) for c in cells]
    if (result.get('completed') is not True or result.get('compiler') != version
            or result.get('real_sleet') is not real_sleet
            or len(actual) != 24 or set(actual) != expected):
        raise ValueError('incomplete/duplicate Task100 compiler/policy/optimization matrix')
    steps = result['steps']
    labels = [s['label'] for s in steps]
    if not steps or len(labels) != len(set(labels)) or any(s['exit'] != 0 for s in steps):
        raise ValueError('failed/duplicate/missing Task100 campaign steps')
    for step in steps:
        if content('campaign/' + step['label'] + '.exit').strip() != '0':
            raise ValueError('Task100 recorded exit mismatch')
    log = content('qualify.log')
    if (sum(line.startswith('TASK100_CELL_COMPLETE ') for line in log.splitlines()) != 24
            or f'TASK100_QUALIFICATION_COMPLETE {version} 24' not in log.splitlines()):
        raise ValueError('Task100 mandatory completion markers missing')
    for namespace, profile, policy, optimization in expected:
        cell = f'{namespace}-{profile}-{policy}-O{optimization}'
        for phase in ('build', 'local'):
            if cell + '-' + phase not in labels:
                raise ValueError('missing Task100 mandatory execution step')
        local = document(f'campaign/{cell}/local/result.json')
        count = 44 if profile == 'lossless' else 43
        expected_cases = LOCAL_CASES | ({'float64'} if profile == 'lossless' else set())
        if (local.get('completed') is not True or len(local['cases']) != count
                or set(local['cases']) != expected_cases):
            raise ValueError('incomplete Task100 local scenarios/Float64 probes')
        if f'TASK100_LOCAL_SUITE_COMPLETE {count}' not in content(f'campaign/{cell}-local.stdout').splitlines():
            raise ValueError('Task100 local completion marker missing')
        directory = f'campaign/{cell}/local/'
        if 'RESOURCE_LIFECYCLES_COMPLETE cycles=30 fd_growth=0 task_growth=0' not in content(directory + 'lifecycle.stdout').splitlines():
            raise ValueError('Task100 resource lifecycle marker missing')
        short_sends = re.search(r'TASK100_FORCED_SHORT_SENDS=(\d+)', content(directory + 'large.stderr'))
        if short_sends is None or int(short_sends[1]) <= 0:
            raise ValueError('Task100 partial-send injection not executed')
        generated = content(directory + 'generated.stdout')
        if ('OWNERSHIP_WEBSOCKET_COMPLETE' not in generated
                or 'TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1' not in generated
                or generated.count('OWP_EVENT DECODE_ERROR ') != 4):
            raise ValueError('Task100 generated ownership/codec completion missing')
        if profile == 'lossless' and 'TASK100_FLOAT64_TRANSPORT_COMPLETE values=4 exact_bytes=4' not in content(directory + 'float64.stdout'):
            raise ValueError('Task100 Float64 transport probe missing')
        if real_sleet and namespace == 'oam' and profile == 'portable':
            if cell + '-sleet' not in labels:
                raise ValueError('missing Task100 pinned-Sleet step')
            sleet = document(f'campaign/{cell}/sleet/result.json')
            if (sleet.get('completed') is not True or sleet.get('revision') != SLEET_REV
                    or sleet.get('generated_typed_handlers') is not True
                    or sleet.get('real_uci') is not False):
                raise ValueError('incomplete/wrong Task100 real pinned-Sleet evidence')
            if 'TASK100_REAL_PINNED_SLEET_COMPLETE' not in content(f'campaign/{cell}-sleet.stdout').splitlines():
                raise ValueError('Task100 pinned-Sleet completion marker missing')
            directory = f'campaign/{cell}/sleet/'
            source = content(directory + 'sleet-source.txt').splitlines()
            if source != [SLEET_REV, SLEET_TREE]:
                raise ValueError('Task100 pinned-Sleet source identity missing')
            if not re.fullmatch('[0-9a-f]{64}', content(directory + 'sleet-binary.sha256').strip()):
                raise ValueError('Task100 pinned-Sleet binary identity missing')
            consumer = content(directory + 'consumer.stdout')
            if (content(directory + 'consumer.exit').strip() != '0'
                    or content(directory + 'server.exit').strip() != '0'
                    or 'OWNERSHIP_WEBSOCKET_COMPLETE' not in consumer
                    or 'TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1' not in consumer):
                raise ValueError('Task100 pinned-Sleet execution/cleanup incomplete')
    return dict(compiler=version, cells=24, local_scenarios=1044,
                float64_probes=12, pinned_sleet_executions=6 if real_sleet else 0,
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))