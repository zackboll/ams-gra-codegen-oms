"""Production-generated UCI 2.5 heterogeneous Ada/Sleet compiler campaign.

Development campaigns are explicitly ineligible for clean-source acceptance.
No model, codec, API, runtime, schema or broker source is rewritten.
"""
import argparse
import hashlib
import importlib.util
import itertools
import json
from pathlib import Path
import re
import shutil
import sys
import tomllib

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DEPS = {'aht': ('https://github.com/ovenpasta/aht.git', '664d5ddeb568626b2f9ff66054c7091894b3b8d4'),
        'websocket-ada': ('https://github.com/kokhlo/websocket-ada.git', '765d9cd612838eb00645037977c8cc9af7171c3e')}
GENERATED = {
    'programs.ads': 'a5d92d26a18c03999588818e68c5a8b3426ff1c780a7582bf21121727b4a0d16',
    'programs-oam.ads': '0e4d1e92a632883c5de4db44649313325ea155ae0515866c919b0976f7ead6e3',
    'programs-oam.adb': 'e1f7bc6fe7d2f422e058219cd58bae7ae1496fc722fb64516f5baf02003ee70d',
    'service_api.ads': 'c9542a84957be6566b7e32a2242cd93d3b91097c8dba8e5e85685808ebc61112',
    'service_codec.ads': '60bbec5768703bad5bdf276764b9db89837d0d7be918de2d23d793a8259731b1',
    'service_codec.adb': 'ef0c13406dd70fbe87d683eb1a357ea3a6723b0a33bff74bfaf311c56a512e8d',
}
CONSUMERS = (
    ('log', 'hexbinary_log_consumer', 'task093-real-log25.json', 'TASK093_LOG_COMPLETE checks= 29 rejections= 9\n'),
    ('rdma', 'task092_real_rdma', 'task092-real-rdma25.json', 'TASK092_REAL_PASS checks= 12\n'),
    ('survivability', 'task092_real_survivability', 'task092-real-survivability25.json', 'TASK092_REAL_PASS checks= 8\n'),
)
INDEPENDENT = {
    'log': {
        'programs-oam.ads': 'e57af3b95f3f66c6feaa4239e0e8d0a614ca4a5845c18ceb2f08331b2db031d4',
        'programs-oam.adb': '53d4061ce6c41662ad4ef932eaf6772b030a8fd8c4f49421eb2ebafd17c8ea6d',
        'service_codec.ads': '6cd02c0a1a55e832b597a92384236adf72c96b3e54798d208fb5a2b48b896fc7',
        'service_codec.adb': '75989c7a382c115c4c39ee42a2b28feedb7df922fe7eecef0bd486f5168c1897',
    },
    'rdma': {
        'programs-oam.ads': 'b3aab7ed3c9b12e842341e782786787e984949ca311e11e330fd73f828e61035',
        'programs-oam.adb': 'f0986e0efa87693f7762e08515f828a8da0fbd1e2730c8434d1e566167788d13',
        'service_codec.ads': 'c0e02e14c43a14cbe7305d4df006ac24a5e6b1eebfb083b29e9e1f7cbf87567c',
        'service_codec.adb': 'fbe3423fa601ecb4988c486368ce84ee76d3b45715dd0aad915f03d05bc7e0f6',
    },
}


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs():
    folders = ('scripts/task106', 'tests/fixtures/task106', 'tests/ada-real-uci-multimessage', 'ada', 'crates')
    paths = [p for folder in folders for p in (ROOT / folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts]
    paths += [ROOT / p for p in ('Cargo.lock', 'scripts/task106_evidence.py', 'scripts/task098/qualify.py',
                                'scripts/check-task087a.py', 'scripts/task098/cases.py')]
    for number, (key, name, fixture, _) in zip((102, 103, 104), CONSUMERS, strict=True):
        paths += [ROOT / f'scripts/task{number}/cases.py', ROOT / f'scripts/task{number}_evidence.py',
                  ROOT / f'tests/fixtures/task{number}/uci25-service.yaml',
                  ROOT / 'tests/ada-generated-codec' / (name + '.adb'),
                  ROOT / 'tests/fixtures/service-generate' / fixture]
    paths += [ROOT / 'tests/ada-real-uci-rdma/codec_probe.adb',
              ROOT / 'tests/ada-real-uci-survivability/codec_probe.adb']
    return {str(p.relative_to(ROOT)): digest(p) for p in paths}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('scratch', 'setup', 'generator', 'sleet', 'uci25'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--compiler', choices=('14', '16'), required=True)
    parser.add_argument('--development', action='store_true')
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    campaign = load('task106_shared_campaign', ROOT / 'scripts/task098/qualify.py').Campaign(
        scratch, args.setup.resolve(), args.compiler)
    before = inputs()
    campaign.provision()
    trusted_git = ['git', '-c', f'safe.directory={ROOT.resolve()}']
    campaign.run('source', [*trusted_git, 'rev-parse', 'HEAD', 'HEAD^{tree}'], ROOT)
    status = campaign.run('source-status', [*trusted_git, 'status', '--porcelain'], ROOT)
    if status.strip() and not args.development:
        raise RuntimeError('Task106 requires a clean source checkout')
    lock = args.setup.resolve() / 'consumer/alire/alire.lock'
    (scratch / 'toolchain-lock.json').write_text(json.dumps(tomllib.loads(lock.read_text()), indent=2) + '\n')
    campaign.run('tool-identities', campaign.shell(['sh', '-ec',
        'uname -m; getconf GNU_LIBC_VERSION; for tool in gcc gnat gprbuild ld; do path=$(command -v "$tool"); printf "%s\\n" "$path"; sha256sum "$path"; done; gcc -v; ld --version']))
    campaign.run('alire-version', ['alr', '--version'])
    generator = args.generator.resolve()
    (scratch / 'generator.sha256').write_text(digest(generator) + '\n')
    schema_dir = scratch / 'uci25'
    schema_dir.mkdir()
    admission = load('task106_campaign_admission', ROOT / 'scripts/task106/admission.py')
    for name, pin in admission.PINS.items():
        source = args.uci25.resolve().parent / name
        if digest(source) != pin:
            raise ValueError('authentic schema pin mismatch')
        shutil.copyfile(source, schema_dir / name)
    schema = schema_dir / 'UCI_MessageDefinitions_v2_5_0.xsd'
    common = ['--schema', schema, '--contract', ROOT / 'tests/fixtures/task106/uci25-service.yaml',
              '--language', 'ada', '--world', 'closed-schema']
    report = campaign.run('check', [generator, 'service-check', *common, '--with-codec'])
    if not all(text in report for text in ('selected oms messages: 3\n', 'renderable selected oms messages: 3\n',
            'selected type closure: 64\n', 'renderable selected types: 64\n', 'status: READY\n',
            'codec renderable emitted declarations: 63/63\ncodec status: READY\n')):
        raise ValueError('MULTI_MESSAGE_GENERATION_BLOCKED: combined readiness mismatch')
    generated = scratch / 'generated'
    campaign.run('generate', [generator, 'service-generate', *common, '--with-codec', '--output', generated])
    campaign.run('model', [generator, 'service-generate', *common, '--output', scratch / 'model'])
    if {p.name: digest(p) for p in generated.iterdir()} != GENERATED:
        raise ValueError('combined generated-source pin mismatch')
    for p in (scratch / 'model').iterdir():
        if p.read_bytes() != (generated / p.name).read_bytes():
            raise ValueError('codec opt-in changed whole-model/API bytes')
    # Per-type representations and independent baseline source identities, NOT
    # whole-union equality with a single-message selection.
    parity = {}
    for number, (key, _, _, _) in zip((102, 103, 104), CONSUMERS, strict=True):
        baseline = scratch / ('baseline-' + key)
        campaign.run('baseline-' + key, [generator, 'service-generate', '--schema', schema,
            '--contract', ROOT / f'tests/fixtures/task{number}/uci25-service.yaml', '--language', 'ada',
            '--world', 'closed-schema', '--with-codec', '--output', baseline])
        original = load('task106_baseline_' + key, ROOT / f'scripts/task{number}_evidence.py')
        pins = original.BASELINE if key == 'survivability' else INDEPENDENT[key]
        for name, pin in pins.items():
            if digest(baseline / name) != pin:
                raise ValueError('independent baseline changed: ' + key + '/' + name)
        payload = {'log': 'LogMT', 'rdma': 'RDMA_InitializeSetupMT', 'survivability': 'SurvivabilityRiskLevelMT'}[key]
        pattern = r'^   type ' + payload + r' is record\n.*?^   end record;'
        a = re.findall(pattern, (baseline / 'programs-oam.ads').read_text(), re.M | re.S)
        b = re.findall(pattern, (generated / 'programs-oam.ads').read_text(), re.M | re.S)
        if len(a) != 1 or a != b:
            raise ValueError('payload representation mismatch: ' + key)
        parity[key] = dict(payload=payload, representation_sha256=hashlib.sha256(a[0].encode()).hexdigest(),
                           independent_baseline_hashes=pins)
    (scratch / 'representation-parity.json').write_text(json.dumps(parity, indent=2) + '\n')
    ignore = shutil.ignore_patterns('obj', 'obj-*', 'lib', 'bin', '.git', '__pycache__', '*.o', '*.ali')
    for key in ('Log', 'Rdma', 'Survivability'):
        for endpoint, forbidden in (('Receive', 'Publisher'), ('Publish', 'Subscriber')):
            label = 'direction-' + key.lower() + '-' + endpoint.lower()
            probe = scratch / label
            shutil.copytree(generated, probe)
            (probe / 'wrong.adb').write_text('with Service_API; procedure Wrong is\n'
                f'package Forbidden renames Service_API.Function_{key}.Exchange_{endpoint}.{forbidden};\n'
                'begin null; end Wrong;\n')
            (probe / 'wrong.gpr').write_text('project Wrong is for Source_Dirs use ("."); for Object_Dir use "obj"; for Main use ("wrong.adb"); end Wrong;\n')
            code = campaign.run_any(label, campaign.shell(['gprbuild', '-p', '-P', 'wrong.gpr']), probe)
            if code == 0 or f'"{forbidden}" not declared' not in (scratch / (label + '.stderr')).read_text():
                raise ValueError('wrong endpoint-direction rejection')
    for policy in ('Normal', 'Check', 'Ignore'):
        build = scratch / ('codec-' + policy)
        shutil.copytree(generated, build)
        for _, name, _, _ in CONSUMERS:
            shutil.copyfile(ROOT / 'tests/ada-generated-codec' / (name + '.adb'), build / (name + '.adb'))
        mains = ', '.join('"' + name + '.adb"' for _, name, _, _ in CONSUMERS)
        (build / 'consumer.gpr').write_text(f'with "{scratch / "support/ams_gra_oms_json"}";\n'
            f'project Consumer is for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ({mains});\n'
            'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato"); end Compiler; end Consumer;\n')
        adc = build / 'policy.adc'
        adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
        campaign.run('codec-' + policy + '-build', campaign.shell(['gprbuild', '-p', '-j4', '-P', 'consumer.gpr',
                     '-cargs:Ada', '-O0', '-gnato', '-gnatec=' + str(adc)]), build)
        for key, name, fixture, marker in CONSUMERS:
            output = campaign.run('codec-' + policy + '-' + key, [build / 'bin' / name,
                ROOT / 'tests/fixtures/service-generate' / fixture])
            if output != marker:
                raise ValueError('original independent consumer mismatch')
        for key, number, count in [('rdma', 103, 9), ('survivability', 104, 14)]:
            probe = scratch / f'independent-{policy}-{key}'
            shutil.copytree(generated, probe)
            shutil.copyfile(ROOT / f'tests/ada-real-uci-{key}/codec_probe.adb', probe / 'codec_probe.adb')
            (probe / 'consumer.gpr').write_text((build / 'consumer.gpr').read_text().replace(
                f'for Main use ({mains});', 'for Main use ("codec_probe.adb");'))
            module = load(f'task106_independent_probe_{number}', ROOT / f'scripts/task{number}/cases.py')
            invalids = module.invalid_documents('25') + module.decoder_invalid('25')
            documents, reasons = probe / 'documents.jsonl', probe / 'reasons.txt'
            documents.write_text(''.join(json.dumps(document) + '\n' for _, document in invalids))
            reasons.write_text(''.join(module.CODEC_REASONS[label] + '\n' for label, _ in invalids))
            (probe / 'policy.adc').write_text(adc.read_text())
            campaign.run(f'independent-{policy}-{key}-build', campaign.shell(['gprbuild', '-p', '-j4', '-P',
                'consumer.gpr', '-cargs:Ada', '-O0', '-gnato', '-gnatec=' + str(probe / 'policy.adc')]), probe)
            output = campaign.run(f'independent-{policy}-{key}-execute', [probe / 'bin/codec_probe', documents, reasons])
            if not output.endswith(f'TASK{number}_INDEPENDENT_CODEC_COMPLETE rejected={count} model_constraints=4\n'):
                raise ValueError('independent combined-model negative controls failed')
    # Phase B must pass before any full combined real-Sleet cell.
    campaign.run('admission', [sys.executable, ROOT / 'scripts/task106/admission.py', '--evidence',
                 scratch / 'admission', '--sleet', args.sleet.resolve(), '--uci25', schema])
    for name, (url, revision) in DEPS.items():
        checkout = scratch / 'deps' / name
        campaign.run(name + '-clone', ['git', 'clone', '--quiet', url, checkout])
        campaign.run(name + '-pin', ['git', '-C', checkout, 'checkout', '--quiet', '--detach', revision])
        if campaign.run(name + '-identity', ['git', '-C', checkout, 'rev-parse', 'HEAD']).strip() != revision:
            raise ValueError('wrong transport dependency')
        if campaign.run(name + '-clean', ['git', '-C', checkout, 'status', '--porcelain']).strip():
            raise ValueError('dirty transport dependency')
    campaign.env.update(OMS_WS_AHT_SOURCE=str(scratch / 'deps/aht'),
                        OMS_WS_CODEC_SOURCE=str(scratch / 'deps/websocket-ada'), PYTHONDONTWRITEBYTECODE='1')
    cells = []
    for policy, optimization in itertools.product(('Normal', 'Check', 'Ignore'), ('0', '2')):
        label = f'uci25-{policy}-O{optimization}'
        build = scratch / label
        shutil.copytree(ROOT / 'ada', build / 'ada', ignore=ignore)
        shutil.copytree(generated, build / 'generated')
        shutil.copyfile(ROOT / 'tests/ada-real-uci-multimessage/multimessage_consumer.adb', build / 'multimessage_consumer.adb')
        (build / 'consumer.gpr').write_text('with "ada/oms-lacal-runtime/oms_lacal_websocket";\n'
            'with "ada/oms-json-support/ams_gra_oms_json";\nproject Consumer is\n'
            'for Source_Dirs use ("generated", "."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
            'for Main use ("multimessage_consumer.adb");\npackage Compiler is\n'
            'for Default_Switches ("Ada") use ("-gnat2022", "-gnato"); end Compiler; end Consumer;\n')
        adc = build / 'policy.adc'
        adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
        campaign.run(label + '-build', campaign.shell(['gprbuild', '-p', '-j4', '-P', 'consumer.gpr',
            '-cargs:Ada', '-gnato', '-O' + optimization, '-gnatec=' + str(adc)]), build)
        campaign.run(label + '-sleet', [sys.executable, ROOT / 'scripts/task106/sleet.py', '--evidence',
            build / 'sleet', '--checkout', args.sleet.resolve(), '--schema', schema,
            '--consumer', build / 'bin/multimessage_consumer'])
        cells.append(dict(release='25', policy=policy, optimization=optimization))
        print('TASK106_CELL_COMPLETE', campaign.version, label, flush=True)
    if inputs() != before:
        raise ValueError('Task106 campaign inputs changed while executing')
    (scratch / 'input-sources.json').write_text(json.dumps(before, indent=2) + '\n')
    (scratch / 'sources.json').write_text(json.dumps({str(p.relative_to(scratch)): digest(p)
        for p in scratch.rglob('*') if p.is_file() and p.suffix in ('.ads', '.adb', '.gpr', '.adc', '.xsd', '.yaml')}, indent=2) + '\n')
    (scratch / 'result.json').write_text(json.dumps(dict(completed=not args.development,
        development=args.development, compiler=campaign.version, cells=cells, steps=campaign.steps), indent=2) + '\n')
    print('TASK106_QUALIFICATION_COMPLETE', campaign.version, len(cells), flush=True)


if __name__ == '__main__':
    main()