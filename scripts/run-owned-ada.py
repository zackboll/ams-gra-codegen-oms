#!/usr/bin/env python3
"""Fixed Ada owned runner: reuse provisioned compilers/generator/pinned broker."""
import argparse
import importlib.util
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from owned_qualification import ROOT, VERSIONS, POLICIES, OPTIMIZATIONS, ada, cells, digest, document
from task101_evidence import PINS, INCLUDES, REV, TREE


def load(relative, name):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['fast', 'pinned'])
    for option in ('output', 'setup', 'generator'):
        parser.add_argument('--' + option, type=Path, required=True)
    parser.add_argument('--compiler', choices=VERSIONS, required=True)
    for option in ('uci25', 'uci26', 'sleet', 'shared-campaign'):
        parser.add_argument('--' + option, type=Path)
    args = parser.parse_args()
    output = args.output.absolute()
    # Reject reuse, including a previous successful receipt. No partial campaign
    # can leave accepted evidence from a different execution.
    if output.exists():
        (output / 'result.json').unlink(missing_ok=True)
        raise ValueError('owned evidence output must be fresh')
    output.mkdir(parents=True)
    records = [r for r in ada() if (r['mode'] == 'fast') == (args.mode == 'fast')]
    campaign = load('scripts/task098/qualify.py', 'owned_campaign').Campaign(
        output, args.setup.resolve(), args.compiler)
    campaign.provision()
    source = campaign.run('owned-source', ['git', '-c', f'safe.directory={ROOT}',
                                          'rev-parse', 'HEAD', 'HEAD^{tree}']).splitlines()
    if campaign.run('owned-clean', ['git', '-c', f'safe.directory={ROOT}', 'status', '--porcelain']):
        raise ValueError('dirty owned source checkout')
    if os.environ.get('GITHUB_SHA', source[0]) != source[0]:
        raise ValueError('owned source differs from workflow source')
    identity = dict(sha=source[0], tree=source[1], run_id=os.environ.get('GITHUB_RUN_ID', 'local'),
                    run_attempt=os.environ.get('GITHUB_RUN_ATTEMPT', '1'), compiler=args.compiler,
                    compiler_version=VERSIONS[args.compiler], generator_sha256=digest(args.generator.read_bytes()),
                    mode=args.mode)
    (output / 'generator.sha256').write_text(identity['generator_sha256'] + '\n')
    campaign.run('tool-identities', campaign.shell(['sh', '-ec',
        'for tool in gnat gcc gprbuild; do path=$(command -v "$tool"); '
        'printf "%s\\n" "$path"; sha256sum "$path"; done']))
    if args.mode == 'pinned':
        for release, schema in [('25', args.uci25), ('26', args.uci26)]:
            if (schema is None or digest(schema.read_bytes()) != PINS[release] or
                    digest((schema.parent / f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd').read_bytes()) != INCLUDES[release]):
                raise ValueError('wrong pinned UCI root/include')
            pinned = output / ('uci' + release)
            pinned.mkdir()
            for name in (f'UCI_MessageDefinitions_v2_{release[-1]}_0.xsd',
                         f'UCI_SecurityMarkings_v2_{release[-1]}_0.xsd'):
                shutil.copyfile(schema.parent / name, pinned / name)
        if (args.sleet is None or subprocess.check_output(['git', '-C', str(args.sleet),
                'rev-parse', 'HEAD', 'HEAD^{tree}'], text=True).splitlines() != [REV, TREE] or
                subprocess.check_output(['git', '-C', str(args.sleet), 'status', '--porcelain'])):
            raise ValueError('wrong/dirty pinned Sleet')
        identity.update(uci_roots=PINS, uci_includes=INCLUDES, sleet_revision=REV, sleet_tree=TREE,
                        sleet_binary_sha256=digest((args.sleet / 'target/release/sleet').read_bytes()))
    actual = []
    ignore = shutil.ignore_patterns('obj', 'obj-*', 'bin', 'lib', '.git', '__pycache__', '*.o', '*.ali')
    for record in records:
        item = record['manifest']
        task = ROOT / record['path']
        generated = None
        if record['mode'] != 'fast':
            schema = args.uci25 if item['release'] == '2.5' else args.uci26
            common = ['--schema', schema, '--contract', task / item['contract'], '--language', 'ada',
                      '--world', 'closed-schema']
            generated = output / (item['task_id'] + '-generated')
            runtime = ['--with-codec', '--ada-codec-runtime', 'linux-x86_64-glibc-2.41-lossless-v1']
            report = campaign.run(item['task_id'] + '-check', [args.generator, 'service-check', *common, *runtime])
            if 'codec status: READY\n' not in report:
                raise ValueError('owned codec not READY')
            campaign.run(item['task_id'] + '-generate', [args.generator, 'service-generate', *common,
                                                       *runtime, '--output', generated])
            model = output / (item['task_id'] + '-model')
            campaign.run(item['task_id'] + '-model', [args.generator, 'service-generate', *common, '--output', model])
            if any(p.read_bytes() != (generated / p.name).read_bytes() for p in model.iterdir()):
                raise ValueError('owned codec changed generated model/API')
        for policy, optimization in itertools.product(POLICIES, OPTIMIZATIONS):
            label = item['task_id'] + '-' + policy + '-O' + optimization
            build = output / label
            build.mkdir()
            if generated:
                shutil.copytree(generated, build / 'generated')
            shutil.copytree(ROOT / 'ada', build / 'ada', ignore=ignore)
            for name in item['sources']:
                shutil.copyfile(task / name, build / name)
            for name in item['fixtures']:
                shutil.copyfile(task / name, build / name)
            extra = ''
            if record['mode'] != 'fast':
                extra = 'with "ada/oms-json-support/ams_gra_oms_json_lossless";\n'
            if record['mode'] == 'sleet':
                extra += 'with "ada/oms-lacal-runtime/oms_lacal_websocket";\n'
                if args.shared_campaign is None:
                    raise ValueError('missing authenticated shared transport dependencies')
                campaign.env.update(OMS_WS_AHT_SOURCE=str(args.shared_campaign / 'deps/aht'),
                                    OMS_WS_CODEC_SOURCE=str(args.shared_campaign / 'deps/websocket-ada'))
            (build / 'consumer.gpr').write_text(extra + 'project Consumer is\n'
                'for Source_Dirs use (".", "generated"); for Object_Dir use "obj";\n'
                f'for Exec_Dir use "bin"; for Main use ("{item["main"]}");\n'
                'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato");\n'
                'end Compiler; end Consumer;\n')
            if not generated:
                (build / 'generated').mkdir()
            adc = build / 'policy.adc'
            adc.write_text('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n')
            campaign.run(label + '-build', campaign.shell(['gprbuild', '-P', 'consumer.gpr', '-p', '-f',
                '-cargs:Ada', '-gnato', '-O' + optimization, '-gnatec=' + str(adc)]), build)
            binary = build / 'bin' / Path(item['main']).stem
            if record['mode'] == 'fast':
                text = campaign.run(label + '-execute', [binary], build)
                if sorted(text.splitlines()) != ['CI004_PASS ' + test for test in item['tests']]:
                    raise ValueError('missing, skipped, duplicate or failed Ada test')
            elif record['mode'] == 'uci':
                cases = document((task / item['cases']).read_text())
                inputs = build / 'documents.jsonl'
                inputs.write_text(''.join(case['input'] + '\n' for case in cases))
                campaign.run(label + '-execute', [binary, inputs], build)
                from owned_ada_evidence import verify_codec
                verify_codec((output / (label + '-execute.stdout')).read_bytes(), cases)
            else:
                campaign.run(label + '-execute', [sys.executable, ROOT / 'scripts/task101/sleet.py',
                    '--checkout', args.sleet, '--consumer', binary, '--schema', schema,
                    '--release', item['release'].replace('.', ''), '--evidence', build / 'sleet'])
            actual.extend([f'{item["task_id"]}/{policy}/O{optimization}'])
    if records != [r for r in ada() if (r['mode'] == 'fast') == (args.mode == 'fast')]:
        raise ValueError('owned sources changed during execution')
    if digest(args.generator.read_bytes()) != identity['generator_sha256']:
        raise ValueError('generator changed during owned execution')
    expected = [cell for r in records for cell in cells(r)]
    # Build products and a writable provider are not evidence and must not inflate
    # the uploaded bundle or leak executable task output into gate processing.
    shutil.rmtree(output / 'provider')
    for path in sorted(output.rglob('*'), reverse=True):
        if path.is_dir() and path.name in ('obj', 'bin', 'lib'):
            shutil.rmtree(path)
    logs = {p.relative_to(output).as_posix(): digest(p.read_bytes()) for p in output.rglob('*')
            if p.is_file() and p.suffix in ('.stdout', '.stderr', '.exit', '.json', '.adc', '.gpr', '.adb', '.ads', '.sha256', '.xsd')
            and 'provider' not in p.parts}
    receipt = dict(version=1, identity=identity, registrations=records, required_cells=expected,
                   actual_cells=actual, steps=campaign.steps, files=logs, completed=True)
    (output / 'result.json').write_text(json.dumps(receipt, indent=2) + '\n')
    from owned_ada_evidence import verify_bundle
    try:
        verify_bundle({p.relative_to(output).as_posix(): p.read_bytes() for p in output.rglob('*')
                       if p.is_file() and 'provider' not in p.parts}, args.mode, args.compiler,
                      source[0], source[1], identity['run_id'], identity['run_attempt'])
    except Exception:
        (output / 'result.json').unlink(missing_ok=True)
        raise
    print('CI004_ADA_COMPLETE', args.mode, args.compiler, len(actual), flush=True)


if __name__ == '__main__':
    main()