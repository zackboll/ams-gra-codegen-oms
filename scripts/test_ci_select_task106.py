"""Task106 adversarial SYNTHETIC verifier fixtures, never live evidence."""
import copy
import json
import unittest
from unittest.mock import patch

import ci_policy
import ci_selection
from ci_retry_test_data import metadata
from test_ci_select import change, gate
from task106 import cases
from task106_evidence import JOB, REV, TREE, verify_bundle, verify_cell
import task106_evidence as evidence


def synthetic_cell():
    """Independent hand-built transcript sufficient for cell-verifier controls."""
    files = {}
    def put(name, value):
        files['cell/' + name] = (value if isinstance(value, str) else json.dumps(value)).encode()
    plan = cases.corpus()
    counts = dict.fromkeys(cases.MESSAGES, 0)
    errors = 0
    output = ['TASK106_UPGRADE_OWP_COMPLETE', 'TASK106_SIDS 1 2 3', 'TASK106_EVENT INITIALIZED 0 synthetic']
    output += ['TASK106_OWNERSHIP_REJECTED ' + str(i) for i in range(1, 10)]
    observations = []
    wire = [dict(subprotocol='owp'), dict(send='INIT synthetic'), dict(receive='+OK')]
    for index, key in enumerate(cases.MESSAGES, 101):
        wire += [dict(send=f'SUB {index} {cases.MESSAGES[key]} {cases.TOPICS[key]}'), dict(receive='+OK')]
    for label, command, prefix, detail in cases.broker_cases():
        response = prefix + ' synthetic ' + detail
        wire += [dict(send=command), dict(receive=response)]
        observations.append(dict(label=label, stage='OWP authorization/state rejection' if
                                 prefix == '-ERR Illegal-State' else 'Broker/XSD rejection', response=response))
    for index, item in enumerate(plan, 1):
        key = item['function']
        if item['action'] == 'external':
            wire += [dict(send='PUB ' + cases.TOPICS[key] + ' ' + json.dumps({cases.MESSAGES[key]: item['document']})), dict(receive='+OK')]
        if item['action'] != 'remove':
            document = item['expected'] if item['action'] == 'publish' else item['document']
            wire.append(dict(receive=f'MSG {101 + list(cases.MESSAGES).index(key)} ' + json.dumps({cases.MESSAGES[key]: document})))
        if item['action'] in ('external', 'publish'):
            document = item['expected'] if item['action'] == 'publish' else item['document']
            output.append(f'TASK106_DELIVERY {1 + list(cases.MESSAGES).index(key)} {cases.MESSAGES[key]} ' + json.dumps({cases.MESSAGES[key]: document}))
        if 'error' in item:
            errors += 1
            output.append('TASK106_DECODE_ERROR ' + str(1 + list(cases.MESSAGES).index(key)) + ' ' + cases.MESSAGES[key])
        if 'expected' in item:
            counts[key] += 1
            output.append(f'TASK106_HANDLER {key} {counts[key]}')
        output.append(f'TASK106_DONE {index} counts' + ''.join(' ' + str(counts[k]) for k in counts) + f' errors {errors}')
    output.append('TASK106_CLEANUP_COMPLETE lost_events=0')
    for sid in (101, 102, 103):
        wire += [dict(send=f'UNSUB {sid}'), dict(receive='+OK')]
    put('plan.jsonl', ''.join(json.dumps(p) + '\n' for p in plan))
    put('consumer.stdout', '\n'.join(output) + '\n')
    put('consumer.exit', '0\n'); put('server.exit', '0\n')
    put('sleet-source.txt', REV + '\n' + TREE + '\n')
    put('sleet-binary.sha256', 'c' * 64 + '\n')
    log = 'connection{id=1}: recv operation="INIT"\n'
    log += ''.join(f'connection{{id=1 service_id="task106-multimessage"}}: subscription registered sid={i} topic={cases.TOPICS[k]} message_name={cases.MESSAGES[k]}\n'
                   for i, k in enumerate(cases.MESSAGES, 1))
    log += 'connection{id=1}: recv operation="UNSUB"\n' * 3 + 'shutdown complete\n'
    put('server.stdout', log)
    put('observations.json', observations)
    put('wire.json', wire)
    put('result.json', dict(completed=True, cleanup=True, counts=counts, decode_errors=errors,
        broker_rejections=len(cases.broker_cases()), steps=len(plan), consumer_exit=0,
        server_exit=0, witness_deliveries=cases.expected_witnesses()))
    return files


def synthetic_bundle_prefix(compiler='14'):
    """Synthetic metadata through the generated-source boundary, not live data."""
    files = {}
    def put(name, value):
        files[name] = (value if isinstance(value, str) else json.dumps(value)).encode()
    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    put('integration-identity.json', dict(sha='a'*40, tree='b'*40, run_id='123', run_attempt='2', compiler=compiler))
    put('campaign/source.stdout', 'a'*40 + '\n' + 'b'*40 + '\n')
    put('campaign/source-status.stdout', '')
    put('campaign/gnat-version.stdout', 'GNAT ' + version + '\n')
    put('campaign/alire-version.stdout', 'alr 2.1.1\n')
    from task105_evidence import TOOL_HASHES
    put('campaign/tool-identities.stdout', '\n'.join(TOOL_HASHES[compiler].values()) +
        '\naff9fde5ca766d5497da9dec185cb8b96878206a0d02512aa5685e1e56d0a4d3\n')
    guard = evidence.importlib.util.spec_from_file_location('task106_synthetic_lock', evidence.ROOT / 'scripts/check-task087a.py')
    module = evidence.importlib.util.module_from_spec(guard); guard.loader.exec_module(module)
    states = []
    for name, v in [('gnat_external' if compiler == '14' else 'gnat_native', version),
                    ('gnatcoll', '26.0.0'), ('gnatcoll_minimal', '26.0.0'), ('gprbuild', '26.0.1')]:
        states.append(dict(release=dict(name=name, version=v, origin=dict(hashes=[module.PROVIDER_HASH,
            'sha256:e3f27f2515ec04d963f6badade6595993b1c091ba15d1919a7c75aad1b7ed49b',
            'sha256:9f74f58a827a2ad40dd84c72a413e75ea52888e0d8f7e252fba4d26762402703']))))
    if compiler == '16':
        states.append(copy.deepcopy(states[0]))  # authentic Alire alias shape
    put('campaign/toolchain-lock.json', dict(solution=dict(state=states)))
    put('campaign/input-sources.json', evidence.inputs())
    steps = []
    for label in sorted(evidence.required_steps()):
        negative = label.startswith('direction-')
        exit_code = 1 if negative else 0
        argv = ['synthetic-metadata-step']
        if label.endswith('-build') or negative:
            argv = ['gprbuild', '-p', '-P', 'consumer.gpr']
            if label.startswith('uci25-'):
                argv += ['-gnato', '-' + label.split('-')[2], '-gnatec=/synthetic/policy.adc']
        elif label in ('check', 'generate', 'model') or label.startswith('baseline-'):
            argv = ['/synthetic/generator', 'service-check' if label == 'check' else 'service-generate',
                    '--schema', '/synthetic/root.xsd', '--contract', '/synthetic/contract.yaml',
                    '--language', 'ada', '--world', 'closed-schema']
            if label != 'model': argv.append('--with-codec')
        elif label.endswith('-sleet'):
            argv = ['python', '/synthetic/scripts/task106/sleet.py', '--evidence', '/e', '--checkout', '/s', '--schema', '/x', '--consumer', '/c']
        elif label == 'admission':
            argv = ['python', '/synthetic/scripts/task106/admission.py', '--evidence', '/e', '--sleet', '/s', '--uci25', '/x']
        elif label.startswith('codec-'):
            key = label.split('-')[-1]
            _, executable, fixture, _ = next(c for c in evidence.CONSUMERS if c[0] == key)
            argv = ['/synthetic/bin/' + executable, '/synthetic/tests/fixtures/service-generate/' + fixture]
        elif label.startswith('independent-') and label.endswith('-execute'):
            argv = ['/synthetic/bin/codec_probe', '/synthetic/documents.jsonl', '/synthetic/reasons.txt']
        put('campaign/' + label + '.command.json', dict(argv=argv, cwd='/synthetic'))
        put('campaign/' + label + '.exit', str(exit_code))
        if negative:
            put('campaign/' + label + '.stderr', '"' + ('Publisher' if label.endswith('-receive') else 'Subscriber') + '" not declared')
        steps.append(dict(label=label, exit=exit_code))
    put('campaign/result.json', dict(completed=True, development=False, compiler=version,
        cells=[dict(release='25', policy=p, optimization=o) for p in ('Normal', 'Check', 'Ignore') for o in ('0', '2')], steps=steps))
    put('campaign/generated/programs.ads', 'synthetic, deliberately NOT qualified generated source')
    return files


class CellControls(unittest.TestCase):
    def verify(self, files):
        return verify_cell(lambda name: files[name], 'cell')

    def test_synthetic_healthy_is_not_interoperability_evidence(self):
        self.assertEqual(self.verify(synthetic_cell())['counts'], {'log': 17, 'rdma': 19, 'survivability': 19})

    def test_missing_duplicate_wrong_routing_isolation_and_cleanup(self):
        mutations = {
            'duplicate-sids': ('TASK106_SIDS 1 2 3', 'TASK106_SIDS 1 1 3'),
            'missing-sids': ('TASK106_SIDS 1 2 3', ''),
            'wrong-global': ('TASK106_DELIVERY 1 Log ', 'TASK106_DELIVERY 1 LogMT '),
            'wrong-sid': ('TASK106_DELIVERY 1 Log ', 'TASK106_DELIVERY 2 Log '),
            'missing-handler': ('TASK106_HANDLER rdma 1\n', ''),
            'wrong-handler': ('TASK106_HANDLER rdma 1\n', 'TASK106_HANDLER log 99\n'),
            'missing-isolation': ('TASK106_DONE 38 ', 'MISSING_DONE 38 '),
            'missing-recovery': ('TASK106_HANDLER survivability 13\n', ''),
            'missing-error': ('TASK106_DECODE_ERROR 1 Log\n', ''),
            'wrong-error-sid': ('TASK106_DECODE_ERROR 1 Log', 'TASK106_DECODE_ERROR 2 Log'),
            'lost-events': ('lost_events=0', 'lost_events=1'),
            'cleanup': ('TASK106_CLEANUP_COMPLETE', 'NO_CLEANUP'),
            'unexpected-event': ('TASK106_EVENT INITIALIZED', 'TASK106_EVENT PROTOCOL_ERROR'),
        }
        for label, (old, new) in mutations.items():
            files = synthetic_cell()
            self.assertIn(old.encode(), files['cell/consumer.stdout'])
            files['cell/consumer.stdout'] = files['cell/consumer.stdout'].replace(old.encode(), new.encode(), 1)
            with self.subTest(label=label), self.assertRaises(ValueError):
                self.verify(files)
        for name, value in [('consumer.exit', '1'), ('server.exit', '1'), ('sleet-source.txt', REV + '\n' + '0'*40),
                            ('server.stdout', 'shutdown complete'), ('wire.json', '[]')]:
            files = synthetic_cell(); files['cell/' + name] = value.encode()
            with self.subTest(file=name), self.assertRaises(ValueError): self.verify(files)

    def test_retired_witness_and_rejection_stage_are_mandatory(self):
        for mutation in ('witness', 'broker-is-not-codec', 'skip-send', 'failed-cell'):
            files = synthetic_cell()
            if mutation == 'witness':
                wire = json.loads(files['cell/wire.json'])
                del wire[next(i for i, entry in enumerate(wire) if entry.get('receive', '').startswith('MSG '))]
                files['cell/wire.json'] = json.dumps(wire).encode()
            elif mutation == 'broker-is-not-codec':
                files['cell/observations.json'] = files['cell/observations.json'].replace(b'Broker/XSD rejection', b'Generated Ada codec rejection', 1)
            elif mutation == 'skip-send':
                files['cell/wire.json'] = files['cell/wire.json'].replace(b'PUB task106-log', b'PUB skipped', 1)
            else:
                files['cell/result.json'] = files['cell/result.json'].replace(b'"completed": true', b'"completed": false')
            with self.subTest(mutation=mutation), self.assertRaises(ValueError): self.verify(files)


class SelectionAndAggregate(unittest.TestCase):
    def test_owned_and_shared_sources_schedule_both_compilers(self):
        policy = ci_selection.inventory()
        for path in ('scripts/task106/qualify.py', 'scripts/task106_evidence.py',
                     'tests/ada-real-uci-multimessage/multimessage_consumer.adb',
                     'tests/fixtures/task106/uci25-service.yaml', 'ada/oms-lacal-runtime/src/oms_lacal-typed.adb',
                     'crates/backend-ada/src/lib.rs', 'crates/codegen-core/src/lib.rs',
                     'scripts/task102/cases.py', 'scripts/task103/cases.py', 'scripts/task104/cases.py'):
            manifest = ci_selection.select([change(path)], policy)
            with self.subTest(path=path):
                self.assertIn(JOB, manifest['deep_jobs'])
                self.assertEqual([c['variant'] for r in policy['jobs'] if r['job_id'] == JOB for c in r['cells']],
                                 [{'compiler': '14'}, {'compiler': '16'}])
        self.assertEqual(ci_policy.check(), 52)

    def test_retry_producer_and_inner_verifier_cannot_be_bypassed(self):
        artifacts = [dict(name='task106-ada-multimessage-' + c) for c in ('14', '16')]
        jobs, attempts = metadata(artifacts)
        manifest = dict(deep_jobs=[JOB])
        with patch.object(gate, 'verify_task106_bundle', return_value=dict(cells=6)) as verify:
            result = gate.reconcile_task106(artifacts, manifest, 'a'*40, 'b'*40, '123', '2',
                                            lambda item: {}, jobs=jobs, attempts=attempts)
            self.assertEqual(len(result), 2); self.assertEqual(verify.call_count, 2)
        for mutation in ('missing', 'duplicate', 'expired', 'wrong-attempt', 'skipped-step'):
            changed, history, current = copy.deepcopy(artifacts), copy.deepcopy(attempts), copy.deepcopy(jobs)
            if mutation == 'missing': changed.pop()
            elif mutation == 'duplicate': changed.append(copy.deepcopy(changed[0]))
            elif mutation == 'expired': changed[0]['expired'] = True
            elif mutation == 'wrong-attempt': history[2]['jobs'][0]['run_attempt'] = 1
            else: current[0]['steps'][0]['conclusion'] = 'skipped'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                gate.reconcile_task106(changed, manifest, 'a'*40, 'b'*40, '123', '2',
                                      lambda item: {}, jobs=current, attempts=history)
        with patch.object(gate, 'verify_task106_bundle', side_effect=ValueError('invalid source bundle')), self.assertRaises(ValueError):
            gate.reconcile_task106(artifacts, manifest, 'a'*40, 'b'*40, '123', '2', lambda item: {}, jobs=jobs, attempts=attempts)

    def test_bundle_identity_and_duplicate_files_fail_closed(self):
        identity = dict(sha='a'*40, tree='b'*40, run_id='123', run_attempt='2', compiler='14')
        for field in identity:
            changed = dict(identity); changed[field] = 'wrong'
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify_bundle({'integration-identity.json': json.dumps(changed).encode()}, '14', 'a'*40, 'b'*40, '123', '2')
        value = json.dumps(identity).encode()
        with self.assertRaises(ValueError):
            verify_bundle({'integration-identity.json': value, 'duplicate/integration-identity.json': value}, '14', 'a'*40, 'b'*40, '123', '2')

    def test_synthetic_matrix_steps_commands_and_source_parity_reject(self):
        for compiler in ('14', '16'):
            for mutation in ('forged-generated', 'dirty', 'missing-cell', 'duplicate-cell', 'wrong-release',
                             'development', 'skipped-step', 'failed-step', 'duplicate-step', 'wrong-direction',
                             'wrong-build', 'source-drift', 'wrong-tool', 'archive-drift'):
                files = synthetic_bundle_prefix(compiler)
                result = json.loads(files['campaign/result.json'])
                expected = 'forged generated-source'
                if mutation == 'dirty':
                    files['campaign/source-status.stdout'] = b'M file'; expected = 'dirty/wrong source'
                elif mutation == 'missing-cell': result['cells'].pop(); expected = 'matrix cells'
                elif mutation == 'duplicate-cell': result['cells'][-1] = result['cells'][0]; expected = 'matrix cells'
                elif mutation == 'wrong-release': result['cells'][0]['release'] = '26'; expected = 'matrix cells'
                elif mutation == 'development': result['development'] = True; expected = 'matrix cells'
                elif mutation == 'skipped-step': result['steps'].pop(); expected = 'mandatory step'
                elif mutation == 'duplicate-step': result['steps'].append(result['steps'][0]); expected = 'mandatory step'
                elif mutation == 'failed-step':
                    step = next(s for s in result['steps'] if s['label'] == 'generate'); step['exit'] = 1
                    files['campaign/generate.exit'] = b'1'; expected = 'failed mandatory'
                elif mutation == 'wrong-direction':
                    files['campaign/direction-log-receive.stderr'] = b'unrelated syntax failure'; expected = 'direction diagnostic'
                elif mutation == 'wrong-build':
                    files['campaign/uci25-Normal-O0-build.command.json'] = json.dumps(dict(argv=['true'], cwd='/synthetic')).encode(); expected = 'builder command'
                elif mutation == 'source-drift': files['campaign/input-sources.json'] = b'{}'; expected = 'authoritative source'
                elif mutation == 'wrong-tool': files['campaign/tool-identities.stdout'] = b'wrong'; expected = 'binary identity'
                elif mutation == 'archive-drift':
                    lock = json.loads(files['campaign/toolchain-lock.json'])
                    lock['solution']['state'][-1]['release']['origin']['hashes'] = []
                    files['campaign/toolchain-lock.json'] = json.dumps(lock).encode(); expected = 'archive pin'
                files['campaign/result.json'] = json.dumps(result).encode()
                with self.subTest(compiler=compiler, mutation=mutation), self.assertRaisesRegex(ValueError, expected):
                    verify_bundle(files, compiler, 'a'*40, 'b'*40, '123', '2')


if __name__ == '__main__':
    unittest.main()