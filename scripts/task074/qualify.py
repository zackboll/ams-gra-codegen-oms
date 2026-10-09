#!/usr/bin/env python3
"""Deterministic research qualification; each negative asserts its own stage."""
import argparse
import copy
import json
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import sys
import tempfile

from lxml import etree as E
from bundle import Bundle, PIN, Rejected, SnapshotResolver, X, digest, make_manifest, verify_toolchain

REPO = Path(__file__).resolve().parents[2]
FIXTURE = REPO / 'tests/fixtures/task074'
RESULTS = []
UCI = 'https://www.vdl.afrl.af.mil/programs/oam'
ROOT_HASH = {
    '2.5': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
    '2.6': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
}
SEC_HASH = {
    '2.5': '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b',
    '2.6': 'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88',
}


def case(name, expected, action):
    try:
        result = action()
    except Rejected as exc:
        result = str(exc)
    assert result == expected, (name, expected, result)
    RESULTS.append({'case': name, 'result': result})


def xml(kind='p:Known', count='1', secret=''):
    return (f'<p:Value xmlns:p="urn:task074:public" xmlns:s="urn:task074:private" '
            f'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="{kind}">'
            f'<p:Count>{count}</p:Count>{secret}</p:Value>').encode()


def synthetic():
    with tempfile.TemporaryDirectory(prefix='task074-') as tmp:
        root = Path(tmp)
        shutil.copytree(FIXTURE, root, dirs_exist_ok=True)
        m = json.loads((root / 'manifest.json').read_text())
        b = Bundle(root, m, research=True)
        case('authority gate', 'authority-not-established', lambda: Bundle(root, m))
        case('known concrete', 'valid-xml-only', lambda: b.validate(xml()))
        private = xml('s:Known', secret='<s:Secret>AB</s:Secret>')
        case('approved private / same local name different namespace', 'valid-xml-only', lambda: b.validate(private))
        case('defined but unapproved', 'unapproved-type', lambda: b.validate(xml('s:Unapproved')))
        case('unknown type', 'invalid:SCHEMAV_CVC_ELT_4_2', lambda: b.validate(xml('s:Absent')))
        case('wrong namespace', 'invalid:SCHEMAV_CVC_ELT_4_2', lambda: b.validate(private.replace(b'urn:task074:private', b'urn:wrong')))
        case('malformed xsi:type', 'invalid:SCHEMAV_CVC_DATATYPE_VALID_1_2_1', lambda: b.validate(xml('bad::name')))
        case('incompatible xsi:type', 'invalid:SCHEMAV_CVC_ELT_4_3', lambda: b.validate(xml('p:Other')))
        case('abstract xsi:type', 'invalid:SCHEMAV_CVC_TYPE_2', lambda: b.validate(xml('p:Base')))
        case('inherited constraint', 'invalid:SCHEMAV_CVC_DATATYPE_VALID_1_2_1', lambda: b.validate(xml(count='0')))
        case('private restriction', 'invalid:SCHEMAV_CVC_PATTERN_VALID', lambda: b.validate(private.replace(b'AB', b'12')))
        case('missing private dependency', 'missing-file', lambda: missing(root, m))
        case('omitted closure member', 'closure', lambda: Bundle(root, {**m, 'files': m['files'][:-1]}, research=True))
        case('modified schema unchanged digest', 'digest', lambda: changed(root, m))
        case('manifest path traversal', 'path', lambda: Bundle(root, {**m, 'files': [{**m['files'][0], 'path': '../escape.xsd'}]}, research=True))
        case('symlink', 'symlink', lambda: symlink(root, m))
        case('duplicate declarations', 'schema:SCHEMAP_REDEFINED_TYPE', lambda: conflict(root, m))
        case('network import', 'path', lambda: retrieval(root, 'http://127.0.0.1:9/evil.xsd'))
        case('external file include', 'path', lambda: retrieval(root, 'file:///etc/passwd'))
        case('include traversal', 'path', lambda: retrieval(root, '../escape.xsd'))
        case('namespace mismatch in manifest', 'namespace', lambda: Bundle(root, {**m, 'files': [{**m['files'][0], 'namespace': 'wrong'}, *m['files'][1:]]}, research=True))
        case('closure order', 'closure-order', lambda: Bundle(root, {**m, 'files': list(reversed(m['files']))}, research=True))
        case('schema version substitution', 'digest', lambda: Bundle(root, {**m, 'files': [{**m['files'][0], 'sha256': '0'*64}, *m['files'][1:]]}, research=True))
        attack = xml().replace(b'xsi:type=', b'xsi:schemaLocation="urn:task074:public http://127.0.0.1:9/evil" xsi:type=')
        case('schema-location override', 'schema-location', lambda: b.validate(attack))
        case('external entity', 'dtd', lambda: b.validate(b'<!DOCTYPE a [<!ENTITY e SYSTEM "file:///etc/passwd">]><a>&e;</a>'))
        case('entity expansion', 'dtd', lambda: b.validate(b'<!DOCTYPE a [<!ENTITY a "x"><!ENTITY b "&a;&a;">]><a>&b;</a>'))
        case('oversize', 'size', lambda: b.validate(b' ' * 65537))
        case('depth', 'depth', lambda: b.validate(b'<a>'*65 + b'</a>'*65))
        deep = xml().replace(b'</p:Count>', b'</p:Count>' + b'<p:Next xsi:type="p:Known"><p:Count>1</p:Count>'*64 + b'</p:Next>'*64)
        case('excessive recursive value', 'depth', lambda: b.validate(deep))
        case('malformed UTF-8', 'utf8', lambda: b.validate(b'<a>\xff</a>'))
        case('alternate encoding declaration', 'encoding', lambda: b.validate(b'<?xml version="1.0" encoding="ISO-8859-1"?><a/>'))
        case('malformed XML', 'xml', lambda: b.validate(b'<a>'))
        # Finite recursive structure is actually assessed by XSD.
        recursive = xml().replace(b'</p:Count>', b'</p:Count><p:Next xsi:type="p:Known"><p:Count>2</p:Count></p:Next>')
        case('finite recursion', 'valid-xml-only', lambda: b.validate(recursive))
        resolver = SnapshotResolver({})
        case('resolver direct network denial', 'resolution', lambda: resolver.resolve('https://example.invalid/schema.xsd', None, None))
        case('resolver direct file denial', 'resolution', lambda: resolver.resolve('file:///etc/passwd', None, None))
        # Already compiled validators consume snapshots, not mutable paths.
        p = root / 'facets.xsd'
        original = p.read_bytes()
        p.write_bytes(b'not a schema')
        try:
            case('immutable compiled snapshot', 'valid-xml-only', lambda: b.validate(private))
        finally:
            p.write_bytes(original)
        assert all(url.startswith('file:///task074-approved/') for url in b.resolver.requests)
        RESULTS.append({'case': 'resolver transcript', 'result': b.resolver.requests})


def missing(root, m):
    p = root / 'facets.xsd'
    data = p.read_bytes()
    p.unlink()
    try:
        return Bundle(root, m, research=True)
    finally:
        p.write_bytes(data)


def changed(root, m):
    p = root / 'public.xsd'
    data = p.read_bytes()
    p.write_bytes(data + b'\n')
    try:
        return Bundle(root, m, research=True)
    finally:
        p.write_bytes(data)


def symlink(root, m):
    p = root / 'facets.xsd'
    data = p.read_bytes()
    p.unlink()
    p.symlink_to('/etc/passwd')
    try:
        return Bundle(root, m, research=True)
    finally:
        p.unlink()
        p.write_bytes(data)


def conflict(root, m):
    p = root / 'private.xsd'
    data = p.read_bytes()
    p.write_bytes(data.replace(b'</xs:schema>', b'<xs:complexType name="Known"/></xs:schema>'))
    altered = copy.deepcopy(m)
    for row in altered['files']:
        if row['path'] == p.name:
            row['sha256'] = digest(p.read_bytes())
    try:
        return Bundle(root, altered, research=True)
    finally:
        p.write_bytes(data)


def retrieval(root, location):
    p = root / 'attack.xsd'
    p.write_text(f'<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:include schemaLocation="{location}"/></xs:schema>')
    m = {'root': p.name, 'toolchain': PIN, 'files': [{'path': p.name, 'namespace': '', 'sha256': digest(p.read_bytes())}]}
    return Bundle(root, m, research=True)


def limits():
    # Containment controls tested on hostile workers, not claimed as a bound
    # on all libxml2 algorithms or concurrent deployment workloads.
    def launch(code, cpu=2, memory=256*1024*1024, timeout=5):
        def setup():
            resource.setrlimit(resource.RLIMIT_CPU, (cpu, cpu))
            resource.setrlimit(resource.RLIMIT_AS, (memory, memory))
            resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        try:
            p = subprocess.run([sys.executable, '-c', code], preexec_fn=setup,
                               timeout=timeout, capture_output=True)
            return p.returncode
        except subprocess.TimeoutExpired:
            return 'timeout'
    assert launch('while True: pass', cpu=1) == -signal.SIGKILL
    RESULTS.append({'case': 'CPU hard limit hostile worker', 'result': 'SIGKILL at 1 CPU second'})
    assert launch('try:\n x=bytearray(512*1024*1024)\nexcept MemoryError:\n raise SystemExit(42)') == 42
    RESULTS.append({'case': 'address-space limit hostile worker', 'result': 'MemoryError under 256 MiB'})
    assert launch('import time; time.sleep(10)', timeout=0.1) == 'timeout'
    RESULTS.append({'case': 'wall timeout hostile worker', 'result': 'killed/reaped at 0.1 second'})


def real(path):
    path = Path(path).resolve()
    data = path.read_bytes()
    release = next((v for v,h in ROOT_HASH.items() if digest(data) == h), None)
    assert release, 'authoritative root digest mismatch'
    source = E.fromstring(data)
    t = source.find(X + 'complexType[@name="ManagedListBaseType"]')
    assert t.get('abstract') == 'true' and t.find(X + 'sequence') is None
    fk = source.find(X + 'complexType[@name="ForeignKeyMapML"]')
    assert fk.find('.//' + X + 'extension').get('base') == 'uci:ManagedListBaseType'
    mdt = source.find(X + 'complexType[@name="DataRecordListManagementRequestMDT"]')
    for name in ['AddRecord', 'DeleteRecord']:
        node = mdt.find('.//' + X + f'element[@name="{name}"]')
        assert node.get('type') == 'uci:ManagedListBaseType' and node.get('minOccurs') == '0'
    with tempfile.TemporaryDirectory(prefix='task074-uci-') as tmp:
        root = Path(tmp)
        # Copy complete include closure, never mutate authoritative inputs.
        def cp(p):
            dest = root / p.name
            if dest.exists():
                return
            dest.write_bytes(p.read_bytes())
            for node in E.fromstring(p.read_bytes()):
                if node.tag in (X+'include', X+'import'):
                    cp(p.parent / node.get('schemaLocation'))
        cp(path)
        security = root / f'UCI_SecurityMarkings_v{release.replace(".", "_")}_0.xsd'
        assert digest(security.read_bytes()) == SEC_HASH[release]
        overlay = (FIXTURE / 'uci-overlay.xsd').read_text().replace('PUBLIC_ROOT', path.name)
        (root / 'overlay.xsd').write_text(overlay)
        (root / 'private-fields.xsd').write_bytes((FIXTURE / 'uci-private-fields.xsd').read_bytes())
        approved = ['{' + UCI + '}ForeignKeyMapML', '{'+UCI+'}Task074SyntheticML']
        manifest = make_manifest(root, 'overlay.xsd', release, approved)
        b = Bundle(root, manifest, research=True)
        public = Bundle(root, make_manifest(root, path.name, release, approved), research=True)
        payload = (FIXTURE / 'uci-message.xml').read_bytes().replace(b'RELEASE', ('002.'+release[-1]+'.0').encode())
        case(release+' known ForeignKeyMapML full message', 'valid-xml-only', lambda: public.validate(payload))
        private = payload.replace(b'uci:ForeignKeyMapML"/>', b'uci:Task074SyntheticML"><uci:Task074Code>AB</uci:Task074Code></uci:AddRecord>')
        case(release+' synthetic private full closure', 'valid-xml-only', lambda: b.validate(private))
        case(release+' synthetic absent from public closure', 'invalid:SCHEMAV_CVC_ELT_4_2', lambda: public.validate(private))
        case(release+' inherited request field', 'invalid:SCHEMAV_CVC_ENUMERATION_VALID', lambda: b.validate(private.replace(b'>NEW<', b'>INVALID<')))
        case(release+' extension field', 'invalid:SCHEMAV_CVC_PATTERN_VALID', lambda: b.validate(private.replace(b'>AB<', b'>12<')))
        missing_manifest = copy.deepcopy(manifest)
        missing_manifest['files'] = [r for r in manifest['files'] if r['path'] != 'private-fields.xsd']
        case(release+' incomplete private closure', 'closure', lambda: Bundle(root, missing_manifest, research=True))
        RESULTS.append({'case': release+' source/closure manifest', 'result': manifest})


def worker(args):
    verify_toolchain()
    synthetic()
    limits()
    for path in args.uci:
        real(path)
    output = {'disposition': 'OFFLINE_SCHEMA_VALIDATION_PARTIALLY_QUALIFIED',
              'toolchain': PIN, 'results': RESULTS}
    expected = json.loads((FIXTURE / 'expected-results.json').read_text())
    releases = {row['case'][:3] for row in RESULTS if row['case'].startswith(('2.5 ', '2.6 '))}
    expected['results'] = [row for row in expected['results']
                           if not row['case'].startswith(('2.5 ', '2.6 ')) or row['case'][:3] in releases]
    assert output == expected, 'frozen qualification results changed'
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--uci', action='append', default=[])
    p.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    args = p.parse_args()
    if args.worker:
        worker(args)
    else:
        # All schema compilation/validation happens in a killable bounded worker.
        def bounded():
            resource.setrlimit(resource.RLIMIT_CPU, (30, 30))
            resource.setrlimit(resource.RLIMIT_AS, (768*1024*1024, 768*1024*1024))
            resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()), '--worker', *sys.argv[1:]],
                                preexec_fn=bounded, timeout=60, check=True)
        raise SystemExit(result.returncode)