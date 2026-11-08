"""Independent lexical expectations and schema-specific integration probes."""
from dataclasses import FrozenInstanceError
import importlib
import json
from pathlib import Path
import sys
import unittest

dest = Path(sys.argv[1])
style = sys.argv[2]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if (dest / 'service_codec.py').exists() else None
qualified = m.Payload.__schema_name__[0] != 'https://www.vdl.afrl.af.mil/programs/oam'


def key(name):
    return '{' + m.Payload.__schema_name__[0] + '}' + name if qualified else name


class NetworkAddresses(unittest.TestCase):
    def endpoint(self, ipv6=False):
        if ipv6:
            return m.NetworkEndpointType.IPv6(m.IPv6_EndpointType(
                ipv6_address=m.IPv6_AddressType(':001.009.099.199')))
        return m.NetworkEndpointType.IPv4(m.IPv4_EndpointType(ipv4_address=m.Leaf('198.51.100.42')))

    def payload(self, **kwargs):
        return m.Payload(required=m.IPv4_AddressType('1.2.3.4'),
                         selected=self.endpoint(), positive=(m.IPv6_AddressType('::'),), **kwargs)

    def test_ipv4_independent_corpus(self):
        count = 0
        # Numeric loop bounds and zero-padding directly author expectations;
        # no generated validator, codec, regex or network parser is an oracle.
        for width in (1, 2, 3):
            for number in range(10**width):
                spelling = f'{number:0{width}d}'
                expected = number <= 255 and (width == 1 or number >= 10**(width - 1))
                for position in range(4):
                    parts = ['0', '9', '10', '255']
                    parts[position] = spelling
                    text = '.'.join(parts)
                    if expected:
                        self.assertEqual(m.IPv4_AddressType(text).value, text)
                    else:
                        with self.assertRaises(ValueError, msg=repr(text)):
                            m.IPv4_AddressType(text)
                    count += 1
        for text in ['1.2.3', '1.2.3.4.5', '1..3.4', '+1.2.3.4', '.1.2.3.4',
                     '1.2.3.4.', '١.2.3.4', '１.2.3.4', '1.2.3.4\x7f',
                     '', '1.2.3.4 ', ' 1.2.3.4', '1.2.3.4\n', '1.2.3.4\r',
                     '1.2.3.4\t', '1.2.3.4\0', '1.2.3.4\ud800']:
            with self.assertRaises(ValueError, msg=repr(text)):
                m.IPv4_AddressType(text)
            count += 1
        for text in ['0.0.0.0', '255.255.255.255']:
            self.assertEqual(m.IPv4_AddressType(text).value, text)
        print(f'PY017 independent IPv4 vectors: {count}', flush=True)

    def test_task062_full_independent_corpus(self):
        cases = json.loads((dest.parent / 'ipv6-cases.json').read_text())
        self.assertGreater(len(cases), 26000)
        for text, expected in cases:
            if expected:
                self.assertEqual(m.IPv6_AddressType(text).value, text)
            else:
                with self.assertRaises(ValueError, msg=repr(text)):
                    m.IPv6_AddressType(text)
        print(f'PY017 independent Task 062 vectors: {len(cases)}', flush=True)

    def test_models_and_storage(self):
        class Text(str):
            pass
        class Address(m.IPv4_AddressType):
            pass
        for carrier, text in [(m.IPv4_AddressType, '198.51.100.42'),
                              (m.IPv6_AddressType, 'fE80::aBcD')]:
            value = carrier(text)
            self.assertEqual(value.value, text)
            self.assertFalse(hasattr(value, '__dict__'))
            with self.assertRaises(FrozenInstanceError):
                value.value = text
            with self.assertRaises(TypeError):
                carrier()
            for invalid in [Text(text), None, 42, b'::', True, ()]:
                with self.assertRaises(TypeError):
                    carrier(invalid)
        with self.assertRaises(TypeError):
            m.Payload(required=Address('1.2.3.4'), selected=self.endpoint(),
                      positive=(m.IPv6_AddressType('::'),))
        with self.assertRaises(TypeError):
            m.IPv4_EndpointType(ipv4_address=m.IPv4_AddressType('1.2.3.4'))
        self.assertEqual(m.Leaf('1.2.3.4').__schema_name__[1], 'Leaf')
        for text in [':001.009.099.199', ':::::::::', '1::2::3', 'fE80::aBcD']:
            self.assertEqual(m.IPv6_AddressType(text).value, text)
        with self.assertRaises(ValueError):
            m.IPv4_AddressType('001.009.099.199')
        for corrupt in [None, 4, Text('1.2.3.4'), '256.0.0.1', '1.2.3.4 ']:
            forged = object.__new__(m.Leaf)
            object.__setattr__(forged, 'value', corrupt)
            with self.assertRaises((TypeError, ValueError)):
                m.IPv4_EndpointType(ipv4_address=forged)
        with self.assertRaises(AttributeError):
            m.IPv4_EndpointType(ipv4_address=object.__new__(m.Leaf))
        endpoint = self.endpoint()
        object.__setattr__(endpoint.value.ipv4_address, 'value', '999.1.2.3')
        with self.assertRaises(ValueError):
            self.payload(endpoint=(endpoint,))
        for kwargs in [dict(bounded=(m.IPv4_AddressType('1.2.3.4'),)*4),
                       dict(endpoint=[self.endpoint()])]:
            with self.assertRaises((TypeError, ValueError)):
                self.payload(**kwargs)
        with self.assertRaises(ValueError):
            m.Payload(required=m.IPv4_AddressType('1.2.3.4'), selected=self.endpoint(), positive=())

    def test_codec_roundtrips_and_paths(self):
        if c is None:
            return
        for endpoints in [(), (self.endpoint(),), (self.endpoint(True),),
                          (self.endpoint(), self.endpoint(True)),
                          (self.endpoint(True), self.endpoint()),
                          (self.endpoint(), self.endpoint(True), self.endpoint())]:
            original = self.payload(endpoint=endpoints, optional=m.IPv6_AddressType('FE80::ABCD'),
                                    inherited=(m.IPv6_AddressType('::1'),))
            wire = c.encode_Payload(original)
            decoded = c.decode_Payload(wire)
            self.assertEqual(original, decoded)
            self.assertEqual(wire, c.encode_Payload(decoded))
            obj = json.loads(wire)
            self.assertEqual(obj[key('Required')], '1.2.3.4')
            self.assertEqual(obj[key('Optional')], 'FE80::ABCD')
            self.assertEqual(obj.get(key('Endpoint'), []), [
                {key('IPv4'): {key('IPv4_Address'): '198.51.100.42'}}
                if e.alternative == 'IPv4' else
                {key('IPv6'): {key('IPv6_Address'): ':001.009.099.199'}} for e in endpoints])
        for position in range(3):
            obj = json.loads(c.encode_Payload(self.payload(endpoint=(self.endpoint(),)*3)))
            obj[key('Endpoint')][position][key('IPv4')][key('IPv4_Address')] = '01.2.3.4'
            path = key('Endpoint') + f'[{position}].' + key('IPv4') + '.' + key('IPv4_Address')
            with self.assertRaises(c.CodecError) as caught:
                c.decode_Payload(json.dumps(obj))
            self.assertIn(path, str(caught.exception))
            payload = self.payload(endpoint=(self.endpoint(), self.endpoint(), self.endpoint()))
            object.__setattr__(payload.endpoint[position].value.ipv4_address, 'value', '256.1.2.3')
            with self.assertRaises(c.CodecError) as caught:
                c.encode_Payload(payload)
            self.assertIn(path, str(caught.exception))
        for invalid in [None, 7, True, [], {}, '::\ud800', '::1 ', '[::1]', 'fe80::1%eth0']:
            obj = json.loads(c.encode_Payload(self.payload()))
            obj[key('Optional')] = invalid
            with self.assertRaises(c.CodecError):
                c.decode_Payload(json.dumps(obj))
        for forged in [object.__new__(m.IPv4_AddressType), m.IPv4_AddressType('1.2.3.4')]:
            if hasattr(forged, 'value'):
                object.__setattr__(forged, 'value', '999.1.2.3')
            payload = self.payload()
            object.__setattr__(payload, 'required', forged)
            with self.assertRaises(c.CodecError):
                c.encode_Payload(payload)
        for invalid in [[], [{key('IPv4'): {}}], {}, {key('IPv4'): {}, key('IPv6'): {}}]:
            obj = json.loads(c.encode_Payload(self.payload()))
            obj[key('Selected')] = invalid
            with self.assertRaises(c.CodecError):
                c.decode_Payload(json.dumps(obj))
        with self.assertRaises(c.CodecError):
            c.decode_Payload('{"Required":"1.2.3.4","Required":"1.2.3.4"}')


unittest.main(argv=[sys.argv[0]])