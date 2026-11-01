"""Read-only UCI 2.6 Survivability inventory; NOT transport qualification."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('schema', 'generator', 'evidence'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    schema = args.schema.resolve()
    pins = {
        'UCI_MessageDefinitions_v2_6_0.xsd': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
        'UCI_SecurityMarkings_v2_6_0.xsd': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88',
    }
    for name, digest in pins.items():
        if hashlib.sha256((schema.parent / name).read_bytes()).hexdigest() != digest:
            raise ValueError('UCI 2.6 inventory pin mismatch')
    ns = {'xs': 'http://www.w3.org/2001/XMLSchema'}
    tree = ET.parse(schema).getroot()
    element = tree.find("xs:element[@name='SurvivabilityRiskLevel']", ns)
    if element is None or element.get('type') != 'uci:SurvivabilityRiskLevelMT':
        raise ValueError('UCI 2.6 global message absent/changed')
    command = [str(args.generator.resolve()), 'service-check', '--schema', str(schema),
               '--contract', str(ROOT / 'tests/fixtures/task104/uci25-service.yaml'),
               '--language', 'ada', '--world', 'closed-schema', '--with-codec']
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    report = dict(release='2.6', global_element='SurvivabilityRiskLevel',
                  payload_type=element.get('type'), schema_sources=pins,
                  command=command, exit=result.returncode, stdout=result.stdout, stderr=result.stderr,
                  compiler_qualified=False, transport_qualified=False)
    args.evidence.resolve().write_text(json.dumps(report, indent=2) + '\n')
    if result.returncode:
        raise SystemExit(result.returncode)
    print('TASK104_UCI26_READ_ONLY_INVENTORY_COMPLETE')


if __name__ == '__main__':
    main()