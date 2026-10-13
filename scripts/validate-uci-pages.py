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
PINS = ['093610b7753944059360d3236770ab446d039556', '78eb61b6112c8bffa40820c33124b57787fc5bd9']


def comparison(root, explorers):
    directory = root / 'compare'
    expected = {directory / 'index.html'} | {directory / 'assets' / name for name in
        ('compare.css', 'compare.js', 'compare-data.js')}
    require(all(p.is_file() for p in expected), 'comparison missing assets')
    require(set(p for p in directory.rglob('*') if p.is_file()) == expected, 'comparison unexpected files')
    match = re.fullmatch(r'window\.schemaCompareData = (.*);\n', (directory / 'assets/compare-data.js').read_text(), re.DOTALL)
    require(match is not None, 'invalid comparison wrapper')
    try:
        model = json.loads(match[1])
    except json.JSONDecodeError as error:
        raise ValueError('invalid comparison wrapper JSON') from error
    require(model.get('version') == 1, 'invalid comparison version')
    require(model.get('scope') == 'Direct normalized SchemaIr changes — not a compatibility verdict.', 'invalid comparison scope')
    require([r['release'] for r in model['releases']] == ['2.5', '2.6'] and
            [r['revision'] for r in model['releases']] == PINS, 'comparison pinned releases mismatch')
    inventories = []
    for i, version in enumerate(('2.5', '2.6')):
        graph = json.loads(re.fullmatch(r'window\.schemaExplorerData = (.*);\n',
            (root / version / 'assets/explorer-data.js').read_text(), re.DOTALL)[1])
        # Task 069 metadata retains its quoted text formatting.
        schema_version = graph['schema']['version']
        require(model['releases'][i]['schemaVersion'] == ('absent' if schema_version is None else json.dumps(schema_version)),
                'comparison schema version mismatch')
        inventories.append({cat: {qkey(d['qname']): d for d in graph[key]}
                            for cat, key in [('Type', 'types'), ('Message', 'messages')]})
    seen = set()
    counts = {cat: dict(added=0, removed=0, changed=0, unchanged=0) for cat in ('Type', 'Message')}
    kinds = {'TYPE_ADDED', 'TYPE_REMOVED', 'TYPE_CHANGED', 'MESSAGE_ADDED', 'MESSAGE_REMOVED', 'MESSAGE_CHANGED',
             'MEMBER_ADDED', 'MEMBER_REMOVED', 'MEMBER_CHANGED', 'MEMBER_ORDER_CHANGED', 'ENUM_VALUE_ADDED',
             'ENUM_VALUE_REMOVED', 'ENUM_ORDER_CHANGED', 'CONSTRAINT_CHANGED', 'BASE_TYPE_CHANGED', 'ABSTRACT_CHANGED'}
    properties = {'presence', 'abstract', 'base_type', 'kind', 'primitive', 'type_ref', 'payload', 'item_type',
                  'cardinality', 'wire_namespace', 'nillable', 'order', 'minInclusive', 'maxInclusive',
                  'minExclusive', 'maxExclusive', 'length', 'minLength', 'maxLength', 'whiteSpace', 'pattern_groups'}
    for d in model['declarations']:
        cat = d['category']
        require(cat in counts, 'invalid comparison category')
        q = qkey(d['qname'])
        require((cat, q) not in seen, 'duplicate comparison identity')
        seen.add((cat, q))
        present = [q in inventory[cat] for inventory in inventories]
        require(any(present), 'unknown comparison declaration')
        changes = d['changes']
        expected_status = 'Added' if not present[0] else 'Removed' if not present[1] else 'Changed' if changes else 'Unchanged'
        require(d['status'] == expected_status, 'comparison status mismatch')
        counts[cat][expected_status.lower()] += 1
        require(len(d['destinations']) == 2, 'invalid comparison destinations')
        for i, version in enumerate(('2.5', '2.6')):
            dest = d['destinations'][i]
            if not present[i]:
                require(dest is None, 'absent comparison destination')
                continue
            require(isinstance(dest, dict) and set(dest) == {'reference', 'explorer'}, 'missing comparison destination')
            require(dest['reference'] == '../' + version + '/' + inventories[i][cat][q]['url'], 'comparison reference mismatch')
            parsed = urllib.parse.urlsplit(dest['explorer'])
            require(not parsed.scheme and not parsed.netloc and not parsed.query and parsed.path == '../' + version + '/explorer.html', 'unsafe comparison explorer path')
            explorer_state(parsed.fragment, explorers[root / version / 'explorer.html'])
            state = json.loads(urllib.parse.unquote(parsed.fragment[4:]))
            require(state == dict(v=1, kind=cat.lower(), root=list(q), path=[]), 'comparison explorer identity mismatch')
        records = set()
        for c in changes:
            require(set(c) == {'property', 'kind', 'before', 'after'}, 'invalid comparison property record')
            require(c['kind'] in kinds and isinstance(c['property'], str) and c['property'].rsplit('/', 1)[-1] in properties,
                    'invalid comparison property labels')
            require((cat == 'Message' and c['kind'].startswith('MESSAGE_') and c['property'] in ('presence', 'payload')) or
                    (cat == 'Type' and not c['kind'].startswith('MESSAGE_')), 'comparison property category mismatch')
            require(all(c[v] is None or isinstance(c[v], str) for v in ('before', 'after')) and c['before'] != c['after'], 'invalid comparison values')
            signature = json.dumps(c, sort_keys=True)
            require(signature not in records, 'duplicate comparison property')
            records.add(signature)
        if expected_status in ('Added', 'Removed'):
            require(len(changes) == 1 and changes[0]['kind'] == cat.upper() + '_' + expected_status.upper() and
                    changes[0]['property'] == 'presence' and changes[0]['before' if expected_status == 'Added' else 'after'] is None,
                    'invalid declaration presence record')
        else:
            require(not any(c['kind'] in {'TYPE_ADDED', 'TYPE_REMOVED', 'MESSAGE_ADDED', 'MESSAGE_REMOVED'} for c in changes), 'common declaration presence record')
    for cat in counts:
        require({q for c, q in seen if c == cat} == set(inventories[0][cat]) | set(inventories[1][cat]), 'comparison incomplete inventory')
        expected_summary = dict(before=len(inventories[0][cat]), after=len(inventories[1][cat]), **counts[cat])
        require(model['summaries'][cat] == expected_summary, 'comparison summary mismatch')


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


def qkey(q):
    require(isinstance(q, list) and len(q) == 2 and all(isinstance(v, str) for v in q), 'invalid QName')
    return tuple(q)


def explorer_graph(directory, records, types, messages, schema):
    wrapper = (directory / 'assets/explorer-data.js').read_text()
    match = re.fullmatch(r'window\.schemaExplorerData = (.*);\n', wrapper, re.DOTALL)
    require(match is not None, 'invalid explorer data wrapper')
    graph = json.loads(match.group(1))
    require(graph.get('version') == 1, 'invalid explorer data version')
    require(len(graph['types']) == types and len(graph['messages']) == messages, 'explorer inventory mismatch')
    registry = {}
    message_registry = {}
    for declarations, index in [(graph['types'], registry), (graph['messages'], message_registry)]:
        for declaration in declarations:
            identity = qkey(declaration['qname'])
            require(identity not in index, 'duplicate explorer identity')
            index[identity] = declaration
    require(set(t['url'] for t in graph['types']) == set(r['url'] for r in records if r['url'].startswith('types/')),
            'explorer type URL inventory mismatch')
    require(set(m['url'] for m in graph['messages']) == set(r['url'] for r in records if r['url'].startswith('index.html#message-')),
            'explorer message URL inventory mismatch')
    for declaration in graph['types'] + graph['messages']:
        matching = [r for r in records if r['url'] == declaration['url']]
        require(len(matching) == 1 and matching[0].get('label') == '{' + declaration['qname'][0] + '}' + declaration['qname'][1],
                'explorer identity/search mismatch')
    roots = graph['schema']['roots']
    require(isinstance(roots, list) and any(pathlib.PurePosixPath(r).name == schema for r in roots), 'explorer schema identity mismatch')
    other = RELEASES['2.6' if directory.name == '2.5' else '2.5'][2]
    require(not any(other in r for r in roots), 'mixed explorer release metadata')
    if schema in ('UCI_MessageDefinitions_v2_5_0.xsd', ROOT26):
        require(graph['schema']['version'] == ('002.5.0' if directory.name == '2.5' else '002.6.0'),
                'swapped explorer schema version')

    def reference(r):
        require(isinstance(r, dict), 'invalid explorer reference')
        if 'named' in r:
            require(set(r) == {'named'} and qkey(r['named']) in registry, 'unresolved explorer reference')
        else:
            require(set(r) == {'primitive'} and r['primitive'] in ('Boolean', 'SignedInteger', 'UnsignedInteger', 'Decimal', 'Float32', 'Float64', 'String', 'Binary', 'DateTime', 'Time', 'Duration'), 'invalid primitive reference')

    def count(c):
        require(isinstance(c, dict) and set(c) == {'min', 'max'}, 'invalid occurrence range')
        require(isinstance(c['min'], str) and re.fullmatch(r'0|[1-9][0-9]*', c['min']), 'invalid occurrence minimum')
        require(c['max'] is None or isinstance(c['max'], str) and re.fullmatch(r'0|[1-9][0-9]*', c['max']) and int(c['max']) >= int(c['min']), 'invalid occurrence maximum')

    def constraints(c):
        require(isinstance(c, dict) and set(c) == {'facets', 'patternGroups'}, 'invalid explorer constraints')
        require(isinstance(c['facets'], list) and isinstance(c['patternGroups'], list), 'invalid constraint lists')
        for facet in c['facets']:
            require(set(facet) == {'name', 'value'} and isinstance(facet['name'], str) and isinstance(facet['value'], str), 'invalid constraint facet')
        for group in c['patternGroups']:
            require(isinstance(group, list), 'invalid pattern group')
            for pattern in group:
                require(set(pattern) == {'dialect', 'expression'} and pattern['dialect'] == 'XmlSchema' and isinstance(pattern['expression'], str), 'invalid pattern expression')

    for t in graph['types']:
        require(re.fullmatch(r'types/[0-9]{6}\.html', t['url']), 'invalid explorer reference URL')
        require(t['kind'] in ('Primitive', 'Alias', 'Enumeration', 'Record', 'Choice', 'List'), 'invalid explorer kind')
        require(isinstance(t['abstract'], bool), 'invalid abstractness')
        constraints(t['constraints'])
        require(isinstance(t['enumeration'], list), 'invalid enumeration')
        for variant in t['enumeration']:
            require(isinstance(variant['value'], str) and (variant['documentation'] is None or isinstance(variant['documentation'], str)), 'invalid enumeration value')
        for name in ('base', 'target', 'item'):
            if t[name] is not None:
                reference(t[name])
        if t['item'] is not None:
            count(t['itemCount'])
        for i, m in enumerate(t['members']):
            require(m['index'] == i and isinstance(m['name'], str) and (m['namespace'] is None or isinstance(m['namespace'], str)) and isinstance(m['nillable'], bool), 'invalid explorer member')
            reference(m['type'])
            count(m['count'])
            constraints(m['constraints'])
        for owner in t['ancestry']:
            require(qkey(owner) in registry, 'unresolved ancestry owner')
        for segment in t['segments']:
            owner = registry.get(qkey(segment['owner']))
            require(owner is not None and segment['compositor'] == owner['kind'] and owner['kind'] in ('Record', 'Choice'), 'invalid structural owner/compositor')
    for m in graph['messages']:
        reference(m['type'])
        require(re.fullmatch(r'index\.html#message-[0-9]{6}', m['url']), 'invalid explorer message URL')
    return graph, registry, message_registry


def explorer_state(fragment, indexes):
    require(fragment.startswith('uci=') and len(fragment) <= 16384 and not re.search(r'%(?![0-9A-Fa-f]{2})', fragment), 'invalid explorer fragment')
    try:
        state = json.loads(urllib.parse.unquote(fragment[4:], errors='strict'))
    except (ValueError, UnicodeError) as error:
        raise ValueError('malformed explorer state') from error
    graph, types, messages = indexes
    require(set(state) == {'v', 'kind', 'root', 'path'} and state['v'] == 1 and state['kind'] in ('message', 'type'), 'invalid explorer state')
    root = (messages if state['kind'] == 'message' else types).get(qkey(state['root']))
    require(root is not None, 'unknown explorer root')
    require(isinstance(state['path'], list) and len(state['path']) <= 24, 'excessive explorer path')
    ref = root['type'] if state['kind'] == 'message' else {'named': root['qname']}
    current = types.get(qkey(ref['named'])) if 'named' in ref else None
    owner = None
    ancestors = set()
    for step in state['path']:
        require(isinstance(step, dict), 'invalid explorer path step')
        if owner is not None:
            require(set(step) == {'member'} and type(step['member']) is int and 0 <= step['member'] < len(owner['members']), 'unknown explorer member')
            ref = owner['members'][step['member']]['type']
            current = types.get(qkey(ref['named'])) if 'named' in ref else None
            owner = None
            continue
        require(current is not None and qkey(current['qname']) not in ancestors, 'terminal or recursive explorer path')
        ancestors.add(qkey(current['qname']))
        if current['kind'] in ('Record', 'Choice'):
            require(set(step) == {'owner', 'compositor'} and step in current['segments'], 'unknown explorer segment')
            owner = types[qkey(step['owner'])]
        else:
            name = 'alias' if current['kind'] == 'Alias' else 'item'
            require(step == {'step': name} and current['kind'] in ('Alias', 'List'), 'unknown explorer reference step')
            ref = current['target'] if name == 'alias' else current['item']
            current = types.get(qkey(ref['named'])) if 'named' in ref else None


def validate(root):
    require(root.is_dir() and not root.is_symlink(), 'missing site root')
    root = root.resolve()
    entries = list(root.rglob('*'))
    require(all(not p.is_symlink() and (p.is_file() or p.is_dir()) for p in entries),
            'non-file or symlink in artifact')
    files = [p for p in entries if p.is_file()]
    require((root / 'index.html').is_file(), 'missing landing page')
    require(set(root.iterdir()) == {root / 'index.html', root / '2.5', root / '2.6', root / 'compare'}, 'unexpected top-level material')
    require('UCI 2.5' in (root / 'index.html').read_text() and
            'UCI 2.6' in (root / 'index.html').read_text(), 'missing release selector')
    search_urls = {}
    explorers = {}
    for version, (types, messages, schema) in RELEASES.items():
        directory = root / version
        expected = {directory / 'index.html', directory / 'assets/style.css',
                    directory / 'assets/search.js', directory / 'assets/search-index.js',
                    directory / 'explorer.html', directory / 'assets/explorer.css',
                    directory / 'assets/explorer.js', directory / 'assets/explorer-search.js', directory / 'assets/explorer-data.js'}
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
        explorers[directory / 'explorer.html'] = explorer_graph(directory, records, types, messages, schema)
        require(set(p for p in directory.rglob('*') if p.is_file()) == expected | set(pages), f'{version}: unexpected files')
        print(f'{version}: {types} types, {messages} messages, {types + 9} files')
    comparison(root, explorers)
    parsed_pages = {}
    for path in files:
        require(path.suffix.lower() not in ('.xsd', '.zip') and
                not any(part in ('.git', 'target', 'standard', 'extracted') for part in path.relative_to(root).parts),
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
                if target in explorers:
                    explorer_state(parsed.fragment, explorers[target])
                else:
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
    except (ValueError, OSError, KeyError, TypeError, IndexError, zipfile.BadZipFile) as error:
        sys.exit(f'Pages validation failed: {error}')
