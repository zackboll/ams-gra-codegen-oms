"""Hosted/default-branch dispatcher; no PR checkout or artifact execution."""
import json
import os
from pathlib import Path
import sys

from gate import (API, DEST, DEST_ID, SOURCE, Denied, authorize_signal,
                  dispatch_identity, integer, repository, require, verify_dispatch_run)


def main():
    if os.environ.get('ACTIVATED') != 'true':
        print('DENY activation disabled')
        return
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    source = API(os.environ['SOURCE_TOKEN'])
    destination = API(os.environ['DISPATCH_TOKEN'])
    require(os.environ['GITHUB_EVENT_NAME'] == 'workflow_run'
            and os.environ['GITHUB_REF'] == 'refs/heads/main', 'wrong dispatcher event/ref')
    require(event['action'] == 'completed', 'signal not completed')
    run_id = integer(event['workflow_run']['id'])
    dispatcher = int(os.environ['GITHUB_RUN_ID'])
    dispatch = verify_dispatch_run(source, dispatcher, int(os.environ['DISPATCH_WORKFLOW_ID']))
    auth = authorize_signal(source, run_id, int(os.environ['SIGNAL_WORKFLOW_ID']),
                            trusted_revision=dispatch['head_sha'])
    auth.update(dispatch_identity(source, dispatch))
    repository(destination, DEST, DEST_ID)
    # Artifacts provide a durable idempotency ledger. Fail closed on any previous
    # receipt for this exact qualification key, even if dispatch later failed.
    previous = source.pages(f'repos/{SOURCE}/actions/artifacts?name={auth["key"]}', 'artifacts')
    require(not any(a['name'] == auth['key'] for a in previous), 'duplicate qualification receipt')
    statuses = source.pages(f'repos/{SOURCE}/commits/{auth["sha"]}/statuses')
    reporter = int(os.environ['STATUS_BOT_USER_ID'])
    require(not any(s['context'] == 'ryzen/float32' and s['creator']['id'] == reporter
                    for s in statuses), 'previous exact-SHA status; duplicate/replay denied')
    auth.pop('stale')
    Path('request.json').write_text(json.dumps(auth, sort_keys=True) + '\n')
    with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
        output.write('key=' + auth['key'] + '\n')
        output.write('signal_run=' + str(run_id) + '\n')
    print(json.dumps({'decision': 'allow', **auth}, sort_keys=True))


if __name__ == '__main__':
    try:
        main()
    except (Denied, KeyError, TypeError, ValueError) as error:
        print('DENY ' + str(error), file=sys.stderr)
        sys.exit(1)