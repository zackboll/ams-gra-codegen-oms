"""Authentic RDMA expectations reuse independent Task 092 goldens, not generated code."""
import copy
import importlib
import json
from pathlib import Path
import sys
import unittest

dest = Path(sys.argv[1])
style, release = sys.argv[2:4]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if (dest / 'service_codec.py').exists() else None
api = importlib.import_module(prefix + 'service_api')
golden = json.loads(Path(sys.argv[4]).read_text())
UUID = '12345678-1234-1234-8234-123456789abc'


def typed(endpoints=None):
    """Construct independently of ingress codecs or generated introspection."""
    if endpoints is None:
        endpoints = (
            m.NetworkEndpointType.IPv4_Endpoint(m.IPv4_EndpointType(
                ipv4_address=m.IPv4_AddressType('192.0.2.10'))),
            m.NetworkEndpointType.IPv6_Endpoint(m.IPv6_EndpointType(
                ipv6_address=m.IPv6_AddressType('2001:db8::1'))))
    return m.RDMA_InitializeSetupMT(
        securityinformation=m.SecurityInformationType(
            classification=m.ClassificationEnum.U,
            ownerproducer=(m.OwnerProducerChoiceType.GovernmentIdentifier(m.OwnerProducerEnum.USA),
                           m.OwnerProducerChoiceType.NATO_SpecialWord(m.NATO_SpecialWordsType('NATO:EXAMPLE')))),
        messageheader=m.HeaderType(
            systemid=m.SystemID_Type(uuid=m.UniversallyUniqueIdentifierType(UUID)),
            timestamp=m.DateTimeType('2026-01-02T03:04:05Z'),
            schemaversion=m.UCI_SchemaVersionStringType('002.' + release[-1] + '.0'),
            mode=m.MessageModeEnum.LIVE),
        messagedata=m.RDMA_InitializeSetupMDT(
            rdma_initializesetupid=m.RDMA_InitializeSetupID_Type(uuid=m.UniversallyUniqueIdentifierType(UUID)),
            endpoint=endpoints, rdma_provider=m.RDMA_ProviderEnum.TCP))


class AuthenticRdma(unittest.TestCase):
    def test_typed_construction_and_message_identity(self):
        value = typed()
        self.assertIs(type(value), m.RDMA_InitializeSetupMT)
        self.assertEqual(value.messagedata.endpoint[0].alternative, 'IPv4_Endpoint')
        self.assertEqual(value.messagedata.endpoint[1].value.ipv6_address.value, '2001:db8::1')
        # Global identity is distinct from the payload type identity.
        self.assertIn('RDMA_InitializeSetup', (dest / 'service_api.py').read_text())
        self.assertEqual(m.RDMA_InitializeSetupMT.__schema_name__[1], 'RDMA_InitializeSetupMT')

    def test_golden_decode_inspect_encode_and_redecode(self):
        if c is None:
            return
        value = typed()
        self.assertEqual(json.loads(c.encode_RDMA_InitializeSetupMT(value)), golden)
        self.assertEqual(c.decode_RDMA_InitializeSetupMT(json.dumps(golden)), value)
        v4, v6 = value.messagedata.endpoint
        for endpoints in [(v4,), (v6,), (v4, v6), (v6, v4), (v4, v6, v4, v6), ()]:
            original = typed(endpoints)
            wire = c.encode_RDMA_InitializeSetupMT(original)
            decoded = c.decode_RDMA_InitializeSetupMT(wire)
            self.assertEqual(decoded, original)
            self.assertEqual(decoded.messagedata.endpoint, endpoints)
            self.assertEqual(c.decode_RDMA_InitializeSetupMT(c.encode_RDMA_InitializeSetupMT(decoded)), original)
        unusual = m.NetworkEndpointType.IPv6_Endpoint(m.IPv6_EndpointType(
            ipv6_address=m.IPv6_AddressType(':001.009.099.199')))
        original = typed((v4, unusual))
        self.assertEqual(c.decode_RDMA_InitializeSetupMT(c.encode_RDMA_InitializeSetupMT(original)), original)

    def test_invalid_addresses_choices_and_required_owners(self):
        if c is None:
            return
        for address, member in [('01.2.3.4', 'IPv4'), ('256.0.0.1', 'IPv4'),
                                ('::ffff:256.0.2.128', 'IPv6'), ('[::1]', 'IPv6')]:
            doc = copy.deepcopy(golden)
            index = 0 if member == 'IPv4' else 1
            doc['MessageData']['Endpoint'][index][member + '_Endpoint'][member + '_Address'] = address
            with self.assertRaises(c.CodecError) as caught:
                c.decode_RDMA_InitializeSetupMT(json.dumps(doc))
            self.assertIn(f'$.MessageData.Endpoint[{index}].{member}_Endpoint.{member}_Address', str(caught.exception))
        for invalid in [[{}], [[], {}], [{'IPv4_Endpoint': {}, 'IPv6_Endpoint': {}}], {}, [None]]:
            doc = copy.deepcopy(golden)
            doc['MessageData']['Endpoint'] = invalid
            with self.assertRaises(c.CodecError):
                c.decode_RDMA_InitializeSetupMT(json.dumps(doc))
        doc = copy.deepcopy(golden)
        doc['SecurityInformation']['OwnerProducer'] = []
        with self.assertRaises(c.CodecError) as caught:
            c.decode_RDMA_InitializeSetupMT(json.dumps(doc))
        self.assertIn('$.SecurityInformation.OwnerProducer', str(caught.exception))


if __name__ == '__main__':
    unittest.main(argv=[sys.argv[0]])