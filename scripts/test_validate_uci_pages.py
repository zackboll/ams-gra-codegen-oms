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


spec = importlib.util.spec_from_file_location(
    'pages', pathlib.Path(__file__).with_name('validate-uci-pages.py'))
pages = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pages)


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
                with contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
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
                import urllib.parse
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
                asset = root / '2.6/assets/explorer.js'
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