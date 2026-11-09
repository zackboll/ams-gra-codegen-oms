"""Independent JSON and typed full-service qualification, unchanged pinned UCI."""
import copy
import importlib
import json
from pathlib import Path
import sys
import unittest

destination = Path(sys.argv[1])
style, release, policy = sys.argv[2:]
sys.path.insert(0, str(destination if style == 'standalone' else destination.parent))
prefix = '' if style == 'standalone' else destination.name + '.'
m = importlib.import_module(prefix + 'model')
api = importlib.import_module(prefix + 'service_api')
UUID = '123e4567-e89b-42d3-a456-426614174000'
NS = 'https://www.vdl.afrl.af.mil/programs/oam'
VERSION = '002.' + release[-1] + '.0'


def value(present):
    uuid = m.UniversallyUniqueIdentifierType(UUID)
    selected = m.ManagedListBaseType.from_ForeignKeyMapML(m.ForeignKeyMapML(
        foreignkeypairlistelement=(m.ForeignKeyPairDRLE(
            foreignkeypairid=m.ForeignKeyPairID_Type(uuid=uuid)),)))
    return m.DataRecordListManagementRequestMT(
        securityinformation=m.SecurityInformationType(classification=m.ClassificationEnum.U,
            ownerproducer=(m.OwnerProducerChoiceType.GovernmentIdentifier(m.OwnerProducerEnum.USA),)),
        messageheader=m.HeaderType(systemid=m.SystemID_Type(uuid=uuid),
            timestamp=m.DateTimeType('2026-01-02T03:04:05Z'),
            schemaversion=m.UCI_SchemaVersionStringType(VERSION), mode=m.MessageModeEnum.LIVE),
        messagedata=m.DataRecordListManagementRequestMDT(requestid=m.RequestID_Type(uuid=uuid),
            requeststate=m.RequestStateEnum.NEW, datarecordlistid=m.DataRecordListID_Type(uuid=uuid),
            addrecord=selected if present else None,
            deleterecord=m.ManagedListBaseType.from_ForeignKeyMapML(m.ForeignKeyMapML()) if present else None))


def expected(present):
    body = {'SecurityInformation': {'Classification': 'U', 'OwnerProducer': [{'GovernmentIdentifier': 'USA'}]},
            'MessageHeader': {'SystemID': {'UUID': UUID}, 'Timestamp': '2026-01-02T03:04:05Z',
                              'SchemaVersion': VERSION, 'Mode': 'LIVE'},
            'MessageData': {'RequestID': {'UUID': UUID}, 'RequestState': 'NEW',
                            'DataRecordListID': {'UUID': UUID}}}
    if present:
        body['MessageData']['AddRecord'] = {'ForeignKeyPairListElement': [{'ForeignKeyPairID': {'UUID': UUID}}],
                                            '$type': 'ForeignKeyMapML'}
        body['MessageData']['DeleteRecord'] = {'$type': 'ForeignKeyMapML'}
    return body


class AuthenticService(unittest.TestCase):
    def test_model_source_identities_and_both_optional_states(self):
        self.assertEqual(m.ManagedListBaseType.__schema_name__, (NS, 'ManagedListBaseType'))
        self.assertEqual(m.ForeignKeyMapML.__schema_name__, (NS, 'ForeignKeyMapML'))
        for present in [False, True]:
            request = value(present)
            self.assertIs(type(request), m.DataRecordListManagementRequestMT)
            self.assertIs(api.function_selected.exchange_output.Payload, type(request))
            if present:
                selected = request.messagedata.addrecord
                self.assertEqual(selected.alternative, (NS, 'ForeignKeyMapML'))
                self.assertIs(type(selected.value), m.ForeignKeyMapML)
                self.assertIs(type(selected.value.foreignkeypairlistelement[0]), m.ForeignKeyPairDRLE)
            else:
                self.assertIsNone(request.messagedata.addrecord)
                self.assertIsNone(request.messagedata.deleterecord)

    def test_actual_typed_service_api_publish(self):
        request = value(True)
        class Adapter:
            def publish(self, **arguments):
                self.arguments = arguments
                return 'published'
        adapter = Adapter()
        self.assertEqual(api.function_selected.exchange_output.publish(adapter, request), 'published')
        self.assertEqual(adapter.arguments, dict(message_namespace=NS,
            message_name='DataRecordListManagementRequest', topic='py006.selected', payload=request))

    def test_exact_payload_and_inherited_value_revalidation(self):
        with self.assertRaises(TypeError):
            m.ManagedListBaseType.from_ForeignKeyMapML({})
        with self.assertRaises(ValueError):
            m.ManagedListBaseType((NS, 'ManagedListBaseType'), m.ForeignKeyMapML())
        request = value(True)
        object.__setattr__(request.messagedata.addrecord.value.foreignkeypairlistelement[0], 'foreignkeypairid', None)
        with self.assertRaises(TypeError):
            m.ManagedListBaseType.from_ForeignKeyMapML(request.messagedata.addrecord.value)


if policy == 'codec':
    c = importlib.import_module(prefix + 'service_codec')

    class AuthenticCodec(unittest.TestCase):
        def test_independent_wire_and_parity(self):
            for present in [False, True]:
                request = value(present)
                text = c.encode_DataRecordListManagementRequestMT(request)
                self.assertEqual(json.loads(text), expected(present))
                decoded = c.decode_DataRecordListManagementRequestMT(json.dumps(expected(present)))
                self.assertEqual(decoded, request)
                self.assertEqual(c.encode_DataRecordListManagementRequestMT(decoded), text)

        def test_subtype_required_unknown_null_and_wrong_content(self):
            for invalid in [None, {}, {'$type': 'ManagedListBaseType'}, {'$type': 'Private'},
                            {'$type': 1}, {'$type': 'ForeignKeyMapML', 'Secret': 'private'},
                            {'$type': 'ForeignKeyMapML', 'ForeignKeyPairListElement': [{}]}]:
                body = expected(True)
                body['MessageData']['AddRecord'] = invalid
                with self.subTest(invalid=invalid), self.assertRaises(c.CodecError) as caught:
                    c.decode_DataRecordListManagementRequestMT(json.dumps(body))
                self.assertIn('$.MessageData.AddRecord', str(caught.exception))

        def test_duplicate_and_nested_indexed_paths(self):
            body = expected(True)
            body['MessageData']['AddRecord']['ForeignKeyPairListElement'][0]['ForeignKeyPairID']['UUID'] = 'not-uuid'
            with self.assertRaises(c.CodecError) as caught:
                c.decode_DataRecordListManagementRequestMT(json.dumps(body))
            self.assertIn('$.MessageData.AddRecord.ForeignKeyPairListElement[0].ForeignKeyPairID.UUID',
                          str(caught.exception))
            duplicate = json.dumps(expected(True)).replace('"$type": "ForeignKeyMapML"',
                '"$type":"ForeignKeyMapML","$type":"ForeignKeyMapML"', 1)
            with self.assertRaises(c.CodecError) as caught:
                c.decode_DataRecordListManagementRequestMT(duplicate)
            self.assertIn('$.MessageData.AddRecord', str(caught.exception))


unittest.main(argv=['py019-real-uci'], verbosity=2)