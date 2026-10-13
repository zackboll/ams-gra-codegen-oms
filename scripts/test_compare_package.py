"""Independent checks of the Task 069 transport, not a semantic comparator."""
import importlib.util
import json
import pathlib
import unittest

spec = importlib.util.spec_from_file_location('comparison_package',
    pathlib.Path(__file__).resolve().parents[1] / 'site/compare/package.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class TransportTests(unittest.TestCase):
    def record_fixture(self, namespaces, before='"before"', after='"after"'):
        """Independent inventories/counts and literal transport records."""
        graph = dict(schema=dict(version=None), types=[
            dict(qname=[ns, 'Same'], url=f'types/{i:06}.html')
            for i, ns in enumerate(namespaces)], messages=[])
        count = len(namespaces)
        header = 'category\tnamespace\tdeclaration\tmember_or_property\tchange\tbefore\tafter\n'
        metadata = 'METADATA\t\t\tschema_version\tVALUE\tabsent\tabsent\n'
        summaries = ''
        for category, n in [('TYPE', count), ('MESSAGE', 0)]:
            summaries += f'SUMMARY\t\t{category}\tcount\tVALUE\t{n}\t{n}\n'
            for status, total in [('added', 0), ('removed', 0), ('changed', n), ('unchanged', 0)]:
                summaries += f'SUMMARY\t\t{category}\t{status}\tCOUNT\t\t{total}\n'
        prefix = header + metadata + summaries + f'TOTAL\t\t\t\tMEMBER_CHANGED\t\t{count * 2}\n'
        records = [f'TYPE\t{ns}\tSame\t"Field"/wire_namespace\tMEMBER_CHANGED\t{before}\t{after}\n'
                   f'TYPE\t{ns}\tSame\t"Field"/nillable\tMEMBER_CHANGED\tfalse\ttrue\n'
                   for ns in namespaces]
        return prefix, records, graph

    def test_lf_records_preserve_unicode_and_transport_values(self):
        # Actual Unicode code points, not literal six-character escape strings.
        cases = [
            ('ordinary Unicode', ['urn:雪'], '"雪"', '"after雪"', '"雪"', '"after雪"'),
            ('line separator namespace', ['urn:a\u2028b'], '"before"', '"after"', '"before"', '"after"'),
            ('paragraph separator namespace', ['urn:a\u2029b'], '"before"', '"after"', '"before"', '"after"'),
            ('both in one record', ['urn:a\u2028b\u2029c'], '"before"', '"after"', '"before"', '"after"'),
            ('semantic values', ['urn:a'], '"a\u2028b\u2029c"', '"d\u2029e\u2028f"', '"a\u2028b\u2029c"', '"d\u2029e\u2028f"'),
            ('multiple LF records', ['urn:a\u2028b', 'urn:a\u2029b'], '"before"', '"after"', '"before"', '"after"'),
            ('escaped transport', ['urn:a'], r'"雪\t\n\r\\\u{1b}"', r'"after\\"', '"雪\t\n\r\\\x1b"', '"after\\"'),
            ('absent versus empty', ['urn:a'], 'absent', '""', None, '""'),
        ]
        for label, namespaces, before, after, expected_before, expected_after in cases:
            with self.subTest(case=label):
                prefix, records, graph = self.record_fixture(namespaces, before, after)
                tsv = prefix + ''.join(records)
                model = package.package(tsv, [graph, graph])
                self.assertEqual([d['qname'] for d in model['declarations']], [[ns, 'Same'] for ns in namespaces])
                self.assertEqual(sum(len(d['changes']) for d in model['declarations']), len(namespaces) * 2)
                for d in model['declarations']:
                    self.assertEqual(d['status'], 'Changed')
                    self.assertEqual(d['changes'], [
                        dict(property='"Field"/wire_namespace', kind='MEMBER_CHANGED', before=expected_before, after=expected_after),
                        dict(property='"Field"/nillable', kind='MEMBER_CHANGED', before='false', after='true')])
                serialized = json.dumps(model, ensure_ascii=True, separators=(',', ':'))
                self.assertEqual(serialized, json.dumps(package.package(tsv, [graph, graph]), ensure_ascii=True, separators=(',', ':')))
                self.assertEqual(json.loads(serialized), model)
                # A missing terminal LF must not remove the last real record.
                self.assertEqual(package.package(tsv[:-1], [graph, graph]), model)

    def test_invalid_record_transport(self):
        prefix, records, graph = self.record_fixture(['urn:a'])
        good = prefix + ''.join(records)
        first = records[0].split('\n')[0]
        cases = [
            ('unexpected LF in cell', good.replace('"before"', '"be\nfore"'), 'invalid TSV columns'),
            ('missing column', prefix + first.rsplit('\t', 1)[0] + '\n' + records[0].split('\n', 1)[1], 'invalid TSV columns'),
            ('extra column', prefix + first + '\textra\n' + records[0].split('\n', 1)[1], 'invalid TSV columns'),
            ('invalid escape', good.replace('"before"', r'"bad\x"'), 'invalid TSV escape'),
            ('duplicate property', good + first + '\n', 'duplicate property record'),
            ('additional terminal empty record', good + '\n', 'invalid TSV columns'),
            ('interior empty record', prefix + '\n' + ''.join(records), 'invalid TSV columns'),
        ]
        # CR, VT, FF and NEL were split as lines by the reviewed parser. Other
        # raw controls are also never emitted by Task 069's escape_tsv().
        cases += [(f'raw control U+{ord(c):04X}', good.replace('"before"', '"be' + c + 'fore"'),
                   'raw TSV control character') for c in ('\r', '\v', '\f', '\x85', '\x00', '\x1f', '\x7f', '\x9f')]
        for label, tsv, error in cases:
            with self.subTest(case=label), self.assertRaisesRegex(ValueError, error):
                package.package(tsv, [graph, graph])

    def test_escape_boundary(self):
        self.assertEqual(package.unescape(r'quote"\\\t\n\r\u{1b}雪</script>'),
                         'quote"\\\t\n\r\x1b雪</script>')
        self.assertEqual(package.unescape(r'\\u{1b}'), r'\u{1b}')
        for cell in ['\\', r'\x', r'\u{bad!}']:
            with self.assertRaises(ValueError):
                package.unescape(cell)

    def test_grouping_missing_empty_and_exact_values(self):
        graph = dict(schema=dict(version=None), types=[dict(qname=['urn:a', 'Same'], url='types/000000.html'),
                            dict(qname=['urn:b', 'Same'], url='types/000001.html')], messages=[])
        tsv = ('category\tnamespace\tdeclaration\tmember_or_property\tchange\tbefore\tafter\n'
               'METADATA\t\t\tschema_version\tVALUE\tabsent\tabsent\n'
               'SUMMARY\t\tTYPE\tcount\tVALUE\t2\t2\n'
               'SUMMARY\t\tTYPE\tadded\tCOUNT\t\t0\n'
               'SUMMARY\t\tTYPE\tremoved\tCOUNT\t\t0\n'
               'SUMMARY\t\tTYPE\tchanged\tCOUNT\t\t1\n'
               'SUMMARY\t\tTYPE\tunchanged\tCOUNT\t\t1\n'
               'SUMMARY\t\tMESSAGE\tcount\tVALUE\t0\t0\n' +
               ''.join('SUMMARY\t\tMESSAGE\t' + s + '\tCOUNT\t\t0\n' for s in ('added', 'removed', 'changed', 'unchanged')) +
               'TOTAL\t\t\t\tMEMBER_CHANGED\t\t2\n'
               'TYPE\turn:a\tSame\t"Field"/cardinality\tMEMBER_CHANGED\t0..18446744073709551615\t1..unbounded\n'
               'TYPE\turn:a\tSame\t"Field"/wire_namespace\tMEMBER_CHANGED\tabsent\t""\n')
        model = package.package(tsv, [graph, graph])
        changed, unchanged = model['declarations']
        self.assertEqual(changed['status'], 'Changed')
        self.assertEqual(len(changed['changes']), 2)
        self.assertEqual(changed['changes'][0]['before'], '0..18446744073709551615')
        self.assertIsNone(changed['changes'][1]['before'])
        self.assertEqual(changed['changes'][1]['after'], '""')
        self.assertEqual(unchanged['status'], 'Unchanged')
        with self.assertRaisesRegex(ValueError, 'status summary mismatch'):
            package.package(tsv.replace('unchanged\tCOUNT\t\t1', 'unchanged\tCOUNT\t\t2'), [graph, graph])


if __name__ == '__main__':
    unittest.main()