"""Full unchanged pinned schemas, authentic global messages, executable goldens."""
import hashlib
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class CompleteWhitespaceVisibleServices(unittest.TestCase):
    def test_ready_complete_services(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py016_measure', root / 'scripts/measure-py016-uci.py')
        measure = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(measure)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        fixture = root / 'tests/fixtures/python-whitespace-visible'
        with tempfile.TemporaryDirectory(prefix='py016-full-uci-') as temporary:
            path = Path(temporary)
            for release, (revision, digest) in measure.driver.PINS.items():
                schema = Path(os.environ['AMS_GRA_UCI_' + release.replace('.', '_') + '_ROOT'])
                self.assertEqual(hashlib.sha256(schema.read_bytes()).hexdigest(), digest)
                include = schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')
                self.assertEqual(hashlib.sha256(include.read_bytes()).hexdigest(), measure.INCLUDES[release])
                for message in ['Log', 'PositionReport', 'SubsystemStream', 'SurvivabilityRiskLevel']:
                    contract = path / f'{release}-{message}.yaml'
                    contract.write_text(measure.driver.contract(message, release))
                    for world in ['closed-schema', 'open-extensions']:
                        destination = path / ('v' + release.replace('.', '') + '_' + message + '_' + world.replace('-', '_'))
                        result = measure.driver.check(generator, schema, contract, world, True, destination)
                        self.assertEqual(result['exit'], 0, result)
                        golden = fixture / 'goldens' / f'{release}-{message}.json'
                        for optimized in [False, True]:
                            for style in ['standalone', 'package']:
                                result = subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                                    [str(fixture / 'full_service_probe.py'), str(destination), style,
                                     release.replace('.', ''), message, str(golden)], capture_output=True, text=True)
                                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                                self.assertIn('Ran 1 test', result.stderr)