"""Adversarial controls independently corrupt subtype, source and transport evidence."""
import copy
import hashlib
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from py019_evidence import verify_files, verify_transport


def observations():
    rows = []
    for name in ['Zeta', 'Deep-Leaf', 'Alpha']:
        for optional in [False, True]:
            body = {'Required': {'$type': name}}
            if optional:
                body['Optional'] = {'$type': 'Zeta'}
            rows.append(dict(outcome='typed delivery', subtype=name, optional=optional,
                raw_frame='MSG independent ' + json.dumps({'AbstractNotice': body})))
    rows.extend(dict(outcome='broker rejection', code='Invalid-Message', decoder_delta=0) for _ in range(6))
    rows.append(dict(outcome='runtime envelope rejection', runtime_event='ProtocolError', decoder_delta=0))
    candidate = '{"Concrete":{"Inherited":4,"Text":"c","$type":"Zeta"}}'
    rows.append(dict(outcome='generated decoder rejection',
        classification='schema-valid unsupported concrete-position discriminator', broker_ack='+OK',
        decoder_delta=1, category='CodecError', path='$.Concrete', runtime_event='CodecFailure',
        subscription_id='sub-1', input=candidate, decoder_input=candidate,
        raw_frame='MSG independent {"AbstractNotice":' + candidate + '}'))
    rows.append(dict(outcome='unsubscribe and callback silence', decoder_delta=0, subscriptions=0))
    return rows


class Evidence(unittest.TestCase):
    def test_transport_controls_reject_forged_attribution_and_subtypes(self):
        valid = observations()
        self.assertEqual(verify_transport(valid)['decoder_rejections'], 1)
        for mutation in range(11):
            bad = copy.deepcopy(valid)
            decoder = bad[-2]
            if mutation == 0:
                bad.pop(0)
            elif mutation == 1:
                bad[0]['subtype'] = 'Private'
            elif mutation == 2:
                bad[0]['raw_frame'] = bad[0]['raw_frame'].replace('Zeta', 'Private')
            elif mutation == 3:
                decoder['broker_ack'] = '-ERR'
            elif mutation == 4:
                decoder['decoder_delta'] = 0
            elif mutation == 5:
                decoder['runtime_event'] = 'ProtocolError'
            elif mutation == 6:
                decoder['path'] = '$.Required'
            elif mutation == 7:
                decoder['decoder_input'] = '{}'
            elif mutation == 8:
                decoder['classification'] = 'malformed abstract data'
            elif mutation == 9:
                bad[-1]['subscriptions'] = 1
            else:
                bad[6]['decoder_delta'] = 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                verify_transport(bad)

    def test_source_identities_and_exact_manifest_are_required(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source.json'
            source.write_text('{"head":"source-a","tree":"tree-a"}')
            manifest = {'source.json': hashlib.sha256(source.read_bytes()).hexdigest()}
            verify_files(root, manifest)
            source.write_text('{"head":"forged","tree":"tree-a"}')
            with self.assertRaises(ValueError):
                verify_files(root, manifest)
            manifest['source.json'] = hashlib.sha256(source.read_bytes()).hexdigest()
            (root / 'extra.json').write_text('{}')
            with self.assertRaises(ValueError):
                verify_files(root, manifest)
            for invalid in [{}, {'../outside': '0' * 64}, {'source.json': '0' * 64}]:
                with self.assertRaises(ValueError):
                    verify_files(root, invalid)


if __name__ == '__main__':
    unittest.main()