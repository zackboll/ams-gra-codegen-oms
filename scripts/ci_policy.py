"""Fail-closed bounded workflow policy, stdlib only.

This is deliberately NOT a general YAML loader. We accept the repository's
explicit block-form jobs and inventory-pinned payloads. Unsupported aliases,
flow jobs, tags, dynamic matrices, or new workflows require policy review.
"""
import json
import hashlib
from pathlib import Path
import re
from ci_selection import ROOT, inventory, ordinary_jobs, select

# Independently reconciled from d603077c6d38ccc0658391776e3af9eecb009255.
# These are normalized ORIGINAL payloads, not candidate workflow snapshots.
INHERITED_PAYLOADS = {
    ('ci.yml', 'ada-lacal-runtime-core'): 'c30d66a0dc4fdf56313964f98407784670cd42b066d3ed1d0fddd5a3a07a1368',
    ('ci002-validation.yml', 'test'): '9b0173054780810fd2541b0eebcb67d51c41173bc0a3e8d25beb069191f0e2ca',
    ('oms-trusted-pr-signal.yml', 'signal'): 'ba84d00fbfcc1b93274e2077c7c91bffb31a810828d1132e4e87ffdb46acb585',
    ('oms-ryzen-trusted-dispatch.yml', 'dispatch'): '837b0a7ed7ff1d6289f3c521e2fffd7548eecbd6a26cccda228e9732fff114ac',
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


def check(policy=None, workflows=None):
    policy = policy or inventory()
    for path, digest in policy.get('security_source_sha256', {}).items():
        if hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            raise ValueError('inherited trusted-dispatch source changed: ' + path)
    if workflows is None:
        workflows = {p.name: p.read_text() for p in (ROOT / '.github/workflows').iterdir() if p.suffix in ('.yml', '.yaml')}
    from real_uci_evidence import check_layout
    check_layout(blocks(workflows['deep-ci.yml'])[1])
    if policy['lanes'] != [f'oms-main-slot-{n}' for n in range(1, 6)]:
        raise ValueError("must define exactly five unique common lanes")
    security = {'ci002-validation.yml', 'oms-trusted-pr-signal.yml', 'oms-ryzen-trusted-dispatch.yml'}
    if set(policy.get('security_headers', {})) != security:
        raise ValueError('required independent security workflow removed')
    runtime = [r for r in policy['jobs'] if (r['workflow'], r['job_id']) == ('ci.yml', 'ada-lacal-runtime-core')]
    if len(runtime) != 1 or [(c['key'], c['name'], c['variant']) for c in runtime[0]['cells']] != [
            ('ada-lacal-runtime-core-' + v, 'ada-lacal-runtime-core (' + v + ')', {'compiler': v}) for v in ('14', '16')]:
        raise ValueError('Task099 requires exactly GNAT14/GNAT16 qualification cells')
    expected = {}
    for record in policy['jobs']:
        key = record['workflow'], record['job_id']
        if key in expected:
            raise ValueError('duplicate runner inventory job')
        if key in INHERITED_PAYLOADS and hashlib.sha256(normalized(record['original_steps_yaml']).encode()).hexdigest() != INHERITED_PAYLOADS[key]:
            raise ValueError('verified current-main baseline payload changed: ' + record['job_id'])
        expected[key] = record
    for job_id, record in policy['orchestration_jobs'].items():
        expected[record['workflow'], job_id] = dict(cells=[dict(lane=record['lane'])])
    if set(workflows) != {w for w, _ in expected}:
        raise ValueError('unmapped workflow runner inventory')
    count = 0
    for workflow, text in workflows.items():
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
                elif workflow=='deep-ci.yml' and field(top,'if'):
                    raise ValueError('Deep main qualification became optional: '+job_id)
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
        sample = select([dict(status='M', paths=[r['paths'][0].replace('**','example').replace('*','example')],
                              old_mode='100644', new_mode='100644')],policy)
        if not all(sample['jobs'][j] for j in r['jobs']):
            raise ValueError('source rule shadows required dependency: '+r['category'])
    return count


if __name__ == '__main__':
    print('CI_POLICY_COMPLETE runner_cells=' + str(check()) + ' maximum_main_lanes=5')