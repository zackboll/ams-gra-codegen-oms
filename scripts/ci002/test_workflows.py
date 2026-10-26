"""Source-level trust contracts plus mutation-negative workflow validation."""
from pathlib import Path
import copy
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]


def load(path):
    return yaml.load(path.read_text(), Loader=yaml.BaseLoader)


def validate_signal(workflow):
    assert workflow['name'] == 'OMS Trusted PR Signal'
    assert set(workflow['on']) == {'pull_request'}
    assert workflow['permissions'] == {}
    assert set(workflow['jobs']) == {'signal'}
    job = workflow['jobs']['signal']
    assert job['runs-on'] == 'ubuntu-24.04'
    assert len(job['steps']) == 1
    assert job['steps'][0]['run'] == "printf '%s\\n' 'PR signal complete; authorization is performed separately.'"
    assert 'secrets.' not in str(workflow)
    assert 'uses' not in job['steps'][0]


def validate_dispatch(workflow):
    assert set(workflow['on']) == {'workflow_run'}
    assert workflow['on']['workflow_run']['workflows'] == ['OMS Trusted PR Signal']
    job = workflow['jobs']['dispatch']
    assert job['if'] == "vars.OMS_TRUSTED_PR_DISPATCH_ENABLED == 'true'"
    assert job['runs-on'] == 'ubuntu-24.04'
    checkout = job['steps'][0]
    assert checkout['with']['ref'] == '${{ github.sha }}'
    assert checkout['with']['persist-credentials'] == 'false'
    assert workflow['permissions'] == {'contents': 'read', 'actions': 'read', 'checks': 'read',
                                       'pull-requests': 'read', 'statuses': 'read'}
    assert 'pull_request_target' not in str(workflow)
    assert 'client_payload' not in checkout['with']['ref']
    assert any(s.get('run') == 'python3 scripts/ci002/dispatch.py' for s in job['steps'])
    for step in job['steps']:
        if step.get('uses', '').startswith('actions/create-github-app-token@'):
            assert step['with']['repositories'] == 'ci-runner-pilot'
            assert step['with']['permission-contents'] == 'write'


class Workflows(unittest.TestCase):
    def setUp(self):
        self.signal = load(ROOT / '.github/workflows/oms-trusted-pr-signal.yml')
        self.dispatch = load(ROOT / '.github/workflows/oms-ryzen-trusted-dispatch.yml')

    def test_actual_source_trust_contracts(self):
        validate_signal(self.signal)
        validate_dispatch(self.dispatch)

    def test_signal_execution_mutations_rejected(self):
        for change in ['runner', 'checkout', 'script', 'secret', 'permission']:
            mutated = copy.deepcopy(self.signal)
            job = mutated['jobs']['signal']
            if change == 'runner': job['runs-on'] = 'self-hosted'
            if change == 'checkout': job['steps'].append({'uses': 'actions/checkout@v4'})
            if change == 'script': job['steps'][0]['run'] = 'bash scripts/from-pr.sh'
            if change == 'secret': job['env'] = {'TOKEN': '${{ secrets.TOKEN }}'}
            if change == 'permission': mutated['permissions'] = {'contents': 'write'}
            with self.subTest(change=change), self.assertRaises(AssertionError):
                validate_signal(mutated)

    def test_dispatcher_pr_code_and_activation_mutations_rejected(self):
        for change in ['ref', 'runner', 'activation', 'event', 'permissions']:
            mutated = copy.deepcopy(self.dispatch)
            job = mutated['jobs']['dispatch']
            if change == 'ref': job['steps'][0]['with']['ref'] = '${{ github.event.workflow_run.head_sha }}'
            if change == 'runner': job['runs-on'] = 'self-hosted'
            if change == 'activation': job['if'] = 'true'
            if change == 'event': mutated['on'] = {'pull_request_target': {}}
            if change == 'permissions': mutated['permissions']['contents'] = 'write'
            with self.subTest(change=change), self.assertRaises(AssertionError):
                validate_dispatch(mutated)


if __name__ == '__main__':
    unittest.main()