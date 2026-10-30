#!/usr/bin/env python3
"""Strict measured PY-009 successor gate; preserve all historical oracles."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/fixtures/python-datetime'
HISTORICAL_PY008 = {
    'before.json': 'b852426d9316b9d7cbe1349ae9da6683c940cf07773c3d6eb3c7ec8fafaddd95',
    'after.json': '31f1ee96956f44f09bd98efe10499b5f4f7daf33e0d203e5b49b8bd3ef21228d',
}


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


parent = module('py008_qualification', 'check-py008-uci.py')
measurement = module('py009_measurement', 'measure-py009-uci.py')
driver = measurement.driver


def historical():
    parent.historical()
    for name, digest in HISTORICAL_PY008.items():
        driver.require(driver.digest(ROOT / 'tests/fixtures/python-float64' / name) == digest,
                       'immutable PY-008 evidence modified: ' + name)


def validate(actual, frozen):
    rows = [dict(row, with_codec=row['codec'], after=row) for row in actual]
    summaries = parent.validate(rows, frozen)
    for row, expected in zip(actual, frozen):
        driver.require(row['selected_type_qnames'] == expected['selected_type_qnames'], 'selected closure changed')
        digest = hashlib.sha256(json.dumps(row['selected_type_qnames'], separators=(',', ':')).encode()).hexdigest()
        driver.require(digest == row['closure_sha256'] == expected['closure_sha256'], 'closure identity mismatch')
        driver.require(row['generation']['exit'] == 1 and row['artifacts'] == {}, 'unqualified generated service artifacts')
        for field in ['exit', 'stdout', 'stderr']:
            driver.require(row['generation'][field] == expected['generation'][field], 'generation diagnostic changed')
    return summaries


def profiles(generator, output):
    """Extract the actual pinned DateTime declaration, execute generated codecs.

    These isolated probes do not claim complete real-service interoperability.
    """
    import os
    for version in driver.PINS:
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT'])
        root = ET.parse(schema).getroot()
        declarations = [e for e in root.findall(driver.XS + 'simpleType') if e.get('name') == 'DateTimeType']
        if not declarations:
            # UCI 2.6 moved this declaration into an included security schema.
            for include in root.findall(driver.XS + 'include'):
                included = schema.parent / include.get('schemaLocation')
                declarations.extend(e for e in ET.parse(included).getroot().findall(driver.XS + 'simpleType')
                                    if e.get('name') == 'DateTimeType')
        driver.require(len(declarations) == 1, 'missing/ambiguous pinned DateTimeType')
        declaration = declarations[0]
        fragment = ET.Element(driver.XS + 'schema', {'targetNamespace': driver.OAM, 'xmlns:t': driver.OAM,
                                                    'elementFormDefault': 'qualified'})
        fragment.append(declaration)
        ET.SubElement(fragment, driver.XS + 'element', {'name': 'ProfileValue', 'type': 't:DateTimeType',
                                                      '{' + driver.OAM + '}version': '001.000.000.000'})
        extracted = output / f'{version}-datetime.xsd'
        ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
        contract = output / f'{version}-datetime.yaml'
        contract.write_text(driver.contract('ProfileValue', version))
        for world in ['closed-schema', 'open-extensions']:
            dest = output / ('profile_' + version.replace('.', '_') + '_' + world.replace('-', '_'))
            generated = driver.check(generator, extracted, contract, world, True, dest)
            driver.require(generated['exit'] == 0, str(generated))
            for optimized in [False, True]:
                for style in ['standalone', 'package']:
                    result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                            [str(FIXTURE / 'profile_probe.py'), str(dest), style],
                                            capture_output=True, text=True)
                    driver.require(result.returncode == 0 and 'Ran 2 tests' in result.stderr and '\nOK\n' in result.stderr,
                                   str(result))
    return 32


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    generator = args.generator.resolve()
    historical()
    baseline = json.loads((FIXTURE / 'after.json').read_text())
    driver.require(baseline.get('completed') is True, 'incomplete frozen measurements')
    actual = measurement.measure(generator, generator.parent / 'examples/python_datetime_closure', args.output.resolve())
    summary = validate(actual['measurements'], baseline['measurements'])
    executions = profiles(generator, args.output.resolve())
    (args.output / 'qualification.json').write_text(json.dumps(dict(completed=True, services=summary,
        profile_tests=executions, real_ready=0, generator_sha256=driver.digest(generator)), indent=2) + '\n')
    print('PY009 PINNED UCI: PASSED selections=24 isolated_profile_tests=32 real_ready=0')


if __name__ == '__main__':
    main()
