"""Independent strict-JSON/Unicode/limit oracle for the validation-only boundary.

Run against a frozen source snapshot. Every case has explicit outcome metadata;
old numeric-DOM limitations and duplicate-occupancy differences are not parity.
"""
import argparse
import json
import random
import re
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def atomic(path, data):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


class ObjectPairs(list):
    pass


def strict(text):
    """Python grammar/UTF-8 plus independent scalar Unicode and bounded walk."""
    if len(text) > 1_048_576:
        return False
    try:
        tree = json.loads(text.decode("utf-8"), object_pairs_hook=ObjectPairs,
                          parse_int=lambda s: ("number", s),
                          parse_float=lambda s: ("number", s),
                          parse_constant=lambda s: (_ for _ in ()).throw(ValueError(s)))
        nodes = 0

        def string(s):
            if any(0xD800 <= ord(c) <= 0xDFFF for c in s):
                raise ValueError("surrogate scalar")

        def visit(value, nesting):
            nonlocal nodes
            nodes += 1
            if nodes > 100_000:
                raise ValueError("nodes")
            if isinstance(value, (list, ObjectPairs)):
                nesting += 1
                if nesting > 128:
                    raise ValueError("depth")
            if isinstance(value, ObjectPairs):
                for key, child in value:
                    string(key)
                    visit(child, nesting)
            elif isinstance(value, list):
                for child in value:
                    visit(child, nesting)
            elif isinstance(value, str):
                string(value)
        visit(tree, 0)
        return True
    except (UnicodeError, ValueError, RecursionError):
        return False


def corpus(directory):
    directory.mkdir()
    cases = []

    def add(name, text, expected=None, old=True, bounds=None):
        data = text.encode("utf-8") if isinstance(text, str) else text
        oracle = strict(data)
        if expected is not None and oracle != expected:
            raise RuntimeError("oracle disagreement: " + name)
        path = directory / (name + ".json")
        path.write_bytes(data)
        cases.append({"name": name, "path": str(path), "expected": oracle,
                      "old_overlap": old, "bounds": bounds or ["1"]})

    for name, value in [
        ("empty", ""), ("whitespace", " \t\r\n"), ("null", "null"), ("true", "true"),
        ("false", "false"), ("object", "{}"), ("array", "[]"), ("scalar-string", '"x"'),
        ("nested", '{"a":[1,{"b":[true,false,null,"[]{}:,\\\"\\\\"]}]}'),
        ("duplicate", '{"a":1,"a":2}'), ("escaped-duplicate", '{"a":1,"\\u0061":2}'),
        ("keys-not-nodes", '{"a":1,"b":2,"c":[]}'),
        ("utf8", '"é 🛰️ 𝄞"'), ("pair", '"\\ud834\\udd1e"'),
        ("escaped-controls", '"\\u0000\\b\\f\\n\\r\\t"'),
        ("escapes", '"\\\"\\\\\\/"'),
    ]:
        add(name, value, bounds=["1", "17", "last"] if value else ["1", "17"])
    malformed = ["[", "{", "[1", '{"a"', '{"a":', '{"a":1', '[1,]', '{"a":1,}',
                 '[,1]', '{,}', '{"a" 1}', '{"a":1 "b":2}', '[1 2]', '[1}', '{"a":1]',
                 'true false', '{}[]', '01', '-01', '+1', '.1', '1.', '1e', '1e+', '1e-',
                 '--1', 'NaN', 'Infinity', '-Infinity', '/*x*/1', '"\\x20"', '"\\u12xz"',
                 '"\\ud800"', '"\\udc00"', '"\\ud800\\u0000"', '"\\udc00\\ud800"',
                 '"\\ud800x"', '"abc', '"abc\\', '"a\nb"', '"\x00"', '\x00', '1\x00',
                 '\v1', '\f1']
    for i, text in enumerate(malformed):
        add(f"malformed-{i}", text, False)
    for i, data in enumerate([b'"\xc0\x80"', b'"\xc1\xbf"', b'"\x80"', b'"\xff"',
                              b'"\xed\xa0\x80"', b'"\xf4\x90\x80\x80"', b'"\xf5\x80\x80\x80"',
                              b'"\xe0\x80\x80"', b'"\xf0\x80\x80\x80"', b'"\xc2x"',
                              b'"\xe2\x82"', b'"\xf0\x9f\x9b"', b'"\xe2x\xac"']):
        add(f"bad-utf8-{i}", data, False)
    for depth in [1, 30, 31, 32, 33, 63, 64, 65, 127, 128, 129]:
        add(f"arrays-depth-{depth}", '[' * depth + '0' + ']' * depth, depth <= 128)
        add(f"objects-depth-{depth}", '{"k":' * depth + '0' + '}' * depth, depth <= 128)
    for size in [1_048_575, 1_048_576, 1_048_577]:
        add(f"bytes-{size}", '"' + 'x' * (size - 2) + '"', size <= 1_048_576)
        add(f"spaces-{size}", '0' + ' ' * (size - 1), size <= 1_048_576)
    for nodes in [99_999, 100_000, 100_001]:
        add(f"array-nodes-{nodes}", '[' + ','.join(['0'] * (nodes - 1)) + ']', nodes <= 100_000,
            old=nodes <= 100_000)
        add(f"object-nodes-{nodes}", '{' + ','.join(f'"k{i}":0' for i in range(nodes - 1)) + '}',
            expected=None, old=False)
        add(f"duplicate-value-nodes-{nodes}", '{' + ','.join(['"k":0'] * (nodes - 1)) + '}',
            nodes <= 100_000, old=False)
    # ~50k keys would exceed Max_Nodes if keys were incorrectly counted.
    add("object-keys-not-values-limit", '{' + ','.join(f'"k{i}":0' for i in range(50_001)) + '}', True)
    numbers = ['-0', '-0.0', '5e-324', '1.7976931348623157e308', '1.0000000000000002',
               '0.9999999999999999', '2.2250738585072014e-308', '2.225073858507201e-308',
               '9223372036854775808', '18446744073709551616', '1e9999', '1e-9999']
    for i, token in enumerate(numbers):
        add(f"number-{i}", token, True, old=i not in [8, 9, 10])
        add(f"quoted-number-{i}", json.dumps(token), True)
    for s in ["NaN", "INF", "-INF"]:
        add("special-" + s.replace('-', 'minus'), json.dumps(s), True)
    rng = random.Random(9704)
    def value(depth=0):
        choices = [None, True, False, rng.randint(-10_000, 10_000), rng.random(),
                   "é 🛰 \"\\\x00,[]{}", "", "plain"]
        if depth < 4:
            choices += [[value(depth + 1) for _ in range(rng.randrange(4))],
                        {f"k{i}": value(depth + 1) for i in range(rng.randrange(4))}]
        return rng.choice(choices)
    for i in range(100):
        text = json.dumps(value(), ensure_ascii=bool(i % 2), separators=(',', ':'))
        add(f"random-{i}", text, True)
        add(f"random-trailing-{i}", text + ' null', False)
    atomic(directory / "cases.json", cases)
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--compiler", type=int, choices=[14, 16], required=True)
    parser.add_argument("--policies", nargs="+", default=["Normal", "Check", "Ignore"])
    parser.add_argument("--optimizations", nargs="+", default=["0", "2"])
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    rows = []
    def run(label, command, cwd=scratch):
        atomic(scratch / (label + ".command.json"), {"argv": command, "cwd": str(cwd)})
        with (scratch / (label + ".stdout")).open('wb') as out, (scratch / (label + ".stderr")).open('wb') as err:
            p = subprocess.run(command, cwd=cwd, stdout=out, stderr=err)
        (scratch / (label + ".exit")).write_text(str(p.returncode) + '\n')
        if p.returncode:
            raise RuntimeError(label)
        return (scratch / (label + ".stdout")).read_text()
    prior = Path(f"/home/zboll/task097-scratch/c3/composition-{args.compiler}-oam")
    ignore = shutil.ignore_patterns('obj', 'obj-*', 'lib', 'bin', '*.o', '*.ali', '*.a', '*.so', '*.lexch', '*.bexch')
    shutil.copytree(ROOT / 'ada/oms-json-support', scratch / 'support', ignore=ignore)
    shutil.copytree(prior / 'provider', scratch / 'provider', ignore=ignore)
    environment = scratch / 'environment.sh'
    environment.write_text((prior / 'environment.sh').read_text().replace(str(prior / 'provider'), str(scratch / 'provider')))
    shell = lambda cmd: ['sh', '-ec', f'. {shlex.quote(str(environment))}; exec ' + shlex.join(cmd)]
    run('compiler', shell(['gnatls', '-v']))
    cases = corpus(scratch / 'cases')
    for policy in args.policies:
        for optimization in args.optimizations:
            label = policy + '-O' + optimization
            src = scratch / label
            src.mkdir()
            for name in ['c4_validate.adb', 'c3_malloc_trace.c']:
                shutil.copy2(ROOT / 'tests/ada-float64-codec' / name, src)
            shutil.copy2(ROOT / 'tests/ada-float64-codec/c1_null_allocation_faults.ads', src / 'allocation_faults.ads')
            shutil.copy2(ROOT / 'tests/ada-float64-codec/c3_trace_faults.adb', src / 'allocation_faults.adb')
            adc = src / 'policy.adc'
            adc.write_text(f'pragma Assertion_Policy ({policy});\n' if policy != 'Normal' else '-- Normal\n')
            (src / 'proof.gpr').write_text('with "../support/ams_gra_oms_json_lossless";\nproject Proof is\n'
                'for Languages use ("Ada", "C"); for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ("c4_validate.adb");\n'
                'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-g", "-O' + optimization + '"); for Default_Switches ("C") use ("-g", "-O0"); end Compiler;\n'
                'package Linker is for Default_Switches ("Ada") use ("-Wl,--wrap=__gnat_malloc", "-no-pie"); end Linker;\nend Proof;\n')
            run(label + '-build', shell(['gprbuild', '-P', 'proof.gpr', '-p', '-j2', '-cargs:Ada', '-g', '-O' + optimization, '-gnatec=' + str(adc)]), src)
            binary = str(src / 'bin/c4_validate')
            for case in cases:
                for bound in case['bounds']:
                    out = run(label + '-' + case['name'] + '-' + bound, [binary, 'new', case['path'], '1000000', bound], src)
                    expected = 'C4_ACCEPT' if case['expected'] else 'C4_REJECT'
                    if out.splitlines()[0] != expected:
                        raise RuntimeError('oracle mismatch ' + label + ' ' + case['name'] + ' ' + out)
                if case['old_overlap'] and policy == 'Normal' and optimization == '0':
                    out = run('old-' + case['name'], [binary, 'old', case['path'], '1000000'], src)
                    # Preserve and report old semantic defects rather than disguise
                    # out-of-domain differences as ordinary parity.
                    rows.append({'case': case['name'], 'expected': case['expected'], 'old': out.splitlines()[0]})
            for depth in [1, 31, 32, 33, 64, 128]:
                path = scratch / 'cases' / f'arrays-depth-{depth}.json'
                if not path.exists():
                    path.write_text('[' * depth + '0' + ']' * depth)
                out = run(label + f'-allocations-{depth}', [binary, 'new', str(path), '1000000'], src)
                count = int(re.search(r'attempts=\s*(\d+)', out)[1])
                for budget in range(count):
                    out = run(label + f'-injection-{depth}-{budget}', [binary, 'inject', str(path), str(budget)], src)
                    if 'C4_EXCEPTION STORAGE_ERROR' not in out or 'C4_VALIDATION_RECOVERY_COMPLETE' not in out:
                        raise RuntimeError('resource/recovery failure')
            print('C4_DIFFERENTIAL_CELL_COMPLETE', args.compiler, label, len(cases), flush=True)
    differences = [r for r in rows if r['old'] != ('C4_ACCEPT' if r['expected'] else 'C4_REJECT')]
    atomic(scratch / 'old-differences.json', differences)
    atomic(scratch / 'result.json', {'completed': True, 'compiler': args.compiler, 'cases': len(cases), 'old_comparisons': rows})
    print('C4_DIFFERENTIAL_COMPLETE', flush=True)


if __name__ == '__main__':
    main()