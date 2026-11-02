"""Authentic extracted Duration members/profiles, NOT full-service interoperability."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET


class DurationUci(unittest.TestCase):
    def test_pinned_direct_and_named_duration(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py014_pins', root / 'scripts/task101_evidence.py')
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        generator = str(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        xs = '{http://www.w3.org/2001/XMLSchema}'
        ns = 'https://www.vdl.afrl.af.mil/programs/oam'
        for release in ['25', '26']:
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release[-1] + '_ROOT'])
            include = schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')
            self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), pins.PINS[release])
            self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), pins.INCLUDES[release])
            source = ET.parse(schema).getroot()
            declarations = {e.get('name'): e for e in source.findall(xs + 'simpleType')}
            owner = next(e for e in source.findall(xs + 'complexType') if e.get('name') == 'LogMDT')
            field = next(e for e in owner.findall('.//' + xs + 'element') if e.get('name') == 'ServiceUpTime')
            with tempfile.TemporaryDirectory(prefix='py014-uci-') as temporary:
                path = Path(temporary)
                fragment = ET.Element(xs + 'schema', {'targetNamespace': ns, 'xmlns:uci': ns,
                                                     'elementFormDefault': 'qualified'})
                fragment.append(copy.deepcopy(declarations['DurationType']))
                payload = ET.SubElement(fragment, xs + 'complexType', {'name': 'Payload'})
                sequence = ET.SubElement(payload, xs + 'sequence')
                sequence.append(copy.deepcopy(field))
                ET.SubElement(sequence, xs + 'element', {'name': 'Named', 'type': 'uci:DurationType', 'minOccurs': '0'})
                ET.SubElement(fragment, xs + 'element', {'name': 'DurationNotice', 'type': 'uci:Payload',
                                                       '{' + ns + '}version': '001.000.000.000'})
                extracted = path / 'authentic.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                for world in ['closed-schema', 'open-extensions']:
                    dest = path / world.replace('-', '_')
                    result = subprocess.run([generator, 'service-generate', '--schema', str(extracted),
                        '--contract', str(root / 'tests/fixtures/python-duration/service.yaml'),
                        '--language', 'python', '--world', world, '--with-codec', '--output', str(dest)],
                        capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    probe = '''import importlib,json,sys,unittest
from pathlib import Path
dest=Path(sys.argv[1]);style=sys.argv[2];release=sys.argv[3]
sys.path.insert(0,str(dest if style=='standalone' else dest.parent))
prefix='' if style=='standalone' else dest.name+'.'
m=importlib.import_module(prefix+'model');c=importlib.import_module(prefix+'service_codec')
class Authentic(unittest.TestCase):
 def test_profiles(self):
  for text in ['P12M','-PT0.000S','PT1.2300S','P'+'9'*8192+'Y']:
   field=text if release=='25' else m.DurationType(text)
   value=m.Payload(serviceuptime=field,named=m.DurationType(text))
   self.assertEqual(c.encode_Payload(value),json.dumps({'ServiceUpTime':text,'Named':text},separators=(',',':')))
   self.assertEqual(c.decode_Payload(json.dumps({'ServiceUpTime':' '+text+' ','Named':text})),value)
  with self.assertRaises(ValueError):m.DurationType('PT')
  with self.assertRaises(c.CodecError) as caught:c.decode_Payload('{"ServiceUpTime":"PT"}')
  self.assertIn('$.ServiceUpTime',str(caught.exception))
unittest.main(argv=[sys.argv[0]])
'''
                    for optimized in [False, True]:
                        for style in ['standalone', 'package']:
                            result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                ['-c', probe, str(dest), style, release], capture_output=True, text=True)
                            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                            self.assertIn('Ran 1 test', result.stderr)
                evidence = os.environ.get('AMS_GRA_PY014_UCI_EVIDENCE')
                if evidence:
                    import shutil
                    shutil.copytree(path, Path(evidence) / release)