"""Execute an authenticated pinned declaration, not a full-service claim."""
import importlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET


class OwnedUci(unittest.TestCase):
    def test_generated_pinned_datetime(self):
        root = Path(__file__).resolve().parents[3]
        fixtures = Path(__file__).parent / 'fixtures/test_owned_uci'
        vectors = json.loads((fixtures / 'values.json').read_text())
        xs = '{http://www.w3.org/2001/XMLSchema}'
        namespace = 'https://www.vdl.afrl.af.mil/programs/oam'
        for release in ('5', '6'):
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release + '_ROOT'])
            source = ET.parse(schema).getroot()
            declarations = [e for e in source.findall(xs + 'simpleType') if e.get('name') == 'DateTimeType']
            for include in source.findall(xs + 'include'):
                declarations.extend(e for e in ET.parse(schema.parent / include.get('schemaLocation')).getroot().findall(xs + 'simpleType')
                                    if e.get('name') == 'DateTimeType')
            self.assertEqual(len(declarations), 1)
            with tempfile.TemporaryDirectory(prefix='ci004-python-uci-') as temporary:
                fragment = ET.Element(xs + 'schema', {'targetNamespace': namespace, 'xmlns:t': namespace,
                                                     'elementFormDefault': 'qualified'})
                fragment.append(declarations[0])
                ET.SubElement(fragment, xs + 'element', {'name': 'ProfileValue', 'type': 't:DateTimeType',
                                                       '{' + namespace + '}version': '001.000.000.000'})
                extracted = Path(temporary) / 'pinned-datetime.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                generated = subprocess.run([str(root / 'target/release/ams-gra-codegen-oms'), 'service-generate',
                                '--schema', str(extracted), '--contract', str(fixtures / 'service.yaml'),
                                '--language', 'python', '--world', 'closed-schema', '--with-codec',
                                '--output', temporary + '/ci004_uci'], capture_output=True, text=True)
                self.assertEqual(generated.returncode, 0, generated.stdout + generated.stderr)
                sys.path.insert(0, temporary)
                try:
                    codec = importlib.import_module('ci004_uci.service_codec')
                    for value in vectors['valid']:
                        encoded = codec.encode_DateTimeType(codec.decode_DateTimeType(json.dumps(value)))
                        self.assertEqual(json.loads(encoded), value)
                    for value in vectors['invalid']:
                        with self.assertRaises(codec.CodecError):
                            codec.decode_DateTimeType(json.dumps(value))
                finally:
                    sys.path.remove(temporary)
                    for name in list(sys.modules):
                        if name == 'ci004_uci' or name.startswith('ci004_uci.'):
                            del sys.modules[name]