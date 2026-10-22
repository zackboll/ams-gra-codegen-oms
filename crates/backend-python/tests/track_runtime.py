from dataclasses import FrozenInstanceError, fields, replace
import unittest


class TrackModelTests(unittest.TestCase):
    def test_numeric_boundaries(self):
        for value in (1, 65535):
            self.assertEqual(Track_Id(value).value, value)
        for value in (0, 65536, -(2**80), 2**80):
            with self.assertRaises(ValueError):
                Track_Id(value)
        for value in (True, False, 1.0, '1', None):
            with self.assertRaises(TypeError):
                Track_Id(value)

    def test_enumeration(self):
        for value in ('Unknown', 'Tentative', 'Confirmed'):
            self.assertEqual(Track_Quality(value).value, value)
        for value in ('unknown', '', 1, None):
            with self.assertRaises(ValueError):
                Track_Quality(value)

    def test_required_optional_and_cardinality(self):
        base = Track(id=Track_Id(1), quality=Track_Quality.Confirmed)
        self.assertIsNone(base.callsign)
        self.assertEqual(base.sensor_ids, ())
        for callsign in (None, '', 'EAGLE', 'é🛩'):
            self.assertEqual(replace(base, callsign=callsign).callsign, callsign)
        for count in range(9):
            sensors = tuple(range(count))
            self.assertEqual(replace(base, sensor_ids=sensors).sensor_ids, sensors)
        with self.assertRaises(ValueError):
            replace(base, sensor_ids=tuple(range(9)))
        for sensors in (None, [], [1], (True,), ('1',), (1.0,), (None,)):
            with self.assertRaises(TypeError):
                replace(base, sensor_ids=sensors)
        # Unconstrained xs:integer is genuinely arbitrary precision.
        self.assertEqual(replace(base, sensor_ids=(-2**100, 2**100)).sensor_ids,
                         (-2**100, 2**100))
        for kwargs in ({}, {'id': Track_Id(1)}, {'quality': Track_Quality.Unknown}):
            with self.assertRaises(TypeError):
                Track(**kwargs)
        for kwargs in ({'id': 1}, {'id': True}, {'id': None},
                       {'quality': 'Confirmed'}, {'quality': None},
                       {'callsign': 1}, {'callsign': False}):
            with self.assertRaises(TypeError):
                replace(base, **kwargs)
        with self.assertRaises(FrozenInstanceError):
            base.id = Track_Id(2)
        with self.assertRaises(FrozenInstanceError):
            base.id.value = 2
        class Derived(Track_Id):
            pass
        with self.assertRaises(TypeError):
            replace(base, id=Derived(1))

    def test_original_names_are_retained(self):
        self.assertEqual(Track_Id.__schema_name__, ('urn:example:oms:track', 'Track_Id'))
        self.assertEqual(Track_Quality.__schema_name__, ('urn:example:oms:track', 'Track_Quality'))
        self.assertEqual([f.metadata['schema_name'] for f in fields(Track)],
                         ['Id', 'Quality', 'Callsign', 'Sensor_Ids'])
        self.assertTrue(all(f.metadata['schema_namespace'] is None for f in fields(Track)))


unittest.main()