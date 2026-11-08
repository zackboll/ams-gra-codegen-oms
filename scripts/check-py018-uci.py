#!/usr/bin/env python3
"""PY-018 real pinned facet qualification and measured AO readiness, no IR widening."""
import argparse
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/fixtures/python-unicode31'
BASE = '1c48bcbff22535ef92b870d7a12ef6be7fdbf90b'
BASE_TREE = '7059845473ad84876760a38995c838341a04bd92'
spec = importlib.util.spec_from_file_location('py018_prior', ROOT / 'scripts/check-py017-uci.py')
prior = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prior)
driver = prior.driver
NAMES = ['NITF_DateAndTimeType', 'NITF_DateType', 'NITF_MSTGTA_TargetLocationType']


def execute_profiles(generator, output):
    """Copy unchanged actual declarations into a labelled synthetic envelope.

    No authored facet is changed. EntryType/Payload are only qualification
    scaffolding; they are not advertised as an authentic selected UCI service.
    Absence of Choices deliberately exercises recursive stored-value checking.
    """
    inventory = []
    for version, (_, digest) in driver.PINS.items():
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT'])
        driver.require(driver.digest(schema) == digest, 'pinned root mismatch')
        include = schema.with_name(f'UCI_SecurityMarkings_v2_{version[-1]}_0.xsd')
        driver.require(driver.digest(include) == prior.measurement.INCLUDES[version], 'pinned include mismatch')
        profiles = json.loads(subprocess.check_output([str(generator.parent / 'examples/python_unicode_profiles'), str(schema)]))
        driver.require({row['name'] for row in profiles} == set(NAMES) and len(profiles) == 3,
                       'exact pinned Unicode profile inventory changed')
        for row in profiles:
            index = NAMES.index(row['name'])
            driver.require(row['profile'] == index and row['length'] == [12, 8, 21][index]
                           and row['white_space'] == 'None' and row['namespace'] == driver.OAM,
                           'wrong normalized profile')
        actual = ET.parse(schema).getroot()
        fragment = ET.parse(FIXTURE / 'oam.xsd').getroot()
        fragment.set('xmlns:t', driver.OAM)
        fragment.set('xmlns:xs', driver.XS[1:-1])
        fragment.set('version', '002.' + version[-1] + '.0')
        # Remove synthetic simple carriers and Choices; preserve exact real
        # named declarations and compare raw facets to shared normalized proof.
        for decl in list(fragment):
            if decl.tag == driver.XS + 'simpleType' or decl.get('name') in ['Selection', 'Payload', 'UnicodeNotice']:
                fragment.remove(decl)
        for name in NAMES:
            matches = [d for d in actual.findall(driver.XS + 'simpleType') if d.get('name') == name]
            driver.require(len(matches) == 1, 'missing/duplicate authentic declaration')
            decl = copy.deepcopy(matches[0])
            restriction = decl.find(driver.XS + 'restriction')
            driver.require(restriction.get('base').split(':')[-1] == 'string', 'unexpected real ancestry')
            # Prefixes are syntactic namespace bindings, not facet edits.
            restriction.set('base', 'xs:string')
            row = next(p for p in profiles if p['name'] == name)
            driver.require([p.get('value') for p in restriction.findall(driver.XS + 'pattern')] == row['patterns'][0]
                           and int(restriction.find(driver.XS + 'length').get('value')) == row['length'],
                           'raw/effective facets differ')
            fragment.insert(0, decl)
        payload = ET.SubElement(fragment, driver.XS + 'complexType', {'name': 'Payload'})
        sequence = ET.SubElement(payload, driver.XS + 'sequence')
        ET.SubElement(sequence, driver.XS + 'element', {'name': 'Entry', 'type': 't:EntryType',
                                                     'minOccurs': '0', 'maxOccurs': 'unbounded'})
        ET.SubElement(fragment, driver.XS + 'element', {'name': 'UnicodeNotice', 'type': 't:Payload',
                      '{' + driver.OAM + '}version': '002.' + version[-1] + '.0'})
        extracted = output / ('real-' + version + '.xsd')
        # ElementTree emits xs itself; avoid duplicating that binding.
        fragment.attrib.pop('xmlns:xs')
        ET.register_namespace('xs', driver.XS[1:-1])
        ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
        contract = output / ('real-' + version + '.yaml')
        contract.write_text(driver.contract('UnicodeNotice', version))
        for world in ['closed-schema', 'open-extensions']:
            dest = output / ('real_' + version.replace('.', '_') + '_' + world.replace('-', '_'))
            generated = driver.check(generator, extracted, contract, world, True, dest)
            driver.require(generated['exit'] == 0, str(generated))
            executions = []
            for optimized in [False, True]:
                for style in ['standalone', 'package']:
                    result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                        [str(FIXTURE / 'probe.py'), str(dest), style], capture_output=True, text=True)
                    driver.require(result.returncode == 0 and 'Ran 4 tests' in result.stderr,
                                   result.stdout + result.stderr)
                    executions.append(dict(optimized=optimized, style=style, tests=4))
            inventory.append(dict(version=version, world=world, root_sha256=digest,
                include_sha256=driver.digest(include), profiles=profiles,
                extracted_sha256=driver.digest(extracted),
                artifacts={p.name: driver.digest(p) for p in sorted(dest.glob('*.py'))}, executions=executions))
    (output / 'profiles.json').write_text(json.dumps(inventory, indent=2) + '\n')
    print('PY018 REAL PINNED PROFILES PASSED releases=2 profiles=3 worlds=2 executions=16 tests=64')
    return inventory


def measure_ao(generator, output):
    rows = []
    closure_tool = generator.parent / 'examples/python_datetime_closure'
    for version in driver.PINS:
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT'])
        driver.require(driver.digest(schema) == driver.PINS[version][1], 'wrong AO schema')
        contract = output / ('ao-' + version + '.yaml')
        contract.write_text(driver.contract('AO_CapabilityStatus', version))
        closure = json.loads(subprocess.check_output([str(closure_tool), str(schema), str(contract)]))
        for world in ['closed-schema', 'open-extensions']:
            for codec in [False, True]:
                result = driver.check(generator, schema, contract, world, codec)
                dest = output / f'ao-{version}-{world}-{codec}'
                generated = driver.check(generator, schema, contract, world, codec, dest)
                driver.require(result['exit'] == generated['exit'], 'readiness/generation disagreement')
                artifacts = {p.name: driver.digest(p) for p in sorted(dest.glob('*.py'))}
                if result['exit']:
                    driver.require(not dest.exists(), 'partial unsupported service')
                rows.append(dict(version=version, world=world, codec=codec, selected_type_qnames=closure,
                    closure_sha256=hashlib.sha256(json.dumps(closure, separators=(',', ':')).encode()).hexdigest(),
                    contract_sha256=driver.digest(contract), artifacts=artifacts, generation=generated, **result))
                (output / 'measurements.json').write_text(json.dumps(rows, indent=2) + '\n')
                print('PY018 AO', version, world, codec, result['exit'], flush=True)
    return rows


def validate_delta(before, after):
    driver.require(len(before) == len(after) == 8, 'incomplete AO campaign')
    keys = {(v, w, c) for v in driver.PINS for w in ['closed-schema', 'open-extensions'] for c in [False, True]}
    for stage in [before, after]:
        driver.require({(r['version'], r['world'], r['codec']) for r in stage} == keys, 'wrong AO cells')
        for row in stage:
            boundary = ('abstract value representations are unsupported: NITF_PackingPlanPET'
                        if row['world'] == 'closed-schema' else
                        'abstract value NITF_PackingPlanPET is not closed under open-extensions generation')
            driver.require(row['exit'] == 1 and row['artifacts'] == {} and
                           boundary in row['stdout'],
                           'AO boundary changed: requires actual typed execution before claiming READY')
    for old, new in zip(before, after):
        for key in ['selected_type_qnames', 'closure_sha256', 'contract_sha256', 'stdout', 'stderr', 'artifacts']:
            driver.require(old[key] == new[key], 'unexpected AO change: ' + key)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--profiles-only', action='store_true')
    parser.add_argument('--before-generator', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    generator = args.generator.resolve()
    execute_profiles(generator, args.output)
    if not args.profiles_only:
        driver.require(args.before_generator is not None, 'before generator required for measured delta')
        before_dir = args.output / 'before'
        after_dir = args.output / 'after'
        before_dir.mkdir()
        after_dir.mkdir()
        before = measure_ao(args.before_generator.resolve(), before_dir)
        after = measure_ao(generator, after_dir)
        validate_delta(before, after)
        (args.output / 'ao-delta.json').write_text(json.dumps(dict(base=BASE, base_tree=BASE_TREE,
            before=before, after=after, generator_sha256=driver.digest(generator),
            before_generator_sha256=driver.digest(args.before_generator.resolve())), indent=2) + '\n')
        print('PY018 AO ACTUAL READINESS before=0/8 after=0/8 blocker=NITF_PackingPlanPET')


if __name__ == '__main__':
    main()