"""Generate and execute a real pinned-UCI codec using owned input data."""
import importlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class OwnedUci(unittest.TestCase):
    def test_generated_dlz(self):
        root = Path(__file__).resolve().parents[3]
        fixtures = Path(__file__).parent / 'fixtures/test_owned_uci'
        with tempfile.TemporaryDirectory(prefix='ci004-python-uci-') as temporary:
            subprocess.run([str(root / 'target/release/ams-gra-codegen-oms'), 'service-generate',
                            '--schema', os.environ['AMS_GRA_UCI_2_5_ROOT'],
                            '--contract', str(fixtures / 'service.yaml'), '--language', 'python',
                            '--world', 'closed-schema', '--with-codec', '--output', temporary + '/ci004_uci'],
                           check=True, capture_output=True)
            sys.path.insert(0, temporary)
            try:
                codec = importlib.import_module('ci004_uci.service_codec')
                value = json.loads((fixtures / 'dlz.json').read_text())
                encoded = codec.encode_DLZ_MT(codec.decode_DLZ_MT(json.dumps(value)))
                self.assertEqual(json.loads(encoded), value)
                del value['MessageData']['DLZ_Data']['TimeOfIntercept']
                with self.assertRaises(codec.CodecError):
                    codec.decode_DLZ_MT(json.dumps(value))
            finally:
                sys.path.remove(temporary)
                for name in list(sys.modules):
                    if name == 'ci004_uci' or name.startswith('ci004_uci.'):
                        del sys.modules[name]