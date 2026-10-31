"""Exact additive workflow deltas; historical payload authorities stay unchanged."""
import re


def upload(name, path):
    return f'''      - name: Upload mandatory source compiler and execution evidence
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: {name}
          path: {path}
          if-no-files-found: error
'''


ADA_FAST = '''      - name: Execute source-bound Ada Fast registrations
        shell: bash
        env:
          PYTHONDONTWRITEBYTECODE: '1'
        run: python3 scripts/run-owned-ada.py fast --compiler '${{ matrix.compiler }}' --setup /tmp/task099-evidence/setup --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms" --output /tmp/ci004-ada-fast
''' + upload('ci004-ada-fast-${{ matrix.compiler }}', '/tmp/ci004-ada-fast/**')
ADA_PINNED = '''      - name: Execute source-bound Ada pinned registrations with shared broker
        shell: bash
        env:
          PYTHONDONTWRITEBYTECODE: '1'
        run: |
          set -euo pipefail
          /tmp/task101-venv/bin/python scripts/run-owned-ada.py pinned --compiler '${{ matrix.compiler }}' --setup /tmp/task101-evidence/setup --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms" --output /tmp/ci004-ada-pinned --sleet /tmp/task101-sleet --shared-campaign /tmp/task101-evidence/campaign --uci25 "$(cat /tmp/task101-evidence/uci25-root.txt)" --uci26 "$(cat /tmp/task101-evidence/uci26-root.txt)"
''' + upload('ci004-ada-pinned-${{ matrix.compiler }}', '/tmp/ci004-ada-pinned/**')
PYTHON_FAST = '''      - name: Execute source-bound Python Fast inventory
        run: |
          set -euo pipefail
          python scripts/run-owned-python.py fast
          python -O scripts/run-owned-python.py fast
''' + upload('ci004-python-fast-${{ matrix.python-version }}', '${{ runner.temp }}/ci004-python-fast/**')
PYTHON_UCI = '''      - name: Execute source-bound Python pinned UCI inventory in both interpreters
        run: |
          set -euo pipefail
          for ci004_interpreter in python3.11 python3.13; do
            "$ci004_interpreter" scripts/run-owned-python.py uci
            "$ci004_interpreter" -O scripts/run-owned-python.py uci
          done
''' + upload('ci004-python-uci', '${{ runner.temp }}/ci004-python-uci/**').replace('        if: always()\n', '')
PYTHON_SLEET = upload('ci004-python-sleet', '${{ runner.temp }}/ci004-python-sleet/**')
HOOKS = (
    ('ci.yml', 'ada-lacal-runtime-core', ADA_FAST),
    ('ci.yml', 'python-model-backend', PYTHON_FAST),
    ('deep-ci.yml', 'ada-real-uci-dlz-sleet', ADA_PINNED),
    ('deep-ci.yml', 'real-uci-regressions', PYTHON_UCI),
    ('deep-ci.yml', 'real-sleet', PYTHON_SLEET),
)
SLEET = ('  "$interpreter" "$ROOT/scripts/run-owned-python.py" sleet\n'
         '  "$interpreter" -O "$ROOT/scripts/run-owned-python.py" sleet\n')


def project(workflows):
    result = dict(workflows)
    for workflow, owner, addition in HOOKS:
        text = result[workflow]
        jobs = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)', text, re.M | re.S))
        body = jobs.get(owner, '')
        if (text.count(addition) != 1 or body.count(addition) != 1 or
                any(line.strip() and not line.lstrip().startswith('#')
                    for line in body.split(addition, 1)[-1].splitlines())):
            raise ValueError('CI-SELECT-004 mandatory hook missing/moved/masked: ' + owner)
        result[workflow] = text.replace(addition, '', 1)
    return result


def project_sleet(text):
    from ci_select003 import SLEET as inherited
    if text.count(inherited) != 1:
        raise ValueError('CI-SELECT-003 mandatory per-interpreter pinned-Sleet gate missing/moved')
    if text.count(inherited + SLEET + 'done\n') != 1:
        raise ValueError('CI-SELECT-004 shared pinned-Sleet hook missing/moved/masked')
    return text.replace(SLEET, '', 1)