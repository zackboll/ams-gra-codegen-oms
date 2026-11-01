"""Independent authentic RDMA inputs and expectations, never a codec oracle."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MESSAGE = 'RDMA_InitializeSetup'
TOPIC = 'task103-rdma'


def golden(release):
    return json.loads((ROOT / f'tests/fixtures/service-generate/task092-real-rdma{release}.json').read_bytes())


def publication(document, name=MESSAGE, topic=TOPIC):
    return f'PUB {topic} ' + json.dumps({name: document})


def positives(release):
    base = golden(release)
    values = [('full', base, copy.deepcopy(base))]
    v4 = {'IPv4_Endpoint': {'IPv4_Address': '198.51.100.42'}}
    v6 = {'IPv6_Endpoint': {'IPv6_Address': '2001:db8::42'}}
    # Both authentic XSDs declare Endpoint minOccurs=0, maxOccurs=unbounded.
    for label, endpoints in [('ipv4-only', [v4]), ('ipv6-only', [v6]),
                             ('reversed', [v6, v4]),
                             ('additional', [v4, v6, v4, v6]), ('empty', [])]:
        doc = copy.deepcopy(base)
        doc['MessageData']['Endpoint'] = copy.deepcopy(endpoints)
        if label == 'additional':
            doc['SecurityInformation']['OwnerProducer'] += [
                {'GovernmentIdentifier': 'CAN'}, {'NATO_SpecialWord': 'NATO:MORE'}]
        expected = copy.deepcopy(doc)
        if not endpoints:
            del expected['MessageData']['Endpoint']  # existing canonical omission
        values.append((label, doc, expected))
    return values


def invalid_documents(release):
    values = []
    for label in ['uuid-variant', 'short-version', 'ipv4-invalid', 'ipv6-invalid',
                  'endpoint-two-arms', 'endpoint-member', 'endpoint-kind', 'missing-required']:
        doc = copy.deepcopy(golden(release))
        data = doc['MessageData']
        if label == 'uuid-variant':
            doc['MessageHeader']['SystemID']['UUID'] = '12345678-1234-1234-1234-123456789abc'
        elif label == 'short-version':
            doc['MessageHeader']['SchemaVersion'] = '002.5'
        elif label == 'ipv4-invalid':
            data['Endpoint'][0]['IPv4_Endpoint']['IPv4_Address'] = '999.0.0.1'
        elif label == 'ipv6-invalid':
            data['Endpoint'][1]['IPv6_Endpoint']['IPv6_Address'] = 'not-an-ipv6'
        elif label == 'endpoint-two-arms':
            data['Endpoint'][0].update(data['Endpoint'][1])
        elif label == 'endpoint-member':
            data['Endpoint'][1] = {'NotAnEndpoint': {}}
        elif label == 'endpoint-kind':
            data['Endpoint'] = 7
        else:
            del data['RDMA_InitializeSetupID']
        values.append((label, doc))
    return values


def protocol_cases(release):
    base = golden(release)
    return [
        ('wrong-global', publication(base, name='RDMA_InitializeSetupMT'),
         '-ERR Invalid-Message', 'UnknownGlobalElement'),
        ('wrong-topic', publication(base, topic='forbidden'),
         '-ERR Illegal-State', 'not authorized'),
        ('unknown-sid', 'UNSUB 987', '-ERR Illegal-State', 'unknown subscription id 987'),
        ('malformed-document', f'PUB {TOPIC} {{', '-ERR Invalid-Message', 'invalid JSON payload'),
    ]


def decoder_invalid(release):
    # Observed +OK through pinned Sleet for both releases. Sleet does not enforce
    # the complete inherited dateTime lexical space; generated Ada does.
    doc = golden(release)
    doc['MessageHeader']['Timestamp'] = 'not-a-dateZ'
    return [('invalid-timestamp', doc)]


BROKER_DETAILS = {
    'uuid-variant': ('FacetViolation', 'MessageHeader.SystemID.UUID'),
    'short-version': ('FacetViolation', 'MessageHeader.SchemaVersion'),
    'ipv4-invalid': ('FacetViolation', 'IPv4_Endpoint.IPv4_Address'),
    'ipv6-invalid': ('FacetViolation', 'IPv6_Endpoint.IPv6_Address'),
    'endpoint-two-arms': ('ChoiceViolation', 'present: 2'),
    'endpoint-member': ('UnknownMember', 'NotAnEndpoint'),
    'endpoint-kind': ('OccurrenceMismatch', 'expected_array: true'),
    'missing-required': ('MissingRequiredMember', 'RDMA_InitializeSetupID'),
}

CODEC_REASONS = {
    'uuid-variant': 'invalid checked String', 'short-version': 'invalid checked String',
    'ipv4-invalid': 'Endpoint item 1', 'ipv6-invalid': 'Endpoint item 2',
    'endpoint-two-arms': 'Endpoint item 1',
    'endpoint-member': 'Endpoint item 2', 'endpoint-kind': 'wrong JSON kind',
    'missing-required': 'missing required member: RDMA_InitializeSetupID',
    'invalid-timestamp': 'invalid named DateTime',
}


def broker_cases(release):
    values = [(label, publication(doc), '-ERR Invalid-Message', BROKER_DETAILS[label][0])
              for label, doc in invalid_documents(release)]
    return values + protocol_cases(release)