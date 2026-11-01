"""Authentic extracted Binary members/profiles, NOT full-service interoperability."""
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


class HexBinaryUci(unittest.TestCase):
    def test_pinned_direct_named_and_constrained_binary(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py013_pins', root / 'scripts/task101_evidence.py')
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
            owner = next(e for e in source.findall(xs + 'complexType') if e.get('name') == 'SubsystemStreamMDT')
            field = next(e for e in owner.findall('.//' + xs + 'element') if e.get('name') == 'SubsystemStreamBinary')
            with tempfile.TemporaryDirectory(prefix='py013-uci-') as temporary:
                path = Path(temporary)
                fragment = ET.Element(xs + 'schema', {'targetNamespace': ns, 'xmlns:uci': ns,
                                                     'elementFormDefault': 'qualified'})
                names = ['AA_CodeType', 'BDS_AddressType', 'SHA_2_256_HashType']
                if release == '26':
                    names = ['HexBinaryType', 'IFF_RegisterType'] + names
                for name in names:
                    fragment.append(copy.deepcopy(declarations[name]))
                payload = ET.SubElement(fragment, xs + 'complexType', {'name': 'Payload'})
                sequence = ET.SubElement(payload, xs + 'sequence')
                sequence.append(copy.deepcopy(field))
                for name in names:
                    ET.SubElement(sequence, xs + 'element', {'name': name, 'type': 'uci:' + name, 'minOccurs': '0'})
                ET.SubElement(fragment, xs + 'element', {'name': 'BinaryNotice', 'type': 'uci:Payload',
                                                       '{' + ns + '}version': '001.000.000.000'})
                extracted = path / 'authentic.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                for world in ['closed-schema', 'open-extensions']:
                    dest = path / world.replace('-', '_')
                    result = subprocess.run([generator, 'service-generate', '--schema', str(extracted),
                        '--contract', str(root / 'tests/fixtures/python-hexbinary/service.yaml'),
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
  for octets,wire in [(b'', ''),(b'\\x00\\x80\\xff','0080FF')]:
   field=octets if release=='25' else m.HexBinaryType(octets)
   value=m.Payload(subsystemstreambinary=field)
   self.assertEqual(c.encode_Payload(value),json.dumps({'SubsystemStreamBinary':wire},separators=(',',':')))
   self.assertEqual(c.decode_Payload(json.dumps({'SubsystemStreamBinary':wire.lower()})),value)
  for name,count in [('AA_CodeType',6),('BDS_AddressType',2),('SHA_2_256_HashType',32)]+([('IFF_RegisterType',7)] if release=='26' else []):
   cls=getattr(m,name);field=name.lower();octets=b'\\x80'*count
   value=m.Payload(**{field:cls(octets)})
   self.assertEqual(c.encode_Payload(value),json.dumps({name:'80'*count},separators=(',',':')))
   self.assertEqual(c.decode_Payload(json.dumps({name:'80'*count})),value)
   for bad in [count-1,count+1]:
    with self.assertRaises(ValueError): cls(b'\\x80'*bad)
    with self.assertRaises(c.CodecError) as caught: c.decode_Payload(json.dumps({name:'80'*bad}))
    self.assertIn('$.'+name,str(caught.exception))
unittest.main(argv=[sys.argv[0]])
'''
                    for optimized in [False, True]:
                        for style in ['standalone', 'package']:
                            result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                ['-c', probe, str(dest), style, release], capture_output=True, text=True)
                            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                            self.assertIn('Ran 1 test', result.stderr)
                evidence = os.environ.get('AMS_GRA_PY013_UCI_EVIDENCE')
                if evidence:
                    import shutil
                    shutil.copytree(path, Path(evidence) / release)