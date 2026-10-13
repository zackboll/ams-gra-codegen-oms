"""Small adversarial checks for the standalone Pages validator."""

import importlib.util
import contextlib
import io
import json
import pathlib
import tempfile
import unittest
import warnings
import zipfile
import urllib.parse


spec = importlib.util.spec_from_file_location(
    'pages', pathlib.Path(__file__).with_name('validate-uci-pages.py'))
pages = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pages)


class MemberDestinationTests(unittest.TestCase):
    def test_browser_hash_length_boundary(self):
        for name_length, index, hash_length in [(4084, 0, 16383), (4084, 10, 16384), (4085, 0, 16385)]:
            with self.subTest(hash_length=hash_length), tempfile.TemporaryDirectory() as tmp:
                q = ['n' * 4004, 'T' * name_length]
                segment = dict(owner=q, compositor='Record')
                d = dict(qname=q, kind='Record', url='types/000000.html', segments=[segment],
                    members=[dict(index=i, name='Field' if i == index else 'Other' + str(i), type={'primitive': 'String'}) for i in range(index + 1)])
                indexes = ({}, {tuple(q): d}, {})
                state = dict(v=1, kind='type', root=q, path=[segment, dict(member=index)])
                fragment = 'uci=' + urllib.parse.quote(json.dumps(state, separators=(',', ':')), safe='')
                self.assertEqual(len('#' + fragment), hash_length)
                # Generic historical validation accepts this valid-shaped state,
                # proving the first rejected hash previously passed validation.
                pages.explorer_state(fragment, indexes)
                c = dict(kind='MEMBER_CHANGED', property='"Field"/type_ref', before='old', after='new')
                root = pathlib.Path(tmp)
                (root / 'compare/assets').mkdir(parents=True)
                (root / 'compare/index.html').write_text('')
                for asset in ('compare.js', 'compare.css'):
                    (root / 'compare/assets' / asset).write_text('')
                explorers = {}
                for version in ('2.5', '2.6'):
                    (root / version / 'assets').mkdir(parents=True)
                    graph = dict(schema=dict(version=None), types=[d], messages=[])
                    (root / version / 'assets/explorer-data.js').write_text('window.schemaExplorerData = ' + json.dumps(graph) + ';\n')
                    explorers[root / version / 'explorer.html'] = indexes
                model = dict(version=1, scope='Direct normalized SchemaIr changes — not a compatibility verdict.',
                    releases=[dict(release=v, revision=pin, schemaVersion='absent') for v, pin in zip(('2.5', '2.6'), pages.PINS)],
                    summaries=dict(Type=dict(before=1, after=1, added=0, removed=0, changed=1, unchanged=0), Message=dict(before=0, after=0, added=0, removed=0, changed=0, unchanged=0)),
                    declarations=[dict(category='Type', qname=q, status='Changed', changes=[c], destinations=[dict(reference='../' + v + '/' + d['url'], explorer='../' + v + '/explorer.html#uci=' + urllib.parse.quote(json.dumps(dict(state, path=[])), safe='')) for v in ('2.5', '2.6')])])
                def validate():
                    (root / 'compare/assets/compare-data.js').write_text('window.schemaCompareData = ' + json.dumps(model) + ';\n')
                    pages.comparison(root, explorers)
                validate()  # Historical record without the optional property.
                c['memberDestinations'] = ['../' + v + '/explorer.html#' + fragment for v in ('2.5', '2.6')]
                if hash_length > 16384:
                    with self.assertRaisesRegex(ValueError, 'oversized member destination hash'):
                        validate()
                else:
                    validate()

    def test_optional_contract_and_independent_escaping(self):
        q = ['urn:a', 'Choice']
        segment = dict(owner=q, compositor='Choice')
        name = 'quote"\\</script>'
        d = dict(qname=q, kind='Choice', segments=[segment], members=[
            dict(index=0, name=name, type={'primitive': 'String'})], url='types/000000.html')
        indexes = ({}, {tuple(q): d}, {})
        state = dict(v=1, kind='type', root=q, path=[segment, dict(member=0)])
        link = '../2.6/explorer.html#uci=' + urllib.parse.quote(json.dumps(state))
        record = dict(kind='MEMBER_ADDED', property='"quote\\"\\\\</script>"/presence', before=None, after='true')
        pages.member_destination(link, '2.6', q, record, 1, indexes)
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / 'compare/assets').mkdir(parents=True)
            for asset in ('compare.js', 'compare.css'):
                (root / 'compare/assets' / asset).write_text('')
            (root / 'compare/index.html').write_text('')
            explorers = {}
            for version in ('2.5', '2.6'):
                (root / version / 'assets').mkdir(parents=True)
                graph = dict(schema=dict(version=None), types=[d], messages=[])
                (root / version / 'assets/explorer-data.js').write_text('window.schemaExplorerData = ' + json.dumps(graph) + ';\n')
                explorers[root / version / 'explorer.html'] = indexes
            model = dict(version=1, scope='Direct normalized SchemaIr changes — not a compatibility verdict.',
                releases=[dict(release=v, revision=pin, schemaVersion='absent') for v, pin in zip(('2.5', '2.6'), pages.PINS)],
                summaries=dict(Type=dict(before=1, after=1, added=0, removed=0, changed=1, unchanged=0),
                               Message=dict(before=0, after=0, added=0, removed=0, changed=0, unchanged=0)),
                declarations=[dict(category='Type', qname=q, status='Changed', changes=[record], destinations=[
                    dict(reference='../' + v + '/types/000000.html', explorer='../' + v + '/explorer.html#uci=' +
                         urllib.parse.quote(json.dumps(dict(state, path=[])))) for v in ('2.5', '2.6')])])
            def validate():
                (root / 'compare/assets/compare-data.js').write_text('window.schemaCompareData = ' + json.dumps(model) + ';\n')
                pages.comparison(root, explorers)
            validate()  # Historical records need no optional property.
            record['memberDestinations'] = [None, link]
            validate()
            for bad in ([], [None], 'bad', [None, 12], [None, ''], [link, link]):
                record['memberDestinations'] = bad
                with self.subTest(destinations=bad), self.assertRaises(ValueError):
                    validate()
            record['memberDestinations'] = [None, link]
            record['kind'] = 'ENUM_VALUE_ADDED'
            with self.assertRaisesRegex(ValueError, 'destination kind'):
                validate()

    def test_independent_identity_proof(self):
        import copy
        q = ['urn:a', 'Derived']
        segment = dict(owner=q, compositor='Record')
        d = dict(qname=q, kind='Record', segments=[dict(owner=['urn:a', 'Base'], compositor='Record'), segment],
                 members=[dict(index=0, name='Field', type={'primitive': 'String'}), dict(index=1, name='Other', type={'primitive': 'String'})])
        base = dict(qname=['urn:a', 'Base'], kind='Record', segments=[dict(owner=['urn:a', 'Base'], compositor='Record')],
                    members=[dict(index=0, name='Field', type={'primitive': 'String'})])
        indexes = ({}, {tuple(q): d, ('urn:a', 'Base'): base}, {})
        c = dict(kind='MEMBER_CHANGED', property='"Field"/cardinality')
        state = dict(v=1, kind='type', root=q, path=[segment, dict(member=0)])
        def url(s, version='2.5'):
            return '../' + version + '/explorer.html#uci=' + urllib.parse.quote(json.dumps(s))
        pages.member_destination(url(state), '2.5', q, c, 0, indexes)
        mutations = [(dict(state, root=['urn:a', 'Missing']), 'unknown explorer root'),
            (dict(state, root=['urn:wrong', 'Derived']), 'unknown explorer root'),
            (dict(state, path=[dict(owner=q, compositor='Choice'), dict(member=0)]), 'unknown explorer segment'),
            (dict(state, path=[segment, dict(member=9)]), 'unknown explorer member'),
            (dict(state, path=[segment, dict(member=1)]), 'identity mismatch'),
            (dict(state, path=[dict(owner=['urn:a', 'Base'], compositor='Record'), dict(member=0)]), 'identity mismatch'),
            (dict(state, path=state['path'] + [dict(member=0)]), 'terminal or recursive'),
            (dict(state, extra=True), 'invalid explorer state')]
        for s, reason in mutations:
            with self.subTest(state=s), self.assertRaisesRegex(ValueError, reason):
                pages.member_destination(url(s), '2.5', q, c, 0, indexes)
        for bad in (url(state, '2.6'), 'https://evil.example/' + url(state), '/' + url(state)):
            with self.subTest(url=bad), self.assertRaisesRegex(ValueError, 'release/path'):
                pages.member_destination(bad, '2.5', q, c, 0, indexes)
        for prop in ('Field/cardinality', '"Field"/unknown', '"Field"/cardinality/extra'):
            with self.subTest(prop=prop), self.assertRaisesRegex(ValueError, 'association'):
                pages.member_destination(url(state), '2.5', q, dict(c, property=prop), 0, indexes)
        with self.assertRaisesRegex(ValueError, 'absent member side'):
            pages.member_destination(url(state), '2.5', q, dict(c, kind='MEMBER_ADDED'), 0, indexes)
        for names, reason in ((['Other', 'Field'], 'identity mismatch'),
                              (['Other'], 'association'), (['Field', 'Field'], 'association')):
            modified = copy.deepcopy(indexes)
            modified[1][tuple(q)]['members'] = [dict(index=i, name=n, type={'primitive': 'String'}) for i, n in enumerate(names)]
            with self.subTest(names=names), self.assertRaisesRegex(ValueError, reason):
                pages.member_destination(url(state), '2.5', q, c, 0, modified)


class ArchiveTests(unittest.TestCase):
    def test_exactly_one_safe_root(self):
        for members, valid in [
            (['a/' + pages.ROOT26], True),
            ([], False),
            ([pages.ROOT26, 'b/' + pages.ROOT26], False),
            (['../' + pages.ROOT26], False),
            ([pages.ROOT26, pages.ROOT26], False),
        ]:
            with self.subTest(members=members), tempfile.TemporaryDirectory() as tmp:
                archive = pathlib.Path(tmp) / 'release.zip'
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore', UserWarning)
                    with zipfile.ZipFile(archive, 'w') as source:
                        for member in members:
                            source.writestr(member, '<schema/>')
                if valid:
                    with contextlib.redirect_stdout(io.StringIO()):
                        pages.extract(archive, pathlib.Path(tmp) / 'extracted')
                    self.assertTrue((pathlib.Path(tmp) / 'extracted/a' / pages.ROOT26).is_file())
                else:
                    with self.assertRaises(ValueError):
                        pages.extract(archive, pathlib.Path(tmp) / 'extracted')


class ExplorerStateTests(unittest.TestCase):
    def test_common_state_corpus(self):
        directory = pathlib.Path(__file__).resolve().parents[1] / 'tests/task080-browser'
        cases = json.loads((directory / 'state-cases.json').read_text())
        # Independently authored structural registry, matching the generated
        # fixture identities; no browser normalizer is reused here.
        q = lambda name: ['urn:explorer', name]
        member = lambda name, target=None: {'name': name, 'type': {'named': q(target)} if target else {'primitive': 'String'}}
        types = {}
        for name, kind, members, segments, edge in [
            ('Root', 'Record', [member('First', 'Child'), member('Second', 'Child'), member('Bounded', 'Enum'), member('Zero'), member('Huge'), member('List', 'Items'), member('Inherited', 'Child'), {'name': 'Other', 'type': {'named': ['urn:other', 'Child']}}], ['Base', 'Root'], {}),
            ('Base', 'Record', [member('Inherited')], ['Base'], {}),
            ('Child', 'Record', [member('constructor'), member('__proto__', 'Child'), member('Pick', 'Choice')], ['Child'], {}),
            ('Choice', 'Choice', [member('Left'), member('Right', 'Enum')], ['Choice'], {}),
            ('Enum', 'Enumeration', [], [], {}),
            ('Alias', 'Alias', [], [], {'target': {'named': q('Enum')}}),
            ('Items', 'List', [], [], {'item': {'named': q('Alias')}}),
        ]:
            types[tuple(q(name))] = dict(qname=q(name), kind=kind, members=members,
                segments=[{'owner': q(owner), 'compositor': kind} for owner in segments], **edge)
        types[('urn:other', 'Child')] = dict(qname=['urn:other', 'Child'], kind='Record',
            members=types[tuple(q('Child'))]['members'], segments=[{'owner': ['urn:other', 'Child'], 'compositor': 'Record'}])
        indexes = ({}, types, {tuple(q('Report')): {'type': {'named': q('Root')}}})
        encode = lambda state: 'uci=' + urllib.parse.quote(json.dumps(state))
        for case in cases['accepted']:
            with self.subTest(accepted=case['name']):
                pages.explorer_state(encode(case['state']), indexes)
        for case in cases['rejected']:
            state = case.get('state', {'v': 1, 'kind': 'message', 'root': q('Report'),
                'path': case.get('path', ([{'member': 0}] * case['depth'] if 'depth' in case else
                    [{'owner': q('Root'), 'compositor': 'Record'}, {'member': case.get('member')}]))})
            fragment = case.get('fragment', 'uci=' + 'x' * case['size'] if 'size' in case else encode(state))
            with self.subTest(rejected=case['name']), self.assertRaises(ValueError):
                pages.explorer_state(fragment, indexes)


class SiteTests(unittest.TestCase):
    def test_links_anchors_search_and_purity(self):
        original = pages.RELEASES
        pages.RELEASES = {'2.5': (1, 1, 'root25.xsd'), '2.6': (1, 1, 'root26.xsd')}
        try:
            with tempfile.TemporaryDirectory() as tmp:
                root = pathlib.Path(tmp)
                (root / 'index.html').write_text('<a href="2.5/">UCI 2.5</a><a href="2.6/">UCI 2.6</a>')
                for version, schema in [('2.5', 'root25.xsd'), ('2.6', 'root26.xsd')]:
                    directory = root / version
                    (directory / 'assets').mkdir(parents=True)
                    (directory / 'types').mkdir()
                    (directory / 'index.html').write_text(
                        schema + '<a href="types/000000.html">Type</a><li id="message-000000">Message</li>'
                        '<script src="assets/search-index.js"></script><script src="assets/search.js"></script>'
                        '<link href="assets/style.css" rel="stylesheet">')
                    (directory / 'types/000000.html').write_text('<a href="../index.html#message-000000">Message</a>')
                    (directory / 'assets/style.css').write_text('body {}')
                    (directory / 'assets/search.js').write_text('void 0;')
                    records = [{'url': 'types/000000.html', 'label': '{urn:test}Type'}, {'url': 'index.html#message-000000', 'label': '{urn:test}Message'}]
                    (directory / 'explorer.html').write_text('<a href="index.html">Reference browser</a>')
                    (directory / 'assets/explorer.css').write_text('body {}')
                    (directory / 'assets/explorer.js').write_text('void 0;')
                    (directory / 'assets/explorer-search.js').write_text('void 0;')
                    graph = {'version': 1, 'schema': {'roots': [schema], 'version': None},
                             'types': [{'qname': ['urn:test', 'Type'], 'kind': 'Record', 'abstract': False,
                                        'url': 'types/000000.html', 'base': None, 'target': None, 'item': None,
                                        'constraints': {'facets': [], 'patternGroups': []}, 'enumeration': [],
                                        'members': [], 'segments': [], 'ancestry': [['urn:test', 'Type']]}],
                             'messages': [{'qname': ['urn:test', 'Message'], 'type': {'named': ['urn:test', 'Type']},
                                           'url': 'index.html#message-000000'}]}
                    (directory / 'assets/explorer-data.js').write_text('window.schemaExplorerData = ' + json.dumps(graph) + ';\n')
                    search = directory / 'assets/search-index.js'
                    search.write_text('window.schemaSearchIndex = ' + json.dumps(records) + ';\n')
                compare = root / 'compare'
                (compare / 'assets').mkdir(parents=True)
                (compare / 'index.html').write_text('<p>Comparison</p>')
                for name in ('compare.js', 'compare.css'):
                    (compare / 'assets' / name).write_text('')
                declarations = []
                for cat, name, url in [('Type', 'Type', 'types/000000.html'), ('Message', 'Message', 'index.html#message-000000')]:
                    q = ['urn:test', name]
                    declarations.append(dict(category=cat, qname=q, status='Unchanged', changes=[], destinations=[
                        dict(reference='../' + v + '/' + url, explorer='../' + v + '/explorer.html#uci=' + urllib.parse.quote(json.dumps(dict(v=1, kind=cat.lower(), root=q, path=[])))) for v in ('2.5', '2.6')]))
                model = dict(version=1, scope='Direct normalized SchemaIr changes — not a compatibility verdict.',
                    releases=[dict(release=v, revision=pin, schemaVersion='absent') for v, pin in zip(('2.5', '2.6'), pages.PINS)],
                    summaries={cat: dict(before=1, after=1, added=0, removed=0, changed=0, unchanged=1) for cat in ('Type', 'Message')}, declarations=declarations)
                comparison_data = compare / 'assets/compare-data.js'
                good_comparison = 'window.schemaCompareData = ' + json.dumps(model) + ';\n'
                comparison_data.write_text(good_comparison)
                with contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
                import copy
                mutations = [
                    ('duplicate comparison identity', lambda m: m['declarations'].append(copy.deepcopy(m['declarations'][0]))),
                    ('pinned releases mismatch', lambda m: m['releases'].reverse()),
                    ('summary mismatch', lambda m: m['summaries']['Type'].update(changed=1)),
                    ('reference mismatch', lambda m: m['declarations'][0]['destinations'][0].update(reference='../2.5/types/missing.html')),
                    ('unsafe comparison explorer path', lambda m: m['declarations'][0]['destinations'][0].update(explorer='../../private.html')),
                    ('status mismatch', lambda m: m['declarations'][0].update(status='Added')),
                    ('invalid comparison values', lambda m: (m['declarations'][0].update(status='Changed'), m['declarations'][0]['changes'].append(dict(property='abstract', kind='ABSTRACT_CHANGED', before=123, after='true')))),
                    ('unknown comparison declaration', lambda m: m['declarations'][0].update(qname=['urn:missing', 'Type'])),
                ]
                for expected, mutate in mutations:
                    broken = copy.deepcopy(model)
                    mutate(broken)
                    comparison_data.write_text('window.schemaCompareData = ' + json.dumps(broken) + ';\n')
                    with self.assertRaisesRegex(ValueError, expected), contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                comparison_data.write_text('window.schemaCompareData = {}; alert(1);\n')
                with self.assertRaisesRegex(ValueError, 'invalid comparison wrapper'), contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
                comparison_data.write_text(good_comparison)
                extra_comparison = compare / 'assets/unexplained.js'
                extra_comparison.write_text('')
                with self.assertRaisesRegex(ValueError, 'comparison unexpected files'), contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
                extra_comparison.unlink()
                for name in ('compare.js', 'compare.css', 'compare-data.js'):
                    asset = compare / 'assets' / name
                    text = asset.read_text()
                    asset.unlink()
                    with self.assertRaisesRegex(ValueError, 'comparison missing assets'), contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                    asset.write_text(text)
                explorer_data = root / '2.6/assets/explorer-data.js'
                good_data = explorer_data.read_text()
                for mutation, expected in [
                    (lambda g: g.update(version=2), 'version'),
                    (lambda g: g['types'].append(g['types'][0]), 'inventory'),
                    (lambda g: g['messages'][0].update(type={'named': ['urn:test', 'Missing']}), 'unresolved'),
                    (lambda g: g['schema'].update(roots=['root25.xsd']), 'identity'),
                    (lambda g: g['types'][0].update(url='../escape.html'), 'inventory'),
                    (lambda g: g['types'][0].update(base={'named': ['urn:test', 'Missing']}), 'unresolved'),
                    (lambda g: g['types'][0].update(target={'named': ['urn:test', 'Missing']}), 'unresolved'),
                    (lambda g: g['types'][0].update(item={'named': ['urn:test', 'Missing']}), 'unresolved'),
                    (lambda g: g['types'][0].update(segments=[{'owner': ['urn:test', 'Missing'], 'compositor': 'Record'}]), 'owner'),
                    (lambda g: g['types'][0].update(members=[{'index': 0, 'name': '__proto__', 'namespace': None, 'nillable': False, 'type': {'named': ['urn:test', 'Missing']}}]), 'unresolved'),
                ]:
                    changed = json.loads(good_data[len('window.schemaExplorerData = '):-2])
                    mutation(changed)
                    explorer_data.write_text('window.schemaExplorerData = ' + json.dumps(changed) + ';\n')
                    with self.assertRaisesRegex(ValueError, expected), contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                    explorer_data.write_text(good_data)
                target_page = root / '2.6/types/000000.html'
                good_page = target_page.read_text()
                state = {'v': 1, 'kind': 'type', 'root': ['urn:test', 'Type'], 'path': []}
                target_page.write_text('<a href="../explorer.html#uci=' + urllib.parse.quote(json.dumps(state)) + '">Explore</a>')
                with contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
                for bad in ['uci=%7B', 'uci=' + 'x' * 16385,
                            'uci=' + urllib.parse.quote(json.dumps(dict(state, root=['urn:test', 'Missing']))),
                            'uci=' + urllib.parse.quote(json.dumps(dict(state, path=[{'member': 0}]))),
                            'uci=' + urllib.parse.quote(json.dumps(dict(state, path=[{'step': 'javascript:alert(1)'}])))]:
                    target_page.write_text('<a href="../explorer.html#' + bad + '">Bad</a>')
                    with self.assertRaises(ValueError), contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                target_page.write_text(good_page)
                for bad_wrapper in ['window.schemaExplorerData = {}; alert(1);\n', 'window.schemaExplorerData = not_json;\n']:
                    explorer_data.write_text(bad_wrapper)
                    with self.assertRaises(ValueError), contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                    explorer_data.write_text(good_data)
                extra = root / '2.6/assets/unexplained.js'
                extra.write_text('void 0;')
                with self.assertRaisesRegex(ValueError, 'unexpected files'), contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
                extra.unlink()
                for filename in ['explorer.js', 'explorer-search.js']:
                    asset = root / '2.6/assets' / filename
                    asset_text = asset.read_text()
                    asset.unlink()
                    with self.assertRaisesRegex(ValueError, 'missing assets'), contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                    asset.write_text(asset_text)
                search.write_text('window.schemaSearchIndex = ' + json.dumps(
                    [{'url': 'types/missing.html', 'label': '{urn:test}Type'}, {'url': 'index.html#message-000000', 'label': '{urn:test}Message'}]) + ';\n')
                with self.assertRaisesRegex(ValueError, 'inventory mismatch'):
                    with contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                search.write_text('window.schemaSearchIndex = ' + json.dumps(records) + ';\n')
                (root / '2.6/types/000000.html').write_text('<a href="../index.html#missing">Bad</a>')
                with self.assertRaisesRegex(ValueError, 'broken anchor'):
                    with contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
                (root / '2.6/types/000000.html').write_text('<a href="file:///tmp/source.xsd">Bad</a>')
                with self.assertRaisesRegex(ValueError, 'local checkout reference'):
                    with contextlib.redirect_stdout(io.StringIO()):
                        pages.validate(root)
        finally:
            pages.RELEASES = original


if __name__ == '__main__':
    unittest.main()