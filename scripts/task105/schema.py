"""Independent structural comparison of the two authentic selected XSD closures.

No generator traversal or generated declarations participate in this oracle.
Annotations are retained separately from executable schema structure.
"""
import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

XS = '{http://www.w3.org/2001/XMLSchema}'
OAM = '{https://www.vdl.afrl.af.mil/programs/oam}'
PINS = {
    '25': ('ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
           '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b'),
    '26': ('af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
           'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88'),
}


def shape(node, metadata=False):
    return [node.tag, {k: v for k, v in sorted(node.attrib.items())
                      if metadata or not k.startswith(OAM)},
            [shape(child, metadata) for child in node if child.tag != XS + 'annotation']]


def closure(schema, release):
    """Resolve named references across the verified two-document closure."""
    names = [f'UCI_{kind}_v2_{release[-1]}_0.xsd'
             for kind in ('MessageDefinitions', 'SecurityMarkings')]
    roots = []
    for name, digest in zip(names, PINS[release], strict=True):
        path = schema.parent / name
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError('authentic schema pin mismatch: ' + name)
        roots.append(ET.parse(path).getroot())
    if ([n.get('schemaLocation') for n in roots[0] if n.tag == XS + 'include'] != [names[1]]
            or any(n.tag == XS + 'import' for n in roots[0])
            or any(n.tag in (XS + 'include', XS + 'import') for n in roots[1])):
        raise ValueError('unexpected authentic schema include/import closure')
    if roots[0].get('version') != f'002.{release[-1]}.0':
        raise ValueError('wrong root release identity')
    declarations = {n.get('name'): n for root in roots for n in root if n.get('name')}
    selected = {}

    def visit(name):
        if name in selected:
            return
        if name not in declarations:
            raise ValueError('unresolved schema reference: ' + name)
        node = declarations[name]
        selected[name] = node
        for child in node.iter():
            for attribute in ('type', 'base', 'ref', 'itemType', 'memberTypes'):
                for reference in child.get(attribute, '').split():
                    if not reference.startswith(('xs:', 'xsd:')):
                        visit(reference.split(':')[-1])

    visit('SurvivabilityRiskLevel')
    if selected['SurvivabilityRiskLevel'].get('type') != 'uci:SurvivabilityRiskLevelMT':
        raise ValueError('wrong global message binding')
    return selected


def compare(uci25, uci26):
    left, right = closure(uci25, '25'), closure(uci26, '26')
    if left.keys() != right.keys() or len(left) != 46:
        raise ValueError('selected named closure changed')
    changed = sorted(name for name in left if shape(left[name]) != shape(right[name]))
    if changed != ['SecurityInformationType', 'WhitespaceVisibleString1024Type',
                   'WhitespaceVisibleString4096Type']:
        raise ValueError('unreviewed selected schema semantic differences')
    return dict(releases=['2.5', '2.6'], named_declarations=46, selected_types=45,
                semantic_changes={name: {'25': shape(left[name]), '26': shape(right[name])}
                                  for name in changed},
                metadata_changes=sorted(name for name in left
                                        if shape(left[name], True) != shape(right[name], True)),
                declarations={r: {name: shape(node, True) for name, node in sorted(values.items())}
                              for r, values in [('25', left), ('26', right)]})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('uci25', 'uci26', 'evidence'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    args.evidence.write_text(json.dumps(compare(args.uci25, args.uci26), indent=2) + '\n')
    print('TASK105_SCHEMA_COMPARISON_COMPLETE types=45 semantic_changes=3')


if __name__ == '__main__':
    main()