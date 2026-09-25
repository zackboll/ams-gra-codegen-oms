#!/usr/bin/env python3
"""Check the publication artifact; extract the pinned archive without executing it."""
import html.parser
import json
import pathlib
import re
import sys
import urllib.parse
import zipfile

ROOT26 = 'UCI_MessageDefinitions_v2_6_0.xsd'
RELEASES = {'2.5': (5557, 722, 'UCI_MessageDefinitions_v2_5_0.xsd'),
            '2.6': (5570, 725, ROOT26)}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def extract(archive, destination):
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive) as source:
        entries = source.infolist()
        roots = [e for e in entries if not e.is_dir() and pathlib.PurePosixPath(e.filename).name == ROOT26]
        require(len(roots) == 1, f'expected exactly one 2.6 root, found {len(roots)}')
        # Reject traversal, symlinks, duplicate paths and other unsafe archive members.
        seen = set()
        for entry in entries:
            parts = pathlib.PurePosixPath(entry.filename).parts
            require(parts and not any(p in ('', '.', '..') for p in parts)
                    and not entry.filename.startswith('/') and '\\' not in entry.filename,
                    f'unsafe archive path: {entry.filename}')
            require(entry.filename not in seen, f'duplicate archive path: {entry.filename}')
            seen.add(entry.filename)
            require((entry.external_attr >> 16) & 0o170000 != 0o120000,
                    f'symlink in archive: {entry.filename}')
        source.extractall(destination)
    print(destination.joinpath(*pathlib.PurePosixPath(roots[0].filename).parts))


class Links(html.parser.HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.links = []
        self.anchors = set()

    def handle_starttag(self, tag, attrs):
        self.links.extend(value for key, value in attrs if key in ('href', 'src') and value is not None)
        self.anchors.update(value for key, value in attrs if key == 'id' and value is not None)


def validate(root):
    require(root.is_dir() and not root.is_symlink(), 'missing site root')
    root = root.resolve()
    entries = list(root.rglob('*'))
    require(all(not p.is_symlink() and (p.is_file() or p.is_dir()) for p in entries),
            'non-file or symlink in artifact')
    files = [p for p in entries if p.is_file()]
    require((root / 'index.html').is_file(), 'missing landing page')
    require(set(root.iterdir()) == {root / 'index.html', root / '2.5', root / '2.6'}, 'unexpected top-level material')
    require('UCI 2.5' in (root / 'index.html').read_text() and
            'UCI 2.6' in (root / 'index.html').read_text(), 'missing release selector')
    search_urls = {}
    for version, (types, messages, schema) in RELEASES.items():
        directory = root / version
        expected = {directory / 'index.html', directory / 'assets/style.css',
                    directory / 'assets/search.js', directory / 'assets/search-index.js'}
        require(all(p.is_file() for p in expected), f'{version}: missing assets')
        pages = sorted((directory / 'types').glob('*.html'))
        require(len(pages) == types and pages[0].name == '000000.html' and
                pages[-1].name == f'{types-1:06}.html', f'{version}: wrong type pages')
        index = (directory / 'index.html').read_text()
        require(schema in index and RELEASES['2.6' if version == '2.5' else '2.5'][2] not in index,
                f'{version}: wrong schema identity')
        search = (directory / 'assets/search-index.js').read_text()
        match = re.fullmatch(r'window\.schemaSearchIndex = (.*);\n', search, re.DOTALL)
        require(match is not None, f'{version}: invalid Task 043 search data')
        records = json.loads(match.group(1))
        require(len(records) == types + messages, f'{version}: normalization count mismatch')
        require(sum(r['url'].startswith('types/') for r in records) == types and
                sum(r['url'].startswith('index.html#message-') for r in records) == messages,
                f'{version}: search inventory mismatch')
        search_urls[version] = [r['url'] for r in records]
        require(len([p for p in directory.rglob('*') if p.is_file()]) == types + 4, f'{version}: unexpected files')
        print(f'{version}: {types} types, {messages} messages, {types + 4} files')
    parsed_pages = {}
    for path in files:
        require(path.suffix.lower() not in ('.xsd', '.zip') and
                not any(part in ('.git', 'target', 'standard', 'extracted') for part in path.parts),
                f'non-publication material: {path}')
        require(path.suffix in ('.html', '.css', '.js'), f'unexpected file type: {path}')
        if path.suffix in ('.html', '.css', '.js'):
            content = path.read_text()
            require(not re.search(r'file://|/tmp/|/home/|/runner/|/standard/|/extracted/', content, re.I),
                    f'local checkout reference in {path}')
        if path.suffix == '.html':
            page = Links()
            page.feed(content)
            parsed_pages[path] = page
    for path, page in parsed_pages.items():
        version = path.parent.name if path.name == 'index.html' else None
        for link in page.links + search_urls.get(version, []):
            parsed = urllib.parse.urlsplit(link)
            if parsed.scheme or parsed.netloc:
                require(path == root / 'index.html' and parsed.scheme == 'https' and
                        parsed.netloc in ('github.com', 'gitlab.com'), f'unexpected external URL: {path}: {link}')
                continue
            require(not link.startswith(('/', '\\')) and not parsed.path.startswith('/') and
                    not re.match(r'^[A-Za-z]:', link), f'absolute link: {path}: {link}')
            target = (path.parent / urllib.parse.unquote(parsed.path)).resolve()
            if target.is_dir():
                target /= 'index.html'
            require(target.is_relative_to(root) and target.is_file(),
                    f'broken link: {path}: {link}')
            if parsed.fragment:
                require(target in parsed_pages and urllib.parse.unquote(parsed.fragment) in parsed_pages[target].anchors,
                        f'broken anchor: {path}: {link}')
    size = sum(p.stat().st_size for p in files)
    require(size < 1_000_000_000, 'Pages artifact exceeds 1 GB')
    print(f'link scan: OK; artifact: {len(files)} files, {size} bytes (< 1 GB)')


if __name__ == '__main__':
    try:
        require(len(sys.argv) == (4 if sys.argv[1] == 'extract' else 3), 'usage: extract ZIP DIR | validate DIR')
        if sys.argv[1] == 'extract':
            extract(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]))
        elif sys.argv[1] == 'validate':
            validate(pathlib.Path(sys.argv[2]))
        else:
            raise ValueError('unknown command')
    except (ValueError, OSError, zipfile.BadZipFile) as error:
        sys.exit(f'Pages validation failed: {error}')
