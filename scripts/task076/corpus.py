"""Versioned bounded fixtures; XML references are specified independently of Mapper."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'task075'))
from mapper import OAM
from bundle import X, digest, make_manifest
from lxml import etree as E

REPO = Path(__file__).resolve().parents[2]
FIX = REPO / 'tests/fixtures/task076'


def prepare(scratch, uci):
    rows, bundles = [], {}
    def bundle(name, root, filename, approved):
        manifest = make_manifest(root, filename, name, approved)
        manifest['bundle_id'] = 'task076-' + name
        bundles[name] = {'root': str(root), 'manifest': manifest}
    def case(cid, group, message, declared, raw, xml, expected):
        path = FIX / (cid + '.json')
        assert path.read_bytes() == raw, ('raw fixture drift', cid)
        reference = FIX / (cid + '.xml')
        assert (reference.read_bytes() if reference.exists() else None) == xml
        obj = json.loads(raw)
        rows.append(dict(id=cid, bundle=group, message_qname=message,
                         declared_type=declared, asserted_type=obj[next(iter(obj))].get('$type'),
                         raw_path=str(path), raw_sha256=digest(raw),
                         reference_path=str(reference) if xml else None,
                         reference_sha256=digest(xml) if xml else None, expected=expected))
    root = FIX / 'schemas'
    approved = ['{'+OAM+'}'+n for n in ['Concrete', 'DerivedConcrete', 'Known', 'Private', 'Other']]
    approved += ['{urn:task076:private}Known']
    bundle('public', root, 'public.xsd', approved)
    bundle('private', root, 'private.xsd', approved)
    for spec in json.loads((FIX/'synthetic.json').read_bytes()):
        cid = spec['id']
        raw = (FIX/(cid+'.json')).read_bytes()
        xp = FIX/(cid+'.xml')
        case(cid, spec['bundle'], '{'+OAM+'}'+spec['element'],
             '{'+OAM+'}'+spec['declared'], raw, xp.read_bytes() if xp.exists() else None,
             spec['expected'])
    hashes = {'2.5':'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
              '2.6':'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
    assert {digest(Path(p).read_bytes()) for p in uci}==set(hashes.values()), 'both pinned releases required'
    for path in uci:
        source = Path(path).resolve()
        release = next(r for r,h in hashes.items() if digest(source.read_bytes()) == h)
        dest = scratch / ('uci-'+release)
        dest.mkdir(exist_ok=True)
        def cp(p):
            if (dest/p.name).exists():
                assert (dest/p.name).read_bytes() == p.read_bytes()
                return
            (dest/p.name).write_bytes(p.read_bytes())
            for n in E.fromstring(p.read_bytes()):
                if n.tag in (X+'include', X+'import'):
                    cp(p.parent/n.get('schemaLocation'))
        cp(source)
        overlay = (REPO/'tests/fixtures/task074/uci-overlay.xsd').read_bytes().replace(b'PUBLIC_ROOT', source.name.encode())
        (dest/'overlay.xsd').write_bytes(overlay)
        (dest/'private-fields.xsd').write_bytes((REPO/'tests/fixtures/task074/uci-private-fields.xsd').read_bytes())
        approved = ['{'+OAM+'}'+n for n in ['ForeignKeyMapML','Task074SyntheticML']]
        bundle(release+'-public', dest, source.name, approved)
        bundle(release+'-private', dest, 'overlay.xsd', approved)
        for name in ['known-add','known-delete','private-add','private-delete','private-public','bad-state','bad-code']:
            cid = 'uci-'+release+'-'+name
            raw = (FIX/(cid+'.json')).read_bytes()
            xml = (FIX/(cid+'.xml')).read_bytes()
            group = release+('-public' if name.startswith('known') or name=='private-public' else '-private')
            expected = ('mapping:unresolved-type' if name=='private-public' else
                        'xsd:invalid:SCHEMAV_CVC_ENUMERATION_VALID' if name=='bad-state' else
                        'xsd:invalid:SCHEMAV_CVC_PATTERN_VALID' if name=='bad-code' else
                        'research-schema-valid-not-authorized')
            case(cid, group, '{'+OAM+'}DataRecordListManagementRequest',
                 '{'+OAM+'}DataRecordListManagementRequestMT', raw, xml, expected)
            rows[-1]['asserted_type'] = json.loads(raw)['DataRecordListManagementRequest']['MessageData'].get('AddRecord',json.loads(raw)['DataRecordListManagementRequest']['MessageData'].get('DeleteRecord'))['$type']
            rows[-1]['declared_position_type'] = '{'+OAM+'}ManagedListBaseType'
            rows[-1]['discriminator_position'] = 'MessageData.'+('DeleteRecord' if name.endswith('delete') else 'AddRecord')
    for row in rows:
        token = row['asserted_type']
        if row['id'] in ('duplicate','duplicate-escaped'):
            row['discriminator_tokens_in_order'] = ['Other','Known']
        row['asserted_concrete_qname'] = (token if token.startswith('{') else '{'+OAM+'}'+token) if isinstance(token,str) and token!='{bad' else None
        manifest = bundles[row['bundle']]['manifest']
        row['bundle_sha256'] = digest(json.dumps(manifest,sort_keys=True,separators=(',',':')).encode())
        references = {
            'private-public':'invalid:SCHEMAV_CVC_ELT_4_2',
            'missing':'invalid:SCHEMAV_CVC_TYPE_2',
            'unknown':'invalid:SCHEMAV_CVC_ELT_4_2',
            'abstract':'invalid:SCHEMAV_CVC_TYPE_2',
            'unrelated':'invalid:SCHEMAV_CVC_ELT_4_3',
            'bad-inherited':'invalid:SCHEMAV_CVC_DATATYPE_VALID_1_2_1',
            'bad-private':'invalid:SCHEMAV_CVC_PATTERN_VALID',
            'unexpected':'invalid:SCHEMAV_ELEMENT_CONTENT',
            'blocked':'invalid:SCHEMAV_CVC_ELT_4_3',
            'erased-diagnostic':'invalid:SCHEMAV_CVC_TYPE_2',
        }
        cid = row['id']
        if cid.startswith('uci-'):
            row['reference_expected'] = ('invalid:SCHEMAV_CVC_ELT_4_2' if cid.endswith('private-public') else
                'invalid:SCHEMAV_CVC_ENUMERATION_VALID' if cid.endswith('bad-state') else
                'invalid:SCHEMAV_CVC_PATTERN_VALID' if cid.endswith('bad-code') else 'valid-xml-only')
        else:
            row['reference_expected'] = references.get(cid,'valid-xml-only') if row['reference_path'] else None
    return {'version':1, 'cases':rows, 'bundles':bundles}