"""PY-019 complete unchanged primary service, never an AO READY assertion."""
import hashlib
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest


class AbstractValuesUci(unittest.TestCase):
    def test_primary_service_models_api_codecs_and_open_refusal(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py019_pins', root / 'scripts/measure-py017-uci.py')
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        with TemporaryDirectory(prefix='py019-pinned-') as temporary:
            path = Path(temporary)
            for release, (_, digest) in pins.driver.PINS.items():
                schema = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
                self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), digest)
                include = schema.with_name('UCI_SecurityMarkings_v2_' + release[-1] + '_0.xsd')
                self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), pins.INCLUDES[release])
                contract = path / (release + '.yaml')
                contract.write_text(pins.driver.contract('DataRecordListManagementRequest', release))
                for codec in [False, True]:
                    destination = path / ('primary_' + release.replace('.', '_') + '_' + str(codec))
                    result = pins.driver.check(generator, schema, contract, 'closed-schema', codec, destination)
                    self.assertEqual(result['exit'], 0, result)
                    self.assertEqual({p.name for p in destination.iterdir()},
                        {'model.py', 'service_api.py'} | ({'service_codec.py'} if codec else set()))
                    for optimized in [False, True]:
                        for style in ['standalone', 'package']:
                            executed = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                [str(root / 'tests/fixtures/python-abstract-values/uci_probe.py'),
                                 str(destination), style, release, 'codec' if codec else 'model'],
                                capture_output=True, text=True)
                            self.assertEqual(executed.returncode, 0, executed.stdout + executed.stderr)
                            self.assertIn('Ran 6 tests' if codec else 'Ran 3 tests', executed.stderr)
                    open_output = path / ('open_' + release.replace('.', '_') + '_' + str(codec))
                    rejected = pins.driver.check(generator, schema, contract, 'open-extensions', codec, open_output)
                    self.assertEqual(rejected['exit'], 1, rejected)
                    self.assertIn('abstract value ManagedListBaseType is not closed under open-extensions generation',
                                  rejected['stdout'])
                    self.assertFalse(open_output.exists())