#!/usr/bin/env python3
"""Strict measured PY-010 successor gate; preserve all historical oracles."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/fixtures/python-visible-ascii'
HISTORICAL_PY009 = {'before.json': '0fa0cb64e84d461532481b666194c8a8eac822cbab72164290cfd3d6c55d9aa9', 'after.json': 'c46102cc62324bb9656f18b5836a6f21a14f09881a0d1fb05a9e3889fdb02f60'}


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


parent = module('py009_qualification', 'check-py009-uci.py')
measurement = module('py010_measurement', 'measure-py010-uci.py')
driver = measurement.driver


def historical():
    parent.historical()
    for name, digest in HISTORICAL_PY009.items():
        driver.require(driver.digest(ROOT / 'tests/fixtures/python-datetime' / name) == digest,
                       'immutable PY-009 evidence modified: ' + name)


def validate(actual, frozen):
    summaries = parent.validate(actual, frozen)
    for row, expected in zip(actual, frozen):
        driver.require(row['selected_type_qnames'] == expected['selected_type_qnames'], 'selected closure changed')
        digest = hashlib.sha256(json.dumps(row['selected_type_qnames'], separators=(',', ':')).encode()).hexdigest()
        driver.require(digest == row['closure_sha256'] == expected['closure_sha256'], 'closure identity mismatch')
        driver.require(row['generation']['exit'] == 1 and row['artifacts'] == {}, 'unqualified generated service artifacts')
        for field in ['exit', 'stdout', 'stderr']:
            driver.require(row['generation'][field] == expected['generation'][field], 'generation diagnostic changed')
    return summaries


def profiles(generator, output):
    """Extract actual pinned declarations, select profiles through the Rust classifier.

    These probes qualify real declaration facets, NOT complete UCI services.
    """
    import copy
    import os
    import subprocess
    import sys
    import xml.etree.ElementTree as ET
    count = 0
    inventory = []
    for version in driver.PINS:
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT'])
        result = subprocess.run([str(generator.parent / 'examples/python_visible_ascii_profiles'), str(schema)],
                                capture_output=True, text=True, check=True)
        profiles = json.loads(result.stdout)
        driver.require(len(profiles) == 13, 'pinned visible-ASCII inventory changed')
        driver.require({(p['min'], p['max']) for p in profiles} ==
                       {(1, n) for n in [20, 32, 64, 81, 128, 256, 480, 512, 1024]} | {(2, 4)},
                       'missing authoritative profile')
        declarations = {}
        visited = set()

        def load(path):
            path = path.resolve()
            if path in visited:
                return
            visited.add(path)
            root = ET.parse(path).getroot()
            for declaration in root.findall(driver.XS + 'simpleType'):
                name = declaration.get('name')
                driver.require(name not in declarations, 'ambiguous pinned declaration')
                declarations[name] = declaration
            for include in root.findall(driver.XS + 'include'):
                load(path.parent / include.get('schemaLocation'))
        load(schema)
        for profile in profiles:
            name = profile['name']
            fragment = ET.Element(driver.XS + 'schema', {'targetNamespace': driver.OAM,
                                  'xmlns:t': driver.OAM, 'elementFormDefault': 'qualified'})
            added = set()

            def append(name):
                if name in added:
                    return
                added.add(name)
                declaration = copy.deepcopy(declarations[name])
                restriction = declaration.find(driver.XS + 'restriction')
                base = restriction.get('base')
                if base.split(':')[-1] in declarations:
                    parent_name = base.split(':')[-1]
                    append(parent_name)
                    restriction.set('base', 't:' + parent_name)
                else:
                    driver.require(base.split(':')[-1] == 'string', 'unproven pinned base')
                    restriction.set('base', 'xs:string')
                fragment.append(declaration)
            append(name)
            ET.SubElement(fragment, driver.XS + 'element', {'name': 'ProfileValue', 'type': 't:' + name,
                          '{' + driver.OAM + '}version': '001.000.000.000'})
            extracted = output / f'{version}-{name}.xsd'
            ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
            contract = extracted.with_suffix('.yaml')
            contract.write_text(driver.contract('ProfileValue', version))
            for world in ['closed-schema', 'open-extensions']:
                dest = output / ('visible_' + version.replace('.', '_') + '_' + name + '_' + world.replace('-', '_'))
                generated = driver.check(generator, extracted, contract, world, True, dest)
                driver.require(generated['exit'] == 0, str(generated))
                for optimized in [False, True]:
                    for style in ['standalone', 'package']:
                        result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                            [str(FIXTURE / 'profile_probe.py'), str(dest), style, name,
                             str(profile['min']), str(profile['max'])], capture_output=True, text=True)
                        driver.require(result.returncode == 0 and 'Ran 2 tests' in result.stderr and '\nOK\n' in result.stderr,
                                       str(result))
                        count += 2
                inventory.append(dict(version=version, profile=profile, world=world,
                                      extracted_sha256=driver.digest(extracted),
                                      artifacts={p.name: driver.digest(p) for p in sorted(dest.glob('*.py'))}))
    (output / 'visible-profiles.json').write_text(json.dumps(inventory, indent=2) + '\n')
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--service-baseline', type=Path, default=FIXTURE / 'after.json',
                        help='Explicit measured successor; historical evidence is unchanged')
    args = parser.parse_args()
    generator = args.generator.resolve()
    historical()
    baseline = json.loads(args.service_baseline.read_text())
    driver.require(baseline.get('completed') is True, 'incomplete frozen measurements')
    actual = measurement.measure(generator, generator.parent / 'examples/python_datetime_closure', args.output.resolve())
    summary = validate(actual['measurements'], baseline['measurements'])
    executions = profiles(generator, args.output.resolve())
    (args.output / 'qualification.json').write_text(json.dumps(dict(completed=True, services=summary,
        profile_tests=executions, real_ready=0, generator_sha256=driver.digest(generator)), indent=2) + '\n')
    print(f'PY010 PINNED UCI: PASSED selections=24 isolated_profile_tests={executions} real_ready=0')


if __name__ == '__main__':
    main()
