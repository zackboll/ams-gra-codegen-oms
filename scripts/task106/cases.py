"""Task106 frozen interleaving, reusing independent Task102--104 value oracles.

Counts are derived from this authored plan, never from observed callback counts.
The three historical goldens remain unchanged and are transmitted verbatim at
the admission gate. This module does not import generator or Ada codec logic.
"""
import copy
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MESSAGES = {'log': 'Log', 'rdma': 'RDMA_InitializeSetup', 'survivability': 'SurvivabilityRiskLevel'}
TOPICS = {key: 'task106-' + key for key in MESSAGES}


def family(number):
    spec = importlib.util.spec_from_file_location(f'task106_independent_{number}', ROOT / f'scripts/task{number}/cases.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def corpus():
    families = {key: family(number) for key, number in zip(MESSAGES, (102, 103, 104), strict=True)}
    plan = []
    # Six variants per family, alternated across payload types. Both independent
    # external publications and the generated Publisher must produce deliveries.
    positives = {key: module.positives('25') for key, module in families.items()}
    for index in range(6):
        for key in MESSAGES:
            label, document, expected = positives[key][index]
            for action in ('external', 'publish'):
                plan.append(dict(action=action, function=key, label=label,
                                 document=copy.deepcopy(document), expected=copy.deepcopy(expected), variant=index))
    # Each family's independently authored broker-accepted decoder negatives
    # must recover to ALL THREE payload types before another negative is sent.
    for key, module in families.items():
        for label, document in module.decoder_invalid('25'):
            plan.append(dict(action='external', function=key, label=label, document=document,
                             error='invalid named DateTime' if key != 'log' else 'hexBinary'))
            for survivor in MESSAGES:
                document = families[survivor].golden('25')
                plan.append(dict(action='external', function=survivor, label='recovery-after-' + key + '-' + label,
                                 document=document, expected=copy.deepcopy(document), variant=0))
    plan.append(dict(action='remove', function='log'))
    # Publication to a retired subscription is witnessed by an independent
    # Python subscriber; RDMA/Survivability remain active on the Ada session.
    plan.append(dict(action='publish-retired', function='log', document=families['log'].golden('25')))
    for key in ('rdma', 'survivability'):
        document = families[key].golden('25')
        for action in ('external', 'publish'):
            plan.append(dict(action=action, function=key, label='survives-log-unsubscribe',
                             document=document, expected=copy.deepcopy(document), variant=0))
    return plan


def expected_counts():
    plan = corpus()
    return {key: sum(item['function'] == key and 'expected' in item for item in plan) for key in MESSAGES}


def expected_errors():
    return sum('error' in item for item in corpus())


def expected_witnesses():
    return sum(item['action'] != 'remove' for item in corpus())


def broker_cases():
    values = []
    for key, number in zip(MESSAGES, (102, 103, 104), strict=True):
        module = family(number)
        for label, command, prefix, detail in module.broker_cases('25'):
            # Only the topic identity changes; payload and expected rejection
            # are inherited independent inputs, not repaired broker rules.
            command = command.replace('task' + str(number) + '-' + key, TOPICS[key])
            values.append((key + '-' + label, command, prefix, detail))
    return values