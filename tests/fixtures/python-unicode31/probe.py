"""Checked model/codec expectations, usable against synthetic or real profiles."""
from dataclasses import FrozenInstanceError
import importlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest

dest = Path(sys.argv[1])
style = sys.argv[2]
sys.path.insert(0, str(dest if style == 'standalone' else dest.parent))
prefix = '' if style == 'standalone' else dest.name + '.'
m = importlib.import_module(prefix + 'model')
c = importlib.import_module(prefix + 'service_codec') if (dest / 'service_codec.py').exists() else None
spec = importlib.util.spec_from_file_location('py018_cases', Path(__file__).with_name('cases.py'))
vectors = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vectors)
carriers = [m.NITF_DateAndTimeType, m.NITF_DateType, m.NITF_MSTGTA_TargetLocationType]
samples = ['2፩𝟎١02312359', '2፩𝟎١0231', '+1፩.𝟎١２345-17፩.123456']


def key(name):
    namespace = m.Payload.__schema_name__[0]
    return name if namespace == 'https://www.vdl.afrl.af.mil/programs/oam' else '{' + namespace + '}' + name


class Unicode31(unittest.TestCase):
    def test_independent_corpus(self):
        cases = vectors.cases()
        for profile, text, expected in cases:
            carrier = carriers[profile]
            if expected:
                self.assertEqual(carrier(text).value, text)
            else:
                with self.assertRaises(ValueError, msg=repr((profile, text))):
                    carrier(text)
        print(f'PY018 independent lexical cases={len(cases)}', flush=True)

    def test_exact_types_storage_and_nonmutation(self):
        class Text(str):
            pass
        for carrier, sample in zip(carriers, samples):
            for wrong in [None, 1, True, sample.encode(), Text(sample)]:
                with self.assertRaises(TypeError):
                    carrier(wrong)
            with self.assertRaises(TypeError):
                carrier()
            value = carrier(sample)
            self.assertFalse(hasattr(value, '__dict__'))
            with self.assertRaises(FrozenInstanceError):
                value.value = 'bad'
            for bad in ['\ud800', '\udfff']:
                with self.assertRaises(ValueError):
                    carrier(sample[:1] + bad + sample[2:])
            carrier.__post_init__(value)
            self.assertEqual(value.value, sample)
            if c:
                symbol = carrier.__name__
                encode = getattr(c, '_encode_' + symbol)
                decode = getattr(c, '_decode_' + symbol)
                self.assertEqual(encode(value, '$.value'), sample)
                self.assertEqual(decode(sample, '$.value'), value)
                wrong = carriers[(carriers.index(carrier) + 1) % 3](samples[(carriers.index(carrier) + 1) % 3])
                with self.assertRaises(c.CodecError):
                    encode(wrong, '$.value')
                class Subclass(carrier):
                    pass
                with self.assertRaises(c.CodecError):
                    encode(Subclass(sample), '$.value')
                for forged in [object.__new__(carrier), carrier(sample)]:
                    if hasattr(forged, 'value'):
                        object.__setattr__(forged, 'value', 'bad')
                    with self.assertRaises(c.CodecError):
                        encode(forged, '$.value')

    def entry(self):
        return m.EntryType(date=carriers[1](samples[1]), time=carriers[0](samples[0]),
                       location=carriers[2](samples[2]))

    def payload(self):
        return m.Payload(required=carriers[1](samples[1]), entry=(self.entry(),),
                         selected=m.Selection.Entry(self.entry()))

    def test_nested_repeated_choices_paths_and_json(self):
        if not hasattr(m, 'Selection'):
            value = m.Payload(entry=(self.entry(),))
            entry = {key('Date'): samples[1], key('Time'): samples[0], key('Location'): samples[2]}
            self.assertEqual(json.loads(c.encode_Payload(value)), {key('Entry'): [entry]})
            self.assertEqual(c.decode_Payload(json.dumps({key('Entry'): [entry]})), value)
            entry[key('Date')] = 'bad'
            with self.assertRaises(c.CodecError) as raised:
                c.decode_Payload(json.dumps({key('Entry'): [entry]}))
            self.assertIn('$.' + key('Entry') + '[0].' + key('Date'), str(raised.exception))
            object.__setattr__(value.entry[0].date, 'value', 'bad')
            with self.assertRaises(c.CodecError) as raised:
                c.encode_Payload(value)
            self.assertIn('$.' + key('Entry') + '[0].' + key('Date'), str(raised.exception))
            with self.assertRaises(ValueError):
                m.Payload(entry=value.entry)
            return
        value = self.payload()
        if not c:
            return
        entry = {key('Date'): samples[1], key('Time'): samples[0], key('Location'): samples[2]}
        golden = {key('Required'): samples[1], key('Entry'): [entry],
                  key('Selected'): {key('Entry'): entry}}
        encoded = c.encode_Payload(value)
        self.assertEqual(json.loads(encoded), golden)
        self.assertIn('፩', encoded)
        self.assertEqual(c.decode_Payload(json.dumps(golden, ensure_ascii=False)), value)
        for path in [('Entry', 'Time'), ('Selected', 'Location')]:
            doc = json.loads(encoded)
            if path[0] == 'Entry':
                doc[key('Entry')][0][key(path[1])] = 'bad'
                expected_path = '$.' + key('Entry') + '[0].' + key(path[1])
                forged = self.payload()
                object.__setattr__(forged.entry[0].time, 'value', 'bad')
            else:
                doc[key('Selected')][key('Entry')][key(path[1])] = 'bad'
                expected_path = '$.' + key('Selected') + '.' + key('Entry') + '.' + key(path[1])
                forged = self.payload()
                object.__setattr__(forged.selected.value.location, 'value', 'bad')
            for operation in [lambda: c.decode_Payload(json.dumps(doc)), lambda: c.encode_Payload(forged)]:
                with self.assertRaises(c.CodecError) as raised:
                    operation()
                self.assertIn(expected_path, str(raised.exception))
        for text in ['NaN', 'Infinity', '{"Required":"20000101","Required":"20000101"}']:
            with self.assertRaises(c.CodecError):
                c.decode_Payload(text)

    def test_recursive_revalidation_without_mutating_storage(self):
        entry = self.entry()
        object.__setattr__(entry.date, 'value', 'bad')
        with self.assertRaises(ValueError):
            m.EntryType(date=entry.date, time=entry.time, location=entry.location)
        self.assertEqual(entry.date.value, 'bad')
        if hasattr(m, 'Selection'):
            with self.assertRaises(ValueError):
                m.Selection.Entry(entry)
            with self.assertRaises(TypeError):
                m.Payload(required=carriers[0](samples[0]), entry=(),
                          selected=m.Selection.Date(m.Leaf(samples[1])))
        else:
            with self.assertRaises(ValueError):
                m.Payload(entry=(entry,))


if __name__ == '__main__':
    unittest.main(argv=[sys.argv[0]])