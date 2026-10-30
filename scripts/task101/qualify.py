#!/usr/bin/env python3
"""Real-UCI DLZ generated Ada model/API/codec and pinned-Sleet campaign."""
import argparse
import hashlib
import importlib.util
import itertools
import json
from pathlib import Path
import shutil
import sys
import xml.etree.ElementTree as ET

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
PINS = {'25': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
        '26': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
DEPS = {'aht': ('https://github.com/ovenpasta/aht.git', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
        'websocket-ada': ('https://github.com/kokhlo/websocket-ada.git', '765d9cd612838eb00645037977c8cc9af7171c3e')}
TOOL_IDENTITIES = ('for tool in gcc gnat gprbuild ld; do path=$(command -v "$tool"); '
                   'printf "%s\\n" "$path"; sha256sum "$path"; done; gcc -print-prog-name=ld')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['scratch', 'setup', 'generator', 'sleet', 'uci25', 'uci26']:
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--compiler', required=True, choices=['14', '16'])
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    ignore = shutil.ignore_patterns('obj', 'obj-*', 'bin', 'lib', '.git', '__pycache__', '*.o', '*.ali')
    spec = importlib.util.spec_from_file_location('task101_task098_qualify', ROOT / 'scripts/task098/qualify.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    real_spec = importlib.util.spec_from_file_location('task101_task098_real', ROOT / 'scripts/task098/real.py')
    real = importlib.util.module_from_spec(real_spec)
    real_spec.loader.exec_module(real)
    campaign = module.Campaign(scratch, args.setup.resolve(), args.compiler)
    campaign.provision()
    campaign.run('source', ['git', 'rev-parse', 'HEAD', 'HEAD^{tree}'])
    campaign.run('source-status', ['git', 'status', '--porcelain'])
    campaign.run('platform', campaign.shell(['sh', '-ec', 'test "$(uname -m)" = x86_64; test "$(getconf GNU_LIBC_VERSION)" = "glibc 2.41"; uname -m; getconf GNU_LIBC_VERSION; gcc -v; ld --version; ' + TOOL_IDENTITIES]))
    generator = args.generator.resolve()
    hashes = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
              for folder in ['scripts/task101', 'tests/ada-real-uci-dlz', 'tests/fixtures/task101', 'ada']
              for p in (ROOT / folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts}
    for relative in ['scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
                     'tests/ada-time-zulu-codec/time_consumer.adb', 'scripts/task101_evidence.py']:
        hashes[relative] = hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()
    (scratch / 'generator.sha256').write_text(hashlib.sha256(generator.read_bytes()).hexdigest() + '\n')
    for name, (url, revision) in DEPS.items():
        directory = scratch / 'deps' / name
        campaign.run(name + '-clone', ['git', 'clone', '--quiet', url, directory])
        campaign.run(name + '-pin', ['git', '-C', directory, 'checkout', '--quiet', '--detach', revision])
        assert campaign.run(name + '-identity', ['git', '-C', directory, 'rev-parse', 'HEAD']).strip() == revision
        assert not campaign.run(name + '-clean', ['git', '-C', directory, 'status', '--porcelain'])
    campaign.env.update(OMS_WS_AHT_SOURCE=str(scratch / 'deps/aht'),
                        OMS_WS_CODEC_SOURCE=str(scratch / 'deps/websocket-ada'), PYTHONDONTWRITEBYTECODE='1')
    cells = []
    schema_sources = {}
    for release, schema in [('25', args.uci25.resolve()), ('26', args.uci26.resolve())]:
        assert hashlib.sha256(schema.read_bytes()).hexdigest() == PINS[release]
        closure = {}
        def collect(path):
            if path in closure:
                return
            closure[path] = hashlib.sha256(path.read_bytes()).hexdigest()
            for child in ET.parse(path).getroot():
                if child.tag in ['{http://www.w3.org/2001/XMLSchema}include', '{http://www.w3.org/2001/XMLSchema}import']:
                    collect(path.parent / child.attrib['schemaLocation'])
        collect(schema)
        assert len(closure) == 2, 'unexpected authentic schema closure'
        schema_sources[release] = {p.name: digest for p, digest in closure.items()}
        (scratch / f'uci{release}').mkdir()
        for path in closure:
            shutil.copyfile(path, scratch / f'uci{release}' / path.name)
        contract = scratch / f'uci{release}-service.yaml'
        original = ROOT / f'tests/fixtures/task101/uci{release}-service.yaml'
        contract.write_text(original.read_text() + '''      - id: publish
        kind: oms_message
        direction: output
        mandate: mandatory
        message: DLZ
        topic: task098-dlz
        timing:
          kind: asynchronous
''')
        common = ['--schema', schema, '--contract', contract, '--language', 'ada', '--world', 'closed-schema']
        runtime = ['--with-codec', '--ada-codec-runtime', 'linux-x86_64-glibc-2.41-lossless-v1']
        report = campaign.run(f'{release}-check', [generator, 'service-check', *common, *runtime])
        assert 'selected type closure: 54\n' in report and 'codec renderable emitted declarations: 53/53\ncodec status: READY\n' in report
        generated = scratch / f'{release}-generated'
        campaign.run(f'{release}-generate', [generator, 'service-generate', *common, *runtime, '--output', generated])
        baseline = scratch / f'{release}-original-generated'
        original_common = ['--schema', schema, '--contract', original, '--language', 'ada', '--world', 'closed-schema']
        campaign.run(f'{release}-original', [generator, 'service-generate', *original_common, *runtime, '--output', baseline])
        model = scratch / f'{release}-model'
        campaign.run(f'{release}-model', [generator, 'service-generate', *common, '--output', model])
        for path in model.iterdir():
            assert path.read_bytes() == (generated / path.name).read_bytes(), 'codec changed model/API'
        for path in baseline.iterdir():
            if path.name.startswith(('programs', 'service_codec')):
                assert path.read_bytes() == (generated / path.name).read_bytes(), 'adding output endpoint changed model/codec'
        codec_cells = real.run_documents(campaign, release, generated)
        assert len(codec_cells) == 3 and all((c['documents'], c['accepted'], c['rejected']) == (26, 10, 16)
                                           for c in codec_cells)
        for policy, optimization in itertools.product(['Normal', 'Check', 'Ignore'], ['0', '2']):
            cell = f'uci{release}-{policy}-O{optimization}'
            build = scratch / cell
            shutil.copytree(ROOT / 'ada', build / 'ada', ignore=ignore)
            shutil.copytree(generated, build / 'generated')
            shutil.copyfile(ROOT / 'tests/ada-real-uci-dlz/dlz_consumer.adb', build / 'dlz_consumer.adb')
            (build / 'consumer.gpr').write_text('''with "ada/oms-lacal-runtime/oms_lacal_websocket";
with "ada/oms-json-support/ams_gra_oms_json_lossless";
project Consumer is
   for Source_Dirs use ("generated", ".");
   for Object_Dir use "obj";
   for Exec_Dir use "bin";
   for Main use ("dlz_consumer.adb");
   package Compiler is
      for Default_Switches ("Ada") use ("-gnat2022", "-gnato");
   end Compiler;
end Consumer;
''')
            adc = build / 'policy.adc'
            adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
            campaign.run(cell + '-build', campaign.shell(['gprbuild', '-p', '-j4', '-P', 'consumer.gpr',
                '-cargs:Ada', '-gnato', f'-O{optimization}', f'-gnatec={adc}']), build)
            output = campaign.run(cell + '-sleet', [sys.executable, ROOT / 'scripts/task101/sleet.py',
                '--checkout', args.sleet.resolve(), '--consumer', build / 'bin/dlz_consumer', '--schema', schema,
                '--release', release, '--evidence', build / 'sleet'])
            assert f'TASK101_REAL_SLEET_CELL_COMPLETE {release} handlers=11 decode_rejected=1 witness=1\n' in output
            cells.append(dict(release=release, policy=policy, optimization=optimization))
            print('TASK101_CELL_COMPLETE', campaign.version, cell, flush=True)
    assert all(hashlib.sha256((ROOT / p).read_bytes()).hexdigest() == digest for p, digest in hashes.items())
    (scratch / 'input-sources.json').write_text(json.dumps(hashes, indent=2) + '\n')
    (scratch / 'schema-sources.json').write_text(json.dumps(schema_sources, indent=2) + '\n')
    sources = {str(p.relative_to(scratch)): hashlib.sha256(p.read_bytes()).hexdigest()
               for p in scratch.rglob('*') if p.is_file() and p.suffix in ['.ads', '.adb', '.gpr', '.adc', '.xsd', '.yaml']}
    (scratch / 'sources.json').write_text(json.dumps(sources, indent=2) + '\n')
    (scratch / 'result.json').write_text(json.dumps(dict(completed=True, compiler=campaign.version,
        cells=cells, steps=campaign.steps, real_uci=True, real_sleet=True), indent=2) + '\n')
    print('TASK101_QUALIFICATION_COMPLETE', campaign.version, len(cells), flush=True)


if __name__ == '__main__':
    main()