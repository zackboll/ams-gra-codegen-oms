"""Offline helper and adversarial evidence tests; never presented as live passes."""
import json
from pathlib import Path
import re
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task110 import cases, verify
from task109 import relay
import pr_smoke


def synthetic_cell():
    """Authored SYNTHETIC transcript for mutation tests, not broker evidence."""
    files = {}

    def put(name, value):
        files['cell/' + name] = (value if isinstance(value, str) else json.dumps(value)).encode()

    plan = cases.corpus()
    counts = dict.fromkeys(cases.MESSAGES, 0)
    errors = 0
    output = ['TASK110_UPGRADE_OWP_COMPLETE', 'TASK110_SIDS 1 2 3', 'TASK110_EVENT INITIALIZED 0 synthetic']
    output += ['TASK110_OWNERSHIP_REJECTED ' + str(i) for i in range(1, 10)]
    wire = [{'send': 'INIT synthetic'}] + [{'send': 'SUB synthetic'}] * 3
    observations = []
    for label, command, prefix, detail in cases.broker_cases():
        wire += [{'send': command}, {'receive': prefix + ' ' + detail}]
        observations.append(dict(label=label, stage='OWP authorization/state rejection'
            if prefix == '-ERR Illegal-State' else 'Broker/XSD rejection', response=prefix + ' ' + detail))
    for index, item in enumerate(plan, 1):
        key = item['function']
        if item['action'] == 'external':
            wire += [{'send': 'PUB ' + cases.TOPICS[key] + ' ' + json.dumps({cases.MESSAGES[key]: item['document']})}]
        if item['action'] != 'remove':
            payload = item['expected'] if item['action'] == 'publish' else item['document']
            wire += [{'receive': 'MSG ' + str(101 + list(cases.MESSAGES).index(key)) + ' ' +
                      json.dumps({cases.MESSAGES[key]: payload})}]
            if item['action'] != 'publish-retired':
                output += ['TASK110_DELIVERY ' + str(1 + list(cases.MESSAGES).index(key)) + ' ' +
                           cases.MESSAGES[key] + ' ' + json.dumps({cases.MESSAGES[key]: payload})]
        if 'expected' in item:
            counts[key] += 1
            output += [f'TASK110_HANDLER {key} {counts[key]}']
        if 'error' in item:
            errors += 1
            output += ['TASK110_DECODE_ERROR ' + str(1 + list(cases.MESSAGES).index(key)) + ' ' + cases.MESSAGES[key]]
        output += [f'TASK110_DONE {index} counts' + ''.join(' ' + str(counts[k]) for k in counts) + f' errors {errors}']
    wire += [{'send': 'UNSUB ' + str(sid)} for sid in (101, 102, 103)]
    wire += [{'receive': '+OK'}] * (7 + sum(p['action'] == 'external' for p in plan))
    output += ['TASK110_CLEANUP_COMPLETE lost_events=0']
    put('plan.jsonl', ''.join(json.dumps(p) + '\n' for p in plan))
    put('consumer.stdout', '\n'.join(output) + '\n')
    put('consumer.exit', '0\n'); put('server.exit', '0\n')
    put('sleet-source.txt', verify.REV + '\n' + verify.TREE + '\n')
    put('sleet-binary.sha256', 'c' * 64 + '\n')
    log = 'connection{id=1}: recv operation="INIT"\n'
    log += ''.join(f'connection{{id=1}}: subscription registered sid={i} topic={cases.TOPICS[k]} message_name={cases.MESSAGES[k]}\n'
                   for i, k in enumerate(cases.MESSAGES, 1))
    log += 'connection{id=1}: recv operation="UNSUB"\n' * 3 + 'shutdown complete\n'
    put('server.stdout', log)
    put('observations.json', observations)
    put('wire.json', wire)
    put('result.json', dict(completed=True, cleanup=True, counts=counts, decode_errors=errors,
        broker_rejections=len(cases.broker_cases()), steps=len(plan), consumer_exit=0,
        server_exit=0, witness_deliveries=cases.expected_witnesses()))
    return files


class QualificationTests(unittest.TestCase):
    def test_independent_release_corpus(self):
        plan = cases.corpus()
        self.assertEqual(len(plan), 62)
        self.assertEqual(cases.expected_counts(), dict(log=17, rdma=19, survivability=19))
        self.assertEqual(cases.expected_errors(), 5)
        self.assertEqual(cases.expected_witnesses(), 61)
        for item in plan:
            if 'document' in item:
                self.assertEqual(item['document']['MessageHeader']['SchemaVersion'], '002.6.0')
        for key, number in zip(cases.MESSAGES, (102, 103, 105), strict=True):
            independent = cases.family(number)
            for index, (_, document, expected) in enumerate(independent.positives('26')):
                items = [p for p in plan if p['function'] == key and p.get('variant') == index][:2]
                self.assertEqual([p['action'] for p in items], ['external', 'publish'])
                self.assertEqual(items[0]['document'], document)
                self.assertEqual(items[1]['expected'], expected)

    def test_recovery_to_all_families(self):
        plan = cases.corpus()
        for index, item in enumerate(plan):
            if 'error' in item:
                survivors = plan[index + 1:index + 4]
                self.assertEqual([p['function'] for p in survivors], list(cases.MESSAGES))
                self.assertTrue(all('expected' in p for p in survivors))

    def test_security_constraints(self):
        item = next(p for p in cases.corpus() if p.get('label') == 'maximum' and p['function'] == 'survivability')
        security = item['expected']['SecurityInformation']
        self.assertEqual(security['ClassifiedBy'], '  Task105  authority  ')
        self.assertEqual(security['DeclassDate'], '2026-01-02T03:04:05Z')
        self.assertEqual(security['CUI_DecontrolDate'], '2026-01-03T03:04:05Z')
        labels = {p[0] for p in cases.broker_cases()}
        self.assertTrue({'survivability-empty-classified-by', 'survivability-empty-sar',
                         'survivability-declass-offset', 'survivability-decontrol-offset'} <= labels)

    def test_consumer_only_authorized_release_differences(self):
        old = (ROOT / 'tests/ada-real-uci-multimessage/multimessage_consumer.adb').read_text()
        new = (ROOT / 'tests/ada-real-uci-multimessage26/multimessage_consumer.adb').read_text()
        security = re.search(r'      --  Task110 UCI 2.6 security assertions,.*?      --  End Task110 security assertions.\n', new, re.S)
        self.assertIsNotNone(security)
        for check in ('named declass UTC', 'named decontrol UTC', 'preserved authority whitespace', 'SAR value'):
            self.assertIn(check, security.group())
        restored = new.replace(security.group(), '').replace('with Ada.Environment_Variables;\n', '')
        restored = restored.replace('Task110', 'Task106').replace('task110', 'task106').replace('TASK110', 'TASK106')
        restored = restored.replace('002.6.0', '002.5.0').replace('446655440110', '446655440106')
        restored = restored.replace('Message.MessageData.Data.Value.Value;', 'Message.MessageData.Data.Value;')
        restored = restored.replace('   WS.Open (Wire, Ada.Command_Line.Argument (1),\n'
            '     CA_Path => Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA", ""));',
            '   WS.Open (Wire, Ada.Command_Line.Argument (1));')
        self.assertEqual(restored, old)

    def test_relay_defaults_and_opt_in(self):
        relay.validate_init(dict(schema='002.5.0', service_id='task106-multimessage'))
        relay.validate_init(dict(schema='002.6.0', service_id='task110-multimessage'), '002.6.0', 'task110-multimessage')
        for schema, service in [('002.6.0', 'task106-multimessage'), ('002.5.0', 'task110-multimessage')]:
            with self.assertRaises(ValueError):
                relay.validate_init(dict(schema=schema, service_id=service))

    def test_selector(self):
        for path in ('scripts/task110/qualify.py', 'tests/fixtures/task110/uci26-service.yaml'):
            self.assertEqual(pr_smoke.classify([path])[0], {'ada'})
        self.assertEqual(pr_smoke.classify(['scripts/task1100/probe.py'])[0], set(pr_smoke.BACKENDS))

    def test_synthetic_verifier_control(self):
        files = synthetic_cell()
        self.assertTrue(verify.verify_cell(files.__getitem__, 'cell')['completed'])

    def test_forged_delivery_order_rejected(self):
        files = synthetic_cell()
        lines = files['cell/consumer.stdout'].decode().splitlines()
        indexes = [i for i, line in enumerate(lines) if line.startswith('TASK110_DELIVERY ')][::2][:2]
        a, b = indexes; lines[a], lines[b] = lines[b], lines[a]
        files['cell/consumer.stdout'] = ('\n'.join(lines) + '\n').encode()
        with self.assertRaises(ValueError):
            verify.verify_cell(files.__getitem__, 'cell')

    def test_forged_payload_release_rejected(self):
        files = synthetic_cell()
        files['cell/consumer.stdout'] = files['cell/consumer.stdout'].replace(b'002.6.0', b'002.5.0', 1)
        with self.assertRaises(ValueError):
            verify.verify_cell(files.__getitem__, 'cell')

    def test_missing_recovery_rejected(self):
        files = synthetic_cell()
        files['cell/consumer.stdout'] = files['cell/consumer.stdout'].replace(b'TASK110_DONE 38 ', b'TASK110_MISSING 38 ')
        with self.assertRaises(ValueError):
            verify.verify_cell(files.__getitem__, 'cell')

    def test_wrong_rejection_stage_rejected(self):
        files = synthetic_cell()
        files['cell/observations.json'] = files['cell/observations.json'].replace(b'Broker/XSD rejection', b'Ada rejection', 1)
        with self.assertRaises(ValueError):
            verify.verify_cell(files.__getitem__, 'cell')

    def test_missing_cleanup_rejected(self):
        files = synthetic_cell()
        files['cell/consumer.stdout'] = files['cell/consumer.stdout'].replace(b'TASK110_CLEANUP_COMPLETE', b'MISSING')
        with self.assertRaises(ValueError):
            verify.verify_cell(files.__getitem__, 'cell')


if __name__ == '__main__':
    unittest.main()