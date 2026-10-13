#!/usr/bin/env python3
"""Generate synthetic data with the production CLI and hand-authored expectations.

Run: python3 site/compare/qualify.py CLI EMPTY_OUTPUT
This helper is not a published asset.
"""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import urllib.parse

here = pathlib.Path(__file__).resolve().parent
repo = here.parents[1]
spec = importlib.util.spec_from_file_location('packager', here / 'package.py')
packager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packager)
cli = pathlib.Path(sys.argv[1]).resolve()
output = pathlib.Path(sys.argv[2]).resolve()
if output.exists() and any(output.iterdir()):
    raise ValueError('qualification output must be empty')
output.mkdir(parents=True, exist_ok=True)
fixtures = repo / 'tests/fixtures/compare'
tsv = subprocess.check_output([cli, 'schema-diff', '--before', fixtures / 'before.xsd',
                               '--after', fixtures / 'after.xsd', '--format', 'tsv'], text=True)
(output / 'comparison.tsv').write_text(tsv)
for version, side in [('2.5', 'before'), ('2.6', 'after')]:
    subprocess.run([cli, 'docs', '--schema', fixtures / (side + '.xsd'), '--output', output / version], check=True)
graphs = [packager.read_data(output / v / 'assets/explorer-data.js', 'schemaExplorerData') for v in ('2.5', '2.6')]
model = packager.package(tsv, graphs)
expected = {
    ('urn:compare', 'Enum'): [('order', 'ENUM_ORDER_CHANGED'),
                            ('"<script>window.hostile=true</script>雪\\"\\\\\\t\\n"/presence', 'ENUM_VALUE_ADDED')],
    ('urn:compare', 'Gone'): [('presence', 'TYPE_REMOVED')],
    ('urn:compare', 'New'): [('presence', 'TYPE_ADDED')],
    ('urn:compare', 'Pattern'): [('minLength', 'CONSTRAINT_CHANGED'), ('pattern_groups', 'CONSTRAINT_CHANGED')],
    ('urn:compare', 'Record'): [('abstract', 'ABSTRACT_CHANGED'), ('base_type', 'BASE_TYPE_CHANGED'),
        ('"Field"/cardinality', 'MEMBER_CHANGED'), ('"Field"/type_ref', 'MEMBER_CHANGED'),
        ('"GoneField"/presence', 'MEMBER_REMOVED'), ('"NewField"/presence', 'MEMBER_ADDED')],
    ('urn:old', 'Moved'): [('presence', 'TYPE_REMOVED')],
    ('urn:new', 'Moved'): [('presence', 'TYPE_ADDED')],
    ('urn:compare', 'GoneMessage'): [('presence', 'MESSAGE_REMOVED')],
    ('urn:compare', 'NewMessage'): [('presence', 'MESSAGE_ADDED')],
}
actual = {tuple(d['qname']): [(c['property'], c['kind']) for c in d['changes']]
          for d in model['declarations'] if d['changes']}
assert actual == expected, (actual, expected)
assert model['summaries']['Type'] == dict(before=9, after=9, added=2, removed=2, changed=3, unchanged=4)
assert model['summaries']['Message'] == dict(before=2, after=2, added=1, removed=1, changed=0, unchanged=1)
record = next(d for d in model['declarations'] if d['qname'] == ['urn:compare', 'Record'])
assert [(c['before'], c['after']) for c in record['changes'][:4]] == [
    ('true', 'false'), ('named:{urn:compare}Base;binary=None', 'named:{urn:compare}OtherBase;binary=None'),
    ('0..18446744073709551615', '1..unbounded'), ('primitive:Boolean;binary=None', 'primitive:String;binary=None')]
assert all(d['status'] == 'Unchanged' for d in model['declarations'] if d['qname'][1] == 'Same')
# Independent local-index expectations: the removed first member is replaced,
# while both property changes on Field still select local index one.
for change in record['changes']:
    links = change.get('memberDestinations', [None, None])
    if change['kind'] in ('MEMBER_CHANGED', 'MEMBER_ADDED', 'MEMBER_REMOVED'):
        for side, link in enumerate(links):
            absent = (change['kind'] == 'MEMBER_ADDED' and side == 0 or
                      change['kind'] == 'MEMBER_REMOVED' and side == 1)
            assert (link is None) == absent
            if link:
                state = json.loads(urllib.parse.unquote(link.split('#uci=')[1]))
                assert state == dict(v=1, kind='type', root=['urn:compare', 'Record'],
                    path=[dict(owner=['urn:compare', 'Record'], compositor='Record'),
                          dict(member=1 if change['kind'] == 'MEMBER_CHANGED' else 0)])
    else:
        assert 'memberDestinations' not in change
(output / 'compare/assets').mkdir(parents=True)
shutil.copyfile(here / 'index.html', output / 'compare/index.html')
for name in ('compare.css', 'compare.js'):
    shutil.copyfile(here / name, output / 'compare/assets' / name)
(output / 'compare/assets/compare-data.js').write_text('window.schemaCompareData = ' + json.dumps(model, ensure_ascii=True, separators=(',', ':')) + ';\n')
print('Independent synthetic expectations: PASS; 16 property/presence records')

# Additional independently authored Record/Choice, reorder and inherited-owner
# cases. Inputs live only in task-owned qualification output, never publication.
extra = output / 'field-cases'
inputs = output / 'field-inputs'
inputs.mkdir()
for side in ('before', 'after'):
    first = '<xs:element name="Field" type="xs:boolean" minOccurs="0" maxOccurs="unbounded"/>'
    second = '<xs:element name="Other" type="xs:string"/>'
    members = first + second if side == 'before' else second + first.replace('xs:boolean', 'xs:string')
    schema = ('<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:fields" targetNamespace="urn:fields">'
        '<xs:complexType name="Base"><xs:sequence><xs:element name="Field" type="xs:string"/></xs:sequence></xs:complexType>'
        '<xs:complexType name="Derived"><xs:complexContent><xs:extension base="t:Base"><xs:sequence>' + members +
        '</xs:sequence></xs:extension></xs:complexContent></xs:complexType>'
        '<xs:complexType name="Choice"><xs:choice>' + members + '</xs:choice></xs:complexType></xs:schema>')
    (inputs / (side + '.xsd')).write_text(schema)
extra_tsv = subprocess.check_output([cli, 'schema-diff', '--before', inputs / 'before.xsd',
    '--after', inputs / 'after.xsd', '--format', 'tsv'], text=True)
for version, side in [('2.5', 'before'), ('2.6', 'after')]:
    subprocess.run([cli, 'docs', '--schema', inputs / (side + '.xsd'), '--output', extra / version], check=True)
extra_model = packager.package(extra_tsv, [packager.read_data(extra / v / 'assets/explorer-data.js', 'schemaExplorerData') for v in ('2.5', '2.6')])
for declaration in extra_model['declarations']:
    if declaration['qname'][1] == 'Base':
        assert declaration['changes'] == []
        continue
    assert [(c['property'], c['kind']) for c in declaration['changes']] == [
        ('order', 'MEMBER_ORDER_CHANGED'), ('"Field"/type_ref', 'MEMBER_CHANGED')]
    assert 'memberDestinations' not in declaration['changes'][0]
    for side, link in enumerate(declaration['changes'][1]['memberDestinations']):
        state = json.loads(urllib.parse.unquote(link.split('#uci=')[1]))
        assert state['path'] == [dict(owner=declaration['qname'], compositor='Choice' if declaration['qname'][1] == 'Choice' else 'Record'), dict(member=side)]
(extra / 'compare/assets').mkdir(parents=True)
shutil.copyfile(here / 'index.html', extra / 'compare/index.html')
for name in ('compare.css', 'compare.js'):
    shutil.copyfile(here / name, extra / 'compare/assets' / name)
(extra / 'compare/assets/compare-data.js').write_text('window.schemaCompareData = ' + json.dumps(extra_model, ensure_ascii=True, separators=(',', ':')) + ';\n')
print('Independent supplemental field expectations: PASS; Choice and inherited Record indices 0 → 1')