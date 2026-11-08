"""PY-018 isolated unchanged pinned UCI profiles; not full AO service coverage."""
import importlib.util
import os
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


class Unicode31Uci(unittest.TestCase):
    def test_authentic_facets_models_and_codecs_without_choices(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py018_uci', root / 'scripts/check-py018-uci.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        with TemporaryDirectory(prefix='py018-real-profiles-') as temporary:
            rows = module.execute_profiles(generator, Path(temporary))
            self.assertEqual(len(rows), 4)