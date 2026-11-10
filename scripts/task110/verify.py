#!/usr/bin/env python3
"""Source-authored UCI 2.6 callback, broker, recovery and actual TLS wire verifier."""
import argparse
import json
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from task110 import cases

REV = '94796daf55ebd1134dce0925e6a204a05ad3423b'
TREE = '47da595b9078cac1a9351e8290d1ae7f3976367f'


def verify_cell(raw, prefix):
    def text(name):
        return raw(prefix + '/' + name).decode()
    def doc(name):
        return json.loads(text(name))
    plan = cases.corpus()
    if [json.loads(line) for line in text('plan.jsonl').splitlines()] != plan:
        raise ValueError('Task110 authored plan differs')
    result = doc('result.json')
    expected = dict(completed=True, cleanup=True, counts=cases.expected_counts(),
                    decode_errors=cases.expected_errors(), broker_rejections=len(cases.broker_cases()),
                    steps=len(plan), consumer_exit=0, server_exit=0, witness_deliveries=cases.expected_witnesses())
    if 'error' in result or any(result.get(k) != v for k, v in expected.items()):
        raise ValueError('Task110 failed/incomplete live cell')
    if text('consumer.exit').strip() != '0' or text('server.exit').strip() != '0':
        raise ValueError('Task110 consumer/server exit mismatch')
    output = text('consumer.stdout')
    if ('TASK110_FAILED' in output or output.count('TASK110_UPGRADE_OWP_COMPLETE') != 1
            or output.count('TASK110_SIDS 1 2 3') != 1
            or output.count('TASK110_CLEANUP_COMPLETE lost_events=0') != 1):
        raise ValueError('Task110 missing/duplicate session/SIDs/cleanup')
    deliveries = [line for line in output.splitlines() if line.startswith('TASK110_DELIVERY ')]
    expected_deliveries = [p for p in plan if p['action'] in ('external', 'publish')]
    if len(deliveries) != len(expected_deliveries):
        raise ValueError('Task110 missing/duplicate broker delivery')
    for line, item in zip(deliveries, expected_deliveries, strict=True):
        _, sid, name, document = line.split(' ', 3)
        key = item['function']
        payload = item['expected'] if item['action'] == 'publish' else item['document']
        if sid != str(1 + list(cases.MESSAGES).index(key)) or name != cases.MESSAGES[key] or json.loads(document) != {name: payload}:
            raise ValueError('Task110 incorrect SID/global/document/order')
    counts = dict.fromkeys(cases.MESSAGES, 0)
    errors = 0
    done = [line for line in output.splitlines() if line.startswith('TASK110_DONE ')]
    if len(done) != len(plan):
        raise ValueError('Task110 missing cross-message isolation observation')
    for index, (item, line) in enumerate(zip(plan, done, strict=True), 1):
        if 'expected' in item:
            counts[item['function']] += 1
        errors += 'error' in item
        want = f'TASK110_DONE {index} counts' + ''.join(' ' + str(counts[k]) for k in cases.MESSAGES) + f' errors {errors}'
        if line != want:
            raise ValueError('Task110 incorrect cross-message isolation/recovery counts')
    for key, count in counts.items():
        actual = [line for line in output.splitlines() if line.startswith('TASK110_HANDLER ' + key + ' ')]
        if actual != [f'TASK110_HANDLER {key} {i}' for i in range(1, count + 1)]:
            raise ValueError('Task110 wrong typed handler sequence')
    rejected = [line for line in output.splitlines() if line.startswith('TASK110_DECODE_ERROR ')]
    want_errors = ['TASK110_DECODE_ERROR ' + str(1 + list(cases.MESSAGES).index(p['function'])) + ' ' +
                   cases.MESSAGES[p['function']] for p in plan if 'error' in p]
    if rejected != want_errors:
        raise ValueError('Task110 missing/wrong codec error SID/global')
    if [line for line in output.splitlines() if line.startswith('TASK110_OWNERSHIP_REJECTED')] != [
            'TASK110_OWNERSHIP_REJECTED ' + str(i) for i in range(1, 10)]:
        raise ValueError('Task110 binding ownership controls missing')
    events = [line for line in output.splitlines() if line.startswith('TASK110_EVENT ')]
    if sum(line.startswith('TASK110_EVENT INITIALIZED ') for line in events) != 1 or any(
            not line.startswith(('TASK110_EVENT INITIALIZED ', 'TASK110_EVENT COMMAND_ACCEPTED ', 'TASK110_EVENT DECODE_ERROR ')) for line in events):
        raise ValueError('Task110 ignored/unexpected connection event')
    if text('sleet-source.txt').splitlines() != [REV, TREE]:
        raise ValueError('Task110 broker pin changed')
    if not re.fullmatch('[0-9a-f]{64}', text('sleet-binary.sha256').strip()):
        raise ValueError('Task110 missing broker binary identity')
    log = text('server.stdout')
    registrations = re.findall(r'connection\{id=(\d+)[^\n]*subscription registered sid=([123]) topic=(\S+) message_name=(\S+)', log)
    if len(registrations) != 3 or len({r[0] for r in registrations}) != 1:
        raise ValueError('Task110 subscriptions do not share one connection')
    connection = registrations[0][0]
    if [(r[1], r[2], r[3]) for r in registrations] != [(str(i), cases.TOPICS[k], cases.MESSAGES[k]) for i, k in enumerate(cases.MESSAGES, 1)]:
        raise ValueError('Task110 broker registration mismatch')
    if len(re.findall(r'connection\{id=' + connection + r'[^\n]*recv operation="INIT"', log)) != 1 or len(
            re.findall(r'connection\{id=' + connection + r'[^\n]*recv operation="UNSUB"', log)) != 3:
        raise ValueError('Task110 unexpected INIT/UNSUB wire traffic')
    if 'shutdown complete' not in log:
        raise ValueError('Task110 broker shutdown missing')
    observations = doc('observations.json')
    negatives = observations[:len(cases.broker_cases())]
    for observed, (label, _, error_prefix, detail) in zip(negatives, cases.broker_cases(), strict=True):
        stage = 'OWP authorization/state rejection' if error_prefix == '-ERR Illegal-State' else 'Broker/XSD rejection'
        if observed.get('label') != label or observed.get('stage') != stage or not observed.get('response', '').startswith(error_prefix) or detail not in observed['response']:
            raise ValueError('Task110 wrong rejection stage/diagnostic')
    wire = doc('wire.json')
    witnessed = [item['receive'] for item in wire if item.get('receive', '').startswith('MSG ')]
    expected_witnessed = [p for p in plan if p['action'] != 'remove']
    if len(witnessed) != len(expected_witnessed):
        raise ValueError('Task110 missing actual witness delivery')
    for frame, item in zip(witnessed, expected_witnessed, strict=True):
        _, sid, document = frame.split(' ', 2)
        key = item['function']
        payload = item['expected'] if item['action'] == 'publish' else item['document']
        if sid != str(101 + list(cases.MESSAGES).index(key)) or json.loads(document) != {cases.MESSAGES[key]: payload}:
            raise ValueError('Task110 witness wrong global/SID/value/order')
    actual_sends = [entry['send'] for entry in wire if 'send' in entry]
    expected_sends = [command for _, command, _, _ in cases.broker_cases()]
    expected_sends += ['PUB ' + cases.TOPICS[p['function']] + ' ' + json.dumps({cases.MESSAGES[p['function']]: p['document']})
                       for p in plan if p['action'] == 'external']
    if actual_sends[4:-3] != expected_sends or actual_sends[-3:] != ['UNSUB 101', 'UNSUB 102', 'UNSUB 103']:
        raise ValueError('Task110 missing/changed independent sends')
    if sum(entry.get('receive') == '+OK' for entry in wire) != 7 + sum(p['action'] == 'external' for p in plan):
        raise ValueError('Task110 failed/missing publisher acceptance')
    return expected



def verify(directory):
    def raw(path):
        return (directory / path).read_bytes()

    expected = verify_cell(raw, "wss")
    tls = json.loads(raw("tls/result.json"))
    if (not tls.get("completed") or not tls.get("cleanup") or tls.get("errors")
            or tls.get("consumer_exit") != 0 or tls.get("tls") not in ("TLSv1.2", "TLSv1.3")
            or tls.get("partial_writes", 0) < 2 or tls.get("record_batches", 0) < 2):
        raise ValueError("missing verified WSS relay completion")
    wire = json.loads(raw("tls/wire.json"))
    messages = [frame["text"] for frame in wire["sleet"]["frames"]
                if frame["opcode"] == 1 and frame["text"].startswith("MSG ")]
    plan = [item for item in cases.corpus() if item["action"] in ("external", "publish")]
    if len(messages) != len(plan):
        raise ValueError("missing actual Ada TLS MSG frames")
    for message, item in zip(messages, plan, strict=True):
        _, sid, document = message.split(" ", 2)
        key = item["function"]
        payload = item["expected"] if item["action"] == "publish" else item["document"]
        if (int(sid) != 1 + list(cases.MESSAGES).index(key)
                or json.loads(document) != {cases.MESSAGES[key]: payload}):
            raise ValueError("actual WSS delivery SID/global/value/order mismatch")
    if (directory / "plaintext").exists():
        verify_cell(raw, "plaintext")
    return dict(completed=True, task110=expected, actual_wss_deliveries=len(messages))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.directory.resolve()), indent=2))