"""Run production dispatch coordinator against safe fixture APIs only."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import dispatch
import gate as g
from test_gate import FixtureAPI, HEAD, SIGNAL, DISPATCH, RUN, DRUN


class DispatchCoordinator(unittest.TestCase):
    def test_absent_activation_does_not_construct_api_client(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(dispatch, 'API') as client:
            dispatch.main()
        client.assert_not_called()

    def test_authorization_creates_exact_receipt_not_caller_identity(self):
        source = FixtureAPI()
        source.artifacts = []
        destination = FixtureAPI()
        destination.data[f'repos/{g.DEST}'] = {
            'id': g.DEST_ID, 'full_name': g.DEST, 'default_branch': 'main',
            'private': True, 'visibility': 'private'}
        with tempfile.TemporaryDirectory() as tmp:
            event = Path(tmp) / 'event.json'
            output = Path(tmp) / 'output'
            event.write_text(json.dumps({'action': 'completed', 'workflow_run': {'id': RUN},
                'sha': '$(malicious)', 'pr': 999, 'login': 'zackboll'}))
            env = {'ACTIVATED': 'true', 'GITHUB_EVENT_PATH': str(event),
                'GITHUB_EVENT_NAME': 'workflow_run', 'GITHUB_REF': 'refs/heads/main',
                'GITHUB_RUN_ID': str(DRUN), 'SIGNAL_WORKFLOW_ID': str(SIGNAL),
                'DISPATCH_WORKFLOW_ID': str(DISPATCH), 'SOURCE_TOKEN': 'fixture',
                'DISPATCH_TOKEN': 'fixture', 'GITHUB_OUTPUT': str(output), 'STATUS_BOT_USER_ID': '789'}
            old = Path.cwd()
            try:
                os.chdir(tmp)
                with patch.dict(os.environ, env), patch.object(dispatch, 'API', side_effect=[source, destination]):
                    dispatch.main()
                result = json.loads(Path('request.json').read_text())
                self.assertEqual(result['pr'], 7)
                self.assertEqual(result['sha'], HEAD)
                self.assertEqual(result['dispatcher_run'], DRUN)
                self.assertEqual(destination.posts, [])  # actual POST is later trusted workflow step
                self.assertIn('key=' + g.key(7, HEAD), output.read_text())
            finally:
                os.chdir(old)

    def test_duplicate_or_wrong_event_never_creates_new_receipt(self):
        for kind in ['duplicate', 'event', 'ref', 'public-destination']:
            source, destination = FixtureAPI(), FixtureAPI()
            destination.data[f'repos/{g.DEST}'] = {
                'id': g.DEST_ID, 'full_name': g.DEST, 'default_branch': 'main',
            'private': True, 'visibility': 'private'}
            if kind == 'public-destination':
                destination.data[f'repos/{g.DEST}']['private'] = False
            with tempfile.TemporaryDirectory() as tmp:
                event = Path(tmp) / 'event.json'
                event.write_text(json.dumps({'action': 'completed', 'workflow_run': {'id': RUN}}))
                env = {'ACTIVATED': 'true', 'GITHUB_EVENT_PATH': str(event),
                    'GITHUB_EVENT_NAME': 'push' if kind == 'event' else 'workflow_run',
                    'GITHUB_REF': 'refs/heads/evil' if kind == 'ref' else 'refs/heads/main',
                    'GITHUB_RUN_ID': str(DRUN), 'SIGNAL_WORKFLOW_ID': str(SIGNAL),
                    'DISPATCH_WORKFLOW_ID': str(DISPATCH), 'SOURCE_TOKEN': 'fixture',
                    'DISPATCH_TOKEN': 'fixture', 'GITHUB_OUTPUT': str(Path(tmp) / 'output')}
                with self.subTest(kind=kind), patch.dict(os.environ, env), \
                        patch.object(dispatch, 'API', side_effect=[source, destination]), self.assertRaises(g.Denied):
                    dispatch.main()
                self.assertEqual(destination.posts, [])

    def test_existing_designated_bot_status_denies_replay(self):
        source, destination = FixtureAPI(), FixtureAPI()
        source.artifacts = []
        actual_pages = source.pages
        source.pages = lambda path, field=None: ([{'context': 'ryzen/float32', 'creator': {'id': 789}}]
            if path.endswith('/statuses') else actual_pages(path, field))
        destination.data[f'repos/{g.DEST}'] = {
            'id': g.DEST_ID, 'full_name': g.DEST, 'default_branch': 'main',
            'private': True, 'visibility': 'private'}
        with tempfile.TemporaryDirectory() as tmp:
            event = Path(tmp) / 'event.json'
            event.write_text(json.dumps({'action': 'completed', 'workflow_run': {'id': RUN}}))
            env = {'ACTIVATED': 'true', 'GITHUB_EVENT_PATH': str(event), 'GITHUB_EVENT_NAME': 'workflow_run',
                'GITHUB_REF': 'refs/heads/main', 'GITHUB_RUN_ID': str(DRUN), 'SIGNAL_WORKFLOW_ID': str(SIGNAL),
                'DISPATCH_WORKFLOW_ID': str(DISPATCH), 'SOURCE_TOKEN': 'fixture', 'DISPATCH_TOKEN': 'fixture',
                'STATUS_BOT_USER_ID': '789', 'GITHUB_OUTPUT': str(Path(tmp) / 'output')}
            with patch.dict(os.environ, env), patch.object(dispatch, 'API', side_effect=[source, destination]), \
                    self.assertRaisesRegex(g.Denied, 'duplicate/replay'):
                dispatch.main()
        self.assertEqual(destination.posts, [])


if __name__ == '__main__':
    unittest.main()