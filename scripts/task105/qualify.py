#!/usr/bin/env python3
"""Real UCI 2.6 Survivability generated Ada/API/codec pinned-Sleet campaign."""
import argparse
import hashlib
import importlib.util
import itertools
import json
import os
from pathlib import Path
import shutil
import sys
import tomllib
import xml.etree.ElementTree as ET

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
PINS = {'25': 'ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27',
        '26': 'af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b'}
DEPS = {'aht': ('https://github.com/ovenpasta/aht.git', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
        'websocket-ada': ('https://github.com/kokhlo/websocket-ada.git', '765d9cd612838eb00645037977c8cc9af7171c3e')}
TOOL_IDENTITIES = ('for tool in gcc gnat gprbuild ld; do path=$(command -v "$tool"); '
                   'printf "%s\\n" "$path"; sha256sum "$path"; done; gcc -print-prog-name=ld')


def record_source(campaign):
    # Campaign deliberately uses an isolated Alire HOME. Trust only this exact
    # checkout for these calls, not persistent global config or other tasks.
    trusted_git = ['git', '-c', f'safe.directory={ROOT.resolve()}']
    campaign.run('source', [*trusted_git, 'rev-parse', 'HEAD', 'HEAD^{tree}'], ROOT)
    status = campaign.run('source-status', [*trusted_git, 'status', '--porcelain'], ROOT)
    if status.strip():
        raise RuntimeError('Task105 dirty source checkout; see source-status.stdout')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['scratch', 'setup', 'generator', 'sleet', 'uci25', 'uci26']:
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--compiler', required=True, choices=['14', '16'])
    parser.add_argument('--development', action='store_true', help='Record dirty-source development evidence, never hosted qualification')
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    ignore = shutil.ignore_patterns('obj', 'obj-*', 'bin', 'lib', '.git', '__pycache__', '*.o', '*.ali')
    spec = importlib.util.spec_from_file_location('task105_task098_qualify', ROOT / 'scripts/task098/qualify.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    campaign = module.Campaign(scratch, args.setup.resolve(), args.compiler)
    campaign.env.update(RUSTUP_HOME=os.environ.get('RUSTUP_HOME', str(Path.home() / '.rustup')),
                        CARGO_HOME=os.environ.get('CARGO_HOME', str(Path.home() / '.cargo')))
    case_spec = importlib.util.spec_from_file_location('task105_qualification_cases', ROOT / 'scripts/task105/cases.py')
    cases = importlib.util.module_from_spec(case_spec)
    case_spec.loader.exec_module(cases)
    campaign.provision()
    lock = args.setup.resolve() / 'consumer/alire/alire.lock'
    (scratch / 'toolchain-lock.json').write_text(json.dumps(tomllib.loads(lock.read_text()), indent=2) + '\n')
    if args.development:
        trusted_git = ['git', '-c', f'safe.directory={ROOT.resolve()}']
        campaign.run('source', [*trusted_git, 'rev-parse', 'HEAD', 'HEAD^{tree}'], ROOT)
        campaign.run('source-status', [*trusted_git, 'status', '--porcelain'], ROOT)
    else:
        record_source(campaign)
    campaign.run('platform', campaign.shell(['sh', '-ec', 'test "$(uname -m)" = x86_64; test "$(getconf GNU_LIBC_VERSION)" = "glibc 2.41"; uname -m; getconf GNU_LIBC_VERSION; gcc -v; ld --version; ' + TOOL_IDENTITIES]))
    campaign.run('alire-version', ['alr', '--version'])
    campaign.run('rustc-version', ['rustc', '+1.95.0', '--version', '--verbose'])
    campaign.run('cargo-version', ['cargo', '+1.95.0', '--version'])
    generator = args.generator.resolve()
    campaign.run('admission', [sys.executable, ROOT / 'scripts/task105/admission.py',
        '--evidence', scratch / 'admission', '--sleet', args.sleet.resolve(),
        '--uci26', args.uci26.resolve()])
    campaign.run('schema-comparison', [sys.executable, ROOT / 'scripts/task105/schema.py',
        '--uci25', args.uci25.resolve(), '--uci26', args.uci26.resolve(),
        '--evidence', scratch / 'schema-comparison.json'])
    inherited = scratch / 'inherited25'
    inherited.mkdir()
    for source in args.uci25.resolve().parent.glob('UCI_*_v2_5_0.xsd'):
        shutil.copyfile(source, inherited / source.name)
    campaign.run('inherited25-generate', [generator, 'service-generate',
        '--schema', args.uci25.resolve(), '--contract', ROOT / 'tests/fixtures/task104/uci25-service.yaml',
        '--language', 'ada', '--world', 'closed-schema', '--with-codec',
        '--output', scratch / 'inherited25-generated'])
    parent_spec = importlib.util.spec_from_file_location('task105_parent_baseline', ROOT / 'scripts/task104_evidence.py')
    parent = importlib.util.module_from_spec(parent_spec)
    parent_spec.loader.exec_module(parent)
    assert all(hashlib.sha256((scratch / 'inherited25-generated' / name).read_bytes()).hexdigest() == digest
               for name, digest in parent.BASELINE.items()), 'inherited UCI 2.5 output changed'
    hashes = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
              for folder in ['scripts/task105', 'tests/ada-real-uci-survivability26', 'tests/fixtures/task105', 'ada']
              for p in (ROOT / folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts}
    for relative in ['scripts/task098/real.py', 'scripts/task098/qualify.py', 'scripts/task098/cases.py',
                     'tests/ada-real-uci-survivability26/codec_consumer.adb', 'scripts/check-task092-real.py',
                     'scripts/check-task087a.py', 'scripts/task105_evidence.py',
                     'tests/fixtures/service-generate/codec-ada-minimal.yaml',
                     'tests/fixtures/task105/survivability26.json', 'scripts/task104_evidence.py',
                     'tests/fixtures/task104/uci25-service.yaml']:
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
    for release, schema in [('26', args.uci26.resolve())]:
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
        contract = ROOT / f'tests/fixtures/task105/uci{release}-service.yaml'
        original = scratch / f'{release}-input-contract.yaml'
        original.write_text((ROOT / 'tests/fixtures/task105/uci26-input-service.yaml').read_text())
        common = ['--schema', schema, '--contract', contract, '--language', 'ada', '--world', 'closed-schema']
        runtime = ['--with-codec']
        report = campaign.run(f'{release}-check', [generator, 'service-check', *common, *runtime])
        selected = 45
        assert f'selected type closure: {selected}\n' in report and f'codec renderable emitted declarations: {selected-1}/{selected-1}\ncodec status: READY\n' in report
        generated = scratch / f'{release}-generated'
        campaign.run(f'{release}-generate', [generator, 'service-generate', *common, *runtime, '--output', generated])
        baseline = scratch / f'{release}-original-generated'
        original_common = ['--schema', schema, '--contract', original, '--language', 'ada', '--world', 'closed-schema']
        campaign.run(f'{release}-original', [generator, 'service-generate', *original_common, *runtime, '--output', baseline])
        api = (generated / 'service_api.ads').read_text()
        start = api.index('      package Exchange_Publish is\n')
        end = api.index('      end Exchange_Publish;\n', start) + len('      end Exchange_Publish;\n\n')
        assert api[:start] + api[end:] == (baseline / 'service_api.ads').read_text(), 'unexpected input API change'
        (scratch / 'service-api-difference.json').write_text(json.dumps(dict(
            baseline='uci26-input-service.yaml', added_package='Exchange_Publish',
            added_source=api[start:end], inherited_input_unchanged=True), indent=2) + '\n')
        model = scratch / f'{release}-model'
        campaign.run(f'{release}-model', [generator, 'service-generate', *common, '--output', model])
        for path in model.iterdir():
            assert path.read_bytes() == (generated / path.name).read_bytes(), 'codec changed model/API'
        for path in baseline.iterdir():
            if path.name.startswith(('programs', 'service_codec')):
                assert path.read_bytes() == (generated / path.name).read_bytes(), 'adding output endpoint changed model/codec'
        # Generated direction enforcement is compiler-checked, not inferred
        # merely from strings or successful positive generic instantiation.
        for direction, endpoint, forbidden in [('input', 'Receive', 'Publisher'), ('output', 'Publish', 'Subscriber')]:
            probe = scratch / f'{release}-direction-{direction}'
            shutil.copytree(generated, probe)
            if direction == 'input':
                hook = ('function Hook (Namespace, Name, Topic : String; '
                        'Value : Service_API.Function_Survivability.Exchange_Receive.Payload) return Boolean is (True);\n')
                actuals = '(Boolean, Hook)'
            else:
                hook = ('function Hook (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String; '
                        "Receiver : not null access Service_API.Function_Survivability.Exchange_Receive.Handler'Class) "
                        'return Natural is (1);\n')
                actuals = '(Natural, Hook)'
            (probe / 'wrong.adb').write_text('with Service_API; procedure Wrong is\n' + hook +
                f'package Forbidden is new Service_API.Function_Survivability.Exchange_{endpoint}.{forbidden} {actuals};\n'
                'begin null; end Wrong;\n')
            (probe / 'wrong.gpr').write_text('project Wrong is for Source_Dirs use ("."); for Object_Dir use "obj"; for Main use ("wrong.adb"); end Wrong;\n')
            label = f'{release}-direction-{direction}'
            code = campaign.run_any(label, campaign.shell(['gprbuild', '-p', '-P', 'wrong.gpr']), probe)
            assert code != 0 and f'"{forbidden}" not declared' in (scratch / f'{label}.stderr').read_text()
        for policy in ['Normal', 'Check', 'Ignore']:
            build = scratch / f'{release}-codec-{policy}'
            shutil.copytree(baseline, build)
            consumer = (ROOT / 'tests/ada-real-uci-survivability26/codec_consumer.adb').read_text()
            (build / 'codec_consumer.adb').write_text(consumer)
            (build / 'consumer.gpr').write_text(
                f'with "{scratch / "support/ams_gra_oms_json"}";\nproject Consumer is\n'
                'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
                'for Main use ("codec_consumer.adb");\npackage Compiler is\n'
                'for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
                'end Compiler; end Consumer;\n')
            adc = build / 'policy.adc'
            adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
            campaign.run(f'{release}-codec-{policy}-build', campaign.shell([
                'gprbuild', '-p', '-P', 'consumer.gpr', '-cargs:Ada', f'-gnatec={adc}']), build)
            output = campaign.run(f'{release}-codec-{policy}-execute', [
                build / 'bin/codec_consumer',
                ROOT / 'tests/fixtures/task105/survivability26.json'])
            assert output == 'TASK105_CODEC_PASS checks= 8\n'
            (build / 'codec_probe.adb').write_text((ROOT / 'tests/ada-real-uci-survivability26/codec_probe.adb').read_text())
            project = build / 'consumer.gpr'
            project.write_text(project.read_text().replace('codec_consumer.adb', 'codec_probe.adb'))
            invalids = cases.invalid_documents(release) + cases.decoder_invalid(release)
            documents = build / 'invalid.jsonl'
            expectations = build / 'reasons.txt'
            documents.write_text(''.join(json.dumps(doc) + '\n' for _, doc in invalids))
            expectations.write_text(''.join(cases.CODEC_REASONS[label] + '\n' for label, _ in invalids))
            campaign.run(f'{release}-independent-{policy}-build', campaign.shell([
                'gprbuild', '-p', '-P', 'consumer.gpr', '-cargs:Ada', f'-gnatec={adc}']), build)
            output = campaign.run(f'{release}-independent-{policy}-execute', [
                build / 'bin/codec_probe', documents, expectations])
            assert output.endswith('TASK105_INDEPENDENT_CODEC_COMPLETE rejected=23 model_constraints=7\n')
        for policy, optimization in itertools.product(['Normal', 'Check', 'Ignore'], ['0', '2']):
            cell = f'uci{release}-{policy}-O{optimization}'
            build = scratch / cell
            shutil.copytree(ROOT / 'ada', build / 'ada', ignore=ignore)
            shutil.copytree(generated, build / 'generated')
            consumer = (ROOT / 'tests/ada-real-uci-survivability26/survivability_consumer.adb').read_text()
            (build / 'survivability_consumer.adb').write_text(consumer)
            (build / 'consumer.gpr').write_text('''with "ada/oms-lacal-runtime/oms_lacal_websocket";
with "ada/oms-json-support/ams_gra_oms_json";
project Consumer is
   for Source_Dirs use ("generated", ".");
   for Object_Dir use "obj";
   for Exec_Dir use "bin";
   for Main use ("survivability_consumer.adb");
   package Compiler is
      for Default_Switches ("Ada") use ("-gnat2022", "-gnato");
   end Compiler;
end Consumer;
''')
            adc = build / 'policy.adc'
            adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
            campaign.run(cell + '-build', campaign.shell(['gprbuild', '-p', '-j4', '-P', 'consumer.gpr',
                '-cargs:Ada', '-gnato', f'-O{optimization}', f'-gnatec={adc}']), build)
            output = campaign.run(cell + '-sleet', [sys.executable, ROOT / 'scripts/task105/sleet.py',
                '--checkout', args.sleet.resolve(), '--consumer', build / 'bin/survivability_consumer', '--schema', schema,
                '--release', release, '--evidence', build / 'sleet'])
            assert f'TASK105_REAL_SLEET_CELL_COMPLETE {release} handlers=12 decode_rejected=1 witness=1\n' in output
            cells.append(dict(release=release, policy=policy, optimization=optimization))
            print('TASK105_CELL_COMPLETE', campaign.version, cell, flush=True)
    assert all(hashlib.sha256((ROOT / p).read_bytes()).hexdigest() == digest for p, digest in hashes.items())
    (scratch / 'input-sources.json').write_text(json.dumps(hashes, indent=2) + '\n')
    (scratch / 'schema-sources.json').write_text(json.dumps(schema_sources, indent=2) + '\n')
    sources = {str(p.relative_to(scratch)): hashlib.sha256(p.read_bytes()).hexdigest()
               for p in scratch.rglob('*') if p.is_file() and p.suffix in ['.ads', '.adb', '.gpr', '.adc', '.xsd', '.yaml']}
    (scratch / 'sources.json').write_text(json.dumps(sources, indent=2) + '\n')
    (scratch / 'result.json').write_text(json.dumps(dict(completed=not args.development, compiler=campaign.version,
        cells=cells, steps=campaign.steps, real_uci=True, real_sleet=True), indent=2) + '\n')
    print('TASK105_QUALIFICATION_COMPLETE', campaign.version, len(cells), flush=True)


if __name__ == '__main__':
    main()
