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
                    records = [{'url': 'types/000000.html'}, {'url': 'index.html#message-000000'}]
                    search = directory / 'assets/search-index.js'
                    search.write_text('window.schemaSearchIndex = ' + json.dumps(records) + ';\n')
                with contextlib.redirect_stdout(io.StringIO()):
                    pages.validate(root)
                search.write_text('window.schemaSearchIndex = ' + json.dumps(
                    [{'url': 'types/missing.html'}, {'url': 'index.html#message-000000'}]) + ';\n')
                with self.assertRaisesRegex(ValueError, 'broken link'):
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