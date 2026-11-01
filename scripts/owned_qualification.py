"""CI-SELECT-004 data-only registrations, shared by selection and verification.

Discovery never imports task Python or evaluates Ada/GPR/shell. Every byte below
an owned root must have one unambiguous registration. Unsupported inputs fail.
"""
import ast
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
MODES = ('fast', 'uci', 'sleet')
VERSIONS = {'14': '14.2.0', '16': '16.1.0'}
POLICIES = ('Normal', 'Check', 'Ignore')
OPTIMIZATIONS = ('0', '2')
NAME = re.compile(r'[a-z][a-z0-9]*(?:[-_][a-z0-9]+)*\Z')
FILE = re.compile(r'[a-z][a-z0-9_-]*\.[a-z0-9]+\Z')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pairs(items):
    result = {}
    for key, value in items:
        if key in result:
            raise ValueError('duplicate JSON key: ' + key)
        result[key] = value
    return result


def document(data):
    return json.loads(data, object_pairs_hook=pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def regular(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_nlink != 1:
        raise ValueError('unsafe/nonregular owned input: ' + str(path))
    if path.stat().st_mode & 0o111:
        raise ValueError('executable owned data/source: ' + str(path))
    return path


def directory(path):
    if path.is_symlink() or not path.is_dir():
        raise ValueError('missing/redirected owned directory: ' + str(path))
    return path


def names(value, pattern=FILE):
    if (not isinstance(value, list) or not value or
            any(not isinstance(item, str) or not pattern.fullmatch(item) for item in value) or
            value != sorted(set(value))):
        raise ValueError('missing, duplicate or noncanonical registered names')
    return value


def python_ids(path):
    """Independent source inventory; load_tests cannot hide source test methods."""
    tree = ast.parse(regular(path).read_text(), filename=str(path))
    identities = []
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == 'load_tests':
            raise ValueError('owned fixture tests cannot filter discovery with load_tests')
        if isinstance(node, ast.ClassDef):
            methods = [item.name for item in node.body
                       if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef))
                       and item.name.startswith('test_')]
            identities.extend(f'{path.stem}.{node.name}.{method}' for method in methods)
    if not identities or len(identities) != len(set(identities)):
        raise ValueError('empty/duplicate Python source test inventory')
    return sorted(identities)


def ada(root=ROOT):
    base = root / 'qualification/ada'
    if not base.exists():
        raise ValueError('missing owned Ada qualification namespace')
    directory(root / 'qualification')
    directory(base)
    if {p.name for p in base.iterdir()} != set(MODES):
        raise ValueError('unexpected/missing Ada mode registration')
    result, ids = [], set()
    for mode in MODES:
        tasks = sorted(directory(base / mode).iterdir())
        if not tasks:
            raise ValueError('missing Ada mode registrations: ' + mode)
        for task in tasks:
            directory(task)
            if not NAME.fullmatch(task.name) or task.name in ids:
                raise ValueError('malformed/duplicate Ada task identity')
            ids.add(task.name)
            manifest = regular(task / 'manifest.json')
            item = document(manifest.read_text())
            required = {'version', 'task_id', 'profile', 'sources', 'fixtures', 'tests', 'main'}
            if mode != 'fast':
                required |= {'release', 'contract'}
            if mode == 'uci':
                required |= {'cases', 'package', 'payload'}
            if not isinstance(item, dict) or set(item) != required:
                raise ValueError('unexpected/missing Ada manifest fields')
            profile = {'fast': 'ada-test-v1', 'uci': 'ada-codec-v1',
                       'sleet': 'ada-dlz-sleet-v1'}[mode]
            if type(item['version']) is not int or item['version'] != 1 or item['task_id'] != task.name or item['profile'] != profile:
                raise ValueError('wrong Ada registration identity/profile')
            sources = names(item['sources'])
            fixtures = item['fixtures']
            if fixtures != []:
                names(fixtures)
            names(item['tests'], NAME)
            if (any(Path(p).suffix not in ('.adb', '.ads') for p in sources) or
                    any(Path(p).suffix not in ('.json', '.txt', '.xsd', '.yaml') for p in fixtures) or
                    item['main'] not in sources or not item['main'].endswith('.adb') or
                    set(sources) & set(fixtures)):
                raise ValueError('unsupported/ambiguous Ada sources')
            expected = {'manifest.json', *sources, *fixtures}
            if {p.name for p in task.iterdir()} != expected:
                raise ValueError('unregistered/missing Ada source or fixture')
            for p in task.iterdir():
                regular(p)
            for name in sources:
                source_text = (task / name).read_text()
                if re.search(r'\bpragma\s+(Suppress|Assertion_Policy|Restrictions|Profile)\b', source_text, re.I):
                    raise ValueError('task source cannot override qualification execution policy')
            if mode == 'fast':
                literals = re.findall(r'"CI004_PASS ([a-z0-9_-]+)"',
                                      '\n'.join((task / p).read_text() for p in sources))
                if sorted(literals) != item['tests']:
                    raise ValueError('Ada test inventory differs from actual source')
            else:
                if item['release'] not in ('2.5', '2.6') or item['contract'] not in fixtures:
                    raise ValueError('unsupported pinned release/contract')
                if mode == 'uci':
                    if (item['cases'] not in fixtures or
                            not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)*', item['package']) or
                            not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*', item['payload'])):
                        raise ValueError('unsafe codec registration')
                    main_source = (task / item['main']).read_text()
                    if not all(token in main_source for token in
                               (item['package'] + '.' + item['payload'],
                                'Service_Codec.Decode_' + item['payload'],
                                'Service_Codec.Encode_' + item['payload'])):
                        raise ValueError('registered generated codec is not the actual consumer')
                    cases = document((task / item['cases']).read_text())
                    if not isinstance(cases, list) or not cases:
                        raise ValueError('missing required codec cases')
                    for case in cases:
                        if (not isinstance(case, dict) or set(case) != {'id', 'input', 'expected'} or
                                not isinstance(case['input'], str) or '\n' in case['input']):
                            raise ValueError('malformed codec case')
                    if (any(not isinstance(c['id'], str) or not NAME.fullmatch(c['id']) for c in cases) or
                            sorted(c['id'] for c in cases) != item['tests'] or
                            not any(c['expected'] is None for c in cases) or
                            not any(c['expected'] is not None for c in cases)):
                        raise ValueError('missing/duplicate registered codec case')
                elif item['tests'] != ['broker-rejection', 'decoder-rejection', 'transport-recovery']:
                    raise ValueError('DLZ profile requires independent transport stage controls')
            result.append(dict(language='ada', mode=mode, path=task.relative_to(root).as_posix(),
                               manifest=item, hashes={p.relative_to(root).as_posix(): digest(p.read_bytes())
                                                      for p in sorted(task.iterdir())}))
    return result


def python_fixtures(root=ROOT):
    result = []
    for path in (root / 'python', root / 'python/qualification'):
        if path.exists() or path.is_symlink():
            directory(path)
    for mode in MODES:
        base = root / 'python/qualification' / mode
        if not base.exists():
            continue
        directory(base)
        fixtures = base / 'fixtures'
        if not fixtures.exists() and not fixtures.is_symlink():
            continue
        for task in sorted(directory(fixtures).iterdir()):
            directory(task)
            if not re.fullmatch(r'test_[a-z0-9_]+', task.name):
                raise ValueError('invalid Python fixture owner')
            test = regular(base / (task.name + '.py'))
            manifest = regular(task / 'manifest.json')
            item = document(manifest.read_text())
            if (not isinstance(item, dict) or set(item) != {'version', 'test', 'files', 'tests'} or
                    type(item['version']) is not int or item['version'] != 1 or item['test'] != test.name):
                raise ValueError('malformed Python fixture registration')
            names(item['files'])
            if any(Path(p).suffix not in ('.json', '.txt', '.xsd', '.yaml') for p in item['files']):
                raise ValueError('unsupported Python owned fixture')
            if {p.name for p in task.iterdir()} != {'manifest.json', *item['files']}:
                raise ValueError('missing/unregistered Python owned fixture')
            if item['tests'] != python_ids(test):
                raise ValueError('Python manifest omits actual source tests')
            paths = [test, *sorted(task.iterdir())]
            result.append(dict(language='python', mode=mode, path=task.relative_to(root).as_posix(),
                               manifest=item, hashes={p.relative_to(root).as_posix(): digest(regular(p).read_bytes())
                                                      for p in paths}))
    return result


def registrations(root=ROOT):
    return ada(root) + python_fixtures(root)


def ownership(path, root=ROOT):
    matches = [r for r in registrations(root) if path in r['hashes']]
    if len(matches) != 1:
        raise ValueError('missing/ambiguous owned registration: ' + path)
    record = matches[0]
    language, mode = record['language'], record['mode']
    jobs = ['rust', 'ada-lacal-runtime-core' if language == 'ada' else 'python-model-backend']
    deep = [] if mode == 'fast' else [
        'ada-real-uci-dlz-sleet' if language == 'ada' else
        'real-uci-regressions' if mode == 'uci' else 'real-sleet']
    return dict(category=language + '-owned-' + mode, jobs=jobs, deep_jobs=deep,
                rationale='Validated source-bound CI-SELECT-004 registration', deep=bool(deep),
                deep_rationale='Fixed existing qualification owner')


def cells(record):
    return [f'{record["manifest"]["task_id"]}/{policy}/O{optimization}'
            for policy in POLICIES for optimization in OPTIMIZATIONS]