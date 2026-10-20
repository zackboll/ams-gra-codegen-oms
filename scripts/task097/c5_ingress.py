"""C5 independent occurrence/Unicode/limit/resource qualification (no skips).

Run from a frozen source tree. Old/new SAX spans are compared independently
from Python's strict decoded structural view. Object enumeration order is not
part of the wire contract. Original numeric slices remain checked.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import random
import re
import shlex
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def atomic(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')
    temporary.replace(path)


class Number(str):
    pass


def flatten(text):
    tree = json.loads(text, parse_int=Number, parse_float=Number,
                      parse_constant=lambda s: (_ for _ in ()).throw(ValueError(s)))
    rows = {}

    def visit(value, path):
        if isinstance(value, dict):
            kind, payload = 'JSON_OBJECT_TYPE', ''
        elif isinstance(value, list):
            kind, payload = 'JSON_ARRAY_TYPE', ''
        elif type(value) is Number:
            kind = 'JSON_FLOAT_TYPE' if any(c in value for c in '.eE') else 'JSON_INT_TYPE'
            payload = value.encode().hex().upper()
        elif isinstance(value, str):
            kind, payload = 'JSON_STRING_TYPE', value.encode('utf-8').hex().upper()
        elif isinstance(value, bool):
            kind, payload = 'JSON_BOOLEAN_TYPE', str(value).upper()
        elif value is None:
            kind, payload = 'JSON_NULL_TYPE', ''
        else:
            raise RuntimeError('oracle type')
        rows[path] = (kind, payload)
        if isinstance(value, dict):
            for key, child in value.items():
                visit(child, path + '/k' + key.encode('utf-8').hex().upper())
        elif isinstance(value, list):
            for i, child in enumerate(value, 1):
                visit(child, path + '/i ' + str(i))
    visit(tree, '$ ')
    return rows


def dumped(output, text):
    rows, semantic = {}, {}
    for line in output.splitlines():
        match = re.fullmatch(r'(.*?) (JSON_\w+)\s+(\d+)\s+(\d+) (.*)', line)
        if not match:
            continue
        path, kind, first, last, payload = match.groups()
        first, last = int(first), int(last)
        if path in rows or not 1 <= first <= last <= len(text):
            raise RuntimeError('duplicate path or invalid byte span')
        token = text[first - 1:last]
        json.loads(token.decode('utf-8'), parse_int=Number, parse_float=Number)
        if kind in ['JSON_INT_TYPE', 'JSON_FLOAT_TYPE'] and bytes.fromhex(payload) != token:
            raise RuntimeError('lost original numeric lexeme')
        rows[path] = (kind, first, last, payload)
        semantic[path] = (kind, payload)
    return rows, semantic


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scratch', type=Path, required=True)
    parser.add_argument('--compiler', choices=[14, 16], type=int, required=True)
    parser.add_argument('--policies', nargs='+', default=['Normal', 'Check', 'Ignore'])
    parser.add_argument('--optimizations', nargs='+', default=['0', '2'])
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    spec = importlib.util.spec_from_file_location('c4_oracle', ROOT / 'scripts/task097/c4_differential.py')
    c4 = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(c4)
    cases = c4.corpus(scratch / 'cases')
    assert len(cases) == 339
    extra = [
        '{"a":{"old":[1,2]},"\\u0061":{"new":[-0,9223372036854775808]}}',
        '{"a":[1],"a":null,"b":{"a":[2]},"a":[{"a":3}]}',
        '{"é":1,"\\u00e9":{"x":2},"𝄞":3,"\\ud834\\udd1e":[4]}',
        '{"":0,"\\u0000":1,"a\\\"b":"\\b\\f\\n\\r\\t\\/\\\\"}',
        '{"a":["retained"]}',
    ]
    # All UTF-16 BMP code units outside the surrogate range, and representative
    # paired scalar boundaries, independently decoded without the provider.
    units = [*range(0, 0xD800), *range(0xE000, 0x10000)]
    for start in range(0, len(units), 512):
        extra.append('"' + ''.join('\\u%04x' % n for n in units[start:start + 512]) + '"')
    for scalar in [0x10000, 0x10001, 0x1F6F0, 0x10FFFE, 0x10FFFF]:
        x = scalar - 0x10000
        extra.append('"\\u%04x\\u%04x"' % (0xD800 + (x >> 10), 0xDC00 + (x & 1023)))
    rng = random.Random(9705)
    for size in [1, 16, 31, 32, 33, 63, 64, 65, 128, 1024]:
        extra.append(json.dumps('é 🛰 "\\\x00' * size, ensure_ascii=False))
    for count in [31, 128, 1000, 10_000]:
        keys = list(range(count))
        rng.shuffle(keys)
        extra.append('{' + ','.join('"k%08d":[%d]' % (i, i) for i in keys) + '}')
    for i, text in enumerate(extra):
        path = scratch / 'cases' / f'occurrence-{i}.json'
        path.write_text(text)
        assert c4.strict(text.encode())
        cases.append({'name': f'occurrence-{i}', 'path': str(path), 'expected': True,
                      'old_overlap': True, 'bounds': ['17']})
    atomic(scratch / 'cases.json', cases)
    rows = []

    def run(label, cmd, cwd=scratch):
        atomic(scratch / (label + '.command.json'), {'argv': cmd, 'cwd': str(cwd)})
        start = time.monotonic()
        with (scratch / (label + '.stdout')).open('wb') as out, (scratch / (label + '.stderr')).open('wb') as err:
            p = subprocess.run(cmd, cwd=cwd, stdout=out, stderr=err)
        elapsed = time.monotonic() - start
        (scratch / (label + '.exit')).write_text(str(p.returncode) + '\n')
        if p.returncode:
            raise RuntimeError(label)
        return (scratch / (label + '.stdout')).read_text(), elapsed

    prior = Path(f'/home/zboll/task097-scratch/c4/alloc-new-{args.compiler}-oam')
    ignore = shutil.ignore_patterns('obj', 'obj-*', 'lib', 'bin', '*.o', '*.ali', '*.a', '*.so', '*.lexch', '*.bexch')
    shutil.copytree(ROOT / 'ada/oms-json-support', scratch / 'support', ignore=ignore)
    shutil.copytree(prior / 'provider', scratch / 'provider', ignore=ignore)
    environment = scratch / 'environment.sh'
    environment.write_text((prior / 'environment.sh').read_text().replace(str(prior / 'provider'), str(scratch / 'provider')))
    shell = lambda cmd: ['sh', '-ec', f'. {shlex.quote(str(environment))}; exec ' + shlex.join(cmd)]
    run('compiler', shell(['gnatls', '-v']))
    for policy in args.policies:
        for opt in args.optimizations:
            label = policy + '-O' + opt
            src = scratch / label
            src.mkdir()
            for name in ['c5_ingress.adb', 'c3_malloc_trace.c']:
                shutil.copy2(ROOT / 'tests/ada-float64-codec' / name, src)
            shutil.copy2(ROOT / 'tests/ada-float64-codec/c1_null_allocation_faults.ads', src / 'allocation_faults.ads')
            shutil.copy2(ROOT / 'tests/ada-float64-codec/c3_trace_faults.adb', src / 'allocation_faults.adb')
            adc = src / 'policy.adc'
            adc.write_text(f'pragma Assertion_Policy ({policy});\n' if policy != 'Normal' else '-- Normal\n')
            (src / 'proof.gpr').write_text('with "../support/ams_gra_oms_json_lossless";\nproject Proof is\n'
                'for Languages use ("Ada", "C"); for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ("c5_ingress.adb");\n'
                'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-g", "-O' + opt + '"); for Default_Switches ("C") use ("-g", "-O0"); end Compiler;\n'
                'package Linker is for Default_Switches ("Ada") use ("-Wl,--wrap=__gnat_malloc", "-no-pie"); end Linker;\nend Proof;\n')
            run(label + '-build', shell(['gprbuild', '-P', 'proof.gpr', '-p', '-j2', '-cargs:Ada', '-g', '-O' + opt, '-gnatec=' + str(adc)]), src)
            binary = str(src / 'bin/c5_ingress')
            for case in cases:
                text = Path(case['path']).read_bytes()
                mode = 'dump' if case['expected'] else 'new'
                output, elapsed = run(label + '-' + case['name'], [binary, mode, case['path'], '1000000'], src)
                if output.splitlines()[0] != ('C5_ACCEPT' if case['expected'] else 'C5_REJECT'):
                    raise RuntimeError('acceptance mismatch ' + case['name'])
                if case['expected']:
                    spans, semantic = dumped(output, text)
                    if semantic != flatten(text):
                        raise RuntimeError('independent occurrence mismatch ' + case['name'])
                    if policy == 'Normal' and opt == '0' and case['old_overlap']:
                        old, _ = run('old-' + case['name'], [binary, 'old', case['path'], '1000000'], src)
                        old_spans, _ = dumped(old, text)
                        if spans != old_spans:
                            raise RuntimeError('old occurrence/span mismatch ' + case['name'])
                rows.append({'cell': label, 'case': case['name'], 'completed': True, 'seconds': elapsed})
            for i in [0, 1, 2, 3, 4]:
                path = scratch / 'cases' / f'occurrence-{i}.json'
                baseline, _ = run(label + f'-allocation-{i}', [binary, 'new', str(path), '1000000'], src)
                count = int(re.search(r'attempts=\s*(\d+)', baseline)[1])
                for budget in range(count):
                    out, _ = run(label + f'-failure-{i}-{budget}', [binary, 'inject', str(path), str(budget)], src)
                    if 'C5_EXCEPTION STORAGE_ERROR' not in out or 'C5_INGRESS_RECOVERY_COMPLETE' not in out:
                        raise RuntimeError('resource boundary')
            out, _ = run(label + '-handles', [binary, 'handles', str(scratch / 'cases/occurrence-4.json'), '1000000'], src)
            if 'C5_NONALLOCATING_HANDLE_LIFETIME_COMPLETE' not in out:
                raise RuntimeError('handle lifetime/copy')
            out, _ = run(label + '-callback', [binary, 'callback', str(scratch / 'cases/occurrence-4.json'), '1000000'], src)
            if 'C5_CALLBACK_INTERNAL_RECOVERY_COMPLETE' not in out:
                raise RuntimeError('internal exception hidden')
            print('C5_INGRESS_CELL_COMPLETE', args.compiler, label, len(cases), flush=True)
    atomic(scratch / 'result.json', {'completed': True, 'compiler': args.compiler, 'cases': len(cases), 'rows': rows})
    print('C5_INGRESS_DIFFERENTIAL_COMPLETE', flush=True)


if __name__ == '__main__':
    main()