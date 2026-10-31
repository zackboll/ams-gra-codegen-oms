"""Independent Task098-derived real DLZ inputs; no generator/codec oracle."""
import copy
import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('task101_task098_real', ROOT / 'scripts/task098/real.py')
real = importlib.util.module_from_spec(spec)
spec.loader.exec_module(real)


def positives(release):
    base = json.loads((ROOT / f'tests/fixtures/task101/uci{release}-full-dlz.json').read_text())
    if base != real.document(release):
        raise ValueError('Task098 independent full DLZ fixture drift')
    values = [('full', base, base)]
    for name in real.TIMES[2:]:
        value = real.removed(base, real.data_path(name))
        values.append(('absent-' + name, value, value))
    for index, (path, value, expected) in enumerate(real.BOUNDARY):
        values.append((f'boundary-{index}', real.cases.replaced(base, path, value),
                       real.cases.replaced(base, path, expected)))
    return values


def negatives(release):
    base = positives(release)[0][1]
    values = []
    for index, (path, value, reason) in enumerate(real.INVALID):
        values.append((f'negative-{index}', real.cases.replaced(base, path, value), reason))
    values.append(('missing-time', real.removed(base, real.data_path('TimeOfIntercept')), 'missing required Time'))
    return values


def broker_cases(release):
    base = positives(release)[0][1]
    values = []
    for label, member, value, error in [
        ('missing-time', 'TimeOfIntercept', 'REMOVE', 'MissingRequiredMember'),
        ('offset-time', 'TimeOfIntercept', '12:34:56+00:00', 'FacetViolation'),
        ('null-time', 'TimeOfIntercept', None, 'TypeMismatch'),
        ('number-time', 'TimeOfIntercept', 123456, 'TypeMismatch'),
        ('quoted-float64', 'RangeMinimum', '1.0', 'TypeMismatch'),
    ]:
        doc = copy.deepcopy(base)
        if value == 'REMOVE':
            del doc['MessageData']['DLZ_Data'][member]
        else:
            doc['MessageData']['DLZ_Data'][member] = value
        values.append((label, 'PUB task098-dlz ' + json.dumps({'DLZ': doc}), '-ERR Invalid-Message', error))
    doc = copy.deepcopy(base)
    doc['SecurityInformation']['OwnerProducer'] = [{'GovernmentIdentifier': 'USA', 'NATO_SpecialWord': 'NATO:COSMIC'}]
    values += [
        ('choice-two-arms', 'PUB task098-dlz ' + json.dumps({'DLZ': doc}), '-ERR Invalid-Message', 'ChoiceViolation'),
        ('wrong-global', 'PUB task098-dlz ' + json.dumps({'DLZ_MT': base}), '-ERR Invalid-Message', 'UnknownGlobalElement'),
        ('wrong-topic', 'PUB forbidden ' + json.dumps({'DLZ': base}), '-ERR Illegal-State', 'not authorized to publish to topic forbidden'),
        ('unknown-sid', 'UNSUB 987', '-ERR Illegal-State', 'unknown subscription id 987'),
    ]
    # The documented codec and broker both use last-value-wins, not a blanket
    # duplicate rejection policy. Reject only the invalid effective member.
    raw = json.dumps({'DLZ': base})
    needle = '"TimeOfIntercept": "12:34:56.5000Z"'
    values.append(('duplicate-invalid-last', 'PUB task098-dlz ' + raw.replace(needle, needle + ', "TimeOfIntercept": 123456'),
                   '-ERR Invalid-Message', 'TypeMismatch'))
    return values