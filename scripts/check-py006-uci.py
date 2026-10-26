#!/usr/bin/env python3
"""Must-execute pinned Python selections; no download or broker build.

Use --before-generator for reproducible pre-implementation comparisons.
Without it the frozen measured service baseline is checked. Extracted primitive
probes are explicitly NOT real-UCI service/runtime qualification.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
XS = '{http://www.w3.org/2001/XMLSchema}'
ET.register_namespace('xs', XS[1:-1])
OAM = 'https://www.vdl.afrl.af.mil/programs/oam'
PINS = {
    '2.5': ('093610b7753944059360d3236770ab446d039556',
            'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27'),
    '2.6': ('78eb61b6112c8bffa40820c33124b57787fc5bd9',
            'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'),
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def contract(message, version):
    return f'''contract_version: "0.1"
service:
  name: PY006 {message}
  version: "0.6.0"
  kind: service
standards:
  oms_version: "2.5"
  uci_schema_version: "{version}"
functions:
  - id: selected
    name: Selected
    category: specific
    applicability: applicable
    exchanges:
      - id: output
        kind: oms_message
        direction: output
        mandate: mandatory
        message: {message}
        topic: py006.selected
        timing: {{kind: asynchronous}}
'''


def check(binary, schema, selected, world, codec, dest=None):
    args = [str(binary), 'service-generate' if dest else 'service-check',
            '--schema', str(schema), '--contract', str(selected),
            '--language', 'python', '--world', world]
    if codec:
        args.append('--with-codec')
    if dest:
        args += ['--output', str(dest)]
    result = subprocess.run(args, capture_output=True, text=True, check=False)
    return {'command': args, 'exit': result.returncode,
            'stdout': result.stdout, 'stderr': result.stderr}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--before-generator', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.generator = args.generator.resolve()
    if args.before_generator:
        args.before_generator = args.before_generator.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    frozen = json.loads((ROOT / 'tests/fixtures/python-header-profiles/before.json').read_text())
    expected_profiles = json.loads((ROOT / 'tests/fixtures/python-header-profiles/after.json').read_text())['isolated_pinned_profiles']
    require(len(frozen['measurements']) == 24, 'missing frozen BEFORE selections')
    rows, profiles = [], []
    for version, (revision, expected_hash) in PINS.items():
        schema = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT']).resolve()
        require(digest(schema) == expected_hash, f'{version}: pinned root hash mismatch')
        tree = ET.parse(schema).getroot()
        require(tree.get('targetNamespace') == OAM, 'unexpected namespace')
        for message in ['PositionReport', 'RDMA_InitializeSetup', 'SubsystemStream']:
            require(any(e.get('name') == message for e in tree.findall(XS + 'element')),
                    f'{version}: missing actual global message {message}')
            selected = args.output / f'{version}-{message}.yaml'
            selected.write_text(contract(message, version))
            for world in ['closed-schema', 'open-extensions']:
                for codec in [False, True]:
                    after = check(args.generator, schema, selected, world, codec)
                    require(after['exit'] == 1 and 'model status: NOT READY\n' in after['stdout']
                            and 'service api status: NOT READY\n' in after['stdout']
                            and '\nstatus: NOT READY\n' in after['stdout'], f'changed service boundary: {after}')
                    if codec:
                        require('codec: not measured' in after['stdout'], 'untruthful codec verdict')
                    if args.before_generator:
                        before = check(args.before_generator, schema, selected, world, codec)
                        require({k: before[k] for k in ['exit', 'stdout', 'stderr']} ==
                                {k: after[k] for k in ['exit', 'stdout', 'stderr']}, 'nonmonotonic/unexpected service delta')
                    else:
                        matches = [row for row in frozen['measurements'] if row['version'] == version
                                   and row['message'] == f'{{{OAM}}}{message}'
                                   and row['world'] == world and row['codec'] == codec]
                        require(len(matches) == 1 and matches[0]['root_sha256'] == expected_hash,
                                'BEFORE schema/selection identity mismatch')
                        before = matches[0]
                        require({k: before[k] for k in ['exit', 'stdout', 'stderr']} ==
                                {k: after[k] for k in ['exit', 'stdout', 'stderr']}, 'unexpected service delta')
                    rows.append(dict(version=version, revision=revision, root_sha256=expected_hash,
                                     message=f'{{{OAM}}}{message}', contract_sha256=digest(selected),
                                     world=world, with_codec=codec, before=before, after=after,
                                     artifacts={}, runtime='NOT QUALIFIED: service NOT READY'))
        # Actual pinned declarations, isolated without pretending the original
        # complex service closures became supported.
        for name in ['UCI_SchemaVersionStringType', 'UniversallyUniqueIdentifierType']:
            declaration = next(e for e in tree.findall(XS + 'simpleType') if e.get('name') == name)
            fragment = ET.Element(XS + 'schema', {'targetNamespace': OAM, 'xmlns:t': OAM,
                                                'elementFormDefault': 'qualified'})
            fragment.append(declaration)
            ET.SubElement(fragment, XS + 'element', {'name': 'ProfileValue', 'type': 't:' + name,
                                                   '{' + OAM + '}version': '001.000.000.000'})
            extracted = args.output / f'{version}-{name}.xsd'
            ET.ElementTree(fragment).write(extracted, encoding='utf-8', xml_declaration=True)
            selected = args.output / f'{version}-{name}.yaml'
            selected.write_text(contract('ProfileValue', version))
            for world in ['closed-schema', 'open-extensions']:
                dest = args.output / f'uci_{version.replace(".", "_")}_{name}_{world.replace("-", "_")}'
                after = check(args.generator, extracted, selected, world, True)
                require(after['exit'] == 0 and '\nstatus: READY\n' in after['stdout']
                        and 'codec status: READY' in after['stdout'], f'profile not READY: {after}')
                generated = check(args.generator, extracted, selected, world, True, dest)
                require(generated['exit'] == 0 and len(list(dest.glob('*.py'))) == 3,
                        f'incomplete generated profile: {generated}')
                good = ('002.5.0ab_Release-2A' if name == 'UCI_SchemaVersionStringType'
                        else '123E4567-e89B-42d3-A456-426614174000')
                bad = ('000.001.000.000' if name == 'UCI_SchemaVersionStringType'
                       else '123e4567-e89b-62d3-a456-426614174000')
                # Literal expected JSON, not merely an encode/decode round trip.
                probe = f'''import importlib,sys,unittest
from pathlib import Path
dest=Path(sys.argv[1]);style=sys.argv[2]
sys.path.insert(0,str(dest if style=='standalone' else dest.parent))
prefix='' if style=='standalone' else dest.name+'.'
m=importlib.import_module(prefix+'model');c=importlib.import_module(prefix+'service_codec')
class PinnedProfile(unittest.TestCase):
 def test_literal(self):
  value=m.{name}({good!r})
  self.assertEqual(c.encode_{name}(value),{json.dumps(good)!r})
  self.assertEqual(c.decode_{name}({json.dumps(good)!r}),value)
  with self.assertRaises(ValueError):m.{name}({bad!r})
  with self.assertRaises(c.CodecError):c.decode_{name}({json.dumps(bad)!r})
  object.__setattr__(value,'value',{bad!r})
  with self.assertRaises(c.CodecError):c.encode_{name}(value)
unittest.main(argv=['pinned-profile'],verbosity=2)
'''
                for optimized in [False, True]:
                    for style in ['standalone', 'package']:
                        executed = subprocess.run([sys.executable] + (['-O'] if optimized else [])
                                                  + ['-c', probe, str(dest), style], capture_output=True, text=True)
                        require(executed.returncode == 0 and 'Ran 1 test' in executed.stderr
                                and '\nOK\n' in executed.stderr, executed.stderr)
                expected_profile = [row for row in expected_profiles if row['version'] == version
                                    and row['declaration'] == name and row['world'] == world]
                require(len(expected_profile) == 1, 'missing profile baseline')
                before = expected_profile[0]['before']
                artifact_hashes = {p.name: digest(p) for p in sorted(dest.glob('*.py'))}
                require(artifact_hashes == expected_profile[0]['artifacts'], 'nonreproducible generated profile')
                if args.before_generator:
                    before = check(args.before_generator, extracted, selected, world, True)
                    require(before['exit'] == 1 and 'model status: NOT READY' in before['stdout'], 'missing profile delta')
                profiles.append(dict(version=version, declaration=name, world=world,
                                     provenance='extracted pinned declaration, NOT complete real service',
                                     before=before, after=after,
                                     execution='4 literal-oracle tests: normal/-O standalone/package',
                                     artifacts=artifact_hashes))
    require(len(rows) == 24 and len(profiles) == 8, 'incomplete campaign')
    result = {'services': rows, 'isolated_pinned_profiles': profiles}
    text = json.dumps(result, indent=2)
    for path, replacement in [(str(args.output.resolve()), '<EVIDENCE>'), (str(ROOT), '<REPO>')]:
        text = text.replace(path, replacement)
    (args.output / 'measurements.json').write_text(text + '\n')
    print('PY006 PINNED UCI: PASSED services=24 isolated_profiles=8 real_ready=0')


if __name__ == '__main__':
    main()