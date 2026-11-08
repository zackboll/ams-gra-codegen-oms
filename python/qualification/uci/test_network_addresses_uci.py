"""Full unchanged UCI RDMA generation and execution, not readiness-only support."""
import importlib.util
import os
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


class NetworkAddressesUci(unittest.TestCase):
    def test_authentic_full_rdma_models_and_codecs(self):
        root = Path(__file__).resolve().parents[3]
        spec = importlib.util.spec_from_file_location('py017_uci', root / 'scripts/check-py017-uci.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        generator = Path(os.environ.get('AMS_GRA_GENERATOR', root / 'target/release/ams-gra-codegen-oms'))
        with TemporaryDirectory(prefix='py017-full-rdma-') as temporary:
            module.execute_rdma(generator, Path(temporary))