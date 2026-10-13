"""Independent checks of the Task 069 transport, not a semantic comparator."""
import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location('comparison_package',
    pathlib.Path(__file__).resolve().parents[1] / 'site/compare/package.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class TransportTests(unittest.TestCase):
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