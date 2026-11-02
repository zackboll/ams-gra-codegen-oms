"""Literal authentic alternative spellings, not generated encoder oracles."""
import importlib
from pathlib import Path
import sys
import unittest

dest = Path(sys.argv[1])
sys.path.insert(0, str(dest if sys.argv[2] == 'standalone' else dest.parent))
prefix = '' if sys.argv[2] == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if (dest / 'service_codec.py').exists() else None


class OriginalChoices(unittest.TestCase):
    def test_alternatives(self):
        for name, enum_name, government, country in [
            ('OwnerProducerChoiceType', 'OwnerProducerEnum', 'GovernmentIdentifier', 'USA'),
            ('ReleasableToChoiceType', 'ReleasableToEnum', 'GovernmentIdentifier', 'USA'),
            ('FGI_SourceOpenChoiceType', 'FGI_SourceOpenEnum', 'ForeignGovernmentIdentifier', 'GBR'),
            ('FGI_SourceProtectedChoiceType', 'FGI_SourceProtectedEnum', 'ForeignGovernmentIdentifier', 'GBR')]:
            cls = getattr(m, name)
            enum = getattr(m, enum_name)
            a = getattr(cls, government)(getattr(enum, country))
            b = cls.NATO_SpecialWord(m.NATO_SpecialWordsType('NATO:EXAMPLE'))
            self.assertEqual(a.alternative, government)
            self.assertEqual(b.value.value, 'NATO:EXAMPLE')
            if c:
                encode = getattr(c, '_encode_' + name)
                decode = getattr(c, '_decode_' + name)
                golden = '{"' + government + '":"' + country + '"}'
                self.assertEqual(c._dumps(encode(a, '$')), golden)
                self.assertEqual(decode(c._loads(golden), '$'), a)
                self.assertEqual(c._dumps(encode(b, '$')), '{"NATO_SpecialWord":"NATO:EXAMPLE"}')
                for invalid in ['{}', '{"NATO_SpecialWord":"BAD"}',
                                '{"NATO_SpecialWord":"NATO:X","Unknown":"USA"}']:
                    with self.assertRaises(c.CodecError):
                        decode(c._loads(invalid), '$')

    def test_repeated_security(self):
        a = m.OwnerProducerChoiceType.GovernmentIdentifier(m.OwnerProducerEnum.USA)
        b = m.OwnerProducerChoiceType.NATO_SpecialWord(m.NATO_SpecialWordsType('NATO:EXAMPLE'))
        value = m.Payload(ownerproducer=(a, b, a),
            fgi_sourceopen=m.FGI_SourceOpenChoiceType.NATO_SpecialWord(m.NATO_SpecialWordsType('NATO:EXAMPLE')),
            fgi_sourceprotected=m.FGI_SourceProtectedChoiceType.ForeignGovernmentIdentifier(m.FGI_SourceProtectedEnum.GBR),
            releasableto=(m.ReleasableToChoiceType.GovernmentIdentifier(m.ReleasableToEnum.USA),),
            displayonlyto=(m.ReleasableToChoiceType.NATO_SpecialWord(m.NATO_SpecialWordsType('NATO:EXAMPLE')),))
        golden = ('{"OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"},'
                  '{"GovernmentIdentifier":"USA"}],"FGI_SourceOpen":{"NATO_SpecialWord":"NATO:EXAMPLE"},'
                  '"FGI_SourceProtected":{"ForeignGovernmentIdentifier":"GBR"},'
                  '"ReleasableTo":[{"GovernmentIdentifier":"USA"}],'
                  '"DisplayOnlyTo":[{"NATO_SpecialWord":"NATO:EXAMPLE"}]}')
        if c:
            self.assertEqual(c.encode_Payload(value), golden)
            self.assertEqual(c.decode_Payload(golden), value)
            for owners, text in [((a, b), '[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}]'),
                                 ((b, a), '[{"NATO_SpecialWord":"NATO:EXAMPLE"},{"GovernmentIdentifier":"USA"}]'),
                                 ((b, b), '[{"NATO_SpecialWord":"NATO:EXAMPLE"},{"NATO_SpecialWord":"NATO:EXAMPLE"}]')]:
                self.assertEqual(c.encode_Payload(m.Payload(ownerproducer=owners)), '{"OwnerProducer":' + text + '}')


unittest.main(argv=[sys.argv[0]], verbosity=2)