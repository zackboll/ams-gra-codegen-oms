#!/usr/bin/env python3
"""Package Task 069 TSV, never infer semantic differences from inventories."""
import collections
import json
import pathlib
import re
import sys
import urllib.parse

PINS = {'2.5': '093610b7753944059360d3236770ab446d039556',
        '2.6': '78eb61b6112c8bffa40820c33124b57787fc5bd9'}
STATUSES = ('Added', 'Removed', 'Changed', 'Unchanged')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_data(path, name):
    match = re.fullmatch(r'window\.' + name + r' = (.*);\n', path.read_text(), re.DOTALL)
    require(match is not None, 'invalid data wrapper')
    return json.loads(match[1])


def unescape(cell):
    # Rust char::is_control() covers C0/C1; these must be escaped on the wire.
    # Unicode line/paragraph separators are text, not control characters.
    require(not re.search(r'[\x00-\x1f\x7f-\x9f]', cell), 'raw TSV control character')
    def decode(m):
        s = m[1]
        return {'\\': '\\', 't': '\t', 'n': '\n', 'r': '\r'}.get(s) if not s.startswith('u{') else chr(int(s[2:-1], 16))
    require('\\' not in re.sub(r'\\([\\tnr]|u\{[0-9a-f]+\})', '', cell), 'invalid TSV escape')
    return re.sub(r'\\([\\tnr]|u\{[0-9a-f]+\})', decode, cell)


def destination(version, category, declaration):
    state = dict(v=1, kind=category.lower(), root=declaration['qname'], path=[])
    return {'reference': '../' + version + '/' + declaration['url'],
            'explorer': '../' + version + '/explorer.html#uci=' +
            urllib.parse.quote(json.dumps(state, ensure_ascii=False, separators=(',', ':')), safe='')}


def member_destination(version, declaration, record, side):
    """Associate only exact supported Debug names, never parse arbitrary Debug text.

    Printable ASCII (including quotes/backslashes/hostile markup) has a known
    Rust Debug representation. Unusual Unicode/control escaping is deliberately
    unsupported. The graph, not the label, supplies owner, compositor and index.
    """
    suffixes = {'MEMBER_ADDED': ('presence',), 'MEMBER_REMOVED': ('presence',),
                'MEMBER_CHANGED': ('type_ref', 'cardinality', 'wire_namespace', 'nillable')}
    if (declaration is None or declaration.get('kind') not in ('Record', 'Choice') or
            record['kind'] not in suffixes or
            record['kind'] == ('MEMBER_ADDED' if side == 0 else 'MEMBER_REMOVED')):
        return None
    segment = dict(owner=declaration['qname'], compositor=declaration['kind'])
    if declaration.get('segments', []).count(segment) != 1:
        return None
    matches = []
    for i, member in enumerate(declaration.get('members', [])):
        name = member['name']
        if not all(' ' <= c <= '~' for c in name):
            continue
        formatted = json.dumps(name, ensure_ascii=True)
        if any(record['property'] == formatted + '/' + suffix for suffix in suffixes[record['kind']]):
            matches.append((i, member))
    if len(matches) != 1:
        return None
    index, member = matches[0]
    if type(member.get('index')) is not int or member['index'] != index:
        return None
    state = dict(v=1, kind='type', root=declaration['qname'],
                 path=[segment, dict(member=member['index'])])
    fragment = 'uci=' + urllib.parse.quote(json.dumps(state, ensure_ascii=False, separators=(',', ':')), safe='')
    return '../' + version + '/explorer.html#' + fragment if len(fragment) <= 16384 else None


def package(tsv, graphs):
    # Task 069 records use LF only. U+2028/U+2029 belong to their cells.
    lines = tsv.split('\n')
    if lines and lines[-1] == '':
        lines.pop()
    require(lines.pop(0) == 'category\tnamespace\tdeclaration\tmember_or_property\tchange\tbefore\tafter', 'invalid TSV header')
    summaries, totals, changes, versions = {}, {}, collections.defaultdict(list), None
    for line in lines:
        cells = [unescape(c) for c in line.split('\t')]
        require(len(cells) == 7, 'invalid TSV columns')
        cat, ns, name, prop, kind, before, after = cells
        if cat == 'METADATA':
            require(versions is None and prop == 'schema_version', 'invalid metadata')
            versions = [before, after]
        elif cat == 'SUMMARY':
            require((name, prop) not in summaries, 'duplicate summary')
            summaries[name, prop] = [int(before), int(after)] if prop == 'count' else int(after)
        elif cat == 'TOTAL':
            require(kind not in totals, 'duplicate total')
            totals[kind] = int(after)
        else:
            require(cat in ('TYPE', 'MESSAGE'), 'unknown TSV category')
            record = dict(property=prop, kind=kind, before=None if before == 'absent' else before,
                          after=None if after == 'absent' else after)
            require(record not in changes[cat, ns, name], 'duplicate property record')
            changes[cat, ns, name].append(record)
    require(versions is not None, 'missing metadata')
    require(versions == [('absent' if g['schema']['version'] is None else json.dumps(g['schema']['version'], ensure_ascii=False)) for g in graphs],
            'schema metadata mismatch')
    result = dict(version=1, scope='Direct normalized SchemaIr changes — not a compatibility verdict.',
                  releases=[dict(release=v, revision=PINS[v], schemaVersion=versions[i]) for i, v in enumerate(PINS)],
                  summaries={}, declarations=[])
    observed = collections.Counter()
    for category, inventory in [('Type', 'types'), ('Message', 'messages')]:
        indexes = [{tuple(d['qname']): d for d in g[inventory]} for g in graphs]
        require(all(len(indexes[i]) == len(graphs[i][inventory]) for i in range(2)), 'duplicate inventory identity')
        counts = collections.Counter()
        for q in sorted(set(indexes[0]) | set(indexes[1])):
            records = changes.pop((category.upper(), *q), [])
            status = 'Added' if q not in indexes[0] else 'Removed' if q not in indexes[1] else 'Changed' if records else 'Unchanged'
            counts[status] += 1
            for r in records:
                observed[r['kind']] += 1
                if category == 'Type' and r['kind'] in ('MEMBER_ADDED', 'MEMBER_REMOVED', 'MEMBER_CHANGED'):
                    links = [member_destination(v, index.get(q), r, i)
                             for i, (v, index) in enumerate(zip(PINS, indexes))]
                    if any(links):
                        r['memberDestinations'] = links
            result['declarations'].append(dict(category=category, qname=list(q), status=status, changes=records,
                destinations=[destination(v, category, index[q]) if q in index else None for v, index in zip(PINS, indexes)]))
        summary = dict(before=len(indexes[0]), after=len(indexes[1]), **{s.lower(): counts[s] for s in STATUSES})
        require(summaries.get((category.upper(), 'count')) == [summary['before'], summary['after']], 'inventory count mismatch')
        require(all(summaries.get((category.upper(), s.lower())) == counts[s] for s in STATUSES), 'status summary mismatch')
        result['summaries'][category] = summary
    require(not changes, 'unknown changed declaration')
    require(dict(observed) == totals, 'property total mismatch')
    for declaration in result['declarations']:
        if declaration['status'] in ('Added', 'Removed'):
            records = declaration['changes']
            require(len(records) == 1 and records[0]['property'] == 'presence' and
                    records[0]['kind'] == declaration['category'].upper() + '_' + declaration['status'].upper(),
                    'invalid declaration presence records')
    return result


if __name__ == '__main__':
    tsv, before, after, output = map(pathlib.Path, sys.argv[1:])
    model = package(tsv.read_text(), [read_data(p, 'schemaExplorerData') for p in (before, after)])
    output.write_text('window.schemaCompareData = ' + json.dumps(model, ensure_ascii=True, separators=(',', ':')) + ';\n')