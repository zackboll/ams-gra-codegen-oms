"""PY-011 UCI discovery: authenticated declarations, independent lexical corpus."""
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


class NatoSpecialWordsUci(unittest.TestCase):
    def test_pinned_declarations(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py011_pins', root / 'scripts/task101_evidence.py')
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        fixture = root / 'tests/fixtures/python-nato-special-words'
        xs = '{http://www.w3.org/2001/XMLSchema}'
        namespace = 'https://www.vdl.afrl.af.mil/programs/oam'
        for release in ('25', '26'):
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release[-1] + '_ROOT'])
            include = schema.parent / f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
            self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), pins.PINS[release])
            self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), pins.INCLUDES[release])
            declarations = [e for e in ET.parse(include).getroot().findall(xs + 'simpleType')
                            if any(p.get('value') == r'NATO:[a-zA-Z\-_]{1,256}'
                                   for p in e.findall('.//' + xs + 'pattern'))]
            self.assertEqual(len(declarations), 1)
            declaration = declarations[0]
            name = declaration.get('name')
            with tempfile.TemporaryDirectory(prefix='py011-pinned-') as temporary:
                path = Path(temporary)
                fragment = ET.Element(xs + 'schema', {'targetNamespace': namespace, 'xmlns:t': namespace,
                                                     'elementFormDefault': 'qualified'})
                fragment.append(copy.deepcopy(declaration))
                ET.SubElement(fragment, xs + 'element', {'name': 'ProfileValue', 'type': 't:' + name,
                                                       '{' + namespace + '}version': '001.000.000.000'})
                extracted = path / 'profile.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                contract = path / 'service.yaml'
                contract.write_text((fixture / 'service.yaml').read_text().replace('TextNotice', 'ProfileValue'))
                for world in ('closed-schema', 'open-extensions'):
                    destination = path / world.replace('-', '_')
                    result = subprocess.run([str(generator), 'service-generate', '--schema', str(extracted),
                        '--contract', str(contract), '--language', 'python', '--world', world,
                        '--with-codec', '--output', str(destination)], capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    for optimized in (False, True):
                        for style in ('standalone', 'package'):
                            result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                [str(fixture / 'profile_probe.py'), str(destination), style, name],
                                capture_output=True, text=True)
                            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                            self.assertIn('Ran 2 tests', result.stderr)
                            self.assertIn('\nOK\n', result.stderr)
        print('PY011 PINNED DECLARATIONS: PASSED releases=2 generated_tests=32', flush=True)