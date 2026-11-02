"""Execute generated full-schema models/API/codecs against independent goldens."""
import importlib
import json
from pathlib import Path
import sys
import unittest

destination, style, release, message, golden = sys.argv[1:]
destination = Path(destination)
sys.path.insert(0, str(destination if style == 'standalone' else destination.parent))
prefix = '' if style == 'standalone' else destination.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec')
api = importlib.import_module(prefix + 'service_api')
document = json.loads(Path(golden).read_text())
encode = getattr(c, 'encode_' + message + 'MT')
decode = getattr(c, 'decode_' + message + 'MT')


class CompleteAuthenticService(unittest.TestCase):
    def test_golden_model_api_codec(self):
        value = decode(json.dumps(document))
        self.assertIs(type(value), getattr(m, message + 'MT'))
        self.assertEqual(api.function_selected.exchange_output.MESSAGE_NAME, message)
        self.assertIs(api.function_selected.exchange_output.Payload, type(value))
        self.assertEqual(value.securityinformation.classification, m.ClassificationEnum.U)
        owners = value.securityinformation.ownerproducer
        self.assertIs(type(owners), tuple)
        self.assertEqual([o.alternative for o in owners], ['GovernmentIdentifier', 'NATO_SpecialWord'])
        self.assertEqual(owners[0].value, m.OwnerProducerEnum.USA)
        self.assertEqual(owners[1].value.value, 'NATO:EXAMPLE')
        self.assertEqual(value.messageheader.schemaversion.value, '002.' + release[-1] + '.0')
        text = document['SecurityInformation']['ClassifiedBy']
        self.assertIs(type(value.securityinformation.classifiedby), m.WhitespaceVisibleString1024Type)
        self.assertEqual(value.securityinformation.classifiedby.value, text)
        wire = encode(value)
        self.assertEqual(json.loads(wire), document)
        self.assertEqual(decode(wire), value)
        self.assertEqual(encode(decode(wire)), wire)
        altered = json.loads(wire)
        altered['SecurityInformation']['ClassifiedBy'] = ' a\t b ' if release == '25' else ' a\n\r  b '
        stored = decode(json.dumps(altered)).securityinformation.classifiedby.value
        self.assertEqual(stored, 'a b' if release == '25' else ' a\n\r  b ')
        if release == '25':
            self.assertEqual(m.WhitespaceVisibleString1024Type('').value, '')
        else:
            with self.assertRaises(ValueError):
                m.WhitespaceVisibleString1024Type('')
        for invalid in ['\x00', '\x0b', '\x0c', '\x7f', '\u00a0', '\ud800'] + (['', '\t'] if release == '26' else []):
            altered['SecurityInformation']['ClassifiedBy'] = invalid
            with self.assertRaises(c.CodecError) as caught:
                decode(json.dumps(altered))
            self.assertIn('$.SecurityInformation.ClassifiedBy:', str(caught.exception))
        forged = decode(wire)
        object.__setattr__(forged.securityinformation.classifiedby, 'value', '\x00')
        with self.assertRaises(c.CodecError):
            encode(forged)
        for section, field, invalid in [('SecurityInformation', 'OwnerProducer', []),
                                        ('MessageHeader', 'SchemaVersion', 'bad'),
                                        ('MessageHeader', 'Timestamp', 'bad')]:
            altered = json.loads(wire)
            altered[section][field] = invalid
            with self.assertRaises(c.CodecError):
                decode(json.dumps(altered))


unittest.main(argv=[sys.argv[0]])