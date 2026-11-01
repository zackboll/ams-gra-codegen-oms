"""Independent real Log inputs and canonical expectations; no codec oracle."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def golden(release):
    return json.loads((ROOT / f'tests/fixtures/service-generate/task093-real-log{release}.json').read_text())


def positives(release):
    base = golden(release)
    values = [('full', base, base)]
    for label, data in [('absent', None), ('empty', ''), ('populated', 'FF008001007F')]:
        doc = copy.deepcopy(base)
        doc['MessageData']['Label'] = 'Task102-' + label
        doc['MessageData']['Details'] = 'Independent optional binary transport control'
        if data is None:
            del doc['MessageData']['Data']
        else:
            doc['MessageData']['Data'] = data
        values.append((label, doc, doc))
    for label, spelling in [('lowercase', '0080ff'), ('xml-whitespace', ' \t0080ff\r\n')]:
        doc = copy.deepcopy(base)
        doc['MessageData']['Data'] = spelling
        values.append((label, doc, base))
    return values


def decoder_invalid(release):
    # Pinned Sleet treats unconstrained hexBinary as string. These documents
    # must reach the Ada decoder, not be counted as broker schema rejections.
    values = []
    for label, spelling in [('odd-hex', '000'), ('invalid-digit', '0G'), ('internal-space', 'AA BB')]:
        doc = copy.deepcopy(golden(release))
        doc['MessageData']['Data'] = spelling
        values.append((label, doc))
    return values


def broker_cases(release):
    base = golden(release)
    values = []
    for label, value in [('null', None), ('number', 7), ('array', []), ('object', {})]:
        doc = copy.deepcopy(base)
        doc['MessageData']['Data'] = value
        error = 'OccurrenceMismatch' if label == 'array' else 'TypeMismatch'
        values.append((label + '-data', 'PUB task102-log ' + json.dumps({'Log': doc}), '-ERR Invalid-Message', error))
    doc = copy.deepcopy(base)
    doc['SecurityInformation']['OwnerProducer'] = [{'GovernmentIdentifier': 'USA', 'NATO_SpecialWord': 'NATO:EXAMPLE'}]
    values.append(('choice-two-arms', 'PUB task102-log ' + json.dumps({'Log': doc}), '-ERR Invalid-Message', 'ChoiceViolation'))
    doc = copy.deepcopy(base)
    del doc['MessageHeader']['SystemID']
    values.append(('missing-header', 'PUB task102-log ' + json.dumps({'Log': doc}), '-ERR Invalid-Message', 'MissingRequiredMember'))
    values += [
        ('wrong-global', 'PUB task102-log ' + json.dumps({'LogMT': base}), '-ERR Invalid-Message', 'UnknownGlobalElement'),
        ('wrong-topic', 'PUB forbidden ' + json.dumps({'Log': base}), '-ERR Illegal-State', 'not authorized'),
        ('unknown-sid', 'UNSUB 987', '-ERR Illegal-State', 'unknown subscription id 987'),
        ('malformed-document', 'PUB task102-log {', '-ERR Invalid-Message', 'invalid JSON payload'),
    ]
    return values