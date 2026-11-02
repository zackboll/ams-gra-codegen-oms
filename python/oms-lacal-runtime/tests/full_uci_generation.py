"""Job-local PY-016 generation reuse; never cache broker or test execution."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory
import time
import xml.etree.ElementTree as ET

from support import ROOT

spec = importlib.util.spec_from_file_location('py016_pins', ROOT / 'scripts/task101_evidence.py')
pins = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pins)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(directory):
    files = {}
    for path in sorted(directory.rglob('*')):
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            raise ValueError('redirected/nonregular generated input')
        if path.is_file():
            files[path.relative_to(directory).as_posix()] = digest(path)
    required = {'__init__.py', 'model.py', 'service_api.py', 'service_codec.py'}
    if not required <= files.keys():
        raise ValueError('incomplete generated Python package')
    return files


def generate(generator, schema, contract, release, world, output):
    """Copy verified source-bound output, or generate and atomically publish it.

    A slot is immutable within one harness invocation. Changed inputs or damaged
    output are errors, not cache misses that could hide invalid provenance.
    Without the explicit harness-owned directory, retain uncached generation.
    """
    start = time.monotonic()
    command = [str(generator), 'service-generate', '--schema', str(schema),
               '--contract', str(contract), '--language', 'python',
               '--world', world, '--with-codec', '--output']
    cache = os.environ.get('AMS_GRA_PY016_GENERATION_CACHE')
    if not cache:
        subprocess.run([*command, str(output)], check=True, capture_output=True)
        return dict(cache='disabled', seconds=time.monotonic() - start)

    # Use the same checked-in pins as mandatory source-bound qualification.
    version = release.replace('.', '')
    if version not in pins.PINS or world not in ('closed-schema', 'open-extensions'):
        raise ValueError('unregistered full-UCI generation selection')
    include = schema.with_name(f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd')
    references = [item.attrib for item in ET.parse(schema).getroot()
                  if item.tag in ('{http://www.w3.org/2001/XMLSchema}include',
                                  '{http://www.w3.org/2001/XMLSchema}import',
                                  '{http://www.w3.org/2001/XMLSchema}redefine')]
    if (digest(schema) != pins.PINS[version] or digest(include) != pins.INCLUDES[version]
            or references != [dict(schemaLocation=include.name)]):
        raise ValueError('wrong pinned full-UCI root/includes')
    source = subprocess.check_output(['git', 'rev-parse', 'HEAD', 'HEAD^{tree}'],
                                     cwd=ROOT, text=True).splitlines()
    if (len(source) != 2 or source[0] != os.environ.get('GITHUB_SHA', source[0])
            or subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)):
        raise ValueError('dirty/mismatched full-UCI generation source')
    identity = dict(version=1, source_sha=source[0], source_tree=source[1],
                    run_id=os.environ.get('GITHUB_RUN_ID', 'local'),
                    run_attempt=os.environ.get('GITHUB_RUN_ATTEMPT', '1'),
                    generator_sha256=digest(Path(generator)),
                    schema_sha256=digest(schema), includes={include.name: digest(include)},
                    contract_sha256=digest(contract), release=release, world=world,
                    language='python', with_codec=True)
    base = Path(cache)
    if base.is_symlink() or not base.is_dir() or base.resolve().is_relative_to(ROOT):
        raise ValueError('generation cache must be an existing job-local directory outside source')
    slot = base / (version + '-' + world)
    hit = slot.exists()
    if not hit:
        with TemporaryDirectory(prefix='.generation-', dir=base) as temporary:
            staging = Path(temporary)
            generated = staging / 'generated'
            subprocess.run([*command, str(generated)], check=True, capture_output=True)
            files = inventory(generated)
            # Recheck all identities after the expensive operation, before publish.
            if (digest(Path(generator)) != identity['generator_sha256']
                    or digest(schema) != identity['schema_sha256']
                    or digest(include) != identity['includes'][include.name]
                    or digest(contract) != identity['contract_sha256']
                    or subprocess.check_output(['git', 'rev-parse', 'HEAD', 'HEAD^{tree}'],
                                               cwd=ROOT, text=True).splitlines() != source
                    or subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)):
                raise ValueError('generation inputs changed during execution')
            (staging / 'manifest.json').write_text(json.dumps(dict(identity=identity, files=files),
                                                             sort_keys=True) + '\n')
            staging.rename(slot)
    if (slot.is_symlink() or not slot.is_dir()
            or {p.name for p in slot.iterdir()} != {'manifest.json', 'generated'}
            or (slot / 'manifest.json').is_symlink() or (slot / 'generated').is_symlink()):
        raise ValueError('incomplete/redirected generation cache entry')
    manifest = json.loads((slot / 'manifest.json').read_text())
    files = inventory(slot / 'generated')
    if manifest != dict(identity=identity, files=files):
        raise ValueError('generation cache identity/content mismatch')
    shutil.copytree(slot / 'generated', output)
    if inventory(output) != files:
        raise ValueError('generation cache copy mismatch')
    return dict(cache='hit' if hit else 'miss', seconds=time.monotonic() - start,
                identity=identity, files=files)