#!/usr/bin/env python3
"""Generate python-uci-client UCI bindings into external build storage.

Runs the production generator exactly as

    service-generate --language python --world closed-schema --with-codec

on this application's contract for the selected release. The schema root and
its SecurityMarkings include must have the SHA-256 identities pinned by
scripts/fetch-pinned-uci-<release>.sh. Generated source is never written into
the repository. A build manifest records the release and the digests of the
three actual generated files; python_uci_client.py refuses any mismatch.

Without --with-log the RDMA-only contract and the PY-OMS-020 manifest (format
1) are used unchanged. With --with-log the combined RDMA + Log contract of the
same release is used; one model.py/service_codec.py/service_api.py then holds
both messages and the manifest (format 2) lists both selected messages.

Usage:
  generate_bindings.py --schema-release {2.5|2.6} --schema ROOT_XSD --output DIR
                       [--with-log] [--generator PATH]
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

APP = Path(__file__).resolve().parent
REPOSITORY = APP.parents[1]
MANIFEST = 'python-uci-client-build.json'
GENERATED = ('model.py', 'service_api.py', 'service_codec.py')
# Same source identities as scripts/fetch-pinned-uci-2.5.sh / -2.6.sh:
#   v2.5: 093610b7753944059360d3236770ab446d039556
#   v2.6: 78eb61b6112c8bffa40820c33124b57787fc5bd9
PINS = {
    '2.5': dict(schema_version='002.5.0',
                root='ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
                include=('UCI_SecurityMarkings_v2_5_0.xsd',
                         '4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b')),
    '2.6': dict(schema_version='002.6.0',
                root='af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b',
                include=('UCI_SecurityMarkings_v2_6_0.xsd',
                         'ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88')),
}


class BuildError(Exception):
    pass


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def contract(release, with_log=False):
    suffix = '-rdma-log-service.yaml' if with_log else '-service.yaml'
    return APP / 'contracts' / ('uci' + release.replace('.', '') + suffix)


# Selected messages of the combined contract, in contract order:
# OWP global element, generated payload class, generated contract topic.
COMBINED_MESSAGES = (
    dict(message='RDMA_InitializeSetup', payload_type='RDMA_InitializeSetupMT', topic='uci.rdma'),
    dict(message='Log', payload_type='LogMT', topic='uci.log'),
)


def default_generator():
    return os.environ.get('AMS_GRA_GENERATOR',
                          str(REPOSITORY / 'target/release/ams-gra-codegen-oms'))



def _check_inputs(release, schema, output, generator):
    if release not in PINS:
        raise BuildError('unsupported schema release; use 2.5 or 2.6')
    pin = PINS[release]
    schema = Path(schema)
    if not schema.is_file():
        raise BuildError(f'schema root not found: {schema}; fetch it with '
                         f'scripts/fetch-pinned-uci-{release}.sh')
    schema = schema.resolve()
    if digest(schema) != pin['root']:
        raise BuildError(f'{schema.name} is not the pinned UCI {release} root (SHA-256 '
                         'mismatch); --schema and --schema-release must select one release')
    include = schema.with_name(pin['include'][0])
    if not include.is_file() or digest(include) != pin['include'][1]:
        raise BuildError(f'pinned UCI {release} include {include.name} is missing or modified')
    generator = Path(generator)
    if not generator.is_file() or not os.access(generator, os.X_OK):
        raise BuildError(f'generator not found: {generator}; build it with '
                         'cargo build --release --locked -p ams-gra-codegen-oms')
    output = Path(os.path.abspath(output))
    if output.is_symlink():
        raise BuildError(f'output must be a new or empty directory: {output}')
    # Judge, and later write to, the real destination: a symlinked (even
    # dangling) ancestor must not alias the repository. REPOSITORY is resolved.
    try:
        resolved = output.resolve()
    except (OSError, RuntimeError):  # symlink loop: RuntimeError on Python 3.11
        resolved = None
    # Python 3.13 instead returns a loop unresolved; a real path has no symlinks.
    if resolved is None or any(path.is_symlink() for path in (resolved, *resolved.parents)):
        raise BuildError(f'cannot resolve output directory: {output}')
    output = resolved
    if output == REPOSITORY or REPOSITORY in output.parents:
        raise BuildError('refusing to write generated UCI bindings inside the repository')
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise BuildError(f'output must be a new or empty directory: {output}')
    return pin, schema, output, generator


def generate(release, schema, output, generator, with_log=False):
    """Generate, verify and atomically publish one release's bindings."""
    pin, schema, output, generator = _check_inputs(release, schema, output, generator)
    selected = contract(release, with_log)
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix='.python-uci-client-', dir=output.parent))
    try:
        target = staging / 'generated'
        result = subprocess.run(
            [str(generator), 'service-generate', '--schema', str(schema),
             '--contract', str(selected), '--language', 'python',
             '--world', 'closed-schema', '--with-codec', '--output', str(target)],
            capture_output=True, text=True)
        if result.returncode != 0:
            raise BuildError('service-generate failed:\n' + result.stdout + result.stderr)
        produced = sorted(path.name for path in target.iterdir())
        if produced != sorted(GENERATED):
            raise BuildError('unexpected generated file set: ' + ', '.join(produced))
        if with_log:
            # Combined manifest: both selected messages are named explicitly.
            identity = dict(format=2, with_log=True,
                            messages=[dict(item) for item in COMBINED_MESSAGES])
        else:
            # PY-OMS-020 manifest, unchanged.
            identity = dict(format=1, message='RDMA_InitializeSetup',
                            payload_type='RDMA_InitializeSetupMT')
        manifest = dict(
            application='python-uci-client', release=release,
            schema_version=pin['schema_version'], language='python',
            world='closed-schema', with_codec=True,
            contract=selected.relative_to(REPOSITORY).as_posix(),
            contract_sha256=digest(selected), schema_sha256=pin['root'],
            include={pin['include'][0]: pin['include'][1]},
            generator_sha256=digest(generator),
            files={name: digest(target / name) for name in GENERATED}, **identity)
        (target / MANIFEST).write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
        if output.exists():
            output.rmdir()
        target.rename(output)
    finally:
        shutil.rmtree(staging, ignore_errors=True)
    return output


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog='generate_bindings.py', allow_abbrev=False,
        description='Generate RDMA_InitializeSetup (optionally + Log) Python bindings '
                    'for python-uci-client.')
    parser.add_argument('--schema-release', required=True, choices=sorted(PINS))
    parser.add_argument('--with-log', action='store_true',
                        help='also select UCI Log (topic uci.log) for python_uci_client.py --with-log')
    parser.add_argument('--schema', required=True,
                        help='pinned UCI_MessageDefinitions root XSD for that release')
    parser.add_argument('--output', required=True,
                        help='new or empty directory outside the repository')
    parser.add_argument('--generator', default=default_generator(),
                        help='production ams-gra-codegen-oms binary '
                             '(default: $AMS_GRA_GENERATOR or target/release in this checkout)')
    args = parser.parse_args(argv)
    try:
        output = generate(args.schema_release, args.schema, args.output, args.generator,
                          with_log=args.with_log)
    except (BuildError, OSError) as error:
        print(f'generate_bindings.py: error: {error}', file=sys.stderr)
        return 1
    selection = 'RDMA_InitializeSetup + Log' if args.with_log else 'RDMA_InitializeSetup'
    print(f'GENERATED UCI {args.schema_release} {selection} bindings: {output}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
