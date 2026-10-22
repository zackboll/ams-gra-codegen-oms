#!/usr/bin/env python3
"""Task 077B bounded-layout guard; stdlib only, for this workflow's block form.

Qualification command digests pin the inherited commands/checks, not results.
Whitespace and whole-line comments are ignored; no qualification is executed here.
"""
import hashlib
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# Normalized run bodies at cf132afc4d4cd45d0511912f5f1d3c18066357ce.
GATES = [('Named TimeZulu pinned inventory and impact (Task 061)',
  'real-uci',
  '3833ff7d67852e9da3e4061a2d91b271f00cd7ec4cef87d2f3c2f75ae673570d'),
 ('Real UCI route identity smoke (Task 071)',
  'real-uci',
  '0f411a1ddd0345335528b262c3d5fe8ae3f8bcbb179ae5185276e7e1f83f1b12'),
 ('Pinned semantic schema diff (Task 069, once for both releases)',
  'real-uci',
  '602728b364330541668dee1bcc8b9f8b5192db545eb2f2236db5c9a1901ca07a'),
 ('Exact IPv6 pinned inventory, impact and real vertical (Task 062)',
  'real-uci',
  '4d132cb5bb518b7055d339b762015cfc6460b34a42d16e7c17c145745909d496'),
 ('Exact patterned integral pinned evidence (Task 063)',
  'real-uci',
  '3e13574d421b19f5174225cfb501b567d10cb5cee4df1e06fef9a345040c8a60'),
 ('Ada semantic companion naming and smallest real vertical (Task 065)',
  'real-uci',
  '80545597b6688c726a2ff0382a1654d6ab772fc8a2790c84c69ff39937b52a01'),
 ('Unicode 3.1 pinned profiles, impact and real vertical (Task 066)',
  'real-uci',
  'c5ab11b84e273c39862685931cd68ba1333912fb2eaf291dcfe862eff22b0fbe'),
 ('Confirm unique newly READY Time services (Task 061)',
  'real-uci-regressions',
  'd860583497398930f5d8a45746e5fc09305c8650874d3142c80b2cc63507a4c4'),
 ('Real UCI Binary provenance inventory (Task 052, must execute)',
  'real-uci-regressions',
  'e3254e809d3a881149c1c6fde719baa057fb8ac5111298e9c04416dedcd34791'),
 ('Real UCI constrained Binary inventory (Task 053, must execute)',
  'real-uci-regressions',
  'a6e954d63bbd1e17f8b707b6cee15627ba311500b503bf2bc26d2b24b8eed0f4'),
 ('Real UCI member identifier inventory (Task 054, must execute)',
  'real-uci-regressions',
  '89a85f9932a677be7550a3df1ac10bf03dc128dba1748c48185ff89f64a94d06'),
 ('Real UCI generated-support readiness parity (Task 056, must execute)',
  'real-uci-regressions',
  'd8fbf44500b89b0a080a8c5eb2c4f7d9664c67b1a5654da40acec91b3813f783'),
 ('Real UCI generated-support plan binding (Task 064, must execute)',
  'real-uci-regressions',
  'c79d2806e72befcfb53c3a32a6ce0270d65d12ca5296463ef1a4ebfe5393a6c3'),
 ('Real UCI duration inventory and message impact (Task 057, must execute)',
  'real-uci-regressions',
  'd5990a59157a025d95e19826bbbd379956c5a9fd4043179d26d6bcd7d8a3bf66'),
 ('Real UCI bounded-ASCII String inventory and message impact (Task 058, must execute)',
  'real-uci-regressions',
  'b39b1a9bb129a3e0cb32999b673e6159c79df5d4695d5bd0131dba28e716e966'),
 ('Real UCI structured ASCII evidence (Task 059, must execute)',
  'real-uci-regressions',
  'ded6e2dbd4cfee35b9a2ca0d651dd0c4580cce3d441470c3a86fec092e4e5231'),
 ('Real UCI alternating ASCII evidence (Task 060, must execute)',
  'real-uci-regressions',
  '808d7473c6591a958a71f74206b18082ee104ff025a0f260d6f494786be0751d')]


def check(path):
    text = Path(path).read_text()
    text = "\n".join(line for line in text.splitlines()
                     if not line.lstrip().startswith("#"))
    jobs = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", text, re.M | re.S))
    errors = []
    def require(ok, message):
        if not ok:
            errors.append("Task077B " + message)
    for job, releases in (("real-uci", ("2.5", "2.6")),
                          ("real-uci-regressions", ("2.5", "2.6")),
                          ("msrv-real-uci", ("2.5",)), ("real-sleet", ("2.5",))):
        body = jobs.get(job, "")
        require(bool(body), "missing job: " + job)
        for release in ("2.5", "2.6"):
            count = len(re.findall(r"scripts/fetch-pinned-uci-" + re.escape(release)
                                  + r"\.sh [\"']?\$", body))
            require(count == int(release in releases), f"fetch ownership: {job} {release}")
        if job not in ("real-uci", "real-uci-regressions"):
            continue
        limit = re.search(r"^    timeout-minutes: ([0-9]+)$", body, re.M)
        require(limit is not None and int(limit[1]) == 240, "finite timeout: " + job)
        require(not re.search(r"^    needs:", body, re.M), "independent job: " + job)
        # PERF-001: only the exact evidence-only upload may run after failure.
        # Do not exempt arbitrary named steps, commands or conditional gates.
        evidence = '''      - name: Preserve Task 061 per-selection evidence
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: task061-service-check
          path: ${{ runner.temp }}/task061-service-check-*/**
          if-no-files-found: error
'''
        gate_body = body.replace(evidence, '') if job == 'real-uci-regressions' else body
        require(not re.search(r"^\s+(if:|continue-on-error:)", gate_body, re.M), "enabled hard gate: " + job)
        for setup in ('uses: actions/checkout@v4', 'uses: dtolnay/rust-toolchain@stable',
                      'sudo apt-get install -y gnat', 'command -v gnatmake', 'gnatmake --version',
                      'AMS_GRA_UCI_2_5_ROOT=$UCI25', 'AMS_GRA_UCI_2_6_ROOT=$UCI26'):
            require(setup in body, "input setup: " + job + " " + setup)
    b = jobs.get("real-uci-regressions", "")
    build = 'cargo build --release --locked -p ams-gra-codegen-oms'
    service = 'python3 scripts/check-task061-service-impact.py target/release/ams-gra-codegen-oms'
    require(b.count(build) == 1 and b.find(build) < b.find(service), "release CLI build before service confirmations")
    for setup in ('cargo fetch --locked', 'test -n "$AMS_GRA_UCI_2_5_ROOT"',
                  'test -n "$AMS_GRA_UCI_2_6_ROOT"', 'export TMPDIR="$RUNNER_TEMP"',
                  'AMS_GRA_REQUIRE_GNAT: "1"'):
        require(setup in b, "regression input: " + setup)
    for owner in ('real-uci', 'real-uci-regressions'):
        expected = [name for name, job, _ in GATES if job == owner]
        actual = re.findall(r'^      - name: (.*)$', jobs.get(owner, ''), re.M)
        require([name for name in actual if name in expected] == expected,
                'qualification order: ' + owner)
    for name, owner, digest in GATES:
        found = []
        for job, body in jobs.items():
            steps = re.split(r"^      - ", body, flags=re.M)[1:]
            for step in steps:
                if step.splitlines()[0] != 'name: ' + name:
                    continue
                found.append(job)
                match = re.search(r"^        run: (.*?)(?:\n|\Z)", step, re.M)
                command = ''
                if match:
                    if match[1] == '|':
                        command = '\n'.join(line.strip() for line in step[match.end():].splitlines()
                                            if line.startswith('          ') and line.strip())
                    else:
                        command = match[1]
                require(hashlib.sha256(command.encode()).hexdigest() == digest,
                        "payload changed: " + name)
                require(not re.search(r"^\s+(if:|continue-on-error:)", step, re.M),
                        "enabled hard gate: " + name)
                if 'Task 054,' in name:
                    require('AMS_GRA_REQUIRE_GNAT: "1"' in step,
                            "compiler environment: " + name)
        require(found == [owner], "gate ownership: " + name)
    return errors


def self_test(path, guard, fast, sleet):
    """Disposable single-defect mutations must produce their own diagnostic."""
    original = Path(path).read_text()
    cases = []
    def job_match(job):
        matches = list(re.finditer(
            r"^  " + re.escape(job) + r":\n(.*?)(?=^  [\w-]+:|\Z)",
            original, re.M | re.S))
        assert len(matches) == 1, job
        return matches[0]
    def job_mutation(job, change):
        match = job_match(job)
        return original[:match.start()] + change(match[0]) + original[match.end():]
    def replace_once(body, target, replacement):
        assert body.count(target) == 1, target
        return body.replace(target, replacement, 1)
    def command_line(job, command):
        lines = [line for line in job_match(job)[0].splitlines(True)
                 if command in line and not line.lstrip().startswith('#')]
        assert len(lines) == 1, (job, command)
        assert lines[0].count(command) == 1, (job, command)
        return lines[0]
    def step_match(body, name):
        matches = list(re.finditer(r'^      - name: ' + re.escape(name)
                                  + r'\n.*?(?=^      - |\Z)', body, re.M | re.S))
        assert len(matches) == 1, name
        return matches[0]
    def add(label, text, diagnostic):
        assert text != original, label
        cases.append((label, text, 'Task077B ' + diagnostic))
    add('missing second job', job_mutation('real-uci-regressions', lambda b: ''),
        'missing job: real-uci-regressions')
    for job in ('real-uci', 'real-uci-regressions'):
        for release in ('2.5', '2.6'):
            # Select the executable fetch in its owner, never another job's
            # command (which may use a different scratch directory).
            line = command_line(job, f'scripts/fetch-pinned-uci-{release}.sh "$')
            add(f'{job} missing {release}',
                job_mutation(job, lambda b: replace_once(b, line, '')),
                f'fetch ownership: {job} {release}')
            add(f'{job} duplicate {release}',
                job_mutation(job, lambda b: replace_once(b, line, line + line)),
                f'fetch ownership: {job} {release}')
        for field in ('if: false', 'continue-on-error: true'):
            add(f'{job} {field}', job_mutation(job, lambda b: replace_once(
                b, '    runs-on:', f'    {field}\n    runs-on:')),
                f'enabled hard gate: {job}')
        add(f'{job} missing timeout', job_mutation(job, lambda b: replace_once(
            b, '    timeout-minutes: 240\n', '')), f'finite timeout: {job}')
        add(f'{job} serialized', job_mutation(job, lambda b: replace_once(
            b, '    runs-on:', '    needs: real-sleet\n    runs-on:')),
            f'independent job: {job}')
    for label, command, diagnostic in (
        ('missing CLI build', 'cargo build --release --locked -p ams-gra-codegen-oms',
         'release CLI build before service confirmations'),
        ('missing dependency preparation', 'cargo fetch --locked',
         'regression input: cargo fetch --locked'),
    ):
        line = command_line('real-uci-regressions', command)
        add(label, job_mutation('real-uci-regressions',
                                lambda b: replace_once(b, line, '')), diagnostic)
    for name, owner, _ in GATES[7:]:
        match = step_match(job_match(owner)[0], name)
        step = match[0]
        add('delete ' + name, job_mutation(owner, lambda b: replace_once(b, step, '')),
            'gate ownership: ' + name)
        add('duplicate ' + name, job_mutation(owner, lambda b: replace_once(b, step, step + step)),
            'gate ownership: ' + name)
    name, owner, _ = GATES[-1]
    match = step_match(job_match(owner)[0], name)
    step = match[0]
    for line in step.splitlines(True):
        if 'cargo test ' in line or 'grep ' in line:
            add('Task060 assertion/command ' + line.strip(),
                job_mutation(owner, lambda b: replace_once(
                    b, step, replace_once(step, line, ''))),
                'payload changed: ' + name)
    for field in ('if: false', 'continue-on-error: true'):
        add('disabled Task060 ' + field, job_mutation(owner, lambda b: replace_once(
            b, '      - name: ' + name + '\n',
            '      - name: ' + name + '\n        ' + field + '\n')),
            'enabled hard gate: ' + name)
    with tempfile.TemporaryDirectory(prefix='task077b-layout-') as tmp:
        fixture = Path(tmp) / 'deep.yml'
        for label, text, diagnostic in [('healthy', original, None)] + cases:
            fixture.write_text(text)
            result = subprocess.run(['bash', guard, fast, str(fixture), sleet],
                                    capture_output=True, text=True)
            if diagnostic is None:
                assert result.returncode == 0, result.stderr
            else:
                assert result.returncode != 0 and diagnostic in result.stderr, (
                    label, diagnostic, result.returncode, result.stderr)
    print(len(cases) + 1)


if __name__ == '__main__' and sys.argv[1] == '--self-test':
    self_test(*sys.argv[2:])
elif __name__ == '__main__':
    errors = check(sys.argv[1])
    for error in errors:
        print('check-ci-split: ' + error, file=sys.stderr)
    sys.exit(bool(errors))
