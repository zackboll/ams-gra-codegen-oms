"""Fail-closed bounded workflow policy, stdlib only.

This is deliberately NOT a general YAML loader. We accept the repository's
explicit block-form jobs and inventory-pinned payloads. Unsupported aliases,
flow jobs, tags, dynamic matrices, or new workflows require policy review.
"""
import json
import hashlib
from pathlib import Path
import re
from ci_selection import ROOT, inventory, ordinary_jobs, deep_jobs, deep_condition, select

# Independently reconciled from d603077c6d38ccc0658391776e3af9eecb009255
# (Task099/security) and 904ad3baebbf237a91e118c91457ac7207e8edd9 (Task100).
# These are normalized ORIGINAL payloads, not candidate workflow snapshots.
INHERITED_PAYLOADS = {
    # PR #117 payloads independently derived from b6c6c9c5, not this candidate.
    ('ci.yml', 'python-model-backend'): '901f3eb0d236d16ec4ae928f069f5ffc046467b8c6834c216d69bea309c593e8',
    ('deep-ci.yml', 'real-uci-regressions'): 'ecd52d12535fd41daf7adb6ef94127ff10bb61a921f8ebb543edb9176c47304f',
    ('deep-ci.yml', 'real-sleet'): '6e038fe34b8548d5d23836d29766118700dfd69ce8ddeff764ef41df9e7d8694',
    ('ci.yml', 'rust'): '572db2f365338fb91318ed95931e2bc5844bbf17a7b92338a475d5382b0f2bf2',
    ('ci.yml', 'ada-websocket-transport'): '866e61d53974f75501df8e516a273a1af63206eb476a190d6ad641f1ae451a3e',
    ('deep-ci.yml', 'ada-websocket-real-sleet'): 'deb7a055c9dd6b268fe04fce07791132d3d7a46c77e9a0adacf78bb4c1a07816',
    ('ci.yml', 'ada-lacal-runtime-core'): 'c30d66a0dc4fdf56313964f98407784670cd42b066d3ed1d0fddd5a3a07a1368',
    ('ci002-validation.yml', 'test'): '9b0173054780810fd2541b0eebcb67d51c41173bc0a3e8d25beb069191f0e2ca',
    ('oms-trusted-pr-signal.yml', 'signal'): 'ba84d00fbfcc1b93274e2077c7c91bffb31a810828d1132e4e87ffdb46acb585',
    ('oms-ryzen-trusted-dispatch.yml', 'dispatch'): '837b0a7ed7ff1d6289f3c521e2fffd7548eecbd6a26cccda228e9732fff114ac',
}

TASK100_INTEGRATED_PAYLOADS = {
    ('ci.yml', 'ada-websocket-transport'): 'd6ef0377f5465890e7f45b9085bbd8d1a46f2cc4a6c395c0e4fefcbd24328ef7',
    ('deep-ci.yml', 'ada-websocket-real-sleet'): 'c25d0bd21c201cdf5bc28c3f94952de286e4dfea0b33235cd26c71cf174efa62',
}

# Independently frozen from the qualified R1 commit af467b4789758cab544829a15bc0ae5d2d96cbba.
# These never replace INHERITED_PAYLOADS: both historical and additive contracts
# must pass. The projection below separately proves preservation of every step.
PY008_INTEGRATED_PAYLOADS = {
    ('ci.yml', 'python-model-backend'): '5c2c35347a3557256a54f5c37fdeaf1b18ea663ec7c15e4fe1243f664b91492d',
    ('deep-ci.yml', 'real-uci-regressions'): '8d1d46b8fa71b89350a728df0046debfb39fc3fd595e282e54a85e7d5d09f61a',
    ('deep-ci.yml', 'real-sleet'): 'b654da26b40c746eefd0d4a8981671b5310a2232f53296e27273875c335a27b5',
}

# Parsed step contracts frozen from those same main/R1 Git objects. The final
# gate consumes these arrays, so YAML-only pins cannot authorize their drift.
PY008_STEP_CONTRACTS = {
    'python-model-backend': (
        'e0f44960f5d46519e51c40172f217cf3a02526e806fcff80e70d5cee629a6968',
        '4c9707160f63fc619c7148b4a1f153d790f450d7552af6dd1556e25c95b9a119'),
    'real-uci-regressions': (
        '18d7fe83c3b91ab21bdca922094444c776ed1b1df3b9fcb437d5cd5fa205cc74',
        'a18c2685e241e6d9707737c71765e4c2a379db4cfd14aacdaee11c1f06e4620b'),
    'real-sleet': (
        '99de431b6580e1c396fd6f7a4b11ec6865a89a2a5ad16a3ab6b9cda4b37cf7e9',
        '3f64250f508e3b80c5001d4b7b7a6df8e0e69e7a668fbed69cb5a5ddf63ac1f3'),
}

# Merged RUST-CI-001 source identities, independently taken from main 7958c4e0.
RUST_CI001_SOURCES = {
    'crates/runtime-rust-facade-tests/tests/common/mod.rs': '9ff7ce54e71ca807b06f5387f14fff13e376a783d95bd8bb63d6a90c9f7e20b1',
    'crates/runtime-rust-facade-tests/tests/common/worker_join.rs': 'ddb37d74a78856e67478064a1f9923918d4b4063002f0c2d93d063c856a81dc8',
    'crates/runtime-rust-facade-tests/tests/runtime_fairness.rs': '0fc3be00ccedbfa7cb4ff8aadeb14829d8b21e063c98ddd933f47df400360a0b',
    'crates/runtime-rust-facade-tests/tests/runtime_initial_retry.rs': '026ea1e7ae49957e40179f3bc9202efbcf2a8a1cd972df2c2576a0005c04514f',
    'crates/runtime-rust-facade-tests/tests/runtime_lifecycle.rs': 'b1bd1b8165014f4c333faa6b2fc8b36669a603438a7176acbfa442ccc3b8761d',
}

SECURITY_SOURCES = {
    'scripts/ci002/test_workflows.py': 'a7c34b958fcbe113e79bbb8e6389e4b9950f52bc9d78733b731d391afa5be5ff',
    'scripts/ci002/test_mutations.py': '86020a418a0dfd202a040aeed2d17fb3044359d306eae799ced1db9f898220f1',
    'scripts/ci002/test_gate.py': '19bb7c6ff89bf58679e0959ea227065a94aebf06c33752b10a436ec3e792a87e',
    'scripts/ci002/test_dispatch.py': 'f6e3db711aa4b5dbe30f87554f00ee0dea62da37348b8dc60d4ce2d80c4fbee5',
    'scripts/ci002/requirements.txt': '9057bfbc5f7470fb710e417b3865d5de491865ea13beb096c2a2c3f1b7418a12',
    'scripts/ci002/gate.py': '6b85173313501c7161284fc16d054466ea6a5af6df970792e8354b30b895a0f8',
    'scripts/ci002/dispatch.py': '09e984d2f7bc0695f235871c2afb26f455f3b0f3e1eba8a8b1d9c9e6a25a6921',
}


def blocks(text):
    code = "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("#")) + "\n"
    if len(re.findall(r"^jobs:\s*$", code, re.M)) != 1:
        raise ValueError("exactly one explicit jobs block required")
    header, body = code.split("jobs:\n")
    entries = list(re.finditer(r"^  ([\w-]+):\n", body, re.M))
    if not entries or body[:entries[0].start()].strip():
        raise ValueError("unsupported jobs layout")
    jobs = {}
    for n, match in enumerate(entries):
        name = match[1]
        if name in jobs:
            raise ValueError("duplicate job: " + name)
        jobs[name] = body[match.end():entries[n+1].start() if n+1<len(entries) else len(body)]
    return header, jobs


def normalized(text):
    return "\n".join(line.rstrip() for line in text.splitlines()
                     if line.strip() and not line.lstrip().startswith("#"))


def expected_group(workflow, job_id, cells):
    if len(cells) > 1:
        axis = next(iter(cells[0]["variant"]))
        main = "format('oms-main-slot-{0}', matrix.slot)"
        other = f"format('oms-pr-{{0}}-{{1}}-{{2}}-{workflow}-{job_id}-{{3}}', github.workflow, github.run_id, github.run_attempt, matrix.{axis})"
    else:
        main = f"'oms-main-slot-{cells[0]['lane']}'"
        other = f"format('oms-pr-{{0}}-{{1}}-{{2}}-{workflow}-{job_id}', github.workflow, github.run_id, github.run_attempt)"
    return "${{ github.ref == 'refs/heads/main' && " + main + " || " + other + " }}"


def field(header, key):
    return re.findall(r"^    " + re.escape(key) + r":(?: (.*))?$", header, re.M)


PY008_SURFACES = (
    'crates/backend-python/src/float64_runtime.py',
    'crates/backend-python/src/float64_codec_runtime.py',
    'crates/backend-python/src/lib.rs', 'crates/backend-python/src/names.rs',
    'crates/backend-python/src/service_codec.rs',
    'crates/backend-python/tests/models.rs', 'crates/cli/tests/python_float64.rs',
    'python/oms-lacal-runtime/tests/test_float64_sleet.py',
    'scripts/check-py008-numeric.py', 'scripts/check-py008-uci.py',
    'scripts/measure-py008-uci.py', 'scripts/test-py008-uci.py',
    'scripts/check-py008-parity.py', 'scripts/check-py008-sleet.sh',
    'tests/fixtures/python-float64/after.json',
    'tests/fixtures/python-float64/before.json',
    'tests/fixtures/python-float64/nonfloat-parity.json',
    'tests/fixtures/python-float64/legacy-parity.json',
    'tests/fixtures/python-float64/oam.xsd',
    'tests/ada-float64-probe/oracle.py',
    'tests/ada-float64-probe/corpus-manifest.json',
)


def check_py008_sleet(text):
    # Verify each additive contract once, newest first, before historical pins.
    from py010_ci import project_sleet as project_py010_sleet
    text = project_py010_sleet(text)
    from ci_select003 import project_sleet as project_sleet003
    text = project_sleet003(text)
    from py009_ci import project_sleet
    text = project_sleet(text)
    # Bind execution to the existing version loop and shared pinned build.
    # A comment, disabled command, filter or invocation outside this loop is
    # not a substitute for executing PY-006/007/008 for every interpreter.
    required = '''for interpreter in ${PY006_PYTHONS:-${PYTHON:-python3}}; do
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py006-sleet.sh"
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py007-sleet.sh"
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py008-sleet.sh"
done'''
    # Original qualified PY-008 harness, b9d9fcf74ace0a3063ec28d823979adf41fc166d.
    # Also reject an outer false condition/early exit around an unchanged loop.
    digest = hashlib.sha256(normalized(text).encode()).hexdigest()
    if (normalized(text).count(required) != 1 or
            digest != 'b34881647d8971ede9e8addeb05e8827f3ec206b737bda597e292b03d51d6b4a'):
        raise ValueError('PY-008 shared-Sleet execution changed')


def check_py008_integration(record):
    """Verify the reviewed additive delta independently of its pinned digest."""
    job = record['job_id']
    original = record['original_steps']
    integrated = record['integration_steps']
    projected = json.loads(json.dumps(integrated))
    if job == 'python-model-backend':
        if projected[1].get('uses') != 'dtolnay/rust-toolchain@1.95.0':
            raise ValueError('PY-008 requires reviewed Rust 1.95.0 pin')
        projected[1]['uses'] = 'dtolnay/rust-toolchain@stable'
        focused = [s for s in projected if s.get('name') == 'Focused independent Float64 numeric controls (PY-008)']
        if len(focused) != 1:
            raise ValueError('PY-008 focused numerical step missing/duplicate')
        projected.remove(focused[0])
        additions = ['cargo test --locked -p ams-gra-codegen-oms --test python_float64 -- --nocapture\n',
                     'python scripts/test-py008-uci.py\n']
    elif job == 'real-uci-regressions':
        step = next(s for s in projected if 'PY-007 and PY-008' in s.get('name', ''))
        step['name'] = step['name'].replace('PY-007 and PY-008', 'PY-007')
        additions = ['python3 scripts/test-py008-uci.py\n',
                     ' --service-baseline tests/fixtures/python-float64/after.json',
                     'python3 scripts/check-py008-uci.py --generator target/release/ams-gra-codegen-oms \\\n  --output "$RUNNER_TEMP/py006-uci" --existing-output\n']
    else:
        full = [s for s in projected if s.get('name') == 'Full independent Python Float64 corpus (PY-008)']
        artifacts = [s for s in projected if s.get('name') == 'Preserve Python Float64 numeric evidence']
        if len(full) != 1 or len(artifacts) != 1:
            raise ValueError('PY-008 full numerical/evidence step missing/duplicate')
        projected.remove(full[0])
        projected.remove(artifacts[0])
        additions = []
    for addition in additions:
        owners = [s for s in projected if addition in s.get('run', '')]
        if len(owners) != 1 or owners[0]['run'].count(addition) != 1:
            raise ValueError('PY-008 mandatory additive command missing/duplicate: ' + job)
        owners[0]['run'] = owners[0]['run'].replace(addition, '', 1)
    def comparable(steps):
        result = json.loads(json.dumps(steps))
        for step in result:
            if 'run' in step:
                step['run'] = normalized(step['run'])
        return result
    if comparable(projected) != comparable(original):
        raise ValueError('PY-008 integration removed/replaced inherited step: ' + job)


def check_rust_ci001(sources=None):
    for path, digest in RUST_CI001_SOURCES.items():
        data = sources[path] if sources is not None else (ROOT / path).read_bytes()
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError('RUST-CI-001 worker-join qualification source changed: ' + path)


def check(policy=None, workflows=None, sleet=None):
    policy = policy or inventory()
    original_headers = {r['job_id']: normalized(r['expected_header'].replace(
        '    if: ' + deep_condition(r['job_id']) + '\n', '').replace(
        'if: ' + deep_condition(r['job_id']) + '\n', ''))
        for r in policy['jobs'] if r['workflow'] == 'deep-ci.yml' and r['job_id'] != 'ada-real-uci-dlz-sleet'}
    if hashlib.sha256(json.dumps(original_headers, sort_keys=True).encode()).hexdigest() != '3946b92fb805271be235bde5170f0fa483ce3c8a56fa3488b4171ee004a7f22a':
        raise ValueError('immutable main Deep runner/matrix/header inventory changed')
    if hashlib.sha256(policy['deep_header'].encode()).hexdigest() != '41b85c9ba045f75f412929efd73e9de54977b77ccddaef8a52a95e698728ed72':
        raise ValueError('reviewed Deep input contract changed')
    if hashlib.sha256(policy['deep_caller'].encode()).hexdigest() != 'a343b3f71a5299740901fef920a1121757a44872850f1c07c1165d41e3170264':
        raise ValueError('reviewed Deep caller input changed')
    from ci_select003 import RULES as CI003_RULES
    if policy['rules'][:len(CI003_RULES)] != CI003_RULES:
        raise ValueError('CI-SELECT-003 reviewed Python Deep ownership changed')
    inherited_rules = policy['rules'][len(CI003_RULES):]
    rules = [{k: r[k] for k in ('category', 'paths', 'deep_jobs', 'deep_rationale')} for r in inherited_rules]
    # Main fc36c97f independently pins the inherited Task101 ownership extension.
    # Keep that authority separate from CI-SELECT-003's four exact prefix rules.
    if hashlib.sha256(json.dumps(rules, sort_keys=True).encode()).hexdigest() != '2fb1c569eaf4c5ef0f0ebf5840c85c8f89395b67ef4517f32eefce60f5b82469':
        raise ValueError('reviewed Deep dependency ownership changed')
    from real_uci_evidence import SHARDS
    for job_id, record in deep_jobs(policy).items():
        wanted = sorted(set(SHARDS) - {job_id}) if job_id in SHARDS else []
        if record['prerequisites'] != wanted:
            raise ValueError('Deep evidence dependency closure changed')
    if (policy.get('ci_select_002_main'), policy.get('ci_select_002_main_tree')) != (
            '7958c4e0d8f78768fd4c7d4c1bd34877c31d893a', 'a94fc3bb844349780772566eafd50db7a7328efe'):
        raise ValueError('CI-SELECT-002 incorporated main identity changed')
    check_rust_ci001()
    if policy.get('security_source_sha256') != SECURITY_SOURCES:
        raise ValueError('missing/modified inherited CI002 source inventory')
    for path, digest in SECURITY_SOURCES.items():
        if hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('inherited trusted-dispatch source changed: ' + path)
    if workflows is None:
        workflows = {p.name: p.read_text() for p in (ROOT / '.github/workflows').iterdir() if p.suffix in ('.yml', '.yaml')}
    from owned_qualification import registrations
    registrations(ROOT)
    from py010_ci import project as project_py010
    workflows = project_py010(workflows)
    from ci_select003 import project as project003
    workflows = project003(workflows)
    from py009_ci import project
    workflows = project(workflows)
    from real_uci_evidence import check_layout
    check_layout(blocks(workflows['deep-ci.yml'])[1])
    if policy['lanes'] != [f'oms-main-slot-{n}' for n in range(1, 6)]:
        raise ValueError("must define exactly five unique common lanes")
    if (policy.get('reconciliation_main'), policy.get('reconciliation_main_tree')) != (
            'b6c6c9c5b1fabcd0716c1fd1183cc411d8841872',
            '9dc99ef6531ab61b248f83b5f613e981e4002bc0'):
        raise ValueError('verified PR117 reconciliation base identity changed')
    security = {'ci002-validation.yml', 'oms-trusted-pr-signal.yml', 'oms-ryzen-trusted-dispatch.yml'}
    if set(policy.get('security_headers', {})) != security:
        raise ValueError('required independent security workflow removed')
    runtime = [r for r in policy['jobs'] if (r['workflow'], r['job_id']) == ('ci.yml', 'ada-lacal-runtime-core')]
    if len(runtime) != 1 or [(c['key'], c['name'], c['variant']) for c in runtime[0]['cells']] != [
            ('ada-lacal-runtime-core-' + v, 'ada-lacal-runtime-core (' + v + ')', {'compiler': v}) for v in ('14', '16')]:
        raise ValueError('Task099 requires exactly GNAT14/GNAT16 qualification cells')
    expected = {}
    task101 = [r for r in policy['jobs'] if r['job_id'] == 'ada-real-uci-dlz-sleet']
    if len(task101) != 1 or task101[0]['workflow'] != 'deep-ci.yml':
        raise ValueError('Task101 mandatory independent Deep job absent/duplicate')
    task101 = task101[0]
    if [(c['variant'], c['lane'], c['artifacts']) for c in task101['cells']] != [
            ({'compiler': c}, lane, ['task101-ada-real-uci-dlz-' + c]) for c, lane in [('14', 3), ('16', 5)]]:
        raise ValueError('Task101 mandatory compiler/artifact/lane inventory changed')
    # R1 uses the inherited producer publication name so CI-BENCH's immutable
    # provenance source remains byte-identical. Project exactly that one name
    # back to Task101's original payload; retain its original hash authority.
    task101_steps = task101['original_steps_yaml']
    hidden_sources = ('          name: task101-ada-real-uci-dlz-${{ matrix.compiler }}\n'
                      '          include-hidden-files: true\n')
    if task101_steps.count(hidden_sources) != 1:
        raise ValueError('Task101 hashed hidden provider evidence must be uploaded')
    task101_steps = task101_steps.replace(hidden_sources,
        '          name: task101-ada-real-uci-dlz-${{ matrix.compiler }}\n')
    publication = '      - name: Upload mandatory source compiler and execution evidence\n'
    if task101_steps.count(publication) != 1:
        raise ValueError('Task101 mandatory publication contract changed')
    task101_steps = task101_steps.replace(publication,
        '      - name: Preserve Task101 mandatory source and execution evidence\n')
    if (hashlib.sha256(task101['expected_header'].encode()).hexdigest() != 'a93ce7ff40b525284f849b9946c603b65602860a0e3e88156d4a22535e8c3e23'
            or hashlib.sha256(normalized(task101_steps).encode()).hexdigest() != '62ce975ac7d3ffdbf7fd6bbcdd360c2fb8ccadd360fbdd29b0fa9c63b57ec31e'):
        raise ValueError('Task101 mandatory qualification header/payload changed')
    for workflow, job_id, kind in [('ci.yml', 'ada-websocket-transport', 'transport'),
                                   ('deep-ci.yml', 'ada-websocket-real-sleet', 'sleet')]:
        records = [r for r in policy['jobs'] if (r['workflow'], r['job_id']) == (workflow, job_id)]
        if len(records) != 1 or [(c['key'], c['name'], c['variant'], c['artifacts'])
                                for c in records[0]['cells']] != [
                (job_id + '-' + v, job_id + ' (' + v + ')', {'compiler': v},
                 ['task100-ada-websocket-' + kind + '-' + v]) for v in ('14', '16')]:
            raise ValueError('Task100 requires exactly GNAT14/GNAT16 cells and artifacts: ' + job_id)
    if 'ada-websocket-transport' not in policy['orchestration_jobs']['pr-ci-required']['prerequisites']:
        raise ValueError('Task100 missing from required aggregate dependencies')
    for record in policy['jobs']:
        key = record['workflow'], record['job_id']
        if key in expected:
            raise ValueError('duplicate runner inventory job')
        if key in INHERITED_PAYLOADS and hashlib.sha256(normalized(record['original_steps_yaml']).encode()).hexdigest() != INHERITED_PAYLOADS[key]:
            raise ValueError('verified current-main baseline payload changed: ' + record['job_id'])
        if key in PY008_INTEGRATED_PAYLOADS:
            if hashlib.sha256(normalized(record.get('integration_steps_yaml', '')).encode()).hexdigest() != PY008_INTEGRATED_PAYLOADS[key]:
                raise ValueError('PY-008 integration payload changed: ' + record['job_id'])
            for field_name, digest in zip(('original_steps', 'integration_steps'), PY008_STEP_CONTRACTS[record['job_id']]):
                if hashlib.sha256(json.dumps(record.get(field_name), sort_keys=True).encode()).hexdigest() != digest:
                    raise ValueError('PY-008 aggregate step contract changed: ' + record['job_id'])
            check_py008_integration(record)
        expected[key] = record
    deep_baseline = [{k: r[k] for k in ('job_id', 'cells', 'original_steps_yaml', 'original_steps')}
                     for r in policy['jobs'] if r['workflow'] == 'deep-ci.yml' and r['job_id'] != 'ada-real-uci-dlz-sleet']
    # Preserve the frozen 7958c4e0 baseline rather than reattribute it to this
    # merge. Main 3f69cb0e adds exactly one PY-008 artifact to real-sleet;
    # its independently pinned additive payload/step contracts are checked below.
    deep_baseline = json.loads(json.dumps(deep_baseline))
    sleet_cells = next(r['cells'] for r in deep_baseline if r['job_id'] == 'real-sleet')
    if len(sleet_cells) != 1 or sleet_cells[0]['artifacts'] != ['py008-numeric', 'py010-sleet']:
        raise ValueError('PY-008 mandatory numeric artifact inventory changed')
    sleet_cells[0]['artifacts'] = []
    if hashlib.sha256(json.dumps(deep_baseline, sort_keys=True).encode()).hexdigest() != '4850f5e34beca0c5ab52c1152fa84a2ad1d1e8883d24a8bece9a4f206ea799f0':
        raise ValueError('immutable main Deep cell/qualification inventory changed')
    for job_id, record in policy['orchestration_jobs'].items():
        expected[record['workflow'], job_id] = dict(cells=[dict(lane=record['lane'])])
    from ci_bench.policy import check as benchmark_policy, WORKFLOW as benchmark_workflow
    benchmark_policy(policy, workflows, ROOT)
    if set(workflows) != {w for w, _ in expected} | {benchmark_workflow}:
        raise ValueError('unmapped workflow runner inventory')
    count = 0
    for workflow, text in workflows.items():
        if workflow == benchmark_workflow:
            # Manual-only infrastructure is registered separately, never part of
            # automatic PR/main selection or mandatory qualification aggregation.
            continue
        header, jobs = blocks(text)
        required = {j for w,j in expected if w==workflow} | set(policy['reusable_jobs'].get(workflow, {}))
        if set(jobs) != required:
            raise ValueError('unmapped/missing workflow job: ' + workflow)
        if workflow=='ci.yml':
            if normalized(header) != policy['ci_header']:
                raise ValueError('ordinary main/PR trigger or cancellation policy changed')
            if not re.search(r'^  pull_request:\s*$', header, re.M) or '    paths:' in header:
                raise ValueError('ordinary PR workflow must always trigger')
            if "group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}" not in header or "cancel-in-progress: ${{ github.event_name == 'pull_request' }}" not in header:
                raise ValueError('main runs must not cancel/replace earlier qualification')
        elif workflow=='deep-ci.yml':
            if normalized(header) != policy['deep_header']:
                raise ValueError('Deep reusable input/trigger/permissions changed')
            if 'concurrency:' in header or '  workflow_call:\n' not in header or '  pull_request:' in header or '  push:' in header:
                raise ValueError('Deep caller owns full merge/PR qualification and cancellation')
        elif workflow in policy.get('security_headers', {}):
            if normalized(header) != policy['security_headers'][workflow]:
                raise ValueError('security trigger/permissions/cancellation policy changed: ' + workflow)
        else:
            if normalized(header) != policy['pages_header']:
                raise ValueError('Pages publication trigger/latest-version policy changed')
        for job_id, body in jobs.items():
            if job_id in policy['reusable_jobs'].get(workflow, {}):
                if normalized(body) != policy['deep_caller']:
                    raise ValueError('mandatory complete Deep dependency changed')
                continue
            record = expected[workflow,job_id]
            cells = record['cells']
            main_eligible = record.get('main_eligible', True)
            if main_eligible and any(type(c.get('lane')) is not int or not 1<=c['lane']<=5 for c in cells):
                raise ValueError('invalid/missing matrix lane: ' + job_id)
            if not main_eligible and (workflow != 'oms-trusted-pr-signal.yml' or any(c.get('lane') is not None for c in cells)):
                raise ValueError('only the PR signal may be excluded from main lanes')
            if '    steps:\n' not in body:
                raise ValueError('runner job has no explicit steps')
            top, steps = body.split('    steps:\n',1)
            concurrency = re.findall(r'^    concurrency:\n((?:      [^\n]*\n)+)', top, re.M)
            want = '      group: '+expected_group(workflow,job_id,cells)+'\n      queue: max\n'
            if not main_eligible:
                want = "      group: ${{ format('oms-pr-{0}-{1}-{2}-oms-trusted-pr-signal.yml-signal', github.workflow, github.run_id, github.run_attempt) }}\n      queue: max\n"
            if concurrency != [want]:
                raise ValueError('job bypasses shared lanes / pending preservation: ' + job_id)
            if 'continue-on-error:' in top or 'cancel-in-progress:' in top:
                raise ValueError('job hides failure or cancels lane: '+job_id)
            if 'original_steps_yaml' in record:
                if normalized(top) != record['expected_header']:
                    raise ValueError('qualification runner/matrix/dependency policy changed: '+job_id)
                pinned = record['original_steps_yaml']
                if (workflow, job_id) in TASK100_INTEGRATED_PAYLOADS:
                    pinned = record['integration_steps_yaml']
                    if hashlib.sha256(normalized(pinned).encode()).hexdigest() != TASK100_INTEGRATED_PAYLOADS[workflow, job_id]:
                        raise ValueError('Task100 integration payload changed: ' + job_id)
                if (workflow, job_id) in PY008_INTEGRATED_PAYLOADS:
                    pinned = record['integration_steps_yaml']
                if record.get('source_policy') != 'exact PR head or main SHA':
                    pinned = pinned.replace('github.event.pull_request.head.sha || github.sha','github.sha')
                if normalized('    steps:\n'+steps) != normalized(pinned):
                    raise ValueError('inherited qualification payload/evidence changed: '+job_id)
                if field(top,'runs-on') != [record['original_job']['runs-on']]:
                    raise ValueError('compiler runner placement changed: '+job_id)
                if workflow=='ci.yml':
                    wanted_needs = '['+', '.join(['changes',*record['prerequisites']])+']'
                    wanted_if = "github.event_name != 'pull_request' || fromJSON(needs.changes.outputs.manifest).jobs['"+job_id+"']"
                    if field(top,'needs') != [wanted_needs] or field(top,'if') != [wanted_if]:
                        raise ValueError('unsafe selection/prerequisite/full-main condition: '+job_id)
                elif workflow=='deep-ci.yml':
                    wanted_if = deep_condition(job_id)
                    if field(top, 'if') != [wanted_if] or record.get('selection_condition') != wanted_if:
                        raise ValueError('unsafe Deep job selection condition: ' + job_id)
            else:
                if normalized(body) != policy['orchestration_payloads'][job_id]:
                    raise ValueError('mandatory baseline/final gate changed: '+job_id)
            if len(cells)>1:
                axis = next(iter(cells[0]['variant']))
                values = '['+', '.join("'"+str(c['variant'][axis])+"'" for c in cells)+']'
                matrix = '      matrix:\n        '+axis+': '+values+'\n        include:\n'
                matrix += ''.join(f"          - {axis}: '{c['variant'][axis]}'\n            slot: {c['lane']}\n" for c in cells)
                if top.count(matrix)!=1 or top.count('      matrix:')!=1:
                    raise ValueError('missing/duplicate/invalid expanded matrix lane: '+job_id)
            elif 'matrix:' in top:
                raise ValueError('unmapped matrix expansion: '+job_id)
            count += len(cells)
    jobs = ordinary_jobs(policy)
    if not {'scripts/check-task061-service-impact.py', 'scripts/test-task061-service-impact.py'} <= set(policy['deep_pr_paths']):
        raise ValueError('lost inherited Deep Task061 PR paths')
    for r in policy['rules']:
        if not set(r['jobs']) <= jobs.keys():
            raise ValueError('unmapped source dependency')
        if (r.get('deep_jobs') != sorted(set(r.get('deep_jobs', [])))
                or not set(r['deep_jobs']) <= deep_jobs(policy).keys()
                or bool(r['deep_jobs']) != r['deep'] or not r.get('deep_rationale')):
            raise ValueError('unmapped Deep source dependency')
        sample = select([dict(status='M', paths=[r['paths'][0].replace('**','example').replace('*','example')],
                              old_mode='100644', new_mode='100644')],policy)
        if not all(sample['jobs'][j] for j in r['jobs']):
            raise ValueError('source rule shadows required dependency: '+r['category'])
    for path in PY008_SURFACES:
        manifest = select([dict(status='M', paths=[path], old_mode='100644', new_mode='100644')], policy)
        if not manifest['deep'] or not {'python-model-backend-3.11', 'python-model-backend-3.13'} <= set(manifest['cells']):
            raise ValueError('PY-008 source avoids Python/Deep qualification: ' + path)
    for mode in ('fast', 'uci', 'sleet'):
        if not (ROOT / ('python/qualification/' + mode + '/test_' + mode + '_contract.py')).is_file():
            raise ValueError('CI-SELECT-003 initial mandatory test registration missing: ' + mode)
    if not (ROOT / 'python/qualification/run.py').is_file():
        raise ValueError('CI-SELECT-003 mandatory runner missing')
    check_py008_sleet(sleet if sleet is not None else (ROOT / 'scripts/run-real-sleet-test.sh').read_text())
    return count


if __name__ == '__main__':
    print('CI_POLICY_COMPLETE runner_cells=' + str(check()) + ' maximum_main_lanes=5')
