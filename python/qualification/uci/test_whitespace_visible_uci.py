"""Isolated original pinned declarations: NOT complete UCI service qualification."""
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


class WhitespaceVisibleUci(unittest.TestCase):
    def test_original_declarations(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py016_pins', root / 'scripts/task101_evidence.py')
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        xs = '{http://www.w3.org/2001/XMLSchema}'
        ns = 'https://www.vdl.afrl.af.mil/programs/oam'
        names = ['WhitespaceVisibleString1024Type', 'WhitespaceVisibleString4096Type', 'QueryString4096Type']
        for release in ['25', '26']:
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release[-1] + '_ROOT'])
            include = schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')
            self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), pins.PINS[release])
            self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), pins.INCLUDES[release])
            declarations = {e.get('name'): e for file in [schema, include]
                            for e in ET.parse(file).getroot() if e.get('name') in names}
            self.assertEqual(set(declarations), set(names))
            with tempfile.TemporaryDirectory(prefix='py016-authentic-declarations-') as temporary:
                path = Path(temporary)
                fragment = ET.Element(xs + 'schema', {'targetNamespace': ns, 'xmlns:uci': ns,
                                                     'elementFormDefault': 'qualified'})
                for name in names:
                    fragment.append(copy.deepcopy(declarations[name]))
                payload = ET.SubElement(fragment, xs + 'complexType', {'name': 'Payload'})
                sequence = ET.SubElement(payload, xs + 'sequence')
                for field, name in zip(['Short', 'Long', 'Query'], names):
                    ET.SubElement(sequence, xs + 'element', {'name': field, 'type': 'uci:' + name})
                ET.SubElement(fragment, xs + 'element', {'name': 'WhitespaceNotice', 'type': 'uci:Payload',
                                                       '{' + ns + '}version': '001.000.000.000'})
                extracted = path / 'original-declarations.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                for world in ['closed-schema', 'open-extensions']:
                    for codec in [False, True]:
                        dest = path / (world.replace('-', '_') + str(codec))
                        result = subprocess.run([generator, 'service-generate', '--schema', str(extracted),
                            '--contract', str(root / 'tests/fixtures/python-whitespace-visible/service.yaml'),
                            '--language', 'python', '--world', world, '--output', str(dest)] +
                            (['--with-codec'] if codec else []), capture_output=True, text=True)
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        probe = r'''import importlib,json,sys,unittest
from pathlib import Path
dest=Path(sys.argv[1]);style=sys.argv[2];release=sys.argv[3];codec=sys.argv[4]=='True'
sys.path.insert(0,str(dest if style=='standalone' else dest.parent))
prefix='' if style=='standalone' else dest.name+'.'
m=importlib.import_module(prefix+'model')
c=importlib.import_module(prefix+'service_codec') if codec else None
class OriginalDeclarations(unittest.TestCase):
 def test_profiles(self):
  carriers=[m.WhitespaceVisibleString1024Type,m.WhitespaceVisibleString4096Type,m.QueryString4096Type]
  for index,T in enumerate(carriers):
   collapse=release=='25' and index<2
   upper=1024 if index==0 else 4096
   text=' a'+chr(10)+chr(13)+'  b '
   self.assertEqual(T(text).value,'a b' if collapse else text)
   self.assertEqual(T('x'*upper).value,'x'*upper)
   with self.assertRaises(ValueError):T('x'*(upper+1))
   if release=='25':self.assertEqual(T('').value,'')
   else:
    with self.assertRaises(ValueError):T('')
   if collapse:self.assertEqual(T((' '+chr(9))*4000+'a'+chr(10)*4000+'b').value,'a b')
   else:
    with self.assertRaises(ValueError):T('a'+chr(9)+'b')
   for invalid in [chr(0),chr(11),chr(12),chr(127),chr(160),chr(0xd800),'é']:
    with self.assertRaises(ValueError):T(invalid)
  value=m.Payload(short=carriers[0](text),long=carriers[1](text),query=carriers[2](text))
  if codec:
   wire=c.encode_Payload(value)
   expected={key:('a b' if release=='25' and key!='Query' else text) for key in ['Short','Long','Query']}
   self.assertEqual(json.loads(wire),expected)
   self.assertEqual(c.decode_Payload(json.dumps(dict.fromkeys(expected,text))),value)
   self.assertEqual(c.encode_Payload(c.decode_Payload(wire)),wire)
   for field in expected:
    for invalid in [None,1,[],chr(0),chr(160)]+([''] if release=='26' else []):
     with self.assertRaises(c.CodecError) as caught:c.decode_Payload(json.dumps({**expected,field:invalid}))
     self.assertIn('$.'+field+':',str(caught.exception))
unittest.main(argv=[sys.argv[0]])
'''
                        for optimized in [False, True]:
                            for style in ['standalone', 'package']:
                                result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                    ['-c', probe, str(dest), style, release, str(codec)], capture_output=True, text=True)
                                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                                self.assertIn('Ran 1 test', result.stderr)