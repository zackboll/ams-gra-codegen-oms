"""Independent UCI 2.6 risk documents; values/cardinalities come from pinned XSD.

RiskSettingType is a record, NOT a Choice. The selected repeated Choice is
SecurityInformation.OwnerProducer. Do not fabricate risk discriminants.
"""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MESSAGE = 'SurvivabilityRiskLevel'
TOPIC = 'task105-survivability'


def golden(release):
    if release != '26':
        raise ValueError('Task105 transport is qualified only for UCI 2.6')
    return json.loads((ROOT / 'tests/fixtures/task105/survivability26.json').read_bytes())


def publication(document, name=MESSAGE, topic=TOPIC):
    return f'PUB {topic} ' + json.dumps({name: document})


def positives(release):
    base = golden(release)
    values = [('full', base, copy.deepcopy(base))]
    # RiskSetting 1..4, RiskLevel 1..5; NONE and ALL are never combined
    # with other AllowedRisk values, consistent with the XSD annotation.
    settings = [
        ('medium-all', [{'RiskPrioritization': 'MEDIUM', 'RiskLevel': [{'AllowedRisk': 'ALL'}]}]),
        ('low-track', [{'RiskPrioritization': 'LOW', 'RiskLevel': [{'AllowedRisk': 'TRACK'}]}]),
        ('ordered', [
            {'RiskPrioritization': 'NONE', 'RiskLevel': [{'AllowedRisk': 'NONE'}]},
            {'RiskPrioritization': 'HIGH', 'RiskLevel': [{'AllowedRisk': 'ALL'}]},
        ]),
        ('maximum', [
            {'RiskPrioritization': priority, 'RiskLevel': [{'AllowedRisk': risk} for risk in
             ('ACQUISITION', 'TRACK', 'LAUNCH', 'INTERCEPT', 'KILL')]}
            for priority in ('HIGH', 'MEDIUM', 'LOW', 'NONE')
        ]),
        ('optional-empty', [{'RiskPrioritization': 'HIGH', 'RiskLevel': [{'AllowedRisk': 'NONE'}]}]),
    ]
    for label, risk in settings:
        doc = copy.deepcopy(base)
        doc['MessageData']['RiskSetting'] = risk
        if label == 'maximum':
            doc['SecurityInformation'].update(
                DeclassDate='2026-01-02T03:04:05Z',
                CUI_DecontrolDate='2026-01-03T03:04:05Z',
                ClassifiedBy='  Task105  authority  ',
                SAR_Identifier=['Task105 marking'])
            doc['SecurityInformation']['OwnerProducer'] += [
                {'GovernmentIdentifier': 'CAN'}, {'NATO_SpecialWord': 'NATO:MORE'}]
            risk[0]['RiskLevel'][0]['MaximumContinuousExposure'] = {
                'CompositeExposureTime': 'PT1S', 'CumulativeExposureTime': 'PT2S'}
        if label == 'optional-empty':
            doc['MessageData']['AppliesToSystemID'] = []
        expected = copy.deepcopy(doc)
        if label == 'optional-empty':
            del expected['MessageData']['AppliesToSystemID']
        values.append((label, doc, expected))
    return values


def invalid_documents(release):
    values = []
    for label in BROKER_DETAILS:
        doc = copy.deepcopy(golden(release))
        data = doc['MessageData']
        risk = data['RiskSetting'][0]
        if label == 'uuid-variant':
            doc['MessageHeader']['SystemID']['UUID'] = '12345678-1234-1234-1234-123456789abc'
        elif label == 'short-version':
            doc['MessageHeader']['SchemaVersion'] = '002.6'
        elif label == 'priority-invalid':
            risk['RiskPrioritization'] = 'INVALID'
        elif label == 'allowed-invalid':
            risk['RiskLevel'][0]['AllowedRisk'] = 'INVALID'
        elif label == 'risk-member':
            risk['RiskLevel'][0]['NotARiskAlternative'] = {}
        elif label == 'risk-kind':
            risk['RiskLevel'] = 7
        elif label == 'owner-two-arms':
            doc['SecurityInformation']['OwnerProducer'][0]['NATO_SpecialWord'] = 'NATO:EXAMPLE'
        elif label == 'missing-priority':
            del risk['RiskPrioritization']
        elif label == 'missing-allowed':
            del risk['RiskLevel'][0]['AllowedRisk']
        elif label == 'empty-risk-setting':
            data['RiskSetting'] = []
        elif label == 'empty-risk-level':
            risk['RiskLevel'] = []
        elif label == 'too-many-settings':
            data['RiskSetting'] *= 5
        elif label == 'too-many-levels':
            risk['RiskLevel'] *= 6
        elif label == 'empty-classified-by':
            doc['SecurityInformation']['ClassifiedBy'] = ''
        elif label == 'empty-sar':
            doc['SecurityInformation']['SAR_Identifier'] = ['']
        elif label == 'declass-offset':
            doc['SecurityInformation']['DeclassDate'] = '2026-01-02T03:04:05+01:00'
        elif label == 'decontrol-offset':
            doc['SecurityInformation']['CUI_DecontrolDate'] = '2026-01-02T03:04:05+01:00'
        elif label == 'settings-kind':
            data['RiskSetting'] = risk
        elif label == 'owners-kind':
            doc['SecurityInformation']['OwnerProducer'] = {'GovernmentIdentifier': 'USA'}
        elif label == 'owner-unknown':
            doc['SecurityInformation']['OwnerProducer'][0] = {'NotAnOwner': 'USA'}
        elif label == 'missing-risk-id':
            del data['SurvivabilityRiskLevelID']
        elif label == 'setting-member':
            risk['NotASetting'] = 'NONE'
        values.append((label, doc))
    return values


def decoder_invalid(release):
    # Candidate only until the actual pinned broker campaign observes +OK/MSG.
    doc = golden(release)
    doc['MessageHeader']['Timestamp'] = 'not-a-dateZ'
    return [('invalid-timestamp', doc)]


BROKER_DETAILS = {
    'uuid-variant': ('FacetViolation', 'MessageHeader.SystemID.UUID'),
    'short-version': ('FacetViolation', 'MessageHeader.SchemaVersion'),
    'priority-invalid': ('FacetViolation', 'RiskPrioritization'),
    'allowed-invalid': ('FacetViolation', 'AllowedRisk'),
    'risk-member': ('UnknownMember', 'NotARiskAlternative'),
    'risk-kind': ('OccurrenceMismatch', 'expected_array: true'),
    'owner-two-arms': ('ChoiceViolation', 'present: 2'),
    'missing-priority': ('MissingRequiredMember', 'RiskPrioritization'),
    'missing-allowed': ('MissingRequiredMember', 'AllowedRisk'),
    'empty-risk-setting': ('OccurrenceMismatch', 'RiskSetting'),
    'empty-risk-level': ('OccurrenceMismatch', 'RiskLevel'),
    'too-many-settings': ('OccurrenceMismatch', 'RiskSetting'),
    'too-many-levels': ('OccurrenceMismatch', 'RiskLevel'),
    'empty-classified-by': ('FacetViolation', 'ClassifiedBy'),
    'empty-sar': ('FacetViolation', 'SAR_Identifier'),
    'declass-offset': ('FacetViolation', 'DeclassDate'),
    'decontrol-offset': ('FacetViolation', 'CUI_DecontrolDate'),
    'settings-kind': ('OccurrenceMismatch', 'RiskSetting'),
    'owners-kind': ('OccurrenceMismatch', 'OwnerProducer'),
    'owner-unknown': ('UnknownMember', 'NotAnOwner'),
    'missing-risk-id': ('MissingRequiredMember', 'SurvivabilityRiskLevelID'),
    'setting-member': ('UnknownMember', 'NotASetting'),
}

CODEC_REASONS = {
    'uuid-variant': 'invalid checked String', 'short-version': 'invalid checked String',
    'priority-invalid': 'RiskSetting item 1', 'allowed-invalid': 'RiskSetting item 1',
    'risk-member': 'RiskSetting item 1', 'risk-kind': 'RiskSetting item 1',
    'owner-two-arms': 'OwnerProducer item 1', 'missing-priority': 'RiskSetting item 1',
    'missing-allowed': 'RiskSetting item 1', 'empty-risk-setting': 'RiskSetting cardinality',
    'empty-risk-level': 'RiskSetting item 1', 'too-many-settings': 'RiskSetting cardinality',
    'too-many-levels': 'RiskSetting item 1', 'invalid-timestamp': 'invalid named DateTime',
    'empty-classified-by': 'invalid checked String',
    'empty-sar': 'SAR_Identifier item 1',
    'declass-offset': 'invalid named DateTime',
    'decontrol-offset': 'invalid named DateTime',
    'settings-kind': 'wrong JSON kind',
    'owners-kind': 'wrong JSON kind',
    'owner-unknown': 'OwnerProducer item 1',
    'missing-risk-id': 'missing required member: SurvivabilityRiskLevelID',
    'setting-member': 'RiskSetting item 1',
}


def broker_cases(release):
    values = [(label, publication(doc), '-ERR Invalid-Message', BROKER_DETAILS[label][0])
              for label, doc in invalid_documents(release)]
    base = golden(release)
    return values + [
        ('wrong-global', publication(base, name=MESSAGE + 'MT'), '-ERR Invalid-Message', 'UnknownGlobalElement'),
        ('wrong-topic', publication(base, topic='forbidden'), '-ERR Illegal-State', 'not authorized'),
        ('unknown-sid', 'UNSUB 987', '-ERR Illegal-State', 'unknown subscription id 987'),
        ('malformed-document', f'PUB {TOPIC} {{', '-ERR Invalid-Message', 'invalid JSON payload'),
    ]