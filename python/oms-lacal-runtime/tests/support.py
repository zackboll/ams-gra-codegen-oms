"""Qualification always generates actual models/API/codecs with the production CLI."""
import importlib
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[3]
OAM = 'https://www.vdl.afrl.af.mil/programs/oam'


class Generated:
    def __init__(self, namespace=OAM):
        self.temp = TemporaryDirectory(prefix='py005-generated-')
        output = Path(self.temp.name) / 'generated'
        schema = ROOT / 'tests/fixtures/python-codec' / ('oam.xsd' if namespace == OAM else 'non-oam.xsd')
        binary = Path(os.environ.get('AMS_GRA_GENERATOR', ROOT / 'target/debug/ams-gra-codegen-oms'))
        subprocess.run([str(binary), 'service-generate', '--schema', str(schema),
                        '--contract', str(ROOT / 'tests/fixtures/python-codec/service.yaml'),
                        '--language', 'python', '--world', 'closed-schema', '--with-codec',
                        '--output', str(output)], check=True, capture_output=True)
        # Each Generated instance is isolated under a distinct package name.
        name = Path(self.temp.name).name.replace('-', '_')
        renamed = Path(self.temp.name).parent / name
        Path(self.temp.name).rename(renamed)
        self.path = renamed
        sys.path.insert(0, str(renamed.parent))
        self.model = importlib.import_module(name + '.generated.model')
        self.codec = importlib.import_module(name + '.generated.service_codec')
        self.api = importlib.import_module(name + '.generated.service_api')
        self.namespace = namespace

    def close(self):
        import shutil
        shutil.rmtree(self.path)
        self.temp.cleanup()

    def registry(self):
        from oms_lacal import CodecBinding, CodecRegistry
        return CodecRegistry(CodecBinding(self.namespace, name, self.model.Payload,
                             self.codec.encode_Payload, self.codec.decode_Payload,
                             self.codec.CodecError) for name in ['SelectedReport', 'OtherReport'])

    def sample(self):
        m = self.model
        return m.Payload(schema_class=m.Flag(True), signed=m.Signed(-1), unsigned=m.Unsigned(1),
                         big=m.Big(9007199254740993), state=m.State.schema_class,
                         child=m.Child(label=m.Text('λ'), count=1), values=(m.Signed(-1),m.Signed(2)))

    def envelope(self, name='SelectedReport', value=None):
        import json
        from oms_lacal.wire import global_name
        return '{' + json.dumps(global_name(self.namespace, name)) + ':' + self.codec.encode_Payload(value or self.sample()) + '}'