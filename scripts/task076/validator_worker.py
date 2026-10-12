#!/usr/bin/env python3
"""Limits precede native imports; entire validator phase is supervised externally."""
import resource
resource.setrlimit(resource.RLIMIT_CPU, (30, 30))
resource.setrlimit(resource.RLIMIT_AS, (768 * 1024 * 1024,) * 2)
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
import argparse
import json
from pathlib import Path
from corpus import prepare
from mapper import Mapper
from bundle import Bundle, Rejected, digest, verify_toolchain
from lxml import etree as E


def outcome(action):
    try:
        value = action()
        return value['result'] if isinstance(value, dict) else value
    except Rejected as exc:
        return str(exc)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--scratch', type=Path, required=True)
    p.add_argument('--uci', action='append', required=True)
    p.add_argument('--token', required=True)
    a = p.parse_args()
    verify_toolchain()
    campaign = prepare(a.scratch, a.uci)
    bundles = {k:Bundle(Path(v['root']),v['manifest'],research=True) for k,v in campaign['bundles'].items()}
    results = []
    for row in campaign['cases']:
        b = bundles[row['bundle']]
        raw = Path(row['raw_path']).read_bytes()
        assert digest(raw)==row['raw_sha256']
        mapper = Mapper(b,row['message_qname'])
        mapped = outcome(lambda:mapper.assess(raw))
        assert mapped==row['expected'], (row['id'],mapped,row['expected'])
        reference = None
        if row['reference_path']:
            xml = Path(row['reference_path']).read_bytes()
            reference = outcome(lambda:b.validate(xml))
            assert reference==row['reference_expected'], (row['id'],reference)
            if mapped=='research-schema-valid-not-authorized':
                assert reference=='valid-xml-only'
                # Exact expanded identities, content and xsi:type, not prefix spelling.
                def items(data):
                    def node(n):
                        lexical = n.get('{http://www.w3.org/2001/XMLSchema-instance}type')
                        ty = None
                        if lexical:
                            prefix, local = lexical.split(':')
                            ty = '{'+n.nsmap[prefix]+'}'+local
                        return [n.tag, ty, n.text if not len(n) else None,
                                [node(c) for c in n if isinstance(c.tag,str)]]
                    return node(E.fromstring(data))
                assert items(mapper.map(raw)['xml'])==items(xml), row['id']
        results.append({'id':row['id'],'mapper':mapped,'reference':reference})
    artifact = dict(token=a.token, campaign=campaign, results=results, validator=verify_toolchain())
    (a.scratch/'validator.json').write_text(json.dumps(artifact, sort_keys=True))
    print('TASK076 VALIDATOR: COMPLETE')


if __name__ == '__main__':
    main()
