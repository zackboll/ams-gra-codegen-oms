"""Execute the authenticated repeated member in isolation, NOT its full service."""
import copy
import hashlib
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET


class UnboundedRepetitionUci(unittest.TestCase):
    def test_pinned_subsystem_string_member(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py012_pinned_inputs', root / 'scripts/task101_evidence.py')
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        xs = '{http://www.w3.org/2001/XMLSchema}'
        namespace = 'https://www.vdl.afrl.af.mil/programs/oam'
        for release in ('25', '26'):
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release[-1] + '_ROOT'])
            include = schema.parent / f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
            self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), pins.PINS[release])
            self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), pins.INCLUDES[release])
            source = ET.parse(schema).getroot()
            owner = next(e for e in source.findall(xs + 'complexType') if e.get('name') == 'SubsystemStreamMDT')
            field = next(e for e in owner.findall('.//' + xs + 'element') if e.get('name') == 'SubsystemStreamString')
            self.assertEqual((field.get('minOccurs'), field.get('maxOccurs')), ('0', 'unbounded'))
            name = field.get('type').split(':')[-1]
            self.assertEqual(name, 'VisibleString1024Type')
            declaration = next(e for e in source.findall(xs + 'simpleType') if e.get('name') == name)
            with tempfile.TemporaryDirectory(prefix='py012-pinned-member-') as temporary:
                path = Path(temporary)
                fragment = ET.Element(xs + 'schema', {'targetNamespace': namespace, 'xmlns:uci': namespace,
                                                     'elementFormDefault': 'qualified'})
                fragment.append(copy.deepcopy(declaration))
                record = ET.SubElement(fragment, xs + 'complexType', {'name': 'Payload'})
                ET.SubElement(record, xs + 'sequence').append(copy.deepcopy(field))
                ET.SubElement(fragment, xs + 'element', {'name': 'SequenceNotice', 'type': 'uci:Payload',
                                                       '{' + namespace + '}version': '001.000.000.000'})
                extracted = path / 'member.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                for world in ('closed-schema', 'open-extensions'):
                    destination = path / world.replace('-', '_')
                    result = subprocess.run([str(generator), 'service-generate', '--schema', str(extracted),
                        '--contract', str(root / 'tests/fixtures/python-unbounded-repetition/service.yaml'),
                        '--language', 'python', '--world', world, '--with-codec', '--output', str(destination)],
                        capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    probe = '''import model as m, service_codec as c, unittest
class PinnedMember(unittest.TestCase):
 def test_literal_wire_and_order(self):
  v=m.Payload(subsystem_stream_string=(m.VisibleString1024Type('first'),m.VisibleString1024Type(' last '),m.VisibleString1024Type('first')))
  text='{"SubsystemStreamString":["first"," last ","first"]}'
  self.assertEqual(c.encode_Payload(v),text)
  self.assertEqual(c.decode_Payload(text),v)
  self.assertEqual(c.decode_Payload('{}'),m.Payload())
  self.assertEqual(c.decode_Payload('{"SubsystemStreamString":[]}'),m.Payload())
  self.assertEqual(c.encode_Payload(m.Payload()),'{}')
  for text in ['{"SubsystemStreamString":null}','{"SubsystemStreamString":"scalar"}','{"SubsystemStreamString":["ok",3]}']:
   with self.assertRaises(c.CodecError): c.decode_Payload(text)
unittest.main()
'''
                    for optimized in (False, True):
                        result = subprocess.run([sys.executable] + (['-O'] if optimized else []) + ['-c', probe],
                                                cwd=destination, capture_output=True, text=True)
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        self.assertIn('Ran 1 test', result.stderr)
                        self.assertIn('\nOK\n', result.stderr)
        print('PY012 PINNED ISOLATED MEMBER: PASSED releases=2 worlds=2 policies=2; NOT full-service interoperability', flush=True)