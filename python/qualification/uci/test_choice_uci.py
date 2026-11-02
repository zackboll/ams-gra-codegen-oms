"""Original pinned security declarations; explicitly not full-service qualification."""
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


class ChoiceUci(unittest.TestCase):
    def test_original_security_choices(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py015_pins', root / 'scripts/task101_evidence.py')
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        xs = '{http://www.w3.org/2001/XMLSchema}'
        ns = 'https://www.vdl.afrl.af.mil/programs/oam'
        choices = ['FGI_SourceOpenChoiceType', 'FGI_SourceProtectedChoiceType',
                   'OwnerProducerChoiceType', 'ReleasableToChoiceType']
        for release in ('25', '26'):
            schema = Path(os.environ['AMS_GRA_UCI_2_' + release[-1] + '_ROOT'])
            include = schema.parent / f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'
            self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), pins.PINS[release])
            self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), pins.INCLUDES[release])
            declarations = {e.get('name'): e for file in [schema, include]
                            for e in ET.parse(file).getroot() if e.get('name')}
            selected = set(choices)
            pending = list(choices)
            while pending:
                for element in declarations[pending.pop()].iter():
                    for attribute in ['type', 'base']:
                        reference = element.get(attribute, '')
                        if reference.startswith('uci:') and reference[4:] not in selected:
                            selected.add(reference[4:])
                            pending.append(reference[4:])
            with tempfile.TemporaryDirectory(prefix='py015-pinned-') as temporary:
                path = Path(temporary)
                fragment = ET.Element(xs + 'schema', {'targetNamespace': ns, 'xmlns:uci': ns,
                                                     'elementFormDefault': 'qualified'})
                for name, declaration in declarations.items():
                    if name in selected:
                        fragment.append(copy.deepcopy(declaration))
                record = ET.SubElement(fragment, xs + 'complexType', {'name': 'Payload'})
                sequence = ET.SubElement(record, xs + 'sequence')
                for name, type_name, minimum, maximum in [
                    ('OwnerProducer', choices[2], '1', 'unbounded'),
                    ('FGI_SourceOpen', choices[0], '0', '1'),
                    ('FGI_SourceProtected', choices[1], '0', '1'),
                    ('ReleasableTo', choices[3], '0', 'unbounded'),
                    ('DisplayOnlyTo', choices[3], '0', 'unbounded')]:
                    ET.SubElement(sequence, xs + 'element', {'name': name, 'type': 'uci:' + type_name,
                                                            'minOccurs': minimum, 'maxOccurs': maximum})
                ET.SubElement(fragment, xs + 'element', {'name': 'ChoiceNotice', 'type': 'uci:Payload',
                                                       '{' + ns + '}version': '001.000.000.000'})
                extracted = path / 'original-choices.xsd'
                ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
                for world in ['closed-schema', 'open-extensions']:
                    for codec in [False, True]:
                        dest = path / (world.replace('-', '_') + str(codec))
                        result = subprocess.run([str(generator), 'service-generate', '--schema', str(extracted),
                            '--contract', str(root / 'tests/fixtures/python-choice/service.yaml'),
                            '--language', 'python', '--world', world, '--output', str(dest)] +
                            (['--with-codec'] if codec else []), capture_output=True, text=True)
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        for optimized in [False, True]:
                            for style in ['standalone', 'package']:
                                result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                    [str(root / 'tests/fixtures/python-choice/uci_probe.py'), str(dest), style],
                                    capture_output=True, text=True)
                                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                                self.assertIn('Ran 2 tests', result.stderr)
        print('PY015 ORIGINAL CHOICES: releases=2 choices=4; NOT complete services', flush=True)