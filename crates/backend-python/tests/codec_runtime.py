"""Independent wire vectors and adversarial generated-code execution, also -O."""
import copy
import importlib
import json
from pathlib import Path
import py_compile
import sys
from typing import get_type_hints
import unittest

directory = Path(sys.argv.pop(1)).resolve()
style = sys.argv.pop(1)
namespace = sys.argv.pop(1)
fixtures = Path(sys.argv.pop(1))
if sys.version_info < (3, 11):
    raise RuntimeError('Python 3.11+ required')
for source in directory.glob('*.py'):
    py_compile.compile(str(source), doraise=True)
sys.path.insert(0, str(directory.parent if style == 'package' else directory))
prefix = directory.name + '.' if style == 'package' else ''
c = importlib.import_module(prefix + 'service_codec')
m = importlib.import_module(prefix + 'model')


def key(name):
    return name if namespace == 'https://www.vdl.afrl.af.mil/programs/oam' else '{' + namespace + '}' + name


class CodecTests(unittest.TestCase):
    def setUp(self):
        vector = 'positive.json' if namespace == 'https://www.vdl.afrl.af.mil/programs/oam' else 'non-oam-positive.json'
        self.document = json.loads((fixtures / vector).read_text(encoding='utf-8'))
        self.expected = m.Payload(
            schema_class=m.Flag(True), signed=m.Signed(-(2**63)),
            unsigned=m.Unsigned(2**32-1), big=m.Big(12345678901234567890123456789012345678901234567890),
            state=m.State.caf_, child=m.Child(label=m.Text('λ 😀 "\\\n'), count=255),
            empty=m.Empty(), note='', values=(m.Signed(-(2**63)), m.Signed(2**63-1)),
            flags=(True, False), children=(m.Child(label=m.Text('first'), count=0),
                                          m.Child(label=m.Text('second'), count=1)))

    def reject(self, text):
        with self.assertRaises(c.CodecError):
            c.decode_Payload(text)

    def test_independent_vector_both_directions_and_type_hints(self):
        text = json.dumps(self.document, ensure_ascii=False, separators=(',', ':'))
        value = c.decode_Payload(text)
        self.assertEqual(value, self.expected)
        self.assertEqual(c.encode_Payload(self.expected), text)
        self.assertEqual(c.encode_Payload(value), text)
        self.assertIs(type(value.signed), m.Signed)
        self.assertIs(type(value.schema_class), m.Flag)
        self.assertIs(type(value.child.label), m.Text)
        self.assertIs(type(value.values), tuple)
        self.assertIs(type(value.children[0]), m.Child)
        self.assertIs(get_type_hints(c.decode_Payload)['return'], m.Payload)
        self.assertIs(get_type_hints(c.encode_Payload)['payload'], m.Payload)
        self.assertEqual(c.encode_Payload(value).encode('utf-8').decode('utf-8'), text)

    def test_order_and_optional_presence(self):
        document = dict(reversed(list(self.document.items())))
        value = c.decode_Payload(json.dumps(document))
        self.assertEqual(c.encode_Payload(value), c.encode_Payload(self.expected))
        for name in ['Note', 'Empty', 'Flags']:
            document.pop(key(name))
        value = c.decode_Payload(json.dumps(document))
        self.assertIsNone(value.note)
        self.assertIsNone(value.empty)
        self.assertEqual(value.flags, ())
        self.assertNotIn(key('Note'), json.loads(c.encode_Payload(value)))
        document[key('Flags')] = []
        self.assertEqual(c.decode_Payload(json.dumps(document)), value)
        self.assertNotIn(key('Flags'), json.loads(c.encode_Payload(value)))

    def test_negative_vector_table(self):
        for vector in json.loads((fixtures / 'negative.json').read_text()):
            with self.subTest(vector=vector['name']):
                document = copy.deepcopy(self.document)
                document[key(vector['member'])] = vector['value']
                self.reject(json.dumps(document))

    def test_missing_unknown_nested_and_type_discriminators(self):
        for name in ['class', 'Signed', 'Unsigned', 'Big', 'State', 'Child', 'Values']:
            document = self.document.copy()
            del document[key(name)]
            self.reject(json.dumps(document))
        for name in ['Label', 'Count']:
            document = copy.deepcopy(self.document)
            del document[key('Child')][key(name)]
            self.reject(json.dumps(document))
        for name, value in [('unknown', 1), ('$type', 'Payload'), ('$type', 'PrivateType')]:
            document = self.document.copy()
            document[name] = value
            self.reject(json.dumps(document))
            document = copy.deepcopy(self.document)
            document[key('Child')][name] = value
            self.reject(json.dumps(document))
        document = copy.deepcopy(self.document)
        document[key('Child')][key('Count')] = 256
        self.reject(json.dumps(document))
        document[key('Child')][key('Count')] = True
        self.reject(json.dumps(document))
        document = copy.deepcopy(self.document)
        document[key('Children')][1][key('Count')] = True
        self.reject(json.dumps(document))
        document[key('Children')] = [self.document[key('Child')]] * 3
        self.reject(json.dumps(document))
        # Python field spelling is never an alias for the element wire QName.
        document = self.document.copy()
        document['schema_class'] = document.pop(key('class'))
        self.reject(json.dumps(document))
        if namespace != 'https://www.vdl.afrl.af.mil/programs/oam':
            document = self.document.copy()
            document['class'] = document.pop(key('class'))
            self.reject(json.dumps(document))
        document = copy.deepcopy(self.document)
        document[key('Children')][1][key('Count')] = True
        with self.assertRaisesRegex(c.CodecError, r'Children\[1\].*Count'):
            c.decode_Payload(json.dumps(document))

    def test_duplicate_decoded_names_everywhere(self):
        text = json.dumps(self.document)
        wire = json.dumps(key('class'))
        escaped = '"' + ''.join('\\u%04x' % ord(ch) for ch in key('class')) + '"'
        self.reject('{' + wire + ':true,' + escaped + ':true,' + text[1:])
        self.reject('{"x":1,"\\u0078":1}')
        self.reject('{"unknown":[{"x":1,"\\u0078":2}]}')
        self.reject('{"$type":"A","\\u0024type":"A"}')
        # Same key in distinct sibling objects is not itself a duplicate.
        self.assertEqual(c._loads('[{"x":1},{"x":2}]'), [{'x':1}, {'x':2}])

    def test_syntax_constants_numbers_and_no_coercion(self):
        for text in ['', '{', '{}{}', '{} trailing', '[1,]', 'NaN', 'Infinity', '-Infinity',
                     '1.0', '1e400', '01', '+1', 'null', '[]', 'true', '"text"']:
            self.reject(text)
        for number in ['NaN', 'Infinity', '-Infinity', '1.0', '1e0', '1e400']:
            text = json.dumps(self.document)
            text = text.replace(str(-(2**63)), number, 1)
            self.reject(text)
        for value in [self.document, b'{}', None, 123]:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(value)

    def test_encoder_exact_types_and_revalidation(self):
        for value in [self.document, m.Empty(), None]:
            with self.assertRaises(c.CodecError):
                c.encode_Payload(value)
        class Derived(m.Payload):
            pass
        with self.assertRaises(c.CodecError):
            c.encode_Payload(Derived(**{f: getattr(self.expected, f) for f in
                ['schema_class','signed','unsigned','big','state','child','empty','note','values','flags','children']}))
        for field, invalid in [('signed', m.Signed(1)), ('flags', [True]), ('values', ()), ('note', 4)]:
            value = copy.deepcopy(self.expected)
            if field == 'signed':
                object.__setattr__(invalid, 'value', True)
            object.__setattr__(value, field, invalid)
            with self.assertRaises(c.CodecError):
                c.encode_Payload(value)

    def test_huge_integer_without_global_limit_changes(self):
        limit = sys.get_int_max_str_digits()
        huge = '1' + '0' * 5000
        document = self.document.copy()
        document[key('Big')] = 'HUGE'
        text = json.dumps(document, ensure_ascii=False, separators=(',', ':')).replace('"HUGE"', huge)
        value = c.decode_Payload(text)
        self.assertEqual(value.big.value, 10**5000)
        self.assertEqual(c.encode_Payload(value), text)
        self.assertEqual(sys.get_int_max_str_digits(), limit)

    def test_unicode_and_internal_errors_are_not_hidden(self):
        for text in ['{"\\ud800":1}', json.dumps({key('Note'): '\ud800'})]:
            self.reject(text)
        value = copy.deepcopy(self.expected)
        object.__setattr__(value, 'note', '\udfff')
        with self.assertRaises(c.CodecError):
            c.encode_Payload(value)
        original = m.Child
        def broken(**kwargs):
            raise RuntimeError('internal programming error')
        m.Child = broken
        try:
            with self.assertRaisesRegex(RuntimeError, 'internal programming error'):
                c.decode_Payload(json.dumps(self.document))
        finally:
            m.Child = original


if __name__ == '__main__':
    unittest.main()