#!/usr/bin/env python3
"""Generate synthetic data with the production CLI and hand-authored expectations.

Run: python3 site/compare/qualify.py CLI EMPTY_OUTPUT
This helper is not a published asset.
"""
import importlib.util
import pathlib
import shutil
import subprocess
import sys

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
(output / 'compare/assets').mkdir(parents=True)
shutil.copyfile(here / 'index.html', output / 'compare/index.html')
for name in ('compare.css', 'compare.js'):
    shutil.copyfile(here / name, output / 'compare/assets' / name)
import json
(output / 'compare/assets/compare-data.js').write_text('window.schemaCompareData = ' + json.dumps(model, ensure_ascii=True, separators=(',', ':')) + ';\n')
print('Independent synthetic expectations: PASS; 16 property/presence records')