"""Verify PY-010 additions before CI-SELECT-003 and immutable PY-009 contracts."""
import re

FAST = ('          cargo test --locked -p ams-gra-codegen-oms --test python_visible_ascii -- --nocapture\n'
        '          python scripts/test-py010-uci.py\n'
        '          python scripts/check-py010-parity.py --generator target/debug/ams-gra-codegen-oms --output "$RUNNER_TEMP/py010-parity"\n')
DEEP_SETUP = '''      - name: Install PY-010 pinned-profile Python interpreters
        uses: actions/setup-python@v5
        with:
          python-version: |
            3.11
            3.13
'''
DEEP = ('          cargo build --locked --release -p ams-gra-codegen-oms --example python_visible_ascii_profiles\n'
        '          python3 scripts/test-py010-uci.py\n'
        '          for py010_interpreter in python3.11 python3.13; do\n'
        '            "$py010_interpreter" scripts/check-py010-uci.py --generator target/release/ams-gra-codegen-oms --output "$RUNNER_TEMP/py006-uci/py010-$py010_interpreter"\n'
        '          done\n')
BASELINE = '--service-baseline tests/fixtures/python-visible-ascii/after.json'
SLEET = '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py010-sleet.sh"\n'
SLEET_ARTIFACT = '''      - name: Preserve PY-010 generated sources and observed broker events
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: py010-sleet
          path: ${{ runner.temp }}/py010-sleet/**
          if-no-files-found: error
'''


def project(workflows):
    result = dict(workflows)
    for workflow, addition, job, following in [
            ('ci.yml', FAST, 'python-model-backend', '          python scripts/test-py007-uci.py\n'),
            ('deep-ci.yml', DEEP, 'real-uci-regressions',
             '      - name: Preserve Python pinned UCI measurements\n')]:
        jobs = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)', result[workflow], re.M | re.S))
        if (result[workflow].count(addition) != 1
                or jobs.get(job, '').count(addition + following) != 1):
            raise ValueError('inherited qualification payload/evidence changed: ' + job + ' (mandatory PY-010)')
        result[workflow] = result[workflow].replace(addition, '', 1)
    deep = result['deep-ci.yml']
    jobs = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)', deep, re.M | re.S))
    setup_following = '      - name: Python current services and immutable historical profiles (PY-007 and PY-008, must execute)\n'
    if (deep.count(DEEP_SETUP) != 1
            or jobs.get('real-uci-regressions', '').count(DEEP_SETUP + setup_following) != 1):
        raise ValueError('mandatory PY-010 pinned-profile interpreter setup changed')
    deep = deep.replace(DEEP_SETUP, '', 1)
    if (deep.count(SLEET_ARTIFACT) != 1
            or not jobs.get('real-sleet', '').rstrip().endswith(SLEET_ARTIFACT.rstrip())):
        raise ValueError('mandatory PY-010 shared-Sleet evidence changed')
    deep = deep.replace(SLEET_ARTIFACT, '', 1)
    command = ('          python3 scripts/check-py009-uci.py --generator target/release/ams-gra-codegen-oms '
               '--output "$RUNNER_TEMP/py006-uci/py009"')
    if deep.count(BASELINE) != 3 or deep.count(command + ' ' + BASELINE + '\n') != 1:
        raise ValueError('inherited qualification payload/evidence changed: real-uci-regressions (PY-010 baselines)')
    deep = deep.replace(command + ' ' + BASELINE, command, 1)
    result['deep-ci.yml'] = deep.replace(BASELINE, '--service-baseline tests/fixtures/python-datetime/after.json')
    return result


def project_sleet(text):
    anchor = '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py009-sleet.sh"\n'
    # CI-SELECT-003 independently verifies its following hooks and loop end.
    # Do not make a missing owned hook masquerade as a PY-010 failure here.
    if text.count(SLEET) != 1 or text.count(anchor + SLEET) != 1:
        raise ValueError('PY-009 shared-Sleet execution changed (mandatory PY-010)')
    return text.replace(SLEET, '', 1)