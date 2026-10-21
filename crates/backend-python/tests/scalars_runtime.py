from dataclasses import fields, replace
import unittest


class ScalarModelTests(unittest.TestCase):
    def test_named_scalars_and_aliases(self):
        self.assertTrue(Flag(True).value)
        self.assertEqual(Text(''), Text(''))
        self.assertEqual(Flag_Alias(Flag(False)).value, Flag(False))
        self.assertEqual(Flag_Chain(Flag_Alias(Flag(True))).value.value.value, True)
        self.assertEqual(Direct_Alias('abc').value, 'abc')
        for constructor, values in ((Flag, (0, 1, 'true', None)),
                                    (Text, (0, True, None)),
                                    (Flag_Alias, (True, 1, Text('true'))),
                                    (Flag_Chain, (True, Flag(True))),
                                    (Direct_Alias, (1, None))):
            for value in values:
                with self.assertRaises(TypeError):
                    constructor(value)

    def test_signed_and_unsigned(self):
        for value in (-128, 127):
            self.assertEqual(Signed(value).value, value)
        for value in (-129, 128):
            with self.assertRaises(ValueError):
                Signed(value)
        for constructor, maximum in ((Unsigned, 2**32-1), (Unsigned64, 2**64-1)):
            for value in (0, maximum):
                self.assertEqual(constructor(value).value, value)
            for value in (-1, maximum+1):
                with self.assertRaises(ValueError):
                    constructor(value)
        for constructor in (Signed, Unsigned, Unsigned64):
            for value in (True, False, 1.0, '1'):
                with self.assertRaises(TypeError):
                    constructor(value)
        self.assertEqual(Unbounded_Unsigned(2**100).value, 2**100)
        self.assertEqual(Unbounded_Unsigned(0).value, 0)
        with self.assertRaises(ValueError):
            Unbounded_Unsigned(-1)
        with self.assertRaises(TypeError):
            Unbounded_Unsigned(True)

    def test_exact_enumeration_values(self):
        for value in ('3-D', 'class', 'a"b\\c', 'café'):
            self.assertEqual(State(value).value, value)
        self.assertEqual(State.schema_3_D.value, '3-D')
        self.assertEqual(State.schema_class.value, 'class')

    def test_record_direct_bounds_required_repetition_and_nested_record(self):
        base = Sample(schema_class=True, title=Text('hello'), state=State('class'),
                      count=0, values=(Signed(-128), Signed(127)))
        self.assertIsNone(base.child)
        self.assertEqual(replace(base, child=Empty()).child, Empty())
        for count in (0, 255):
            self.assertEqual(replace(base, count=count).count, count)
        for count in (-1, 256):
            with self.assertRaises(ValueError):
                replace(base, count=count)
        for count in (True, False, 1.0):
            with self.assertRaises(TypeError):
                replace(base, count=count)
        for values in ((), (Signed(0),), (Signed(0),)*4):
            with self.assertRaises(ValueError):
                replace(base, values=values)
        self.assertEqual(len(replace(base, values=(Signed(0),)*3).values), 3)
        for kwargs in ({'schema_class': 1}, {'title': 'hello'}, {'state': 'class'},
                       {'values': (1, 2)}, {'values': [Signed(0)]*2}, {'child': 'bad'}):
            with self.assertRaises(TypeError):
                replace(base, **kwargs)
        with self.assertRaises(TypeError):
            Sample(schema_class=True, title=Text(''), state=State('class'), count=0)
        self.assertEqual(fields(Sample)[0].metadata['schema_name'], 'class')
        self.assertEqual(fields(Sample)[0].metadata['schema_namespace'], 'urn:test:python')


unittest.main()