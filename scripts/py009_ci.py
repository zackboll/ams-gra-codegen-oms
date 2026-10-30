"""Additive PY-009 payload projection onto immutable PY-008 CI contracts.

No job, matrix, lane, evidence or final-gate policy is replaced. The original
policy independently checks every byte left after these mandatory additions.
"""
import re

FAST = ('          cargo test --locked -p ams-gra-codegen-oms --test python_datetime -- --nocapture\n'
        '          python scripts/test-py009-uci.py\n')
DEEP = ('          python3 scripts/test-py009-uci.py\n'
        '          python3 scripts/check-py009-uci.py --generator target/release/ams-gra-codegen-oms --output "$RUNNER_TEMP/py006-uci/py009"\n')
BASELINE = '--service-baseline tests/fixtures/python-datetime/after.json'
SLEET = '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py009-sleet.sh"\n'
BUILD = '          cargo build --locked --release -p ams-gra-codegen-oms --example python_datetime_closure\n'


def project(workflows):
    result = dict(workflows)
    for workflow, addition, job in [('ci.yml', FAST, 'python-model-backend'),
                                    ('deep-ci.yml', DEEP, 'real-uci-regressions')]:
        jobs = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)', result[workflow], re.M | re.S))
        if result[workflow].count(addition) != 1 or jobs.get(job, '').count(addition) != 1:
            raise ValueError('inherited qualification payload/evidence changed: ' + job + ' (mandatory PY-009)')
        result[workflow] = result[workflow].replace(addition, '', 1)
    deep = result['deep-ci.yml']
    if deep.count(BUILD) != 1:
        raise ValueError('inherited qualification payload/evidence changed: real-uci-regressions (PY-009 closure tool)')
    deep = deep.replace(BUILD, '', 1)
    if deep.count(BASELINE) != 2:
        raise ValueError('inherited qualification payload/evidence changed: real-uci-regressions (PY-009 baseline)')
    deep = deep.replace(' --existing-output ' + BASELINE, ' --existing-output', 1)
    deep = deep.replace(BASELINE, '--service-baseline tests/fixtures/python-float64/after.json', 1)
    result['deep-ci.yml'] = deep
    return result


def project_sleet(text):
    loop = ('for interpreter in ${PY006_PYTHONS:-${PYTHON:-python3}}; do\n'
            '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py006-sleet.sh"\n'
            '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py007-sleet.sh"\n'
            '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py008-sleet.sh"\n' + SLEET + 'done')
    if text.count(SLEET) != 1 or text.count(loop) != 1:
        raise ValueError('PY-008 shared-Sleet execution changed (mandatory PY-009)')
    return text.replace(SLEET, '', 1)
