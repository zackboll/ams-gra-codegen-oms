#!/usr/bin/env python3
"""Nine supplemental identity/backend/runtime adversaries per compiler."""
import argparse
import copy
import json
from pathlib import Path

from perf005_provider import digest
from perf009_evidence import verify_trace
from perf009_portable import validate_build
from perf009_provider import verify_provider


def reject(callback):
    try:
        callback()
    except (RuntimeError, ValueError, KeyError):
        return
    raise RuntimeError('supplemental adversary accepted')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--campaign', type=Path, required=True)
    parser.add_argument('--compiler', choices=['14', '16'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    records = []
    for policy in ('normal', 'Check', 'Ignore'):
        variant = args.campaign / 'perf005-providers' / policy
        identity = json.loads((variant / 'complete.json').read_text())['identity']
        other = dict(identity, compiler='16' if args.compiler == '14' else '14')
        reject(lambda: verify_provider(variant, other, digest(variant / 'complete.json')))
        records.append(dict(policy=policy, control='cross-compiler', rejected=True))
        data = verify_trace(args.campaign / f'oam-ArraySample-{policy}-build.profile.json')
        for control in ('missing-backend', 'wrong-runtime'):
            changed = copy.deepcopy(data)
            if control == 'missing-backend':
                changed['executions'] = [r for r in changed['executions'] if Path(r['executable']).name != 'gnat1']
            else:
                for row in changed['executions']:
                    if '-c' in row['argv']:
                        row['argv'].append('--RTS=/wrong/runtime')
            reject(lambda: validate_build(changed['command'], args.campaign / 'oam-ArraySample',
                                          changed, args.compiler, policy, 0))
            records.append(dict(policy=policy, control=control, rejected=True))
    if len(records) != 9:
        raise RuntimeError('supplemental inventory incomplete')
    args.output.write_text(json.dumps(dict(completed=True, compiler=args.compiler, records=records), indent=2) + '\n')
    print('PERF009R_SUPPLEMENTAL_COMPLETE', args.compiler, len(records))


if __name__ == '__main__':
    main()