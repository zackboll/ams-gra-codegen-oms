"""Independent source inventory and outcome verification for owned Ada bundles."""
import json
import re
from owned_qualification import ROOT, VERSIONS, POLICIES, OPTIMIZATIONS, ada, cells, digest, document
from task101_evidence import PINS, INCLUDES, REV, TREE, BROKER_ERRORS


def verify_codec(data, cases):
    # The historical independent JSON-kind/binary64 oracle is not the decoder.
    import importlib.util
    spec = importlib.util.spec_from_file_location('owned_oracle', ROOT / 'scripts/task098/cases.py')
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    lines = data.decode().splitlines()
    accepted = sum(c['expected'] is not None for c in cases)
    marker = (f'TASK098_CONSUMER_COMPLETE count= {len(cases)} accepted= {accepted}'
              f' rejected= {len(cases) - accepted}')
    if len(lines) != len(cases) + 1 or lines[-1] != marker:
        raise ValueError('incomplete owned codec execution')
    for line, case in zip(lines[:-1], cases, strict=True):
        kind, _, value = line.partition('\t')
        if case['expected'] is None:
            if kind != 'REJECT' or not value:
                raise ValueError('owned negative codec case did not reject')
        elif kind != 'ACCEPT' or not oracle.same(oracle.decode(value), case['expected']):
            raise ValueError('owned codec case wrong value/kind')


def verify_bundle(files, mode, compiler, sha, tree, run_id, attempt, root=ROOT):
    def raw(name):
        if name not in files:
            raise ValueError('missing owned Ada evidence: ' + name)
        return files[name]
    def text(name):
        return raw(name).decode('utf-8')
    result = document(raw('result.json'))
    records = [r for r in ada(root) if (r['mode'] == 'fast') == (mode == 'fast')]
    expected = [cell for r in records for cell in cells(r)]
    identity = result['identity']
    wanted = dict(sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt),
                  compiler=compiler, compiler_version=VERSIONS[compiler], mode=mode)
    if any(identity.get(key) != value for key, value in wanted.items()):
        raise ValueError('wrong owned source/run/attempt/compiler/owner')
    if not re.fullmatch('[0-9a-f]{64}', identity.get('generator_sha256', '')):
        raise ValueError('missing owned generator identity')
    if text('generator.sha256').strip() != identity['generator_sha256']:
        raise ValueError('wrong actual generator identity')
    if mode == 'pinned' and (identity.get('uci_roots') != PINS or identity.get('uci_includes') != INCLUDES or
            identity.get('sleet_revision') != REV or identity.get('sleet_tree') != TREE or
            not re.fullmatch('[0-9a-f]{64}', identity.get('sleet_binary_sha256', ''))):
        raise ValueError('wrong pinned owned qualification source')
    if mode == 'pinned':
        for release in PINS:
            if (digest(raw(f'uci{release}/UCI_MessageDefinitions_v2_{release[-1]}_0.xsd')) != PINS[release] or
                    digest(raw(f'uci{release}/UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')) != INCLUDES[release]):
                raise ValueError('wrong actual pinned schema/include bytes')
    if (result.get('completed') is not True or result.get('version') != 1 or result['registrations'] != records or
            result['required_cells'] != expected or result['actual_cells'] != expected):
        raise ValueError('incomplete/forged owned registration/cell inventory')
    for name, value in result['files'].items():
        if name.startswith('/') or '..' in name.split('/') or digest(raw(name)) != value:
            raise ValueError('missing/corrupt owned Ada log/source')
    if not result['files']:
        raise ValueError('missing owned source/log hashes')
    if (not text('gnat-version.stdout').startswith('GNAT ' + VERSIONS[compiler] + '\n') or
            text('owned-source.stdout').splitlines() != [sha, tree] or text('owned-clean.stdout')):
        raise ValueError('wrong actual owned compiler/source identity')
    labels = [s['label'] for s in result['steps']]
    if len(labels) != len(set(labels)) or any(s['exit'] != 0 or text(s['label'] + '.exit') != '0\n'
                                            for s in result['steps']):
        raise ValueError('masked/duplicate failed owned step')
    tools = text('tool-identities.stdout').splitlines()
    if (len(tools) != 6 or any(not tools[n].startswith('/') or not re.fullmatch('[0-9a-f]{64}  .+', tools[n+1])
                              for n in (0, 2, 4))):
        raise ValueError('missing actual compiler binary identities')
    required = {'printenv', 'gnat-version', 'gprbuild-version', 'owned-source', 'owned-clean', 'tool-identities'}
    for record in records:
        item = record['manifest']
        if record['mode'] != 'fast':
            required.update(item['task_id'] + '-' + suffix for suffix in ('check', 'generate', 'model'))
            if 'codec status: READY\n' not in text(item['task_id'] + '-check.stdout'):
                raise ValueError('owned generated codec not ready')
        for policy in POLICIES:
            for optimization in OPTIMIZATIONS:
                label = item['task_id'] + '-' + policy + '-O' + optimization
                required.update([label + '-build', label + '-execute'])
                for name in item['sources']:
                    if raw(label + '/' + name) != (root / record['path'] / name).read_bytes():
                        raise ValueError('compiled Ada source differs from registered source')
                command = json.loads(text(label + '-build.command.json'))['argv']
                if not all(flag in ' '.join(command) for flag in ('-gnato', '-O' + optimization, '-gnatec=')):
                    raise ValueError('wrong owned assertion/optimization build policy')
                if text(label + '/policy.adc') != ('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n'):
                    raise ValueError('wrong actual Ada assertion policy')
                if record['mode'] == 'fast':
                    if sorted(text(label + '-execute.stdout').splitlines()) != ['CI004_PASS ' + t for t in item['tests']]:
                        raise ValueError('missing/duplicate/skipped owned Ada tests')
                elif record['mode'] == 'uci':
                    verify_codec(raw(label + '-execute.stdout'), document((root / record['path'] / item['cases']).read_text()))
                else:
                    prefix = label + '/sleet/'
                    observed = document(raw(prefix + 'result.json'))
                    release = item['release'].replace('.', '')
                    if observed != dict(completed=True, release=release, revision=REV, tree=TREE, handlers=11,
                            witness_handlers=1, decode_rejected=1, published=10, broker_rejected=10,
                            consumer_exit=0, server_exit=0, cleanup=True):
                        raise ValueError('owned real transport incomplete')
                    wire = document(raw(prefix + 'wire.json'))
                    for test, (code, detail) in BROKER_ERRORS.items():
                        matches = [w for w in wire if w.get('label') == test]
                        if len(matches) != 1 or not matches[0]['response'].startswith('-ERR ' + code) or detail not in matches[0]['response']:
                            raise ValueError('missing independently observed broker rejection')
                    for test in ('broker-accepted-codec-invalid-time', 'duplicate-valid-last-recovery'):
                        if [w.get('response') for w in wire if w.get('label') == test] != ['+OK']:
                            raise ValueError('broker/decoder rejection not distinguished')
                    consumer = text(prefix + 'consumer.stdout')
                    for marker in ('TASK101_EVENT DECODE_ERROR invalid named Time',
                                   'TASK101_ADA_REAL_DLZ_COMPLETE handlers= 11 rejected= 1',
                                   'TASK101_DECODE_RECOVERY_COMPLETE', 'TASK101_CLEANUP_COMPLETE'):
                        if consumer.splitlines().count(marker) != 1:
                            raise ValueError('actual generated Ada transport did not execute')
                    if (text(prefix + 'sleet-source.txt').splitlines() != [REV, TREE] or
                            text(prefix + 'consumer.exit') != '0\n' or text(prefix + 'server.exit') != '0\n' or
                            'route PUB message' not in text(prefix + 'server.stdout')):
                        raise ValueError('wrong/failed actual broker transport')
    if set(labels) != required:
        raise ValueError('missing/unexpected owned execution steps')
    required_files = {'generator.sha256'} | {label + suffix for label in required
                     for suffix in ('.command.json', '.stdout', '.stderr', '.exit')}
    required_files |= {item['manifest']['task_id'] + '-' + p + '-O' + o + '/policy.adc'
                       for item in records for p in POLICIES for o in OPTIMIZATIONS}
    required_files |= {item['manifest']['task_id'] + '-' + p + '-O' + o + '/' + source
                       for item in records for p in POLICIES for o in OPTIMIZATIONS
                       for source in item['manifest']['sources']}
    if not required_files <= result['files'].keys():
        raise ValueError('mandatory owned logs omitted from hashes')
    return dict(compiler=VERSIONS[compiler], mode=mode, cells=len(expected), tasks=len(records),
                sha=sha, tree=tree, run_id=str(run_id), run_attempt=str(attempt))