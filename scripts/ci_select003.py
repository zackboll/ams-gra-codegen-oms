"""CI-SELECT-003: exact Python-owned source contract and workflow deltas.

The original CI-SELECT-002 rules remain separately pinned by ci_policy.py.
Only these four reviewed additions are permitted before that immutable suffix.
"""
import re

RULES = [
    {
        'category': "python-qualification-fast",
        'paths': [
            "python/qualification/fast/test_*.py"
        ],
        'jobs': [
            "rust",
            "python-model-backend"
        ],
        'rationale': "CI-SELECT-003 fixed fail-closed Fast discovery runs in both Python matrix cells and Rust retains generator/workspace coverage.",
        'deep': False,
        'deep_jobs': [],
        'deep_rationale': "This flat Fast test source is consumed by Python Fast and Rust, not an external pinned runner."
    },
    {
        'category': "python-qualification-uci",
        'paths': [
            "python/qualification/uci/test_*.py"
        ],
        'jobs': [
            "rust",
            "python-model-backend"
        ],
        'rationale': "CI-SELECT-003 fixed pinned-UCI discovery runs after verified roots/release generator; Python Fast and Rust retain primary tests.",
        'deep': True,
        'deep_jobs': [
            "real-uci-regressions"
        ],
        'deep_rationale': "Only the existing real-uci-regressions job executes pinned Python UCI tests and preserves py006-uci evidence."
    },
    {
        'category': "python-qualification-sleet",
        'paths': [
            "python/qualification/sleet/test_*.py"
        ],
        'jobs': [
            "rust",
            "python-model-backend"
        ],
        'rationale': "CI-SELECT-003 fixed pinned-Sleet discovery reuses shared broker across Python versions/policies; ordinary Python/Rust tests remain.",
        'deep': True,
        'deep_jobs': [
            "real-sleet"
        ],
        'deep_rationale': "Only the existing real-sleet job owns the Python broker test hook; no Ada transport dependency."
    },
    {
        'category': "python-cli-example",
        'paths': [
            "crates/cli/examples/python_*.rs"
        ],
        'jobs': [
            "rust",
            "python-model-backend"
        ],
        'rationale': "Python-specific evidence-only CLI examples compile with the Rust workspace and are used for pinned Python UCI qualification; not production CLI.",
        'deep': True,
        'deep_jobs': [
            "msrv-real-uci",
            "real-uci-regressions"
        ],
        'deep_rationale': "MSRV compiles the CLI examples through Rust all-target coverage; Python UCI regression job owns release example execution."
    }
]

FAST_ANCHOR = '          python scripts/test-py009-uci.py\n'
FAST = ('          python python/qualification/run.py fast\n'
        '          python -O python/qualification/run.py fast\n'
        '          python scripts/test_ci_select003.py\n')
DEEP_ANCHOR = ('          python3 scripts/check-py009-uci.py --generator '
               'target/release/ams-gra-codegen-oms --output "$RUNNER_TEMP/py006-uci/py009"\n')
DEEP = '          python3 python/qualification/run.py uci\n'
SLEET_ANCHOR = '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py009-sleet.sh"\n'
SLEET = ('  "$interpreter" "$ROOT/python/qualification/run.py" sleet\n'
         '  "$interpreter" -O "$ROOT/python/qualification/run.py" sleet\n')


def project(workflows):
    result = dict(workflows)
    for workflow, job, anchor, addition in (
            ('ci.yml', 'python-model-backend', FAST_ANCHOR, FAST),
            ('deep-ci.yml', 'real-uci-regressions', DEEP_ANCHOR, DEEP)):
        text = result[workflow]
        jobs = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)', text, re.M | re.S))
        if (text.count(anchor + addition) != 1
                or jobs.get(job, '').count(anchor + addition) != 1):
            raise ValueError('CI-SELECT-003 mandatory owned Python gate missing/moved: ' + job)
        result[workflow] = text.replace(anchor + addition, anchor, 1)
    return result


def project_sleet(text):
    expected = SLEET_ANCHOR + SLEET + 'done\n'
    if text.count(expected) != 1:
        raise ValueError('CI-SELECT-003 mandatory per-interpreter pinned-Sleet gate missing/moved')
    return text.replace(expected, SLEET_ANCHOR + 'done\n', 1)
