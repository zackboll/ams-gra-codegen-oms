#!/usr/bin/env python3
"""Opt-in pinned end-to-end campaign. No missing-input success or authority."""
import argparse
import copy
import json
import resource
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from mapper import Mapper, Failure, OAM, raw_parse, LIMITS
from bundle import Bundle, Rejected, PIN, X, I, digest, make_manifest, verify_toolchain
from unittest.mock import patch
from lxml import etree as E

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'task074'))
from qualify import ROOT_HASH, SEC_HASH

REPO = Path(__file__).resolve().parents[2]
FIX = REPO / 'tests/fixtures/task075'
OLD = REPO / 'tests/fixtures/task074'
RESULTS = []
ARTIFACTS = {}


def case(name, expected, action):
    try:
        result = action()
        if isinstance(result, dict):
            ARTIFACTS[name] = {k: v.decode('utf-8') if isinstance(v, bytes) else v
                               for k, v in result.items()}
            result = result['result']
    except Rejected as exc:
        result = str(exc)
    assert result == expected, (name, expected, result)
    RESULTS.append({'case': name, 'result': result})


def encode(value):
    return json.dumps(value, ensure_ascii=True).encode()


def items(xml):
    """Prefix-independent, whitespace-insensitive for element-only content."""
    root = E.fromstring(xml)
    def node(n):
        lexical = n.get(I+'type')
        ty = None
        if lexical:
            prefix, local = lexical.split(':')
            ty = '{'+n.nsmap[prefix]+'}'+local
        return [n.tag, ty, n.text if not len(n) else None, [node(c) for c in n if isinstance(c.tag, str)]]
    return node(root)


def synthetic():
    m = json.loads((OLD/'manifest.json').read_text())
    bundle = Bundle(OLD, m, research=True)
    mapper = Mapper(bundle, '{urn:task074:public}Value')
    known = b'{"{urn:task074:public}Value":{"$type":"{urn:task074:public}Known","{urn:task074:public}Count":1}}'
    private = b'{"{urn:task074:public}Value":{"$type":"{urn:task074:private}Known","{urn:task074:public}Count":1,"{urn:task074:private}Secret":"AB"}}'
    good = 'research-schema-valid-not-authorized'
    case('known', good, lambda: mapper.assess(known))
    case('same local private identity', good, lambda: mapper.assess(private))
    case('defined unapproved type', 'assessment-policy:unapproved-type', lambda: mapper.assess(
        known.replace(b'{urn:task074:public}Known', b'{urn:task074:private}Unapproved')))
    # Independent XML reference, not produced with mapper under test.
    expected = b'<p:Value xmlns:p="urn:task074:public" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="p:Known"><p:Count>1</p:Count></p:Value>'
    assert items(mapper.map(known)['xml']) == items(expected)
    negatives = [
        ('syntax', b'{', 'syntax:json'),
        ('utf8', b'\xff', 'syntax:utf8'),
        ('duplicate escaped type', known.replace(b'"$type":', b'"\\u0024type":"x","$type":'), 'policy:duplicate-name'),
        ('nested duplicate', known.replace(b'Count":1', b'Count":1,"{urn:task074:public}Count":2'), 'policy:duplicate-name'),
        ('multiple envelope', b'{"A":{},"B":{}}', 'envelope:one-member-object'),
        ('wrong envelope', b'{"Value":{}}', 'envelope:wrong-global'),
        ('nonobject envelope', b'[]', 'envelope:one-member-object'),
        ('missing type', known.replace(b'"$type":"{urn:task074:public}Known",', b''), 'mapping:missing-type'),
        ('abstract', known.replace(b'}Known', b'}Base'), 'mapping:abstract-type'),
        ('unrelated', known.replace(b'}Known', b'}Other'), 'mapping:unrelated-type'),
        ('unresolved', known.replace(b'}Known', b'}Absent'), 'mapping:unresolved-type'),
        ('malformed type', known.replace(b'}Known', b'}bad:name'), 'unsupported:non-profile-qname'),
        ('type number', known.replace(b'"{urn:task074:public}Known"', b'1'), 'shape:qname-token'),
        ('wrong namespace type', known.replace(b'{urn:task074:public}Known', b'{urn:wrong}Known'), 'mapping:unresolved-type'),
        ('brace malformed', known.replace(b'{urn:task074:public}Known', b'{unclosed'), 'shape:qname'),
        ('number string', known.replace(b'Count":1', b'Count":"1"'), 'shape:number-token'),
        ('number boolean', known.replace(b'Count":1', b'Count":true'), 'shape:number-token'),
        ('null', known.replace(b'Count":1', b'Count":null'), 'shape:null'),
        ('missing member', known.replace(b',"{urn:task074:public}Count":1', b''), 'shape:missing-member'),
        ('unknown member', known.replace(b'Count":1', b'Count":1,"Extra":2'), 'shape:unknown-member'),
        ('inherited facet', known.replace(b'Count":1', b'Count":0'), 'xsd:invalid:SCHEMAV_CVC_DATATYPE_VALID_1_2_1'),
        ('private facet', private.replace(b'"AB"', b'"12"'), 'xsd:invalid:SCHEMAV_CVC_PATTERN_VALID'),
        ('recursive value', known.replace(b'Count":1', b'Count":1,"{urn:task074:public}Next":{}'), 'unsupported:recursive-value'),
    ]
    for name, raw, reason in negatives:
        case(name, reason, lambda raw=raw: mapper.assess(raw))
    large = known.replace(b'Count":1', b'Count":9007199254740993')
    case('large inherited integer', good, lambda: mapper.assess(large))
    assert E.fromstring(mapper.map(large)['xml'])[0].text == '9007199254740993'
    case('default authority', 'authority-not-established', lambda: Bundle(OLD, m))
    case('toolchain mismatch', 'toolchain-manifest', lambda: Bundle(OLD, {**m, 'toolchain': {}}, research=True))
    with patch('bundle.E.LXML_VERSION', (0,0,0,0)):
        case('binary version mismatch', 'toolchain-pin', lambda: Bundle(OLD, m, research=True))
    case('missing closure', 'closure', lambda: Bundle(OLD, {**m, 'files': m['files'][:-1]}, research=True))
    with tempfile.TemporaryDirectory(prefix='task075-bundle-') as tmp:
        root = Path(tmp)
        shutil.copytree(OLD, root, dirs_exist_ok=True)
        (root/'public.xsd').write_bytes(b'altered')
        case('altered closure', 'digest', lambda: Bundle(root, m, research=True))
        (root/'public.xsd').write_bytes((OLD/'public.xsd').read_bytes())
        (root/'private.xsd').unlink()
        case('missing file', 'missing-file', lambda: Bundle(root, m, research=True))
    vm = make_manifest(FIX, 'values.xsd', 'synthetic', [])
    vb = Bundle(FIX, vm, research=True)
    v = Mapper(vb, '{urn:task075}Values')
    raw = b'{"{urn:task075}Values":{"{urn:task075}Text":"<&\\r\\n\\t\\\"\\u00e9\\udbff\\udfff","{urn:task075}Integer":9007199254740993,"{urn:task075}Decimal":-0.1250,"{urn:task075}Flag":true,"{urn:task075}Entry":["second","first"]}}'
    case('exact values and preservation', good, lambda: v.assess(raw))
    xml = E.fromstring(v.map(raw)['xml'])
    assert [n.tag for n in xml] == ['{urn:task075}'+k for k in ['Text','Integer','Decimal','Flag','Entry','Entry']]
    assert [n.text for n in xml] == ['<&\r\n\t"é\U0010ffff', '9007199254740993', '-0.1250', 'true', 'second', 'first']
    obj = json.loads(raw)
    body = obj['{urn:task075}Values']
    for rows in [[], ['one'], ['two','one']]:
        body['{urn:task075}Entry'] = rows
        case('array '+str(len(rows)), good, lambda: v.assess(encode(obj)))
    body.pop('{urn:task075}Entry')
    case('optional absent', good, lambda: v.assess(encode(obj)))
    body['{urn:task075}Text'] = ''
    case('empty string', good, lambda: v.assess(encode(obj)))
    changed = copy.deepcopy(obj)
    changed['{urn:task075}Values']['{urn:task075}Flag'] = False
    changed['{urn:task075}Values']['{urn:task075}Integer'] = -9007199254740993
    case('false and negative exact integer', good, lambda: v.assess(encode(changed)))
    reversed_obj = {'{urn:task075}Values': dict(reversed(list(body.items())))}
    assert items(v.map(encode(obj))['xml']) == items(v.map(encode(reversed_obj))['xml'])
    case('permuted members', good, lambda: v.assess(encode(reversed_obj)))
    changes = [
        ('Entry', 'one', 'shape:array-required'), ('Text', [], 'shape:scalar-required'),
        ('Text', {}, 'shape:string-token'), ('Text', 1, 'shape:string-token'),
        ('Text', None, 'shape:null'), ('Flag', 1, 'shape:boolean-token'),
        ('Flag', 'true', 'shape:boolean-token'), ('Text', '\x00', 'mapping:xml-character'),
        ('Text', '\ud800', 'mapping:xml-character'),
    ]
    for index, (key, value, reason) in enumerate(changes):
        changed = copy.deepcopy(obj)
        changed['{urn:task075}Values']['{urn:task075}'+key] = value
        case('token/shape '+str(index), reason, lambda changed=changed: v.assess(encode(changed)))
    case('exponent', 'unsupported:decimal-exponent', lambda: v.assess(raw.replace(b'-0.1250', b'1e2')))
    case('fraction integer lexical', 'xsd:invalid:SCHEMAV_CVC_DATATYPE_VALID_1_2_1', lambda: v.assess(raw.replace(b'9007199254740993', b'1.0')))
    for key in ['A', 'B']:
        changed = copy.deepcopy(obj)
        changed['{urn:task075}Values']['{urn:task075}'+key] = key
        case('choice '+key, good, lambda: v.assess(encode(changed)))
    changed['{urn:task075}Values']['{urn:task075}A'] = 'a'
    case('choice conflict', 'shape:choice-conflict', lambda: v.assess(encode(changed)))
    case('float unsupported', 'unsupported:simple-primitive', lambda: Mapper(vb, '{urn:task075}Float').assess(b'{"{urn:task075}Float":{"{urn:task075}Value":1}}'))
    # QName expansion ambiguity is distinct from raw duplicate-name policy.
    changed = copy.deepcopy(obj)
    changed['{urn:task075}Values']['Alias'] = 1
    changed['{urn:task075}Values']['{'+OAM+'}Alias'] = 2
    case('expanded duplicate', 'policy:duplicate-expanded-qname', lambda: v.assess(encode(changed)))
    for label, data, limits, reason in [
        ('bytes', raw, {'bytes':10}, 'byte-limit'),
        ('depth', b'[[[[]]]]', {'depth':3}, 'depth-limit'),
        ('members', raw, {'members':2}, 'member-limit'),
        ('array', raw, {'array':1}, 'array-limit'),
        ('number', raw, {'number':4}, 'number-limit'),
        ('output', raw, {'output':30}, 'output-limit'),
    ]:
        case('resource '+label, 'policy:'+reason, lambda data=data, limits=limits: Mapper(vb, '{urn:task075}Values', limits=limits).assess(data))
    case('non-JSON NaN', 'syntax:non-json-number', lambda: raw_parse(b'NaN', LIMITS))
    for label, data, limit in [
        ('bytes', b'{}', 2), ('depth', b'[[]]', 2),
        ('members', b'{"a":0}', 1), ('array', b'[0]', 1), ('number', b'123', 3),
    ]:
        raw_parse(data, {**LIMITS, label:limit})
        RESULTS.append({'case':'boundary '+label, 'result':'policy-boundary-accepted'})
    # Same known closed document as generated_codec_qname's executable control.
    schema = REPO/'tests/fixtures/service-generate'
    rb = Bundle(schema, make_manifest(schema, 'runtime-test.xsd', 'codec-control', []), research=True)
    rm = Mapper(rb, '{urn:test}MessageB')
    case('generated-codec closed vector', good, lambda: rm.assess(b'{"{urn:test}MessageB":{"{urn:test}Count":7}}'))
    assert items(rm.map(b'{"{urn:test}MessageB":{"{urn:test}Count":7}}')['xml']) == items(b'<p:MessageB xmlns:p="urn:test"><p:Count>7</p:Count></p:MessageB>')
    with tempfile.TemporaryDirectory(prefix='task075-codec-overlay-') as tmp:
        root = Path(tmp)
        fixtures = REPO/'tests/fixtures/open-world/task068-private'
        for name in ['public.xsd', 'private.xsd']:
            (root/name).write_bytes((fixtures/name).read_bytes())
        (root/'root.xsd').write_text('<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:audit"><xs:include schemaLocation="public.xsd"/><xs:include schemaLocation="private.xsd"/></xs:schema>')
        pb = Bundle(root, make_manifest(root, 'root.xsd', 'private-codec-control', ['{urn:audit}PrivateB']), research=True)
        mapper = Mapper(pb, '{urn:audit}HolderReport')
        raw = b'{"{urn:audit}HolderReport":{"{urn:audit}Value":{"$type":"{urn:audit}PrivateB","{urn:audit}Flag":true,"{urn:audit}Enabled":false}}}'
        case('generated-codec private overlay vector', good, lambda: mapper.assess(raw))
        expected = b'<p:HolderReport xmlns:p="urn:audit" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><p:Value xsi:type="p:PrivateB"><p:Flag>true</p:Flag><p:Enabled>false</p:Enabled></p:Value></p:HolderReport>'
        assert items(mapper.map(raw)['xml']) == items(expected)


def review_regressions():
    good = 'research-schema-valid-not-authorized'
    namespace = 'urn:task075:review'
    bundles = {}
    for variant in ['base', 'unused', 'metadata']:
        manifest = make_manifest(FIX, 'review-'+variant+'.xsd', 'review', [])
        bundles[variant] = Bundle(FIX, manifest, research=True)
        ARTIFACTS['review '+variant+' bundle'] = {'manifest': manifest, 'result':'compiled'}
        RESULTS.append({'case':'review '+variant+' bundle', 'result':'compiled'})

    def probe(variant, name, body, expected=good, reference_valid=True):
        bundle = bundles[variant]
        raw = encode({'{'+namespace+'}'+name: {'{'+namespace+'}'+k:v for k,v in body.items()}})
        # Independent reference XML: fixed hand-reviewed order, not mapper output.
        root = E.Element('{'+namespace+'}'+name)
        for key in ['Text', 'Extra', 'A', 'B']:
            if key in body:
                E.SubElement(root, '{'+namespace+'}'+key).text = body[key]
        reference = E.tostring(root)
        label = 'review '+variant+' '+name+' '+('empty' if not body else '+'.join(body))
        case(label+' reference', 'valid-xml-only' if reference_valid else 'invalid:SCHEMAV_ELEMENT_CONTENT',
             lambda: bundle.validate(reference))
        ARTIFACTS[label+' reference'] = {'xml':reference.decode(), 'raw':raw.decode()}
        mapper = Mapper(bundle, '{'+namespace+'}'+name)
        case(label, expected, lambda: mapper.assess(raw))
        if expected == good:
            assert items(mapper.map(raw)['xml']) == items(reference)
            return mapper.map(raw)['xml']

    baseline = probe('base', 'Record', {'Text':'ok','Extra':'more'})
    for variant in ['unused', 'metadata']:
        assert items(probe(variant, 'Record', {'Text':'ok','Extra':'more'})) == items(baseline)
    # Keep no-namespace declarations indexed and distinct, not discarded/aliased.
    mapper = Mapper(bundles['unused'], '{'+namespace+'}Record')
    assert '{}Record' in mapper.types and '{}Record' in mapper.elements
    assert '{'+namespace+'}Record' in mapper.types
    case('review selected no namespace input', 'shape:qname',
         lambda: mapper.assess(b'{"{}Record":{"Wrong":1}}'))
    for variant in ['base','metadata']:
        probe(variant, 'Base', {'Text':'ok'})
        for name in ['Nullable', 'Nested', 'Optional']:
            probe(variant, name, {})
        probe(variant, 'Required', {}, 'shape:choice-missing', reference_valid=False)
        for name in ['Nullable','Required','Nested']:
            probe(variant, name, {'A':'a'})
            probe(variant, name, {'B':'b'})
            probe(variant, name, {'A':'a','B':'b'}, 'shape:choice-conflict', reference_valid=False)
        probe(variant, 'Unsupported', {}, 'unsupported:particle')


def real(path):
    path = Path(path).resolve()
    release = next((r for r, h in ROOT_HASH.items() if digest(path.read_bytes()) == h), None)
    assert release, 'authoritative root digest mismatch'
    with tempfile.TemporaryDirectory(prefix='task075-uci-') as tmp:
        root = Path(tmp)
        def cp(p):
            if (root/p.name).exists():
                return
            data = p.read_bytes()
            (root/p.name).write_bytes(data)
            for n in E.fromstring(data):
                if n.tag in (X+'include', X+'import'):
                    cp(p.parent/n.get('schemaLocation'))
        cp(path)
        assert digest((root/f'UCI_SecurityMarkings_v{release.replace(".", "_")}_0.xsd').read_bytes()) == SEC_HASH[release]
        (root/'overlay.xsd').write_text((OLD/'uci-overlay.xsd').read_text().replace('PUBLIC_ROOT', path.name))
        (root/'private-fields.xsd').write_bytes((OLD/'uci-private-fields.xsd').read_bytes())
        approved = ['{'+OAM+'}'+n for n in ['ForeignKeyMapML','Task074SyntheticML']]
        pm = make_manifest(root, path.name, release, approved)
        sm = make_manifest(root, 'overlay.xsd', release, approved)
        public = Mapper(Bundle(root, pm, research=True), '{'+OAM+'}DataRecordListManagementRequest')
        private = Mapper(Bundle(root, sm, research=True), public.expected)
        obj = json.loads((FIX/'message.json').read_bytes())
        body = obj['DataRecordListManagementRequest']
        body['MessageHeader']['SchemaVersion'] = '002.'+release[-1]+'.0'
        good = 'research-schema-valid-not-authorized'
        def probe(name, expected, action):
            case(release+' '+name, expected, action)
        probe('known complete', good, lambda: public.assess(encode(obj)))
        reference = (OLD/'uci-message.xml').read_bytes().replace(b'RELEASE', body['MessageHeader']['SchemaVersion'].encode())
        assert items(public.map(encode(obj))['xml']) == items(reference)
        permuted = {'DataRecordListManagementRequest': dict(reversed(list(body.items())))}
        probe('complete permuted', good, lambda: public.assess(encode(permuted)))
        assert items(public.map(encode(permuted))['xml']) == items(reference)
        body['SecurityInformation']['OwnerProducer'][0]['NATO_SpecialWord'] = 'NATO'
        probe('security choice conflict', 'shape:choice-conflict', lambda: public.assess(encode(obj)))
        del body['SecurityInformation']['OwnerProducer'][0]['NATO_SpecialWord']
        # Execute the real security choice conflict and nonempty repeated DRLEs.
        data = body['MessageData']
        entries = [{'ForeignKeyPairID': {'UUID': uid}} for uid in [
            '697eba54-6f37-11ed-a1eb-0242ac120002', '539a5e3a-1c72-4c19-83c7-0024c0a15d8c']]
        for rows in [[], entries[:1], entries]:
            data['AddRecord']['ForeignKeyPairListElement'] = rows
            probe('real repeated '+str(len(rows)), good, lambda: public.assess(encode(obj)))
            mapped = E.fromstring(public.map(encode(obj))['xml'])
            uuids = mapped.findall('.//{'+OAM+'}ForeignKeyPairID/{'+OAM+'}UUID')
            assert [n.text for n in uuids] == [e['ForeignKeyPairID']['UUID'] for e in rows]
        data['DeleteRecord'] = data.pop('AddRecord')
        probe('known DeleteRecord', good, lambda: public.assess(encode(obj)))
        data['DeleteRecord'] = {'$type':'ForeignKeyMapML'}
        probe('empty concrete managed list', good, lambda: public.assess(encode(obj)))
        data['AddRecord'] = {'$type':'Task074SyntheticML', 'Task074Code':'AB'}
        probe('private complete', good, lambda: private.assess(encode(obj)))
        probe('private public-only', 'mapping:unresolved-type', lambda: public.assess(encode(obj)))
        missing = copy.deepcopy(sm)
        missing['files'] = [r for r in missing['files'] if r['path'] != 'private-fields.xsd']
        probe('private incomplete closure', 'closure', lambda: Bundle(root, missing, research=True))
        data['RequestState'] = 'INVALID'
        probe('inherited RequestState', 'xsd:invalid:SCHEMAV_CVC_ENUMERATION_VALID', lambda: private.assess(encode(obj)))
        data['RequestState'] = 'NEW'
        data['AddRecord']['Task074Code'] = '12'
        probe('private Task074Code', 'xsd:invalid:SCHEMAV_CVC_PATTERN_VALID', lambda: private.assess(encode(obj)))
        data['AddRecord']['Task074Code'] = 'AB'
        data['DeleteRecord'] = data.pop('AddRecord')
        probe('private DeleteRecord', good, lambda: private.assess(encode(obj)))
        ARTIFACTS[release+' manifests'] = {'public': pm, 'private': sm}


def run_worker(command, timeout=60):
    def bounded():
        resource.setrlimit(resource.RLIMIT_CPU, (30,30))
        resource.setrlimit(resource.RLIMIT_AS, (768*1024*1024,)*2)
        resource.setrlimit(resource.RLIMIT_CORE, (0,0))
    try:
        result = subprocess.run(command, preexec_fn=bounded, timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        raise Failure('resource', 'worker-timeout')
    if result.returncode:
        raise Failure('resource', 'worker-failure')


def worker(args):
    verify_toolchain()
    synthetic()
    review_regressions()
    if args.real:
        assert len(args.uci) == 2, '--real requires exactly both pinned UCI roots'
        assert {digest(Path(p).read_bytes()) for p in args.uci} == set(ROOT_HASH.values()), 'both releases required'
    for path in args.uci:
        real(path)
    case('worker failure', 'resource:worker-failure', lambda: run_worker([sys.executable,'-c','raise SystemExit(3)']))
    case('worker timeout', 'resource:worker-timeout', lambda: run_worker([sys.executable,'-c','import time; time.sleep(5)'], timeout=0.05))
    output = {'disposition':'OMS_JSON_XSD_MAPPING_PARTIALLY_QUALIFIED',
              'toolchain':PIN, 'limits':LIMITS, 'results':RESULTS, 'evidence':ARTIFACTS,
              'source_sha256':{str(p.relative_to(REPO)):digest(p.read_bytes()) for p in [Path(__file__), Path(__file__).with_name('mapper.py'), REPO/'scripts/task074/bundle.py']}}
    frozen = FIX/'expected-results.json'
    if frozen.exists():
        expected = json.loads(frozen.read_bytes())
        releases = {row['case'][:3] for row in RESULTS if row['case'].startswith(('2.5 ', '2.6 '))}
        expected = [r for r in expected if not r['case'].startswith(('2.5 ', '2.6 ')) or r['case'][:3] in releases]
        assert RESULTS == expected, 'frozen rejection-stage corpus changed'
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--real', action='store_true', help='require both pinned releases, never skip')
    parser.add_argument('--uci', action='append', default=[])
    parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        worker(args)
    else:
        run_worker([sys.executable, str(Path(__file__).resolve()), '--worker', *sys.argv[1:]])